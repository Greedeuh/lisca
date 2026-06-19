// Atomic state machine for playback control.
// Uses AtomicBool flags for pause/stop and AtomicU8 for state,
// enabling thread-safe control from the async task and frontend.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlaybackState {
    Idle = 0,
    Playing = 1,
    Paused = 2,
}

impl From<u8> for PlaybackState {
    fn from(v: u8) -> Self {
        match v {
            1 => PlaybackState::Playing,
            2 => PlaybackState::Paused,
            _ => PlaybackState::Idle,
        }
    }
}

pub struct PlaybackController {
    pub(crate) stop_flag: Arc<AtomicBool>,
    pub(crate) pause_flag: Arc<AtomicBool>,
    pub(crate) state: Arc<AtomicU8>,
    pub(crate) notify: Arc<tokio::sync::Notify>,
}

impl Default for PlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackController {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            state: Arc::new(AtomicU8::new(PlaybackState::Idle as u8)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
        self.notify.notify_one();
    }

    pub fn pause(&self) {
        if self.state.load(Ordering::SeqCst) == PlaybackState::Playing as u8 {
            self.pause_flag.store(true, Ordering::SeqCst);
            self.state.store(PlaybackState::Paused as u8, Ordering::SeqCst);
        }
    }

    pub fn resume(&self) {
        if self.state.load(Ordering::SeqCst) == PlaybackState::Paused as u8 {
            self.pause_flag.store(false, Ordering::SeqCst);
            self.state.store(PlaybackState::Playing as u8, Ordering::SeqCst);
            self.notify.notify_one();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.state.load(Ordering::SeqCst) == PlaybackState::Idle as u8
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
        assert_eq!(ctrl.playback_state(), PlaybackState::Idle);
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
        ctrl.pause();
        assert!(ctrl.is_idle());

        ctrl.state_arc().store(PlaybackState::Playing as u8, Ordering::SeqCst);
        ctrl.pause();
        assert!(!ctrl.is_idle());
        assert_eq!(ctrl.playback_state(), PlaybackState::Paused);
        assert!(ctrl.pause_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn resume_only_works_when_paused() {
        let ctrl = PlaybackController::new();
        ctrl.resume();
        assert!(ctrl.is_idle());

        ctrl.state_arc().store(PlaybackState::Paused as u8, Ordering::SeqCst);
        ctrl.resume();
        assert!(!ctrl.is_idle());
        assert_eq!(ctrl.playback_state(), PlaybackState::Playing);
        assert!(!ctrl.pause_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn stop_clears_pause_flag() {
        let ctrl = PlaybackController::new();
        ctrl.state_arc().store(PlaybackState::Playing as u8, Ordering::SeqCst);
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
        state.store(PlaybackState::Playing as u8, Ordering::SeqCst);
        assert_eq!(ctrl.playback_state(), PlaybackState::Playing);
    }
}
