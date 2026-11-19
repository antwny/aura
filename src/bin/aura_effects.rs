//! Aura's lightweight effects renderer.
//!
//! A separate process (like `mpvpaper`) so it can own its own Wayland
//! connection and create one layer-shell surface per monitor:
//!
//! * `parallax`: a click-through bottom-layer image displaced by a depth map
//!   with a slow automatic drift (2.5D wallpaper).
//! * `spectrum`: a transparent bottom-layer strip with cava-style audio bars.
//!
//! Aura writes a JSON config and spawns this binary; it exits automatically
//! when its parent process disappears.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::ptr::NonNull;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use serde::Deserialize;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry,
    delegate_seat,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_compositor, wl_output, wl_region, wl_seat, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct EffectsConfig {
    #[serde(default)]
    outputs: Vec<OutputConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct OutputConfig {
    name: String,
    #[serde(default)]
    parallax: Option<ParallaxConfig>,
    #[serde(default)]
    spectrum: Option<SpectrumConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct ParallaxConfig {
    image: String,
    #[serde(default)]
    depth: Option<String>,
    #[serde(default = "default_strength")]
    strength: f32,
}

fn default_strength() -> f32 {
    0.55
}

#[derive(Debug, Clone, Deserialize)]
struct SpectrumConfig {
    #[serde(default = "default_bars")]
    bars: u32,
    #[serde(default = "default_height")]
    height_pct: u32,
    #[serde(default = "default_color1")]
    color1: [u8; 3],
    #[serde(default = "default_color2")]
    color2: [u8; 3],
    #[serde(default = "default_alpha")]
    alpha: f32,
}

fn default_bars() -> u32 {
    56
}
fn default_height() -> u32 {
    16
}
fn default_color1() -> [u8; 3] {
    [120, 190, 255]
}
fn default_color2() -> [u8; 3] {
    [196, 120, 255]
}
fn default_alpha() -> f32 {
    0.88
}

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ParallaxUniform {
    mouse: [f32; 2],
    aspect: f32,
    strength: f32,
    zoom: f32,
    has_depth: f32,
    image_size: [f32; 2],
    _pad: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpectrumUniform {
    bars: [[f32; 4]; 16],
    color1: [f32; 4],
    color2: [f32; 4],
    // bar count, alpha, time, unused
    params: [f32; 4],
}

const PARALLAX_WGSL: &str = r#"
struct Uniforms {
    mouse: vec2<f32>,
    aspect: f32,
    strength: f32,
    zoom: f32,
    has_depth: f32,
    image_size: vec2<f32>,
    _pad: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var image_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, -p.y * 0.5 + 0.5);
    return out;
}

fn cover_uv(uv: vec2<f32>, screen_aspect: f32, image_size: vec2<f32>) -> vec2<f32> {
    let image_aspect = image_size.x / max(image_size.y, 1.0);
    var p = uv - vec2<f32>(0.5);
    if (screen_aspect > image_aspect) {
        p.y *= image_aspect / screen_aspect;
    } else {
        p.x *= screen_aspect / image_aspect;
    }
    return p + vec2<f32>(0.5);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var uv = cover_uv(in.uv, u.aspect, u.image_size);

    // Depth Anything returns inverse depth: bright = close to the camera.
    let sampled_depth = textureSample(depth_tex, samp, uv).r;
    let depth = mix(0.5, sampled_depth, u.has_depth);
    let amount = mix(0.35, 1.0, depth) * u.strength;
    let shift = vec2<f32>(u.mouse.x, -u.mouse.y) * amount * 0.055;
    uv = (uv - vec2<f32>(0.5)) / u.zoom + vec2<f32>(0.5) + shift / u.zoom;

    let color = textureSample(image_tex, samp, uv).rgb;

    // Subtle vignette keeps the displaced edges from looking flat.
    let centered = (in.uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 0.85);
    let vignette = 1.0 - smoothstep(0.45, 0.92, length(centered)) * 0.28;
    return vec4<f32>(color * vignette, 1.0);
}
"#;

const SPECTRUM_WGSL: &str = r#"
struct Uniforms {
    bars: array<vec4<f32>, 16>,
    color1: vec4<f32>,
    color2: vec4<f32>,
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, -p.y * 0.5 + 0.5);
    return out;
}

fn bar_height(index: u32) -> f32 {
    let v = u.bars[index / 4u];
    let c = index % 4u;
    if (c == 0u) { return v.x; }
    if (c == 1u) { return v.y; }
    if (c == 2u) { return v.z; }
    return v.w;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let count = max(u.params.x, 1.0);
    let x = in.uv.x * count;
    let index = min(u32(x), u32(count) - 1u);
    let height = bar_height(index);
    if (height <= 0.001) { discard; }

    let local = fract(x);
    let gap = 0.24;
    let half_width = 0.5 - gap * 0.5;
    let dx = abs(local - 0.5);
    if (dx > half_width) { discard; }

    let y = 1.0 - in.uv.y;              // 0 at the bottom edge
    let bottom = 0.035;
    let top = bottom + height * 0.90;
    if (y < bottom || y > top) { discard; }

    let edge_x = 1.0 - smoothstep(half_width - 0.035, half_width, dx);
    let edge_y = 1.0 - smoothstep(top - 0.012, top, y);
    let alpha = u.params.y * edge_x * edge_y;

    let t = clamp((y - bottom) / max(top - bottom, 0.001), 0.0, 1.0);
    let color = mix(u.color1.rgb, u.color2.rgb, t);
    let output_color = select(color, color * alpha, u.params.w > 0.5);
    return vec4<f32>(output_color, alpha);
}
"#;

struct Gpu {
    _instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

// ---------------------------------------------------------------------------
// Surfaces
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Parallax,
    Spectrum,
}

struct EffectSurface {
    kind: Kind,
    layer: LayerSurface,
    wsurf: wgpu::Surface<'static>,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    configured: bool,
    size: (u32, u32),
    scale: u32,
    last_drawn: Instant,
    image_size: (f32, f32),
    strength: f32,
    has_depth: bool,
    color1: [f32; 4],
    color2: [f32; 4],
    alpha: f32,
}

struct App {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell: LayerShell,
    raw_compositor: wl_compositor::WlCompositor,
    gpu: Option<Gpu>,
    surfaces: Vec<EffectSurface>,
    bars_rx: Option<Receiver<Vec<f32>>>,
    cava: Option<Child>,
    cava_config: Option<PathBuf>,
    levels: Vec<f32>,
    last_active_at: Instant,
    parent_pid: Option<u32>,
    last_parent_check: Instant,
}

fn main() {
    let mut config_path: Option<PathBuf> = None;
    let mut parent_pid: Option<u32> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_path = args.next().map(PathBuf::from),
            "--parent-pid" => parent_pid = args.next().and_then(|v| v.parse().ok()),
            other => eprintln!("aura-effects: ignoring unknown argument {other}"),
        }
    }

    let Some(config_path) = config_path else {
        eprintln!("aura-effects: --config <path> is required");
        std::process::exit(2);
    };

    let raw = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!("aura-effects: cannot read {}: {e}", config_path.display());
        std::process::exit(2);
    });
    let config: EffectsConfig = serde_json::from_str(&raw).unwrap_or_else(|e| {
        eprintln!("aura-effects: invalid config: {e}");
        std::process::exit(2);
    });

    if config.outputs.is_empty() {
        eprintln!("aura-effects: nothing to draw");
        std::process::exit(0);
    }

    if let Err(error) = run(config, parent_pid) {
        eprintln!("aura-effects: {error}");
        std::process::exit(1);
    }
}

fn run(config: EffectsConfig, parent_pid: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh: QueueHandle<App> = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let raw_compositor = globals.bind::<wl_compositor::WlCompositor, App, ()>(&qh, 1..=6, ())?;
    let output_state = OutputState::new(&globals, &qh);
    let seat_state = SeatState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);

    let mut app = App {
        registry_state,
        seat_state,
        output_state,
        compositor_state,
        layer_shell,
        raw_compositor,
        gpu: None,
        surfaces: Vec::new(),
        bars_rx: None,
        cava: None,
        cava_config: None,
        levels: vec![0.0; 56],
        last_active_at: Instant::now(),
        parent_pid,
        last_parent_check: Instant::now(),
    };

    // Populate the output list (seat capabilities are picked up too).
    event_queue.roundtrip(&mut app)?;

    let display_ptr = conn.backend().display_ptr() as *mut _;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    // Build every surface first, then create the adapter with one of them.
    let mut pending = Vec::new();
    for output_cfg in &config.outputs {
        let Some(output) = find_output(&app.output_state, &output_cfg.name) else {
            eprintln!(
                "aura-effects: output {} not found (available: {:?})",
                output_cfg.name,
                output_names(&app.output_state)
            );
            continue;
        };
        let (scale, logical_size) = app
            .output_state
            .info(&output)
            .map(|info| {
                let scale = info.scale_factor.max(1);
                let size = info
                    .logical_size
                    .or_else(|| {
                        info.modes.first().map(|mode| {
                            (
                                mode.dimensions.0 / scale.max(1),
                                mode.dimensions.1 / scale.max(1),
                            )
                        })
                    })
                    .unwrap_or((1920, 1080));
                (scale as u32, size)
            })
            .unwrap_or((1, (1920, 1080)));

        if let Some(parallax) = &output_cfg.parallax {
            let surface = app.compositor_state.create_surface(&qh);
            let layer = app.layer_shell.create_layer_surface(
                &qh,
                surface,
                Layer::Bottom,
                Some("aura-parallax"),
                Some(&output),
            );
            layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_exclusive_zone(-1);
            layer.set_size(0, 0);
            let _ = layer.set_buffer_scale(scale);

            // Fully click-through: the wallpaper must never steal desktop input.
            let region = app.raw_compositor.create_region(&qh, ());
            layer.set_input_region(Some(&region));
            layer.commit();
            region.destroy();

            let image = load_rgba(&parallax.image)?;
            let (iw, ih) = (image.0, image.1);
            let depth = parallax
                .depth
                .as_ref()
                .and_then(|path| load_depth(path).ok());
            pending.push(PendingSurface {
                kind: Kind::Parallax,
                layer,
                scale,
                image: Some(image),
                depth,
                image_size: (iw as f32, ih as f32),
                strength: parallax.strength,
                color1: [0.0; 4],
                color2: [0.0; 4],
                alpha: 0.0,
            });
        }

        if let Some(spectrum) = &output_cfg.spectrum {
            let height = ((logical_size.1 as f32) * (spectrum.height_pct as f32 / 100.0))
                .clamp(40.0, logical_size.1 as f32) as u32;
            let surface = app.compositor_state.create_surface(&qh);
            let layer = app.layer_shell.create_layer_surface(
                &qh,
                surface,
                Layer::Bottom,
                Some("aura-spectrum"),
                Some(&output),
            );
            layer.set_anchor(Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_exclusive_zone(-1);
            layer.set_size(0, height);
            let _ = layer.set_buffer_scale(scale);

            // Fully click-through: an empty input region.
            let region = app.raw_compositor.create_region(&qh, ());
            layer.set_input_region(Some(&region));
            layer.commit();
            region.destroy();

            pending.push(PendingSurface {
                kind: Kind::Spectrum,
                layer,
                scale,
                image: None,
                depth: None,
                image_size: (1.0, 1.0),
                strength: 0.0,
                color1: srgb(spectrum.color1),
                color2: srgb(spectrum.color2),
                alpha: spectrum.alpha.clamp(0.0, 1.0),
            });
        }
    }

    if pending.is_empty() {
        return Err("no matching outputs".into());
    }

    // Create wgpu surfaces from the raw Wayland handles.
    let mut created: Vec<(PendingSurface, wgpu::Surface<'static>)> = Vec::new();
    for p in pending {
        let wh = WaylandWindowHandle::new(
            NonNull::new(p.layer.wl_surface().id().as_ptr() as *mut _).ok_or("no wl_surface")?,
        );
        let wsurf = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                    NonNull::new(display_ptr).ok_or("no wl_display")?,
                )),
                raw_window_handle: RawWindowHandle::Wayland(wh),
            })?
        };
        created.push((p, wsurf));
    }

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: Some(&created[0].1),
        force_fallback_adapter: false,
    }))?;
    let adapter_info = adapter.get_info();
    eprintln!("aura-effects: adapter {} ({:?})", adapter_info.name, adapter_info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("aura-effects"),
        ..Default::default()
    }))?;

    let gpu = Gpu {
        _instance: instance,
        adapter,
        device,
        queue,
    };

    let mut surfaces = Vec::new();
    for (pending, wsurf) in created {
        let caps = wsurf.get_capabilities(&gpu.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(caps.formats[0]);
        let alpha_mode = if pending.kind == Kind::Spectrum {
            caps.alpha_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::CompositeAlphaMode::PreMultiplied)
                .or_else(|| {
                    caps.alpha_modes
                        .iter()
                        .copied()
                        .find(|m| *m == wgpu::CompositeAlphaMode::PostMultiplied)
                })
                .unwrap_or(caps.alpha_modes[0])
        } else {
            wgpu::CompositeAlphaMode::Opaque
        };

        surfaces.push(create_effect_surface(
            &gpu, pending, wsurf, format, alpha_mode,
        )?);
    }

    let bars_count = config
        .outputs
        .iter()
        .find_map(|output| output.spectrum.as_ref().map(|s| s.bars))
        .unwrap_or(56)
        .clamp(8, 64);
    let (bars_rx, cava, cava_config) = if surfaces.iter().any(|s| s.kind == Kind::Spectrum) {
        match spawn_cava(bars_count) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("aura-effects: cava unavailable: {error}");
                (None, None, None)
            }
        }
    } else {
        (None, None, None)
    };

    app.gpu = Some(gpu);
    app.surfaces = surfaces;
    app.bars_rx = bars_rx;
    app.cava = cava;
    app.cava_config = cava_config;
    app.levels = vec![0.0; bars_count as usize];

    // Let the compositor send the initial configure events.
    event_queue.roundtrip(&mut app)?;
    app.reconfigure_all();

    let has_spectrum = app.surfaces.iter().any(|s| s.kind == Kind::Spectrum);
    let started = Instant::now();
    let mut last_draw = Instant::now();

    loop {
        // Dispatch Wayland events with a short timeout so rendering can be
        // paced without a separate thread.
        event_queue.flush()?;
        if let Some(guard) = event_queue.prepare_read() {
            let fd = guard.connection_fd();
            let timeout = if has_spectrum { 16 } else { 120 };
            let mut pollfd = libc::pollfd {
                fd: fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut pollfd, 1, timeout) } > 0
                && (pollfd.revents & libc::POLLIN) != 0;
            if ready {
                let _ = guard.read();
            }
        }
        event_queue.dispatch_pending(&mut app)?;

        if app.last_parent_check.elapsed() > Duration::from_secs(1) {
            app.last_parent_check = Instant::now();
            if let Some(pid) = app.parent_pid {
                if unsafe { libc::kill(pid as i32, 0) } != 0 {
                    app.shutdown();
                    return Ok(());
                }
            }
        }

        app.poll_levels();
        if has_spectrum {
            app.apply_idle_levels(started.elapsed().as_secs_f32());
        }

        let now = Instant::now();
        if now.duration_since(last_draw) >= Duration::from_millis(16) {
            last_draw = now;
            app.draw_all(started.elapsed().as_secs_f32());
        }
    }
}

// ---------------------------------------------------------------------------
// Surface creation
// ---------------------------------------------------------------------------

struct PendingSurface {
    kind: Kind,
    layer: LayerSurface,
    scale: u32,
    image: Option<(u32, u32, Vec<u8>)>,
    depth: Option<(u32, u32, Vec<u8>)>,
    image_size: (f32, f32),
    strength: f32,
    color1: [f32; 4],
    color2: [f32; 4],
    alpha: f32,
}

fn create_effect_surface(
    gpu: &Gpu,
    pending: PendingSurface,
    wsurf: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
) -> Result<EffectSurface, Box<dyn std::error::Error>> {
    let uniform_size = if pending.kind == Kind::Parallax {
        std::mem::size_of::<ParallaxUniform>() as u64
    } else {
        std::mem::size_of::<SpectrumUniform>() as u64
    };
    let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("aura-effects uniform"),
        size: uniform_size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let (shader, bind_layouts) = match pending.kind {
        Kind::Parallax => (
            PARALLAX_WGSL,
            vec![
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        ),
        Kind::Spectrum => (
            SPECTRUM_WGSL,
            vec![wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        ),
    };

    let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("aura-effects shader"),
        source: wgpu::ShaderSource::Wgsl(shader.into()),
    });

    let bind_group_layout = gpu
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aura-effects layout"),
            entries: &bind_layouts,
        });

    let bind_group = if let Some((w, h, data)) = &pending.image {
        let image_view = create_texture(
            &gpu.device,
            &gpu.queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            *w,
            *h,
            data,
        );
        let (dw, dh, ddata) = pending.depth.clone().unwrap_or((1, 1, vec![128]));
        let depth_view = create_texture(
            &gpu.device,
            &gpu.queue,
            wgpu::TextureFormat::R8Unorm,
            dw,
            dh,
            &ddata,
        );
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("aura-effects sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aura-effects bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&image_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    } else {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aura-effects bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        })
    };

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aura-effects pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

    let pipeline = gpu
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aura-effects pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

    let surface = EffectSurface {
        kind: pending.kind,
        layer: pending.layer,
        wsurf,
        pipeline,
        bind_group,
        uniform,
        format,
        alpha_mode,
        configured: false,
        size: (1, 1),
        scale: pending.scale,
        last_drawn: Instant::now(),
        image_size: pending.image_size,
        strength: pending.strength,
        has_depth: pending.depth.is_some(),
        color1: pending.color1,
        color2: pending.color2,
        alpha: pending.alpha,
    };
    Ok(surface)
}

fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    data: &[u8],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("aura-effects texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bytes_per_pixel = if format == wgpu::TextureFormat::R8Unorm {
        1
    } else {
        4
    };
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * bytes_per_pixel),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

impl EffectSurface {
    fn configure(&mut self, gpu: &Gpu) {
        if self.size.0 == 0 || self.size.1 == 0 {
            return;
        }
        let phys_w = self.size.0.saturating_mul(self.scale.max(1));
        let phys_h = self.size.1.saturating_mul(self.scale.max(1));
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            width: phys_w.max(1),
            height: phys_h.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: self.alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        self.wsurf.configure(&gpu.device, &config);
        self.configured = true;
    }
}

impl App {
    fn reconfigure_all(&mut self) {
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        for surface in &mut self.surfaces {
            surface.configure(gpu);
        }
    }

    fn shutdown(&mut self) {
        if let Some(child) = self.cava.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(path) = self.cava_config.as_ref() {
            let _ = std::fs::remove_file(path);
        }
    }

    fn poll_levels(&mut self) {
        let Some(rx) = self.bars_rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(values) => {
                    let has_signal = values.iter().any(|value| *value > 0.01);
                    if self.levels.len() != values.len() {
                        self.levels.resize(values.len(), 0.0);
                    }
                    for (level, value) in self.levels.iter_mut().zip(values) {
                        // Fast attack, slow release; cava already smooths a bit.
                        if value > *level {
                            *level = value;
                        } else {
                            *level = (*level * 0.86).max(value * 0.35);
                        }
                    }
                    if has_signal {
                        self.last_active_at = Instant::now();
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.bars_rx = None;
                    break;
                }
            }
        }
    }

    fn apply_idle_levels(&mut self, elapsed: f32) {
        if self.last_active_at.elapsed() < Duration::from_secs(2) {
            return;
        }
        for (index, level) in self.levels.iter_mut().enumerate() {
            let wave = ((elapsed * 1.6) + index as f32 * 0.35).sin() * 0.5 + 0.5;
            *level = 0.06 + wave * 0.05;
        }
    }

    fn draw_all(&mut self, elapsed: f32) {
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        let levels = &self.levels;
        for surface in &mut self.surfaces {
            if !surface.configured {
                surface.configure(gpu);
            }
            if !surface.configured {
                continue;
            }
            let interval = if surface.kind == Kind::Parallax {
                Duration::from_millis(33)
            } else {
                Duration::from_millis(16)
            };
            if surface.last_drawn.elapsed() < interval {
                continue;
            }
            draw_surface(gpu, surface, levels, elapsed);
            surface.last_drawn = Instant::now();
        }
    }
}

fn draw_surface(gpu: &Gpu, surface: &mut EffectSurface, levels: &[f32], elapsed: f32) {
    let frame = match surface.wsurf.get_current_texture() {
        Ok(frame) => frame,
        Err(wgpu::SurfaceError::Timeout) => return,
        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
            surface.configure(gpu);
            return;
        }
        Err(error) => {
            eprintln!("aura-effects: surface error: {error:?}");
            return;
        }
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("aura-effects encoder"),
        });

    let clear = if surface.kind == Kind::Spectrum {
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    } else {
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    };

    match surface.kind {
        Kind::Parallax => {
            let aspect = surface.size.0 as f32 / surface.size.1.max(1) as f32;
            // Slow Lissajous drift: a 2.5D wallpaper that keeps moving without
            // requiring pointer events (and therefore without stealing clicks).
            let drift = [
                (elapsed * 0.13).sin() * 0.60 + (elapsed * 0.047).sin() * 0.20,
                (elapsed * 0.09).cos() * 0.45 + (elapsed * 0.031).cos() * 0.15,
            ];
            let uniform = ParallaxUniform {
                mouse: drift,
                aspect,
                strength: surface.strength,
                zoom: 1.07,
                has_depth: if surface.has_depth { 1.0 } else { 0.0 },
                image_size: [surface.image_size.0, surface.image_size.1],
                _pad: [0.0; 4],
            };
            gpu.queue
                .write_buffer(&surface.uniform, 0, bytemuck::bytes_of(&uniform));
        }
        Kind::Spectrum => {
            let mut bars = [[0.0f32; 4]; 16];
            for (i, level) in levels.iter().take(64).enumerate() {
                bars[i / 4][i % 4] = level.clamp(0.0, 1.0);
            }
            let uniform = SpectrumUniform {
                bars,
                color1: surface.color1,
                color2: surface.color2,
                params: [
                    levels.len().clamp(1, 64) as f32,
                    surface.alpha,
                    elapsed,
                    if surface.alpha_mode == wgpu::CompositeAlphaMode::PreMultiplied {
                        1.0
                    } else {
                        0.0
                    },
                ],
            };
            gpu.queue
                .write_buffer(&surface.uniform, 0, bytemuck::bytes_of(&uniform));
        }
    }

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("aura-effects pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&surface.pipeline);
        pass.set_bind_group(0, &surface.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    gpu.queue.submit([encoder.finish()]);
    frame.present();
}

fn srgb(rgb: [u8; 3]) -> [f32; 4] {
    let convert = |v: u8| {
        let c = v as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [convert(rgb[0]), convert(rgb[1]), convert(rgb[2]), 1.0]
}

fn load_rgba(path: &str) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
    let image = image::open(path)?.to_rgba8();
    let (w, h) = image.dimensions();
    Ok((w, h, image.into_raw()))
}

fn load_depth(path: &str) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
    let image = image::open(path)?.to_luma8();
    let (w, h) = image.dimensions();
    Ok((w, h, image.into_raw()))
}

fn find_output(state: &OutputState, name: &str) -> Option<wl_output::WlOutput> {
    state.outputs().find(|output| {
        state
            .info(output)
            .map(|info| info.name.as_deref() == Some(name))
            .unwrap_or(false)
    })
}

fn output_names(state: &OutputState) -> Vec<String> {
    state
        .outputs()
        .filter_map(|output| state.info(&output).and_then(|i| i.name))
        .collect()
}

fn logs_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?
        .join("aura")
        .join("logs");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

fn open_log(name: &str) -> Option<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs_dir()?.join(name))
        .ok()
}

/// Cava must listen to the monitor of the default sink, otherwise it samples
/// the microphone while the desktop plays music.
fn default_sink_monitor() -> Option<String> {
    let output = Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sink.is_empty() {
        None
    } else {
        Some(format!("{sink}.monitor"))
    }
}

fn spawn_cava(
    bars: u32,
) -> Result<
    (
        Option<Receiver<Vec<f32>>>,
        Option<Child>,
        Option<PathBuf>,
    ),
    Box<dyn std::error::Error>,
> {
    let path = std::env::temp_dir().join(format!("aura-cava-{}.conf", std::process::id()));
    let source = default_sink_monitor().unwrap_or_else(|| "auto".to_string());
    let conf = format!(
        "[general]\nbars = {bars}\nframerate = 60\n\n[input]\nmethod = pulse\nsource = {source}\n\n[output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\nascii_max_range = 1000\nbar_delimiter = 59\nframe_delimiter = 10\n"
    );
    std::fs::write(&path, conf)?;

    let mut log = open_log("cava.log");
    if let Some(log) = log.as_mut() {
        let _ = writeln!(log, "starting cava with source={source}");
    }

    let mut child = Command::new("cava")
        .arg("-p")
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(match log {
            Some(file) => Stdio::from(file),
            None => Stdio::null(),
        })
        .stdin(Stdio::null())
        .spawn()
        .map_err(|error| {
            if let Some(mut log) = open_log("cava.log") {
                let _ = writeln!(log, "cava could not start: {error}");
            }
            error
        })?;

    let stdout = child.stdout.take().ok_or("cava stdout unavailable")?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let values: Vec<f32> = line
                .split(';')
                .filter_map(|part| part.trim().parse::<f32>().ok())
                .map(|value| (value / 1000.0).clamp(0.0, 1.0))
                .collect();
            if !values.is_empty() && tx.send(values).is_err() {
                break;
            }
        }
    });

    Ok((Some(rx), Some(child), Some(path)))
}

// ---------------------------------------------------------------------------
// Wayland handlers
// ---------------------------------------------------------------------------

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.shutdown();
        std::process::exit(0);
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        serial: u32,
    ) {
        // SCTK acknowledges the protocol configure before calling this hook.
        let _ = serial;
        let index = self
            .surfaces
            .iter()
            .position(|s| s.layer.wl_surface().id() == layer.wl_surface().id());
        let Some(index) = index else { return };
        let (w, h) = configure.new_size;
        if w == 0 || h == 0 {
            return;
        }
        let gpu = self.gpu.as_ref();
        let Some(gpu) = gpu else { return };
        let surface = &mut self.surfaces[index];
        if surface.size != (w, h) {
            surface.size = (w, h);
            surface.configure(gpu);
        }
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
}

impl Dispatch<wl_compositor::WlCompositor, ()> for App {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        unreachable!("wl_compositor has no events")
    }
}

impl Dispatch<wl_region::WlRegion, ()> for App {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        unreachable!("wl_region has no events")
    }
}

delegate_compositor!(App);
delegate_output!(App);
delegate_seat!(App);
delegate_layer!(App);
delegate_registry!(App);
