import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DownloadProgress } from "../DownloadProgress";

describe("DownloadProgress", () => {
  it("renders voice key", () => {
    render(
      <DownloadProgress
        voiceKey="en_US-lessac-medium"
        bytesDownloaded={0}
        totalBytes={63000000}
      />
    );
    expect(screen.getByText("en_US-lessac-medium")).toBeInTheDocument();
  });

  it("shows size progress", () => {
    render(
      <DownloadProgress
        voiceKey="en_US-lessac-medium"
        bytesDownloaded={31500000}
        totalBytes={63000000}
      />
    );
    expect(screen.getByText(/30 MB/)).toBeInTheDocument();
    expect(screen.getByText(/60 MB/)).toBeInTheDocument();
  });

  it("renders progress bar at 50%", () => {
    render(
      <DownloadProgress
        voiceKey="test"
        bytesDownloaded={50}
        totalBytes={100}
      />
    );
    const fill = document.querySelector(".progress-fill") as HTMLElement;
    expect(fill.style.width).toBe("50%");
  });

  it("renders progress bar at 0% when total is 0", () => {
    render(
      <DownloadProgress
        voiceKey="test"
        bytesDownloaded={0}
        totalBytes={0}
      />
    );
    const fill = document.querySelector(".progress-fill") as HTMLElement;
    expect(fill.style.width).toBe("0%");
  });
});
