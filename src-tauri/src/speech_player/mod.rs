// Audio playback utilities and the PlaybackController state machine.
// The SpeechPlayerActor uses play_with_controls() for actual audio playback.

pub mod playback;

pub use playback::{PlaybackController, PlaybackState};

use std::sync::atomic::Ordering;
use std::time::Duration;

pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

pub async fn play_with_controls(
    samples: Vec<f32>,
    sample_rate: u32,
    controller: &PlaybackController,
) -> bool {
    controller
        .state
        .store(PlaybackState::Playing as u8, Ordering::SeqCst);

    let state = controller.state.clone();
    let stop_flag = controller.stop_flag.clone();
    let pause_flag = controller.pause_flag.clone();
    let mutex = controller.mutex.clone();
    let condvar = controller.condvar.clone();

    match tokio::task::spawn_blocking(move || {
        let (stream, handle) = match rodio::OutputStream::try_default() {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to open audio output stream: {e}");
                state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
                return true;
            }
        };
        let sink = match rodio::Sink::try_new(&handle) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create audio sink: {e}");
                state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
                return true;
            }
        };

        let i16_samples = f32_to_i16(&samples);
        let buffer = rodio::buffer::SamplesBuffer::new(1, sample_rate, i16_samples);
        sink.append(buffer);

        loop {
            let guard = mutex.lock().unwrap();
            let (guard, _) = condvar.wait_timeout(guard, Duration::from_millis(200)).unwrap();
            drop(guard);

            if stop_flag.load(Ordering::SeqCst) {
                stop_flag.store(false, Ordering::SeqCst);
                pause_flag.store(false, Ordering::SeqCst);
                state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
                return true;
            }

            if pause_flag.load(Ordering::SeqCst) {
                sink.pause();
                state.store(PlaybackState::Paused as u8, Ordering::SeqCst);
                let guard = mutex.lock().unwrap();
                drop(condvar.wait_while(guard, |()| {
                    pause_flag.load(Ordering::SeqCst) && !stop_flag.load(Ordering::SeqCst)
                }));
                if stop_flag.load(Ordering::SeqCst) {
                    stop_flag.store(false, Ordering::SeqCst);
                    pause_flag.store(false, Ordering::SeqCst);
                    state.store(PlaybackState::Idle as u8, Ordering::SeqCst);
                    return true;
                }
                state.store(PlaybackState::Playing as u8, Ordering::SeqCst);
                sink.play();
            }

            if sink.empty() {
                break;
            }
        }

        sink.sleep_until_end();
        drop(sink);
        drop(stream);
        false
    })
    .await
    {
        Ok(interrupted) => interrupted,
        Err(e) => {
            log::error!("Playback task panicked: {e}");
            controller
                .state
                .store(PlaybackState::Idle as u8, Ordering::SeqCst);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
