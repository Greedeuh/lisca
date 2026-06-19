// Rodio audio output for tests only.
// Production code creates OutputStream directly on the blocking thread
// because OutputStream is not Send.

#[cfg(test)]
pub mod rodio_output {
    use rodio::{OutputStream, OutputStreamHandle, Sink, buffer::SamplesBuffer};
    use crate::speech_player::AudioOutput;

    pub struct RodioAudioOutput {
        _stream: OutputStream,
        handle: OutputStreamHandle,
        sink: Option<Sink>,
    }

    impl RodioAudioOutput {
        pub fn new() -> Result<Self, String> {
            let (stream, handle) = OutputStream::try_default()
                .map_err(|e| format!("failed to open audio output stream: {e}"))?;
            let sink = Sink::try_new(&handle)
                .map_err(|e| format!("failed to create audio sink: {e}"))?;
            Ok(Self { _stream: stream, handle, sink: Some(sink) })
        }
    }

    impl AudioOutput for RodioAudioOutput {
        fn play(&mut self, samples: Vec<i16>, sample_rate: u32) {
            let buffer = SamplesBuffer::new(1, sample_rate, samples);
            if let Some(sink) = &self.sink {
                sink.append(buffer);
            }
        }

        fn pause(&self) {
            if let Some(sink) = &self.sink {
                sink.pause();
            }
        }

        fn resume(&self) {
            if let Some(sink) = &self.sink {
                sink.play();
            }
        }

        fn is_empty(&self) -> bool {
            self.sink.as_ref().is_none_or(|s| s.empty())
        }

        fn sleep_until_end(&self) {
            if let Some(sink) = &self.sink {
                sink.sleep_until_end();
            }
        }
    }
}
