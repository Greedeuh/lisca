import type { QueueItem } from "./queue";

export type QueueEvent =
  | { type: "item_added" }
  | { type: "item_removed" }
  | { type: "item_moved" }
  | { type: "item_cleared" }
  | { type: "item_replaced" }
  | { type: "config_changed" };

export type TranscriptionEvent =
  | { type: "transcription_started"; item_id: number }
  | { type: "transcription_completed"; item: QueueItem }
  | { type: "transcription_error"; item_id: number; message: string };

export type PlaybackEvent =
  | { type: "playback_started"; item: QueueItem }
  | { type: "playback_paused" }
  | { type: "playback_resumed" }
  | { type: "playback_stopped" }
  | { type: "item_completed"; id: number };

export type ModelEvent =
  | { type: "model_loaded"; voice_key: string }
  | { type: "model_unloaded"; voice_key: string };

export type DownloadEvent =
  | {
      type: "download_progress";
      voice_key: string;
      bytes_downloaded: number;
      total_bytes: number;
    }
  | { type: "download_complete"; voice_key: string }
  | { type: "voice_installed"; voice_key: string }
  | { type: "voice_uninstalled"; voice_key: string };

export type AppEvent =
  | QueueEvent
  | TranscriptionEvent
  | PlaybackEvent
  | ModelEvent
  | DownloadEvent;
