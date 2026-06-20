import { useState, useEffect, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { QueueListView } from "../components/queue/QueueListView";
import type { QueueItem } from "../types/queue";
import { getQueueState, getPlayerState, queueRemove, queueMove, queueClear, queueToggleAutoRead, queueToggleOverlay } from "../types/ipc";
import "./OverlayApp.css";

export default function OverlayApp() {
  const [items, setItems] = useState<QueueItem[]>([]);
  const [autoRead, setAutoRead] = useState(true);

  const refreshQueue = useCallback(async () => {
    try {
      const [snapshot, playerSnapshot] = await Promise.all([
        getQueueState(),
        getPlayerState(),
      ]);
      setItems(snapshot.items);
      setAutoRead(playerSnapshot.auto_read);

      if (snapshot.items.length === 0) {
        const win = getCurrentWindow();
        await win.hide();
      }
    } catch {}
  }, []);

  useEffect(() => {
    refreshQueue();
  }, [refreshQueue]);

  const handleRemove = useCallback(
    async (id: number) => {
      try {
        await queueRemove(id);
        await refreshQueue();
      } catch {}
    },
    [refreshQueue],
  );

  const handleMove = useCallback(
    async (id: number, index: number) => {
      try {
        await queueMove(id, index);
        await refreshQueue();
      } catch {}
    },
    [refreshQueue],
  );

  const handleClear = useCallback(async () => {
    try {
      await queueClear();
      await refreshQueue();
    } catch {}
  }, [refreshQueue]);

  const handleToggleAutoRead = useCallback(async () => {
    try {
      const val = await queueToggleAutoRead();
      setAutoRead(val);
    } catch {}
  }, []);

  const handleClose = useCallback(async () => {
    try {
      const snapshot = await getQueueState();
      if (snapshot.show_overlay) {
        await queueToggleOverlay();
      }
    } catch {}
    const win = getCurrentWindow();
    await win.hide();
  }, []);

  const handleDragStart = useCallback(async (e: React.MouseEvent) => {
    if (e.target instanceof HTMLElement && e.target.closest(".ol-close")) return;
    const win = getCurrentWindow();
    await win.startDragging();
  }, []);

  return (
    <div className="ol-container">
      <div className="ol-header" data-tauri-drag-region onMouseDown={handleDragStart}>
        <span className="ol-title">Lisca</span>
        <button className="ol-close" onClick={handleClose}>
          ✕
        </button>
      </div>
      <div className="ol-body">
        <QueueListView
          items={items}
          autoRead={autoRead}
          onRemove={handleRemove}
          onMove={handleMove}
          onToggleAutoRead={handleToggleAutoRead}
          onClear={handleClear}
        />
      </div>
    </div>
  );
}
