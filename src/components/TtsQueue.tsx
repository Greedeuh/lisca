import { useTtsQueue } from "../hooks/useTtsQueue";
import { QueueControls } from "./QueueControls";
import { QueueList } from "./QueueList";

export function TtsQueue() {
  const {
    items,
    playback,
    autoRead,
    showOverlay,
    remove,
    moveItem,
    clear,
    pause,
    resume,
    stop,
    toggleAutoRead,
    toggleShowOverlay,
  } = useTtsQueue();

  return (
    <section className="section queue-section">
      <h3>Queue</h3>
      <QueueControls
        playback={playback}
        autoRead={autoRead}
        showOverlay={showOverlay}
        onToggleAutoRead={toggleAutoRead}
        onToggleShowOverlay={toggleShowOverlay}
        onPause={pause}
        onResume={resume}
        onStop={stop}
        onClear={clear}
        disabled={items.length === 0}
      />
      <QueueList
        items={items}
        onRemove={remove}
        onMove={moveItem}
      />
    </section>
  );
}
