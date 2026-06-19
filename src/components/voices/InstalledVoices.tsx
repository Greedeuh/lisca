import type { InstalledVoice } from "../../types/voice-catalog";
import type { VoiceMapping } from "../../types/voice-prefs";
import "./InstalledVoices.css";

interface InstalledVoicesProps {
  voices: InstalledVoice[];
  voiceMapping: VoiceMapping;
  onUninstall: (voiceKey: string) => void;
  onSetActive: (language: string, voiceKey: string) => void;
  onSetFallback: (voiceKey: string | null) => void;
}

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

export function InstalledVoices({
  voices,
  voiceMapping,
  onUninstall,
  onSetActive,
  onSetFallback,
}: InstalledVoicesProps) {
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
                          onClick={() => onSetActive(lang, voice.voice_key)}
                        >
                          Set Active
                        </button>
                      )}
                      <button
                        className="iv-btn iv-btn-remove"
                        onClick={() => onUninstall(voice.voice_key)}
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
          onChange={(e) => onSetFallback(e.target.value || null)}
        >
          <option value="">None</option>
          {allVoices}
        </select>
      </div>
    </div>
  );
}
