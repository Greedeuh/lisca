// TTS model trait, LRU pool, and backend implementations (Piper, Kokoro).

mod kokoro;
pub mod kokoro_phonemizer;
mod piper;
mod pool;

pub use kokoro::{KokoroEngine, KokoroFactory, KokoroModel};
pub use piper::{PiperFactory, PiperModel};
pub use pool::{ModelEvent, ModelPool};

use std::sync::Arc;
use tokio::sync::Mutex;

pub trait Model: Send {
    fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, String>;
    fn sample_rate(&self) -> u32;
}

pub trait ModelFactory: Send + Sync {
    fn create(&self, voice_key: &str) -> Result<Arc<Mutex<dyn Model>>, String>;
    fn is_installed(&self, voice_key: &str) -> bool;
    fn installed_voices(&self) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockModel {
        sample_rate: u32,
    }

    impl Model for MockModel {
        fn synthesize(&mut self, _text: &str) -> Result<Vec<f32>, String> {
            Ok(vec![0.0; 100])
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
    }

    #[test]
    fn mock_model_synthesize() {
        let mut model = MockModel { sample_rate: 22050 };
        let audio = model.synthesize("hello").unwrap();
        assert_eq!(audio.len(), 100);
        assert_eq!(model.sample_rate(), 22050);
    }
}
