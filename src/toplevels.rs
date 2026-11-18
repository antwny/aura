//! Window-occlusion watcher used by the automatic pause feature.
//!
//! `mpvpaper -p` can only tell when the compositor stops sending frame
//! callbacks, which happens when the wallpaper is fully covered (maximized or
//! fullscreen). Tiling compositors leave part of the wallpaper visible, so
//! mpvpaper keeps decoding behind tiled windows. Older mpvpaper builds also
//! lack `--auto-mode`, so `-p` alone never fires for tiled or maximized
//! windows.
//!
//! Two backends publish, per output, how much of the wallpaper is covered:
//!
//! * COSMIC via its toplevel-info protocol on a dedicated Wayland connection.
//! * Niri via `niri msg --json event-stream` (workspace, window and layout
//!   events). Niri does not expose the COSMIC protocol, and its IPC is the
//!   only place with per-window sizes and output association.
//!
//! The UI turns the published coverage into per-monitor pauses through the
//! existing mpv IPC controls.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::engine::DesktopEnvironment;

use cosmic::cctk::{
    cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::State as ToplevelState,
    delegate_toplevel_info, delegate_workspace,
    sctk::{
        delegate_output, delegate_registry,
        output::{OutputHandler, OutputState},
        registry::{ProvidesRegistryState, RegistryState},
        registry_handlers,
    },
    toplevel_info::{ToplevelInfoHandler, ToplevelInfoState},
    wayland_client::{
        globals::registry_queue_init, protocol::wl_output, Connection, Proxy, QueueHandle,
    },
    wayland_protocols::{
        ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        ext::workspace::v1::client::ext_workspace_handle_v1,
    },
    workspace::{WorkspaceHandler, WorkspaceState},
};

/// Per-output occlusion data published by the watcher thread.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcclusionSnapshot {
    /// True when COSMIC's toplevel-info protocol was available.
    pub available: bool,
    /// Covered fraction (0.0..=1.0) keyed by output name.
    pub coverage: HashMap<String, f32>,
    /// Increments whenever the compositor publishes new window state. The UI
    /// uses it to keep manual resume overrides scoped to one window layout.
    pub revision: u64,
}

impl OcclusionSnapshot {
    pub fn coverage_for(&self, output: &str) -> f32 {
        self.coverage.get(output).copied().unwrap_or(0.0)
    }
}

static SNAPSHOT: OnceLock<Mutex<OcclusionSnapshot>> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();
static REVISION: AtomicU64 = AtomicU64::new(0);
static HEARTBEAT: AtomicU64 = AtomicU64::new(0);
static HEARTBEAT_RUNNING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Monotonic process-local clock in milliseconds. Wall-clock jumps do not
/// affect the watcher heartbeat.
fn heartbeat_clock() -> u64 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

/// Keeps the heartbeat alive while the watcher thread is running. Dropping the
/// guard stops it (also during unwind in debug builds).
struct HeartbeatGuard;

impl Drop for HeartbeatGuard {
    fn drop(&mut self) {
        HEARTBEAT_RUNNING.store(false, Ordering::Relaxed);
        HEARTBEAT.store(0, Ordering::Relaxed);
    }
}

fn heartbeat_start() -> HeartbeatGuard {
    HEARTBEAT_RUNNING.store(true, Ordering::Relaxed);
    HEARTBEAT.store(heartbeat_clock().max(1), Ordering::Relaxed);
    thread::Builder::new()
        .name("aura-occlusion-heartbeat".into())
        .spawn(|| {
            while HEARTBEAT_RUNNING.load(Ordering::Relaxed) {
                HEARTBEAT.store(heartbeat_clock().max(1), Ordering::Relaxed);
                thread::sleep(Duration::from_secs(1));
            }
        })
        .ok();
    HeartbeatGuard
}

/// False when the watcher thread died or stopped publishing for too long. The
/// UI then drops the last occlusion state instead of leaving the wallpaper
/// paused forever.
pub fn watcher_alive() -> bool {
    let last = HEARTBEAT.load(Ordering::Relaxed);
    HEARTBEAT_RUNNING.load(Ordering::Relaxed)
        && last != 0
        && heartbeat_clock().saturating_sub(last) < 5_000
}

fn snapshot_slot() -> &'static Mutex<OcclusionSnapshot> {
    SNAPSHOT.get_or_init(|| Mutex::new(OcclusionSnapshot::default()))
}

fn publish(mut snapshot: OcclusionSnapshot) {
    snapshot.revision = REVISION.fetch_add(1, Ordering::Relaxed) + 1;
    if let Ok(mut slot) = snapshot_slot().lock() {
        *slot = snapshot;
    }
}

/// Latest snapshot, or `None` when the watcher has not been started.
pub fn snapshot() -> Option<OcclusionSnapshot> {
    SNAPSHOT.get().map(|slot| {
        slot.lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    })
}

/// Starts the watcher once. Safe to call when the protocol is missing: the
/// thread publishes `available: false` and exits.
pub fn start() {
    STARTED.get_or_init(|| {
        let niri = DesktopEnvironment::detect() == DesktopEnvironment::Niri
            || std::env::var_os("NIRI_SOCKET").is_some();
        thread::Builder::new()
            .name("aura-toplevels".into())
            .spawn(move || loop {
                if !(if niri { run_niri_watcher() } else { run_watcher() }) {
                    break;
                }
                // A protocol or compositor error must not leave auto-pause
                // stuck on the last window layout. Reconnect and publish a
                // fresh snapshot as soon as the new connection is ready.
                eprintln!(
                    "[Aura] occlusion watcher disconnected; retrying in 750 ms"
                );
                thread::sleep(Duration::from_millis(750));
            })
            .ok();
    });
}

/// Returns `true` when the connection should be retried.
fn run_watcher() -> bool {
    let _heartbeat = heartbeat_start();
    let conn = match Connection::connect_to_env() {
        Ok(conn) => conn,
        Err(_) => {
            publish(OcclusionSnapshot::default());
            return false;
        }
    };
    let (globals, mut queue) = match registry_queue_init::<Watcher>(&conn) {
        Ok(value) => value,
        Err(_) => {
            publish(OcclusionSnapshot::default());
            return false;
        }
    };
    let qh = queue.handle();
    let registry = RegistryState::new(&globals);
    let output_state = OutputState::new(&globals, &qh);
    let Some(toplevel_info) = ToplevelInfoState::try_new(&registry, &qh) else {
        // Not COSMIC (or an older COSMIC without the protocol). Nothing to do;
        // mpvpaper's own `-p` remains the only mechanism.
        publish(OcclusionSnapshot::default());
        return false;
    };

    let workspace_state = WorkspaceState::new(&registry, &qh);
    let mut watcher = Watcher {
        registry,
        output_state,
        toplevel_info,
        workspace_state,
        last_signature: None,
    };
    if queue.roundtrip(&mut watcher).is_err() {
        publish(OcclusionSnapshot::default());
        return true;
    }
    watcher.recompute();

    // Event-driven: recompute after toplevel/output changes. The connection
    // also wakes us for unrelated registry events, which is cheap.
    loop {
        match queue.blocking_dispatch(&mut watcher) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("[Aura] occlusion watcher error: {err}");
                publish(OcclusionSnapshot::default());
                return true;
            }
        }
    }
}

/// Niri backend: consumes the compositor IPC event stream and derives, per
/// output, the fraction of the wallpaper covered by windows on the active
/// workspace. Returns `true` when the connection should be retried (niri was
/// probably restarting); returns `false` when the CLI or session is gone.
fn run_niri_watcher() -> bool {
    let _heartbeat = heartbeat_start();
    let outputs = match query_niri_outputs() {
        Ok(outputs) => outputs,
        Err(err) => {
            publish(OcclusionSnapshot::default());
            // At login the niri IPC socket may not be ready yet: keep
            // retrying. A missing CLI is permanent.
            return err.kind() != std::io::ErrorKind::NotFound;
        }
    };
    let mut command = std::process::Command::new("niri");
    command
        .args(["msg", "--json", "event-stream"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        // Do not outlive Aura: an orphaned client keeps the login autostart
        // unit alive (ExitType=cgroup) and leaks one process per restart.
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            publish(OcclusionSnapshot::default());
            return true;
        }
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        publish(OcclusionSnapshot::default());
        return false;
    };

    let mut state = NiriState {
        outputs,
        ..NiriState::default()
    };
    let reader = std::io::BufReader::new(stdout);
    for line in std::io::BufRead::lines(reader) {
        let Ok(line) = line else { break };
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        state.apply_event(&event);
        state.recompute();
    }

    let _ = child.kill();
    let _ = child.wait();
    publish(OcclusionSnapshot::default());
    true
}

/// Runs a `niri msg --json` query with a hard timeout. A compositor that
/// accepts the socket but never answers used to block the watcher thread
/// forever (leaving the last pause state in place).
fn run_niri_json(args: &[&str], timeout: Duration) -> std::io::Result<serde_json::Value> {
    use std::io::Read;

    let mut child = std::process::Command::new("niri")
        .args(["msg", "--json"])
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut buffer = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    stdout.read_to_string(&mut buffer)?;
                }
                if !status.success() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "niri msg failed",
                    ));
                }
                return serde_json::from_str(&buffer)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err));
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "niri msg timed out",
                ));
            }
        }
    }
}

fn query_niri_outputs() -> std::io::Result<HashMap<String, (f32, f32)>> {
    let value = run_niri_json(&["outputs"], Duration::from_secs(2))?;
    let Some(object) = value.as_object() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "niri outputs is not an object",
        ));
    };
    let mut outputs = HashMap::new();
    for (name, info) in object {
        let Some(logical) = info.get("logical") else {
            continue;
        };
        let (Some(width), Some(height)) = (
            logical.get("width").and_then(|value| value.as_f64()),
            logical.get("height").and_then(|value| value.as_f64()),
        ) else {
            continue;
        };
        if width > 0.0 && height > 0.0 {
            outputs.insert(name.clone(), (width as f32, height as f32));
        }
    }
    Ok(outputs)
}

#[derive(Debug, Default, Clone)]
struct NiriWindow {
    workspace_id: Option<u64>,
    is_floating: bool,
    layout: NiriLayout,
}

#[derive(Debug, Default, Clone)]
struct NiriLayout {
    /// 1-based column index in the scrolling layout.
    column: Option<usize>,
    tile_size: (f64, f64),
    /// Only set for floating windows in this niri version; tiled windows have
    /// no absolute position in the IPC layout.
    view_pos: Option<(f64, f64)>,
}

#[derive(Debug, Default, Clone)]
struct NiriWorkspace {
    output: Option<String>,
    is_active: bool,
}

#[derive(Default)]
struct NiriState {
    outputs: HashMap<String, (f32, f32)>,
    workspaces: HashMap<u64, NiriWorkspace>,
    windows: HashMap<u64, NiriWindow>,
    last_signature: Option<u64>,
}

fn niri_pair(value: &serde_json::Value, key: &str) -> Option<(f64, f64)> {
    let array = value.get(key)?.as_array()?;
    Some((array.first()?.as_f64()?, array.get(1)?.as_f64()?))
}

fn niri_layout(value: &serde_json::Value) -> NiriLayout {
    NiriLayout {
        column: value
            .get("pos_in_scrolling_layout")
            .and_then(|value| value.as_array())
            .and_then(|array| array.first())
            .and_then(|value| value.as_u64())
            .map(|value| value as usize),
        tile_size: niri_pair(value, "tile_size").unwrap_or((0.0, 0.0)),
        view_pos: niri_pair(value, "tile_pos_in_workspace_view"),
    }
}

fn niri_window(value: &serde_json::Value) -> Option<(u64, NiriWindow)> {
    Some((
        value.get("id")?.as_u64()?,
        NiriWindow {
            workspace_id: value.get("workspace_id").and_then(|value| value.as_u64()),
            is_floating: value
                .get("is_floating")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            layout: value.get("layout").map(niri_layout).unwrap_or_default(),
        },
    ))
}

impl NiriState {
    fn apply_event(&mut self, event: &serde_json::Value) {
        if let Some(workspaces) = event
            .get("WorkspacesChanged")
            .and_then(|value| value.get("workspaces"))
            .and_then(|value| value.as_array())
        {
            self.workspaces.clear();
            for workspace in workspaces {
                let Some(id) = workspace.get("id").and_then(|value| value.as_u64()) else {
                    continue;
                };
                self.workspaces.insert(
                    id,
                    NiriWorkspace {
                        output: workspace
                            .get("output")
                            .and_then(|value| value.as_str())
                            .map(str::to_owned),
                        is_active: workspace
                            .get("is_active")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                    },
                );
            }
        } else if let Some(windows) = event
            .get("WindowsChanged")
            .and_then(|value| value.get("windows"))
            .and_then(|value| value.as_array())
        {
            self.windows.clear();
            for window in windows {
                if let Some((id, window)) = niri_window(window) {
                    self.windows.insert(id, window);
                }
            }
        } else if let Some(window) = event
            .get("WindowOpenedOrChanged")
            .and_then(|value| value.get("window"))
        {
            if let Some((id, window)) = niri_window(window) {
                self.windows.insert(id, window);
            }
        } else if let Some(id) = event
            .get("WindowClosed")
            .and_then(|value| value.get("id"))
            .and_then(|value| value.as_u64())
        {
            self.windows.remove(&id);
        } else if let Some(changes) = event
            .get("WindowLayoutsChanged")
            .and_then(|value| value.get("changes"))
            .and_then(|value| value.as_array())
        {
            for change in changes {
                let Some(change) = change.as_array() else {
                    continue;
                };
                let (Some(id), Some(layout)) = (
                    change.first().and_then(|value| value.as_u64()),
                    change.get(1),
                ) else {
                    continue;
                };
                if let Some(window) = self.windows.get_mut(&id) {
                    window.layout = niri_layout(layout);
                }
            }
        } else if let Some(id) = event
            .get("WorkspaceActivated")
            .and_then(|value| value.get("id"))
            .and_then(|value| value.as_u64())
        {
            let output = self.workspaces.get(&id).and_then(|workspace| {
                workspace.output.clone()
            });
            for (workspace_id, workspace) in self.workspaces.iter_mut() {
                if workspace.output == output {
                    workspace.is_active = *workspace_id == id;
                }
            }
        }
    }

    fn recompute(&mut self) {
        // Monitor hotplug and resolution changes do not emit a dedicated
        // event in this niri version. Re-query when the cached outputs cannot
        // explain the current workspaces or window sizes.
        if self.outputs_stale() {
            if let Ok(outputs) = query_niri_outputs() {
                self.outputs = outputs;
            }
        }

        let coverage = self.coverage();
        let signature = self.signature(&coverage);
        if self.last_signature == Some(signature) {
            return;
        }
        self.last_signature = Some(signature);

        if std::env::var_os("AURA_DEBUG_OCCLUSION").is_some() {
            eprintln!("[occlusion] niri coverage={coverage:?}");
        }

        publish(OcclusionSnapshot {
            available: true,
            coverage,
            revision: 0,
        });
    }

    fn outputs_stale(&self) -> bool {
        for (workspace_id, workspace) in &self.workspaces {
            if !workspace.is_active {
                continue;
            }
            let Some(output) = workspace.output.as_ref() else {
                continue;
            };
            let Some((width, height)) = self.outputs.get(output) else {
                return true;
            };
            for window in self.windows.values() {
                if window.workspace_id != Some(*workspace_id) {
                    continue;
                }
                let (tile_width, tile_height) = window.layout.tile_size;
                if tile_width > f64::from(*width) + 1.0
                    || tile_height > f64::from(*height) + 1.0
                {
                    return true;
                }
            }
        }
        false
    }

    /// Coverage is derived from the active workspace of each output only.
    /// Tiled windows have no absolute position in the IPC layout, so columns
    /// are laid out left to right by their 1-based index; that matches what
    /// the viewport shows for the common one- and two-column layouts.
    fn coverage(&self) -> HashMap<String, f32> {
        let mut rects: HashMap<String, Vec<(f32, f32, f32, f32)>> = HashMap::new();

        for (workspace_id, workspace) in &self.workspaces {
            if !workspace.is_active {
                continue;
            }
            let Some(output) = workspace.output.as_ref() else {
                continue;
            };
            let Some((output_width, output_height)) = self.outputs.get(output).copied() else {
                continue;
            };

            let mut columns: std::collections::BTreeMap<usize, (f64, f64)> =
                std::collections::BTreeMap::new();
            for window in self.windows.values() {
                if window.workspace_id != Some(*workspace_id) {
                    continue;
                }
                let (width, height) = window.layout.tile_size;
                if window.is_floating {
                    if let Some((x, y)) = window.layout.view_pos {
                        if let Some(rect) = clip_rect(
                            (x as f32, y as f32, width as f32, height as f32),
                            output_width,
                            output_height,
                        ) {
                            rects.entry(output.clone()).or_default().push(rect);
                        }
                    }
                    continue;
                }
                let column = window.layout.column.unwrap_or(1);
                let entry = columns.entry(column).or_insert((0.0, 0.0));
                entry.0 = entry.0.max(width);
                // Stacked tiles split the column height; adding them restores
                // the full covered strip.
                entry.1 += height;
            }

            let mut x = 0.0_f32;
            for (_, (width, height)) in columns {
                if width <= 0.0 || height <= 0.0 {
                    continue;
                }
                let width = width as f32;
                let height = (height as f32).min(output_height);
                if let Some(rect) = clip_rect((x, 0.0, width, height), output_width, output_height)
                {
                    rects.entry(output.clone()).or_default().push(rect);
                }
                x += width;
            }
        }

        let mut coverage = HashMap::new();
        for (name, output_rects) in rects {
            let Some((width, height)) = self.outputs.get(&name).copied() else {
                continue;
            };
            let area = union_area(&output_rects);
            coverage.insert(name, (area / (width * height)).clamp(0.0, 1.0));
        }
        coverage
    }

    /// Hashes layout-relevant state only. Focus and title changes do not alter
    /// it, so a manual resume is not cancelled by an unrelated event.
    fn signature(&self, coverage: &HashMap<String, f32>) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        let mut outputs: Vec<(&String, i32, i32)> = self
            .outputs
            .iter()
            .map(|(name, (width, height))| (name, *width as i32, *height as i32))
            .collect();
        outputs.sort_unstable();
        outputs.hash(&mut hasher);

        let mut active: Vec<u64> = self
            .workspaces
            .iter()
            .filter(|(_, workspace)| workspace.is_active)
            .map(|(id, _)| *id)
            .collect();
        active.sort_unstable();
        active.hash(&mut hasher);

        let mut covered: Vec<(&String, i32)> = coverage
            .iter()
            .map(|(name, value)| (name, (value * 10_000.0).round() as i32))
            .collect();
        covered.sort_unstable_by(|a, b| a.0.cmp(b.0));
        covered.hash(&mut hasher);

        let mut windows: Vec<(u64, Option<u64>, bool, Option<usize>, i64, i64, i64, i64)> = self
            .windows
            .iter()
            .map(|(id, window)| {
                let scaled = |value: f64| (value * 10.0).round() as i64;
                let view = window.layout.view_pos;
                (
                    *id,
                    window.workspace_id,
                    window.is_floating,
                    window.layout.column,
                    scaled(window.layout.tile_size.0),
                    scaled(window.layout.tile_size.1),
                    view.map(|pos| scaled(pos.0)).unwrap_or(i64::MIN),
                    view.map(|pos| scaled(pos.1)).unwrap_or(i64::MIN),
                )
            })
            .collect();
        windows.sort_unstable();
        windows.hash(&mut hasher);

        hasher.finish()
    }
}

struct Watcher {
    registry: RegistryState,
    output_state: OutputState,
    toplevel_info: ToplevelInfoState,
    workspace_state: WorkspaceState,
    /// Hash of the last published window layout. Focus changes do not alter
    /// it, so a manual resume is not cancelled by an unrelated focus event.
    last_signature: Option<u64>,
}

impl Watcher {
    fn recompute(&mut self) {
        let mut coverage: HashMap<String, f32> = HashMap::new();
        let mut output_sizes: HashMap<String, (f32, f32)> = HashMap::new();

        for output in self.output_state.outputs() {
            if let Some(info) = self.output_state.info(&output) {
                let Some(name) = info.name else {
                    continue;
                };
                let Some((width, height)) = info.logical_size else {
                    continue;
                };
                if width > 0 && height > 0 {
                    output_sizes.insert(name.clone(), (width as f32, height as f32));
                    coverage.entry(name).or_insert(0.0);
                }
            }
        }

        // Only windows on the workspace currently shown on their monitor
        // count. Otherwise switching to an empty workspace would keep the
        // wallpaper paused because of windows left on the previous one.
        let active_workspaces: HashSet<u32> = self
            .workspace_state
            .workspaces()
            .filter(|workspace| workspace.state.contains(ext_workspace_handle_v1::State::Active))
            .map(|workspace| workspace.handle.id().protocol_id())
            .collect();

        // Collect every visible toplevel; overlapping windows must be unioned,
        // not summed, or two stacked windows would report 200% coverage.
        let mut rects: HashMap<String, Vec<(f32, f32, f32, f32)>> = HashMap::new();
        for toplevel in self.toplevel_info.toplevels() {
            if toplevel.state.contains(&ToplevelState::Minimized) {
                continue;
            }
            let toplevel_workspaces: HashSet<u32> =
                toplevel.workspace
                    .iter()
                    .map(|workspace| workspace.id().protocol_id())
                    .collect();
            let visible =
                visible_on_active_workspace(&toplevel_workspaces, &active_workspaces);
            if std::env::var_os("AURA_DEBUG_OCCLUSION").is_some() {
                eprintln!(
                    "[occlusion] toplevel={:?} workspaces={:?} visible={visible}",
                    toplevel.app_id, toplevel_workspaces
                );
            }
            if !visible {
                continue;
            }
            let fully_covered = toplevel.state.contains(&ToplevelState::Fullscreen)
                || toplevel.state.contains(&ToplevelState::Maximized);

            for output in &toplevel.output {
                let Some(info) = self.output_state.info(output) else {
                    continue;
                };
                let Some(name) = info.name.clone() else {
                    continue;
                };
                if fully_covered {
                    coverage.insert(name, 1.0);
                    continue;
                }
                let Some(geometry) = toplevel.geometry.get(output) else {
                    continue;
                };
                let Some((width, height)) = output_sizes.get(&name) else {
                    continue;
                };
                let Some(clipped) = clip_rect(
                    (
                        geometry.x as f32,
                        geometry.y as f32,
                        geometry.width as f32,
                        geometry.height as f32,
                    ),
                    *width,
                    *height,
                ) else {
                    continue;
                };
                rects.entry(name).or_default().push(clipped);
            }
        }

        for (name, output_rects) in rects {
            let Some((width, height)) = output_sizes.get(&name) else {
                continue;
            };
            let area = union_area(&output_rects);
            let fraction = (area / (width * height)).clamp(0.0, 1.0);
            let entry = coverage.entry(name).or_insert(0.0);
            // A maximized/fullscreen toplevel already set this to 1.0.
            *entry = entry.max(fraction);
        }

        if std::env::var_os("AURA_DEBUG_OCCLUSION").is_some() {
            eprintln!(
                "[occlusion] active_workspaces={:?} coverage={:?}",
                active_workspaces, coverage
            );
        }

        let signature = self.layout_signature(&active_workspaces, &coverage);
        if self.last_signature == Some(signature) {
            return;
        }
        self.last_signature = Some(signature);

        publish(OcclusionSnapshot {
            available: true,
            coverage,
            revision: 0,
        });
    }

    /// Hashes only layout-relevant state: outputs, coverage, workspaces and
    /// window geometry/state. Focus changes are intentionally excluded.
    fn layout_signature(
        &self,
        active_workspaces: &HashSet<u32>,
        coverage: &HashMap<String, f32>,
    ) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        let mut active: Vec<u32> = active_workspaces.iter().copied().collect();
        active.sort_unstable();
        active.hash(&mut hasher);

        let mut outputs: Vec<(&String, i32)> = coverage
            .iter()
            .map(|(name, value)| (name, (value * 10_000.0).round() as i32))
            .collect();
        outputs.sort_unstable_by(|a, b| a.0.cmp(b.0));
        outputs.hash(&mut hasher);

        for toplevel in self.toplevel_info.toplevels() {
            let visible_workspaces: HashSet<u32> = toplevel
                .workspace
                .iter()
                .map(|workspace| workspace.id().protocol_id())
                .collect();
            if !visible_on_active_workspace(&visible_workspaces, active_workspaces) {
                continue;
            }
            toplevel.app_id.hash(&mut hasher);
            toplevel
                .state
                .contains(&ToplevelState::Minimized)
                .hash(&mut hasher);
            toplevel
                .state
                .contains(&ToplevelState::Maximized)
                .hash(&mut hasher);
            toplevel
                .state
                .contains(&ToplevelState::Fullscreen)
                .hash(&mut hasher);

            let mut workspaces: Vec<u32> = toplevel
                .workspace
                .iter()
                .map(|workspace| workspace.id().protocol_id())
                .collect();
            workspaces.sort_unstable();
            workspaces.hash(&mut hasher);

            let mut output_ids: Vec<u32> = toplevel
                .output
                .iter()
                .map(|output| output.id().protocol_id())
                .collect();
            output_ids.sort_unstable();
            output_ids.hash(&mut hasher);

            for output in &toplevel.output {
                if let Some(geometry) = toplevel.geometry.get(output) {
                    geometry.x.hash(&mut hasher);
                    geometry.y.hash(&mut hasher);
                    geometry.width.hash(&mut hasher);
                    geometry.height.hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }
}

impl WorkspaceHandler for Watcher {
    fn workspace_state(&mut self) -> &mut WorkspaceState {
        &mut self.workspace_state
    }

    fn done(&mut self) {
        self.recompute();
    }
}

impl ProvidesRegistryState for Watcher {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState];
}

impl OutputHandler for Watcher {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.recompute();
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.recompute();
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.recompute();
    }
}

impl ToplevelInfoHandler for Watcher {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        &mut self.toplevel_info
    }

    fn new_toplevel(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &ExtForeignToplevelHandleV1,
    ) {
        self.recompute();
    }

    fn update_toplevel(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &ExtForeignToplevelHandleV1,
    ) {
        self.recompute();
    }

    fn toplevel_closed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &ExtForeignToplevelHandleV1,
    ) {
        self.recompute();
    }

    fn info_done(&mut self, _: &Connection, _: &QueueHandle<Self>) {
        self.recompute();
    }
}

delegate_output!(Watcher);
delegate_registry!(Watcher);
delegate_toplevel_info!(Watcher);
delegate_workspace!(Watcher);

/// A toplevel counts only when it is on an active workspace. If the compositor
/// does not expose workspace information (or the toplevel was not assigned
/// yet), keep it visible rather than leaving the wallpaper stuck paused.
fn visible_on_active_workspace(
    toplevel_workspaces: &HashSet<u32>,
    active_workspaces: &HashSet<u32>,
) -> bool {
    active_workspaces.is_empty()
        || toplevel_workspaces.is_empty()
        || toplevel_workspaces
            .iter()
            .any(|workspace| active_workspaces.contains(workspace))
}

fn clip_rect(
    rect: (f32, f32, f32, f32),
    output_width: f32,
    output_height: f32,
) -> Option<(f32, f32, f32, f32)> {
    let (x, y, width, height) = rect;
    let left = x.max(0.0);
    let top = y.max(0.0);
    let right = (x + width.max(0.0)).min(output_width);
    let bottom = (y + height.max(0.0)).min(output_height);
    if right <= left || bottom <= top {
        return None;
    }
    Some((left, top, right - left, bottom - top))
}

/// Fraction of an output covered by `rect` (x, y, width, height), clipped to
/// the output bounds. Used by the geometry tests; runtime code unions toplevels.
#[cfg(test)]
pub fn covered_fraction(rect: (f32, f32, f32, f32), output_width: f32, output_height: f32) -> f32 {
    if output_width <= 0.0 || output_height <= 0.0 {
        return 0.0;
    }
    match clip_rect(rect, output_width, output_height) {
        Some((_, _, width, height)) => (width * height / (output_width * output_height)).clamp(0.0, 1.0),
        None => 0.0,
    }
}

/// Area of the union of axis-aligned rectangles. Sweeps vertical strips so
/// overlapping/tiled windows are only counted once.
pub fn union_area(rects: &[(f32, f32, f32, f32)]) -> f32 {
    let mut xs: Vec<f32> = rects
        .iter()
        .flat_map(|(x, _, width, _)| [*x, x + width.max(0.0)])
        .collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    let mut area = 0.0;
    for pair in xs.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        let strip_width = right - left;
        if strip_width <= 0.0 {
            continue;
        }
        let mut intervals: Vec<(f32, f32)> = rects
            .iter()
            .filter(|(x, _, width, _)| *x <= left && x + width.max(0.0) >= right)
            .map(|(_, y, _, height)| (*y, y + height.max(0.0)))
            .collect();
        if intervals.is_empty() {
            continue;
        }
        intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut merged_height = 0.0;
        let (mut start, mut end) = intervals[0];
        for (next_start, next_end) in intervals.into_iter().skip(1) {
            if next_start > end {
                merged_height += (end - start).max(0.0);
                start = next_start;
                end = next_end;
            } else if next_end > end {
                end = next_end;
            }
        }
        merged_height += (end - start).max(0.0);
        area += strip_width * merged_height;
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_clips_to_output() {
        assert!((covered_fraction((0.0, 0.0, 960.0, 1080.0), 1920.0, 1080.0) - 0.5).abs() < 0.001);
        assert!((covered_fraction((-100.0, -100.0, 200.0, 200.0), 1920.0, 1080.0)
            - (100.0 * 100.0) / (1920.0 * 1080.0))
            .abs()
            < 0.001);
        assert_eq!(covered_fraction((3000.0, 0.0, 100.0, 100.0), 1920.0, 1080.0), 0.0);
        assert_eq!(covered_fraction((0.0, 0.0, 0.0, 0.0), 0.0, 0.0), 0.0);
    }

    #[test]
    #[ignore = "requires a live COSMIC Wayland session"]
    fn live_snapshot_probe() {
        start();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        println!("occlusion snapshot: {:?}", snapshot());
    }

    #[test]
    fn hides_toplevels_left_on_inactive_workspaces() {
        let active: HashSet<u32> = [1, 3].into_iter().collect();
        let inactive: HashSet<u32> = [2].into_iter().collect();
        let mixed: HashSet<u32> = [2, 3].into_iter().collect();
        assert!(visible_on_active_workspace(&active, &active));
        assert!(!visible_on_active_workspace(&inactive, &active));
        assert!(visible_on_active_workspace(&mixed, &active));
        // No workspace protocol data: never leave the wallpaper stuck paused.
        assert!(visible_on_active_workspace(&inactive, &HashSet::new()));
        assert!(visible_on_active_workspace(&HashSet::new(), &active));
    }

    #[test]
    fn union_area_counts_overlaps_once() {
        let half = (0.0, 0.0, 960.0, 1080.0);
        assert!((union_area(&[half]) / (1920.0 * 1080.0) - 0.5).abs() < 0.001);
        // Two half-width windows side by side cover the whole output.
        let other = (960.0, 0.0, 960.0, 1080.0);
        assert!((union_area(&[half, other]) / (1920.0 * 1080.0) - 1.0).abs() < 0.001);
        // Stacked copies of the same window still cover half.
        assert!((union_area(&[half, half]) / (1920.0 * 1080.0) - 0.5).abs() < 0.001);
        assert_eq!(union_area(&[]), 0.0);
    }

    fn niri_event(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("valid niri event")
    }

    fn niri_state_with_workspace() -> NiriState {
        let mut state = NiriState::default();
        state.outputs.insert("eDP-1".into(), (1366.0, 768.0));
        state.apply_event(&niri_event(
            r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"output":"eDP-1","is_active":true}]}}"#,
        ));
        state
    }

    #[test]
    fn niri_maximized_column_pauses() {
        let mut state = niri_state_with_workspace();
        state.apply_event(&niri_event(
            r#"{"WindowsChanged":{"windows":[{"id":2,"workspace_id":1,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1358.0,718.0],"tile_pos_in_workspace_view":null}}]}}"#,
        ));
        let coverage = state.coverage();
        let value = coverage["eDP-1"];
        assert!((value - 0.929).abs() < 0.01, "coverage was {value}");
        assert!(crate::config::AutoPauseMode::Maximized.should_pause(value));
    }

    #[test]
    fn niri_default_column_does_not_count_as_maximized() {
        let mut state = niri_state_with_workspace();
        state.apply_event(&niri_event(
            r#"{"WindowsChanged":{"windows":[{"id":2,"workspace_id":1,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[677.0,718.0],"tile_pos_in_workspace_view":null}}]}}"#,
        ));
        let value = state.coverage()["eDP-1"];
        assert!((value - 0.463).abs() < 0.01, "coverage was {value}");
        assert!(!crate::config::AutoPauseMode::Maximized.should_pause(value));
        assert!(crate::config::AutoPauseMode::Partial.should_pause(value));
    }

    #[test]
    fn niri_inactive_workspace_does_not_pause() {
        let mut state = niri_state_with_workspace();
        state.apply_event(&niri_event(
            r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"output":"eDP-1","is_active":true},{"id":2,"output":"eDP-1","is_active":false}]}}"#,
        ));
        state.apply_event(&niri_event(
            r#"{"WindowsChanged":{"windows":[{"id":3,"workspace_id":2,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1358.0,718.0],"tile_pos_in_workspace_view":null}}]}}"#,
        ));
        assert!(state.coverage().is_empty());
        state.apply_event(&niri_event(r#"{"WorkspaceActivated":{"id":2,"focused":true}}"#));
        assert!(state.coverage()["eDP-1"] > 0.9);
    }

    #[test]
    fn niri_layout_change_updates_coverage() {
        let mut state = niri_state_with_workspace();
        state.apply_event(&niri_event(
            r#"{"WindowsChanged":{"windows":[{"id":2,"workspace_id":1,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[677.0,718.0],"tile_pos_in_workspace_view":null}}]}}"#,
        ));
        let before = state.coverage()["eDP-1"];
        state.apply_event(&niri_event(
            r#"{"WindowLayoutsChanged":{"changes":[[2,{"pos_in_scrolling_layout":[1,1],"tile_size":[1366.0,768.0],"tile_pos_in_workspace_view":null}]]}}"#,
        ));
        let after = state.coverage()["eDP-1"];
        assert!(after > before);
        assert!((after - 1.0).abs() < 0.001);
    }

    #[test]
    fn niri_focus_events_do_not_change_signature() {
        let mut state = niri_state_with_workspace();
        state.apply_event(&niri_event(
            r#"{"WindowOpenedOrChanged":{"window":{"id":2,"workspace_id":1,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1358.0,718.0],"tile_pos_in_workspace_view":null}}}}"#,
        ));
        let signature = state.signature(&state.coverage());
        state.apply_event(&niri_event(r#"{"WindowFocusChanged":{"id":2}}"#));
        state.apply_event(&niri_event(
            r#"{"WindowOpenedOrChanged":{"window":{"id":2,"workspace_id":1,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1358.0,718.0],"tile_pos_in_workspace_view":null}}}}"#,
        ));
        assert_eq!(state.signature(&state.coverage()), signature);
    }

    #[test]
    fn snapshot_coverage_lookup_defaults_to_zero() {
        let mut snapshot = OcclusionSnapshot {
            available: true,
            coverage: HashMap::new(),
            revision: 0,
        };
        snapshot.coverage.insert("eDP-1".into(), 0.75);
        assert_eq!(snapshot.coverage_for("eDP-1"), 0.75);
        assert_eq!(snapshot.coverage_for("HDMI-A-1"), 0.0);
    }
}
