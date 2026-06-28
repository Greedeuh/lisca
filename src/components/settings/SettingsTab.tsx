import { useCallback, useEffect, useState } from "react";
import { useToast } from "../../contexts/toast";
import { useTtsQueue } from "../../hooks";
import {
  queueToggleOverlay,
  getIdleTimeout,
  setIdleTimeout,
} from "../../types/ipc";
import { HotkeyRecorder } from "./HotkeyRecorder";
import "./SettingsTab.css";

export function SettingsTab() {
  const { addToast } = useToast();
  const { showOverlay, refresh } = useTtsQueue();
  const [idleTimeoutMinutes, setIdleTimeoutMinutes] = useState(5);

  useEffect(() => {
    getIdleTimeout()
      .then((secs) => setIdleTimeoutMinutes(Math.round(secs / 60)))
      .catch(() => {});
  }, []);

  const handleToggleOverlay = useCallback(async () => {
    try {
      await queueToggleOverlay();
      await refresh();
    } catch (e) {
      addToast(`Failed to toggle overlay: ${e}`);
    }
  }, [addToast, refresh]);

  const handleIdleTimeoutChange = useCallback(
    async (minutes: number) => {
      const clamped = Math.max(0, Math.min(60, minutes));
      setIdleTimeoutMinutes(clamped);
      try {
        await setIdleTimeout(clamped * 60);
      } catch (e) {
        addToast(`Failed to set idle timeout: ${e}`);
      }
    },
    [addToast],
  );

  return (
    <div className="app-settings">
      <section className="app-section">
        <h2 className="app-section-title">Overlay</h2>
        <label className="app-setting-row">
          <input
            type="checkbox"
            checked={showOverlay}
            onChange={handleToggleOverlay}
          />
          Show overlay when main window is closed
        </label>
      </section>
      <section className="app-section">
        <h2 className="app-section-title">Model Unload</h2>
        <label className="app-setting-row">
          Unload idle models after
          <input
            type="number"
            min={0}
            max={60}
            value={idleTimeoutMinutes}
            onChange={(e) => handleIdleTimeoutChange(Number(e.target.value))}
          />
          minutes
        </label>
      </section>
      <section className="app-section">
        <HotkeyRecorder />
      </section>
    </div>
  );
}
