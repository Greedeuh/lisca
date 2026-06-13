import { useTtsQueue } from "../hooks/useTtsQueue";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function QueueOverlay() {
  const {
    items,
    autoRead,
    showOverlay,
    remove,
    clear,
    toggleAutoRead,
    toggleShowOverlay,
  } = useTtsQueue();

  const totalItems = items.length;

  const handleToggleOverlay = async () => {
    await toggleShowOverlay();
    if (showOverlay) {
      getCurrentWindow().hide();
    }
  };

  return (
    <div className="queue-overlay">
      <div className="overlay-header">
        <span className="overlay-title">Lisca</span>
        <div className="overlay-header-right">
          <label className="overlay-auto-read">
            <input
              type="checkbox"
              checked={autoRead}
              onChange={toggleAutoRead}
            />
            <span>Auto</span>
          </label>
          <button
            className="overlay-btn-close"
            onClick={handleToggleOverlay}
            title="Hide overlay"
          >
            ✕
          </button>
        </div>
      </div>

      <div className="overlay-body">
        {items.length > 0 ? (
          <div className="overlay-queue-list">
            {items.map((item) => (
              <div key={item.id} className="overlay-queue-item">
                <span className="overlay-queue-text">{item.text}</span>
                <button
                  className="overlay-btn-remove"
                  onClick={() => remove(item.id)}
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="overlay-empty">Queue empty</div>
        )}
      </div>

      {totalItems > 0 && (
        <div className="overlay-footer">
          <button className="overlay-btn-clear" onClick={clear}>
            Clear
          </button>
        </div>
      )}
    </div>
  );
}
