import { HotkeyRecorder } from "./components/HotkeyRecorder";
import { SpeechTest } from "./components/SpeechTest";
import { ModelLoader } from "./components/ModelLoader";
import "./App.css";

function App() {
  return (
    <main className="container">
      <h1>Lisca - Text to Speech</h1>
      <HotkeyRecorder />
      <SpeechTest />
      <ModelLoader />
    </main>
  );
}

export default App;
