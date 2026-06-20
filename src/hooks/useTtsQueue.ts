import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getQueueState, getPlayerState } from "../types/ipc";
import type { QueueItem } from "../types/queue";

export interface UseTtsQueueReturn {
  items: QueueItem[];
  autoRead: boolean;
  showOverlay: boolean;
  refresh: () => Promise<void>;
}

export function useTtsQueue(): UseTtsQueueReturn {
  const [items, setItems] = useState<QueueItem[]>([]);
  const [autoRead, setAutoRead] = useState(true);
  const [showOverlay, setShowOverlay] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const [snapshot, playerSnapshot] = await Promise.all([
        getQueueState(),
        getPlayerState(),
      ]);
      setItems(snapshot.items);
      setAutoRead(playerSnapshot.auto_read);
      setShowOverlay(snapshot.show_overlay);
    } catch (e) {
      console.error("Failed to load queue:", e);
    }
  }, []);

  useEffect(() => {
    // Fetch initial state
    refresh();

    // Subscribe to queue events
    const queueEvents = [
      "item_added",
      "item_removed",
      "item_moved",
      "item_cleared",
      "item_replaced",
      "config_changed",
      "transcription_started",
      "transcription_completed",
      "playback_started",
      "playback_stopped",
      "playback_paused",
      "playback_resumed",
      "item_paused",
      "item_resumed",
      "item_stopped",
    ] as const;

    const unlisteners = queueEvents.map((event) =>
      listen(event, () => {
        refresh();
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, [refresh]);

  return { items, autoRead, showOverlay, refresh };
}
