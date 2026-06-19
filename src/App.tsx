import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { VoiceBrowser, InstalledVoices } from "./components/voices";
import { QueueList } from "./components/queue";
import { HotkeyRecorder } from "./components/settings";
import type { VoiceEntry, InstalledVoice, DownloadProgress } from "./types/voice-catalog";
import type { QueueItem, QueueSnapshot } from "./types/queue";
import type { VoiceMapping } from "./types/voice-prefs";

import type { ShortcutConfig } from "./types/hotkey";
import {
  listCatalogVoices,
  listInstalledVoices,
  installVoice,
  uninstallVoice,
  getQueueState,
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
  const [queueItems, setQueueItems] = useState<QueueItem[]>([]);
  const [autoRead, setAutoRead] = useState(true);
  const [showOverlay, setShowOverlay] = useState(true);
  const [voiceMapping, setVoiceMapping] = useState<VoiceMapping>({
    language_voice: {},
    fallback_voice_key: null,
  });
  const [hotkey, setHotkey] = useState<ShortcutConfig | null>(null);
  const [downloading, setDownloading] = useState<Map<string, DownloadProgress>>(new Map());

  const refreshInstalled = useCallback(async () => {
    try {
      const voices = await listInstalledVoices();
      setInstalledVoices(voices);
    } catch {}
  }, []);

  const refreshQueue = useCallback(async () => {
    try {
      const snapshot: QueueSnapshot = await getQueueState();
      setQueueItems(snapshot.items);
      setAutoRead(snapshot.auto_read);
      setShowOverlay(snapshot.show_overlay);
    } catch {}
  }, []);

  const refreshVoiceMapping = useCallback(async () => {
    try {
      const mapping = await getVoicePreference();
      setVoiceMapping(mapping);
    } catch {}
  }, []);

  useEffect(() => {
    listCatalogVoices().then(setCatalogVoices).catch(() => {});
    refreshInstalled();
    refreshQueue();
    refreshVoiceMapping();
    getHotkey().then(setHotkey).catch(() => {});

    const unlistenProgress = listen<DownloadProgress>("download_progress", (event) => {
      setDownloading((prev) => new Map(prev).set(event.payload.voice_key, event.payload));
    });
    const unlistenComplete = listen<string>("download_complete", () => {
      refreshInstalled();
      setDownloading(new Map());
    });
    const unlistenQueue = listen("queue_updated", () => {
      refreshQueue();
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenQueue.then((fn) => fn());
    };
  }, [refreshInstalled, refreshQueue, refreshVoiceMapping]);

  const handleInstall = useCallback(
    async (voiceKey: string) => {
      try {
        await installVoice(voiceKey);
        await refreshInstalled();
      } catch {}
    },
    [refreshInstalled],
  );

  const handleUninstall = useCallback(
    async (voiceKey: string) => {
      try {
        await uninstallVoice(voiceKey);
        await refreshInstalled();
      } catch {}
    },
    [refreshInstalled],
  );

  const handleSetActive = useCallback(
    async (language: string, voiceKey: string) => {
      try {
        await setVoicePreference(language, voiceKey);
        await refreshVoiceMapping();
      } catch {}
    },
    [refreshVoiceMapping],
  );

  const handleSetFallback = useCallback(
    async (voiceKey: string | null) => {
      try {
        await setFallbackVoice(voiceKey);
        await refreshVoiceMapping();
      } catch {}
    },
    [refreshVoiceMapping],
  );

  const handleRemove = useCallback(
    async (id: number) => {
      try {
        await queueRemove(id);
        await refreshQueue();
      } catch {}
    },
    [refreshQueue],
  );

  const handleMove = useCallback(
    async (id: number, index: number) => {
      try {
        await queueMove(id, index);
        await refreshQueue();
      } catch {}
    },
    [refreshQueue],
  );

  const handleClear = useCallback(async () => {
    try {
      await queueClear();
      await refreshQueue();
    } catch {}
  }, [refreshQueue]);

  const handleToggleAutoRead = useCallback(async () => {
    try {
      const val = await queueToggleAutoRead();
      setAutoRead(val);
    } catch {}
  }, []);

  const handleSaveHotkey = useCallback(async (shortcut: string) => {
    try {
      const config = await saveHotkey(shortcut);
      setHotkey(config);
    } catch {}
  }, []);

  const handleToggleOverlay = useCallback(async () => {
    try {
      const val = await queueToggleOverlay();
      setShowOverlay(val);
    } catch {}
  }, []);

  const installedKeys = new Set(installedVoices.map((v) => v.voice_key));

  return (
    <main className="app-container">
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
