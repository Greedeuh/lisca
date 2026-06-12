import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [shortcut, setShortcut] = useState("");
  const [recording, setRecording] = useState(false);
  const [status, setStatus] = useState("");

  const [rate, setRate] = useState(200);
  const [volume, setVolume] = useState(100);
  const [voices, setVoices] = useState<string[]>([]);
  const [voice, setVoice] = useState("");
  const [testText, setTestText] = useState("");
  const [speaking, setSpeaking] = useState(false);

  useEffect(() => {
    invoke<string | null>("hotkey_get").then((saved) => {
      if (saved) {
        setShortcut(saved);
        invoke("hotkey_set", { shortcut: saved }).catch(console.error);
      }
    });
    invoke<string[]>("tts_list_voices").then((v) => {
      setVoices(v);
      if (v.length > 0 && !voice) setVoice(v[0]);
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

  async function handleSaveConfig() {
    try {
      await invoke("tts_update_config", { rate, volume, voice });
      setStatus("Config saved");
    } catch (err) {
      setStatus("Error: " + err);
    }
  }

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
        <h2>Voice</h2>
        <select value={voice} onChange={(e) => setVoice(e.target.value)}>
          {voices.map((v) => (
            <option key={v} value={v}>{v}</option>
          ))}
        </select>
        <label className="slider-label">
          Rate: {rate}
          <input type="range" min="50" max="400" value={rate}
            onChange={(e) => setRate(Number(e.target.value))} />
        </label>
        <label className="slider-label">
          Volume: {volume}
          <input type="range" min="0" max="100" value={volume}
            onChange={(e) => setVolume(Number(e.target.value))} />
        </label>
        <button onClick={handleSaveConfig}>Save Config</button>
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

      {status && <p className="status">{status}</p>}
    </main>
  );
}

export default App;
