import { describe, it, expect } from "vitest";
import { formatSize } from "../format";

describe("formatSize", () => {
  it("formats 0 bytes", () => {
    expect(formatSize(0)).toBe("0 KB");
  });

  it("formats bytes under 1KB", () => {
    expect(formatSize(500)).toBe("0 KB");
  });

  it("formats exactly 1KB", () => {
    expect(formatSize(1024)).toBe("1 KB");
  });

  it("formats KB range", () => {
    expect(formatSize(1536)).toBe("2 KB");
  });

  it("formats exactly 1MB", () => {
    expect(formatSize(1048576)).toBe("1 MB");
  });

  it("formats MB with precision", () => {
    expect(formatSize(1572864, 1)).toBe("1.5 MB");
  });

  it("formats large MB without precision", () => {
    expect(formatSize(63000000)).toBe("60 MB");
  });
});
