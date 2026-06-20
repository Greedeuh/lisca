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
      language: string | null;
      voice_key: string | null;
      status: SpeechStatus;
    };

export type QueueSnapshot = {
  items: QueueItem[];
  show_overlay: boolean;
};
