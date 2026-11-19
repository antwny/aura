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
