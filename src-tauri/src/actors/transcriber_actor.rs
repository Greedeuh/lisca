use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext, Context, Handler, WrapFuture};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as TokioMutex;

use crate::models::ModelPool;
use crate::transcriber::{detect_language_family, UnifiedFactory};
use crate::voice_prefs::VoiceMapping;

use super::messages::*;
use super::queue_actor::QueueActor;

pub struct TranscriberActor {
    queue_addr: Addr<QueueActor>,
    model_pool: Arc<TokioMutex<ModelPool>>,
    factory: Arc<UnifiedFactory>,
    voice_mapping: Arc<TokioMutex<VoiceMapping>>,
    app_handle: AppHandle,
    busy: bool,
}

impl TranscriberActor {
    pub fn new(
        queue_addr: Addr<QueueActor>,
        model_pool: Arc<TokioMutex<ModelPool>>,
        factory: Arc<UnifiedFactory>,
        voice_mapping: Arc<TokioMutex<VoiceMapping>>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            queue_addr,
            model_pool,
            factory,
            voice_mapping,
            app_handle,
            busy: false,
        }
    }
}

impl Actor for TranscriberActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(std::time::Duration::from_millis(500), |act, ctx| {
            if !act.busy {
                ctx.address().do_send(PollNextTextTick);
            }
        });
    }
}

struct PollNextTextTick;

impl actix::Message for PollNextTextTick {
    type Result = ();
}

struct TranscriptionDone;

impl actix::Message for TranscriptionDone {
    type Result = ();
}

impl Handler<PollNextTextTick> for TranscriberActor {
    type Result = ();

    fn handle(&mut self, _: PollNextTextTick, ctx: &mut Context<Self>) {
        if self.busy {
            return;
        }

        self.busy = true;

        let queue_addr = self.queue_addr.clone();
        let model_pool = self.model_pool.clone();
        let factory = self.factory.clone();
        let voice_mapping = self.voice_mapping.clone();
        let app_handle = self.app_handle.clone();
        let my_addr = ctx.address();

        let fut = async move {
            // 1. Get next pending text from QueueActor
            let pending = match queue_addr.send(PollNextText).await {
                Ok(Some(item)) => item,
                _ => {
                    let _ = my_addr.send(TranscriptionDone).await;
                    return;
                }
            };

            // 2. Mark as Processing
            if queue_addr
                .send(MarkProcessing { id: pending.id })
                .await
                .is_err()
            {
                let _ = my_addr.send(TranscriptionDone).await;
                return;
            }

            let id = pending.id;
            let text = pending.text.clone();
            let language = pending.language;

            log::debug!("Transcribing item {id}: {}", &text[..text.len().min(50)]);
            let _ = app_handle.emit("transcription_started", (id, text.clone()));

            // 3. Detect language
            let detected_lang = detect_language_family(&text);
            let resolved_language = language
                .clone()
                .or_else(|| detected_lang.map(String::from));
            let lang = resolved_language.as_deref();

            // 4. Resolve voice
            let voice_key = {
                let mapping = voice_mapping.lock().await;
                mapping.resolve(lang).map(|s| s.to_string())
            };

            // 5. Synthesize
            let result = match voice_key {
                Some(ref vk) => {
                    let model = {
                        let mut pool = model_pool.lock().await;
                        pool.get(vk, factory.as_ref()).await
                    };
                    match model {
                        Ok(m) => {
                            let text_clone = text.clone();
                            tokio::task::spawn_blocking(move || {
                                let mut model = m.blocking_lock();
                                model.synthesize(&text_clone)
                            })
                            .await
                            .unwrap_or_else(|e| Err(format!("synthesis task panicked: {e}")))
                        }
                        Err(e) => Err(e),
                    }
                }
                None => Err("no voice resolved for language".to_string()),
            };

            // 6. Report back
            match result {
                Ok(audio_data) => {
                    let _ = queue_addr
                        .send(ReplaceWithSpeech {
                            id,
                            audio_data: Some(audio_data),
                            voice_key,
                            language: resolved_language,
                        })
                        .await;
                    let _ = app_handle.emit("transcription_completed", id);
                    let _ = app_handle.emit("queue_updated", ());
                }
                Err(e) => {
                    log::error!("Transcription error for item {id}: {e}");
                    let _ = queue_addr
                        .send(SetTranscriptionError {
                            id,
                            error: e.clone(),
                        })
                        .await;
                    let _ = app_handle.emit("transcription_error", (id, e));
                    let _ = app_handle.emit("queue_updated", ());
                }
            }

            // 7. Signal done
            let _ = my_addr.send(TranscriptionDone).await;
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<TranscriptionDone> for TranscriberActor {
    type Result = ();

    fn handle(&mut self, _: TranscriptionDone, _: &mut Context<Self>) {
        self.busy = false;
    }
}

impl Handler<WakeTranscriber> for TranscriberActor {
    type Result = ();

    fn handle(&mut self, _: WakeTranscriber, ctx: &mut Context<Self>) {
        if !self.busy {
            ctx.address().do_send(PollNextTextTick);
        }
    }
}
