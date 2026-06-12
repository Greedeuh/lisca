import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ModelLoader() {
  const [modelPath, setModelPath] = useState("");
  const [voicePath, setVoicePath] = useState("");
  const [status, setStatus] = useState("");

  async function handleLoad() {
    if (!modelPath.trim() || !voicePath.trim()) return;
    try {
      await invoke("tts_load_model", { modelPath, voicePath });
      setStatus("Model loaded");
    } catch (err) {
      setStatus("Error: " + err);
    }
  }

  return (
    <section className="section">
      <h2>Kokoro Model</h2>
      <input
        type="text"
        placeholder="Path to .onnx model file"
        value={modelPath}
        onChange={(e) => setModelPath(e.target.value)}
      />
      <input
        type="text"
        placeholder="Path to voice .bin file"
        value={voicePath}
        onChange={(e) => setVoicePath(e.target.value)}
      />
      <button onClick={handleLoad} disabled={!modelPath.trim() || !voicePath.trim()}>
        Load Model
      </button>
      {status && <p className="status">{status}</p>}
    </section>
  );
}
