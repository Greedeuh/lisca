# Voice Catalog & Model Management

## Feature
Browse, search, download, install, and manage Piper TTS voice models from the HuggingFace catalog.

## Scenarios

- **As a user**, I can browse available voices grouped by language family and locale, so I can find the right voice for my language.
- **As a user**, I can type in a search box to filter voices by name, language, or locale code, so I can quickly find a specific voice.
- **As a user**, I can see each voice's quality level (x_low / low / medium / high) displayed as a colored badge, so I can compare quality at a glance.
- **As a user**, I can see the file size of each voice before downloading, so I can decide if I want to download it.
- **As a user**, I can click "Download" on a voice and see a progress bar, so I know the download is working and how long it will take.
- **As a user**, after downloading, the voice appears in my "Installed Voices" list, so I know what's available.
- **As a user**, I can see which voice is currently active (highlighted with an "Active" badge), so I know what's loaded.
- **As a user**, I can delete an installed voice to free up disk space.
- **As a user**, the voice catalog is fetched automatically on app start, and I can see an error with a "Retry" button if the fetch fails.

## Key Files
- `src-tauri/src/tts/piper_models.rs` — catalog fetch, download, install, delete
- `src/components/PiperModelPicker.tsx` — main picker container
- `src/components/VoiceBrowser.tsx` — searchable catalog browser
- `src/components/VoiceRow.tsx` — individual voice entry
- `src/components/InstalledModels.tsx` — installed models list
- `src/components/DownloadProgress.tsx` — download progress bar
- `src/hooks/usePiperModels.ts` — frontend state management
