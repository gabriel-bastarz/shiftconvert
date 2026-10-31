import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { StepHeader } from "../StepHeader";
import {
  FolderIcon,
  KeyIcon,
  PaletteIcon,
  WindowsIcon,
} from "../OnboardingIcons";

interface SettingRowProps {
  icon: ReactNode;
  title: string;
  description: string;
  value?: string;
  fixed?: boolean;
  control?: ReactNode;
}

function SettingRow({
  icon,
  title,
  description,
  value,
  fixed,
  control,
}: SettingRowProps) {
  return (
    <div className={`settings-row ${control ? "has-control" : ""}`}>
      <span className="settings-icon">{icon}</span>
      <span className="settings-copy">
        <span className="settings-title">{title}</span>
        <span className="settings-description">{description}</span>
      </span>
      <span className="settings-value">
        {value ? <span className="settings-value-text">{value}</span> : null}
        {fixed ? <span className="settings-fixed">Fixo</span> : null}
        {control}
      </span>
    </div>
  );
}

export function SettingsStep() {
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      try {
        const value = await invoke<boolean>("autostart_enabled");
        if (!disposed) setEnabled(value);
      } catch (reason) {
        if (!disposed) setError(String(reason));
      } finally {
        if (!disposed) setLoading(false);
      }

      const stop = await listen<boolean>("autostart-changed", (event) => {
        if (!disposed) {
          setEnabled(event.payload);
          setError(null);
        }
      });

      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    };

    void setup();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const toggle = async () => {
    if (loading || saving) return;
    const next = !enabled;
    setSaving(true);
    setError(null);
    try {
      const actual = await invoke<boolean>("set_autostart", { enabled: next });
      setEnabled(actual);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="onboarding-step">
      <StepHeader
        eyebrow="Preferências"
        title={
          <>
            Do seu <span className="onboarding-title-accent">jeito.</span>
          </>
        }
        description="Personalize o ShiftConvert para combinar com seu fluxo de trabalho."
      />

      <div className="settings-panel">
        <SettingRow
          icon={<PaletteIcon size={17} />}
          title="Tema"
          description="Interface escura, desenhada para uso rápido."
          value="Dark"
          fixed
        />
        <SettingRow
          icon={<WindowsIcon size={17} />}
          title="Iniciar com o Windows"
          description="Abre o ShiftConvert junto com o Windows, sem administrador."
          control={
            <button
              type="button"
              className={`settings-switch ${enabled ? "is-on" : ""} ${
                saving ? "is-saving" : ""
              }`}
              role="switch"
              aria-checked={enabled}
              aria-label="Iniciar com o Windows"
              disabled={loading || saving}
              onClick={() => void toggle()}
            >
              <span className="settings-switch-knob" />
            </button>
          }
        />
        <SettingRow
          icon={<FolderIcon size={17} />}
          title="Pasta de destino"
          description="O resultado fica ao lado do original e nunca o sobrescreve."
          value="Ao lado do arquivo"
          fixed
        />
        <SettingRow
          icon={<KeyIcon size={17} />}
          title="Atalho de ativação"
          description="Segure Shift enquanto arrasta um arquivo no Explorer."
          value="Shift + arraste"
          fixed
        />
      </div>

      {error ? (
        <p className="settings-error" role="alert">
          {error}
        </p>
      ) : (
        <p className="onboarding-footnote">
          A inicialização automática é salva imediatamente. Tema e atalho são
          fixos nesta versão.
        </p>
      )}
    </div>
  );
}
