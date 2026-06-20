import { useState } from "react";
import { ToastProvider } from "./contexts/toast";
import { VoiceBrowser, InstalledVoices } from "./components/voices";
import { QueueList } from "./components/queue";
import { SettingsTab } from "./components/settings";
import "./App.css";

type Tab = "voices" | "queue" | "settings";

function App() {
  const [tab, setTab] = useState<Tab>("voices");

  return (
    <ToastProvider>
      <main className="app-container">
        <header className="app-header">
          <h1 className="app-title">Lisca</h1>
          <nav className="app-tabs">
            <button
              className={`app-tab ${tab === "voices" ? "app-tab-active" : ""}`}
              onClick={() => setTab("voices")}
            >
              Voices
            </button>
            <button
              className={`app-tab ${tab === "queue" ? "app-tab-active" : ""}`}
              onClick={() => setTab("queue")}
            >
              Queue
            </button>
            <button
              className={`app-tab ${tab === "settings" ? "app-tab-active" : ""}`}
              onClick={() => setTab("settings")}
            >
              Settings
            </button>
          </nav>
        </header>

        <div className="app-content">
          {tab === "voices" && (
            <div className="app-voices">
              <section className="app-section">
                <h2 className="app-section-title">Available Voices</h2>
                <VoiceBrowser />
              </section>
              <section className="app-section">
                <h2 className="app-section-title">Installed Voices</h2>
                <InstalledVoices />
              </section>
            </div>
          )}

          {tab === "queue" && (
            <section className="app-section">
              <QueueList />
            </section>
          )}

          {tab === "settings" && <SettingsTab />}
        </div>
      </main>
    </ToastProvider>
  );
}

export default App;
