use actix::{Actor, Addr, Context, Handler};
use tauri::{AppHandle, Emitter};

use crate::commands::QueueSnapshotDto;
use crate::queue::{Playable, Queue, QueueControllable, Transcribable};

use super::messages::*;
use super::speech_player_actor::SpeechPlayerActor;
use super::transcriber_actor::TranscriberActor;

pub struct QueueActor {
    queue: Queue,
    app_handle: AppHandle,
    player_addr: Option<Addr<SpeechPlayerActor>>,
    transcriber_addr: Option<Addr<TranscriberActor>>,
}

impl QueueActor {
    pub fn new(queue: Queue, app_handle: AppHandle) -> Self {
        Self {
            queue,
            app_handle,
            player_addr: None,
            transcriber_addr: None,
        }
    }

    pub fn set_player_addr(&mut self, addr: Addr<SpeechPlayerActor>) {
        self.player_addr = Some(addr);
    }

    pub fn set_transcriber_addr(&mut self, addr: Addr<TranscriberActor>) {
        self.transcriber_addr = Some(addr);
    }

    fn emit_updated(&self) {
        let _ = self.app_handle.emit("queue_updated", ());
    }
}

impl Actor for QueueActor {
    type Context = Context<Self>;
}

impl Handler<AddText> for QueueActor {
    type Result = Result<u64, String>;

    fn handle(&mut self, msg: AddText, _: &mut Context<Self>) -> Self::Result {
        let id = self.queue.add_text(msg.text)?;
        self.emit_updated();
        if let Some(ref addr) = self.transcriber_addr {
            addr.do_send(TextAdded);
        }
        Ok(id)
    }
}

impl Handler<RemoveItem> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RemoveItem, _: &mut Context<Self>) -> Self::Result {
        self.queue.remove(msg.id)?;
        self.emit_updated();
        Ok(())
    }
}

impl Handler<MoveItem> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: MoveItem, _: &mut Context<Self>) -> Self::Result {
        self.queue.reorder(msg.id, msg.new_index)?;
        self.emit_updated();
        Ok(())
    }
}

impl Handler<ClearQueue> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: ClearQueue, _: &mut Context<Self>) -> Self::Result {
        self.queue.clear()?;
        self.emit_updated();
        Ok(())
    }
}

impl Handler<GetQueueState> for QueueActor {
    type Result = Result<QueueSnapshotDto, ()>;

    fn handle(&mut self, _: GetQueueState, _: &mut Context<Self>) -> Self::Result {
        Ok(self.queue.snapshot_dto())
    }
}

impl Handler<ToggleAutoRead> for QueueActor {
    type Result = bool;

    fn handle(&mut self, _: ToggleAutoRead, _: &mut Context<Self>) -> Self::Result {
        self.queue.config.auto_read = !self.queue.config.auto_read;
        let val = self.queue.config.auto_read;
        if let Err(e) = self.queue.save_config() {
            log::error!("Failed to save queue config: {e}");
        }
        if let Some(ref player) = self.player_addr {
            player.do_send(AutoReadChanged { value: val });
        }
        self.emit_updated();
        val
    }
}

impl Handler<ToggleOverlay> for QueueActor {
    type Result = bool;

    fn handle(&mut self, _: ToggleOverlay, _: &mut Context<Self>) -> Self::Result {
        self.queue.config.show_overlay = !self.queue.config.show_overlay;
        let val = self.queue.config.show_overlay;
        if let Err(e) = self.queue.save_config() {
            log::error!("Failed to save queue config: {e}");
        }
        self.emit_updated();
        val
    }
}

impl Handler<GetNextText> for QueueActor {
    type Result = Option<PendingTextItem>;

    fn handle(&mut self, _: GetNextText, _: &mut Context<Self>) -> Self::Result {
        let (_, id) = self.queue.next_pending_text_message()?;
        let item = self.queue.items().iter().find(|i| i.id() == id)?;
        match item {
            crate::queue::QueueItem::TextMessage {
                id, text, language, ..
            } => Some(PendingTextItem {
                id: *id,
                text: text.clone(),
                language: language.clone(),
            }),
            _ => None,
        }
    }
}

impl Handler<MarkProcessing> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: MarkProcessing, _: &mut Context<Self>) -> Self::Result {
        self.queue
            .set_text_message_status(msg.id, crate::queue::TextMessageStatus::Processing)
    }
}

impl Handler<ReplaceWithSpeech> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ReplaceWithSpeech, _: &mut Context<Self>) -> Self::Result {
        self.queue.replace_with_speech(
            msg.id,
            msg.audio_data,
            msg.voice_key,
            msg.language,
        )?;
        self.emit_updated();
        if let Some(ref addr) = self.player_addr {
            addr.do_send(SpeechReady);
        }
        Ok(())
    }
}

impl Handler<SetTranscriptionError> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SetTranscriptionError, _: &mut Context<Self>) -> Self::Result {
        let _ = self.queue.remove(msg.id);
        self.emit_updated();
        Ok(())
    }
}

impl Handler<GetNextSpeech> for QueueActor {
    type Result = Option<PendingSpeechItem>;

    fn handle(&mut self, _: GetNextSpeech, _: &mut Context<Self>) -> Self::Result {
        let (_, id) = self.queue.next_to_play_speech()?;
        let _ = self
            .queue
            .set_speech_status(id, crate::queue::SpeechStatus::Playing);
        let item = self.queue.items().iter().find(|i| i.id() == id)?;
        match item {
            crate::queue::QueueItem::Speech { id, audio_data, .. } => Some(PendingSpeechItem {
                id: *id,
                audio_data: audio_data.clone(),
            }),
            _ => None,
        }
    }
}

impl Handler<SetItemCompleted> for QueueActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SetItemCompleted, _: &mut Context<Self>) -> Self::Result {
        self.queue
            .set_speech_status(msg.id, crate::queue::SpeechStatus::Played)?;
        self.emit_updated();
        Ok(())
    }
}

impl Handler<SetPlayerAddr> for QueueActor {
    type Result = ();

    fn handle(&mut self, msg: SetPlayerAddr, _: &mut Context<Self>) {
        self.player_addr = Some(msg.addr);
    }
}

impl Handler<SetTranscriberAddr> for QueueActor {
    type Result = ();

    fn handle(&mut self, msg: SetTranscriberAddr, _: &mut Context<Self>) {
        self.transcriber_addr = Some(msg.addr);
    }
}
