import { useState, useMemo } from "react";
import type { VoiceCatalog, VoiceGroup, LocaleGroup } from "../types/piper";
import { VoiceRow } from "./VoiceRow";

interface VoiceBrowserProps {
  catalog: VoiceCatalog;
  downloadedVoices: Set<string>;
  downloadingVoice: string | null;
  onDownload: (voiceKey: string) => void;
  onSelect: (voiceKey: string) => void;
}

export function VoiceBrowser({
  catalog,
  downloadedVoices,
  downloadingVoice,
  onDownload,
  onSelect,
}: VoiceBrowserProps) {
  const [search, setSearch] = useState("");
  const [expandedFamilies, setExpandedFamilies] = useState<Set<string>>(
    new Set()
  );

  // Group voices by language family -> locale
  const voiceGroups = useMemo(() => {
    const groups: VoiceGroup[] = [];
    const familyMap = new Map<string, Map<string, LocaleGroup>>();

    for (const voice of Object.values(catalog)) {
      const { family, code, name_english, country_english } = voice.language;

      if (!familyMap.has(family)) {
        familyMap.set(family, new Map());
      }
      const localeMap = familyMap.get(family)!;

      if (!localeMap.has(code)) {
        localeMap.set(code, {
          code,
          name: name_english,
          country: country_english,
          voices: [],
        });
      }
      localeMap.get(code)!.voices.push(voice);
    }

    // Convert to array and sort
    for (const [family, localeMap] of familyMap) {
      const locales = Array.from(localeMap.values()).sort((a, b) =>
        a.code.localeCompare(b.code)
      );
      groups.push({
        family,
        familyName: locales[0]?.name || family,
        locales,
      });
    }

    return groups.sort((a, b) => a.family.localeCompare(b.family));
  }, [catalog]);

  // Filter by search
  const filteredGroups = useMemo(() => {
    if (!search.trim()) return voiceGroups;

    const searchLower = search.toLowerCase();
    return voiceGroups
      .map((group) => ({
        ...group,
        locales: group.locales
          .map((locale) => ({
            ...locale,
            voices: locale.voices.filter(
              (v) =>
                v.key.toLowerCase().includes(searchLower) ||
                v.name.toLowerCase().includes(searchLower) ||
                locale.code.toLowerCase().includes(searchLower) ||
                locale.name.toLowerCase().includes(searchLower)
            ),
          }))
          .filter((locale) => locale.voices.length > 0),
      }))
      .filter((group) => group.locales.length > 0);
  }, [voiceGroups, search]);

  const toggleFamily = (family: string) => {
    setExpandedFamilies((prev) => {
      const next = new Set(prev);
      if (next.has(family)) {
        next.delete(family);
      } else {
        next.add(family);
      }
      return next;
    });
  };

  return (
    <div className="voice-browser">
      <div className="search-box">
        <input
          type="text"
          placeholder="Search voices..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      <div className="voice-list">
        {filteredGroups.map((group) => (
          <div key={group.family} className="voice-group">
            <div
              className="voice-group-header"
              onClick={() => toggleFamily(group.family)}
            >
              <span className="expand-icon">
                {(search.trim() || expandedFamilies.has(group.family)) ? "▼" : "▶"}
              </span>
              <span className="family-name">{group.familyName}</span>
              <span className="family-code">({group.family})</span>
            </div>

            {(search.trim() || expandedFamilies.has(group.family)) && (
              <div className="voice-group-content">
                {group.locales.map((locale) => (
                  <div key={locale.code} className="locale-group">
                    <div className="locale-header">
                      <span className="locale-code">{locale.code}</span>
                      <span className="locale-name">{locale.country}</span>
                    </div>
                    <div className="locale-voices">
                      {locale.voices.map((voice) => (
                        <VoiceRow
                          key={voice.key}
                          voice={voice}
                          isDownloaded={downloadedVoices.has(voice.key)}
                          isDownloading={downloadingVoice === voice.key}
                          onDownload={() => onDownload(voice.key)}
                          onSelect={() => onSelect(voice.key)}
                        />
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
