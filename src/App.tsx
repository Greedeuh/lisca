import { HotkeyRecorder } from "./components/HotkeyRecorder";
import { ModelConfig } from "./components/ModelConfig";
import "./App.css";

function App() {
  return (
    <main className="container">
      <h1>Lisca - Text to Speech</h1>
      <HotkeyRecorder />
      <ModelConfig />
    </main>
  );
}

export default App;
