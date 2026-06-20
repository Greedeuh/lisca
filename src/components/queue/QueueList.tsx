import { useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useToast } from "../../contexts/toast";
import { useTtsQueue } from "../../hooks";
import {
  queueRemove,
  queueMove,
  queueClear,
  queueToggleAutoRead,
} from "../../types/ipc";
import { QueueListView } from "./QueueListView";

export function QueueList() {
  const { addToast } = useToast();
  const { items, autoRead, refresh } = useTtsQueue();

  useEffect(() => {
    const unlisten = listen<{ id: number; error: string }>(
      "transcription_error",
      (event) => {
        addToast(`Transcription failed: ${event.payload.error}`);
        refresh();
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addToast, refresh]);

  const handleRemove = useCallback(
    async (id: number) => {
      try {
        await queueRemove(id);
        await refresh();
      } catch (e) {
        addToast(`Failed to remove item: ${e}`);
      }
    },
    [addToast, refresh],
  );

  const handleMove = useCallback(
    async (id: number, index: number) => {
      try {
        await queueMove(id, index);
        await refresh();
      } catch (e) {
        addToast(`Failed to move item: ${e}`);
      }
    },
    [addToast, refresh],
  );

  const handleClear = useCallback(async () => {
    try {
      await queueClear();
      await refresh();
    } catch (e) {
      addToast(`Failed to clear queue: ${e}`);
    }
  }, [addToast, refresh]);

  const handleToggleAutoRead = useCallback(async () => {
    try {
      await queueToggleAutoRead();
      await refresh();
    } catch (e) {
      addToast(`Failed to toggle auto-read: ${e}`);
    }
  }, [addToast, refresh]);

  return (
    <QueueListView
      items={items}
      autoRead={autoRead}
      onRemove={handleRemove}
      onMove={handleMove}
      onToggleAutoRead={handleToggleAutoRead}
      onClear={handleClear}
    />
  );
}
