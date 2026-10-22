import type { HudState } from "../core/types";

interface StatusHudProps {
  hud: HudState;
  onCancel: () => void;
  onClose: () => void;
  onReveal: () => void;
}

export function StatusHud({ hud, onCancel, onClose, onReveal }: StatusHudProps) {
  const inProgress = hud.status === "in-progress";
  const completed = hud.status === "completed";
  const failed = hud.status === "failed" || hud.status === "cancelled";
  const percent = Math.round(Math.max(0, Math.min(1, hud.progress)) * 100);

  return (
    <div className="hud-root">
      <section className={`menu-card hud-card hud-${hud.status}`} aria-live="polite">
        <header className="hud-header">
          <div className="hud-file">
            <span className="hud-file-name" title={hud.file.fileName}>
              {hud.file.fileName}
            </span>
            <span className="hud-arrow" aria-hidden="true">
              →
            </span>
            <span className="hud-target">{hud.target.label}</span>
          </div>
          <button
            type="button"
            className="icon-button hud-close"
            onClick={onClose}
            aria-label="Fechar"
            title="Fechar (Esc)"
          >
            ×
          </button>
        </header>

        <div className="hud-progress" role="progressbar" aria-valuenow={percent} aria-valuemin={0} aria-valuemax={100}>
          <div
            className={`hud-progress-fill ${inProgress ? "is-running" : ""}`}
            style={{ width: `${completed ? 100 : percent}%` }}
          />
        </div>

        <p className={`hud-message ${failed ? "is-error" : ""}`}>
          {failed ? hud.error ?? hud.message : hud.message}
        </p>

        {completed && hud.outputPath ? (
          <p className="hud-output" title={hud.outputPath}>
            {hud.outputPath.split(/[\\/]/).pop()}
          </p>
        ) : null}

        <footer className="hud-actions">
          {completed && hud.outputPath ? (
            <button type="button" className="btn-primary hud-button" onClick={onReveal}>
              Abrir pasta
            </button>
          ) : null}
          {inProgress ? (
            <button type="button" className="btn-ghost hud-button" onClick={onCancel}>
              Cancelar
            </button>
          ) : null}
          {!inProgress ? (
            <button type="button" className="btn-ghost hud-button" onClick={onClose}>
              Fechar
            </button>
          ) : null}
        </footer>
      </section>
    </div>
  );
}
