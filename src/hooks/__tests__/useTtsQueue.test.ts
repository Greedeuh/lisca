import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useTtsQueue } from "../useTtsQueue";
import type { QueueSnapshot, QueueEvent } from "../../types/queue";

const mockSnapshot: QueueSnapshot = {
  items: [{ id: 1, text: "queued", language: "en" }],
  playback: "idle",
  auto_read: true,
  show_overlay: true,
};

let listenCb: ((event: { payload: QueueEvent }) => void) | null = null;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((_channel: string, cb: any) => {
    listenCb = cb;
    return Promise.resolve(vi.fn());
  }),
}));

function mockInvoke(fn: (cmd: string, args?: any) => any) {
  (window as any).__TAURI_INTERNALS__.invoke = vi.fn(fn);
}

describe("useTtsQueue", () => {
  beforeEach(() => {
    listenCb = null;
    mockInvoke((cmd: string) => {
      if (cmd === "tts_queue_state") return Promise.resolve(mockSnapshot);
      return Promise.resolve();
    });
  });

  it("loads queue state on mount", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => {
      expect(result.current.items).toHaveLength(1);
    });
    expect(result.current.playback).toBe("idle");
    expect(result.current.autoRead).toBe(true);
  });

  it("remove calls invoke", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    await act(async () => {
      await result.current.remove(1);
    });

    const invoke = (window as any).__TAURI_INTERNALS__.invoke;
    expect(invoke).toHaveBeenCalledWith("tts_queue_remove", { id: 1 }, undefined);
  });

  it("handles playback_started event", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    const item = { id: 1, text: "queued", language: "en" };
    act(() => {
      listenCb!({ payload: { type: "playback_started", item } as QueueEvent });
    });

    expect(result.current.current).toEqual(item);
    expect(result.current.playback).toBe("playing");
  });

  it("handles playback_stopped event", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    act(() => {
      listenCb!({ payload: { type: "playback_started", item: result.current.items[0] } as QueueEvent });
    });
    expect(result.current.playback).toBe("playing");

    act(() => {
      listenCb!({ payload: { type: "playback_stopped" } as QueueEvent });
    });
    expect(result.current.current).toBeNull();
    expect(result.current.playback).toBe("idle");
  });

  it("handles processor_idle event", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    act(() => {
      listenCb!({ payload: { type: "processor_idle" } as QueueEvent });
    });

    expect(result.current.current).toBeNull();
    expect(result.current.playback).toBe("idle");
  });

  it("toggleAutoRead flips and calls invoke", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.autoRead).toBe(true));

    await act(async () => {
      await result.current.toggleAutoRead();
    });

    expect(result.current.autoRead).toBe(false);
    const invoke = (window as any).__TAURI_INTERNALS__.invoke;
    expect(invoke).toHaveBeenCalledWith(
      "tts_set_queue_config",
      { config: { max_items: 50, auto_read: false, show_overlay: true } },
      undefined
    );
  });

  it("handles playback_paused event", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    act(() => {
      listenCb!({ payload: { type: "playback_started", item: result.current.items[0] } as QueueEvent });
    });
    act(() => {
      listenCb!({ payload: { type: "playback_paused" } as QueueEvent });
    });

    expect(result.current.playback).toBe("paused");
  });

  it("handles queue_updated event", async () => {
    const { result } = renderHook(() => useTtsQueue());
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    act(() => {
      listenCb!({
        payload: {
          type: "queue_updated",
          items: [{ id: 5, text: "new", language: null }],
          auto_read: false,
          show_overlay: false,
        } as QueueEvent,
      });
    });

    expect(result.current.items).toHaveLength(1);
    expect(result.current.items[0].id).toBe(5);
    expect(result.current.autoRead).toBe(false);
    expect(result.current.showOverlay).toBe(false);
  });
});
