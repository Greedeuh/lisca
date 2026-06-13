import { formatSize } from "../utils/format";

interface DownloadProgressProps {
  voiceKey: string;
  bytesDownloaded: number;
  totalBytes: number;
}

export function DownloadProgress({
  voiceKey,
  bytesDownloaded,
  totalBytes,
}: DownloadProgressProps) {
  const percentage = totalBytes > 0 ? (bytesDownloaded / totalBytes) * 100 : 0;

  return (
    <div className="download-progress">
      <div className="download-info">
        <span className="download-voice">{voiceKey}</span>
        <span className="download-size">
          {formatSize(bytesDownloaded)} / {formatSize(totalBytes)}
        </span>
      </div>
      <div className="progress-bar">
        <div
          className="progress-fill"
          style={{ width: `${percentage}%` }}
        />
      </div>
    </div>
  );
}
