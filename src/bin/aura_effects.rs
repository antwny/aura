//! Aura's lightweight effects renderer.
//!
//! A separate process (like `mpvpaper`) so it can own its own Wayland
//! connection and create one layer-shell surface per monitor:
//!
//! * `parallax`: a click-through bottom-layer image with mouse-driven depth.
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

#[path = "../effects_cursor.rs"]
mod effects_cursor;

use effects_cursor::CursorTracker;
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

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn request_shutdown(_: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum SpectrumPosition {
    #[default]
    Bottom,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum SpectrumStyle {
    #[default]
    Bars,
    Wave,
    Dots,
    Mirror,
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
    #[serde(default = "default_halo")]
    halo: [u8; 3],
    #[serde(default = "default_alpha")]
    alpha: f32,
    #[serde(default)]
    position: SpectrumPosition,
    #[serde(default)]
    style: SpectrumStyle,
    #[serde(default = "default_bar_width")]
    bar_width_pct: u32,
    #[serde(default = "default_glow")]
    glow: bool,
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
fn default_halo() -> [u8; 3] {
    [255, 255, 255]
}
fn default_alpha() -> f32 {
    0.88
}
fn default_bar_width() -> u32 {
    76
}
fn default_glow() -> bool {
    true
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
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpectrumUniform {
    bars: [[f32; 4]; 16],
    color1: [f32; 4],
    color2: [f32; 4],
    halo: [f32; 4],
    // bar count, alpha, time, premultiplied flag
    params: [f32; 4],
    // half width fraction, glow flag, top-anchored flag, style code
    shape: [f32; 4],
    // strip aspect ratio, unused
    extra: [f32; 4],
}

const PARALLAX_WGSL: &str = r#"
struct Uniforms {
    mouse: vec2<f32>,
    aspect: f32,
    strength: f32,
    zoom: f32,
    has_depth: f32,
    image_size: vec2<f32>,
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
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

fn depth_at(uv: vec2<f32>) -> f32 {
    return mix(0.5, textureSampleLevel(depth_tex, samp, uv, 0.0).r, u.has_depth);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5);
    let base = (cover_uv(in.uv, u.aspect, u.image_size) - center) / u.zoom + center;
    let coverage = (cover_uv(vec2<f32>(1.0), u.aspect, u.image_size) - center) * 2.0;
    let offset = u.mouse * coverage * u.strength * 0.075 / u.zoom;
    let pixels = length(u.mouse * u.screen_size * u.strength * 0.075);
    let steps = clamp(ceil(pixels / 2.5), 4.0, 40.0);
    let step_uv = offset / steps;
    let step_depth = 1.0 / steps;
    var uv = base + offset * 0.5;
    var ray_depth = 1.0;
    var depth = depth_at(uv);
    var previous_uv = uv;
    var previous_gap = ray_depth - depth;

    // Inverse depth puts the closest surface first along the camera ray.
    for (var i = 0u; i < 40u; i += 1u) {
        if (f32(i) >= steps || ray_depth <= depth) { break; }
        previous_uv = uv;
        previous_gap = ray_depth - depth;
        uv -= step_uv;
        ray_depth -= step_depth;
        depth = depth_at(uv);
    }
    let gap = depth - ray_depth;
    let weight = clamp(gap / max(gap + previous_gap, 0.00001), 0.0, 1.0);
    uv = mix(uv, previous_uv, weight);
    return vec4<f32>(textureSampleLevel(image_tex, samp, clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb, 1.0);
}
"#;

const SPECTRUM_WGSL: &str = r#"
struct Uniforms {
    bars: array<vec4<f32>, 16>,
    color1: vec4<f32>,
    color2: vec4<f32>,
    halo: vec4<f32>,
    params: vec4<f32>,
    shape: vec4<f32>,
    extra: vec4<f32>,
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

fn bar_height_at(x: f32, count: f32) -> f32 {
    let last = u32(count) - 1u;
    let clamped = clamp(x, 0.0, count - 0.0001);
    let index = min(u32(clamped), last);
    let next = min(index + 1u, last);
    let t = smoothstep(0.0, 1.0, fract(clamped));
    return mix(bar_height(index), bar_height(next), t);
}

fn halo_color(alpha: f32) -> vec4<f32> {
    let rgb = select(u.halo.rgb, u.halo.rgb * alpha, u.params.w > 0.5);
    return vec4<f32>(rgb, alpha);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let count = max(u.params.x, 1.0);
    let x = in.uv.x * count;
    let y = select(1.0 - in.uv.y, in.uv.y, u.shape.z > 0.5);
    let bottom = 0.035;
    let halo_width = 0.020;

    // 0: bars.
    if (u.shape.w < 0.5) {
        let index = min(u32(x), u32(count) - 1u);
        let height = bar_height(index);
        if (height <= 0.001) { discard; }

        let local = fract(x);
        let half_width = clamp(u.shape.x, 0.04, 0.5);
        let dx = abs(local - 0.5);
        let top = bottom + height * 0.90;

        if (dx <= half_width && y >= bottom && y <= top) {
            let edge_x = 1.0 - smoothstep(half_width - 0.035, half_width, dx);
            let edge_y = 1.0 - smoothstep(top - 0.012, top, y);
            let alpha = u.params.y * edge_x * edge_y;
            let t = clamp((y - bottom) / max(top - bottom, 0.001), 0.0, 1.0);
            let color = mix(u.color1.rgb, u.color2.rgb, t);
            let output_color = select(color, color * alpha, u.params.w > 0.5);
            return vec4<f32>(output_color, alpha);
        }

        if (u.shape.y > 0.5) {
            let outside_x = max(dx - half_width, 0.0);
            let outside_y = max(max(bottom - y, y - top), 0.0);
            let distance = max(outside_x, outside_y);
            if (distance < halo_width) {
                let alpha = u.halo.a * u.params.y * (1.0 - smoothstep(0.0, halo_width, distance));
                return halo_color(alpha);
            }
        }
        discard;
        return vec4<f32>(0.0);
    }

    // 1: wave.
    if (u.shape.w < 1.5) {
        let height = bar_height_at(x, count);
        if (height <= 0.001) { discard; }
        let top = bottom + height * 0.90;

        if (y >= bottom && y <= top) {
            let edge_y = 1.0 - smoothstep(top - 0.010, top, y);
            let alpha = u.params.y * edge_y;
            let t = clamp((y - bottom) / max(top - bottom, 0.001), 0.0, 1.0);
            var color = mix(u.color1.rgb, u.color2.rgb, t);
            let crest = 1.0 - smoothstep(0.0, 0.016, abs(y - top));
            color = mix(color, vec3<f32>(1.0), crest * 0.35);
            let output_color = select(color, color * alpha, u.params.w > 0.5);
            return vec4<f32>(output_color, alpha);
        }

        if (u.shape.y > 0.5) {
            let distance = max(max(bottom - y, y - top), 0.0);
            if (distance < halo_width) {
                let alpha = u.halo.a * u.params.y * (1.0 - smoothstep(0.0, halo_width, distance));
                return halo_color(alpha);
            }
        }
        discard;
        return vec4<f32>(0.0);
    }

    // 2: dots. Aspect correction keeps them round on wide strips.
    if (u.shape.w < 2.5) {
        let index = min(u32(x), u32(count) - 1u);
        let height = bar_height(index);
        if (height <= 0.001) { discard; }
        let dx = abs(fract(x) - 0.5);
        let top = bottom + height * 0.90;
        let radius_x = clamp(u.shape.x * 0.95, 0.05, 0.5);
        let radius_y = max(radius_x * (u.extra.x / count), 0.012);
        let center_y = max(top - radius_y, bottom + radius_y);
        let nx = dx / radius_x;
        let ny = (y - center_y) / radius_y;
        let distance = sqrt(nx * nx + ny * ny);

        if (distance <= 1.0) {
            let alpha = u.params.y * (1.0 - smoothstep(0.86, 1.0, distance));
            let t = clamp((center_y - bottom) / max(height * 0.90, 0.001), 0.0, 1.0);
            let color = mix(u.color1.rgb, u.color2.rgb, t);
            let output_color = select(color, color * alpha, u.params.w > 0.5);
            return vec4<f32>(output_color, alpha);
        }

        if (u.shape.y > 0.5) {
            let halo_limit = 1.28;
            if (distance <= halo_limit) {
                let alpha = u.halo.a * u.params.y * (1.0 - smoothstep(1.0, halo_limit, distance));
                return halo_color(alpha);
            }
        }
        discard;
        return vec4<f32>(0.0);
    }

    // 3: mirrored bars growing from the strip center.
    let index = min(u32(x), u32(count) - 1u);
    let height = bar_height(index);
    if (height <= 0.001) { discard; }
    let half_width = clamp(u.shape.x, 0.04, 0.5);
    let dx = abs(fract(x) - 0.5);
    let half = height * 0.45;
    let center = 0.5;
    let top = center + half;
    let bot = center - half;

    if (dx <= half_width && y >= bot && y <= top) {
        let edge_x = 1.0 - smoothstep(half_width - 0.035, half_width, dx);
        let edge_top = 1.0 - smoothstep(top - 0.012, top, y);
        let edge_bot = smoothstep(bot, bot + 0.012, y);
        let alpha = u.params.y * edge_x * min(edge_top, edge_bot);
        let t = clamp(abs(y - center) / max(half, 0.001), 0.0, 1.0);
        let color = mix(u.color1.rgb, u.color2.rgb, t);
        let output_color = select(color, color * alpha, u.params.w > 0.5);
        return vec4<f32>(output_color, alpha);
    }

    if (u.shape.y > 0.5) {
        let outside_x = max(dx - half_width, 0.0);
        let outside_y = max(max(bot - y, y - top), 0.0);
        let distance = max(outside_x, outside_y);
        if (distance < halo_width) {
            let alpha = u.halo.a * u.params.y * (1.0 - smoothstep(0.0, halo_width, distance));
            return halo_color(alpha);
        }
    }

    discard;
    return vec4<f32>(0.0);
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
    output_name: String,
    cursor: [f32; 2],
    last_cursor_update: Instant,
    needs_redraw: bool,
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
    halo: [f32; 4],
    bar_half_width: f32,
    glow: f32,
    flip: f32,
    style: f32,
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
    bars_count: u32,
    last_cava_retry: Instant,
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
    // Leave through the render loop so cursor sessions close before the process exits.
    for signal in [libc::SIGTERM, libc::SIGINT] {
        unsafe {
            libc::signal(signal, request_shutdown as *const () as libc::sighandler_t);
        }
    }
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
        bars_count: 56,
        last_cava_retry: Instant::now(),
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
                output_name: output_cfg.name.clone(),
                layer,
                scale,
                image: Some(image),
                depth,
                image_size: (iw as f32, ih as f32),
                strength: parallax.strength,
                color1: [0.0; 4],
                color2: [0.0; 4],
                halo: [0.0; 4],
                bar_half_width: 0.38,
                glow: 0.0,
                flip: 0.0,
                style: 0.0,
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
            let anchor = match spectrum.position {
                SpectrumPosition::Bottom => Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                SpectrumPosition::Top => Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            };
            layer.set_anchor(anchor);
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
                output_name: output_cfg.name.clone(),
                layer,
                scale,
                image: None,
                depth: None,
                image_size: (1.0, 1.0),
                strength: 0.0,
                color1: srgb(spectrum.color1),
                color2: srgb(spectrum.color2),
                halo: srgb(spectrum.halo),
                bar_half_width: (spectrum.bar_width_pct.clamp(10, 100) as f32 / 100.0) * 0.5,
                glow: if spectrum.glow { 1.0 } else { 0.0 },
                flip: if spectrum.position == SpectrumPosition::Top {
                    1.0
                } else {
                    0.0
                },
                style: match spectrum.style {
                    SpectrumStyle::Bars => 0.0,
                    SpectrumStyle::Wave => 1.0,
                    SpectrumStyle::Dots => 2.0,
                    SpectrumStyle::Mirror => 3.0,
                },
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
    app.bars_count = bars_count;
    app.levels = vec![0.0; bars_count as usize];

    // Let the compositor send the initial configure events.
    event_queue.roundtrip(&mut app)?;
    app.reconfigure_all();

    let has_spectrum = app.surfaces.iter().any(|s| s.kind == Kind::Spectrum);
    let has_parallax = app.surfaces.iter().any(|s| s.kind == Kind::Parallax);
    let cursor = has_parallax.then(|| {
        CursorTracker::start(
            app.surfaces
                .iter()
                .filter(|surface| surface.kind == Kind::Parallax)
                .map(|surface| surface.output_name.clone())
                .collect(),
        )
    });
    let started = Instant::now();
    let mut last_draw = Instant::now();

    loop {
        if SHUTDOWN_REQUESTED.load(Ordering::Relaxed) {
            app.shutdown();
            return Ok(());
        }
        // Dispatch Wayland events with a short timeout so rendering can be
        // paced without a separate thread.
        event_queue.flush()?;
        if let Some(guard) = event_queue.prepare_read() {
            let fd = guard.connection_fd();
            let audio_active = app.last_active_at.elapsed() < Duration::from_millis(1500);
            let timeout = if has_parallax || (has_spectrum && audio_active) {
                16
            } else if has_spectrum {
                50
            } else {
                120
            };
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
        // cava can die when the audio server restarts (GNOME/KDE session
        // changes, PipeWire restarts, device hotplug). Without this the
        // spectrum stayed on the idle wave until the renderer was restarted.
        if has_spectrum
            && app.bars_rx.is_none()
            && app.last_cava_retry.elapsed() >= Duration::from_secs(10)
        {
            app.restart_cava();
        }
        if has_spectrum {
            app.apply_idle_levels(started.elapsed().as_secs_f32());
        }

        let active = app.last_active_at.elapsed() < Duration::from_millis(1500);
        let now = Instant::now();
        let frame_budget = if has_parallax || (has_spectrum && active) {
            16
        } else {
            50
        };
        if now.duration_since(last_draw) >= Duration::from_millis(frame_budget) {
            last_draw = now;
            app.draw_all(started.elapsed().as_secs_f32(), active, cursor.as_ref());
        }
    }
}

// ---------------------------------------------------------------------------
// Surface creation
// ---------------------------------------------------------------------------

struct PendingSurface {
    kind: Kind,
    output_name: String,
    layer: LayerSurface,
    scale: u32,
    image: Option<(u32, u32, Vec<u8>)>,
    depth: Option<(u32, u32, Vec<u8>)>,
    image_size: (f32, f32),
    strength: f32,
    color1: [f32; 4],
    color2: [f32; 4],
    halo: [f32; 4],
    bar_half_width: f32,
    glow: f32,
    flip: f32,
    style: f32,
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
        output_name: pending.output_name,
        cursor: [0.0; 2],
        last_cursor_update: Instant::now(),
        needs_redraw: true,
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
        halo: pending.halo,
        bar_half_width: pending.bar_half_width,
        glow: pending.glow,
        flip: pending.flip,
        style: pending.style,
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
        self.needs_redraw = true;
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

    fn restart_cava(&mut self) {
        self.last_cava_retry = Instant::now();
        if find_cava_binary().is_none() {
            return;
        }
        if let Some(mut child) = self.cava.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(path) = self.cava_config.take() {
            let _ = std::fs::remove_file(path);
        }
        match spawn_cava(self.bars_count) {
            Ok((rx, child, config)) => {
                self.bars_rx = rx;
                self.cava = child;
                self.cava_config = config;
                if let Some(mut log) = open_log("cava.log") {
                    let _ = writeln!(log, "cava restarted");
                }
            }
            Err(error) => {
                if let Some(mut log) = open_log("cava.log") {
                    let _ = writeln!(log, "cava restart failed: {error}");
                }
            }
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

    fn draw_all(&mut self, elapsed: f32, audio_active: bool, cursor: Option<&CursorTracker>) {
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
            let interval = if surface.kind == Kind::Parallax || audio_active {
                Duration::from_millis(16)
            } else {
                Duration::from_millis(50)
            };
            let dt = surface.last_drawn.elapsed();
            if dt < interval {
                continue;
            }
            if surface.kind == Kind::Parallax {
                let target = cursor
                    .map(|tracker| tracker.position(&surface.output_name))
                    .unwrap_or([0.0; 2]);
                let moved = smooth_cursor(
                    &mut surface.cursor,
                    target,
                    surface.last_cursor_update.elapsed().as_secs_f32(),
                );
                surface.last_cursor_update = Instant::now();
                if !moved && !surface.needs_redraw {
                    continue;
                }
            }
            if draw_surface(gpu, surface, levels, elapsed) {
                surface.needs_redraw = false;
                surface.last_drawn = Instant::now();
            }
        }
    }
}

fn smooth_cursor(current: &mut [f32; 2], target: [f32; 2], dt: f32) -> bool {
    let previous = *current;
    let blend = 1.0 - (-8.0 * dt.min(0.1)).exp();
    for (value, target) in current.iter_mut().zip(target) {
        *value += (target - *value) * blend;
        if (target - *value).abs() < 0.0001 {
            *value = target;
        }
    }
    *current != previous
}

fn draw_surface(gpu: &Gpu, surface: &mut EffectSurface, levels: &[f32], elapsed: f32) -> bool {
    let frame = match surface.wsurf.get_current_texture() {
        Ok(frame) => frame,
        Err(wgpu::SurfaceError::Timeout) => return false,
        Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
            surface.configure(gpu);
            return false;
        }
        Err(error) => {
            eprintln!("aura-effects: surface error: {error:?}");
            return false;
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
            let uniform = ParallaxUniform {
                mouse: surface.cursor,
                aspect,
                strength: surface.strength,
                zoom: 1.0 + surface.strength * 0.1,
                has_depth: if surface.has_depth { 1.0 } else { 0.0 },
                image_size: [surface.image_size.0, surface.image_size.1],
                screen_size: [
                    surface.size.0.saturating_mul(surface.scale) as f32,
                    surface.size.1.saturating_mul(surface.scale) as f32,
                ],
                _pad: [0.0; 2],
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
                halo: [surface.halo[0], surface.halo[1], surface.halo[2], 0.55],
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
                shape: [surface.bar_half_width, surface.glow, surface.flip, surface.style],
                extra: [
                    surface.size.0 as f32 / surface.size.1.max(1) as f32,
                    0.0,
                    0.0,
                    0.0,
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
    true
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
    let image = prepare_depth_map(&image::open(path)?.to_luma8());
    let (w, h) = image.dimensions();
    Ok((w, h, image.into_raw()))
}

fn prepare_depth_map(depth: &image::GrayImage) -> image::GrayImage {
    let (width, height) = depth.dimensions();
    // Widen foreground silhouettes before filtering to reduce disocclusion tears.
    let dilated = image::GrayImage::from_fn(width, height, |x, y| {
        let mut nearest = 0;
        for ny in y.saturating_sub(3)..=y.saturating_add(3).min(height - 1) {
            for nx in x.saturating_sub(3)..=x.saturating_add(3).min(width - 1) {
                nearest = nearest.max(depth.get_pixel(nx, ny)[0]);
            }
        }
        image::Luma([nearest])
    });
    image::imageops::blur(&dilated, 1.2)
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

/// Locates `cava` beyond PATH so minimal sessions on GNOME/KDE that do not
/// export ~/.local/bin still find a user install.
fn find_cava_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("cava");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let mut candidates = vec![
        PathBuf::from("/usr/bin/cava"),
        PathBuf::from("/usr/local/bin/cava"),
        PathBuf::from("/app/bin/cava"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin/cava"));
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Cava must listen to the monitor of the default sink, otherwise it samples
/// the microphone while the desktop plays music. `pactl` is the most portable
/// probe; when it is missing or PulseAudio is absent, the first monitor source
/// reported by PipeWire is a better guess than the microphone.
fn default_sink_monitor() -> Option<String> {
    if let Some(sink) = pactl_default_sink() {
        return Some(format!("{sink}.monitor"));
    }
    pactl_first_monitor_source()
}

fn pactl_default_sink() -> Option<String> {
    let output = Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!sink.is_empty()).then_some(sink)
}

fn pactl_first_monitor_source() -> Option<String> {
    let output = Command::new("pactl")
        .args(["list", "short", "sources"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fallback = None;
    for line in text.lines() {
        let mut fields = line.split('\t');
        let _index = fields.next();
        let Some(name) = fields.next() else { continue };
        let name = name.trim();
        if name.ends_with(".monitor") {
            if name.contains("alsa_output") {
                return Some(name.to_string());
            }
            fallback.get_or_insert_with(|| name.to_string());
        }
    }
    fallback
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

    let binary = find_cava_binary().ok_or("cava executable not found")?;

    let mut log = open_log("cava.log");
    if let Some(log) = log.as_mut() {
        let _ = writeln!(
            log,
            "starting {} with source={source}",
            binary.display()
        );
    }

    let mut child = Command::new(&binary)
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
        SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationary_cursor_does_not_drift() {
        let mut cursor = [0.0; 2];
        for dt in [0.016, 0.1, 1.0, 60.0] {
            assert!(!smooth_cursor(&mut cursor, [0.0; 2], dt));
            assert_eq!(cursor, [0.0; 2]);
        }
    }

    #[test]
    fn cursor_smoothing_follows_both_axes_without_overshooting() {
        let mut cursor = [0.0; 2];
        let target = [1.0, -1.0];
        assert!(smooth_cursor(&mut cursor, target, 1.0 / 60.0));
        assert!(cursor[0] > 0.0 && cursor[0] < target[0]);
        assert!(cursor[1] < 0.0 && cursor[1] > target[1]);
        for _ in 0..120 {
            smooth_cursor(&mut cursor, target, 1.0 / 60.0);
        }
        assert_eq!(cursor, target);
        assert!(!smooth_cursor(&mut cursor, target, 1.0 / 60.0));
    }

    #[test]
    fn cursor_smoothing_is_frame_rate_independent() {
        let mut fast = [0.0; 2];
        let mut slow = [0.0; 2];
        for _ in 0..30 {
            smooth_cursor(&mut fast, [0.8, -0.4], 1.0 / 60.0);
        }
        for _ in 0..15 {
            smooth_cursor(&mut slow, [0.8, -0.4], 1.0 / 30.0);
        }
        for (fast, slow) in fast.into_iter().zip(slow) {
            assert!((fast - slow).abs() < 0.00001);
        }
    }

    #[test]
    fn cursor_returns_to_center() {
        let mut cursor = [1.0, -1.0];
        for _ in 0..120 {
            smooth_cursor(&mut cursor, [0.0; 2], 1.0 / 60.0);
        }
        assert_eq!(cursor, [0.0; 2]);
    }

    #[test]
    fn flat_depth_map_stays_flat() {
        let depth = image::GrayImage::from_pixel(16, 16, image::Luma([128]));
        assert_eq!(prepare_depth_map(&depth), depth);
    }

    #[test]
    fn foreground_depth_is_extended_before_blurring() {
        let mut depth = image::GrayImage::new(21, 21);
        depth.put_pixel(10, 10, image::Luma([255]));
        let prepared = prepare_depth_map(&depth);
        assert_eq!(prepared.dimensions(), depth.dimensions());
        assert!(prepared.get_pixel(10, 10)[0] > 240);
        assert!(prepared.get_pixel(13, 10)[0] > 100);
        assert_eq!(prepared.get_pixel(0, 0)[0], 0);
    }

    #[test]
    fn parallax_uniform_matches_shader_layout() {
        assert_eq!(std::mem::size_of::<ParallaxUniform>(), 48);
    }
}
