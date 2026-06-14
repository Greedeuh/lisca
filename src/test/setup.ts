import "@testing-library/jest-dom";
import { randomFillSync } from "crypto";

beforeEach(() => {
  // @tauri-apps/api/mocks needs crypto in jsdom
  Object.defineProperty(window, "crypto", {
    value: {
      getRandomValues: (buffer: Uint8Array) => randomFillSync(buffer),
    },
  });

  // Mock __TAURI_INTERNALS__ for components that use invoke
  (window as any).__TAURI_INTERNALS__ = {
    invoke: vi.fn(),
  };
});

afterEach(() => {
  vi.restoreAllMocks();
});
