import type { QueueItem } from "../../types/queue";

export function TextMessageControls({
  item,
  onRemove,
}: {
  item: QueueItem & { type: "TextMessage" };
  onRemove: (id: number) => void;
}) {
  return (
    <button
      className="ql-btn ql-btn-remove"
      onClick={() => onRemove(item.id)}
      aria-label="Remove"
    >
      ✕
    </button>
  );
}

export function SpeechControls({
  item,
  index,
  total,
  onRemove,
  onMove,
  onStop,
  onSkip,
}: {
  item: QueueItem & { type: "Speech" };
  index: number;
  total: number;
  onRemove: (id: number) => void;
  onMove: (id: number, index: number) => void;
  onStop: () => void;
  onSkip: () => void;
}) {
  const isPlaying = item.status === "playing";
  const isPaused = item.status === "paused";

  return (
    <div className="ql-controls">
      {index > 0 && (
        <button
          className="ql-btn"
          onClick={() => onMove(item.id, index - 1)}
          aria-label="Move up"
        >
          ▲
        </button>
      )}
      {index < total - 1 && (
        <button
          className="ql-btn"
          onClick={() => onMove(item.id, index + 1)}
          aria-label="Move down"
        >
          ▼
        </button>
      )}
      {(isPlaying || isPaused) && (
        <button
          className="ql-btn"
          onClick={() => onSkip()}
          aria-label="Skip"
        >
          ⏭
        </button>
      )}
      <button
        className="ql-btn ql-btn-remove"
        onClick={async () => {
          if (isPlaying || isPaused) {
            await onStop();
          }
          await onRemove(item.id);
        }}
        aria-label="Remove"
      >
        ✕
      </button>
    </div>
  );
}
