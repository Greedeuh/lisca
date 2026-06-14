import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { InstalledModels } from "../InstalledModels";
import type { InstalledModel } from "../../types/piper";

const mockModels: InstalledModel[] = [
  {
    voice_key: "en_US-lessac-medium",
    model_path: "/path/to/en_US-lessac-medium.onnx",
    config_path: "/path/to/en_US-lessac-medium.onnx.json",
    language: { code: "en-US", family: "en", region: "US", name_native: "English", name_english: "English", country_english: "United States" },
    quality: "medium",
    name: "Lessac Medium",
  },
  {
    voice_key: "fr_FR-siwis-medium",
    model_path: "/path/to/fr_FR-siwis-medium.onnx",
    config_path: "/path/to/fr_FR-siwis-medium.onnx.json",
    language: { code: "fr-FR", family: "fr", region: "FR", name_native: "Français", name_english: "French", country_english: "France" },
    quality: "medium",
    name: "Siwis Medium",
  },
];

describe("InstalledModels", () => {
  it("shows empty state when no models", () => {
    render(
      <InstalledModels
        models={[]}
        activeModelPath={null}
        onDelete={vi.fn()}
      />
    );
    expect(screen.getByText("No models installed yet.")).toBeInTheDocument();
  });

  it("renders all models", () => {
    render(
      <InstalledModels
        models={mockModels}
        activeModelPath={null}
        onDelete={vi.fn()}
      />
    );
    expect(screen.getByText("Lessac Medium")).toBeInTheDocument();
    expect(screen.getByText("Siwis Medium")).toBeInTheDocument();
  });

  it("shows active badge for active model", () => {
    render(
      <InstalledModels
        models={mockModels}
        activeModelPath="/path/to/en_US-lessac-medium.onnx"
        onDelete={vi.fn()}
      />
    );
    expect(screen.getByText("Active")).toBeInTheDocument();
  });

  it("calls onDelete with voice_key when Delete clicked", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    render(
      <InstalledModels
        models={mockModels}
        activeModelPath={null}
        onDelete={onDelete}
      />
    );
    const deleteButtons = screen.getAllByText("Delete");
    await user.click(deleteButtons[0]);
    expect(onDelete).toHaveBeenCalledWith("en_US-lessac-medium");
  });
});
