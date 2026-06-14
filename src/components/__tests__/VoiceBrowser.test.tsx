import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VoiceBrowser } from "../VoiceBrowser";
import type { VoiceCatalog } from "../../types/piper";

const mockCatalog: VoiceCatalog = {
  "en_US-lessac-medium": {
    key: "en_US-lessac-medium",
    name: "Lessac Medium",
    language: { code: "en-US", family: "en", region: "US", name_native: "English", name_english: "English", country_english: "United States" },
    quality: "medium",
    num_speakers: 1,
    speaker_id_map: {},
    files: { "en_US-lessac-medium.onnx": { size_bytes: 63000000, md5_digest: "abc" } },
    aliases: [],
  },
  "en_US-lessac-low": {
    key: "en_US-lessac-low",
    name: "Lessac Low",
    language: { code: "en-US", family: "en", region: "US", name_native: "English", name_english: "English", country_english: "United States" },
    quality: "low",
    num_speakers: 1,
    speaker_id_map: {},
    files: { "en_US-lessac-low.onnx": { size_bytes: 30000000, md5_digest: "def" } },
    aliases: [],
  },
  "fr_FR-siwis-medium": {
    key: "fr_FR-siwis-medium",
    name: "Siwis Medium",
    language: { code: "fr-FR", family: "fr", region: "FR", name_native: "Français", name_english: "French", country_english: "France" },
    quality: "medium",
    num_speakers: 1,
    speaker_id_map: {},
    files: { "fr_FR-siwis-medium.onnx": { size_bytes: 40000000, md5_digest: "ghi" } },
    aliases: [],
  },
};

describe("VoiceBrowser", () => {
  const defaultProps = {
    catalog: mockCatalog,
    downloadedVoices: new Set<string>(),
    downloadingVoice: null,
    onDownload: vi.fn(),
    onSelect: vi.fn(),
  };

  it("renders voice groups by family", () => {
    render(<VoiceBrowser {...defaultProps} />);
    expect(screen.getByText("English")).toBeInTheDocument();
    expect(screen.getByText("(en)")).toBeInTheDocument();
    expect(screen.getByText("French")).toBeInTheDocument();
    expect(screen.getByText("(fr)")).toBeInTheDocument();
  });

  it("search filters voices by name", async () => {
    const user = userEvent.setup();
    render(<VoiceBrowser {...defaultProps} />);
    await user.type(screen.getByPlaceholderText("Search voices..."), "siwis");
    expect(screen.getByText("Siwis Medium")).toBeInTheDocument();
    expect(screen.queryByText("Lessac Medium")).not.toBeInTheDocument();
  });

  it("search filters voices by locale code", async () => {
    const user = userEvent.setup();
    render(<VoiceBrowser {...defaultProps} />);
    await user.type(screen.getByPlaceholderText("Search voices..."), "fr-FR");
    expect(screen.getByText("Siwis Medium")).toBeInTheDocument();
    expect(screen.queryByText("Lessac Medium")).not.toBeInTheDocument();
  });

  it("expands family group on click", async () => {
    const user = userEvent.setup();
    render(<VoiceBrowser {...defaultProps} />);
    await user.click(screen.getByText("English"));
    expect(screen.getByText("Lessac Medium")).toBeInTheDocument();
    expect(screen.getByText("Lessac Low")).toBeInTheDocument();
  });

  it("collapses expanded family on second click", async () => {
    const user = userEvent.setup();
    render(<VoiceBrowser {...defaultProps} />);
    await user.click(screen.getByText("English"));
    expect(screen.getByText("Lessac Medium")).toBeInTheDocument();
    await user.click(screen.getByText("English"));
    expect(screen.queryByText("Lessac Medium")).not.toBeInTheDocument();
  });

  it("auto-expands when searching", async () => {
    const user = userEvent.setup();
    render(<VoiceBrowser {...defaultProps} />);
    await user.type(screen.getByPlaceholderText("Search voices..."), "lessac");
    expect(screen.getByText("Lessac Medium")).toBeInTheDocument();
    expect(screen.getByText("Lessac Low")).toBeInTheDocument();
  });

  it("shows Use button for downloaded voices", async () => {
    const user = userEvent.setup();
    render(
      <VoiceBrowser
        {...defaultProps}
        downloadedVoices={new Set(["en_US-lessac-medium"])}
      />
    );
    await user.click(screen.getByText("English"));
    const useButtons = screen.getAllByText("Use");
    expect(useButtons.length).toBeGreaterThanOrEqual(1);
  });

  it("calls onDownload when download button clicked", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    render(<VoiceBrowser {...defaultProps} onDownload={onDownload} />);
    await user.click(screen.getByText("English"));
    const downloadButtons = screen.getAllByText("Download");
    await user.click(downloadButtons[0]);
    expect(onDownload).toHaveBeenCalledOnce();
  });

  it("shows downloading state for specific voice", async () => {
    const user = userEvent.setup();
    render(
      <VoiceBrowser
        {...defaultProps}
        downloadingVoice="en_US-lessac-medium"
      />
    );
    await user.click(screen.getByText("English"));
    expect(screen.getByText("Downloading...")).toBeInTheDocument();
  });

  it("shows empty state when catalog is empty", () => {
    render(<VoiceBrowser {...defaultProps} catalog={{}} />);
    expect(screen.queryByText("English")).not.toBeInTheDocument();
  });
});
