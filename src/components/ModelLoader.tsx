import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ModelLoader() {
  const [modelPath, setModelPath] = useState("models/kokoro-q8.onnx");
  const [voicePath, setVoicePath] = useState("models/voices/af.bin");
  const [status, setStatus] = useState("");
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    invoke<boolean>("tts_model_loaded").then((isLoaded) => {
      setLoaded(isLoaded);
      if (isLoaded) setStatus("Auto-loaded");
    }).catch(() => {});
  }, []);

  async function handleLoad() {
    if (!modelPath.trim() || !voicePath.trim()) return;
    setStatus("Loading...");
    try {
      await invoke("tts_load_model", { modelPath, voicePath });
      setStatus("Loaded");
      setLoaded(true);
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
        {loaded ? "Reload" : "Load"}
      </button>
      {status && <p className="status">{status}</p>}
    </section>
  );
}
