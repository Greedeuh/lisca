import { useCallback } from "react";
import { useToast } from "../../contexts/toast";
import { useTtsQueue } from "../../hooks";
import { queueToggleOverlay } from "../../types/ipc";
import { HotkeyRecorder } from "./HotkeyRecorder";
import "./SettingsTab.css";

export function SettingsTab() {
  const { addToast } = useToast();
  const { showOverlay, refresh } = useTtsQueue();

  const handleToggleOverlay = useCallback(async () => {
    try {
      await queueToggleOverlay();
      await refresh();
    } catch (e) {
      addToast(`Failed to toggle overlay: ${e}`);
    }
  }, [addToast, refresh]);

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
        <HotkeyRecorder />
      </section>
    </div>
  );
}
