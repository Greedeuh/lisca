import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ToastProvider } from "../../../contexts/toast";
import { HotkeyRecorder } from "../HotkeyRecorder";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

function renderWithToast(ui: React.ReactElement) {
  return render(<ToastProvider>{ui}</ToastProvider>);
}

const hotkey = {
  modifiers: ["Control", "Shift"],
  key: "K",
};

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === "get_hotkey") return Promise.resolve(hotkey);
    return Promise.resolve(null);
  });
});

describe("HotkeyRecorder", () => {
  it("displays current hotkey", async () => {
    renderWithToast(<HotkeyRecorder />);
    expect(await screen.findByText("Control+Shift+K")).toBeInTheDocument();
  });

  it("displays Not set when no hotkey", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_hotkey") return Promise.resolve(null);
      return Promise.resolve(null);
    });
    renderWithToast(<HotkeyRecorder />);
    expect(await screen.findByText("Not set")).toBeInTheDocument();
  });

  it("shows Record button", async () => {
    renderWithToast(<HotkeyRecorder />);
    await screen.findByText("Record");
    expect(screen.getByText("Record")).toBeInTheDocument();
  });

  it("shows recording state when Record clicked", async () => {
    renderWithToast(<HotkeyRecorder />);
    await screen.findByText("Record");
    fireEvent.click(screen.getByText("Record"));
    await waitFor(() => {
      expect(screen.getByText("Press keys...")).toBeInTheDocument();
    });
  });

  it("shows hint when recording", async () => {
    renderWithToast(<HotkeyRecorder />);
    await screen.findByText("Record");
    fireEvent.click(screen.getByText("Record"));
    await waitFor(() => {
      expect(screen.getByText(/Press a key combination/)).toBeInTheDocument();
    });
  });
});
