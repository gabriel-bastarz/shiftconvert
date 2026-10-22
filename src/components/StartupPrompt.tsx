interface StartupPromptProps {
  message: string | null;
  onPick: () => void;
  onClose: () => void;
}

export function StartupPrompt({ message, onPick, onClose }: StartupPromptProps) {
  return (
    <div className="prompt-root">
      <section className="menu-card prompt-card" aria-label="ShiftConvert">
        <button
          type="button"
          className="icon-button prompt-close"
          onClick={onClose}
          aria-label="Fechar"
          title="Fechar (Esc)"
        >
          ×
        </button>

        <header className="prompt-brand">
          <span className="brand-orb" aria-hidden="true" />
          <span className="brand-name">ShiftConvert</span>
        </header>

        <h1 className="prompt-title">Converta sem sair do lugar.</h1>

        <div className="prompt-instruction">
          <span className="keycap">Shift</span>
          <span className="prompt-plus">+</span>
          <span className="prompt-drag" aria-hidden="true">
            <span className="drag-file" />
            <span className="drag-arrow">→</span>
          </span>
          <span className="prompt-instruction-text">arraste um arquivo do Explorer</span>
        </div>

        <p className="prompt-hint">
          A roda abre durante o arraste. Solte sobre o formato desejado — o
          original nunca é alterado.
        </p>

        {message ? (
          <p className="prompt-error" role="alert">
            {message}
          </p>
        ) : null}

        <div className="prompt-actions">
          <button type="button" className="btn-primary" onClick={onPick}>
            Escolher arquivo
          </button>
          <button type="button" className="btn-ghost" onClick={onClose}>
            Agora não
          </button>
        </div>

        <footer className="prompt-footer">
          <span>Imagens, texto, vídeo e ZIP</span>
          <span className="prompt-footer-dot" aria-hidden="true" />
          <span>Esc para fechar</span>
        </footer>
      </section>
    </div>
  );
}
