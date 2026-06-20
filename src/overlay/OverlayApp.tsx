import { useState, useEffect, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { QueueListView } from "../components/queue/QueueListView";
import type { QueueItem } from "../types/queue";
import { getQueueState, getPlayerState, queueRemove, queueMove, queueClear, queueToggleAutoRead, queueToggleOverlay, playbackPause, playbackResume, playbackStop, playbackSkip } from "../types/ipc";
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

    const events = [
      "item_added",
      "item_removed",
      "item_moved",
      "item_cleared",
      "item_replaced",
      "config_changed",
      "playback_started",
      "playback_stopped",
      "playback_paused",
      "playback_resumed",
      "item_paused",
      "item_resumed",
      "item_stopped",
    ] as const;

    const unlisteners = events.map((event) =>
      listen(event, () => {
        refreshQueue();
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
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

  const handlePause = useCallback(async () => {
    try {
      await playbackPause();
      await refreshQueue();
    } catch {}
  }, [refreshQueue]);

  const handleResume = useCallback(async () => {
    try {
      await playbackResume();
      await refreshQueue();
    } catch {}
  }, [refreshQueue]);

  const handleStop = useCallback(async () => {
    try {
      await playbackStop();
      await refreshQueue();
    } catch {}
  }, [refreshQueue]);

  const handleSkip = useCallback(async () => {
    try {
      await playbackSkip();
      await refreshQueue();
    } catch {}
  }, [refreshQueue]);

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
          onPause={handlePause}
          onResume={handleResume}
          onStop={handleStop}
          onSkip={handleSkip}
        />
      </div>
    </div>
  );
}
