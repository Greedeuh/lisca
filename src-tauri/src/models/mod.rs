// TTS model trait, LRU pool, and backend implementations (Piper, Kokoro).

mod kokoro;
mod kokoro_phonemizer;
mod piper;
mod pool;

pub(super)  use kokoro::KokoroFactory;
pub(super)  use piper::PiperFactory;
pub(super)  use pool::ModelPool;

use std::sync::Arc;
use tokio::sync::Mutex;

pub(super)  trait Model: Send {
    fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, String>;}

pub(super)  trait ModelFactory: Send + Sync {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String>;
    fn is_installed(&self, voice_key: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockModel {
    }

    impl Model for MockModel {
        fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
            Ok(vec![0.0; 100])
        }
    }

    #[test]
    fn mock_model_synthesize() {
        let mut model = MockModel { };
        let audio = model.synthesize("hello").unwrap();
        assert_eq!(audio.len(), 100);
    }
}
