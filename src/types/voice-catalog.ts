export type VoiceEntry = {
  voice_key: string;
  name: string;
  language: string;
  quality: string;
  size_bytes: number;
  speed: string | null;
  model_type: "piper" | "kokoro";
};

export type InstalledVoice = {
  voice_key: string;
  name: string;
  language: string;
  quality: string;
  model_type: "piper" | "kokoro";
  model_path: string;
};

export type DownloadProgress =
  | {
      type: "downloading";
      voice_key: string;
      bytes_downloaded: number;
      total_bytes: number;
    }
  | { type: "complete"; voice_key: string }
  | { type: "error"; voice_key: string; reason: string };
