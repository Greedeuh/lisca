// Background task that plays Speech items from the queue.
// AudioOutput trait abstracts the audio backend for testability.
// Exposes PlaybackController state machine (Idle/Playing/Paused) and controls.

mod playback;

pub use playback::{PlaybackController, PlaybackState};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use crate::queue::{Playable, Queue, QueueItem, SpeechStatus};

pub trait AudioOutput: Send {
    fn play(&mut self, samples: Vec<i16>, sample_rate: u32);
    fn pause(&self);
    fn resume(&self);
    fn is_empty(&self) -> bool;
    fn sleep_until_end(&self);
}

pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

pub enum PlaybackEvent {
    Started { id: u64 },
    Paused { id: u64 },
    Resumed { id: u64 },
    Stopped,
    ItemCompleted { id: u64 },
}

pub struct SpeechPlayerHandle {
    notify: Arc<Notify>,
    controller: PlaybackController,
}

impl SpeechPlayerHandle {
    pub fn wake(&self) {
        self.notify.notify_one();
    }

    pub fn play(&self) {
        self.controller.resume();
    }

    pub fn pause(&self) {
        self.controller.pause();
    }

    pub fn stop(&self) {
        self.controller.stop();
    }

    pub fn is_idle(&self) -> bool {
        self.controller.is_idle()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.controller.playback_state()
    }
}

pub fn spawn_speech_player(
    queue: Arc<Mutex<Queue>>,
    auto_read: Arc<AtomicBool>,
    audio_factory: Box<dyn Fn() -> Result<Box<dyn AudioOutput>, String> + Send + Sync>,
    on_event: impl Fn(PlaybackEvent) + Send + Sync + 'static,
) -> SpeechPlayerHandle {
    let controller = PlaybackController::new();
    let notify = Arc::new(Notify::new());
    let ctrl_clone = PlaybackController {
        stop_flag: controller.stop_flag(),
        pause_flag: controller.pause_flag(),
        state: controller.state_arc(),
        notify: controller.notify(),
    };
    let notify_clone = notify.clone();

    tokio::spawn(async move {
        run_loop(queue, auto_read, ctrl_clone, audio_factory, &on_event, &notify_clone).await;
    });

    SpeechPlayerHandle {
        notify,
        controller,
    }
}

async fn run_loop(
    queue: Arc<Mutex<Queue>>,
    auto_read: Arc<AtomicBool>,
    controller: PlaybackController,
    audio_factory: Box<dyn Fn() -> Result<Box<dyn AudioOutput>, String> + Send + Sync>,
    on_event: &(impl Fn(PlaybackEvent) + Send + Sync),
    notify: &Notify,
) {
    loop {
        notify.notified().await;

        loop {
            let (id, audio_data, sample_rate) = {
                let q = queue.lock().await;
                match q.next_to_play_speech() {
                    Some((_, id)) => {
                        let item = q.items.iter().find(|i| i.id() == id).unwrap();
                        match item {
                            QueueItem::Speech {
                                audio_data,
                                status,
                                ..
                            } => {
                                if *status != SpeechStatus::ToPlay {
                                    break;
                                }
                                (
                                    id,
                                    audio_data.clone(),
                                    22050u32,
                                )
                            }
                            _ => unreachable!(),
                        }
                    }
                    None => break,
                }
            };

            {
                let mut q = queue.lock().await;
                let _ = q.set_speech_status(id, SpeechStatus::Playing);
            }
            on_event(PlaybackEvent::Started { id });

            if let Some(samples) = audio_data {
                let interrupted = play_with_controls(
                    samples,
                    sample_rate,
                    controller.stop_flag(),
                    controller.pause_flag(),
                    controller.state_arc(),
                    audio_factory.as_ref(),
                )
                .await;

                if interrupted {
                    on_event(PlaybackEvent::Stopped);
                    continue;
                }

                {
                    let mut q = queue.lock().await;
                    let _ = q.set_speech_status(id, SpeechStatus::Played);
                }
                on_event(PlaybackEvent::ItemCompleted { id });

                if !auto_read.load(Ordering::SeqCst) {
                    break;
                }
            } else {
                let mut q = queue.lock().await;
                let _ = q.set_speech_status(id, SpeechStatus::Played);
                on_event(PlaybackEvent::ItemCompleted { id });

                if !auto_read.load(Ordering::SeqCst) {
                    break;
                }
            }
        }
    }
}

async fn play_with_controls(
    samples: Vec<f32>,
    sample_rate: u32,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    state: Arc<std::sync::atomic::AtomicU8>,
    audio_factory: &(dyn Fn() -> Result<Box<dyn AudioOutput>, String> + Send + Sync),
) -> bool {
    state.store(PlaybackState::Playing as u8, Ordering::SeqCst);

    // Create the audio output before entering spawn_blocking
    let mut output = match audio_factory() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{}", e);
            state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
            return true;
        }
    };

    let state_clone = state.clone();
    match tokio::task::spawn_blocking(move || {
        let i16_samples = f32_to_i16(&samples);
        output.play(i16_samples, sample_rate);

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            if stop_flag.load(Ordering::SeqCst) {
                stop_flag.store(false, Ordering::SeqCst);
                pause_flag.store(false, Ordering::SeqCst);
                state_clone.store(PlaybackState::Idle as u8, Ordering::SeqCst);
                return true;
            }

            if pause_flag.load(Ordering::SeqCst) {
                output.pause();
                state_clone.store(PlaybackState::Paused as u8, Ordering::SeqCst);
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if stop_flag.load(Ordering::SeqCst) || !pause_flag.load(Ordering::SeqCst) {
                        break;
                    }
                }
                if stop_flag.load(Ordering::SeqCst) {
                    stop_flag.store(false, Ordering::SeqCst);
                    pause_flag.store(false, Ordering::SeqCst);
                    state_clone.store(PlaybackState::Idle as u8, Ordering::SeqCst);
                    return true;
                }
                pause_flag.store(false, Ordering::SeqCst);
                state_clone.store(PlaybackState::Playing as u8, Ordering::SeqCst);
                output.resume();
            }

            if output.is_empty() {
                break;
            }
        }

        output.sleep_until_end();
        false
    })
    .await
    {
        Ok(interrupted) => interrupted,
        Err(e) => {
            eprintln!("Playback task panicked: {}", e);
            state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{QueueControllable, Transcribable};
    use tokio::sync::mpsc;

    struct MockAudioOutput {
        played: Arc<Mutex<Vec<(Vec<i16>, u32)>>>,
    }

    impl MockAudioOutput {
        fn new() -> (Self, Arc<Mutex<Vec<(Vec<i16>, u32)>>>) {
            let played = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    played: played.clone(),
                },
                played,
            )
        }
    }

    impl AudioOutput for MockAudioOutput {
        fn play(&mut self, samples: Vec<i16>, sample_rate: u32) {
            self.played
                .try_lock()
                .unwrap()
                .push((samples, sample_rate));
        }

        fn pause(&self) {}
        fn resume(&self) {}

        fn is_empty(&self) -> bool {
            true
        }

        fn sleep_until_end(&self) {}
    }

    fn setup() -> Arc<Mutex<Queue>> {
        Arc::new(Mutex::new(Queue::new()))
    }

    async fn add_speech(queue: &Arc<Mutex<Queue>>, text: &str) -> u64 {
        let mut q = queue.lock().await;
        let id = q.add_text(text.to_string()).unwrap();
        q.replace_with_speech(
            id,
            Some(vec![0.1, 0.2, 0.3]),
            Some("test-voice".to_string()),
            Some("en".to_string()),
        )
        .unwrap();
        id
    }

    fn make_factory(
        played: Arc<Mutex<Vec<(Vec<i16>, u32)>>>,
    ) -> Box<dyn Fn() -> Result<Box<dyn AudioOutput>, String> + Send + Sync> {
        Box::new(move || {
            Ok(Box::new(MockAudioOutput {
                played: played.clone(),
            }))
        })
    }

    async fn wait_for_event(rx: &mut mpsc::Receiver<PlaybackEvent>) -> PlaybackEvent {
        rx.recv()
            .await
            .expect("channel closed before event received")
    }

    #[test]
    fn f32_to_i16_converts_correctly() {
        let out = f32_to_i16(&[0.0, 0.5, -0.5, 1.0, -1.0]);
        assert_eq!(out[0], 0);
        assert_eq!(out[1], 16383);
        assert_eq!(out[2], -16383);
        assert_eq!(out[3], 32767);
        assert_eq!(out[4], -32767);
    }

    #[test]
    fn f32_to_i16_clamps_overflow() {
        let out = f32_to_i16(&[2.0, -2.0]);
        assert_eq!(out[0], 32767);
        assert_eq!(out[1], -32768);
    }

    #[test]
    fn f32_to_i16_empty_input() {
        let out = f32_to_i16(&[]);
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn picks_up_speech_item() {
        let queue = setup();
        let id = add_speech(&queue, "hello").await;

        let (tx, mut rx) = mpsc::channel(16);
        let (_, played_ref) = MockAudioOutput::new();
        let q = queue.clone();
        let auto_read = Arc::new(AtomicBool::new(false));
        let handle = spawn_speech_player(
            q,
            auto_read,
            make_factory(played_ref),
            move |e| {
                tx.try_send(e).ok();
            },
        );
        handle.wake();

        let event = wait_for_event(&mut rx).await;
        assert!(matches!(event, PlaybackEvent::Started { id: i } if i == id));

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn state_transitions_idle_to_playing_to_idle() {
        let queue = setup();
        add_speech(&queue, "hello").await;

        let q = queue.clone();
        let auto_read = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = mpsc::channel(16);
        let (_, played_ref) = MockAudioOutput::new();
        let handle = spawn_speech_player(
            q,
            auto_read,
            make_factory(played_ref),
            move |e| {
                tx.try_send(e).ok();
            },
        );
        handle.wake();

        let started = wait_for_event(&mut rx).await;
        assert!(matches!(started, PlaybackEvent::Started { id: 1 }));
        let completed = wait_for_event(&mut rx).await;
        assert!(matches!(completed, PlaybackEvent::ItemCompleted { id: 1 }));
    }

    #[test]
    fn pause_only_works_when_playing() {
        let ctrl = PlaybackController::new();
        ctrl.pause();
        assert!(ctrl.is_idle());

        ctrl.state_arc()
            .store(PlaybackState::Playing as u8, Ordering::SeqCst);
        ctrl.pause();
        assert_eq!(ctrl.playback_state(), PlaybackState::Paused);
    }

    #[test]
    fn resume_only_works_when_paused() {
        let ctrl = PlaybackController::new();
        ctrl.resume();
        assert!(ctrl.is_idle());

        ctrl.state_arc()
            .store(PlaybackState::Paused as u8, Ordering::SeqCst);
        ctrl.resume();
        assert_eq!(ctrl.playback_state(), PlaybackState::Playing);
    }

    #[test]
    fn stop_clears_pause_flag() {
        let ctrl = PlaybackController::new();
        ctrl.state_arc()
            .store(PlaybackState::Playing as u8, Ordering::SeqCst);
        ctrl.pause();
        assert!(ctrl.pause_flag().load(Ordering::SeqCst));

        ctrl.stop();
        assert!(!ctrl.pause_flag().load(Ordering::SeqCst));
        assert!(ctrl.is_idle());
    }

    #[tokio::test]
    async fn auto_play_sequential() {
        let queue = setup();
        let id1 = add_speech(&queue, "first").await;
        let id2 = add_speech(&queue, "second").await;

        let (tx, mut rx) = mpsc::channel(16);
        let (_, played_ref) = MockAudioOutput::new();
        let q = queue.clone();
        let auto_read = Arc::new(AtomicBool::new(true));
        let _handle = spawn_speech_player(
            q,
            auto_read,
            make_factory(played_ref),
            move |e| {
                tx.try_send(e).ok();
            },
        );
        _handle.wake();

        let mut events = Vec::new();
        while let Ok(event) =
            tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
        {
            if let Some(e) = event {
                events.push(e);
            } else {
                break;
            }
        }

        let started_ids: Vec<u64> = events
            .iter()
            .filter_map(|e| match e {
                PlaybackEvent::Started { id } => Some(*id),
                _ => None,
            })
            .collect();
        assert!(started_ids.contains(&id1));
        assert!(started_ids.contains(&id2));

        let completed_ids: Vec<u64> = events
            .iter()
            .filter_map(|e| match e {
                PlaybackEvent::ItemCompleted { id } => Some(*id),
                _ => None,
            })
            .collect();
        assert!(completed_ids.contains(&id1));
        assert!(completed_ids.contains(&id2));
    }

    #[tokio::test]
    async fn auto_read_off_stops_after_current() {
        let queue = setup();
        let id1 = add_speech(&queue, "first").await;
        let _id2 = add_speech(&queue, "second").await;

        let (tx, mut rx) = mpsc::channel(16);
        let (_, played_ref) = MockAudioOutput::new();
        let q = queue.clone();
        let auto_read = Arc::new(AtomicBool::new(false));
        let _handle = spawn_speech_player(
            q,
            auto_read,
            make_factory(played_ref),
            move |e| {
                tx.try_send(e).ok();
            },
        );
        _handle.wake();

        let mut events = Vec::new();
        while let Ok(event) =
            tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
        {
            if let Some(e) = event {
                events.push(e);
            } else {
                break;
            }
        }

        let started_ids: Vec<u64> = events
            .iter()
            .filter_map(|e| match e {
                PlaybackEvent::Started { id } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(started_ids, vec![id1]);

        let q = queue.lock().await;
        match &q.items()[1] {
            QueueItem::Speech { status, .. } => assert_eq!(*status, SpeechStatus::ToPlay),
            _ => panic!("expected Speech"),
        }
    }

    #[tokio::test]
    async fn item_completed_event_emitted() {
        let queue = setup();
        add_speech(&queue, "hello").await;

        let (tx, mut rx) = mpsc::channel(16);
        let (_, played_ref) = MockAudioOutput::new();
        let q = queue.clone();
        let auto_read = Arc::new(AtomicBool::new(false));
        let _handle = spawn_speech_player(
            q,
            auto_read,
            make_factory(played_ref),
            move |e| {
                tx.try_send(e).ok();
            },
        );
        _handle.wake();

        let mut found_completed = false;
        while let Ok(event) =
            tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
        {
            if let Some(PlaybackEvent::ItemCompleted { .. }) = event {
                found_completed = true;
                break;
            }
            if event.is_none() {
                break;
            }
        }
        assert!(found_completed);
    }
}
