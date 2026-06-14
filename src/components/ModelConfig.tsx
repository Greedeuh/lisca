import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PiperModelPicker } from "./PiperModelPicker";
import { VoiceMappingSettings } from "./VoiceMappingSettings";
import type { InstalledModel } from "../types/piper";

const STATUS_DURATION = 2000;

interface KokoroConfig {
  type: "kokoro";
  model_path: string;
  voice_path: string;
}

interface PiperConfig {
  type: "piper";
  model_path: string;
  config_path: string;
}

type BackendConfig = KokoroConfig | PiperConfig;

export function ModelConfig() {
  const [config, setConfig] = useState<BackendConfig | null>(null);
  const [backendType, setBackendType] = useState<"kokoro" | "piper">("piper");
  const [modelPath, setModelPath] = useState("");
  const [voicePath, setVoicePath] = useState("");
  const [configPath, setConfigPath] = useState("");
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState(true);
  const [installedModels, setInstalledModels] = useState<InstalledModel[]>([]);
  const statusTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<BackendConfig>("tts_get_config")
      .then((cfg) => {
        setConfig(cfg);
        setBackendType(cfg.type);
        if (cfg.type === "kokoro") {
          setModelPath(cfg.model_path);
          setVoicePath(cfg.voice_path);
        } else if (cfg.type === "piper") {
          setModelPath(cfg.model_path);
          setConfigPath(cfg.config_path);
        }
      })
      .catch((err) => setStatus("Failed to load config: " + err))
      .finally(() => setLoading(false));

    invoke<InstalledModel[]>("piper_list_installed")
      .then(setInstalledModels)
      .catch(console.error);
  }, []);

  useEffect(() => {
    return () => {
      if (statusTimeout.current) clearTimeout(statusTimeout.current);
    };
  }, []);

  function handleSave() {
    if (!config) return;

    let newConfig: BackendConfig;
    if (backendType === "kokoro") {
      newConfig = {
        type: "kokoro",
        model_path: modelPath,
        voice_path: voicePath,
      };
    } else {
      newConfig = {
        type: "piper",
        model_path: modelPath,
        config_path: configPath,
      };
    }

    setLoading(true);
    invoke("tts_set_config", { config: newConfig })
      .then(() => {
        setConfig(newConfig);
        setStatus("Saved & backend reloaded");
        if (statusTimeout.current) clearTimeout(statusTimeout.current);
        statusTimeout.current = setTimeout(() => setStatus(""), STATUS_DURATION);
      })
      .catch((err) => setStatus("Error: " + err))
      .finally(() => setLoading(false));
  }

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
            setConfigPath(newConfigPath);
            setLoading(true);
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
                setLoading(false);
                if (statusTimeout.current) clearTimeout(statusTimeout.current);
                statusTimeout.current = setTimeout(() => setStatus(""), STATUS_DURATION);
              });
          }}
        />
      )}

      {backendType === "piper" && installedModels.length > 0 && (
        <VoiceMappingSettings installedModels={installedModels} />
      )}

      <details className="advanced-section">
        <summary>Advanced (Manual Path Configuration)</summary>
        <p className="hint">Relative paths resolve from the app resource directory.</p>
        <div className="field">
          <label>Model Path</label>
          <input
            type="text"
            value={modelPath}
            onChange={(e) => setModelPath(e.target.value)}
            placeholder={
              backendType === "piper"
                ? "models/en_US-lessac-medium.onnx"
                : "models/kokoro-q8.onnx"
            }
          />
        </div>
        {backendType === "kokoro" ? (
          <div className="field">
            <label>Voice Path</label>
            <input
              type="text"
              value={voicePath}
              onChange={(e) => setVoicePath(e.target.value)}
              placeholder="models/voices/af.bin"
            />
          </div>
        ) : (
          <div className="field">
            <label>Config Path</label>
            <input
              type="text"
              value={configPath}
              onChange={(e) => setConfigPath(e.target.value)}
              placeholder="models/en_US-lessac-medium.onnx.json"
            />
          </div>
        )}
        <div className="row">
          <button onClick={handleSave} disabled={loading}>
            {loading ? "Loading..." : "Save & Reload"}
          </button>
          <button onClick={() => invoke("tts_open_resource_dir")} className="secondary">
            Open Folder
          </button>
        </div>
      </details>

      {status && <p className="status">{status}</p>}
    </section>
  );
}
