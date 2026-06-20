import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useToast } from "../../contexts/toast";
import {
  listInstalledVoices,
  uninstallVoice,
  setVoicePreference,
  setFallbackVoice,
  getVoicePreference,
} from "../../types/ipc";
import type { InstalledVoice } from "../../types/voice-catalog";
import type { VoiceMapping } from "../../types/voice-prefs";
import "./InstalledVoices.css";

function groupByLanguage(voices: InstalledVoice[]): Map<string, InstalledVoice[]> {
  const groups = new Map<string, InstalledVoice[]>();
  for (const v of voices) {
    const lang = v.language || "unknown";
    const list = groups.get(lang) || [];
    list.push(v);
    groups.set(lang, list);
  }
  return groups;
}

export function InstalledVoices() {
  const { addToast } = useToast();
  const [voices, setVoices] = useState<InstalledVoice[]>([]);
  const [voiceMapping, setVoiceMapping] = useState<VoiceMapping>({
    language_voice: {},
    fallback_voice_key: null,
  });

  const refreshInstalled = useCallback(async () => {
    try {
      const installed = await listInstalledVoices();
      setVoices(installed);
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
    refreshInstalled();
    refreshVoiceMapping();

    const unlistenComplete = listen("download_complete", () => {
      refreshInstalled();
    });
    const unlistenUninstalled = listen("voice_uninstalled", () => {
      refreshInstalled();
    });

    return () => {
      unlistenComplete.then((fn) => fn());
      unlistenUninstalled.then((fn) => fn());
    };
  }, [refreshInstalled, refreshVoiceMapping]);

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

  const groups = groupByLanguage(voices);

  if (voices.length === 0) {
    return <div className="iv-empty">No voices installed. Browse the catalog to install voices.</div>;
  }

  const allVoices = voices.map((v) => (
    <option key={v.voice_key} value={v.voice_key}>
      {v.name} ({v.voice_key})
    </option>
  ));

  return (
    <div className="iv-container">
      {Array.from(groups.entries()).map(([lang, langVoices]) => {
        const activeVoice = voiceMapping.language_voice[lang];
        return (
          <div key={lang} className="iv-group">
            <div className="iv-group-header">
              <h3 className="iv-lang-header">{lang}</h3>
              {activeVoice && (
                <span className="iv-active-badge">Active: {activeVoice}</span>
              )}
            </div>
            <div className="iv-voices">
              {langVoices.map((voice) => {
                const isActive = activeVoice === voice.voice_key;
                return (
                  <div
                    key={voice.voice_key}
                    className={`iv-voice-row ${isActive ? "iv-voice-active" : ""}`}
                  >
                    <div className="iv-voice-info">
                      <span className="iv-voice-name">{voice.name}</span>
                      <div className="iv-voice-meta">
                        <span className="iv-badge">{voice.quality}</span>
                        <span className="iv-badge">{voice.model_type}</span>
                      </div>
                    </div>
                    <div className="iv-voice-actions">
                      {!isActive && (
                        <button
                          className="iv-btn iv-btn-set"
                          onClick={() => handleSetActive(lang, voice.voice_key)}
                        >
                          Set Active
                        </button>
                      )}
                      <button
                        className="iv-btn iv-btn-remove"
                        onClick={() => handleUninstall(voice.voice_key)}
                      >
                        Uninstall
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        );
      })}

      <div className="iv-fallback">
        <label className="iv-fallback-label">Fallback voice:</label>
        <select
          className="iv-fallback-select"
          value={voiceMapping.fallback_voice_key || ""}
          onChange={(e) => handleSetFallback(e.target.value || null)}
        >
          <option value="">None</option>
          {allVoices}
        </select>
      </div>
    </div>
  );
}
