use actix::{Actor, Addr, AsyncContext, Context, Handler, WrapFuture};
use tauri::{AppHandle, Emitter};

use crate::speech_player::play_with_controls;
use crate::speech_player::playback::PlaybackController;

use super::messages::*;
use super::queue_actor::QueueActor;

pub struct SpeechPlayerActor {
    queue_addr: Addr<QueueActor>,
    app_handle: AppHandle,
    playback: PlaybackController,
    busy: bool,
    auto_read: bool,
    pending_work: bool,
}

impl SpeechPlayerActor {
    pub fn new(
        queue_addr: Addr<QueueActor>,
        app_handle: AppHandle,
        auto_read: bool,
    ) -> Self {
        Self {
            queue_addr,
            app_handle,
            playback: PlaybackController::new(),
            busy: false,
            auto_read,
            pending_work: false,
        }
    }
}

impl Actor for SpeechPlayerActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {}
}

struct PlayNext;

impl actix::Message for PlayNext {
    type Result = ();
}

struct PlaybackDone;

impl actix::Message for PlaybackDone {
    type Result = ();
}

impl Handler<PlayNext> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlayNext, ctx: &mut Context<Self>) {
        // Guards against concurrent playback. Message handlers are sequential,
        // but spawned futures run concurrently on the tokio runtime — two PlayNext
        // handlers could otherwise both spawn playback futures for the same item.
        if self.busy || !self.auto_read {
            return;
        }

        self.busy = true;

        let queue_addr = self.queue_addr.clone();
        let app_handle = self.app_handle.clone();
        let playback = self.playback.clone();
        let my_addr = ctx.address();

        let fut = async move {
            // 1. Get next speech to play from QueueActor
            let pending = match queue_addr.send(GetNextSpeech).await {
                Ok(Some(item)) => item,
                _ => {
                    let _ = my_addr.send(PlaybackDone).await;
                    return;
                }
            };

            let id = pending.id;
            let audio_data = match pending.audio_data {
                Some(d) => d,
                None => {
                    let _ = queue_addr.send(SetItemCompleted { id }).await;
                    let _ = app_handle.emit("queue_updated", ());
                    let _ = my_addr.send(PlaybackDone).await;
                    return;
                }
            };

            let _ = app_handle.emit("playback_started", id);
            let _ = app_handle.emit("queue_updated", ());

            // 2. Play audio via spawn_blocking (rodio is blocking)
            let interrupted = play_with_controls(audio_data, 22050u32, &playback).await;

            if interrupted {
                let _ = app_handle.emit("playback_stopped", ());
                let _ = app_handle.emit("queue_updated", ());
            } else {
                let _ = queue_addr.send(SetItemCompleted { id }).await;
                let _ = app_handle.emit("queue_updated", ());
            }

            // 3. Signal done
            let _ = my_addr.send(PlaybackDone).await;
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<PlaybackDone> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackDone, ctx: &mut Context<Self>) {
        self.busy = false;
        // Sends PlayNext synchronously (not via ctx.spawn) to avoid a race:
        // a spawned future could query the queue concurrently with a future
        // spawned by SpeechReady → PlayNext, causing double playback.
        // pending_work prevents infinite loops when the queue is empty.
        if self.auto_read && self.pending_work {
            self.pending_work = false;
            ctx.address().do_send(PlayNext);
        }
    }
}

impl Handler<SpeechReady> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: SpeechReady, ctx: &mut Context<Self>) {
        self.pending_work = true;
        // Only trigger if idle — PlaybackDone will chain the next item when
        // this one finishes. If busy, the current playback future will check
        // pending_work when it completes.
        if !self.busy && self.auto_read {
            ctx.address().do_send(PlayNext);
        }
    }
}

impl Handler<PlaybackPause> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackPause, _: &mut Context<Self>) {
        self.playback.pause();
    }
}

impl Handler<PlaybackResume> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackResume, _: &mut Context<Self>) {
        self.playback.resume();
    }
}

impl Handler<PlaybackStop> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackStop, _: &mut Context<Self>) {
        self.playback.stop();
    }
}

impl Handler<AutoReadChanged> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, msg: AutoReadChanged, ctx: &mut Context<Self>) {
        self.auto_read = msg.value;
        if msg.value {
            self.pending_work = true;
            // If idle, start playing immediately. If busy, PlaybackDone will
            // chain when current playback finishes.
            if !self.busy {
                ctx.address().do_send(PlayNext);
            }
        }
    }
}
