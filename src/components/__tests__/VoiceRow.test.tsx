import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VoiceRow } from "../VoiceRow";
import type { VoiceEntry } from "../../types/piper";

const mockVoice: VoiceEntry = {
  key: "en_US-lessac-medium",
  name: "Lessac Medium",
  language: {
    code: "en-US",
    family: "en",
    region: "US",
    name_native: "English",
    name_english: "English",
    country_english: "United States",
  },
  quality: "medium",
  num_speakers: 1,
  speaker_id_map: {},
  files: {
    "en_US-lessac-medium.onnx": { size_bytes: 63000000, md5_digest: "abc" },
    "en_US-lessac-medium.onnx.json": { size_bytes: 1000, md5_digest: "def" },
  },
  aliases: [],
};

const multiSpeakerVoice: VoiceEntry = {
  ...mockVoice,
  key: "en_US-multi",
  name: "Multi Speaker",
  num_speakers: 3,
};

describe("VoiceRow", () => {
  it("renders voice name and quality", () => {
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={false}
        isDownloading={false}
        onDownload={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText("Lessac Medium")).toBeInTheDocument();
    expect(screen.getByText("medium")).toBeInTheDocument();
  });

  it("shows download button when not downloaded", () => {
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={false}
        isDownloading={false}
        onDownload={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText("Download")).toBeInTheDocument();
  });

  it("shows use button when downloaded", () => {
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={true}
        isDownloading={false}
        onDownload={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText("Use")).toBeInTheDocument();
  });

  it("calls onDownload when download button clicked", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={false}
        isDownloading={false}
        onDownload={onDownload}
        onSelect={vi.fn()}
      />
    );
    await user.click(screen.getByText("Download"));
    expect(onDownload).toHaveBeenCalledOnce();
  });

  it("calls onSelect when use button clicked", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={true}
        isDownloading={false}
        onDownload={vi.fn()}
        onSelect={onSelect}
      />
    );
    await user.click(screen.getByText("Use"));
    expect(onSelect).toHaveBeenCalledOnce();
  });

  it("shows downloading state", () => {
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={false}
        isDownloading={true}
        onDownload={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText("Downloading...")).toBeInTheDocument();
    expect(screen.getByText("Downloading...")).toBeDisabled();
  });

  it("shows speaker count when > 1", () => {
    render(
      <VoiceRow
        voice={multiSpeakerVoice}
        isDownloaded={false}
        isDownloading={false}
        onDownload={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.getByText("3 speakers")).toBeInTheDocument();
  });

  it("hides speaker count when 1", () => {
    render(
      <VoiceRow
        voice={mockVoice}
        isDownloaded={false}
        isDownloading={false}
        onDownload={vi.fn()}
        onSelect={vi.fn()}
      />
    );
    expect(screen.queryByText("speakers")).not.toBeInTheDocument();
  });
});
