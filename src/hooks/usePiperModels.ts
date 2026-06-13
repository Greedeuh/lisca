import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  VoiceCatalog,
  InstalledModel,
  DownloadProgress,
} from "../types/piper";

export function usePiperModels() {
  const [catalog, setCatalog] = useState<VoiceCatalog | null>(null);
  const [installed, setInstalled] = useState<InstalledModel[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<{
    bytes: number;
    total: number;
  } | null>(null);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgress>(
      "piper-download-progress",
      (event) => {
        const progress = event.payload;
        if (progress.type === "downloading") {
          setDownloading(progress.voice_key);
          setDownloadProgress({
            bytes: progress.bytes_downloaded,
            total: progress.total_bytes,
          });
        } else if (progress.type === "complete") {
          setDownloading(null);
          setDownloadProgress(null);
          loadInstalled();
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const fetchCatalog = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<VoiceCatalog>("piper_fetch_voices");
      setCatalog(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadInstalled = useCallback(async () => {
    try {
      const result = await invoke<InstalledModel[]>("piper_list_installed");
      setInstalled(result);
    } catch (err) {
      console.error("Failed to load installed models:", err);
    }
  }, []);

  const downloadModel = useCallback(async (voiceKey: string) => {
    setDownloading(voiceKey);
    setDownloadProgress(null);
    setError(null);
    try {
      await invoke<InstalledModel>("piper_download_model", { voiceKey });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setDownloading(null);
      setDownloadProgress(null);
    }
  }, []);

  const deleteModel = useCallback(async (voiceKey: string) => {
    try {
      await invoke("piper_delete_model", { voiceKey });
      setInstalled((prev) => prev.filter((m) => m.voice_key !== voiceKey));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  // Load installed models on mount
  useEffect(() => {
    loadInstalled();
  }, [loadInstalled]);

  return {
    catalog,
    installed,
    loading,
    error,
    downloading,
    downloadProgress,
    fetchCatalog,
    downloadModel,
    deleteModel,
  };
}
