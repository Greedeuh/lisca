import type { PlaybackState } from "../types/queue";

interface QueueControlsProps {
  playback: PlaybackState;
  autoRead: boolean;
  showOverlay: boolean;
  onToggleAutoRead: () => void;
  onToggleShowOverlay: () => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onClear: () => void;
  disabled: boolean;
}

export function QueueControls({
  playback,
  autoRead,
  showOverlay,
  onToggleAutoRead,
  onToggleShowOverlay,
  onPause,
  onResume,
  onStop,
  onClear,
  disabled,
}: QueueControlsProps) {
  return (
    <div className="queue-controls">
      <div className="queue-controls-buttons">
        {playback === "playing" ? (
          <button
            onClick={onPause}
            disabled={disabled}
            className="secondary"
          >
            Pause
          </button>
        ) : (
          <button onClick={onResume} disabled={disabled} className="secondary">
            Play
          </button>
        )}
        <button onClick={onStop} disabled={disabled} className="secondary">
          Stop
        </button>
        <button onClick={onClear} disabled={disabled} className="secondary">
          Clear
        </button>
      </div>
      <div className="queue-toggles">
        <label className="queue-auto-read">
          <input
            type="checkbox"
            checked={autoRead}
            onChange={onToggleAutoRead}
          />
          Auto-read
        </label>
        <label className="queue-auto-read">
          <input
            type="checkbox"
            checked={showOverlay}
            onChange={onToggleShowOverlay}
          />
          Show overlay
        </label>
      </div>
    </div>
  );
}
