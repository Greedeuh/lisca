export type TextMessageStatus = "pending" | "processing";

export type SpeechStatus = "to_play" | "playing" | "paused" | "played";

export type QueueItem =
  | {
      type: "TextMessage";
      id: number;
      text: string;
      language: string | null;
      status: TextMessageStatus;
    }
  | {
      type: "Speech";
      id: number;
      text: string;
      audio_path: string | null;
      voice_key: string | null;
      language: string | null;
      status: SpeechStatus;
    };

export type QueueSnapshot = {
  items: QueueItem[];
  auto_read: boolean;
  show_overlay: boolean;
};
