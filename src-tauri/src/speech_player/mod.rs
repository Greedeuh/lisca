pub struct SpeechPlayer;

impl SpeechPlayer {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_player_creation() {
        let _p = SpeechPlayer::new();
    }
}
