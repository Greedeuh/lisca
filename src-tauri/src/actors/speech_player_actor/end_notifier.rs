use std::time::Duration;

use actix::Addr;
use rodio::Source;

use crate::actors::messages::PlaybackComplete;

use super::SpeechPlayerActor;

pub struct EndNotifier<S> {
    source: S,
    addr: Addr<SpeechPlayerActor>,
    id: u64,
    done: bool,
}

impl<S> EndNotifier<S> {
    pub fn new(source: S, addr: Addr<SpeechPlayerActor>, id: u64) -> Self {
        Self {
            source,
            addr,
            id,
            done: false,
        }
    }
}

impl<S: Source> Iterator for EndNotifier<S>
where
    S::Item: rodio::Sample,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<S::Item> {
        if self.done {
            return None;
        }
        match self.source.next() {
            Some(sample) => Some(sample),
            None => {
                self.done = true;
                let _ = self.addr.do_send(PlaybackComplete { id: self.id });
                None
            }
        }
    }
}

impl<S: Source> Source for EndNotifier<S>
where
    S::Item: rodio::Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.source.try_seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct VecSource {
        data: Vec<f32>,
        pos: usize,
    }

    impl Iterator for VecSource {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            if self.pos < self.data.len() {
                let v = self.data[self.pos];
                self.pos += 1;
                Some(v)
            } else {
                None
            }
        }
    }

    impl Source for VecSource {
        fn current_frame_len(&self) -> Option<usize> {
            Some(self.data.len() - self.pos)
        }
        fn channels(&self) -> u16 {
            1
        }
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn total_duration(&self) -> Option<Duration> {
            Some(Duration::from_secs_f64(
                self.data.len() as f64 / 44100.0,
            ))
        }
    }

    fn make_source(data: Vec<f32>) -> VecSource {
        VecSource { data, pos: 0 }
    }

    #[test]
    fn delegates_to_inner_source() {
        let source = make_source(vec![0.1, 0.2, 0.3]);
        let items: Vec<f32> = source.collect();
        assert_eq!(items, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn empty_source_yields_nothing() {
        let source = make_source(vec![]);
        let items: Vec<f32> = source.collect();
        assert!(items.is_empty());
    }

    #[test]
    fn delegates_source_methods() {
        let source = make_source(vec![0.0; 100]);
        assert_eq!(source.channels(), 1);
        assert_eq!(source.sample_rate(), 44100);
        assert_eq!(source.total_duration(), Some(Duration::from_secs_f64(100.0 / 44100.0)));
    }
}
