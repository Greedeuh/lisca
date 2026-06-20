import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "player_state") return Promise.resolve({ auto_read: true });
    if (cmd === "queue_state") return Promise.resolve({ items: [], show_overlay: true });
    return Promise.resolve(null);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    hide: vi.fn().mockResolvedValue(undefined),
    show: vi.fn().mockResolvedValue(undefined),
    startDragging: vi.fn().mockResolvedValue(undefined),
    is_visible: vi.fn().mockResolvedValue(false),
  })),
}));

describe("App", () => {
  it("renders the app title", () => {
    render(<App />);
    expect(screen.getByText("Lisca")).toBeInTheDocument();
  });

  it("renders tab buttons", () => {
    render(<App />);
    expect(screen.getByText("Voices")).toBeInTheDocument();
    expect(screen.getByText("Queue")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("shows Voices tab as active by default", () => {
    render(<App />);
    expect(screen.getByText("Voices")).toHaveClass("app-tab-active");
  });

  it("switches to Queue tab on click", () => {
    render(<App />);
    fireEvent.click(screen.getByText("Queue"));
    expect(screen.getByText("Queue")).toHaveClass("app-tab-active");
    expect(screen.getByText("Voices")).not.toHaveClass("app-tab-active");
  });

  it("switches to Settings tab on click", () => {
    render(<App />);
    fireEvent.click(screen.getByText("Settings"));
    expect(screen.getByText("Settings")).toHaveClass("app-tab-active");
    expect(screen.getByText("Voices")).not.toHaveClass("app-tab-active");
  });
});
