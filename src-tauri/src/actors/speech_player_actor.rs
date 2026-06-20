use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use actix::{Actor, Addr, AsyncContext, Context, Handler, WrapFuture};
use tauri::{AppHandle, Emitter};

use crate::speech_player::EndNotifier;

use super::messages::*;
use super::queue_actor::QueueActor;

pub struct SpeechPlayerActor {
    queue_addr: Addr<QueueActor>,
    app_handle: AppHandle,
    sink: Arc<Mutex<Option<rodio::Sink>>>,
    _keepalive: Option<mpsc::Sender<()>>,
    auto_read: bool,
    stopped: bool,
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
            sink: Arc::new(Mutex::new(None)),
            _keepalive: None,
            auto_read,
            stopped: false,
        }
    }

    fn ensure_sink(&mut self) {
        if self.sink.lock().unwrap().is_some() {
            return;
        }

        let sink = self.sink.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (keepalive_tx, keepalive_rx) = mpsc::channel::<()>();

        thread::spawn(move || {
            let (stream, handle) = match rodio::OutputStream::try_default() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to open audio output stream: {e}");
                    let _ = ready_tx.send(false);
                    return;
                }
            };
            let new_sink = match rodio::Sink::try_new(&handle) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to create audio sink: {e}");
                    let _ = ready_tx.send(false);
                    return;
                }
            };
            *sink.lock().unwrap() = Some(new_sink);
            let _ = ready_tx.send(true);

            let _keep_alive = stream;
            let _ = keepalive_rx.recv();
        });

        if ready_rx.recv().unwrap_or(false) {
            self._keepalive = Some(keepalive_tx);
            log::info!("Audio output stream opened");
        } else {
            log::error!("Audio output stream failed to open");
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

#[derive(actix::Message)]
#[rtype(result = "()")]
pub(crate) struct PlaybackComplete {
    pub id: u64,
}

impl Handler<PlayNext> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlayNext, ctx: &mut Context<Self>) {
        if !self.auto_read || self.stopped {
            return;
        }

        self.ensure_sink();

        let sink_empty = self
            .sink
            .lock()
            .unwrap()
            .as_ref()
            .map_or(true, |s| s.empty());

        if !sink_empty {
            return;
        }

        let queue_addr = self.queue_addr.clone();
        let app_handle = self.app_handle.clone();
        let audio_sink = self.sink.clone();
        let my_addr = ctx.address();

        let fut = async move {
            let pending = match queue_addr.send(GetNextSpeech).await {
                Ok(Some(item)) => item,
                _ => return,
            };

            let id = pending.id;
            let audio_data = match pending.audio_data {
                Some(d) => d,
                None => {
                    let _ = queue_addr.send(SetItemCompleted { id }).await;
                    return;
                }
            };

            let _ = app_handle.emit("playback_started", id);

            let sink_guard = audio_sink.lock().unwrap();
            let sink = match sink_guard.as_ref() {
                Some(s) => s,
                None => {
                    drop(sink_guard);
                    let _ = app_handle.emit("playback_stopped", ());
                    return;
                }
            };

            let buffer = rodio::buffer::SamplesBuffer::new(1, 22050, audio_data);
            let notifier = EndNotifier::new(buffer, my_addr, id);
            sink.append(notifier);
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<PlaybackComplete> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, msg: PlaybackComplete, ctx: &mut Context<Self>) {
        if self.stopped {
            return;
        }
        self
            .queue_addr
            .do_send(SetItemCompleted { id: msg.id });
        ctx.address().do_send(PlayNext);
    }
}

impl Handler<SpeechReady> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: SpeechReady, ctx: &mut Context<Self>) {
        self.stopped = false;
        if self.auto_read {
            ctx.address().do_send(PlayNext);
        }
    }
}

impl Handler<PlaybackPause> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackPause, _: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.pause();
        }
    }
}

impl Handler<PlaybackResume> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackResume, _: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.play();
        }
    }
}

impl Handler<PlaybackStop> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackStop, _: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.stop();
        }
        self.stopped = true;
    }
}

impl Handler<AutoReadChanged> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, msg: AutoReadChanged, ctx: &mut Context<Self>) {
        self.auto_read = msg.value;
        if msg.value {
            self.stopped = false;
            ctx.address().do_send(PlayNext);
        }
    }
}
