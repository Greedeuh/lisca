mod end_notifier;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use actix::{Actor, Addr, AsyncContext, Context, Handler, WrapFuture};
use tauri::{AppHandle, Emitter};

use crate::persist::{load_json, save_json};

use self::end_notifier::EndNotifier;
use super::messages::*;
use super::queue_actor::QueueActor;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub(super) struct PlayerConfig {
    pub(super) auto_read: bool,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self { auto_read: true }
    }
}

pub(crate) struct SpeechPlayerActor {
    queue_addr: Addr<QueueActor>,
    app_handle: AppHandle,
    sink: Arc<Mutex<Option<rodio::Sink>>>,
    _keepalive: Option<mpsc::Sender<()>>,
    auto_read: bool,
    stopped: bool,
    current_id: Option<u64>,
    config_path: Option<std::path::PathBuf>,
}

impl SpeechPlayerActor {
    pub(super) fn new(
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
            current_id: None,
            config_path: None,
        }
    }

    pub(super) fn with_config_path(mut self, path: std::path::PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub(super) fn load_config(path: &std::path::Path) -> PlayerConfig {
        load_json(path)
    }

    fn save_config(&self) -> Result<(), String> {
        let path = self
            .config_path
            .as_ref()
            .ok_or("no config path configured")?;
        let config = PlayerConfig {
            auto_read: self.auto_read,
        };
        save_json(path, &config)
    }

    fn init_sink(&mut self) {
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
    fn started(&mut self, _ctx: &mut Self::Context) {
        self.init_sink();
    }
}

struct PlayNext;

impl actix::Message for PlayNext {
    type Result = ();
}

impl Handler<PlayNext> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlayNext, ctx: &mut Context<Self>) {
        log::debug!(
            "PlayNext received auto_read:{}, stopped:{}",
            self.auto_read,
            self.stopped
        );
        if self.stopped {
            return;
        }

        let sink_empty = self
            .sink
            .lock()
            .unwrap()
            .as_ref()
            .map_or(true, |s| s.empty());

        log::debug!("PlayNext sink_empty: {}", sink_empty);

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
            let _ = my_addr.send(SetCurrentId { id: Some(id) }).await;

            let audio_data = match pending.audio_data {
                Some(d) => d,
                None => {
                    let _ = queue_addr.send(SetItemCompleted { id }).await;
                    let _ = my_addr.send(SetCurrentId { id: None }).await;
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
            sink.play();
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
        self.current_id = None;
        self.queue_addr.do_send(SetItemCompleted { id: msg.id });
        ctx.address().do_send(PlayNext);
    }
}

impl Handler<SpeechReady> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: SpeechReady, ctx: &mut Context<Self>) {
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
        if let Some(id) = self.current_id {
            let _ = self.queue_addr.do_send(SetSpeechPaused { id });
        }
        self.app_handle.emit("playback_paused", ()).ok();
    }
}

impl Handler<PlaybackResume> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackResume, ctx: &mut Context<Self>) {
        self.stopped = false;

        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.play();
            log::debug!("PlaybackResume: sink.empty(): {}", sink.empty());

            if sink.empty() {
                log::debug!("PlaybackResume: sink is empty sending PlayNext");
                ctx.address().do_send(PlayNext);
            }
        }
        if let Some(id) = self.current_id {
            let _ = self.queue_addr.do_send(SetSpeechResumed { id });
        }
        self.app_handle.emit("playback_resumed", ()).ok();
    }
}

impl Handler<PlaybackStop> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackStop, _: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.stop();
            sink.clear();
        }
        if let Some(id) = self.current_id {
            let _ = self.queue_addr.do_send(SetSpeechStopped { id });
            self.current_id = None;
        }
        self.stopped = true;
        self.app_handle.emit("playback_stopped", ()).ok();
    }
}

impl Handler<PlaybackSkip> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackSkip, ctx: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.stop();
            sink.clear();
            log::debug!("PlaybackSkip sink.empty() {:?}", sink.empty());
        }

        let current_id = self.current_id;
        let queue_addr = self.queue_addr.clone();
        let app_handle = self.app_handle.clone();
        let my_addr = ctx.address();

        let fut = async move {
            if let Some(id) = current_id {
                let _ = queue_addr.send(SkipItem { id }).await;
            }
            app_handle.emit("playback_stopped", ()).ok();
            my_addr.do_send(PlayNext);
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<PlaybackRestart> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, _: PlaybackRestart, _: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            if let Err(e) = sink.try_seek(Duration::from_secs(0)) {
                log::warn!("Failed to seek to start: {e}");
            }
        }
    }
}

impl Handler<PlaybackReplay> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, msg: PlaybackReplay, ctx: &mut Context<Self>) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.stop();
            sink.clear();
        }

        let queue_addr = self.queue_addr.clone();
        let my_addr = ctx.address();

        let fut = async move {
            let _ = queue_addr.send(ReplayItem { id: msg.id }).await;
            my_addr.do_send(PlayNext);
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<ToggleAutoRead> for SpeechPlayerActor {
    type Result = bool;

    fn handle(&mut self, _: ToggleAutoRead, ctx: &mut Context<Self>) -> Self::Result {
        self.auto_read = !self.auto_read;
        if let Err(e) = self.save_config() {
            log::error!("Failed to save player config: {e}");
        }
        self.app_handle.emit("config_changed", ()).ok();
        if self.auto_read {
            self.stopped = false;
            ctx.address().do_send(PlayNext);
        }
        self.auto_read
    }
}

impl Handler<GetAutoRead> for SpeechPlayerActor {
    type Result = bool;

    fn handle(&mut self, _: GetAutoRead, _: &mut Context<Self>) -> Self::Result {
        self.auto_read
    }
}

impl Handler<SetCurrentId> for SpeechPlayerActor {
    type Result = ();

    fn handle(&mut self, msg: SetCurrentId, _: &mut Context<Self>) {
        self.current_id = msg.id;
    }
}
