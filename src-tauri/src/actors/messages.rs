use actix::{Addr, Message};
use crate::commands::QueueSnapshotDto;

use super::speech_player_actor::SpeechPlayerActor;
use super::transcriber_actor::TranscriberActor;

// ── QueueActor messages ────────────────────────────────────────────

#[derive(Message)]
#[rtype(result = "Result<u64, String>")]
pub struct AddText {
    pub text: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveItem {
    pub id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct MoveItem {
    pub id: u64,
    pub new_index: usize,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ClearQueue;

#[derive(Message)]
#[rtype(result = "Result<QueueSnapshotDto, ()>")]
pub struct GetQueueState;

#[derive(Message)]
#[rtype(result = "bool")]
pub struct ToggleAutoRead;

#[derive(Message)]
#[rtype(result = "bool")]
pub struct ToggleOverlay;

#[derive(Message)]
#[rtype(result = "Option<PendingTextItem>")]
pub struct GetNextText;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct MarkProcessing {
    pub id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ReplaceWithSpeech {
    pub id: u64,
    pub audio_data: Option<Vec<f32>>,
    pub voice_key: Option<String>,
    pub language: Option<String>,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetTranscriptionError {
    pub id: u64,
    pub error: String,
}

#[derive(Message)]
#[rtype(result = "Option<PendingSpeechItem>")]
pub struct GetNextSpeech;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetItemCompleted {
    pub id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetSpeechPaused {
    pub id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetSpeechResumed {
    pub id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetSpeechStopped {
    pub id: u64,
}

#[derive(Clone, Debug)]
pub struct PendingTextItem {
    pub id: u64,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct PendingSpeechItem {
    pub id: u64,
    pub audio_data: Option<Vec<f32>>,
}

// ── Queue event notifications (sent to peer actors) ────────────────

#[derive(Message)]
#[rtype(result = "()")]
pub struct TextAdded;

#[derive(Message)]
#[rtype(result = "()")]
pub struct SpeechReady;

// ── SpeechPlayerActor messages ─────────────────────────────────────

// ── SpeechPlayerActor messages ─────────────────────────────────────

#[derive(Message)]
#[rtype(result = "bool")]
pub struct GetAutoRead;

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlaybackPause;

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlaybackResume;

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlaybackStop;

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetCurrentId {
    pub id: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AutoReadChanged {
    pub value: bool,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetPlayerAddr {
    pub addr: Addr<SpeechPlayerActor>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetTranscriberAddr {
    pub addr: Addr<TranscriberActor>,
}
