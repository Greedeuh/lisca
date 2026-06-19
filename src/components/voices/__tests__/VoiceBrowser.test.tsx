import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { VoiceBrowser } from "../VoiceBrowser";
import type { VoiceEntry } from "../../../types/voice-catalog";

const piperVoice: VoiceEntry = {
  voice_key: "en_US-amy-medium",
  name: "Amy (English, US)",
  language: "en",
  quality: "medium",
  size_bytes: 52_000_000,
  speed: "1.0x",
  model_type: "piper",
};

const kokoroVoice: VoiceEntry = {
  voice_key: "af_heart",
  name: "Heart (American Female)",
  language: "en",
  quality: "high",
  size_bytes: 15_000_000,
  speed: "1.0x",
  model_type: "kokoro",
};

const defaults = {
  installedKeys: new Set<string>(),
  downloading: new Map(),
  onInstall: vi.fn(),
};

function renderBrowser(overrides: Partial<typeof defaults> = {}) {
  const props = { ...defaults, ...overrides };
  return render(
    <VoiceBrowser
      voices={[piperVoice, kokoroVoice]}
      installedKeys={props.installedKeys}
      downloading={props.downloading}
      onInstall={props.onInstall}
    />,
  );
}

describe("VoiceBrowser", () => {
  it("renders voices grouped by language", () => {
    renderBrowser();
    expect(screen.getByText("en")).toBeInTheDocument();
    expect(screen.getByText("Amy (English, US)")).toBeInTheDocument();
    expect(screen.getByText("Heart (American Female)")).toBeInTheDocument();
  });

  it("shows install buttons for uninstalled voices", () => {
    renderBrowser();
    const buttons = screen.getAllByText("Install");
    expect(buttons.length).toBe(2);
  });

  it("calls onInstall with voice key when first install clicked", () => {
    const onInstall = vi.fn();
    renderBrowser({ onInstall });
    const buttons = screen.getAllByText("Install");
    fireEvent.click(buttons[0]);
    expect(onInstall).toHaveBeenCalledWith("en_US-amy-medium");
  });

  it("shows Installed label and hides Install button for installed voice", () => {
    renderBrowser({ installedKeys: new Set(["en_US-amy-medium"]) });
    expect(screen.getByText("Installed")).toBeInTheDocument();
    const installButtons = screen.getAllByText("Install");
    expect(installButtons.length).toBe(1);
  });

  it("shows quality badge", () => {
    renderBrowser();
    expect(screen.getByText("medium")).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
  });

  it("shows model type badge", () => {
    renderBrowser();
    expect(screen.getAllByText("piper").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("kokoro").length).toBeGreaterThanOrEqual(1);
  });

  it("shows empty state when no voices", () => {
    render(
      <VoiceBrowser
        voices={[]}
        installedKeys={new Set()}
        downloading={new Map()}
        onInstall={vi.fn()}
      />,
    );
    expect(screen.getByText("No voices available.")).toBeInTheDocument();
  });

  it("shows download progress percentage", () => {
    const downloading = new Map([
      [
        "en_US-amy-medium",
        {
          type: "downloading" as const,
          voice_key: "en_US-amy-medium",
          bytes_downloaded: 26_000_000,
          total_bytes: 52_000_000,
        },
      ],
    ]);
    renderBrowser({ downloading });
    expect(screen.getByText("50%")).toBeInTheDocument();
  });
});
