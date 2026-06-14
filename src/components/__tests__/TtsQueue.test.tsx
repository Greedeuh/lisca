import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TtsQueue } from "../TtsQueue";

vi.mock("../../hooks/useTtsQueue", () => ({
  useTtsQueue: vi.fn(),
}));

import { useTtsQueue } from "../../hooks/useTtsQueue";
const mockUseTtsQueue = vi.mocked(useTtsQueue);

function mockQueueState(overrides: Partial<ReturnType<typeof useTtsQueue>> = {}) {
  mockUseTtsQueue.mockReturnValue({
    items: [],
    current: null,
    playback: "idle",
    autoRead: true,
    showOverlay: true,
    add: vi.fn(),
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

describe("TtsQueue", () => {
  it("renders queue section", () => {
    mockQueueState();
    render(<TtsQueue />);
    expect(screen.getByText("Queue")).toBeInTheDocument();
  });

  it("shows empty state", () => {
    mockQueueState();
    render(<TtsQueue />);
    expect(screen.getByText(/Queue is empty/)).toBeInTheDocument();
  });

  it("shows controls disabled when queue empty and nothing playing", () => {
    mockQueueState();
    render(<TtsQueue />);
    expect(screen.getByText("Play")).toBeDisabled();
    expect(screen.getByText("Stop")).toBeDisabled();
    expect(screen.getByText("Clear")).toBeDisabled();
  });

  it("shows controls enabled when queue has items", () => {
    mockQueueState({
      items: [{ id: 1, text: "hello" }],
    });
    render(<TtsQueue />);
    expect(screen.getByText("Play")).not.toBeDisabled();
  });

  it("shows pause button when playing", () => {
    mockQueueState({
      current: { id: 1, text: "hello" },
      playback: "playing",
    });
    render(<TtsQueue />);
    expect(screen.getByText("Pause")).toBeInTheDocument();
  });

  it("renders queue items", () => {
    mockQueueState({
      items: [
        { id: 1, text: "First" },
        { id: 2, text: "Second" },
      ],
    });
    render(<TtsQueue />);
    expect(screen.getByText("First")).toBeInTheDocument();
    expect(screen.getByText("Second")).toBeInTheDocument();
  });
});
