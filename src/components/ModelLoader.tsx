import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ModelLoader() {
  const [modelPath, setModelPath] = useState("models/kokoro-q8.onnx");
  const [voicePath, setVoicePath] = useState("models/voices/af.bin");
  const [status, setStatus] = useState("");
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    // Check if model was auto-loaded
    invoke<boolean>("tts_model_loaded").then(setLoaded).catch(() => {});
  }, []);

  async function handleLoad() {
    if (!modelPath.trim() || !voicePath.trim()) return;
    try {
      await invoke("tts_load_model", { modelPath, voicePath });
      setStatus("Model loaded");
      setLoaded(true);
    } catch (err) {
      setStatus("Error: " + err);
    }
  }

  return (
    <section className="section">
      <h2>Kokoro Model</h2>
      {loaded && <p className="hint">Model loaded</p>}
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
        {loaded ? "Reload" : "Load Model"}
      </button>
      {status && <p className="status">{status}</p>}
    </section>
  );
}
