use std::sync::Arc;

use actix::{Actor, ActorFutureExt, Addr, AsyncContext, AtomicResponse, Context, Handler, WrapFuture};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as TokioMutex;

use crate::catalog::{VoiceCatalog, VoiceCatalogOps};
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
    catalog: Arc<VoiceCatalog>,
    app_handle: AppHandle,
}

impl TranscriberActor {
    pub fn new(
        queue_addr: Addr<QueueActor>,
        model_pool: Arc<TokioMutex<ModelPool>>,
        factory: Arc<UnifiedFactory>,
        voice_mapping: Arc<TokioMutex<VoiceMapping>>,
        catalog: Arc<VoiceCatalog>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            queue_addr,
            model_pool,
            factory,
            voice_mapping,
            catalog,
            app_handle,
        }
    }
}

impl Actor for TranscriberActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {}
}

struct Transcribe;

impl actix::Message for Transcribe {
    type Result = ();
}

impl Handler<Transcribe> for TranscriberActor {
    type Result = AtomicResponse<Self, ()>;

    fn handle(&mut self, _: Transcribe, _ctx: &mut Context<Self>) -> Self::Result {
        let queue_addr = self.queue_addr.clone();
        let model_pool = self.model_pool.clone();
        let factory = self.factory.clone();
        let voice_mapping = self.voice_mapping.clone();
        let catalog = self.catalog.clone();
        let app_handle = self.app_handle.clone();

        AtomicResponse::new(Box::pin(
            async move {
                let pending = match queue_addr.send(GetNextText).await {
                    Ok(Some(item)) => item,
                    _ => return,
                };

                if queue_addr
                    .send(MarkProcessing { id: pending.id })
                    .await
                    .is_err()
                {
                    return;
                }

                let id = pending.id;
                let text = pending.text.clone();

                log::debug!("Transcribing item {id}: {}", &text[..text.len().min(50)]);
                let _ = app_handle.emit("transcription_started", (id, text.clone()));

                let installed_langs: Vec<String> = catalog
                    .list_installed()
                    .into_iter()
                    .map(|v| v.language)
                    .collect();

                let language = detect_language_family(&text, &installed_langs).map(|s| s.to_string());

                let voice_key = {
                    let mapping = voice_mapping.lock().await;
                    mapping.resolve(language.as_deref()).map(|s| s.to_string())
                };

                log::info!(
                    "Transcribing item {id} with voice {:?} (language: {:?})",
                    voice_key,
                    language
                );

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

                match result {
                    Ok(audio_data) => {
                        let _ = queue_addr
                            .send(ReplaceWithSpeech {
                                id,
                                audio_data: Some(audio_data),
                                voice_key,
                                language,
                            })
                            .await;
                        let _ = app_handle.emit("transcription_completed", id);
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
                    }
                }
            }
            .into_actor(self)
            .map(|_, _, ctx| {
                ctx.address().do_send(Transcribe);
            }),
        ))
    }
}

impl Handler<TextAdded> for TranscriberActor {
    type Result = ();

    fn handle(&mut self, _: TextAdded, _ctx: &mut Context<Self>) {
        _ctx.address().do_send(Transcribe);
    }
}
