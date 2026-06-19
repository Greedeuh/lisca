import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { VoiceBrowser, InstalledVoices } from "./components/voices";
import { QueueList } from "./components/queue";
import { HotkeyRecorder } from "./components/settings";
import { ErrorToast, type ErrorToastItem } from "./components/common";
import { useTtsQueue } from "./hooks";
import type { VoiceEntry, InstalledVoice, DownloadProgress } from "./types/voice-catalog";
import type { VoiceMapping } from "./types/voice-prefs";

import type { ShortcutConfig } from "./types/hotkey";
import {
  listCatalogVoices,
  listInstalledVoices,
  installVoice,
  uninstallVoice,
  queueRemove,
  queueMove,
  queueClear,
  queueToggleAutoRead,
  getVoicePreference,
  setVoicePreference,
  setFallbackVoice,
  getHotkey,
  saveHotkey,
  queueToggleOverlay,
} from "./types/ipc";
import "./App.css";

type Tab = "voices" | "queue" | "settings";

function App() {
  const [tab, setTab] = useState<Tab>("voices");
  const [catalogVoices, setCatalogVoices] = useState<VoiceEntry[]>([]);
  const [installedVoices, setInstalledVoices] = useState<InstalledVoice[]>([]);
  const [voiceMapping, setVoiceMapping] = useState<VoiceMapping>({
    language_voice: {},
    fallback_voice_key: null,
  });
  const [hotkey, setHotkey] = useState<ShortcutConfig | null>(null);
  const [downloading, setDownloading] = useState<Map<string, DownloadProgress>>(new Map());
  const [toasts, setToasts] = useState<ErrorToastItem[]>([]);
  const toastIdRef = useRef(0);

  // Use the useTtsQueue hook for live queue updates
  const { items: queueItems, autoRead, showOverlay, refresh: refreshQueue } = useTtsQueue();

  const addToast = useCallback((message: string) => {
    const id = ++toastIdRef.current;
    setToasts((prev) => [...prev, { id, message }]);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const refreshInstalled = useCallback(async () => {
    try {
      const voices = await listInstalledVoices();
      setInstalledVoices(voices);
    } catch (e) {
      addToast(`Failed to load installed voices: ${e}`);
    }
  }, [addToast]);

  const refreshVoiceMapping = useCallback(async () => {
    try {
      const mapping = await getVoicePreference();
      setVoiceMapping(mapping);
    } catch (e) {
      addToast(`Failed to load voice preferences: ${e}`);
    }
  }, [addToast]);

  useEffect(() => {
    listCatalogVoices()
      .then(setCatalogVoices)
      .catch((e) => addToast(`Failed to load catalog: ${e}`));
    refreshInstalled();
    refreshVoiceMapping();
    getHotkey()
      .then(setHotkey)
      .catch((e) => addToast(`Failed to load hotkey: ${e}`));

    const unlistenProgress = listen<DownloadProgress>("download_progress", (event) => {
      setDownloading((prev) => new Map(prev).set(event.payload.voice_key, event.payload));
    });
    const unlistenComplete = listen<string>("download_complete", () => {
      refreshInstalled();
      setDownloading(new Map());
    });
    const unlistenTranscriptionError = listen<{ id: number; error: string }>(
      "transcription_error",
      (event) => {
        addToast(`Transcription failed: ${event.payload.error}`);
        refreshQueue();
      },
    );
    const unlistenDownloadError = listen<{ voice_key: string; reason: string }>(
      "download_error",
      (event) => {
        addToast(`Download failed for ${event.payload.voice_key}: ${event.payload.reason}`);
      },
    );

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenTranscriptionError.then((fn) => fn());
      unlistenDownloadError.then((fn) => fn());
    };
  }, [addToast, refreshInstalled, refreshQueue, refreshVoiceMapping]);

  const handleInstall = useCallback(
    async (voiceKey: string) => {
      try {
        await installVoice(voiceKey);
        await refreshInstalled();
      } catch (e) {
        addToast(`Failed to install voice: ${e}`);
      }
    },
    [addToast, refreshInstalled],
  );

  const handleUninstall = useCallback(
    async (voiceKey: string) => {
      try {
        await uninstallVoice(voiceKey);
        await refreshInstalled();
      } catch (e) {
        addToast(`Failed to uninstall voice: ${e}`);
      }
    },
    [addToast, refreshInstalled],
  );

  const handleSetActive = useCallback(
    async (language: string, voiceKey: string) => {
      try {
        await setVoicePreference(language, voiceKey);
        await refreshVoiceMapping();
      } catch (e) {
        addToast(`Failed to set voice preference: ${e}`);
      }
    },
    [addToast, refreshVoiceMapping],
  );

  const handleSetFallback = useCallback(
    async (voiceKey: string | null) => {
      try {
        await setFallbackVoice(voiceKey);
        await refreshVoiceMapping();
      } catch (e) {
        addToast(`Failed to set fallback voice: ${e}`);
      }
    },
    [addToast, refreshVoiceMapping],
  );

  const handleRemove = useCallback(
    async (id: number) => {
      try {
        await queueRemove(id);
        await refreshQueue();
      } catch (e) {
        addToast(`Failed to remove item: ${e}`);
      }
    },
    [addToast, refreshQueue],
  );

  const handleMove = useCallback(
    async (id: number, index: number) => {
      try {
        await queueMove(id, index);
        await refreshQueue();
      } catch (e) {
        addToast(`Failed to move item: ${e}`);
      }
    },
    [addToast, refreshQueue],
  );

  const handleClear = useCallback(async () => {
    try {
      await queueClear();
      await refreshQueue();
    } catch (e) {
      addToast(`Failed to clear queue: ${e}`);
    }
  }, [addToast, refreshQueue]);

  const handleToggleAutoRead = useCallback(async () => {
    try {
      await queueToggleAutoRead();
      await refreshQueue();
    } catch (e) {
      addToast(`Failed to toggle auto-read: ${e}`);
    }
  }, [addToast, refreshQueue]);

  const handleSaveHotkey = useCallback(
    async (shortcut: string) => {
      try {
        const config = await saveHotkey(shortcut);
        setHotkey(config);
      } catch (e) {
        addToast(`Failed to save hotkey: ${e}`);
      }
    },
    [addToast],
  );

  const handleToggleOverlay = useCallback(async () => {
    try {
      await queueToggleOverlay();
      await refreshQueue();
    } catch (e) {
      addToast(`Failed to toggle overlay: ${e}`);
    }
  }, [addToast, refreshQueue]);

  const installedKeys = new Set(installedVoices.map((v) => v.voice_key));

  return (
    <main className="app-container">
      <ErrorToast toasts={toasts} onDismiss={dismissToast} />
      <header className="app-header">
        <h1 className="app-title">Lisca</h1>
        <nav className="app-tabs">
          <button
            className={`app-tab ${tab === "voices" ? "app-tab-active" : ""}`}
            onClick={() => setTab("voices")}
          >
            Voices
          </button>
          <button
            className={`app-tab ${tab === "queue" ? "app-tab-active" : ""}`}
            onClick={() => setTab("queue")}
          >
            Queue
          </button>
          <button
            className={`app-tab ${tab === "settings" ? "app-tab-active" : ""}`}
            onClick={() => setTab("settings")}
          >
            Settings
          </button>
        </nav>
      </header>

      <div className="app-content">
        {tab === "voices" && (
          <div className="app-voices">
            <section className="app-section">
              <h2 className="app-section-title">Available Voices</h2>
              <VoiceBrowser
                voices={catalogVoices}
                installedKeys={installedKeys}
                downloading={downloading}
                onInstall={handleInstall}
              />
            </section>
            <section className="app-section">
              <h2 className="app-section-title">Installed Voices</h2>
              <InstalledVoices
                voices={installedVoices}
                voiceMapping={voiceMapping}
                onUninstall={handleUninstall}
                onSetActive={handleSetActive}
                onSetFallback={handleSetFallback}
              />
            </section>
          </div>
        )}

        {tab === "queue" && (
          <section className="app-section">
            <QueueList
              items={queueItems}
              autoRead={autoRead}
              onRemove={handleRemove}
              onMove={handleMove}
              onToggleAutoRead={handleToggleAutoRead}
              onClear={handleClear}
            />
          </section>
        )}

        {tab === "settings" && (
          <div className="app-settings">
            <section className="app-section">
              <h2 className="app-section-title">Overlay</h2>
              <label className="app-setting-row">
                <input
                  type="checkbox"
                  checked={showOverlay}
                  onChange={handleToggleOverlay}
                />
                Show overlay when main window is closed
              </label>
            </section>
            <section className="app-section">
              <HotkeyRecorder currentHotkey={hotkey} onSave={handleSaveHotkey} />
            </section>
          </div>
        )}
      </div>
    </main>
  );
}

export default App;
