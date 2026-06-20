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
        }
    }
}

impl Actor for SpeechPlayerActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(std::time::Duration::from_millis(200), |act, ctx| {
            if !act.busy && act.auto_read {
                ctx.address().do_send(PollNextSpeechTick);
            }
        });
    }
}

struct PollNextSpeechTick;

impl actix::Message for PollNextSpeechTick {
    type Result = ();
}

struct PlaybackDone;

impl actix::Message for PlaybackDone {
    type Result = ();
}

impl Handler<PollNextSpeechTick> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PollNextSpeechTick, ctx: &mut Context<Self>) {
        if self.busy || !self.auto_read {
            return;
        }

        self.busy = true;

        let queue_addr = self.queue_addr.clone();
        let app_handle = self.app_handle.clone();
        let stop_flag = self.playback.stop_flag();
        let pause_flag = self.playback.pause_flag();
        let state_arc = self.playback.state_arc();
        let my_addr = ctx.address();

        let fut = async move {
            // 1. Get next speech to play from QueueActor
            let pending = match queue_addr.send(PollNextSpeech).await {
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
            let interrupted = play_with_controls(
                audio_data,
                22050u32,
                stop_flag,
                pause_flag,
                state_arc,
            )
            .await;

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

    fn handle(&mut self, _: PlaybackDone, _: &mut Context<Self>) {
        self.busy = false;
    }
}

impl Handler<SpeechReady> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: SpeechReady, ctx: &mut Context<Self>) {
        if !self.busy && self.auto_read {
            ctx.address().do_send(PollNextSpeechTick);
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

    fn handle(&mut self, msg: AutoReadChanged, _: &mut Context<Self>) {
        self.auto_read = msg.value;
    }
}
