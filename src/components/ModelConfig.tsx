import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PiperModelPicker } from "./PiperModelPicker";
import { VoiceMappingSettings } from "./VoiceMappingSettings";
import type { InstalledModel } from "../types/piper";

const STATUS_DURATION = 2000;

interface PiperConfig {
  type: "piper";
  model_path: string;
  config_path: string;
}

type BackendConfig = { type: string; model_path: string; config_path?: string; voice_path?: string };

export function ModelConfig() {
  const [config, setConfig] = useState<BackendConfig | null>(null);
  const [backendType, setBackendType] = useState<"kokoro" | "piper">("piper");
  const [modelPath, setModelPath] = useState("");
  const [status, setStatus] = useState("");
  const [installedModels, setInstalledModels] = useState<InstalledModel[]>([]);
  const statusTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<BackendConfig>("tts_get_config")
      .then((cfg) => {
        setConfig(cfg);
        setBackendType(cfg.type as "kokoro" | "piper");
        setModelPath(cfg.model_path);
      })
      .catch((err) => setStatus("Failed to load config: " + err));

    invoke<InstalledModel[]>("piper_list_installed")
      .then(setInstalledModels)
      .catch(console.error);
  }, []);

  useEffect(() => {
    return () => {
      if (statusTimeout.current) clearTimeout(statusTimeout.current);
    };
  }, []);

  if (!config) return null;

  return (
    <section className="section">
      <h2>Model</h2>
      <div className="field">
        <label>Backend</label>
        <select
          value={backendType}
          onChange={(e) => setBackendType(e.target.value as "kokoro" | "piper")}
        >
          <option value="piper">Piper</option>
          <option value="kokoro">Kokoro</option>
        </select>
      </div>

      {backendType === "piper" && (
        <PiperModelPicker
          currentModelPath={modelPath}
          onSelectModel={(newModelPath, newConfigPath) => {
            setModelPath(newModelPath);
            const newConfig: PiperConfig = {
              type: "piper",
              model_path: newModelPath,
              config_path: newConfigPath,
            };
            invoke("tts_set_config", { config: newConfig })
              .then(() => {
                setConfig(newConfig);
                setStatus("Model activated");
                invoke<InstalledModel[]>("piper_list_installed")
                  .then(setInstalledModels)
                  .catch(console.error);
              })
              .catch((err) => setStatus("Error: " + err))
              .finally(() => {
                if (statusTimeout.current) clearTimeout(statusTimeout.current);
                statusTimeout.current = setTimeout(() => setStatus(""), STATUS_DURATION);
              });
          }}
        />
      )}

      {backendType === "piper" && installedModels.length > 0 && (
        <VoiceMappingSettings installedModels={installedModels} />
      )}

      {status && <p className="status">{status}</p>}
    </section>
  );
}
