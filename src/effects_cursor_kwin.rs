use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::StreamExt;
use serde::Deserialize;
use tempfile::NamedTempFile;
use tokio::{
    sync::watch,
    time::{timeout, timeout_at, Instant},
};
use zbus::{fdo::DBusProxy, message::Header, Connection, Proxy};

use super::{stopped, Positions, Result, CLOSE_TIMEOUT, SETUP_TIMEOUT};

const SERVICE: &str = "org.kde.KWin";
const SCRIPTING_PATH: &str = "/Scripting";
const SCRIPTING_INTERFACE: &str = "org.kde.kwin.Scripting";
const SCRIPT_INTERFACE: &str = "org.kde.kwin.Script";
const CURSOR_PATH: &str = "/io/github/antwny/Aura/Cursor";
const MAX_OUTPUTS: usize = 64;
const MAX_NAME_BYTES: usize = 256;
const MAX_SAMPLE_BYTES: usize = 64 * 1024;

type Status = Option<Result<()>>;

pub(super) async fn run(
    outputs: Vec<String>,
    positions: &Positions,
    stop: &AtomicBool,
) -> Result<bool> {
    let center = CenterOnExit {
        positions: positions.clone(),
        active: Arc::new(AtomicBool::new(true)),
    };
    let deadline = Instant::now() + SETUP_TIMEOUT;
    let connection = tokio::select! {
        biased;
        _ = stopped(stop) => return Ok(true),
        result = timeout_at(deadline, Connection::session()) => {
            result.map_err(|_| "KWin D-Bus connection timed out".to_owned())?
                .map_err(|error| format!("KWin D-Bus connection: {error}"))?
        }
    };
    let mut lease: Option<ScriptLease> = None;
    let mut result = async {
        let discovery = async {
            let bus = DBusProxy::new(&connection)
                .await
                .map_err(|error| format!("KWin bus proxy: {error}"))?;
            let owners = bus
                .receive_name_owner_changed_with_args(&[(0, SERVICE)])
                .await
                .map_err(|error| format!("KWin owner subscription: {error}"))?;
            let owner = match bus.get_name_owner(SERVICE.try_into().unwrap()).await {
                Ok(owner) => owner,
                Err(zbus::fdo::Error::NameHasNoOwner(_)) => return Ok(None),
                Err(error) => return Err(format!("KWin owner lookup: {error}")),
            };
            Ok(Some((owner, owners)))
        };
        let discovered = tokio::select! {
            biased;
            _ = stopped(stop) => return Ok(true),
            _ = connection.closed() => return Err("KWin D-Bus disconnected during discovery".into()),
            result = timeout_at(deadline, discovery) => {
                result.map_err(|_| "KWin discovery timed out".to_owned())??
            }
        };
        let Some((owner, mut owners)) = discovered else {
            return Ok(false);
        };
        if stop.load(Ordering::Acquire) {
            return Ok(true);
        }
        let expected = expected_outputs(outputs)?;
        let output_count = expected.len();
        let (events, mut status) = watch::channel(None);
        eprintln!("aura-effects: cursor backend KWin detected; loading temporary script");
        let setup = async {
            connection
                .object_server()
                .at(
                    CURSOR_PATH,
                    CursorReceiver {
                        owner: owner.to_string(),
                        expected,
                        positions: positions.clone(),
                        active: center.active.clone(),
                        events,
                    },
                )
                .await
                .map_err(|error| format!("KWin cursor receiver: {error}"))?;
            let destination = connection
                .unique_name()
                .ok_or("KWin cursor connection has no unique bus name")?;
            let source = script_source(destination.as_str())?;
            let mut file = tempfile::Builder::new()
                .prefix("aura-kwin-cursor-")
                .suffix(".js")
                .tempfile()
                .map_err(|error| format!("KWin cursor tempfile: {error}"))?;
            file.write_all(source.as_bytes())
                .and_then(|()| file.flush())
                .map_err(|error| format!("KWin cursor script write: {error}"))?;
            let path = file
                .path()
                .to_str()
                .ok_or("KWin cursor script path is not UTF-8")?
                .to_owned();
            let plugin = file
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("KWin cursor script name is not UTF-8")?
                .to_owned();
            let scripting = Proxy::new(
                &connection,
                owner.as_str(),
                SCRIPTING_PATH,
                SCRIPTING_INTERFACE,
            )
            .await
            .map_err(|error| format!("KWin scripting proxy: {error}"))?;
            // Keep the lease before loadScript: cancellation can lose its reply.
            lease = Some(ScriptLease {
                owner: owner.to_string(),
                plugin,
                _file: file,
                unload: true,
                confirmed: false,
            });
            let script = lease.as_mut().unwrap();
            let id: i32 = scripting
                .call("loadScript", &(&path, &script.plugin))
                .await
                .map_err(|error| format!("KWin loadScript: {error}"))?;
            if id < 0 {
                script.unload = false;
                return Err(format!("KWin loadScript returned invalid id {id}"));
            }
            script.confirmed = true;
            start_script(&connection, owner.as_str(), id).await?;
            wait_ready(&mut status).await
        };
        tokio::select! {
            biased;
            _ = stopped(stop) => return Ok(true),
            _ = connection.closed() => return Err("KWin D-Bus disconnected during setup".into()),
            _ = owners.next() => return Err("KWin owner disappeared or changed during setup".into()),
            result = timeout_at(deadline, setup) => {
                result.map_err(|_| "KWin cursor setup/readiness timed out".to_owned())??;
            }
        }
        eprintln!(
            "aura-effects: cursor backend KWin ready; logical cursor tracking on {output_count} output(s)"
        );
        loop {
            tokio::select! {
                biased;
                _ = stopped(stop) => return Ok(true),
                _ = connection.closed() => return Err("KWin D-Bus disconnected".into()),
                _ = owners.next() => return Err("KWin owner disappeared or changed".into()),
                changed = status.changed() => {
                    changed.map_err(|_| "KWin cursor receiver disappeared".to_owned())?;
                    if let Some(Err(error)) = status.borrow_and_update().clone() {
                        return Err(error);
                    }
                }
            }
        }
    }
    .await;

    center.center();
    if let Some(script) = lease.as_ref().filter(|script| script.unload) {
        let unloading = async {
            let reply = connection
                .call_method(
                    Some(script.owner.as_str()),
                    SCRIPTING_PATH,
                    Some(SCRIPTING_INTERFACE),
                    "unloadScript",
                    &(&script.plugin,),
                )
                .await
                .map_err(|error| format!("KWin unloadScript: {error}"))?;
            let unloaded: bool = reply
                .body()
                .deserialize()
                .map_err(|error| format!("KWin unloadScript reply: {error}"))?;
            if !unloaded && script.confirmed {
                return Err("KWin unloadScript could not find the loaded script".into());
            }
            Ok(())
        };
        match timeout(CLOSE_TIMEOUT, unloading).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => append_error(&mut result, error),
            Err(_) => append_error(&mut result, "KWin unloadScript timed out".into()),
        }
    }
    match timeout(CLOSE_TIMEOUT, connection.close()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => append_error(&mut result, format!("KWin D-Bus disconnect: {error}")),
        Err(_) => append_error(&mut result, "KWin D-Bus disconnect timed out".into()),
    }
    drop(lease);
    result
}

struct ScriptLease {
    owner: String,
    plugin: String,
    _file: NamedTempFile,
    unload: bool,
    confirmed: bool,
}

async fn start_script(connection: &Connection, owner: &str, id: i32) -> Result<()> {
    let modern = format!("/Scripting/Script{id}");
    let legacy = format!("/{id}");
    for (index, path) in [modern, legacy].iter().enumerate() {
        match connection
            .call_method(
                Some(owner),
                path.as_str(),
                Some(SCRIPT_INTERFACE),
                "run",
                &(),
            )
            .await
        {
            Ok(reply) => {
                return reply
                    .body()
                    .deserialize::<()>()
                    .map_err(|error| format!("KWin Script.run reply: {error}"));
            }
            Err(zbus::Error::MethodError(name, _, _))
                if index == 0
                    && matches!(
                        name.as_str(),
                        "org.freedesktop.DBus.Error.UnknownObject"
                            | "org.freedesktop.DBus.Error.UnknownMethod"
                            | "org.freedesktop.DBus.Error.UnknownInterface"
                    ) => {}
            Err(error) => return Err(format!("KWin Script.run ({path}): {error}")),
        }
    }
    Err("KWin Script.run unavailable".into())
}

async fn wait_ready(status: &mut watch::Receiver<Status>) -> Result<()> {
    loop {
        if let Some(result) = status.borrow_and_update().clone() {
            return result;
        }
        status
            .changed()
            .await
            .map_err(|_| "KWin cursor receiver disappeared before readiness".to_owned())?;
    }
}

fn append_error(result: &mut Result<bool>, error: String) {
    match result {
        Ok(_) => *result = Err(error),
        Err(previous) => {
            previous.push_str("; ");
            previous.push_str(&error);
        }
    }
}

struct CenterOnExit {
    positions: Positions,
    active: Arc<AtomicBool>,
}

impl CenterOnExit {
    fn center(&self) {
        let mut positions = self
            .positions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.active.store(false, Ordering::Release);
        positions
            .values_mut()
            .for_each(|position| *position = [0.0; 2]);
    }
}

impl Drop for CenterOnExit {
    fn drop(&mut self) {
        self.center();
    }
}

struct CursorReceiver {
    owner: String,
    expected: HashSet<String>,
    positions: Positions,
    active: Arc<AtomicBool>,
    events: watch::Sender<Status>,
}

#[zbus::interface(name = "io.github.antwny.Aura.Cursor")]
impl CursorReceiver {
    #[zbus(name = "Update")]
    async fn update(
        &self,
        payload: String,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<()> {
        if header.sender().map(|sender| sender.as_str()) != Some(self.owner.as_str()) {
            return Err(zbus::fdo::Error::AccessDenied(
                "cursor sender is not KWin".into(),
            ));
        }
        let sample = match sample_positions(&payload, &self.expected) {
            Ok(sample) => sample,
            Err(error) => {
                self.active.store(false, Ordering::Release);
                self.events
                    .send_replace(Some(Err(format!("KWin cursor sample: {error}"))));
                return Err(zbus::fdo::Error::InvalidArgs(error));
            }
        };
        let mut positions = self
            .positions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.active.load(Ordering::Acquire) {
            return Ok(());
        }
        for (name, position) in sample {
            if let Some(value) = positions.get_mut(&name) {
                *value = position;
            }
        }
        self.events.send_if_modified(|status| {
            if status.is_none() {
                *status = Some(Ok(()));
                true
            } else {
                false
            }
        });
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    cursor: [f64; 2],
    screens: Vec<Screen>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Screen {
    name: String,
    geometry: [f64; 4],
}

fn expected_outputs(outputs: Vec<String>) -> Result<HashSet<String>> {
    if outputs.len() > MAX_OUTPUTS || outputs.iter().any(|name| !valid_name(name)) {
        return Err("KWin cursor output names exceed limits or are empty".into());
    }
    Ok(outputs.into_iter().collect())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= MAX_NAME_BYTES
}

fn valid_geometry([x, y, width, height]: [f64; 4]) -> bool {
    [x, y, width, height, x + width, y + height]
        .iter()
        .all(|value| value.is_finite())
        && width > 0.0
        && height > 0.0
}

fn sample_positions(
    payload: &str,
    expected: &HashSet<String>,
) -> Result<HashMap<String, [f32; 2]>> {
    if payload.len() > MAX_SAMPLE_BYTES || expected.len() > MAX_OUTPUTS {
        return Err("cursor sample exceeds limits".into());
    }
    let sample: Sample =
        serde_json::from_str(payload).map_err(|error| format!("invalid cursor JSON: {error}"))?;
    if sample.screens.len() > MAX_OUTPUTS || !sample.cursor.iter().all(|value| value.is_finite()) {
        return Err("invalid cursor coordinates or too many screens".into());
    }
    let mut seen = HashSet::with_capacity(sample.screens.len());
    let mut positions: HashMap<_, _> = expected
        .iter()
        .map(|name| (name.clone(), [0.0; 2]))
        .collect();
    for screen in sample.screens {
        if !valid_name(&screen.name)
            || !valid_geometry(screen.geometry)
            || !seen.insert(screen.name.clone())
        {
            return Err("invalid or duplicate screen name/geometry".into());
        }
        if let Some(position) = positions.get_mut(&screen.name) {
            *position = normalize_position(sample.cursor, screen.geometry);
        }
    }
    Ok(positions)
}

fn normalize_position(cursor: [f64; 2], geometry: [f64; 4]) -> [f32; 2] {
    let [x, y, width, height] = geometry;
    if !valid_geometry(geometry)
        || !cursor.iter().all(|value| value.is_finite())
        || cursor[0] < x
        || cursor[1] < y
        || cursor[0] >= x + width
        || cursor[1] >= y + height
    {
        return [0.0; 2];
    }
    [
        (2.0 * ((cursor[0] - x) / width) - 1.0).clamp(-1.0, 1.0) as f32,
        (2.0 * ((cursor[1] - y) / height) - 1.0).clamp(-1.0, 1.0) as f32,
    ]
}

fn script_source(destination: &str) -> Result<String> {
    let destination = serde_json::to_string(destination)
        .map_err(|error| format!("KWin cursor bus name serialization: {error}"))?
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(format!(
        "(function () {{\n    var destination = {destination};\n{SCRIPT_BODY}\n}})();\n"
    ))
}

const SCRIPT_BODY: &str = r#"    var inFlight = false;
    var dirty = false;

    function sendLatest() {
        if (inFlight) {
            dirty = true;
            return;
        }
        dirty = false;
        var cursor = workspace.cursorPos;
        var screens = workspace.screens;
        var sample = { cursor: [cursor.x, cursor.y], screens: [] };
        for (var i = 0; i < screens.length; ++i) {
            var screen = screens[i];
            var geometry = screen.geometry;
            sample.screens.push({
                name: screen.name,
                geometry: [geometry.x, geometry.y, geometry.width, geometry.height]
            });
        }
        inFlight = true;
        callDBus(destination, "/io/github/antwny/Aura/Cursor",
            "io.github.antwny.Aura.Cursor", "Update", JSON.stringify(sample), function () {
                inFlight = false;
                if (dirty) {
                    sendLatest();
                }
            });
    }

    workspace.cursorPosChanged.connect(sendLatest);
    workspace.screensChanged.connect(sendLatest);
    sendLatest();"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn expected() -> HashSet<String> {
        expected_outputs(vec!["DP-1".into(), "HDMI-A-1".into()]).unwrap()
    }

    #[test]
    fn logical_position_and_axis_direction() {
        let geometry = [-1920.0, 200.0, 1920.0, 1080.0];
        assert_eq!(normalize_position([-960.0, 740.0], geometry), [0.0; 2]);
        assert_eq!(normalize_position([-1920.0, 200.0], geometry), [-1.0; 2]);
        assert_eq!(normalize_position([-480.0, 470.0], geometry), [0.5, -0.5]);
    }

    #[test]
    fn fractional_scale_uses_logical_geometry() {
        assert_eq!(
            normalize_position(
                [1280.0 / 1.5, 720.0 / 1.5],
                [0.0, 0.0, 2560.0 / 1.5, 1440.0 / 1.5]
            ),
            [0.0; 2]
        );
        assert_eq!(
            normalize_position([1280.0, 240.0], [0.0, 0.0, 2560.0 / 1.5, 1440.0 / 1.5]),
            [0.5, -0.5]
        );
        assert_eq!(
            normalize_position([1920.0, 360.0], [0.0, 0.0, 2560.0, 1440.0]),
            [0.5, -0.5]
        );
    }

    #[test]
    fn outside_output_and_invalid_numbers_are_centered() {
        for cursor in [
            [-1.0, 50.0],
            [100.0, 50.0],
            [50.0, -1.0],
            [50.0, 100.0],
            [f64::NAN, 0.0],
        ] {
            assert_eq!(
                normalize_position(cursor, [0.0, 0.0, 100.0, 100.0]),
                [0.0; 2]
            );
        }
        for geometry in [
            [0.0, 0.0, 0.0, 100.0],
            [0.0, 0.0, 100.0, -1.0],
            [f64::INFINITY, 0.0, 100.0, 100.0],
            [f64::MAX, 0.0, f64::MAX, 100.0],
        ] {
            assert_eq!(normalize_position([50.0, 50.0], geometry), [0.0; 2]);
        }
    }

    #[test]
    fn only_expected_outputs_are_updated_and_removed_outputs_center() {
        let sample = r#"{"cursor":[75,25],"screens":[{"name":"DP-1","geometry":[0,0,100,100]},{"name":"unrequested","geometry":[0,0,100,100]}]}"#;
        let positions = sample_positions(sample, &expected()).unwrap();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions["DP-1"], [0.5, -0.5]);
        assert_eq!(positions["HDMI-A-1"], [0.0; 2]);
        assert!(!positions.contains_key("unrequested"));
        let empty = sample_positions(r#"{"cursor":[75,25],"screens":[]}"#, &expected()).unwrap();
        assert!(empty.values().all(|position| *position == [0.0; 2]));
    }

    #[test]
    fn invalid_json_and_geometry_are_rejected_atomically() {
        for payload in [
            "{}",
            r#"{"cursor":[1e400,0],"screens":[]}"#,
            r#"{"cursor":[null,0],"screens":[]}"#,
            r#"{"cursor":[0,0,0],"screens":[]}"#,
            r#"{"cursor":[0,0],"screens":[],"extra":true}"#,
            r#"{"cursor":[0,0],"screens":[{"name":"DP-1","geometry":[0,0,0,100]}]}"#,
            r#"{"cursor":[0,0],"screens":[{"name":"DP-1","geometry":[0,0,100,100]},{"name":"DP-1","geometry":[0,0,100,100]}]}"#,
        ] {
            assert!(sample_positions(payload, &expected()).is_err(), "{payload}");
        }
    }

    #[test]
    fn payload_maps_and_names_are_bounded() {
        assert!(sample_positions(&" ".repeat(MAX_SAMPLE_BYTES + 1), &expected()).is_err());
        assert!(expected_outputs(vec!["DP-1".into(); MAX_OUTPUTS + 1]).is_err());
        assert!(expected_outputs(vec![String::new()]).is_err());
        assert!(expected_outputs(vec!["x".repeat(MAX_NAME_BYTES + 1)]).is_err());
        let screens: Vec<_> = (0..=MAX_OUTPUTS)
            .map(|i| serde_json::json!({"name": format!("DP-{i}"), "geometry": [0, 0, 100, 100]}))
            .collect();
        let payload = serde_json::json!({"cursor": [0, 0], "screens": screens}).to_string();
        assert!(sample_positions(&payload, &expected()).is_err());
    }

    #[test]
    fn exit_centers_without_removing_or_adding_outputs() {
        let positions = Arc::new(Mutex::new(HashMap::from([
            ("DP-1".into(), [0.75, -0.25]),
            ("other".into(), [-1.0, 1.0]),
        ])));
        let active = Arc::new(AtomicBool::new(true));
        drop(CenterOnExit {
            positions: positions.clone(),
            active: active.clone(),
        });
        assert!(!active.load(Ordering::Acquire));
        let positions = positions.lock().unwrap();
        assert_eq!(positions.len(), 2);
        assert!(positions.values().all(|position| *position == [0.0; 2]));
    }

    #[test]
    fn receiver_authenticates_sender_and_only_readies_after_a_valid_sample() {
        let positions = Arc::new(Mutex::new(HashMap::from([("DP-1".into(), [0.0; 2])])));
        let active = Arc::new(AtomicBool::new(true));
        let (events, status) = watch::channel(None);
        let receiver = CursorReceiver {
            owner: ":1.42".into(),
            expected: expected(),
            positions: positions.clone(),
            active: active.clone(),
            events,
        };
        let message = |sender| {
            zbus::Message::method_call(CURSOR_PATH, "Update")
                .unwrap()
                .sender(sender)
                .unwrap()
                .build(&())
                .unwrap()
        };
        let payload = r#"{"cursor":[75,25],"screens":[{"name":"DP-1","geometry":[0,0,100,100]}]}"#;
        let unauthorized = message(":1.99");
        assert!(matches!(
            futures::executor::block_on(receiver.update(payload.into(), unauthorized.header())),
            Err(zbus::fdo::Error::AccessDenied(_))
        ));
        assert!(status.borrow().is_none());
        assert_eq!(positions.lock().unwrap()["DP-1"], [0.0; 2]);
        let authorized = message(":1.42");
        futures::executor::block_on(receiver.update(payload.into(), authorized.header())).unwrap();
        assert_eq!(*status.borrow(), Some(Ok(())));
        assert_eq!(positions.lock().unwrap()["DP-1"], [0.5, -0.5]);
        assert_eq!(positions.lock().unwrap().len(), 1);
        assert!(
            futures::executor::block_on(receiver.update("{}".into(), authorized.header())).is_err()
        );
        assert!(matches!(*status.borrow(), Some(Err(_))));
        assert!(!active.load(Ordering::Acquire));
        let center = CenterOnExit {
            positions: positions.clone(),
            active,
        };
        center.center();
        futures::executor::block_on(receiver.update(payload.into(), authorized.header())).unwrap();
        assert_eq!(positions.lock().unwrap()["DP-1"], [0.0; 2]);
        assert!(matches!(*status.borrow(), Some(Err(_))));
    }

    #[test]
    fn cleanup_errors_are_propagated_without_losing_the_original_error() {
        let mut result = Ok(true);
        append_error(&mut result, "unload failed".into());
        append_error(&mut result, "disconnect failed".into());
        assert_eq!(result, Err("unload failed; disconnect failed".into()));
        let mut result = Err("setup failed".into());
        append_error(&mut result, "unload failed".into());
        assert_eq!(result, Err("setup failed; unload failed".into()));
    }

    #[test]
    fn bus_name_is_only_a_json_literal() {
        for destination in [
            ":1.42",
            "\"; throw new Error('injected'); //\n\\\u{2028}\u{2029}",
        ] {
            let source = script_source(destination).unwrap();
            let literal = source
                .lines()
                .nth(1)
                .unwrap()
                .strip_prefix("    var destination = ")
                .unwrap()
                .strip_suffix(';')
                .unwrap();
            assert_eq!(
                serde_json::from_str::<String>(literal).unwrap(),
                destination
            );
            assert!(source.ends_with("    sendLatest();\n})();\n"));
            assert!(!source.contains("QTimer"));
        }
    }
}
