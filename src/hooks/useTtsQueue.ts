import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  QueueItem,
  QueueSnapshot,
  QueueEvent,
  PlaybackState,
} from "../types/queue";

export function useTtsQueue() {
  const [items, setItems] = useState<QueueItem[]>([]);
  const [current, setCurrent] = useState<QueueItem | null>(null);
  const [playback, setPlayback] = useState<PlaybackState>("idle");
  const [autoRead, setAutoRead] = useState(true);

  useEffect(() => {
    invoke<QueueSnapshot>("tts_queue_state").then((snap) => {
      setItems(snap.items);
      setCurrent(snap.current);
      setPlayback(snap.playback);
      setAutoRead(snap.auto_read);
    });
  }, []);

  useEffect(() => {
    const unlisten = listen<QueueEvent>("tts-queue-event", (event) => {
      const e = event.payload;
      switch (e.type) {
        case "queue_updated":
          setItems(e.items);
          setAutoRead(e.auto_read);
          break;
        case "playback_started":
          setCurrent(e.item);
          setPlayback("playing");
          setItems((prev) => prev.filter((i) => i.id !== e.item.id));
          break;
        case "item_completed":
          setCurrent(null);
          setPlayback("idle");
          break;
        case "playback_paused":
          setPlayback("paused");
          break;
        case "playback_resumed":
          setPlayback("playing");
          break;
        case "playback_stopped":
          setCurrent(null);
          setPlayback("idle");
          break;
        case "error":
          console.error("TTS error:", e.message);
          break;
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const addItem = useCallback(async (text: string) => {
    return invoke<QueueItem>("tts_queue_add", { text });
  }, []);

  const remove = useCallback(async (id: number) => {
    await invoke("tts_queue_remove", { id });
  }, []);

  const moveItem = useCallback(async (id: number, index: number) => {
    await invoke("tts_queue_move", { id, index });
  }, []);

  const clear = useCallback(async () => {
    await invoke("tts_queue_clear");
  }, []);

  const pause = useCallback(async () => {
    await invoke("tts_pause");
  }, []);

  const resume = useCallback(async () => {
    await invoke("tts_resume");
  }, []);

  const stop = useCallback(async () => {
    await invoke("tts_stop");
  }, []);

  const toggleAutoRead = useCallback(async () => {
    const newValue = !autoRead;
    setAutoRead(newValue);
    try {
      await invoke("tts_set_queue_config", {
        config: { max_size: 50, auto_read: newValue },
      });
    } catch {
      setAutoRead(!newValue);
    }
  }, [autoRead]);

  return {
    items,
    current,
    playback,
    autoRead,
    addItem,
    remove,
    moveItem,
    clear,
    pause,
    resume,
    stop,
    toggleAutoRead,
  };
}
