import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function SpeechTest() {
  const [text, setText] = useState("");
  const [speaking, setSpeaking] = useState(false);
  const [status, setStatus] = useState("");

  async function handleSpeak() {
    if (!text.trim()) return;
    setSpeaking(true);
    try {
      await invoke("tts_speak", { text });
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
    <section className="section">
      <h2>Test</h2>
      <textarea
        placeholder="Type text to speak..."
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={3}
      />
      <div className="row">
        <button onClick={handleSpeak} disabled={speaking || !text.trim()}>
          {speaking ? "Speaking..." : "Speak"}
        </button>
        {speaking && <button onClick={handleStop}>Stop</button>}
      </div>
      {status && <p className="status">{status}</p>}
    </section>
  );
}
