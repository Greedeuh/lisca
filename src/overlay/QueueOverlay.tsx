import { useState, useEffect } from "react";
import { useTtsQueue } from "../hooks/useTtsQueue";
import { listen } from "@tauri-apps/api/event";

export function QueueOverlay() {
  const {
    items,
    current,
    playback,
    autoRead,
    remove,
    pause,
    resume,
    stop,
    clear,
    toggleAutoRead,
    hideOverlay,
  } = useTtsQueue();

  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const unlisten = listen<boolean>("overlay-visibility", (event) => {
      setVisible(event.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const totalItems = items.length + (current ? 1 : 0);

  const handleToggleOverlay = async () => {
    await hideOverlay();
  };

  return (
    <div className={`queue-overlay${visible ? "" : " hidden"}`}>
      <div className="overlay-header" data-tauri-drag-region>
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
        {current ? (
          <div className="overlay-now-playing">
            <div className="overlay-now-label">
              {playback === "paused" ? "Paused" : "Playing"}
            </div>
            <div className="overlay-now-text">{current.text}</div>
            <div className="overlay-now-actions">
              <button
                className="overlay-btn"
                onClick={playback === "playing" ? pause : resume}
              >
                {playback === "playing" ? "⏸" : "▶"}
              </button>
              <button className="overlay-btn overlay-btn-skip" onClick={stop}>
                ⏭
              </button>
            </div>
          </div>
        ) : (
          <div className="overlay-empty">Queue empty</div>
        )}

        {items.length > 0 && (
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
