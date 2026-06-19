import { useState, useEffect, useCallback } from "react";
import type { ShortcutConfig } from "../../types/hotkey";
import "./HotkeyRecorder.css";

interface HotkeyRecorderProps {
  currentHotkey: ShortcutConfig | null;
  onSave: (shortcut: string) => void;
}

function formatShortcut(config: ShortcutConfig): string {
  return [...config.modifiers, config.key].join("+");
}

export function HotkeyRecorder({ currentHotkey, onSave }: HotkeyRecorderProps) {
  const [recording, setRecording] = useState(false);
  const [pressed, setPressed] = useState<Set<string>>(new Set());
  const [display, setDisplay] = useState(
    currentHotkey ? formatShortcut(currentHotkey) : "Not set"
  );

  useEffect(() => {
    if (currentHotkey) {
      setDisplay(formatShortcut(currentHotkey));
    }
  }, [currentHotkey]);

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
        onSave(shortcut);
      }

      setPressed(newPressed);
    },
    [recording, pressed, currentHotkey, onSave],
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
