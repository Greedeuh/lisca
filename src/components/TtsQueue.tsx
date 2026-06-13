import { useTtsQueue } from "../hooks/useTtsQueue";
import { QueueControls } from "./QueueControls";
import { QueueList } from "./QueueList";

export function TtsQueue() {
  const {
    items,
    current,
    playback,
    autoRead,
    remove,
    moveItem,
    clear,
    pause,
    resume,
    stop,
    toggleAutoRead,
  } = useTtsQueue();

  return (
    <section className="section queue-section">
      <h3>Queue</h3>
      <QueueControls
        playback={playback}
        autoRead={autoRead}
        onToggleAutoRead={toggleAutoRead}
        onPause={pause}
        onResume={resume}
        onStop={stop}
        onClear={clear}
        disabled={items.length === 0 && !current}
      />
      <QueueList
        items={items}
        current={current}
        playback={playback}
        onRemove={remove}
        onMove={moveItem}
      />
    </section>
  );
}
