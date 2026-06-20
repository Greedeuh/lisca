pub mod messages;
pub mod queue_actor;
pub mod speech_player_actor;
pub mod transcriber_actor;

use actix::Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::voice_prefs::VoiceMapping;

pub struct AppActors {
    pub queue: Addr<queue_actor::QueueActor>,
    pub player: Addr<speech_player_actor::SpeechPlayerActor>,
    pub voice_mapping: Arc<Mutex<VoiceMapping>>,
}
