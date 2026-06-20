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
import { LANG_NAMES } from "../../types/lang-names";
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

function matchesFilter(voice: InstalledVoice, filter: string): boolean {
  if (!filter) return true;
  const q = filter.toLowerCase();
  return (
    voice.name.toLowerCase().includes(q) ||
    voice.language.toLowerCase().includes(q) ||
    (LANG_NAMES[voice.language] || "").toLowerCase().includes(q) ||
    voice.model_type.toLowerCase().includes(q)
  );
}

export function InstalledVoices() {
  const { addToast } = useToast();
  const [voices, setVoices] = useState<InstalledVoice[]>([]);
  const [voiceMapping, setVoiceMapping] = useState<VoiceMapping>({
    language_voice: {},
    fallback_voice_key: null,
  });
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");

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

  const filtered = voices.filter((v) => matchesFilter(v, filter));
  const groups = groupByLanguage(filtered);

  const toggleLang = (lang: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(lang)) next.delete(lang);
      else next.add(lang);
      return next;
    });
  };

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
      <input
        type="text"
        className="iv-search"
        placeholder="Filter voices..."
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      {filtered.length === 0 && (
        <div className="iv-empty">No voices match your filter.</div>
      )}
      {Array.from(groups.entries()).map(([lang, langVoices]) => {
        const activeVoice = voiceMapping.language_voice[lang];
        return (
          <div key={lang} className="iv-group">
            <button className="iv-group-header" onClick={() => toggleLang(lang)}>
              <h3 className="iv-lang-header">
                {expanded.has(lang) ? "−" : "+"} {lang}
              </h3>
              {activeVoice && (
                <span className="iv-active-badge">Active: {activeVoice}</span>
              )}
              <span className="iv-lang-count">{langVoices.length}</span>
            </button>
            {expanded.has(lang) && (
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
            )}
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
