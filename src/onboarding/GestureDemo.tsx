import { useEffect, useMemo, useState } from "react";

import { FormatWheel } from "../components/FormatWheel";
import {
  GESTURE_DEMO_FILE,
  GESTURE_DEMO_TARGETS,
  buildDemoWheelPayload,
} from "./onboarding-demo";
import { DocumentIcon } from "./OnboardingIcons";
import { useElementSize } from "./useElementSize";
import { usePrefersReducedMotion } from "./usePrefersReducedMotion";

type Phase = 0 | 1 | 2 | 3;

const PHASE_LABELS = ["Arraste", "Segure Shift", "Escolha"];

function phaseLabelIndex(phase: Phase): number {
  if (phase <= 0) return 0;
  if (phase <= 2) return 1;
  return 2;
}

function Cursor() {
  return (
    <svg
      className="gesture-cursor"
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
    >
      <path
        d="M5.2 3.8 18.6 11l-6.1 1.2-2.8 5.8Z"
        fill="#f4f8fd"
        stroke="#0b1118"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function GestureDemo() {
  const reduced = usePrefersReducedMotion();
  const [phase, setPhase] = useState<Phase>(reduced ? 3 : 0);
  const [wheelRef, wheelSize] = useElementSize<HTMLDivElement>();
  const size = Math.min(wheelSize.width, wheelSize.height);
  const payload = useMemo(() => {
    if (size <= 0) return null;
    return buildDemoWheelPayload({
      file: GESTURE_DEMO_FILE,
      targets: GESTURE_DEMO_TARGETS,
      size,
      mode: "drag",
    });
  }, [size]);

  useEffect(() => {
    if (reduced) {
      setPhase(3);
      return;
    }

    let cancelled = false;
    const wait = (milliseconds: number) =>
      new Promise<void>((resolve) => {
        window.setTimeout(resolve, milliseconds);
      });

    const run = async () => {
      while (!cancelled) {
        setPhase(0);
        await wait(620);
        if (cancelled) return;
        setPhase(1);
        await wait(620);
        if (cancelled) return;
        setPhase(2);
        await wait(760);
        if (cancelled) return;
        setPhase(3);
        await wait(2350);
      }
    };

    void run();
    return () => {
      cancelled = true;
    };
  }, [reduced]);

  const activeLabel = phaseLabelIndex(phase);

  return (
    <div
      className={`gesture-stage phase-${phase}`}
      role="img"
      aria-label="Demonstração: segure Shift, arraste um arquivo e escolha um formato no Orbit."
    >
      <div className="gesture-ambient" aria-hidden="true" />
      <div className="gesture-trail" aria-hidden="true" />

      <div className={`gesture-shift ${phase >= 1 ? "is-pressed" : ""}`}>
        <span className="gesture-shift-label">Shift</span>
        <span className="gesture-shift-glow" aria-hidden="true" />
      </div>

      <div className="gesture-actor" aria-hidden="true">
        <div className="gesture-file">
          <span className="gesture-file-icon">
            <DocumentIcon size={17} />
          </span>
          <span className="gesture-file-copy">
            <span className="gesture-file-name">relatório.docx</span>
            <span className="gesture-file-size">284 KB</span>
          </span>
        </div>
        <Cursor />
      </div>

      <div className="gesture-wheel-frame" ref={wheelRef} aria-hidden="true">
        {payload ? (
          <FormatWheel
            payload={payload}
            hovered={null}
            onHover={() => undefined}
            onSelect={() => undefined}
          />
        ) : null}
      </div>

      <div className="gesture-steps" aria-hidden="true">
        {PHASE_LABELS.map((label, index) => (
          <span key={label} className="gesture-step-wrap">
            {index > 0 ? <span className="gesture-step-separator" /> : null}
            <span
              className={`gesture-step ${
                index === activeLabel ? "is-active" : ""
              }`}
            >
              {label}
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}
