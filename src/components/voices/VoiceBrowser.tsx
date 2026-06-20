import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useToast } from "../../contexts/toast";
import {
  listCatalogVoices,
  listInstalledVoices,
  installVoice,
} from "../../types/ipc";
import type { VoiceEntry, DownloadProgress } from "../../types/voice-catalog";
import "./VoiceBrowser.css";

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
}

function groupByLanguage(voices: VoiceEntry[]): Map<string, VoiceEntry[]> {
  const groups = new Map<string, VoiceEntry[]>();
  for (const v of voices) {
    const lang = v.language || "unknown";
    const list = groups.get(lang) || [];
    list.push(v);
    groups.set(lang, list);
  }
  return groups;
}

export function VoiceBrowser() {
  const { addToast } = useToast();
  const [voices, setVoices] = useState<VoiceEntry[]>([]);
  const [installedKeys, setInstalledKeys] = useState<Set<string>>(new Set());
  const [downloading, setDownloading] = useState<Map<string, DownloadProgress>>(new Map());

  const refreshInstalled = useCallback(async () => {
    try {
      const installed = await listInstalledVoices();
      setInstalledKeys(new Set(installed.map((v) => v.voice_key)));
    } catch (e) {
      addToast(`Failed to load installed voices: ${e}`);
    }
  }, [addToast]);

  useEffect(() => {
    listCatalogVoices()
      .then(setVoices)
      .catch((e) => addToast(`Failed to load catalog: ${e}`));
    refreshInstalled();

    const unlistenProgress = listen<DownloadProgress>("download_progress", (event) => {
      setDownloading((prev) => new Map(prev).set(event.payload.voice_key, event.payload));
    });
    const unlistenComplete = listen<string>("download_complete", () => {
      refreshInstalled();
      setDownloading(new Map());
    });
    const unlistenError = listen<{ voice_key: string; reason: string }>(
      "download_error",
      (event) => {
        addToast(`Download failed for ${event.payload.voice_key}: ${event.payload.reason}`);
      },
    );
    const unlistenUninstalled = listen("voice_uninstalled", () => {
      refreshInstalled();
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
      unlistenUninstalled.then((fn) => fn());
    };
  }, [addToast, refreshInstalled]);

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

  const groups = groupByLanguage(voices);

  if (voices.length === 0) {
    return <div className="vb-empty">No voices available.</div>;
  }

  return (
    <div className="vb-container">
      {Array.from(groups.entries()).map(([lang, langVoices]) => (
        <div key={lang} className="vb-group">
          <h3 className="vb-lang-header">{lang}</h3>
          <div className="vb-voices">
            {langVoices.map((voice) => {
              const isInstalled = installedKeys.has(voice.voice_key);
              const progress = downloading.get(voice.voice_key);
              const isDownloading = progress !== undefined;
              const pct = isDownloading && progress.type === "downloading"
                ? Math.round((progress.bytes_downloaded / progress.total_bytes) * 100)
                : 0;

              return (
                <div key={voice.voice_key} className="vb-voice-card">
                  <div className="vb-voice-info">
                    <span className="vb-voice-name">{voice.name}</span>
                    <div className="vb-voice-meta">
                      <span className="vb-badge">{voice.quality}</span>
                      {voice.speed && <span className="vb-badge">{voice.speed}</span>}
                      <span className="vb-badge">{voice.model_type}</span>
                      <span className="vb-size">{formatSize(voice.size_bytes)}</span>
                    </div>
                  </div>
                  <div className="vb-voice-action">
                    {isInstalled ? (
                      <span className="vb-installed-label">Installed</span>
                    ) : isDownloading ? (
                      <div className="vb-progress">
                        <div
                          className="vb-progress-bar"
                          style={{ width: `${pct}%` }}
                        />
                        <span className="vb-progress-text">{pct}%</span>
                      </div>
                    ) : (
                      <button
                        className="vb-btn vb-btn-install"
                        onClick={() => handleInstall(voice.voice_key)}
                      >
                        Install
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
