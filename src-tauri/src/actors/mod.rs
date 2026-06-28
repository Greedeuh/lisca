pub(super) mod messages;

mod queue_actor;
mod speech_player_actor;
mod transcriber_actor;

use self::speech_player_actor::SpeechPlayerActor;

use actix::Actor;
use actix::Addr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::app_paths::AppPaths;
use crate::catalog::VoiceCatalog;
use crate::models::{self, ModelFactory};
use crate::queue::Queue;
use crate::transcriber::UnifiedFactory;
use crate::voice_prefs::VoiceMapping;

use self::queue_actor::QueueActor;
use self::transcriber_actor::TranscriberActor;

pub(super) struct AppActors {
    pub(super) queue: Addr<QueueActor>,
    pub(super) player: Addr<SpeechPlayerActor>,
    pub(super) transcriber: Addr<TranscriberActor>,
    pub(super) voice_mapping: Arc<Mutex<VoiceMapping>>,
    pub(super) model_pool: Arc<Mutex<crate::models::ModelPool>>,
}

impl AppActors {
    pub(super) fn new(app_handle: tauri::AppHandle, paths: &AppPaths) -> Self {
        let queue_config_path = paths.app_data_dir.join("queue_config.json");
        let queue_config = Queue::load_config(&queue_config_path);
        let queue = Queue::new()
            .with_config(queue_config)
            .with_config_path(queue_config_path);

        let player_config_path = paths.app_data_dir.join("player_config.json");
        let player_config = SpeechPlayerActor::load_config(&player_config_path);

        let voice_mapping_path = paths.app_data_dir.join("voice_mapping.json");
        let voice_mapping = Arc::new(Mutex::new(VoiceMapping::load(&voice_mapping_path)));

        let model_pool = Arc::new(Mutex::new(
            models::ModelPool::new(4, None).with_config_path(
                paths.app_data_dir.join("pool_config.json"),
            ),
        ));

        let idle_timeout_secs = {
            let pool = model_pool.clone();
            let guard = pool.try_lock();
            match guard {
                Ok(p) => p.idle_timeout_secs(),
                Err(_) => 300,
            }
        };

        let piper_factory: Arc<dyn ModelFactory> = Arc::new(models::PiperFactory::new(
            paths.piper_models_dir.clone(),
            paths.app_data_dir.clone(),
        ));
        let shared_engine_path = paths.kokoro_models_dir.join("kokoro_engine.onnx");
        let kokoro_factory: Arc<dyn ModelFactory> = Arc::new(models::KokoroFactory::new(
            paths.kokoro_models_dir.clone(),
            shared_engine_path,
            paths.resource_dir.clone(),
        ));
        let unified_factory = Arc::new(UnifiedFactory::new(piper_factory, kokoro_factory));

        let catalog = Arc::new(VoiceCatalog::new(
            paths.piper_models_dir.clone(),
            paths.kokoro_models_dir.clone(),
            &paths.resource_dir,
        ));

        let queue_actor = QueueActor::new(queue, app_handle.clone()).start();
        let transcriber_actor = TranscriberActor::new(
            queue_actor.clone(),
            model_pool.clone(),
            unified_factory,
            voice_mapping.clone(),
            catalog,
            app_handle.clone(),
            idle_timeout_secs,
        )
        .start();
        let speech_player_actor =
            SpeechPlayerActor::new(queue_actor.clone(), app_handle, player_config.auto_read)
                .with_config_path(player_config_path)
                .start();

        queue_actor.do_send(messages::SetPlayerAddr {
            addr: speech_player_actor.clone(),
        });

        queue_actor.do_send(messages::SetTranscriberAddr {
            addr: transcriber_actor.clone(),
        });

        AppActors {
            queue: queue_actor,
            player: speech_player_actor,
            transcriber: transcriber_actor,
            voice_mapping,
            model_pool,
        }
    }
}
