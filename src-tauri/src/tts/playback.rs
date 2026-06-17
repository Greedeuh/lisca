/// Playback state management for the TTS queue processor. Provides atomic
/// stop/pause/resume flags and a notify channel to wake the processor loop.
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use super::queue::PlaybackState;

pub(crate) const STATE_IDLE: u8 = PlaybackState::Idle as u8;
pub(crate) const STATE_PLAYING: u8 = PlaybackState::Playing as u8;
pub(crate) const STATE_PAUSED: u8 = PlaybackState::Paused as u8;

pub(crate) struct PlaybackController {
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    state: Arc<AtomicU8>,
    notify: Arc<tokio::sync::Notify>,
}

impl PlaybackController {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            state: Arc::new(AtomicU8::new(STATE_IDLE)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.state.store(STATE_IDLE, Ordering::SeqCst);
        self.notify.notify_one();
    }

    pub fn pause(&self) {
        if self.state.load(Ordering::SeqCst) == STATE_PLAYING {
            self.pause_flag.store(true, Ordering::SeqCst);
            self.state.store(STATE_PAUSED, Ordering::SeqCst);
        }
    }

    pub fn resume(&self) {
        if self.state.load(Ordering::SeqCst) == STATE_PAUSED {
            self.pause_flag.store(false, Ordering::SeqCst);
            self.state.store(STATE_PLAYING, Ordering::SeqCst);
            self.notify.notify_one();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.state.load(Ordering::SeqCst) == STATE_IDLE
    }

    pub fn playback_state(&self) -> PlaybackState {
        PlaybackState::from(self.state.load(Ordering::SeqCst))
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    pub fn pause_flag(&self) -> Arc<AtomicBool> {
        self.pause_flag.clone()
    }

    pub fn state_arc(&self) -> Arc<AtomicU8> {
        self.state.clone()
    }

    pub fn notify(&self) -> Arc<tokio::sync::Notify> {
        self.notify.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn new_creates_idle_state() {
        let ctrl = PlaybackController::new();
        assert!(ctrl.is_idle());
        assert!(matches!(ctrl.playback_state(), PlaybackState::Idle));
        assert!(!ctrl.stop_flag().load(Ordering::SeqCst));
        assert!(!ctrl.pause_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn stop_sets_idle_and_flags() {
        let ctrl = PlaybackController::new();
        ctrl.stop();
        assert!(ctrl.is_idle());
        assert!(ctrl.stop_flag().load(Ordering::SeqCst));
        assert!(!ctrl.pause_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn pause_only_works_when_playing() {
        let ctrl = PlaybackController::new();
        // pause on idle — no-op
        ctrl.pause();
        assert!(ctrl.is_idle());

        // simulate playing state (processor does this via state_arc)
        ctrl.state_arc().store(STATE_PLAYING, Ordering::SeqCst);
        ctrl.pause();
        assert!(!ctrl.is_idle());
        assert!(matches!(ctrl.playback_state(), PlaybackState::Paused));
        assert!(ctrl.pause_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn resume_only_works_when_paused() {
        let ctrl = PlaybackController::new();
        // resume on idle — no-op
        ctrl.resume();
        assert!(ctrl.is_idle());

        // simulate paused state
        ctrl.state_arc().store(STATE_PAUSED, Ordering::SeqCst);
        ctrl.resume();
        assert!(!ctrl.is_idle());
        assert!(matches!(ctrl.playback_state(), PlaybackState::Playing));
        assert!(!ctrl.pause_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn stop_clears_pause_flag() {
        let ctrl = PlaybackController::new();
        ctrl.state_arc().store(STATE_PLAYING, Ordering::SeqCst);
        ctrl.pause();
        assert!(ctrl.pause_flag().load(Ordering::SeqCst));

        ctrl.stop();
        assert!(!ctrl.pause_flag().load(Ordering::SeqCst));
        assert!(ctrl.is_idle());
    }

    #[test]
    fn state_arc_shares_state() {
        let ctrl = PlaybackController::new();
        let state = ctrl.state_arc();
        state.store(STATE_PLAYING, Ordering::SeqCst);
        assert!(matches!(ctrl.playback_state(), PlaybackState::Playing));
    }
}
