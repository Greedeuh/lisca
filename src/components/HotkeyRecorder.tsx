import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function HotkeyRecorder() {
  const [shortcut, setShortcut] = useState("");
  const [recording, setRecording] = useState(false);
  const [status, setStatus] = useState("");

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

  return (
    <section className="section">
      <h2>Hotkey</h2>
      <div className="row">
        <button onClick={() => { setRecording(!recording); setStatus(""); }}>
          {recording ? "Press keys..." : "Record Hotkey"}
        </button>
        {shortcut && <span className="badge">{shortcut}</span>}
      </div>
      {recording && <p className="hint">Press your key combination...</p>}
      {status && <p className="status">{status}</p>}
    </section>
  );
}
