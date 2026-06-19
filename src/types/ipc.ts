// Typed wrappers around Tauri invoke() calls.

import { invoke } from "@tauri-apps/api/core";
import type { VoiceEntry, InstalledVoice } from "./voice-catalog";
import type { QueueSnapshot } from "./queue";
import type { VoiceMapping } from "./voice-prefs";
import type { ShortcutConfig } from "./hotkey";

export function listCatalogVoices(): Promise<VoiceEntry[]> {
  return invoke("list_catalog_voices");
}

export function listInstalledVoices(): Promise<InstalledVoice[]> {
  return invoke("list_installed_voices");
}

export function installVoice(voiceKey: string): Promise<InstalledVoice> {
  return invoke("install_voice", { voiceKey });
}

export function uninstallVoice(voiceKey: string): Promise<void> {
  return invoke("uninstall_voice", { voiceKey });
}

export function getQueueState(): Promise<QueueSnapshot> {
  return invoke("queue_state");
}

export function queueAdd(text: string): Promise<number> {
  return invoke("queue_add", { text });
}

export function queueRemove(id: number): Promise<void> {
  return invoke("queue_remove", { id });
}

export function queueMove(id: number, index: number): Promise<void> {
  return invoke("queue_move", { id, index });
}

export function queueClear(): Promise<void> {
  return invoke("queue_clear");
}

export function queueToggleAutoRead(): Promise<boolean> {
  return invoke("queue_toggle_auto_read");
}

export function getVoicePreference(): Promise<VoiceMapping> {
  return invoke("get_voice_preference");
}

export function setVoicePreference(
  language: string,
  voiceKey: string,
): Promise<void> {
  return invoke("set_voice_preference", { language, voiceKey });
}

export function setFallbackVoice(
  voiceKey: string | null,
): Promise<void> {
  return invoke("set_fallback_voice", { voiceKey });
}

export function getHotkey(): Promise<ShortcutConfig | null> {
  return invoke("get_hotkey");
}

export function saveHotkey(shortcut: string): Promise<ShortcutConfig> {
  return invoke("save_hotkey_cmd", { shortcut });
}

// ── Overlay commands ──────────────────────────────────────────────

export function createOverlayWindow(): Promise<void> {
  return invoke("create_overlay_window");
}

export function showOverlayWindow(): Promise<void> {
  return invoke("show_overlay_window");
}

export function hideOverlayWindow(): Promise<void> {
  return invoke("hide_overlay_window");
}

export function toggleOverlayWindow(): Promise<boolean> {
  return invoke("toggle_overlay_window");
}

export function queueToggleOverlay(): Promise<boolean> {
  return invoke("queue_toggle_overlay");
}
