//! Audio policy and backend dispatch for wallpaper playback.

use std::sync::Arc;
use cosmic::iced::Task;
use super::{AuraApp, Backend, Message};

fn effective_mute_state(manual: bool, auto_enabled: bool, external_playing: bool) -> bool {
    manual || (auto_enabled && external_playing)
}

impl AuraApp {
    pub(crate) fn effective_mute(&self) -> bool {
        effective_mute_state(self.config.mute, self.config.mute_when_media_playing, self.external_media_playing)
    }

    pub(super) fn apply_effective_mute(&self) -> Task<cosmic::Action<Message>> {
        let mute = self.effective_mute();
        let volume = self.config.volume;
        match self.backend() {
            Backend::Mpvpaper => {
                let engine = Arc::clone(&self.engine);
                let operation = Arc::clone(&self.engine_op_lock);
                Task::perform(async move {
                    let _guard = operation.lock().await;
                    let _ = tokio::task::spawn_blocking(move || {
                        let mut engine = engine.lock().unwrap_or_else(|p| p.into_inner());
                        engine.set_mute(None, mute);
                        if !mute { engine.set_volume(None, volume); }
                    }).await;
                }, |_| cosmic::Action::None)
            }
            Backend::Gnome => Task::perform(async move {
                let _ = tokio::task::spawn_blocking(move || crate::engine::gnome::set_mute(mute)).await;
            }, |_| cosmic::Action::None),
            Backend::Kde => Task::perform(async move {
                let _ = tokio::task::spawn_blocking(move || crate::engine::kde::set_mute(mute)).await;
            }, |_| cosmic::Action::None),
            Backend::Static => Task::none(),
        }
    }

    pub(super) fn apply_wallpaper_volume(&self, volume: u8) -> Task<cosmic::Action<Message>> {
        if self.effective_mute() { return Task::none(); }
        match self.backend() {
            Backend::Mpvpaper => {
                let engine = Arc::clone(&self.engine);
                let operation = Arc::clone(&self.engine_op_lock);
                Task::perform(async move {
                    let _guard = operation.lock().await;
                    let _ = tokio::task::spawn_blocking(move || {
                        engine.lock().unwrap_or_else(|p| p.into_inner()).set_volume(None, volume)
                    }).await;
                }, |_| cosmic::Action::None)
            }
            Backend::Gnome => Task::perform(async move {
                let _ = tokio::task::spawn_blocking(move || crate::engine::gnome::set_volume(volume)).await;
            }, |_| cosmic::Action::None),
            Backend::Kde => Task::perform(async move {
                let _ = tokio::task::spawn_blocking(move || crate::engine::kde::set_volume(volume)).await;
            }, |_| cosmic::Action::None),
            Backend::Static => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::effective_mute_state;

    #[test]
    fn automatic_mute_never_overrides_manual_choice() {
        assert!(!effective_mute_state(false, false, true));
        assert!(effective_mute_state(false, true, true));
        assert!(!effective_mute_state(false, true, false));
        assert!(effective_mute_state(true, true, false));
        assert!(effective_mute_state(true, true, true));
    }
}
