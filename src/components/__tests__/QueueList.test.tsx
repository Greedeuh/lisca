import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueueList } from "../QueueList";
import type { QueueItem, PlaybackState } from "../../types/queue";

const mockItems: QueueItem[] = [
  { id: 1, text: "Hello world" },
  { id: 2, text: "Second item" },
  { id: 3, text: "Third item" },
];

describe("QueueList", () => {
  it("shows empty state", () => {
    render(
      <QueueList items={[]} current={null} playback="idle" onRemove={vi.fn()} onMove={vi.fn()} />
    );
    expect(screen.getByText(/Queue is empty/)).toBeInTheDocument();
  });

  it("renders queued items", () => {
    render(
      <QueueList items={mockItems} current={null} playback="idle" onRemove={vi.fn()} onMove={vi.fn()} />
    );
    expect(screen.getByText("Hello world")).toBeInTheDocument();
    expect(screen.getByText("Second item")).toBeInTheDocument();
    expect(screen.getByText("Third item")).toBeInTheDocument();
  });

  it("shows current item as playing", () => {
    render(
      <QueueList
        items={mockItems.slice(1)}
        current={mockItems[0]}
        playback="playing"
        onRemove={vi.fn()}
        onMove={vi.fn()}
      />
    );
    expect(screen.getByText("Playing")).toBeInTheDocument();
    expect(screen.getByText("Hello world")).toBeInTheDocument();
  });

  it("shows current item as paused", () => {
    render(
      <QueueList
        items={[]}
        current={mockItems[0]}
        playback="paused"
        onRemove={vi.fn()}
        onMove={vi.fn()}
      />
    );
    expect(screen.getByText("Paused")).toBeInTheDocument();
  });

  it("calls onRemove with Skip button for current item", async () => {
    const user = userEvent.setup();
    const onRemove = vi.fn();
    render(
      <QueueList
        items={[]}
        current={mockItems[0]}
        playback="playing"
        onRemove={onRemove}
        onMove={vi.fn()}
      />
    );
    await user.click(screen.getByText("Skip"));
    expect(onRemove).toHaveBeenCalledWith(1);
  });

  it("calls onRemove with Remove button for queued items", async () => {
    const user = userEvent.setup();
    const onRemove = vi.fn();
    render(
      <QueueList items={mockItems} current={null} playback="idle" onRemove={onRemove} onMove={vi.fn()} />
    );
    const removeButtons = screen.getAllByText("Remove");
    await user.click(removeButtons[0]);
    expect(onRemove).toHaveBeenCalledWith(1);
  });

  it("shows Up button for items after first", () => {
    render(
      <QueueList items={mockItems} current={null} playback="idle" onRemove={vi.fn()} onMove={vi.fn()} />
    );
    const upButtons = screen.getAllByText("Up");
    expect(upButtons).toHaveLength(2);
  });

  it("shows Down button for items before last", () => {
    render(
      <QueueList items={mockItems} current={null} playback="idle" onRemove={vi.fn()} onMove={vi.fn()} />
    );
    const downButtons = screen.getAllByText("Down");
    expect(downButtons).toHaveLength(2);
  });

  it("calls onMove when Up clicked", async () => {
    const user = userEvent.setup();
    const onMove = vi.fn();
    render(
      <QueueList items={mockItems} current={null} playback="idle" onRemove={vi.fn()} onMove={onMove} />
    );
    const upButtons = screen.getAllByText("Up");
    await user.click(upButtons[0]);
    expect(onMove).toHaveBeenCalledWith(2, 0);
  });

  it("shows item indices", () => {
    render(
      <QueueList items={mockItems} current={null} playback="idle" onRemove={vi.fn()} onMove={vi.fn()} />
    );
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });
});
