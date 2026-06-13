import type { InstalledModel } from "../types/piper";

interface InstalledModelsProps {
  models: InstalledModel[];
  activeModelPath: string | null;
  onSelect: (model: InstalledModel) => void;
  onDelete: (voiceKey: string) => void;
}

export function InstalledModels({
  models,
  activeModelPath,
  onSelect,
  onDelete,
}: InstalledModelsProps) {
  if (models.length === 0) {
    return (
      <div className="installed-empty">
        <p>No models installed yet.</p>
        <p className="hint">Browse the catalog to download voices.</p>
      </div>
    );
  }

  return (
    <div className="installed-models">
      {models.map((model) => {
        const isActive = model.model_path === activeModelPath;
        return (
          <div
            key={model.voice_key}
            className={`installed-row ${isActive ? "active" : ""}`}
          >
            <div className="installed-info">
              <span className="installed-name">{model.name}</span>
              <span className={`quality-badge quality-${model.quality}`}>
                {model.quality}
              </span>
              <span className="installed-language">
                {model.language.code}
              </span>
              {isActive && <span className="active-badge">Active</span>}
            </div>
            <div className="installed-actions">
              {!isActive && (
                <button
                  className="use-button"
                  onClick={() => onSelect(model)}
                >
                  Use
                </button>
              )}
              <button
                className="delete-button"
                onClick={() => onDelete(model.voice_key)}
              >
                Delete
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
