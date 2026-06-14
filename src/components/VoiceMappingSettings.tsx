import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { InstalledModel } from "../types/piper";
import type { VoiceMapping } from "../types/voiceMapping";

interface VoiceMappingSettingsProps {
  installedModels: InstalledModel[];
}

export function VoiceMappingSettings({ installedModels }: VoiceMappingSettingsProps) {
  const [mapping, setMapping] = useState<VoiceMapping>({ language_voice: {}, fallback_voice_key: null });
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState("");

  useEffect(() => {
    invoke<VoiceMapping>("tts_get_voice_mapping")
      .then(setMapping)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const languages = [...new Set(installedModels.map((m) => m.language.family))].sort();
  const mappedLanguages = Object.keys(mapping.language_voice);

  const unmappedLanguages = languages.filter((l) => !mappedLanguages.includes(l));

  function handleSetLanguageVoice(family: string, voiceKey: string) {
    const newMapping: VoiceMapping = {
      ...mapping,
      language_voice: { ...mapping.language_voice, [family]: voiceKey },
    };
    saveMapping(newMapping);
  }

  function handleRemoveLanguage(family: string) {
    const { [family]: _, ...rest } = mapping.language_voice;
    const newMapping: VoiceMapping = { ...mapping, language_voice: rest };
    saveMapping(newMapping);
  }

  function handleSetFallback(voiceKey: string | null) {
    const newMapping: VoiceMapping = { ...mapping, fallback_voice_key: voiceKey };
    saveMapping(newMapping);
  }

  function handleAddLanguage(family: string) {
    const model = installedModels.find((m) => m.language.family === family);
    if (!model) return;
    const newMapping: VoiceMapping = {
      ...mapping,
      language_voice: { ...mapping.language_voice, [family]: model.voice_key },
    };
    saveMapping(newMapping);
  }

  function saveMapping(newMapping: VoiceMapping) {
    setMapping(newMapping);
    invoke("tts_set_voice_mapping", { mapping: newMapping })
      .then(() => {
        setStatus("Saved");
        setTimeout(() => setStatus(""), 2000);
      })
      .catch((err) => setStatus("Error: " + err));
  }

  function modelsForFamily(family: string): InstalledModel[] {
    return installedModels.filter((m) => m.language.family === family);
  }

  if (loading) return null;

  return (
    <div className="voice-mapping-settings">
      <h3>Language Routing</h3>
      <p className="hint">
        Choose which voice to use for each detected language.
      </p>

      {mappedLanguages.map((family) => {
        const models = modelsForFamily(family);
        const currentKey = mapping.language_voice[family];
        return (
          <div key={family} className="mapping-row">
            <span className="mapping-language">{family}</span>
            <select
              value={currentKey}
              onChange={(e) => handleSetLanguageVoice(family, e.target.value)}
            >
              {models.map((m) => (
                <option key={m.voice_key} value={m.voice_key}>
                  {m.name} ({m.language.code})
                </option>
              ))}
            </select>
            <button
              className="mapping-remove-btn"
              onClick={() => handleRemoveLanguage(family)}
              title="Remove mapping"
            >
              ✕
            </button>
          </div>
        );
      })}

      {unmappedLanguages.length > 0 && (
        <div className="mapping-add">
          <select
            value=""
            onChange={(e) => {
              if (e.target.value) handleAddLanguage(e.target.value);
              e.target.value = "";
            }}
          >
            <option value="">Add language...</option>
            {unmappedLanguages.map((family) => (
              <option key={family} value={family}>
                {family}
              </option>
            ))}
          </select>
        </div>
      )}

      <div className="mapping-row mapping-fallback">
        <span className="mapping-language">Fallback</span>
        <select
          value={mapping.fallback_voice_key ?? ""}
          onChange={(e) => handleSetFallback(e.target.value || null)}
        >
          <option value="">None</option>
          {installedModels.map((m) => (
            <option key={m.voice_key} value={m.voice_key}>
              {m.name} ({m.language.code})
            </option>
          ))}
        </select>
      </div>

      {status && <p className="status">{status}</p>}
    </div>
  );
}
