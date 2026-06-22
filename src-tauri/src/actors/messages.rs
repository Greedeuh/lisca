use actix::{Addr, Message};
use crate::commands::QueueSnapshotDto;

use super::speech_player_actor::SpeechPlayerActor;
use super::transcriber_actor::TranscriberActor;

// ── QueueActor messages ────────────────────────────────────────────

#[derive(Message)]
#[rtype(result = "Result<u64, String>")]
pub(crate)  struct AddText {
    pub(crate)  text: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(crate)  struct RemoveItem {
    pub(crate)  id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(crate)  struct MoveItem {
    pub(crate)  id: u64,
    pub(crate)  new_index: usize,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(crate)  struct ClearQueue;

#[derive(Message)]
#[rtype(result = "Result<QueueSnapshotDto, ()>")]
pub(crate)  struct GetQueueState;

#[derive(Message)]
#[rtype(result = "bool")]
pub(crate)  struct HasPlayableItems;

#[derive(Message)]
#[rtype(result = "bool")]
pub(crate)  struct ToggleAutoRead;

#[derive(Message)]
#[rtype(result = "bool")]
pub(crate)  struct ToggleOverlay;

#[derive(Message)]
#[rtype(result = "Option<PendingTextItem>")]
pub(super)  struct GetNextText;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct MarkProcessing {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct ReplaceWithSpeech {
    pub(super)  id: u64,
    pub(super)  audio_data: Option<Vec<f32>>,
    pub(super)  voice_key: Option<String>,
    pub(super)  language: Option<String>,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct SetTranscriptionError {
    pub(super)  id: u64,
    pub(super)  error: String,
}

#[derive(Message)]
#[rtype(result = "Option<PendingSpeechItem>")]
pub(super)  struct GetNextSpeech;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct SetItemCompleted {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct SkipItem {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct SetSpeechPaused {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct SetSpeechResumed {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct SetSpeechStopped {
    pub(super)  id: u64,
}

#[derive(Clone, Debug)]
pub(super)  struct PendingTextItem {
    pub(super)  id: u64,
    pub(super)  text: String,
}

#[derive(Clone, Debug)]
pub(super)  struct PendingSpeechItem {
    pub(super)  id: u64,
    pub(super)  audio_data: Option<Vec<f32>>,
}

// ── Queue event notifications (sent to peer actors) ────────────────

#[derive(Message)]
#[rtype(result = "()")]
pub(super)  struct TextAdded;

#[derive(Message)]
#[rtype(result = "()")]
pub(super)  struct SpeechReady;

// ── SpeechPlayerActor messages ─────────────────────────────────────

#[derive(Message)]
#[rtype(result = "()")]
pub(super)  struct PlaybackComplete {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "bool")]
pub(crate)  struct GetAutoRead;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate)  struct PlaybackPause;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate)  struct PlaybackResume;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate)  struct PlaybackStop;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate)  struct PlaybackSkip;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate)  struct PlaybackRestart;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub(super)  struct ReplayItem {
    pub(super)  id: u64,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(crate)  struct PlaybackReplay {
    pub(crate)  id: u64,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(super)  struct SetCurrentId {
    pub(super)  id: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(super)  struct SetPlayerAddr {
    pub(super)  addr: Addr<SpeechPlayerActor>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub(super)  struct SetTranscriberAddr {
    pub(super)  addr: Addr<TranscriberActor>,
}
