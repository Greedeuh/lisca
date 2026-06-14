import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueueControls } from "../QueueControls";
import type { PlaybackState } from "../../types/queue";

const defaultProps = {
  playback: "idle" as PlaybackState,
  autoRead: true,
  showOverlay: true,
  onToggleAutoRead: vi.fn(),
  onToggleShowOverlay: vi.fn(),
  onPause: vi.fn(),
  onResume: vi.fn(),
  onStop: vi.fn(),
  onClear: vi.fn(),
  disabled: false,
};

describe("QueueControls", () => {
  it("shows Play when idle", () => {
    render(<QueueControls {...defaultProps} playback="idle" />);
    expect(screen.getByText("Play")).toBeInTheDocument();
  });

  it("shows Pause when playing", () => {
    render(<QueueControls {...defaultProps} playback="playing" />);
    expect(screen.getByText("Pause")).toBeInTheDocument();
  });

  it("shows Play when paused", () => {
    render(<QueueControls {...defaultProps} playback="paused" />);
    expect(screen.getByText("Play")).toBeInTheDocument();
  });

  it("calls onPause when Pause clicked", async () => {
    const user = userEvent.setup();
    const onPause = vi.fn();
    render(<QueueControls {...defaultProps} playback="playing" onPause={onPause} />);
    await user.click(screen.getByText("Pause"));
    expect(onPause).toHaveBeenCalledOnce();
  });

  it("calls onResume when Play clicked", async () => {
    const user = userEvent.setup();
    const onResume = vi.fn();
    render(<QueueControls {...defaultProps} playback="idle" onResume={onResume} />);
    await user.click(screen.getByText("Play"));
    expect(onResume).toHaveBeenCalledOnce();
  });

  it("calls onStop when Stop clicked", async () => {
    const user = userEvent.setup();
    const onStop = vi.fn();
    render(<QueueControls {...defaultProps} onStop={onStop} />);
    await user.click(screen.getByText("Stop"));
    expect(onStop).toHaveBeenCalledOnce();
  });

  it("calls onClear when Clear clicked", async () => {
    const user = userEvent.setup();
    const onClear = vi.fn();
    render(<QueueControls {...defaultProps} onClear={onClear} />);
    await user.click(screen.getByText("Clear"));
    expect(onClear).toHaveBeenCalledOnce();
  });

  it("disables buttons when disabled prop is true", () => {
    render(<QueueControls {...defaultProps} disabled={true} />);
    expect(screen.getByText("Play")).toBeDisabled();
    expect(screen.getByText("Stop")).toBeDisabled();
    expect(screen.getByText("Clear")).toBeDisabled();
  });

  it("reflects auto-read checkbox", () => {
    render(<QueueControls {...defaultProps} autoRead={false} />);
    const checkbox = screen.getByRole("checkbox", { name: /auto-read/i });
    expect(checkbox).not.toBeChecked();
  });

  it("reflects show overlay checkbox", () => {
    render(<QueueControls {...defaultProps} showOverlay={false} />);
    const checkbox = screen.getByRole("checkbox", { name: /show overlay/i });
    expect(checkbox).not.toBeChecked();
  });

  it("calls onToggleAutoRead when checkbox toggled", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<QueueControls {...defaultProps} onToggleAutoRead={onToggle} />);
    await user.click(screen.getByRole("checkbox", { name: /auto-read/i }));
    expect(onToggle).toHaveBeenCalledOnce();
  });
});
