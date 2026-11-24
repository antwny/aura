//! Detect external media players through the session MPRIS interface.

use std::time::Duration;
use zbus::{fdo::DBusProxy, Connection, Proxy};

pub async fn external_media_playing() -> Option<bool> {
    tokio::time::timeout(Duration::from_secs(2), query()).await.ok()?.ok()
}

async fn query() -> zbus::Result<bool> {
    let connection = Connection::session().await?;
    let bus = DBusProxy::new(&connection).await?;
    let names = bus.list_names().await?;
    for name in names {
        let name = name.as_str();
        if !name.starts_with("org.mpris.MediaPlayer2.") {
            continue;
        }
        let Ok(proxy) = Proxy::new(
            &connection,
            name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        ).await else { continue };
        if proxy.get_property::<String>("PlaybackStatus").await.as_deref() == Ok("Playing") {
            return Ok(true);
        }
    }
    Ok(false)
}
