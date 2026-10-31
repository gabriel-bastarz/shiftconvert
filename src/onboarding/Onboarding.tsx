import { useCallback, useEffect, useRef, useState } from "react";

import type { OnboardingExitReason } from "../core/types";
import { ArrowLeftIcon, CloseIcon } from "./OnboardingIcons";
import { GestureStep } from "./steps/GestureStep";
import { OrbitStep } from "./steps/OrbitStep";
import { ReadyStep } from "./steps/ReadyStep";
import { SettingsStep } from "./steps/SettingsStep";
import { TryItStep } from "./steps/TryItStep";
import { WelcomeStep } from "./steps/WelcomeStep";

interface OnboardingProps {
  isFirstRun: boolean;
  resetToken: number;
  onExit: (reason: OnboardingExitReason) => void;
}

const STEP_META = [
  { label: "Boas-vindas", primary: "Começar" },
  { label: "O gesto", primary: "Continuar" },
  { label: "Orbit", primary: "Continuar" },
  { label: "Preferências", primary: "Continuar" },
  { label: "Experimente", primary: "Continuar" },
  { label: "Pronto", primary: "Começar a usar ShiftConvert" },
] as const;

const LAST_STEP = STEP_META.length - 1;

function StepContent({ step }: { step: number }) {
  switch (step) {
    case 0:
      return <WelcomeStep />;
    case 1:
      return <GestureStep />;
    case 2:
      return <OrbitStep />;
    case 3:
      return <SettingsStep />;
    case 4:
      return <TryItStep />;
    default:
      return <ReadyStep />;
  }
}

export function Onboarding({ isFirstRun, resetToken, onExit }: OnboardingProps) {
  const [step, setStep] = useState(0);
  const [direction, setDirection] = useState<1 | -1>(1);
  const [maxVisited, setMaxVisited] = useState(0);
  const [closing, setClosing] = useState(false);
  const shellRef = useRef<HTMLDivElement>(null);
  const exitTimer = useRef<number | null>(null);

  const clearExitTimer = useCallback(() => {
    if (exitTimer.current !== null) {
      window.clearTimeout(exitTimer.current);
      exitTimer.current = null;
    }
  }, []);

  const requestExit = useCallback(
    (reason: OnboardingExitReason) => {
      if (closing) return;
      setClosing(true);
      clearExitTimer();
      exitTimer.current = window.setTimeout(() => {
        exitTimer.current = null;
        onExit(reason);
      }, 190);
    },
    [clearExitTimer, closing, onExit]
  );

  useEffect(() => {
    clearExitTimer();
    setStep(0);
    setDirection(1);
    setMaxVisited(0);
    setClosing(false);
  }, [clearExitTimer, resetToken]);

  useEffect(() => {
    shellRef.current?.focus();
  }, [step]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      requestExit("dismiss");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [requestExit]);

  useEffect(() => () => clearExitTimer(), [clearExitTimer]);

  const goTo = (next: number) => {
    if (closing) return;
    const target = Math.max(0, Math.min(LAST_STEP, next));
    if (target === step) return;
    setDirection(target > step ? 1 : -1);
    setStep(target);
    setMaxVisited((current) => Math.max(current, target));
  };

  const primary = () => {
    if (step === LAST_STEP) {
      requestExit("complete");
      return;
    }
    goTo(step + 1);
  };

  const meta = STEP_META[step];

  return (
    <div className="onboarding-root">
      <section
        ref={shellRef}
        className={`onboarding-shell ${closing ? "is-closing" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="onboarding-title"
        tabIndex={-1}
      >
        <header className="onboarding-topbar">
          <div className="onboarding-brand">
            <img src="/favicon.png" alt="" className="onboarding-brand-logo" />
            <span className="onboarding-brand-name">ShiftConvert</span>
            {isFirstRun ? (
              <span className="onboarding-first-run">Primeira configuração</span>
            ) : null}
          </div>

          <div className="onboarding-topbar-right">
            <span className="onboarding-step-count">
              <strong>{String(step + 1).padStart(2, "0")}</strong>
              <span> / {String(STEP_META.length).padStart(2, "0")}</span>
            </span>
            {step < LAST_STEP ? (
              <button
                type="button"
                className="onboarding-skip"
                onClick={() => requestExit("skip")}
              >
                Pular configuração
              </button>
            ) : null}
            <button
              type="button"
              className="onboarding-close"
              onClick={() => requestExit("dismiss")}
              aria-label="Fechar guia"
              title="Fechar (Esc)"
            >
              <CloseIcon size={15} />
            </button>
          </div>
        </header>

        <main className="onboarding-main">
          <div className="onboarding-step-viewport" aria-live="polite">
            <div
              key={step}
              className={`onboarding-step-wrap ${
                direction > 0 ? "from-forward" : "from-back"
              }`}
            >
              <StepContent step={step} />
            </div>
          </div>
        </main>

        <footer className="onboarding-footer">
          <div className="onboarding-footer-side">
            <button
              type="button"
              className={`onboarding-back ${step === 0 ? "is-hidden" : ""}`}
              onClick={() => goTo(step - 1)}
              disabled={step === 0}
            >
              <ArrowLeftIcon size={14} />
              <span>Anterior</span>
            </button>
          </div>

          <div
            className="onboarding-progress"
            role="group"
            aria-label="Progresso do guia"
          >
            {STEP_META.map((item, index) => {
              const visited = index <= maxVisited;
              const active = index === step;
              const complete = index < step;
              return (
                <button
                  key={item.label}
                  type="button"
                  className={`onboarding-progress-segment ${
                    active ? "is-active" : complete ? "is-complete" : ""
                  } ${visited ? "is-visited" : ""}`}
                  disabled={!visited}
                  aria-label={`Etapa ${index + 1}: ${item.label}`}
                  aria-current={active ? "step" : undefined}
                  title={item.label}
                  onClick={() => goTo(index)}
                />
              );
            })}
          </div>

          <div className="onboarding-footer-side is-right">
            <button
              type="button"
              className="btn-primary onboarding-primary"
              onClick={primary}
            >
              {meta.primary}
            </button>
          </div>
        </footer>
      </section>
    </div>
  );
}
