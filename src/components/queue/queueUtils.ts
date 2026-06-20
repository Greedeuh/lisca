import type { QueueItem } from "../../types/queue";

export const MAX_TEXT_PREVIEW = 80;

export function truncate(text: string, max: number): string {
  return text.length > max ? text.slice(0, max) + "…" : text;
}

export function statusLabel(item: QueueItem): string {
  if (item.type === "TextMessage") {
    return item.status === "processing" ? "Processing" : "Pending";
  }
  switch (item.status) {
    case "playing":
      return "Playing";
    case "paused":
      return "Paused";
    case "played":
      return "Done";
    default:
      return "Queued";
  }
}

export function statusClass(item: QueueItem): string {
  if (item.type === "TextMessage") {
    return item.status === "processing" ? "status-processing" : "status-pending";
  }
  switch (item.status) {
    case "playing":
      return "status-playing";
    case "paused":
      return "status-paused";
    case "played":
      return "status-played";
    default:
      return "status-queued";
  }
}
