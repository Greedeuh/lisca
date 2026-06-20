import type { QueueItem } from "../../types/queue";

interface PlaybackControlsProps {
  item: (QueueItem & { type: "Speech" }) | null;
  hasItems: boolean;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onRestart: () => void;
}

export function PlaybackControls({
  item,
  hasItems,
  onPause,
  onResume,
  onStop,
  onRestart,
}: PlaybackControlsProps) {
  const isPlaying = item?.status === "playing";
  const isPaused = item?.status === "paused";
  const active = isPlaying || isPaused;
  const canControl = active || hasItems;

  return (
    <div className={`ql-playback ${active ? "ql-playback-active" : ""}`}>
      <div className="ql-playback-controls">
        <button
          className="ql-btn"
          onClick={onRestart}
          aria-label="Restart"
          disabled={!active}
        >
          ↺
        </button>
        <button
          className="ql-btn"
          onClick={isPlaying ? onPause : onResume}
          aria-label={isPlaying ? "Pause" : "Resume"}
          disabled={!canControl}
        >
          {isPlaying ? "⏸" : "▶"}
        </button>
        <button
          className="ql-btn ql-btn-stop"
          onClick={onStop}
          aria-label="Stop"
          disabled={!active}
        >
          ■
        </button>
      </div>
    </div>
  );
}
