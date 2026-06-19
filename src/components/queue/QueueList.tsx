import type { QueueItem } from "../../types/queue";
import "./QueueList.css";

const MAX_TEXT_PREVIEW = 80;

interface QueueListProps {
  items: QueueItem[];
  autoRead: boolean;
  onRemove: (id: number) => void;
  onMove: (id: number, index: number) => void;
  onToggleAutoRead: () => void;
  onClear: () => void;
}

function truncate(text: string, max: number): string {
  return text.length > max ? text.slice(0, max) + "…" : text;
}

function statusLabel(item: QueueItem): string {
  if (item.type === "TextMessage") {
    return item.status === "processing" ? "Processing" : "Pending";
  }
  switch (item.status) {
    case "playing":
      return "Playing";
    case "paused":
      return "Paused";
    case "played":
      return "Done";
    default:
      return "Queued";
  }
}

function statusClass(item: QueueItem): string {
  if (item.type === "TextMessage") {
    return item.status === "processing" ? "status-processing" : "status-pending";
  }
  switch (item.status) {
    case "playing":
      return "status-playing";
    case "paused":
      return "status-paused";
    case "played":
      return "status-played";
    default:
      return "status-queued";
  }
}

function TextMessageControls({
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

function SpeechControls({
  item,
  index,
  total,
  onRemove,
  onMove,
}: {
  item: QueueItem & { type: "Speech" };
  index: number;
  total: number;
  onRemove: (id: number) => void;
  onMove: (id: number, index: number) => void;
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
      <button
        className="ql-btn ql-btn-remove"
        onClick={() => onRemove(item.id)}
        aria-label={isPlaying || isPaused ? "Skip" : "Remove"}
      >
        {(isPlaying || isPaused) ? "⏭" : "✕"}
      </button>
    </div>
  );
}

export function QueueList({
  items,
  autoRead,
  onRemove,
  onMove,
  onToggleAutoRead,
  onClear,
}: QueueListProps) {
  if (items.length === 0) {
    return (
      <div className="ql-empty">
        Queue is empty. Use the hotkey to add text.
      </div>
    );
  }

  return (
    <div className="ql-container">
      <div className="ql-items">
        {items.map((item, index) => (
          <div
            key={item.id}
            className={`ql-item ${item.type === "Speech" && (item.status === "playing" || item.status === "paused") ? "ql-item-active" : ""}`}
          >
            <div className="ql-item-body">
              <span className={`ql-status ${statusClass(item)}`}>
                {statusLabel(item)}
              </span>
              {item.language && (
                <span className="ql-lang">{item.language}</span>
              )}
              <span className="ql-text">{truncate(item.text, MAX_TEXT_PREVIEW)}</span>
            </div>
            {item.type === "TextMessage" ? (
              <TextMessageControls item={item} onRemove={onRemove} />
            ) : (
              <SpeechControls
                item={item}
                index={index}
                total={items.length}
                onRemove={onRemove}
                onMove={onMove}
              />
            )}
          </div>
        ))}
      </div>
      <div className="ql-footer">
        <label className="ql-auto-read">
          <input
            type="checkbox"
            checked={autoRead}
            onChange={onToggleAutoRead}
          />
          Auto-play
        </label>
        <button className="ql-btn ql-btn-clear" onClick={onClear}>
          Clear
        </button>
      </div>
    </div>
  );
}
