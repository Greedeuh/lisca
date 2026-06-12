import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ModelLoader() {
  const [modelPath, setModelPath] = useState("");
  const [status, setStatus] = useState("");

  async function handleLoad() {
    if (!modelPath.trim()) return;
    try {
      await invoke("tts_load_model", { modelPath });
      setStatus("Model loaded: " + modelPath);
    } catch (err) {
      setStatus("Error: " + err);
    }
  }

  return (
    <section className="section">
      <h2>ONNX Model (optional)</h2>
      <input
        type="text"
        placeholder="Path to .onnx model file"
        value={modelPath}
        onChange={(e) => setModelPath(e.target.value)}
      />
      <button onClick={handleLoad} disabled={!modelPath.trim()}>
        Load Model
      </button>
      {status && <p className="status">{status}</p>}
    </section>
  );
}
