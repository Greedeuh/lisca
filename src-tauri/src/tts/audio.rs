use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use super::I16_SAMPLE_SCALE;

pub(crate) fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s * I16_SAMPLE_SCALE).clamp(-32768.0, I16_SAMPLE_SCALE) as i16)
        .collect()
}

pub(crate) struct AudioOutput {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioOutput {
    pub fn try_new() -> Result<Self, String> {
        let (_stream, handle) =
            OutputStream::try_default().map_err(|e| format!("Failed to open audio output: {}", e))?;
        let sink =
            Sink::try_new(&handle).map_err(|e| format!("Failed to create audio sink: {}", e))?;
        Ok(Self { _stream, sink })
    }

    pub fn play_buffer(&self, samples: Vec<i16>, sample_rate: u32) {
        let buffer = SamplesBuffer::new(1, sample_rate, samples);
        self.sink.append(buffer);
    }

    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn play(&self) {
        self.sink.play();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_normal_values() {
        let out = f32_to_i16(&[0.0, 0.5]);
        assert_eq!(out[0], 0);
        assert_eq!(out[1], 16383);
    }

    #[test]
    fn converts_full_scale() {
        let out = f32_to_i16(&[1.0, -1.0]);
        assert_eq!(out[0], 32767);
        assert_eq!(out[1], -32767);
    }

    #[test]
    fn clamps_positive_overflow() {
        let out = f32_to_i16(&[2.0]);
        assert_eq!(out[0], 32767);
    }

    #[test]
    fn clamps_negative_overflow() {
        let out = f32_to_i16(&[-2.0]);
        assert_eq!(out[0], -32768);
    }

    #[test]
    fn empty_input() {
        let out = f32_to_i16(&[]);
        assert!(out.is_empty());
    }
}
