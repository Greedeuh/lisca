import type { QueueItem, PlaybackState } from "../types/queue";

interface QueueListProps {
  items: QueueItem[];
  current: QueueItem | null;
  playback: PlaybackState;
  onRemove: (id: number) => void;
  onMove: (id: number, index: number) => void;
}

export function QueueList({
  items,
  current,
  playback,
  onRemove,
  onMove,
}: QueueListProps) {
  if (!current && items.length === 0) {
    return <p className="queue-empty">Queue is empty. Use the hotkey to add text.</p>;
  }

  return (
    <div className="queue-list">
      {current && (
        <div className="queue-item queue-item-playing">
          <div className="queue-item-info">
            <span className="queue-now-playing">
              {playback === "paused" ? "Paused" : "Playing"}
            </span>
            <span className="queue-item-text">{current.text}</span>
          </div>
          <button
            onClick={() => onRemove(current.id)}
            className="secondary queue-item-remove"
          >
            Skip
          </button>
        </div>
      )}
      {items.map((item, index) => (
        <div key={item.id} className="queue-item">
          <div className="queue-item-info">
            <span className="queue-item-index">{index + 1}</span>
            <span className="queue-item-text">{item.text}</span>
          </div>
          <div className="queue-item-actions">
            {index > 0 && (
              <button
                onClick={() => onMove(item.id, index - 1)}
                className="secondary queue-item-btn"
              >
                Up
              </button>
            )}
            {index < items.length - 1 && (
              <button
                onClick={() => onMove(item.id, index + 1)}
                className="secondary queue-item-btn"
              >
                Down
              </button>
            )}
            <button
              onClick={() => onRemove(item.id)}
              className="secondary queue-item-remove"
            >
              Remove
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
