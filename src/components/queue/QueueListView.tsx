import type { QueueItem } from "../../types/queue";
import { MAX_TEXT_PREVIEW, truncate, statusLabel, statusClass } from "./queueUtils";
import { TextMessageControls, SpeechControls } from "./QueueItemControls";
import { PlaybackControls } from "./PlaybackControls";
import "./QueueList.css";

export interface QueueListViewProps {
  items: QueueItem[];
  autoRead: boolean;
  onRemove: (id: number) => void;
  onMove: (id: number, index: number) => void;
  onToggleAutoRead: () => void;
  onClear: () => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onSkip: () => void;
  onRestart: () => void;
}

export function QueueListView({
  items,
  autoRead,
  onRemove,
  onMove,
  onToggleAutoRead,
  onClear,
  onPause,
  onResume,
  onStop,
  onSkip,
  onRestart,
}: QueueListViewProps) {
  const activeItem = items.find(
    (item): item is QueueItem & { type: "Speech" } =>
      item.type === "Speech" &&
      (item.status === "playing" || item.status === "paused"),
  );

  return (
    <div className="ql-container">
      <PlaybackControls
        item={activeItem ?? null}
        hasItems={items.length > 0}
        onPause={onPause}
        onResume={onResume}
        onStop={onStop}
        onRestart={onRestart}
      />
      {items.length === 0 ? (
        <div className="ql-empty">
          Queue is empty. Use the hotkey to add text.
        </div>
      ) : (
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
                  onStop={onStop}
                  onSkip={onSkip}
                  onRestart={onRestart}
                />
              )}
            </div>
          ))}
        </div>
      )}
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
