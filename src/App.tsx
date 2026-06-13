import { HotkeyRecorder } from "./components/HotkeyRecorder";
import { ModelConfig } from "./components/ModelConfig";
import { TtsQueue } from "./components/TtsQueue";
import "./App.css";

function App() {
  return (
    <main className="container">
      <h1>Lisca - Text to Speech</h1>
      <HotkeyRecorder />
      <ModelConfig />
      <TtsQueue />
    </main>
  );
}

export default App;
