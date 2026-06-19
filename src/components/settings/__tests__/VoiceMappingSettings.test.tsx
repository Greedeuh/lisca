import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { VoiceMappingSettings } from "../VoiceMappingSettings";
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

describe("VoiceMappingSettings", () => {
  it("shows empty state when no voices installed", () => {
    render(
      <VoiceMappingSettings
        voiceMapping={emptyMapping}
        installedVoices={[]}
        onSetLanguageVoice={vi.fn()}
        onSetFallback={vi.fn()}
      />,
    );
    expect(screen.getByText(/Install voices/)).toBeInTheDocument();
  });

  it("shows language dropdown per language", () => {
    render(
      <VoiceMappingSettings
        voiceMapping={emptyMapping}
        installedVoices={installed}
        onSetLanguageVoice={vi.fn()}
        onSetFallback={vi.fn()}
      />,
    );
    expect(screen.getByText("en")).toBeInTheDocument();
  });

  it("shows fallback dropdown", () => {
    render(
      <VoiceMappingSettings
        voiceMapping={emptyMapping}
        installedVoices={installed}
        onSetLanguageVoice={vi.fn()}
        onSetFallback={vi.fn()}
      />,
    );
    expect(screen.getByText("Fallback")).toBeInTheDocument();
  });

  it("calls onSetLanguageVoice when language dropdown changed", () => {
    const onSet = vi.fn();
    render(
      <VoiceMappingSettings
        voiceMapping={emptyMapping}
        installedVoices={installed}
        onSetLanguageVoice={onSet}
        onSetFallback={vi.fn()}
      />,
    );
    const selects = screen.getAllByRole("combobox");
    fireEvent.change(selects[0], { target: { value: "af_heart" } });
    expect(onSet).toHaveBeenCalledWith("en", "af_heart");
  });

  it("calls onSetFallback when fallback dropdown changed", () => {
    const onFallback = vi.fn();
    render(
      <VoiceMappingSettings
        voiceMapping={emptyMapping}
        installedVoices={installed}
        onSetLanguageVoice={vi.fn()}
        onSetFallback={onFallback}
      />,
    );
    const selects = screen.getAllByRole("combobox");
    fireEvent.change(selects[selects.length - 1], {
      target: { value: "en_US-amy-medium" },
    });
    expect(onFallback).toHaveBeenCalledWith("en_US-amy-medium");
  });

  it("reflects current mapping in dropdown value", () => {
    const mapping: VoiceMapping = {
      language_voice: { en: "af_heart" },
      fallback_voice_key: null,
    };
    render(
      <VoiceMappingSettings
        voiceMapping={mapping}
        installedVoices={installed}
        onSetLanguageVoice={vi.fn()}
        onSetFallback={vi.fn()}
      />,
    );
    const selects = screen.getAllByRole("combobox");
    expect(selects[0]).toHaveValue("af_heart");
  });
});
