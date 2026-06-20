import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ToastProvider } from "../../../contexts/toast";
import { VoiceBrowser } from "../VoiceBrowser";

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

const catalogVoices = [
  {
    voice_key: "en_US-amy-medium",
    name: "Amy (English, US)",
    language: "en",
    quality: "medium",
    size_bytes: 52_000_000,
    speed: "1.0x",
    model_type: "piper" as const,
  },
  {
    voice_key: "af_heart",
    name: "Heart (American Female)",
    language: "en",
    quality: "high",
    size_bytes: 15_000_000,
    speed: "1.0x",
    model_type: "kokoro" as const,
  },
];

const installedVoices = [
  {
    voice_key: "en_US-amy-medium",
    name: "Amy (English, US)",
    language: "en",
    quality: "medium",
    model_type: "piper" as const,
    model_path: "/tmp/test.onnx",
  },
];

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === "list_catalog_voices") return Promise.resolve(catalogVoices);
    if (cmd === "list_installed_voices") return Promise.resolve(installedVoices);
    return Promise.resolve(null);
  });
});

describe("VoiceBrowser", () => {
  it("renders voices grouped by language", async () => {
    renderWithToast(<VoiceBrowser />);
    expect(await screen.findByText("Amy (English, US)")).toBeInTheDocument();
    expect(screen.getByText("Heart (American Female)")).toBeInTheDocument();
    expect(screen.getByText("en")).toBeInTheDocument();
  });

  it("shows install buttons for uninstalled voices", async () => {
    renderWithToast(<VoiceBrowser />);
    await screen.findByText("Amy (English, US)");
    const buttons = screen.getAllByText("Install");
    expect(buttons.length).toBe(1);
  });

  it("calls installVoice when install clicked", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_catalog_voices") return Promise.resolve(catalogVoices);
      if (cmd === "list_installed_voices") return Promise.resolve([]);
      if (cmd === "install_voice") return Promise.resolve({});
      return Promise.resolve(null);
    });
    renderWithToast(<VoiceBrowser />);
    await screen.findByText("Amy (English, US)");
    const buttons = screen.getAllByText("Install");
    fireEvent.click(buttons[0]);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("install_voice", { voiceKey: "en_US-amy-medium" });
    });
  });

  it("shows Installed label for installed voice", async () => {
    renderWithToast(<VoiceBrowser />);
    await screen.findByText("Amy (English, US)");
    expect(screen.getByText("Installed")).toBeInTheDocument();
    const installButtons = screen.getAllByText("Install");
    expect(installButtons.length).toBe(1);
  });

  it("shows quality badge", async () => {
    renderWithToast(<VoiceBrowser />);
    await screen.findByText("Amy (English, US)");
    expect(screen.getByText("medium")).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
  });

  it("shows model type badge", async () => {
    renderWithToast(<VoiceBrowser />);
    await screen.findByText("Amy (English, US)");
    expect(screen.getAllByText("piper").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("kokoro").length).toBeGreaterThanOrEqual(1);
  });

  it("shows empty state when no voices", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_catalog_voices") return Promise.resolve([]);
      if (cmd === "list_installed_voices") return Promise.resolve([]);
      return Promise.resolve(null);
    });
    renderWithToast(<VoiceBrowser />);
    expect(await screen.findByText("No voices available.")).toBeInTheDocument();
  });
});
