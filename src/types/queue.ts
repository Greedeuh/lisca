export interface QueueItem {
  id: number;
  text: string;
  language: string | null;
}

export type PlaybackState = "idle" | "playing" | "paused";

export interface QueueSnapshot {
  items: QueueItem[];
  playback: PlaybackState;
  auto_read: boolean;
  show_overlay: boolean;
}

export interface QueueConfig {
  max_items: number;
  auto_read: boolean;
  show_overlay: boolean;
}

export type QueueEvent =
  | { type: "playback_started"; item: QueueItem }
  | { type: "item_completed"; id: number }
  | { type: "playback_paused" }
  | { type: "playback_resumed" }
  | { type: "playback_stopped" }
  | { type: "queue_updated"; items: QueueItem[]; auto_read: boolean; show_overlay: boolean }
  | { type: "error"; id: number | null; message: string }
  | { type: "processor_idle" };
