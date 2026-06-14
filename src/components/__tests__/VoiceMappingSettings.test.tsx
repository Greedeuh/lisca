import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VoiceMappingSettings } from "../VoiceMappingSettings";
import type { InstalledModel } from "../../types/piper";

const mockModels: InstalledModel[] = [
  {
    voice_key: "en_US-lessac",
    model_path: "/m/en_US-lessac.onnx",
    config_path: "/m/en_US-lessac.onnx.json",
    language: { code: "en-US", family: "en", region: "US", name_native: "English", name_english: "English", country_english: "United States" },
    quality: "high",
    name: "Lessac",
  },
  {
    voice_key: "fr_FR-siwis",
    model_path: "/m/fr_FR-siwis.onnx",
    config_path: "/m/fr_FR-siwis.onnx.json",
    language: { code: "fr-FR", family: "fr", region: "FR", name_native: "Français", name_english: "French", country_english: "France" },
    quality: "medium",
    name: "Siwis",
  },
];

function mockInvoke(fn: (cmd: string, args?: any) => any) {
  (window as any).__TAURI_INTERNALS__.invoke = vi.fn(fn);
}

describe("VoiceMappingSettings", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("renders language routing heading", async () => {
    mockInvoke(() => Promise.resolve({ language_voice: {}, fallback_voice_key: null }));
    render(<VoiceMappingSettings installedModels={mockModels} />);
    await waitFor(() => {
      expect(screen.getByText("Language Routing")).toBeInTheDocument();
    });
  });

  it("shows loading state then renders after fetch", async () => {
    mockInvoke(() => Promise.resolve({ language_voice: { en: "en_US-lessac" }, fallback_voice_key: null }));
    render(<VoiceMappingSettings installedModels={mockModels} />);
    expect(screen.queryByText("Language Routing")).not.toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText("Language Routing")).toBeInTheDocument();
    });
  });

  it("renders dropdown for mapped language", async () => {
    mockInvoke(() => Promise.resolve({ language_voice: { en: "en_US-lessac" }, fallback_voice_key: null }));
    render(<VoiceMappingSettings installedModels={mockModels} />);
    await waitFor(() => {
      expect(screen.getByText("en")).toBeInTheDocument();
    });
    const selects = screen.getAllByRole("combobox");
    expect(selects.length).toBeGreaterThanOrEqual(1);
  });

  it("shows fallback selector", async () => {
    mockInvoke(() => Promise.resolve({ language_voice: {}, fallback_voice_key: null }));
    render(<VoiceMappingSettings installedModels={mockModels} />);
    await waitFor(() => {
      expect(screen.getByText("Fallback")).toBeInTheDocument();
    });
  });

  it("add language dropdown shows unmapped families", async () => {
    mockInvoke(() => Promise.resolve({ language_voice: { en: "en_US-lessac" }, fallback_voice_key: null }));
    render(<VoiceMappingSettings installedModels={mockModels} />);
    await waitFor(() => {
      expect(screen.getByText("Add language...")).toBeInTheDocument();
    });
    const addSelect = screen.getAllByRole("combobox").find(
      (el) => (el as HTMLSelectElement).options[0]?.text === "Add language..."
    );
    expect(addSelect).toBeDefined();
    const options = Array.from((addSelect as HTMLSelectElement).options);
    expect(options.some((o) => o.value === "fr")).toBe(true);
  });

  it("saves mapping and shows status", async () => {
    const user = userEvent.setup();
    mockInvoke(() => Promise.resolve({ language_voice: {}, fallback_voice_key: null }));
    render(<VoiceMappingSettings installedModels={mockModels} />);
    await waitFor(() => {
      expect(screen.getByText("Language Routing")).toBeInTheDocument();
    });

    const addSelect = screen.getAllByRole("combobox").find(
      (el) => (el as HTMLSelectElement).options[0]?.text === "Add language..."
    );
    await user.selectOptions(addSelect!, "en");

    await waitFor(() => {
      expect(screen.getByText("Saved")).toBeInTheDocument();
    });
  });
});
