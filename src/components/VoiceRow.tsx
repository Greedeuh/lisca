import type { VoiceEntry } from "../types/piper";
import { formatSize } from "../utils/format";

interface VoiceRowProps {
  voice: VoiceEntry;
  isDownloaded: boolean;
  isDownloading: boolean;
  onDownload: () => void;
  onSelect: () => void;
}

function getVoiceSize(voice: VoiceEntry): number {
  for (const [key, file] of Object.entries(voice.files)) {
    if (key.endsWith(".onnx") && !key.endsWith(".onnx.json")) {
      return file.size_bytes;
    }
  }
  return 0;
}

export function VoiceRow({
  voice,
  isDownloaded,
  isDownloading,
  onDownload,
  onSelect,
}: VoiceRowProps) {
  const size = getVoiceSize(voice);

  return (
    <div className="voice-row">
      <div className="voice-info">
        <span className="voice-name">{voice.name}</span>
        <span className={`quality-badge quality-${voice.quality}`}>
          {voice.quality}
        </span>
        {voice.num_speakers > 1 && (
          <span className="speaker-badge">{voice.num_speakers} speakers</span>
        )}
        <span className="voice-size">{formatSize(size)}</span>
      </div>
      <div className="voice-actions">
        {isDownloaded ? (
          <button className="use-button" onClick={onSelect}>
            Use
          </button>
        ) : (
          <button
            className="download-button"
            onClick={onDownload}
            disabled={isDownloading}
          >
            {isDownloading ? "Downloading..." : "Download"}
          </button>
        )}
      </div>
    </div>
  );
}
