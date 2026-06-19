import type { VoiceMapping } from "../../types/voice-prefs";
import type { InstalledVoice } from "../../types/voice-catalog";
import "./VoiceMappingSettings.css";

interface VoiceMappingSettingsProps {
  voiceMapping: VoiceMapping;
  installedVoices: InstalledVoice[];
  onSetLanguageVoice: (language: string, voiceKey: string) => void;
  onSetFallback: (voiceKey: string | null) => void;
}

function getLanguages(installed: InstalledVoice[]): string[] {
  const langs = new Set<string>();
  for (const v of installed) {
    if (v.language) langs.add(v.language);
  }
  return Array.from(langs).sort();
}

export function VoiceMappingSettings({
  voiceMapping,
  installedVoices,
  onSetLanguageVoice,
  onSetFallback,
}: VoiceMappingSettingsProps) {
  const languages = getLanguages(installedVoices);

  if (languages.length === 0) {
    return (
      <div className="vms-empty">
        Install voices to configure per-language voice preferences.
      </div>
    );
  }

  const allVoiceOptions = installedVoices.map((v) => (
    <option key={v.voice_key} value={v.voice_key}>
      {v.name} ({v.voice_key})
    </option>
  ));

  return (
    <div className="vms-container">
      <h3 className="vms-title">Voice Preferences</h3>

      {languages.map((lang) => (
        <div key={lang} className="vms-row">
          <label className="vms-lang-label">{lang}</label>
          <select
            className="vms-select"
            value={voiceMapping.language_voice[lang] || ""}
            onChange={(e) => onSetLanguageVoice(lang, e.target.value)}
          >
            <option value="">Use fallback</option>
            {allVoiceOptions}
          </select>
        </div>
      ))}

      <div className="vms-row vms-fallback-row">
        <label className="vms-lang-label">Fallback</label>
        <select
          className="vms-select"
          value={voiceMapping.fallback_voice_key || ""}
          onChange={(e) => onSetFallback(e.target.value || null)}
        >
          <option value="">None</option>
          {allVoiceOptions}
        </select>
      </div>
    </div>
  );
}
