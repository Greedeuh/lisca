import { useState, useEffect, useCallback } from "react";
import { useToast } from "../../contexts/toast";
import { getHotkey, saveHotkey } from "../../types/ipc";
import type { ShortcutConfig } from "../../types/hotkey";
import "./HotkeyRecorder.css";

function formatShortcut(config: ShortcutConfig): string {
  return [...config.modifiers, config.key].join("+");
}

export function HotkeyRecorder() {
  const { addToast } = useToast();
  const [currentHotkey, setCurrentHotkey] = useState<ShortcutConfig | null>(null);
  const [recording, setRecording] = useState(false);
  const [pressed, setPressed] = useState<Set<string>>(new Set());
  const [display, setDisplay] = useState("Not set");

  useEffect(() => {
    getHotkey()
      .then((hotkey) => {
        setCurrentHotkey(hotkey);
        if (hotkey) setDisplay(formatShortcut(hotkey));
      })
      .catch((e) => addToast(`Failed to load hotkey: ${e}`));
  }, [addToast]);

  const handleSave = useCallback(
    async (shortcut: string) => {
      try {
        const config = await saveHotkey(shortcut);
        setCurrentHotkey(config);
        setDisplay(formatShortcut(config));
      } catch (e) {
        addToast(`Failed to save hotkey: ${e}`);
      }
    },
    [addToast],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!recording) return;
      e.preventDefault();
      e.stopPropagation();

      const key = e.key;
      if (key === "Escape") {
        setRecording(false);
        setPressed(new Set());
        if (currentHotkey) setDisplay(formatShortcut(currentHotkey));
        return;
      }

      const newPressed = new Set(pressed);

      if (["Control", "Alt", "Shift", "Meta"].includes(key)) {
        newPressed.add(key === "Meta" ? "Super" : key);
      } else {
        newPressed.add(key.length === 1 ? key.toUpperCase() : key);
        const shortcut = Array.from(newPressed).join("+");
        setDisplay(shortcut);
        setRecording(false);
        setPressed(new Set());
        handleSave(shortcut);
      }

      setPressed(newPressed);
    },
    [recording, pressed, currentHotkey, handleSave],
  );

  useEffect(() => {
    if (!recording) return;
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recording, handleKeyDown]);

  return (
    <div className="hr-container">
      <div className="hr-label">Global hotkey:</div>
      <div className="hr-display">
        <span className="hr-shortcut">{display}</span>
        <button
          className={`hr-btn ${recording ? "hr-btn-recording" : ""}`}
          onClick={() => {
            setRecording(!recording);
            if (!recording) setPressed(new Set());
          }}
        >
          {recording ? "Press keys..." : "Record"}
        </button>
      </div>
      {recording && (
        <div className="hr-hint">Press a key combination, or Esc to cancel.</div>
      )}
    </div>
  );
}
