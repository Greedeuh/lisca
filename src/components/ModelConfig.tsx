import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PiperModelPicker } from "./PiperModelPicker";
import { VoiceMappingSettings } from "./VoiceMappingSettings";
import type { InstalledModel } from "../types/piper";

export function ModelConfig() {
  const [modelPath, setModelPath] = useState("");
  const [status, setStatus] = useState("");
  const [installedModels, setInstalledModels] = useState<InstalledModel[]>([]);

  useEffect(() => {
    invoke<{ model_path: string }>("tts_get_config")
      .then((cfg) => setModelPath(cfg.model_path))
      .catch((err) => setStatus("Failed to load config: " + err));

    invoke<InstalledModel[]>("piper_list_installed")
      .then(setInstalledModels)
      .catch(console.error);
  }, []);

  return (
    <section className="section">
      <h2>Model</h2>

      <PiperModelPicker currentModelPath={modelPath} />

      {installedModels.length > 0 && (
        <VoiceMappingSettings installedModels={installedModels} />
      )}

      {status && <p className="status">{status}</p>}
    </section>
  );
}
