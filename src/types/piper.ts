export interface VoiceCatalog {
  [key: string]: VoiceEntry;
}

export interface VoiceEntry {
  key: string;
  name: string;
  language: VoiceLanguage;
  quality: string;
  num_speakers: number;
  speaker_id_map: Record<string, number>;
  files: Record<string, VoiceFile>;
  aliases: string[];
}

export interface VoiceLanguage {
  code: string;
  family: string;
  region: string;
  name_native: string;
  name_english: string;
  country_english: string;
}

export interface VoiceFile {
  size_bytes: number;
  md5_digest: string;
}

export interface InstalledModel {
  voice_key: string;
  model_path: string;
  config_path: string;
  language: VoiceLanguage;
  quality: string;
  name: string;
}

export type DownloadProgress =
  | { type: "downloading"; voice_key: string; bytes_downloaded: number; total_bytes: number }
  | { type: "complete"; voice_key: string };

export interface VoiceGroup {
  family: string;
  familyName: string;
  locales: LocaleGroup[];
}

export interface LocaleGroup {
  code: string;
  name: string;
  country: string;
  voices: VoiceEntry[];
}
