import { useEffect, useMemo } from "react";
import { usePiperModels } from "../hooks/usePiperModels";
import { VoiceBrowser } from "./VoiceBrowser";
import { InstalledModels } from "./InstalledModels";
import { DownloadProgress } from "./DownloadProgress";

interface PiperModelPickerProps {
  currentModelPath: string | null;
  onSelectModel: (modelPath: string, configPath: string) => void;
}

export function PiperModelPicker({
  currentModelPath,
  onSelectModel,
}: PiperModelPickerProps) {
  const {
    catalog,
    installed,
    loading,
    error,
    downloading,
    downloadProgress,
    fetchCatalog,
    downloadModel,
    deleteModel,
  } = usePiperModels();

  // Fetch catalog on mount
  useEffect(() => {
    if (!catalog && !loading && !error) {
      fetchCatalog();
    }
  }, [catalog, loading, error, fetchCatalog]);

  // Create set of downloaded voice keys for quick lookup
  const downloadedVoices = useMemo(
    () => new Set(installed.map((m) => m.voice_key)),
    [installed]
  );

  const handleSelectModel = (model: { voice_key: string; model_path: string; config_path: string }) => {
    onSelectModel(model.model_path, model.config_path);
  };

  return (
    <div className="piper-model-picker">
      <div className="picker-header">
        <h3>Voices</h3>
        <button
          className="refresh-button secondary"
          onClick={fetchCatalog}
          disabled={loading}
        >
          {loading ? "Loading..." : "Refresh Catalog"}
        </button>
      </div>

      {error && (
        <div className="picker-error">
          <p>{error}</p>
          <button onClick={fetchCatalog}>Retry</button>
        </div>
      )}

      {downloading && downloadProgress && (
        <DownloadProgress
          voiceKey={downloading}
          bytesDownloaded={downloadProgress.bytes}
          totalBytes={downloadProgress.total}
        />
      )}

      <div className="picker-body">
          <div className="tab-section">
            <h4>Installed Voices</h4>
            <InstalledModels
              models={installed}
              activeModelPath={currentModelPath}
              onSelect={handleSelectModel}
              onDelete={deleteModel}
            />
          </div>

          {catalog && (
            <div className="tab-section">
              <h4>Available Voices</h4>
              <VoiceBrowser
                catalog={catalog}
                downloadedVoices={downloadedVoices}
                downloadingVoice={downloading}
                onDownload={downloadModel}
                onSelect={(voiceKey: string) => {
                  const model = installed.find((m) => m.voice_key === voiceKey);
                  if (model) handleSelectModel(model);
                }}
              />
            </div>
          )}
      </div>
    </div>
  );
}
