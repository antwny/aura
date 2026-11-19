use std::{
    cell::Cell,
    collections::HashMap,
    io::Cursor,
    ptr::NonNull,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[path = "effects_cursor_kwin.rs"]
mod kwin;

use futures::{future::try_join_all, StreamExt};
use pipewire::{self as pw, properties::properties, spa};
use spa::{param::video::VideoInfoRaw, pod::Pod};
use tokio::{io::unix::AsyncFd, time::timeout};
use zbus::{
    proxy::{OwnerChangedStream, SignalStream},
    zvariant::{OwnedObjectPath, Value},
    Connection, Proxy,
};

const SERVICE: &str = "org.gnome.Mutter.ScreenCast";
const ROOT: &str = "/org/gnome/Mutter/ScreenCast";
const SESSION_INTERFACE: &str = "org.gnome.Mutter.ScreenCast.Session";
const STREAM_INTERFACE: &str = "org.gnome.Mutter.ScreenCast.Stream";
const SETUP_TIMEOUT: Duration = Duration::from_secs(10);
const NODE_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
// Producers can attach a bitmap even though only the cursor header is consumed.
const CURSOR_META_SIZE: usize = std::mem::size_of::<spa::sys::spa_meta_cursor>()
    + std::mem::size_of::<spa::sys::spa_meta_bitmap>()
    + 384 * 384 * 4;

type Positions = Arc<Mutex<HashMap<String, [f32; 2]>>>;
type Result<T> = std::result::Result<T, String>;

pub struct CursorTracker {
    positions: Positions,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl CursorTracker {
    pub fn start(mut outputs: Vec<String>) -> Self {
        outputs.retain(|output| !output.is_empty());
        outputs.sort_unstable();
        outputs.dedup();
        let positions = Arc::new(Mutex::new(
            outputs
                .iter()
                .map(|output| (output.clone(), [0.0; 2]))
                .collect(),
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = if outputs.is_empty() {
            None
        } else {
            let positions = positions.clone();
            let stop = stop.clone();
            match thread::Builder::new()
                .name("aura-effects-cursor".into())
                .spawn(move || {
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(runtime) => {
                            if let Err(error) =
                                runtime.block_on(run_worker(outputs, &positions, &stop))
                            {
                                eprintln!(
                                    "aura-effects: cursor backend stopped: {error}; parallax centered"
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!("aura-effects: cursor runtime unavailable: {error}");
                        }
                    }
                    positions
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .clear();
                })
            {
                Ok(worker) => Some(worker),
                Err(error) => {
                    eprintln!("aura-effects: cannot start cursor worker: {error}");
                    None
                }
            }
        };
        Self {
            positions,
            stop,
            worker,
        }
    }

    pub fn position(&self, output: &str) -> [f32; 2] {
        self.positions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(output)
            .copied()
            .unwrap_or([0.0; 2])
    }
}

impl Drop for CursorTracker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("aura-effects: cursor worker exited unexpectedly");
            }
        }
    }
}

async fn stopped(stop: &AtomicBool) {
    while !stop.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn run_worker(outputs: Vec<String>, positions: &Positions, stop: &AtomicBool) -> Result<()> {
    if ext_cursor::run(&outputs, positions, stop).await?
        || kwin::run(outputs.clone(), positions, stop).await?
    {
        Ok(())
    } else {
        run_mutter_worker(outputs, positions, stop).await
    }
}

async fn run_mutter_worker(outputs: Vec<String>, positions: &Positions, stop: &AtomicBool) -> Result<()> {
    let connection = tokio::select! {
        biased;
        _ = stopped(stop) => return Ok(()),
        result = timeout(SETUP_TIMEOUT, Connection::session()) => {
            result.map_err(|_| "D-Bus connection timed out".to_owned())?
                .map_err(|error| format!("D-Bus connection: {error}"))?
        }
    };
    let mut session_path = None;
    let result = async {
        let streams = tokio::select! {
            biased;
            _ = stopped(stop) => return Ok(()),
            result = timeout(SETUP_TIMEOUT, setup_dbus(&connection, &mut session_path, outputs)) => {
                result.map_err(|_| "ScreenCast setup timed out".to_owned())??
            }
        };
        run_pipewire(&connection, streams, positions, stop).await
    }
    .await;

    positions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    if let Some(path) = session_path {
        match timeout(
            CLOSE_TIMEOUT,
            connection.call_method(Some(SERVICE), path, Some(SESSION_INTERFACE), "Stop", &()),
        )
        .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => eprintln!("aura-effects: cursor ScreenCast Stop: {error}"),
            Err(_) => {
                eprintln!("aura-effects: cursor ScreenCast Stop timed out; disconnecting");
            }
        }
    }
    // Disconnect also releases a session whose CreateSession reply was lost to cancellation.
    match timeout(CLOSE_TIMEOUT, connection.close()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => eprintln!("aura-effects: cursor D-Bus disconnect: {error}"),
        Err(_) => eprintln!("aura-effects: cursor D-Bus disconnect timed out"),
    }
    result
}

// EXT uses only cursor metadata: no capture session, frames, buffers or surfaces.
mod ext_cursor {
    use super::{stopped, Positions, Result, CLOSE_TIMEOUT, SETUP_TIMEOUT};
    use std::{
        collections::HashMap,
        io::ErrorKind,
        os::fd::{AsFd, BorrowedFd},
        path::PathBuf,
        sync::atomic::{AtomicBool, Ordering},
    };
    use smithay_client_toolkit::reexports::protocols::{
        ext::{
            image_capture_source::v1::client::{
                ext_image_capture_source_v1 as source,
                ext_output_image_capture_source_manager_v1 as source_manager,
            },
            image_copy_capture::v1::client::{
                ext_image_copy_capture_cursor_session_v1 as cursor_session,
                ext_image_copy_capture_manager_v1 as capture_manager,
            },
        },
        xdg::xdg_output::zv1::client::{
            zxdg_output_manager_v1 as xdg_manager, zxdg_output_v1 as xdg_output,
        },
    };
    use tokio::{io::unix::AsyncFd, time::timeout};
    use wayland_client::{
        backend::WaylandError,
        protocol::{wl_callback, wl_output, wl_pointer, wl_registry, wl_seat},
        Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    };

    #[derive(Clone, Copy, Default)]
    struct Capabilities {
        source: bool,
        capture: bool,
        pointer: bool,
        output: bool,
    }

    impl Capabilities {
        fn protocols(self) -> bool {
            self.source && self.capture
        }

        fn can_track(self) -> bool {
            self.protocols() && self.pointer && self.output
        }
    }

    #[derive(Clone, Copy)]
    struct OutputSize {
        mode: [i32; 2],
        transform: Option<wl_output::Transform>,
        scale: i32,
        logical: Option<[i32; 2]>,
    }

    impl Default for OutputSize {
        fn default() -> Self {
            Self { mode: [0; 2], transform: None, scale: 1, logical: None }
        }
    }

    impl OutputSize {
        fn dimensions(self, logical_coordinates: bool) -> Option<[f64; 2]> {
            if self.mode.iter().any(|value| *value <= 0) || self.scale <= 0 {
                return None;
            }
            let swapped = match self.transform? {
                wl_output::Transform::_90 | wl_output::Transform::_270
                | wl_output::Transform::Flipped90 | wl_output::Transform::Flipped270 => true,
                wl_output::Transform::Normal | wl_output::Transform::_180
                | wl_output::Transform::Flipped | wl_output::Transform::Flipped180 => false,
                _ => return None,
            };
            let pixels = if swapped { [self.mode[1], self.mode[0]] } else { self.mode };
            if logical_coordinates {
                // xdg-output supplies the real logical box, including fractional scaling.
                if let Some(logical) = self.logical {
                    return logical.iter().all(|value| *value > 0)
                        .then_some([logical[0] as f64, logical[1] as f64]);
                }
                return Some([
                    pixels[0] as f64 / self.scale as f64,
                    pixels[1] as f64 / self.scale as f64,
                ]);
            }
            Some([pixels[0] as f64, pixels[1] as f64])
        }
    }

    fn normalize(point: [i32; 2], size: [f64; 2]) -> Option<[f32; 2]> {
        if size.iter().any(|value| !value.is_finite() || *value <= 0.0) {
            return None;
        }
        // EXT permits a hotspot outside the output while the cursor image intersects it.
        Some([
            (2.0 * point[0] as f64 / size[0] - 1.0).clamp(-1.0, 1.0) as f32,
            (2.0 * point[1] as f64 / size[1] - 1.0).clamp(-1.0, 1.0) as f32,
        ])
    }

    fn set_position(positions: &Positions, name: &str, point: [f32; 2]) {
        if let Some(value) = positions.lock().unwrap_or_else(|error| error.into_inner()).get_mut(name) {
            *value = point;
        }
    }

    #[derive(Default)]
    struct Metadata {
        inside: bool,
        point: Option<[i32; 2]>,
    }

    impl Metadata {
        fn enter(&mut self) {
            self.inside = true;
            self.point = None;
        }

        fn leave(&mut self, positions: &Positions, name: &str) {
            self.inside = false;
            self.point = None;
            set_position(positions, name, [0.0; 2]);
        }

        fn position(&self, size: Option<[f64; 2]>) -> Option<[f32; 2]> {
            if !self.inside {
                return None;
            }
            normalize(self.point?, size?)
        }
    }

    struct Output {
        proxy: wl_output::WlOutput,
        xdg: Option<xdg_output::ZxdgOutputV1>,
        name: Option<String>,
        pending_name: Option<String>,
        size: OutputSize,
        pending_size: OutputSize,
    }

    impl Output {
        fn commit(&mut self) {
            self.name = self.pending_name.clone();
            self.size = self.pending_size;
        }

        fn destroy(self) {
            if let Some(xdg) = self.xdg {
                xdg.destroy();
            }
            if self.proxy.version() >= 3 {
                self.proxy.release();
            }
        }
    }

    struct Seat {
        proxy: wl_seat::WlSeat,
        pointer: Option<wl_pointer::WlPointer>,
    }

    impl Seat {
        fn release_pointer(&mut self) {
            if let Some(pointer) = self.pointer.take() {
                if pointer.version() >= 3 {
                    pointer.release();
                }
            }
        }

        fn destroy(mut self) {
            self.release_pointer();
            if self.proxy.version() >= 5 {
                self.proxy.release();
            }
        }
    }

    struct Cursor {
        name: String,
        seat: u32,
        source: source::ExtImageCaptureSourceV1,
        session: cursor_session::ExtImageCopyCaptureCursorSessionV1,
        metadata: Metadata,
    }

    impl Cursor {
        fn destroy(mut self, positions: &Positions) {
            self.metadata.leave(positions, &self.name);
            self.session.destroy();
            self.source.destroy();
        }
    }

    struct State {
        requested: Vec<String>,
        positions: Positions,
        source_manager: Option<(u32, source_manager::ExtOutputImageCaptureSourceManagerV1)>,
        capture_manager: Option<(u32, capture_manager::ExtImageCopyCaptureManagerV1)>,
        xdg_manager: Option<(u32, xdg_manager::ZxdgOutputManagerV1)>,
        outputs: HashMap<u32, Output>,
        seats: HashMap<u32, Seat>,
        cursors: HashMap<u32, Cursor>,
        logical_coordinates: bool,
        synced: bool,
        active_logged: bool,
        error: Option<String>,
    }

    impl State {
        fn new(outputs: &[String], positions: &Positions) -> Self {
            Self {
                requested: outputs.to_vec(), positions: positions.clone(),
                source_manager: None, capture_manager: None, xdg_manager: None,
                outputs: HashMap::new(), seats: HashMap::new(), cursors: HashMap::new(),
                logical_coordinates: false, synced: false, active_logged: false, error: None,
            }
        }

        fn capabilities(&self) -> Capabilities {
            Capabilities {
                source: self.source_manager.is_some(),
                capture: self.capture_manager.is_some(),
                pointer: self.seats.values().any(|seat| seat.pointer.is_some()),
                output: self.outputs.values().any(|output| {
                    output.name.as_ref().is_some_and(|name| self.requested.contains(name))
                        && output.size.dimensions(self.logical_coordinates).is_some()
                }),
            }
        }

        fn attach_xdg_outputs(&mut self, qh: &QueueHandle<Self>) {
            if let Some((_, manager)) = &self.xdg_manager {
                for (id, output) in &mut self.outputs {
                    if output.xdg.is_none() {
                        output.xdg = Some(manager.get_xdg_output(&output.proxy, qh, *id));
                    }
                }
            }
        }

        fn remove_cursor(&mut self, id: u32) {
            if let Some(cursor) = self.cursors.remove(&id) {
                cursor.destroy(&self.positions);
            }
        }

        fn remove_seat_cursors(&mut self, seat: u32) {
            let ids: Vec<_> = self.cursors.iter()
                .filter_map(|(id, cursor)| (cursor.seat == seat).then_some(*id)).collect();
            for id in ids {
                self.remove_cursor(id);
            }
        }

        fn publish(&mut self, id: u32, real_position: bool) {
            let Some(cursor) = self.cursors.get(&id) else { return };
            let size = self.outputs.get(&id)
                .and_then(|output| output.size.dimensions(self.logical_coordinates));
            let position = cursor.metadata.position(size);
            set_position(&self.positions, &cursor.name, position.unwrap_or([0.0; 2]));
            if real_position && position.is_some() && !self.active_logged {
                self.active_logged = true;
                eprintln!("aura-effects: EXT cursor metadata tracking active ({})", cursor.name);
            }
        }

        fn reconcile(&mut self, qh: &QueueHandle<Self>) {
            // One deterministic seat; changing/removing it clears all of its metadata first.
            let pointer = self.seats.iter().filter_map(|(id, seat)| {
                seat.pointer.as_ref().map(|pointer| (*id, pointer.clone()))
            }).min_by_key(|(id, _)| *id);
            let stale: Vec<_> = self.cursors.iter().filter_map(|(id, cursor)| {
                let valid_output = self.outputs.get(id).is_some_and(|output| {
                    output.name.as_ref() == Some(&cursor.name)
                        && output.size.dimensions(self.logical_coordinates).is_some()
                });
                (!valid_output || pointer.as_ref().map(|(id, _)| *id) != Some(cursor.seat))
                    .then_some(*id)
            }).collect();
            for id in stale {
                self.remove_cursor(id);
            }
            if let (Some((_, source)), Some((_, capture)), Some((seat, pointer))) =
                (&self.source_manager, &self.capture_manager, pointer)
            {
                for (id, output) in &self.outputs {
                    let Some(name) = output.name.as_ref() else { continue };
                    if !self.requested.contains(name) || self.cursors.contains_key(id)
                        || output.size.dimensions(self.logical_coordinates).is_none()
                    {
                        continue;
                    }
                    let source = source.create_source(&output.proxy, qh, ());
                    let session = capture.create_pointer_cursor_session(&source, &pointer, qh, *id);
                    self.cursors.insert(*id, Cursor {
                        name: name.clone(), seat, source, session, metadata: Metadata::default(),
                    });
                }
            }
            let ids: Vec<_> = self.cursors.keys().copied().collect();
            for id in ids {
                // Re-map only previously received metadata when output dimensions change.
                self.publish(id, false);
            }
        }

        fn destroy(&mut self) {
            for (_, cursor) in self.cursors.drain() {
                cursor.destroy(&self.positions);
            }
            for (_, seat) in self.seats.drain() {
                seat.destroy();
            }
            for (_, output) in self.outputs.drain() {
                output.destroy();
            }
            if let Some((_, manager)) = self.capture_manager.take() { manager.destroy(); }
            if let Some((_, manager)) = self.source_manager.take() { manager.destroy(); }
            if let Some((_, manager)) = self.xdg_manager.take() { manager.destroy(); }
            // Preserve the requested keys when handing an unsupported compositor to Mutter.
            for point in self.positions.lock().unwrap_or_else(|error| error.into_inner()).values_mut() {
                *point = [0.0; 2];
            }
        }
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event,
            _: &(), _: &Connection, qh: &QueueHandle<Self>)
        {
            match event {
                wl_registry::Event::Global { name, interface, version } => {
                    if version == 0 { return; }
                    // Hyprland 0.57.2 emits logicalBox positions instead of EXT pixel coordinates.
                    // Compositor-specific globals identify this quirk, never select the backend.
                    if interface.starts_with("hyprland_") {
                        state.logical_coordinates = true;
                    }
                    match interface.as_str() {
                        "ext_output_image_capture_source_manager_v1" => {
                            state.source_manager = Some((name, registry.bind(name, 1, qh, ())));
                        }
                        "ext_image_copy_capture_manager_v1" => {
                            state.capture_manager = Some((name, registry.bind(name, 1, qh, ())));
                        }
                        "zxdg_output_manager_v1" => {
                            state.xdg_manager = Some((name, registry.bind(name, version.min(3), qh, ())));
                            state.attach_xdg_outputs(qh);
                        }
                        "wl_output" => {
                            state.outputs.insert(name, Output {
                                proxy: registry.bind(name, version.min(4), qh, name),
                                xdg: None, name: None, pending_name: None,
                                size: OutputSize::default(), pending_size: OutputSize::default(),
                            });
                            state.attach_xdg_outputs(qh);
                        }
                        "wl_seat" => {
                            state.seats.insert(name, Seat {
                                proxy: registry.bind(name, version.min(9), qh, name), pointer: None,
                            });
                        }
                        _ => {}
                    }
                }
                wl_registry::Event::GlobalRemove { name } => {
                    state.remove_cursor(name);
                    if let Some(output) = state.outputs.remove(&name) { output.destroy(); }
                    state.remove_seat_cursors(name);
                    if let Some(seat) = state.seats.remove(&name) { seat.destroy(); }
                    if state.source_manager.as_ref().is_some_and(|(id, _)| *id == name)
                        || state.capture_manager.as_ref().is_some_and(|(id, _)| *id == name)
                    {
                        state.error = Some("EXT cursor manager global removed".into());
                        let ids: Vec<_> = state.cursors.keys().copied().collect();
                        for id in ids { state.remove_cursor(id); }
                    }
                    if state.xdg_manager.as_ref().is_some_and(|(id, _)| *id == name) {
                        if let Some((_, manager)) = state.xdg_manager.take() { manager.destroy(); }
                    }
                }
                _ => {}
            }
        }
    }

    impl Dispatch<wl_output::WlOutput, u32> for State {
        fn event(state: &mut Self, proxy: &wl_output::WlOutput, event: wl_output::Event,
            id: &u32, _: &Connection, _: &QueueHandle<Self>)
        {
            let Some(output) = state.outputs.get_mut(id) else { return };
            match event {
                wl_output::Event::Geometry { transform, .. } => {
                    output.pending_size.transform = match transform {
                        WEnum::Value(transform) => Some(transform), WEnum::Unknown(_) => None,
                    };
                }
                wl_output::Event::Mode { flags: WEnum::Value(flags), width, height, .. }
                    if flags.contains(wl_output::Mode::Current) => {
                        output.pending_size.mode = [width, height];
                    }
                wl_output::Event::Scale { factor } => output.pending_size.scale = factor,
                wl_output::Event::Name { name } => output.pending_name = Some(name),
                wl_output::Event::Done => output.commit(),
                _ => {}
            }
            if proxy.version() < 2 { output.commit(); }
        }
    }

    impl Dispatch<xdg_output::ZxdgOutputV1, u32> for State {
        fn event(state: &mut Self, proxy: &xdg_output::ZxdgOutputV1, event: xdg_output::Event,
            id: &u32, _: &Connection, _: &QueueHandle<Self>)
        {
            let Some(output) = state.outputs.get_mut(id) else { return };
            match event {
                xdg_output::Event::LogicalSize { width, height } => {
                    output.pending_size.logical = Some([width, height]);
                }
                xdg_output::Event::Name { name } => output.pending_name = Some(name),
                // v3 completes xdg-output updates with wl_output.done instead.
                xdg_output::Event::Done if proxy.version() < 3 => output.commit(),
                _ => {}
            }
        }
    }

    impl Dispatch<wl_seat::WlSeat, u32> for State {
        fn event(state: &mut Self, _: &wl_seat::WlSeat, event: wl_seat::Event,
            id: &u32, _: &Connection, qh: &QueueHandle<Self>)
        {
            if let wl_seat::Event::Capabilities { capabilities } = event {
                let has_pointer = matches!(capabilities,
                    WEnum::Value(value) if value.contains(wl_seat::Capability::Pointer));
                if !has_pointer { state.remove_seat_cursors(*id); }
                if let Some(seat) = state.seats.get_mut(id) {
                    if has_pointer && seat.pointer.is_none() {
                        seat.pointer = Some(seat.proxy.get_pointer(qh, ()));
                    } else if !has_pointer {
                        seat.release_pointer();
                    }
                }
            }
        }
    }

    impl Dispatch<cursor_session::ExtImageCopyCaptureCursorSessionV1, u32> for State {
        fn event(state: &mut Self, proxy: &cursor_session::ExtImageCopyCaptureCursorSessionV1,
            event: cursor_session::Event, id: &u32, _: &Connection, _: &QueueHandle<Self>)
        {
            let Some(cursor) = state.cursors.get_mut(id) else { return };
            // Ignore events queued for an output/seat session that has already been replaced.
            if cursor.session.id() != proxy.id() { return; }
            let mut real_position = false;
            match event {
                cursor_session::Event::Enter => cursor.metadata.enter(),
                cursor_session::Event::Leave => cursor.metadata.leave(&state.positions, &cursor.name),
                cursor_session::Event::Position { x, y } if cursor.metadata.inside => {
                    cursor.metadata.point = Some([x, y]);
                    real_position = true;
                }
                _ => {}
            }
            state.publish(*id, real_position);
        }
    }

    impl Dispatch<wl_callback::WlCallback, ()> for State {
        fn event(state: &mut Self, _: &wl_callback::WlCallback, _: wl_callback::Event,
            _: &(), _: &Connection, _: &QueueHandle<Self>)
        {
            state.synced = true;
        }
    }

    wayland_client::delegate_noop!(State: ignore wl_pointer::WlPointer);
    wayland_client::delegate_noop!(State: ignore source::ExtImageCaptureSourceV1);
    wayland_client::delegate_noop!(State: ignore source_manager::ExtOutputImageCaptureSourceManagerV1);
    wayland_client::delegate_noop!(State: ignore capture_manager::ExtImageCopyCaptureManagerV1);
    wayland_client::delegate_noop!(State: ignore xdg_manager::ZxdgOutputManagerV1);

    async fn connect() -> Result<Connection> {
        // Never consume WAYLAND_SOCKET: it may belong to the application's existing connection.
        let display = std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into());
        let display = PathBuf::from(display);
        let path = if display.is_absolute() {
            display
        } else {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from).filter(|path| path.is_absolute())
                .ok_or("EXT cursor: no absolute XDG_RUNTIME_DIR for dedicated connection")?;
            runtime.join(display)
        };
        let socket = tokio::net::UnixStream::connect(&path).await
            .map_err(|error| format!("EXT cursor Wayland connect ({}): {error}", path.display()))?;
        let socket = socket.into_std().map_err(|error| format!("EXT cursor Wayland socket: {error}"))?;
        Connection::from_socket(socket).map_err(|error| format!("EXT cursor Wayland connection: {error}"))
    }

    fn would_block(error: &WaylandError) -> bool {
        matches!(error, WaylandError::Io(error) if error.kind() == ErrorKind::WouldBlock)
    }

    async fn dispatch(fd: &AsyncFd<BorrowedFd<'_>>, queue: &mut EventQueue<State>, state: &mut State)
        -> Result<()>
    {
        loop {
            if queue.dispatch_pending(state).map_err(|error| format!("EXT cursor dispatch: {error}"))? > 0 {
                return Ok(());
            }
            let pending_flush = match queue.flush() {
                Ok(()) => false,
                Err(error) if would_block(&error) => true,
                Err(error) => return Err(format!("EXT cursor flush: {error}")),
            };
            let Some(reader) = queue.prepare_read() else { continue };
            // This dedicated connection has no other readers; read() cannot wait on another queue.
            tokio::select! {
                result = fd.writable(), if pending_flush => {
                    let mut ready = result.map_err(|error| format!("EXT cursor write readiness: {error}"))?;
                    ready.clear_ready();
                    drop(reader);
                }
                result = fd.readable() => {
                    let mut ready = result.map_err(|error| format!("EXT cursor read readiness: {error}"))?;
                    let result = reader.read();
                    ready.clear_ready();
                    match result {
                        Ok(_) => {}
                        Err(error) if would_block(&error) => {}
                        Err(error) => return Err(format!("EXT cursor Wayland read: {error}")),
                    }
                }
            }
        }
    }

    async fn roundtrip(connection: &Connection, fd: &AsyncFd<BorrowedFd<'_>>,
        queue: &mut EventQueue<State>, state: &mut State) -> Result<()>
    {
        state.synced = false;
        connection.display().sync(&queue.handle(), ());
        while !state.synced {
            dispatch(fd, queue, state).await?;
            if let Some(error) = state.error.take() { return Err(error); }
        }
        Ok(())
    }

    async fn flush(connection: &Connection, fd: &AsyncFd<BorrowedFd<'_>>) -> Result<()> {
        loop {
            match connection.flush() {
                Ok(()) => return Ok(()),
                Err(error) if would_block(&error) => {
                    let mut ready = fd.writable().await
                        .map_err(|error| format!("EXT cursor cleanup readiness: {error}"))?;
                    ready.clear_ready();
                }
                Err(error) => return Err(format!("EXT cursor cleanup flush: {error}")),
            }
        }
    }

    // false means missing EXT protocols, not a failed EXT session or fabricated success.
    pub(super) async fn run(outputs: &[String], positions: &Positions, stop: &AtomicBool) -> Result<bool> {
        let connection = tokio::select! {
            biased;
            _ = stopped(stop) => return Ok(true),
            result = timeout(SETUP_TIMEOUT, connect()) => {
                result.map_err(|_| "EXT cursor Wayland connection timed out".to_owned())??
            }
        };
        let fd = AsyncFd::new(connection.as_fd())
            .map_err(|error| format!("EXT cursor Wayland fd: {error}"))?;
        let mut queue = connection.new_event_queue();
        let qh = queue.handle();
        let _registry = connection.display().get_registry(&qh, ());
        let mut state = State::new(outputs, positions);
        let result = async {
            let available = tokio::select! {
                biased;
                _ = stopped(stop) => return Ok(true),
                result = timeout(SETUP_TIMEOUT, async {
                    roundtrip(&connection, &fd, &mut queue, &mut state).await?;
                    if !state.capabilities().protocols() { return Ok(false); }
                    roundtrip(&connection, &fd, &mut queue, &mut state).await?;
                    let capabilities = state.capabilities();
                    if !capabilities.can_track() {
                        return Err(format!("EXT cursor capabilities incomplete: pointer={}, matching output with dimensions={}",
                            capabilities.pointer, capabilities.output));
                    }
                    state.reconcile(&qh);
                    // Check server acceptance without requiring the cursor to be on an output.
                    roundtrip(&connection, &fd, &mut queue, &mut state).await?;
                    Ok(true)
                }) => result.map_err(|_| "EXT cursor setup timed out".to_owned())??,
            };
            if !available { return Ok(false); }
            loop {
                if stop.load(Ordering::Acquire) { return Ok(true); }
                if let Some(error) = state.error.take() { return Err(error); }
                state.reconcile(&qh);
                tokio::select! {
                    biased;
                    _ = stopped(stop) => return Ok(true),
                    result = dispatch(&fd, &mut queue, &mut state) => result?,
                }
            }
        }.await;
        state.destroy();
        match timeout(CLOSE_TIMEOUT, flush(&connection, &fd)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => eprintln!("aura-effects: {error}"),
            Err(_) => eprintln!("aura-effects: EXT cursor cleanup flush timed out; disconnecting"),
        }
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::{Arc, Mutex};

        // Local proxies and directly dispatched events: no compositor or image capture.
        struct Fixture {
            state: State,
            connection: Connection,
            queue: EventQueue<State>,
            registry: wl_registry::WlRegistry,
            _peer: std::os::unix::net::UnixStream,
        }

        impl Fixture {
            fn new() -> Self {
                let (socket, peer) = std::os::unix::net::UnixStream::pair().unwrap();
                socket.set_nonblocking(true).unwrap();
                let connection = Connection::from_socket(socket).unwrap();
                let queue = connection.new_event_queue();
                let registry = connection.display().get_registry(&queue.handle(), ());
                let positions = Arc::new(Mutex::new(HashMap::from([("eDP-1".into(), [0.0; 2])])));
                Self { state: State::new(&["eDP-1".into()], &positions), connection, queue, registry, _peer: peer }
            }

            fn registry_event(&mut self, event: wl_registry::Event) {
                <State as Dispatch<wl_registry::WlRegistry, ()>>::event(
                    &mut self.state, &self.registry, event, &(), &self.connection, &self.queue.handle());
            }

            fn global(&mut self, name: u32, interface: &str, version: u32) {
                self.registry_event(wl_registry::Event::Global { name, interface: interface.into(), version });
            }

            fn pointer(&mut self, present: bool) {
                let seat = self.state.seats[&4].proxy.clone();
                let capabilities = if present { wl_seat::Capability::Pointer } else { wl_seat::Capability::empty() };
                <State as Dispatch<wl_seat::WlSeat, u32>>::event(&mut self.state, &seat,
                    wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) },
                    &4, &self.connection, &self.queue.handle());
                self.state.reconcile(&self.queue.handle());
            }

            fn cursor(&mut self, event: cursor_session::Event) {
                let session = self.state.cursors[&3].session.clone();
                <State as Dispatch<cursor_session::ExtImageCopyCaptureCursorSessionV1, u32>>::event(
                    &mut self.state, &session, event, &3, &self.connection, &self.queue.handle());
            }

            fn ready() -> Self {
                let mut fixture = Self::new();
                fixture.global(1, "ext_output_image_capture_source_manager_v1", 1);
                fixture.global(2, "ext_image_copy_capture_manager_v1", 1);
                fixture.global(3, "wl_output", 4);
                fixture.global(4, "wl_seat", 9);
                let output = fixture.state.outputs.get_mut(&3).unwrap();
                output.pending_name = Some("eDP-1".into());
                output.pending_size = OutputSize { mode: [100, 100], scale: 1,
                    transform: Some(wl_output::Transform::Normal), logical: None };
                output.commit();
                fixture.pointer(true);
                fixture
            }

            fn position(&self) -> [f32; 2] {
                self.state.positions.lock().unwrap()["eDP-1"]
            }
        }

        #[test]
        fn registry_detection_and_active_log_require_real_capabilities_and_metadata() {
            let mut fixture = Fixture::new();
            fixture.global(1, "ext_output_image_capture_source_manager_v1", 0);
            assert!(!fixture.state.capabilities().protocols());
            fixture.global(1, "ext_output_image_capture_source_manager_v1", 1);
            assert!(!fixture.state.capabilities().protocols());
            fixture.global(2, "ext_image_copy_capture_manager_v1", 1);
            assert!(fixture.state.capabilities().protocols());
            assert!(!fixture.state.capabilities().can_track());
            let mut fixture = Fixture::ready();
            assert!(fixture.state.capabilities().can_track());
            assert!(!fixture.state.active_logged);
            fixture.global(5, "cosmic_workspace_manager_v2", 2);
            assert!(!fixture.state.logical_coordinates);
            fixture.global(6, "hyprland_toplevel_export_manager_v1", 2);
            assert!(fixture.state.logical_coordinates);
            fixture.cursor(cursor_session::Event::Enter);
            assert!(!fixture.state.active_logged);
            fixture.cursor(cursor_session::Event::Position { x: 75, y: 25 });
            assert!(fixture.state.active_logged);
            assert_eq!(fixture.position(), [0.5, -0.5]);
        }

        #[test]
        fn output_leave_and_registry_removal_clear_live_session_metadata() {
            let mut fixture = Fixture::ready();
            fixture.cursor(cursor_session::Event::Enter);
            fixture.cursor(cursor_session::Event::Position { x: 75, y: 25 });
            assert_eq!(fixture.position(), [0.5, -0.5]);
            fixture.cursor(cursor_session::Event::Leave);
            assert_eq!(fixture.position(), [0.0; 2]);
            fixture.cursor(cursor_session::Event::Enter);
            fixture.cursor(cursor_session::Event::Position { x: 75, y: 25 });
            fixture.registry_event(wl_registry::Event::GlobalRemove { name: 3 });
            assert!(fixture.state.outputs.is_empty());
            assert!(fixture.state.cursors.is_empty());
            assert_eq!(fixture.position(), [0.0; 2]);
            fixture.state.reconcile(&fixture.queue.handle());
            assert!(fixture.state.cursors.is_empty());
        }

        #[test]
        fn pointer_loss_and_shutdown_center_without_losing_mutter_position_keys() {
            let mut fixture = Fixture::ready();
            fixture.cursor(cursor_session::Event::Enter);
            fixture.cursor(cursor_session::Event::Position { x: 75, y: 25 });
            fixture.pointer(false);
            assert!(fixture.state.cursors.is_empty());
            assert!(!fixture.state.capabilities().can_track());
            assert_eq!(fixture.position(), [0.0; 2]);
            fixture.pointer(true);
            assert_eq!(fixture.state.cursors.len(), 1);
            assert_eq!(fixture.position(), [0.0; 2]);
            fixture.cursor(cursor_session::Event::Enter);
            fixture.cursor(cursor_session::Event::Position { x: 75, y: 25 });
            fixture.state.destroy();
            assert!(fixture.state.cursors.is_empty());
            assert_eq!(fixture.position(), [0.0; 2]);
            assert_eq!(fixture.state.positions.lock().unwrap().len(), 1);
        }

        #[test]
        fn manager_removal_is_an_error_not_an_unsupported_protocol_fallback() {
            let mut fixture = Fixture::ready();
            fixture.cursor(cursor_session::Event::Enter);
            fixture.cursor(cursor_session::Event::Position { x: 75, y: 25 });
            fixture.registry_event(wl_registry::Event::GlobalRemove { name: 2 });
            assert!(fixture.state.error.is_some());
            assert!(fixture.state.cursors.is_empty());
            assert_eq!(fixture.position(), [0.0; 2]);
        }

        #[tokio::test(flavor = "current_thread")]
        async fn silent_compositor_roundtrip_is_cancelable() {
            let mut fixture = Fixture::new();
            let stop = Arc::new(AtomicBool::new(false));
            let signal = stop.clone();
            let stopper = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                signal.store(true, Ordering::Release);
            });
            let fd = AsyncFd::new(fixture.connection.as_fd()).unwrap();
            timeout(std::time::Duration::from_secs(1), async {
                tokio::select! {
                    _ = stopped(&stop) => {}
                    result = roundtrip(&fixture.connection, &fd, &mut fixture.queue, &mut fixture.state) => {
                        panic!("a peer without a compositor must not complete the roundtrip: {result:?}");
                    }
                }
            }).await.unwrap();
            assert!(!fixture.state.synced);
            fixture.state.destroy();
            timeout(CLOSE_TIMEOUT, flush(&fixture.connection, &fd)).await.unwrap().unwrap();
            stopper.await.unwrap();
        }

        #[test]
        fn capabilities_require_both_protocols_pointer_and_output() {
            for mask in 0..16 {
                let capabilities = Capabilities {
                    source: mask & 1 != 0, capture: mask & 2 != 0,
                    pointer: mask & 4 != 0, output: mask & 8 != 0,
                };
                assert_eq!(capabilities.protocols(), mask & 3 == 3);
                assert_eq!(capabilities.can_track(), mask == 15);
            }
        }

        #[test]
        fn pixels_are_transformed_but_not_divided_by_scale() {
            for transform in [wl_output::Transform::Normal, wl_output::Transform::_180,
                wl_output::Transform::Flipped, wl_output::Transform::Flipped180]
            {
                let size = OutputSize { mode: [3840, 2160], scale: 2, transform: Some(transform), logical: None };
                assert_eq!(size.dimensions(false), Some([3840.0, 2160.0]));
                assert_eq!(size.dimensions(true), Some([1920.0, 1080.0]));
                assert_eq!(normalize([2880, 540], size.dimensions(false).unwrap()), Some([0.5, -0.5]));
            }
            for transform in [wl_output::Transform::_90, wl_output::Transform::_270,
                wl_output::Transform::Flipped90, wl_output::Transform::Flipped270]
            {
                let size = OutputSize { mode: [3840, 2160], scale: 2, transform: Some(transform), logical: None };
                assert_eq!(size.dimensions(false), Some([2160.0, 3840.0]));
                assert_eq!(size.dimensions(true), Some([1080.0, 1920.0]));
                assert_eq!(normalize([540, 2880], size.dimensions(false).unwrap()), Some([-0.5, 0.5]));
            }
        }

        #[test]
        fn hyprland_uses_real_logical_box_even_with_fractional_scale() {
            let size = OutputSize { mode: [3840, 2160], scale: 2,
                transform: Some(wl_output::Transform::_90), logical: Some([1440, 2560]) };
            assert_eq!(size.dimensions(false), Some([2160.0, 3840.0]));
            assert_eq!(size.dimensions(true), Some([1440.0, 2560.0]));
            assert_eq!(normalize([1080, 640], size.dimensions(true).unwrap()), Some([0.5, -0.5]));
        }

        #[test]
        fn missing_or_invalid_dimensions_do_not_make_up_coordinates() {
            let valid = OutputSize { mode: [1366, 768], scale: 1,
                transform: Some(wl_output::Transform::Normal), logical: None };
            assert_eq!(normalize([683, 384], valid.dimensions(false).unwrap()), Some([0.0; 2]));
            assert_eq!(OutputSize::default().dimensions(false), None);
            assert_eq!(OutputSize { scale: 0, ..valid }.dimensions(true), None);
            assert_eq!(OutputSize { mode: [0, 768], ..valid }.dimensions(false), None);
            assert_eq!(OutputSize { transform: None, ..valid }.dimensions(false), None);
            assert_eq!(OutputSize { logical: Some([0, 768]), ..valid }.dimensions(true), None);
            assert_eq!(normalize([0, 0], [f64::NAN, 100.0]), None);
            assert_eq!(normalize([-10, 110], [100.0; 2]), Some([-1.0, 1.0]));
        }

        #[test]
        fn enter_alone_leave_and_output_withdrawal_are_centered() {
            let positions = Arc::new(Mutex::new(HashMap::from([("eDP-1".into(), [0.5, -0.5])])));
            let mut metadata = Metadata::default();
            metadata.enter();
            assert_eq!(metadata.position(Some([100.0; 2])), None);
            metadata.point = Some([75, 25]);
            assert_eq!(metadata.position(Some([100.0; 2])), Some([0.5, -0.5]));
            // Removing the output also removes the dimensions before session destruction.
            assert_eq!(metadata.position(None), None);
            metadata.leave(&positions, "eDP-1");
            assert_eq!(positions.lock().unwrap()["eDP-1"], [0.0; 2]);
            assert_eq!(metadata.point, None);
            assert_eq!(metadata.position(Some([100.0; 2])), None);
            metadata.enter();
            assert_eq!(metadata.position(Some([100.0; 2])), None);
        }
    }
}

struct DbusStreams {
    nodes: Vec<(String, u32)>,
    closed: SignalStream<'static>,
    owner_changed: OwnerChangedStream<'static>,
}

async fn setup_dbus(
    connection: &Connection,
    session_path: &mut Option<OwnedObjectPath>,
    outputs: Vec<String>,
) -> Result<DbusStreams> {
    let screencast = Proxy::new(connection, SERVICE, ROOT, SERVICE)
        .await
        .map_err(|error| format!("ScreenCast proxy: {error}"))?;
    let mut owner_changed = screencast
        .receive_owner_changed()
        .await
        .map_err(|error| format!("ScreenCast owner subscription: {error}"))?;
    let properties: HashMap<&str, Value<'_>> = HashMap::new();
    let path: OwnedObjectPath = screencast
        .call("CreateSession", &(properties,))
        .await
        .map_err(|error| format!("ScreenCast CreateSession: {error}"))?;
    *session_path = Some(path.clone());
    let session = Proxy::new_owned(connection.clone(), SERVICE, path, SESSION_INTERFACE)
        .await
        .map_err(|error| format!("ScreenCast session proxy: {error}"))?;
    let mut closed = session
        .receive_signal("Closed")
        .await
        .map_err(|error| format!("ScreenCast Closed subscription: {error}"))?;
    let mut pending = Vec::with_capacity(outputs.len());
    for output in outputs {
        let properties = HashMap::from([("cursor-mode", Value::U32(2))]);
        let path: OwnedObjectPath = session
            .call("RecordMonitor", &(&output, properties))
            .await
            .map_err(|error| format!("ScreenCast RecordMonitor({output}): {error}"))?;
        let stream = Proxy::new_owned(connection.clone(), SERVICE, path, STREAM_INTERFACE)
            .await
            .map_err(|error| format!("ScreenCast stream proxy ({output}): {error}"))?;
        let signals = stream
            .receive_signal("PipeWireStreamAdded")
            .await
            .map_err(|error| format!("PipeWireStreamAdded subscription ({output}): {error}"))?;
        pending.push((output, signals));
    }
    session
        .call::<_, _, ()>("Start", &())
        .await
        .map_err(|error| format!("ScreenCast Start: {error}"))?;
    let nodes = tokio::select! {
        biased;
        _ = connection.closed() => return Err("D-Bus disconnected during ScreenCast setup".into()),
        _ = closed.next() => return Err("ScreenCast session closed during setup".into()),
        _ = owner_changed.next() => return Err("ScreenCast owner disappeared or changed during setup".into()),
        result = try_join_all(pending.into_iter().map(|(output, mut signals)| async move {
            let message = timeout(NODE_TIMEOUT, signals.next()).await
                .map_err(|_| format!("PipeWireStreamAdded timed out ({output})"))?
                .ok_or_else(|| format!("ScreenCast stream disappeared ({output})"))?;
            let (node,): (u32,) = message.body().deserialize()
                .map_err(|error| format!("PipeWireStreamAdded node ({output}): {error}"))?;
            if node == 0 || node == pw::constants::ID_ANY {
                return Err(format!("invalid PipeWire node {node} ({output})"));
            }
            Ok((output, node))
        })) => result?,
    };
    Ok(DbusStreams {
        nodes,
        closed,
        owner_changed,
    })
}

struct StreamData {
    output: String,
    positions: Positions,
    format: VideoInfoRaw,
    metadata: Rc<Vec<u8>>,
    ready: Rc<Cell<bool>>,
    failed: Rc<Cell<bool>>,
    missing_metadata_logged: bool,
}

impl StreamData {
    fn fail(&self, error: impl std::fmt::Display) {
        eprintln!(
            "aura-effects: cursor PipeWire stream ({}): {error}",
            self.output
        );
        self.failed.set(true);
        self.ready.set(false);
        self.set_position([0.0; 2]);
    }

    fn set_position(&self, position: [f32; 2]) {
        if let Some(value) = self
            .positions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(&self.output)
        {
            *value = position;
        }
    }
}

struct CursorStream {
    _listener: pw::stream::StreamListener<StreamData>,
    stream: pw::stream::StreamRc,
    ready: Rc<Cell<bool>>,
}

async fn run_pipewire(
    connection: &Connection,
    mut dbus: DbusStreams,
    positions: &Positions,
    stop: &AtomicBool,
) -> Result<()> {
    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|error| format!("PipeWire main loop: {error}"))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|error| format!("PipeWire context: {error}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|error| format!("PipeWire connection: {error}"))?;
    let failed = Rc::new(Cell::new(false));
    let core_failed = failed.clone();
    let _core_listener = core
        .add_listener_local()
        .error(move |id, _, code, message| {
            eprintln!(
                "aura-effects: cursor PipeWire core error {code} (object {id}): {message}"
            );
            core_failed.set(true);
        })
        .register();
    let metadata = Rc::new(metadata_param()?);
    let format = format_param()?;
    let format_pod = Pod::from_bytes(&format).ok_or("invalid PipeWire format pod")?;
    let mut streams = Vec::with_capacity(dbus.nodes.len());
    for (output, node) in dbus.nodes.drain(..) {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        let stream = pw::stream::StreamRc::new(
            core.clone(),
            "aura-effects-cursor",
            properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
                "node.rate" => "1/30",
                "node.latency" => "1/30",
            },
        )
        .map_err(|error| format!("PipeWire stream ({output}): {error}"))?;
        let ready = Rc::new(Cell::new(false));
        let data = StreamData {
            output: output.clone(),
            positions: positions.clone(),
            format: VideoInfoRaw::new(),
            metadata: metadata.clone(),
            ready: ready.clone(),
            failed: failed.clone(),
            missing_metadata_logged: false,
        };
        let listener = stream
            .add_local_listener_with_user_data(data)
            .state_changed(|_, data, old, new| match new {
                pw::stream::StreamState::Error(error) => data.fail(error),
                pw::stream::StreamState::Unconnected
                    if old != pw::stream::StreamState::Unconnected =>
                {
                    data.fail("stream disconnected");
                }
                _ => {}
            })
            .param_changed(format_changed)
            .process(process_cursor)
            .register()
            .map_err(|error| format!("PipeWire stream listener ({output}): {error}"))?;
        let cursor_stream = CursorStream {
            _listener: listener,
            stream,
            ready,
        };
        cursor_stream
            .stream
            .connect(
                spa::utils::Direction::Input,
                Some(node),
                pw::stream::StreamFlags::AUTOCONNECT
                    | pw::stream::StreamFlags::DONT_RECONNECT
                    | pw::stream::StreamFlags::NO_CONVERT,
                &mut [format_pod],
            )
            .map_err(|error| format!("PipeWire connect node {node} ({output}): {error}"))?;
        streams.push(cursor_stream);
    }
    let fd = AsyncFd::new(mainloop.loop_().fd())
        .map_err(|error| format!("PipeWire loop fd: {error}"))?;
    let startup = tokio::time::sleep(SETUP_TIMEOUT);
    tokio::pin!(startup);
    let mut ready = false;
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if failed.get() {
            return Err("PipeWire session/stream failed".into());
        }
        if !ready
            && streams.iter().all(|stream| {
                stream.ready.get()
                    && matches!(
                        stream.stream.state(),
                        pw::stream::StreamState::Paused | pw::stream::StreamState::Streaming
                    )
            })
        {
            ready = true;
            eprintln!(
                "aura-effects: cursor metadata tracking active on {} output(s)",
                streams.len()
            );
        }
        tokio::select! {
            biased;
            _ = stopped(stop) => return Ok(()),
            _ = connection.closed() => return Err("D-Bus disconnected".into()),
            _ = dbus.closed.next() => return Err("ScreenCast session closed".into()),
            _ = dbus.owner_changed.next() => return Err("ScreenCast owner disappeared or changed".into()),
            _ = &mut startup, if !ready => return Err("PipeWire cursor format negotiation timed out".into()),
            result = fd.readable() => {
                let mut guard = result.map_err(|error| format!("PipeWire loop readiness: {error}"))?;
                let result = mainloop.loop_().iterate(Duration::ZERO);
                guard.clear_ready();
                if result < 0 {
                    return Err(format!("PipeWire loop iteration: {}", std::io::Error::from_raw_os_error(-result)));
                }
            }
        }
    }
}

fn format_changed(
    stream: &pw::stream::Stream,
    data: &mut StreamData,
    id: u32,
    param: Option<&Pod>,
) {
    if id != spa::param::ParamType::Format.as_raw() {
        return;
    }
    let Some(param) = param else {
        if data.ready.get() {
            data.fail("negotiated format removed");
        }
        return;
    };
    data.ready.set(false);
    data.set_position([0.0; 2]);
    let media = spa::param::format_utils::parse_format(param);
    if !matches!(
        media,
        Ok((
            spa::param::format::MediaType::Video,
            spa::param::format::MediaSubtype::Raw
        ))
    ) {
        data.fail("expected raw video format");
        return;
    }
    if let Err(error) = data.format.parse(param) {
        data.fail(format!("cannot parse raw video format: {error}"));
        return;
    }
    let size = data.format.size();
    let video_format = data.format.format();
    if size.width == 0
        || size.height == 0
        || !matches!(
            video_format,
            spa::param::video::VideoFormat::BGRx
                | spa::param::video::VideoFormat::RGBx
                | spa::param::video::VideoFormat::BGRA
                | spa::param::video::VideoFormat::RGBA
        )
    {
        data.fail("unsupported raw video format or empty dimensions");
        return;
    }
    let Some(metadata) = Pod::from_bytes(&data.metadata) else {
        data.fail("invalid cursor metadata pod");
        return;
    };
    if let Err(error) = stream.update_params(&mut [metadata]) {
        data.fail(format!("cursor metadata negotiation: {error}"));
        return;
    }
    data.missing_metadata_logged = false;
}

struct Requeue<'a> {
    stream: &'a pw::stream::Stream,
    buffer: NonNull<pw::sys::pw_buffer>,
}

impl Drop for Requeue<'_> {
    fn drop(&mut self) {
        unsafe {
            self.stream.queue_raw_buffer(self.buffer.as_ptr());
        }
    }
}

fn process_cursor(stream: &pw::stream::Stream, data: &mut StreamData) {
    let Some(buffer) = NonNull::new(unsafe { stream.dequeue_raw_buffer() }) else {
        return;
    };
    let buffer = Requeue { stream, buffer };
    let size = data.format.size();
    // MAP_BUFFERS is deliberately absent: only the metadata header is ever read.
    let position = unsafe {
        cursor_metadata(
            buffer.buffer.as_ref().buffer,
            [size.width, size.height],
        )
    };
    if position.is_some() {
        data.ready.set(true);
    }
    if position.is_none() && size.width > 0 && !data.missing_metadata_logged {
        eprintln!(
            "aura-effects: cursor metadata absent or invalid ({}); parallax centered",
            data.output
        );
        data.missing_metadata_logged = true;
    }
    data.set_position(position.unwrap_or([0.0; 2]));
}

unsafe fn cursor_metadata(buffer: *const spa::sys::spa_buffer, size: [u32; 2]) -> Option<[f32; 2]> {
    let buffer = buffer.as_ref()?;
    if buffer.metas.is_null() || buffer.n_metas == 0 || buffer.n_metas > 64 {
        return None;
    }
    for index in 0..buffer.n_metas as usize {
        let meta = std::ptr::read_unaligned(buffer.metas.add(index));
        if meta.type_ != spa::sys::SPA_META_Cursor {
            continue;
        }
        if meta.data.is_null()
            || (meta.size as usize) < std::mem::size_of::<spa::sys::spa_meta_cursor>()
        {
            return None;
        }
        let cursor = std::ptr::read_unaligned(meta.data.cast::<spa::sys::spa_meta_cursor>());
        return Some(normalize_position(
            cursor.id,
            [cursor.position.x, cursor.position.y],
            size,
        ));
    }
    None
}

fn normalize_position(id: u32, position: [i32; 2], size: [u32; 2]) -> [f32; 2] {
    if id == 0
        || size.contains(&0)
        || position[0] < 0
        || position[1] < 0
        || position[0] as u32 >= size[0]
        || position[1] as u32 >= size[1]
    {
        return [0.0; 2];
    }
    [
        (2.0 * position[0] as f64 / size[0] as f64 - 1.0) as f32,
        (2.0 * position[1] as f64 / size[1] as f64 - 1.0) as f32,
    ]
}

fn serialize_object(object: spa::pod::Object) -> Result<Vec<u8>> {
    spa::pod::serialize::PodSerializer::serialize(
        Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .map(|(cursor, _)| cursor.into_inner())
    .map_err(|error| format!("PipeWire parameter serialization: {error}"))
}

fn metadata_param() -> Result<Vec<u8>> {
    serialize_object(spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamMeta.as_raw(),
        id: spa::param::ParamType::Meta.as_raw(),
        properties: vec![
            spa::pod::Property::new(
                spa::sys::SPA_PARAM_META_type,
                spa::pod::Value::Id(spa::utils::Id(spa::sys::SPA_META_Cursor)),
            ),
            spa::pod::Property::new(
                spa::sys::SPA_PARAM_META_size,
                spa::pod::Value::Choice(spa::pod::ChoiceValue::Int(spa::utils::Choice(
                    spa::utils::ChoiceFlags::empty(),
                    spa::utils::ChoiceEnum::Range {
                        default: CURSOR_META_SIZE as i32,
                        min: (std::mem::size_of::<spa::sys::spa_meta_cursor>()
                            + std::mem::size_of::<spa::sys::spa_meta_bitmap>()) as i32,
                        max: CURSOR_META_SIZE as i32,
                    },
                ))),
            ),
        ],
    })
}

fn format_param() -> Result<Vec<u8>> {
    use spa::{
        param::format::{FormatProperties, MediaSubtype, MediaType},
        param::video::VideoFormat,
    };
    serialize_object(spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        spa::pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        // Niri exposes DMA-BUF-only streams; their modifier is mandatory even without pixel access.
        spa::pod::Property {
            key: FormatProperties::VideoModifier.as_raw(),
            flags: spa::pod::PropertyFlags::MANDATORY,
            value: spa::pod::Value::Long(0),
        },
        spa::pod::property!(
            FormatProperties::VideoFormat, Choice, Enum, Id,
            VideoFormat::BGRx, VideoFormat::BGRx, VideoFormat::RGBx,
            VideoFormat::BGRA, VideoFormat::RGBA
        ),
        spa::pod::property!(
            FormatProperties::VideoSize, Choice, Range, Rectangle,
            spa::utils::Rectangle { width: 1920, height: 1080 },
            spa::utils::Rectangle { width: 1, height: 1 },
            spa::utils::Rectangle { width: 32768, height: 32768 }
        ),
        spa::pod::property!(
            FormatProperties::VideoFramerate, Choice, Range, Fraction,
            spa::utils::Fraction { num: 30, denom: 1 },
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 30, denom: 1 }
        ),
        spa::pod::property!(
            FormatProperties::VideoMaxFramerate, Fraction,
            spa::utils::Fraction { num: 30, denom: 1 }
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_and_absent_cursor() {
        assert_eq!(normalize_position(1, [960, 540], [1920, 1080]), [0.0; 2]);
        assert_eq!(normalize_position(0, [0, 0], [1920, 1080]), [0.0; 2]);
        assert_eq!(normalize_position(1, [0, 0], [0, 1080]), [0.0; 2]);
    }

    #[test]
    fn edges_and_axis_direction() {
        assert_eq!(normalize_position(1, [0, 0], [100, 100]), [-1.0; 2]);
        assert_eq!(normalize_position(1, [99, 99], [100, 100]), [0.98; 2]);
        assert_eq!(normalize_position(1, [75, 25], [100, 100]), [0.5, -0.5]);
    }

    #[test]
    fn outside_output_is_centered() {
        for point in [
            [-1, 50],
            [50, -1],
            [100, 50],
            [50, 100],
            [i32::MAX, i32::MIN],
        ] {
            assert_eq!(normalize_position(1, point, [100, 100]), [0.0; 2]);
        }
    }

    #[test]
    fn negotiated_size_handles_scaled_outputs() {
        assert_eq!(
            normalize_position(1, [480, 810], [1920, 1080]),
            normalize_position(1, [960, 1620], [3840, 2160])
        );
        assert_eq!(
            normalize_position(1, [960, 1620], [3840, 2160]),
            [-0.5, 0.5]
        );
    }

    #[test]
    fn only_valid_cursor_headers_are_read() {
        fn read(
            cursor: Option<&mut spa::sys::spa_meta_cursor>,
            type_: u32,
            size: u32,
        ) -> Option<[f32; 2]> {
            let mut meta = spa::sys::spa_meta {
                type_,
                size,
                data: cursor.map_or(std::ptr::null_mut(), |cursor| {
                    (cursor as *mut spa::sys::spa_meta_cursor).cast()
                }),
            };
            let buffer = spa::sys::spa_buffer {
                n_metas: 1,
                n_datas: 0,
                metas: std::ptr::addr_of_mut!(meta),
                datas: std::ptr::null_mut(),
            };
            unsafe { cursor_metadata(&buffer, [100, 100]) }
        }

        let mut cursor = spa::sys::spa_meta_cursor {
            id: 1,
            flags: 0,
            position: spa::sys::spa_point { x: 75, y: 25 },
            hotspot: spa::sys::spa_point { x: 0, y: 0 },
            bitmap_offset: u32::MAX,
        };
        let size = std::mem::size_of::<spa::sys::spa_meta_cursor>() as u32;
        let type_ = spa::sys::SPA_META_Cursor;
        assert_eq!(read(Some(&mut cursor), type_, size), Some([0.5, -0.5]));
        cursor.id = 0;
        assert_eq!(read(Some(&mut cursor), type_, size), Some([0.0; 2]));
        assert_eq!(read(Some(&mut cursor), type_, size - 1), None);
        assert_eq!(
            read(Some(&mut cursor), type_, CURSOR_META_SIZE as u32 + 1),
            Some([0.0; 2])
        );
        assert_eq!(read(Some(&mut cursor), spa::sys::SPA_META_Header, size), None);
        assert_eq!(read(None, type_, size), None);
        assert_eq!(
            unsafe { cursor_metadata(std::ptr::null(), [100, 100]) },
            None
        );
    }

    #[test]
    fn cursor_metadata_range_accepts_niri_bitmap_size() {
        let bytes = metadata_param().unwrap();
        let (_, value) = spa::pod::deserialize::PodDeserializer::deserialize_from::<spa::pod::Value>(
            &bytes,
        )
        .unwrap();
        let spa::pod::Value::Object(object) = value else {
            panic!("expected metadata object")
        };
        let size = object
            .properties
            .iter()
            .find(|property| property.key == spa::sys::SPA_PARAM_META_size)
            .unwrap();
        let spa::pod::Value::Choice(spa::pod::ChoiceValue::Int(spa::utils::Choice(
            _,
            spa::utils::ChoiceEnum::Range { min, max, .. },
        ))) = &size.value
        else {
            panic!("expected metadata size range")
        };
        assert!(*min <= 589872 && *max >= 589872);
    }

    #[test]
    fn video_format_accepts_niri_dma_buf_modifier() {
        let bytes = format_param().unwrap();
        let (_, value) = spa::pod::deserialize::PodDeserializer::deserialize_from::<spa::pod::Value>(
            &bytes,
        )
        .unwrap();
        let spa::pod::Value::Object(object) = value else {
            panic!("expected format object")
        };
        let modifier = object
            .properties
            .iter()
            .find(|property| {
                property.key == spa::param::format::FormatProperties::VideoModifier.as_raw()
            })
            .unwrap();
        assert!(modifier.flags.contains(spa::pod::PropertyFlags::MANDATORY));
        assert_eq!(modifier.value, spa::pod::Value::Long(0));
    }

    #[test]
    fn empty_tracker_is_centered_and_stops() {
        let tracker = CursorTracker::start(Vec::new());
        assert_eq!(tracker.position("DP-1"), [0.0; 2]);
    }
}
