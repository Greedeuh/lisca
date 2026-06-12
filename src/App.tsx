import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [shortcut, setShortcut] = useState("");
  const [recording, setRecording] = useState(false);
  const [status, setStatus] = useState("");
  const [testText, setTestText] = useState("");
  const [speaking, setSpeaking] = useState(false);
  const [modelPath, setModelPath] = useState("");

  useEffect(() => {
    invoke<string | null>("hotkey_get").then((saved) => {
      if (saved) {
        setShortcut(saved);
        invoke("hotkey_set", { shortcut: saved }).catch(console.error);
      }
    });
  }, []);

  useEffect(() => {
    if (!recording) return;

    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("Control");
      if (e.shiftKey) parts.push("Shift");
      if (e.altKey) parts.push("Alt");
      if (e.metaKey) parts.push("Super");

      const key = e.key;
      if (!["Control", "Shift", "Alt", "Super"].includes(key)) {
        parts.push(key.length === 1 ? key.toUpperCase() : key);
        const combo = parts.join("+");
        setShortcut(combo);
        setRecording(false);
        invoke("hotkey_set", { shortcut: combo })
          .then(() => setStatus("Saved: " + combo))
          .catch((err) => setStatus("Error: " + err));
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [recording]);

  async function handleTestSpeak() {
    if (!testText.trim()) return;
    setSpeaking(true);
    try {
      await invoke("tts_speak", { text: testText });
    } catch (err) {
      setStatus("Error: " + err);
    } finally {
      setSpeaking(false);
    }
  }

  async function handleStop() {
    await invoke("tts_stop").catch(() => {});
    setSpeaking(false);
  }

  async function handleLoadModel() {
    if (!modelPath.trim()) return;
    try {
      await invoke("tts_load_model", { modelPath });
      setStatus("Model loaded: " + modelPath);
    } catch (err) {
      setStatus("Error: " + err);
    }
  }

  return (
    <main className="container">
      <h1>Lisca - Text to Speech</h1>

      <section className="section">
        <h2>Hotkey</h2>
        <div className="row">
          <button onClick={() => { setRecording(!recording); setStatus(""); }}>
            {recording ? "Press keys..." : "Record Hotkey"}
          </button>
          {shortcut && <span className="badge">{shortcut}</span>}
        </div>
        {recording && <p className="hint">Press your key combination...</p>}
      </section>

      <section className="section">
        <h2>Test</h2>
        <textarea
          placeholder="Type text to speak..."
          value={testText}
          onChange={(e) => setTestText(e.target.value)}
          rows={3}
        />
        <div className="row">
          <button onClick={handleTestSpeak} disabled={speaking || !testText.trim()}>
            {speaking ? "Speaking..." : "Speak"}
          </button>
          {speaking && <button onClick={handleStop}>Stop</button>}
        </div>
      </section>

      <section className="section">
        <h2>ONNX Model (optional)</h2>
        <input
          type="text"
          placeholder="Path to .onnx model file"
          value={modelPath}
          onChange={(e) => setModelPath(e.target.value)}
        />
        <button onClick={handleLoadModel} disabled={!modelPath.trim()}>
          Load Model
        </button>
      </section>

      {status && <p className="status">{status}</p>}
    </main>
  );
}

export default App;
