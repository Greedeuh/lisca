import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Super"]);

export function HotkeyRecorder() {
  const [hotkey, setHotkey] = useState("");
  const [recording, setRecording] = useState(false);
  const [status, setStatus] = useState("");

  useEffect(() => {
    invoke<string | null>("hotkey_get").then((saved) => {
      if (saved) {
        setHotkey(saved);
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
      if (!MODIFIER_KEYS.has(key)) {
        parts.push(key.length === 1 ? key.toUpperCase() : key);
        const combo = parts.join("+");
        setHotkey(combo);
        setRecording(false);
        invoke("hotkey_set", { shortcut: combo })
          .then(() => setStatus("Saved: " + combo))
          .catch((err) => setStatus("Error: " + err));
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [recording]);

  return (
    <section className="section">
      <h2>Hotkey</h2>
      <div className="row">
        <button onClick={() => { setRecording(!recording); setStatus(""); }}>
          {recording ? "Press keys..." : "Record Hotkey"}
        </button>
        {hotkey && <span className="badge">{hotkey}</span>}
      </div>
      {recording && <p className="hint">Press your key combination...</p>}
      {status && <p className="status">{status}</p>}
    </section>
  );
}
