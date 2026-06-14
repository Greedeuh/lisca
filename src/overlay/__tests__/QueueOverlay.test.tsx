import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { QueueItem, PlaybackState } from "../../types/queue";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("../../hooks/useTtsQueue", () => ({
  useTtsQueue: vi.fn(),
}));

import { QueueOverlay } from "../../overlay/QueueOverlay";
import { useTtsQueue } from "../../hooks/useTtsQueue";
const mockUseTtsQueue = vi.mocked(useTtsQueue);

function mockQueueState(overrides: Partial<ReturnType<typeof useTtsQueue>> = {}) {
  mockUseTtsQueue.mockReturnValue({
    items: [],
    current: null,
    playback: "idle" as PlaybackState,
    autoRead: true,
    showOverlay: true,
    remove: vi.fn(),
    moveItem: vi.fn(),
    clear: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
    stop: vi.fn(),
    toggleAutoRead: vi.fn(),
    toggleShowOverlay: vi.fn(),
    hideOverlay: vi.fn(),
    ...overrides,
  });
}

describe("QueueOverlay", () => {
  it("shows empty state when queue is empty", () => {
    mockQueueState();
    render(<QueueOverlay />);
    expect(screen.getByText("Queue empty")).toBeInTheDocument();
  });

  it("shows current item as playing", () => {
    const current: QueueItem = { id: 1, text: "Hello world" };
    mockQueueState({ current, playback: "playing" });
    render(<QueueOverlay />);
    expect(screen.getByText("Playing")).toBeInTheDocument();
    expect(screen.getByText("Hello world")).toBeInTheDocument();
  });

  it("shows current item as paused", () => {
    const current: QueueItem = { id: 1, text: "Hello world" };
    mockQueueState({ current, playback: "paused" });
    render(<QueueOverlay />);
    expect(screen.getByText("Paused")).toBeInTheDocument();
  });

  it("shows queued items", () => {
    const items: QueueItem[] = [
      { id: 2, text: "Second" },
      { id: 3, text: "Third" },
    ];
    mockQueueState({ items });
    render(<QueueOverlay />);
    expect(screen.getByText("Second")).toBeInTheDocument();
    expect(screen.getByText("Third")).toBeInTheDocument();
  });

  it("calls pause when pause button clicked during playback", async () => {
    const user = userEvent.setup();
    const pause = vi.fn();
    const current: QueueItem = { id: 1, text: "text" };
    mockQueueState({ current, playback: "playing", pause });
    render(<QueueOverlay />);
    await user.click(screen.getByText("⏸"));
    expect(pause).toHaveBeenCalledOnce();
  });

  it("calls resume when play button clicked while paused", async () => {
    const user = userEvent.setup();
    const resume = vi.fn();
    const current: QueueItem = { id: 1, text: "text" };
    mockQueueState({ current, playback: "paused", resume });
    render(<QueueOverlay />);
    await user.click(screen.getByText("▶"));
    expect(resume).toHaveBeenCalledOnce();
  });

  it("calls stop when skip button clicked", async () => {
    const user = userEvent.setup();
    const stop = vi.fn();
    const current: QueueItem = { id: 1, text: "text" };
    mockQueueState({ current, playback: "playing", stop });
    render(<QueueOverlay />);
    await user.click(screen.getByText("⏭"));
    expect(stop).toHaveBeenCalledOnce();
  });

  it("calls remove when remove button clicked on queued item", async () => {
    const user = userEvent.setup();
    const remove = vi.fn();
    const items: QueueItem[] = [{ id: 2, text: "Second" }];
    mockQueueState({ items, remove });
    render(<QueueOverlay />);
    const removeButtons = screen.getAllByText("✕");
    await user.click(removeButtons[1]);
    expect(remove).toHaveBeenCalledWith(2);
  });

  it("calls clear when Clear button clicked", async () => {
    const user = userEvent.setup();
    const clear = vi.fn();
    const items: QueueItem[] = [{ id: 2, text: "Second" }];
    mockQueueState({ items, clear });
    render(<QueueOverlay />);
    await user.click(screen.getByText("Clear"));
    expect(clear).toHaveBeenCalledOnce();
  });

  it("calls toggleAutoRead when Auto checkbox toggled", async () => {
    const user = userEvent.setup();
    const toggleAutoRead = vi.fn();
    mockQueueState({ toggleAutoRead });
    render(<QueueOverlay />);
    await user.click(screen.getByRole("checkbox"));
    expect(toggleAutoRead).toHaveBeenCalledOnce();
  });

  it("calls hideOverlay when close button clicked", async () => {
    const user = userEvent.setup();
    const hideOverlay = vi.fn();
    mockQueueState({ hideOverlay });
    render(<QueueOverlay />);
    await user.click(screen.getByText("✕"));
    expect(hideOverlay).toHaveBeenCalledOnce();
  });

  it("hides footer when queue is empty", () => {
    mockQueueState();
    render(<QueueOverlay />);
    expect(screen.queryByText("Clear")).not.toBeInTheDocument();
  });

  it("shows footer when queue has items", () => {
    const items: QueueItem[] = [{ id: 2, text: "Second" }];
    mockQueueState({ items });
    render(<QueueOverlay />);
    expect(screen.getByText("Clear")).toBeInTheDocument();
  });
});
