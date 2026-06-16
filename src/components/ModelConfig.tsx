import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PiperModelPicker } from "./PiperModelPicker";
import { VoiceMappingSettings } from "./VoiceMappingSettings";
import type { InstalledModel, BackendType } from "../types/piper";

export function ModelConfig() {
  const [backend, setBackend] = useState<BackendType>("piper");
  const [modelPath, setModelPath] = useState("");
  const [status, setStatus] = useState("");
  const [installedModels, setInstalledModels] = useState<InstalledModel[]>([]);

  useEffect(() => {
    invoke<{ type: string }>("tts_get_config")
      .then((cfg) => setBackend(cfg.type as BackendType))
      .catch((err) => setStatus("Failed to load config: " + err));

    invoke<InstalledModel[]>("piper_list_installed")
      .then(setInstalledModels)
      .catch(console.error);
  }, []);

  const handleBackendChange = async (newBackend: BackendType) => {
    setStatus("");
    try {
      await invoke("tts_set_backend_type", { backend: newBackend });
      setBackend(newBackend);
      if (newBackend === "piper") {
        const cfg = await invoke<{ model_path: string }>("tts_get_config");
        setModelPath(cfg.model_path);
      }
    } catch (err) {
      setStatus("Failed to switch backend: " + err);
    }
  };

  return (
    <section className="section">
      <h2>Model</h2>

      <div className="backend-toggle">
        <label className="backend-option">
          <input
            type="radio"
            name="backend"
            value="piper"
            checked={backend === "piper"}
            onChange={() => handleBackendChange("piper")}
          />
          <span>Piper</span>
        </label>
        <label className="backend-option">
          <input
            type="radio"
            name="backend"
            value="kokoro"
            checked={backend === "kokoro"}
            onChange={() => handleBackendChange("kokoro")}
          />
          <span>Kokoro</span>
        </label>
      </div>

      {backend === "piper" && (
        <>
          <PiperModelPicker currentModelPath={modelPath} />
          {installedModels.length > 0 && (
            <VoiceMappingSettings installedModels={installedModels} />
          )}
        </>
      )}

      {backend === "kokoro" && (
        <p className="status">Kokoro backend active</p>
      )}

      {status && <p className="status">{status}</p>}
    </section>
  );
}
