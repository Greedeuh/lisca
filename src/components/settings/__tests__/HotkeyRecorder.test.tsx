import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { HotkeyRecorder } from "../HotkeyRecorder";
import type { ShortcutConfig } from "../../../types/hotkey";

const hotkey: ShortcutConfig = {
  modifiers: ["Control", "Shift"],
  key: "K",
};

describe("HotkeyRecorder", () => {
  it("displays current hotkey", () => {
    render(<HotkeyRecorder currentHotkey={hotkey} onSave={vi.fn()} />);
    expect(screen.getByText("Control+Shift+K")).toBeInTheDocument();
  });

  it("displays Not set when no hotkey", () => {
    render(<HotkeyRecorder currentHotkey={null} onSave={vi.fn()} />);
    expect(screen.getByText("Not set")).toBeInTheDocument();
  });

  it("shows Record button", () => {
    render(<HotkeyRecorder currentHotkey={null} onSave={vi.fn()} />);
    expect(screen.getByText("Record")).toBeInTheDocument();
  });

  it("shows recording state when Record clicked", () => {
    render(<HotkeyRecorder currentHotkey={null} onSave={vi.fn()} />);
    fireEvent.click(screen.getByText("Record"));
    expect(screen.getByText("Press keys...")).toBeInTheDocument();
  });

  it("shows hint when recording", () => {
    render(<HotkeyRecorder currentHotkey={null} onSave={vi.fn()} />);
    fireEvent.click(screen.getByText("Record"));
    expect(screen.getByText(/Press a key combination/)).toBeInTheDocument();
  });

  it("updates display when hotkey prop changes", () => {
    const { rerender } = render(
      <HotkeyRecorder currentHotkey={null} onSave={vi.fn()} />,
    );
    expect(screen.getByText("Not set")).toBeInTheDocument();
    rerender(
      <HotkeyRecorder currentHotkey={hotkey} onSave={vi.fn()} />,
    );
    expect(screen.getByText("Control+Shift+K")).toBeInTheDocument();
  });
});
