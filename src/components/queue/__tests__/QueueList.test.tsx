import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { QueueListView } from "../QueueListView";
import type { QueueItem } from "../../../types/queue";

const textMsg = (id: number, text: string, status: "pending" | "processing" = "pending"): QueueItem => ({
  type: "TextMessage",
  id,
  text,
  language: null,
  status,
});

const speech = (
  id: number,
  text: string,
  status: "to_play" | "playing" | "paused" | "played" = "to_play",
): QueueItem => ({
  type: "Speech",
  id,
  text,
  voice_key: null,
  language: null,
  status,
});

const defaults = {
  autoRead: true,
  onRemove: vi.fn(),
  onMove: vi.fn(),
  onToggleAutoRead: vi.fn(),
  onClear: vi.fn(),
};

function renderList(items: QueueItem[], overrides: Partial<typeof defaults> = {}) {
  const props = { ...defaults, ...overrides };
  return render(
    <QueueListView
      items={items}
      autoRead={props.autoRead}
      onRemove={props.onRemove}
      onMove={props.onMove}
      onToggleAutoRead={props.onToggleAutoRead}
      onClear={props.onClear}
    />,
  );
}

describe("QueueListView", () => {
  it("shows empty state", () => {
    renderList([]);
    expect(screen.getByText(/Queue is empty/)).toBeInTheDocument();
  });

  it("renders TextMessage items with text preview", () => {
    renderList([textMsg(1, "Hello world"), textMsg(2, "Second item")]);
    expect(screen.getByText("Hello world")).toBeInTheDocument();
    expect(screen.getByText("Second item")).toBeInTheDocument();
  });

  it("renders Speech items with text preview", () => {
    renderList([speech(1, "Spoken text")]);
    expect(screen.getByText("Spoken text")).toBeInTheDocument();
  });

  it("shows Pending badge for pending TextMessage", () => {
    renderList([textMsg(1, "text", "pending")]);
    expect(screen.getByText("Pending")).toBeInTheDocument();
  });

  it("shows Processing badge for processing TextMessage", () => {
    renderList([textMsg(1, "text", "processing")]);
    expect(screen.getByText("Processing")).toBeInTheDocument();
  });

  it("shows Queued badge for to_play Speech", () => {
    renderList([speech(1, "text", "to_play")]);
    expect(screen.getByText("Queued")).toBeInTheDocument();
  });

  it("shows Playing badge for playing Speech", () => {
    renderList([speech(1, "text", "playing")]);
    expect(screen.getByText("Playing")).toBeInTheDocument();
  });

  it("shows Paused badge for paused Speech", () => {
    renderList([speech(1, "text", "paused")]);
    expect(screen.getByText("Paused")).toBeInTheDocument();
  });

  it("shows Done badge for played Speech", () => {
    renderList([speech(1, "text", "played")]);
    expect(screen.getByText("Done")).toBeInTheDocument();
  });

  it("highlights active playing item", () => {
    const { container } = renderList([
      speech(1, "first", "to_play"),
      speech(2, "playing", "playing"),
      speech(3, "third", "to_play"),
    ]);
    const items = container.querySelectorAll(".ql-item");
    expect(items[0]).not.toHaveClass("ql-item-active");
    expect(items[1]).toHaveClass("ql-item-active");
    expect(items[2]).not.toHaveClass("ql-item-active");
  });

  it("calls onRemove when remove button clicked for TextMessage", () => {
    const onRemove = vi.fn();
    renderList([textMsg(1, "text")], { onRemove });
    fireEvent.click(screen.getByLabelText("Remove"));
    expect(onRemove).toHaveBeenCalledWith(1);
  });

  it("calls onRemove when skip button clicked for playing Speech", () => {
    const onRemove = vi.fn();
    renderList([speech(1, "text", "playing")], { onRemove });
    fireEvent.click(screen.getByLabelText("Skip"));
    expect(onRemove).toHaveBeenCalledWith(1);
  });

  it("calls onRemove when remove button clicked for to_play Speech", () => {
    const onRemove = vi.fn();
    renderList([speech(1, "text", "to_play")], { onRemove });
    fireEvent.click(screen.getByLabelText("Remove"));
    expect(onRemove).toHaveBeenCalledWith(1);
  });

  it("calls onMove when up button clicked", () => {
    const onMove = vi.fn();
    renderList(
      [speech(1, "first"), speech(2, "second"), speech(3, "third")],
      { onMove },
    );
    const upButtons = screen.getAllByLabelText("Move up");
    fireEvent.click(upButtons[1]);
    expect(onMove).toHaveBeenCalledWith(3, 1);
  });

  it("calls onMove when down button clicked", () => {
    const onMove = vi.fn();
    renderList(
      [speech(1, "first"), speech(2, "second"), speech(3, "third")],
      { onMove },
    );
    const downButtons = screen.getAllByLabelText("Move down");
    fireEvent.click(downButtons[0]);
    expect(onMove).toHaveBeenCalledWith(1, 1);
  });

  it("does not show up button for first item", () => {
    renderList([textMsg(1, "first"), textMsg(2, "second")]);
    expect(screen.queryByLabelText("Move up")).not.toBeInTheDocument();
  });

  it("does not show down button for last item", () => {
    renderList([textMsg(1, "first"), textMsg(2, "second")]);
    expect(screen.queryByLabelText("Move down")).not.toBeInTheDocument();
  });

  it("calls onToggleAutoRead when checkbox toggled", () => {
    const onToggleAutoRead = vi.fn();
    renderList([textMsg(1, "text")], { onToggleAutoRead });
    fireEvent.click(screen.getByRole("checkbox"));
    expect(onToggleAutoRead).toHaveBeenCalled();
  });

  it("reflects autoRead prop in checkbox", () => {
    const { rerender } = render(
      <QueueListView
        items={[textMsg(1, "text")]}
        autoRead={true}
        onRemove={vi.fn()}
        onMove={vi.fn()}
        onToggleAutoRead={vi.fn()}
        onClear={vi.fn()}
      />,
    );
    expect(screen.getByRole("checkbox")).toBeChecked();
    rerender(
      <QueueListView
        items={[textMsg(1, "text")]}
        autoRead={false}
        onRemove={vi.fn()}
        onMove={vi.fn()}
        onToggleAutoRead={vi.fn()}
        onClear={vi.fn()}
      />,
    );
    expect(screen.getByRole("checkbox")).not.toBeChecked();
  });

  it("calls onClear when clear button clicked", () => {
    const onClear = vi.fn();
    renderList([textMsg(1, "text")], { onClear });
    fireEvent.click(screen.getByText("Clear"));
    expect(onClear).toHaveBeenCalled();
  });

  it("truncates long text", () => {
    const longText = "a".repeat(100);
    renderList([textMsg(1, longText)]);
    expect(screen.getByText(longText.slice(0, 80) + "…")).toBeInTheDocument();
  });

  it("shows language badge when language is set", () => {
    const item: QueueItem = { ...textMsg(1, "text"), language: "en" };
    renderList([item]);
    expect(screen.getByText("en")).toBeInTheDocument();
  });

  it("does not show language badge when language is null", () => {
    renderList([textMsg(1, "text")]);
    expect(screen.queryByText("en")).not.toBeInTheDocument();
  });

  it("renders mixed TextMessage and Speech items", () => {
    renderList([
      textMsg(1, "pending text"),
      speech(2, "ready speech", "to_play"),
      textMsg(3, "another text"),
    ]);
    expect(screen.getByText("pending text")).toBeInTheDocument();
    expect(screen.getByText("ready speech")).toBeInTheDocument();
    expect(screen.getByText("another text")).toBeInTheDocument();
  });
});
