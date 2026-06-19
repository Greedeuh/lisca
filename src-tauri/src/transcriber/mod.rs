// Background task that dequeues TextMessages, detects language,
// resolves voice, synthesizes via Model, replaces with Speech.

mod language;

pub use language::detect_language_family;

use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use crate::models::Model;
use crate::queue::{Queue, QueueControllable, TextMessageStatus, Transcribable};
use crate::voice_prefs::VoiceMapping;

pub enum TranscriptionEvent {
    Started { id: u64, text: String },
    Completed { id: u64 },
    Error { id: u64, error: String },
}

pub struct TranscriberHandle {
    notify: Arc<Notify>,
}

impl TranscriberHandle {
    pub fn wake(&self) {
        self.notify.notify_one();
    }
}

pub fn spawn_transcriber(
    queue: Arc<Mutex<Queue>>,
    model: Arc<Mutex<dyn Model>>,
    voice_mapping: Arc<Mutex<VoiceMapping>>,
    on_event: impl Fn(TranscriptionEvent) + Send + Sync + 'static,
) -> TranscriberHandle {
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    tokio::spawn(async move {
        run_loop(queue, model, voice_mapping, &on_event, &notify_clone).await;
    });

    TranscriberHandle { notify }
}

async fn run_loop(
    queue: Arc<Mutex<Queue>>,
    model: Arc<Mutex<dyn Model>>,
    voice_mapping: Arc<Mutex<VoiceMapping>>,
    on_event: &(impl Fn(TranscriptionEvent) + Send + Sync),
    notify: &Notify,
) {
    loop {
        notify.notified().await;

        loop {
            let (id, text, language) = {
                let mut q = queue.lock().await;
                match q.next_pending_text_message() {
                    Some((_, id)) => {
                        let item = q.items().iter().find(|i| i.id() == id).unwrap();
                        let (text, language) = match item {
                            crate::queue::QueueItem::TextMessage {
                                text,
                                language,
                                ..
                            } => (text.clone(), language.clone()),
                            _ => unreachable!(),
                        };
                        q.set_text_message_status(id, TextMessageStatus::Processing)
                            .unwrap();
                        (id, text, language)
                    }
                    None => break,
                }
            };

            on_event(TranscriptionEvent::Started {
                id,
                text: text.clone(),
            });

            let detected_lang = detect_language_family(&text);
            let resolved_language = language.clone().or_else(|| detected_lang.map(String::from));
            let lang = resolved_language.as_deref();

            let voice_key = {
                let mapping = voice_mapping.lock().await;
                mapping.resolve(lang).map(|s| s.to_string())
            };

            let result = {
                let mut m = model.lock().await;
                m.synthesize(&text)
            };

            match result {
                Ok(audio_data) => {
                    let mut q = queue.lock().await;
                    let _ = q.replace_with_speech(
                        id,
                        Some(audio_data),
                        voice_key,
                        resolved_language,
                    );
                    on_event(TranscriptionEvent::Completed { id });
                }
                Err(e) => {
                    on_event(TranscriptionEvent::Error { id, error: e });
                    let mut q = queue.lock().await;
                    let _ = q.remove(id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::QueueControllable;
    use tokio::sync::mpsc;

    struct MockModel {
        result: Result<Vec<f32>, String>,
    }

    impl MockModel {
        fn success() -> Self {
            Self {
                result: Ok(vec![0.1, 0.2, 0.3]),
            }
        }

        fn failing(error: &str) -> Self {
            Self {
                result: Err(error.to_string()),
            }
        }
    }

    impl Model for MockModel {
        fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
            self.result.clone()
        }

        fn sample_rate(&self) -> u32 {
            22050
        }
    }

    fn setup() -> (
        Arc<Mutex<Queue>>,
        Arc<Mutex<dyn Model>>,
        Arc<Mutex<VoiceMapping>>,
    ) {
        let queue = Arc::new(Mutex::new(Queue::new()));
        let model = Arc::new(Mutex::new(MockModel::success()));
        let voice_mapping = Arc::new(Mutex::new(VoiceMapping::default()));
        (queue, model, voice_mapping)
    }

    async fn wait_for_event(rx: &mut mpsc::Receiver<TranscriptionEvent>) -> TranscriptionEvent {
        rx.recv()
            .await
            .expect("channel closed before event received")
    }

    #[tokio::test]
    async fn picks_up_text_message() {
        let (queue, model, vm) = setup();
        queue.lock().await.add_text("hello".to_string()).unwrap();

        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, |_| {});
        handle.wake();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let q = queue.lock().await;
        assert_eq!(q.items().len(), 1);
        assert!(matches!(
            &q.items()[0],
            crate::queue::QueueItem::Speech { .. }
        ));
    }

    #[tokio::test]
    async fn language_detected() {
        let test_text = "Hello, world! This is a test.";
        // Verify detection works outside async context
        let detected = detect_language_family(test_text);
        assert_eq!(
            detected,
            Some("en"),
            "language detection should work for this text"
        );

        let (queue, model, vm) = setup();
        queue.lock().await.add_text(test_text.to_string()).unwrap();

        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, |_| {});
        handle.wake();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let q = queue.lock().await;
        match &q.items()[0] {
            crate::queue::QueueItem::Speech { language, .. } => {
                assert_eq!(language.as_deref(), Some("en"));
            }
            _ => panic!("expected Speech"),
        }
    }

    #[tokio::test]
    async fn voice_resolved_via_mapping() {
        let (queue, model, vm) = setup();
        {
            let mut m = vm.lock().await;
            m.language_voice
                .insert("en".to_string(), "en-us-voice".to_string());
        }

        queue
            .lock()
            .await
            .add_text(
                "Hello, world! This is a test of English language detection in the transcriber."
                    .to_string(),
            )
            .unwrap();

        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, |_| {});
        handle.wake();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let q = queue.lock().await;
        match &q.items()[0] {
            crate::queue::QueueItem::Speech { voice_key, .. } => {
                assert_eq!(voice_key.as_deref(), Some("en-us-voice"));
            }
            _ => panic!("expected Speech"),
        }
    }

    #[tokio::test]
    async fn replaced_at_same_position() {
        let (queue, model, vm) = setup();
        {
            let mut q = queue.lock().await;
            q.add_text("first".to_string()).unwrap();
            q.add_text("second".to_string()).unwrap();
            q.add_text("third".to_string()).unwrap();
        }

        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, |_| {});
        handle.wake();

        // Wait for all 3 items to be processed (Started + Completed for each)
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let q = queue.lock().await;
        assert_eq!(q.items().len(), 3);
        // All items should now be Speech, preserving their original positions
        for item in q.items() {
            assert!(
                matches!(item, crate::queue::QueueItem::Speech { .. }),
                "expected all items to be Speech after processing"
            );
        }
        // Verify IDs are preserved in order
        assert_eq!(q.items()[0].id(), 1);
        assert_eq!(q.items()[1].id(), 2);
        assert_eq!(q.items()[2].id(), 3);
    }

    #[tokio::test]
    async fn speech_has_audio_data() {
        let (queue, model, vm) = setup();
        queue.lock().await.add_text("hello".to_string()).unwrap();

        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, |_| {});
        handle.wake();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let q = queue.lock().await;
        match &q.items()[0] {
            crate::queue::QueueItem::Speech { audio_data, .. } => {
                assert!(audio_data.is_some());
                assert!(!audio_data.as_ref().unwrap().is_empty());
            }
            _ => panic!("expected Speech"),
        }
    }

    #[tokio::test]
    async fn started_event_emitted() {
        let (queue, model, vm) = setup();
        queue.lock().await.add_text("hello".to_string()).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, move |e| {
            tx.try_send(e).ok();
        });
        handle.wake();

        let event = wait_for_event(&mut rx).await;
        assert!(matches!(event, TranscriptionEvent::Started { id: 1, .. }));
    }

    #[tokio::test]
    async fn completed_event_emitted() {
        let (queue, model, vm) = setup();
        queue.lock().await.add_text("hello".to_string()).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, move |e| {
            tx.try_send(e).ok();
        });
        handle.wake();

        // Wait for Started, then Completed
        let _started = wait_for_event(&mut rx).await;
        let completed = wait_for_event(&mut rx).await;
        assert!(matches!(completed, TranscriptionEvent::Completed { id: 1 }));
    }

    #[tokio::test]
    async fn error_event_on_synthesis_failure() {
        let queue = Arc::new(Mutex::new(Queue::new()));
        let model: Arc<Mutex<dyn Model>> = Arc::new(Mutex::new(MockModel::failing("synthesis failed")));
        let vm = Arc::new(Mutex::new(VoiceMapping::default()));

        queue.lock().await.add_text("hello".to_string()).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, move |e| {
            tx.try_send(e).ok();
        });
        handle.wake();

        let _started = wait_for_event(&mut rx).await;
        let error = wait_for_event(&mut rx).await;
        assert!(matches!(error, TranscriptionEvent::Error { id: 1, .. }));
    }

    #[tokio::test]
    async fn error_item_removed_next_processed() {
        let queue = Arc::new(Mutex::new(Queue::new()));
        let vm = Arc::new(Mutex::new(VoiceMapping::default()));

        {
            let mut q = queue.lock().await;
            q.add_text("will fail".to_string()).unwrap();
            q.add_text("will succeed".to_string()).unwrap();
        }

        // Use a model that fails on first call, succeeds on second
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let model: Arc<Mutex<dyn Model>> = Arc::new(Mutex::new(FailThenSucceedModel {
            call_count: call_count_clone,
        }));

        let (tx, mut rx) = mpsc::channel(16);
        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, move |e| {
            tx.try_send(e).ok();
        });
        handle.wake();

        // Collect all events
        let mut events = Vec::new();
        while let Ok(event) =
            tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await
        {
            if let Some(e) = event {
                events.push(e);
            } else {
                break;
            }
        }

        // Should have: Started(1), Error(1), Started(2), Completed(2)
        let error_count = events
            .iter()
            .filter(|e| matches!(e, TranscriptionEvent::Error { id: 1, .. }))
            .count();
        let completed_count = events
            .iter()
            .filter(|e| matches!(e, TranscriptionEvent::Completed { id: 2, .. }))
            .count();
        assert_eq!(error_count, 1, "should have one error for item 1");
        assert_eq!(completed_count, 1, "should have one completed for item 2");

        let q = queue.lock().await;
        assert_eq!(q.items().len(), 1);
        assert_eq!(q.items()[0].id(), 2);
        assert!(matches!(&q.items()[0], crate::queue::QueueItem::Speech { .. }));
    }

    struct FailThenSucceedModel {
        call_count: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Model for FailThenSucceedModel {
        fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count == 0 {
                Err("first call fails".to_string())
            } else {
                Ok(vec![0.1, 0.2, 0.3])
            }
        }

        fn sample_rate(&self) -> u32 {
            22050
        }
    }

    #[tokio::test]
    async fn transcriber_runs_concurrently() {
        let (queue, model, vm) = setup();

        let q = queue.clone();
        let handle = spawn_transcriber(q, model, vm, |_| {});

        // Add items after spawning — they should be picked up
        {
            let mut q = queue.lock().await;
            q.add_text("concurrent test".to_string()).unwrap();
        }
        handle.wake();

        // Verify we can do other work while transcriber processes
        let q_clone = queue.clone();
        let other_work = tokio::spawn(async move {
            let q = q_clone.lock().await;
            q.items().len()
        });

        let count = other_work.await.unwrap();
        assert!(count <= 1); // item may or may not be processed yet

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let q = queue.lock().await;
        assert!(matches!(
            &q.items()[0],
            crate::queue::QueueItem::Speech { .. }
        ));
    }
}
