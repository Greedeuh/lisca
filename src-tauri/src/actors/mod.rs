pub mod messages;
pub mod queue_actor;
pub mod speech_player_actor;
pub mod transcriber_actor;

use actix::Addr;

pub struct AppActors {
    pub queue: Addr<queue_actor::QueueActor>,
    pub player: Addr<speech_player_actor::SpeechPlayerActor>,
}
