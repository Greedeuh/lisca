import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { InstalledVoices } from "../InstalledVoices";
import type { InstalledVoice } from "../../../types/voice-catalog";
import type { VoiceMapping } from "../../../types/voice-prefs";

const installed: InstalledVoice[] = [
  {
    voice_key: "en_US-amy-medium",
    name: "Amy (English, US)",
    language: "en",
    quality: "medium",
    model_type: "piper",
    model_path: "/tmp/test.onnx",
  },
  {
    voice_key: "af_heart",
    name: "Heart (American Female)",
    language: "en",
    quality: "high",
    model_type: "kokoro",
    model_path: "/tmp/test.bin",
  },
];

const emptyMapping: VoiceMapping = {
  language_voice: {},
  fallback_voice_key: null,
};

const defaults = {
  voiceMapping: emptyMapping,
  onUninstall: vi.fn(),
  onSetActive: vi.fn(),
  onSetFallback: vi.fn(),
};

function renderInstalled(overrides: Partial<typeof defaults> = {}) {
  const props = { ...defaults, ...overrides };
  return render(
    <InstalledVoices
      voices={installed}
      voiceMapping={props.voiceMapping}
      onUninstall={props.onUninstall}
      onSetActive={props.onSetActive}
      onSetFallback={props.onSetFallback}
    />,
  );
}

describe("InstalledVoices", () => {
  it("shows empty state when no voices", () => {
    render(
      <InstalledVoices
        voices={[]}
        voiceMapping={emptyMapping}
        onUninstall={vi.fn()}
        onSetActive={vi.fn()}
        onSetFallback={vi.fn()}
      />,
    );
    expect(screen.getByText(/No voices installed/)).toBeInTheDocument();
  });

  it("renders installed voices grouped by language", () => {
    renderInstalled();
    expect(screen.getByText("Amy (English, US)")).toBeInTheDocument();
    expect(screen.getByText("Heart (American Female)")).toBeInTheDocument();
    expect(screen.getByText("en")).toBeInTheDocument();
  });

  it("shows Set Active button for each voice", () => {
    renderInstalled();
    const buttons = screen.getAllByText("Set Active");
    expect(buttons.length).toBe(2);
  });

  it("calls onSetActive when Set Active clicked", () => {
    const onSetActive = vi.fn();
    renderInstalled({ onSetActive });
    const buttons = screen.getAllByText("Set Active");
    fireEvent.click(buttons[0]);
    expect(onSetActive).toHaveBeenCalledWith("en", "en_US-amy-medium");
  });

  it("shows Uninstall button for each voice", () => {
    renderInstalled();
    const buttons = screen.getAllByText("Uninstall");
    expect(buttons.length).toBe(2);
  });

  it("calls onUninstall when Uninstall clicked", () => {
    const onUninstall = vi.fn();
    renderInstalled({ onUninstall });
    const buttons = screen.getAllByText("Uninstall");
    fireEvent.click(buttons[0]);
    expect(onUninstall).toHaveBeenCalledWith("en_US-amy-medium");
  });

  it("shows active badge when voice is set active", () => {
    const mapping: VoiceMapping = {
      language_voice: { en: "en_US-amy-medium" },
      fallback_voice_key: null,
    };
    renderInstalled({ voiceMapping: mapping });
    expect(screen.getByText(/Active: en_US-amy-medium/)).toBeInTheDocument();
  });

  it("hides Set Active button for active voice", () => {
    const mapping: VoiceMapping = {
      language_voice: { en: "en_US-amy-medium" },
      fallback_voice_key: null,
    };
    renderInstalled({ voiceMapping: mapping });
    const buttons = screen.getAllByText("Set Active");
    expect(buttons.length).toBe(1);
  });

  it("renders fallback voice dropdown", () => {
    renderInstalled();
    expect(screen.getByText("Fallback voice:")).toBeInTheDocument();
  });

  it("calls onSetFallback when fallback changed", () => {
    const onSetFallback = vi.fn();
    renderInstalled({ onSetFallback });
    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "af_heart" } });
    expect(onSetFallback).toHaveBeenCalledWith("af_heart");
  });
});
