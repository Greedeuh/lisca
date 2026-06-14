import { useEffect, useMemo } from "react";
import { usePiperModels } from "../hooks/usePiperModels";
import { VoiceBrowser } from "./VoiceBrowser";
import { InstalledModels } from "./InstalledModels";
import { DownloadProgress } from "./DownloadProgress";

interface PiperModelPickerProps {
  currentModelPath: string | null;
}

export function PiperModelPicker({
  currentModelPath,
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

  useEffect(() => {
    if (!catalog && !loading && !error) {
      fetchCatalog();
    }
  }, [catalog, loading, error, fetchCatalog]);

  const downloadedVoices = useMemo(
    () => new Set(installed.map((m) => m.voice_key)),
    [installed]
  );

  return (
    <div className="piper-model-picker">
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
              />
            </div>
          )}
      </div>
    </div>
  );
}
