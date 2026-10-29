//! COSMIC toplevel watcher used by the automatic pause feature.
//!
//! `mpvpaper -p` can only tell when the compositor stops sending frame
//! callbacks, which happens when the wallpaper is fully covered (maximized or
//! fullscreen). COSMIC's tiling manager leaves part of the wallpaper visible,
//! so mpvpaper keeps decoding behind tiled windows. COSMIC does not implement
//! `wlr-foreign-toplevel-management`, so mpvpaper's `--auto-mode` cannot help
//! there either.
//!
//! This module listens to the COSMIC toplevel-info protocol on its own
//! Wayland connection and publishes, per output, how much of the wallpaper is
//! covered. The UI turns that into per-monitor pauses through the existing IPC
//! controls.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

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
        thread::Builder::new()
            .name("aura-toplevels".into())
            .spawn(|| loop {
                if !run_watcher() {
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
