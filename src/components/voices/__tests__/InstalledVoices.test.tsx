import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ToastProvider } from "../../../contexts/toast";
import { InstalledVoices } from "../InstalledVoices";

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

const installedVoices = [
  {
    voice_key: "en_US-amy-medium",
    name: "Amy (English, US)",
    language: "en",
    quality: "medium",
    model_type: "piper" as const,
    model_path: "/tmp/test.onnx",
  },
  {
    voice_key: "af_heart",
    name: "Heart (American Female)",
    language: "en",
    quality: "high",
    model_type: "kokoro" as const,
    model_path: "/tmp/test.bin",
  },
];

const emptyMapping = {
  language_voice: {},
  fallback_voice_key: null,
};

const mappingWithActive = {
  language_voice: { en: "en_US-amy-medium" },
  fallback_voice_key: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === "list_installed_voices") return Promise.resolve(installedVoices);
    if (cmd === "get_voice_preference") return Promise.resolve(emptyMapping);
    return Promise.resolve(null);
  });
});

async function expandEn() {
  const btn = await screen.findByRole("button", { name: /en/ });
  fireEvent.click(btn);
}

describe("InstalledVoices", () => {
  it("shows empty state when no voices", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_installed_voices") return Promise.resolve([]);
      if (cmd === "get_voice_preference") return Promise.resolve(emptyMapping);
      return Promise.resolve(null);
    });
    renderWithToast(<InstalledVoices />);
    expect(await screen.findByText(/No voices installed/)).toBeInTheDocument();
  });

  it("renders installed voices grouped by language", async () => {
    renderWithToast(<InstalledVoices />);
    await expandEn();
    expect(await screen.findByText("Amy (English, US)")).toBeInTheDocument();
    expect(screen.getByText("Heart (American Female)")).toBeInTheDocument();
  });

  it("shows Set Active button for each voice", async () => {
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText("Amy (English, US)");
    const buttons = screen.getAllByText("Set Active");
    expect(buttons.length).toBe(2);
  });

  it("calls setVoicePreference when Set Active clicked", async () => {
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText("Amy (English, US)");
    const buttons = screen.getAllByText("Set Active");
    fireEvent.click(buttons[0]);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("set_voice_preference", {
        language: "en",
        voiceKey: "en_US-amy-medium",
      });
    });
  });

  it("shows Uninstall button for each voice", async () => {
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText("Amy (English, US)");
    const buttons = screen.getAllByText("Uninstall");
    expect(buttons.length).toBe(2);
  });

  it("calls uninstallVoice when Uninstall clicked", async () => {
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText("Amy (English, US)");
    const buttons = screen.getAllByText("Uninstall");
    fireEvent.click(buttons[0]);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("uninstall_voice", {
        voiceKey: "en_US-amy-medium",
      });
    });
  });

  it("shows active badge when voice is set active", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_installed_voices") return Promise.resolve(installedVoices);
      if (cmd === "get_voice_preference") return Promise.resolve(mappingWithActive);
      return Promise.resolve(null);
    });
    renderWithToast(<InstalledVoices />);
    expect(await screen.findByText(/Active: en_US-amy-medium/)).toBeInTheDocument();
  });

  it("hides Set Active button for active voice", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_installed_voices") return Promise.resolve(installedVoices);
      if (cmd === "get_voice_preference") return Promise.resolve(mappingWithActive);
      return Promise.resolve(null);
    });
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText(/Active: en_US-amy-medium/);
    const buttons = screen.getAllByText("Set Active");
    expect(buttons.length).toBe(1);
  });

  it("renders fallback voice dropdown", async () => {
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText("Amy (English, US)");
    expect(screen.getByText("Fallback voice:")).toBeInTheDocument();
  });

  it("calls setFallbackVoice when fallback changed", async () => {
    renderWithToast(<InstalledVoices />);
    expandEn();
    await screen.findByText("Amy (English, US)");
    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "af_heart" } });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("set_fallback_voice", {
        voiceKey: "af_heart",
      });
    });
  });
});
