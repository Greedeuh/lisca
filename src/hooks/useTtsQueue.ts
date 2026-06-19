import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getQueueState } from "../types/ipc";
import type { QueueItem, QueueSnapshot } from "../types/queue";

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
      const snapshot: QueueSnapshot = await getQueueState();
      setItems(snapshot.items);
      setAutoRead(snapshot.auto_read);
      setShowOverlay(snapshot.show_overlay);
    } catch (e) {
      console.error("Failed to load queue:", e);
    }
  }, []);

  useEffect(() => {
    // Fetch initial state
    refresh();

    // Subscribe to queue events
    const unlistenQueue = listen("queue_updated", () => {
      refresh();
    });

    const unlistenTranscriptionStarted = listen("transcription_started", () => {
      refresh();
    });

    const unlistenTranscriptionCompleted = listen("transcription_completed", () => {
      refresh();
    });

    const unlistenPlaybackStarted = listen("playback_started", () => {
      refresh();
    });

    const unlistenPlaybackStopped = listen("playback_stopped", () => {
      refresh();
    });

    const unlistenItemCompleted = listen("item_completed", () => {
      refresh();
    });

    return () => {
      unlistenQueue.then((fn) => fn());
      unlistenTranscriptionStarted.then((fn) => fn());
      unlistenTranscriptionCompleted.then((fn) => fn());
      unlistenPlaybackStarted.then((fn) => fn());
      unlistenPlaybackStopped.then((fn) => fn());
      unlistenItemCompleted.then((fn) => fn());
    };
  }, [refresh]);

  return { items, autoRead, showOverlay, refresh };
}
