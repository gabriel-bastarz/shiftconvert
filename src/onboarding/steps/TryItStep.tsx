import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { FormatWheel } from "../../components/FormatWheel";
import type {
  ConversionProgress,
  FileInfo,
  OnboardingFileInfo,
  TargetFormat,
} from "../../core/types";
import { StepHeader } from "../StepHeader";
import {
  CheckIcon,
  FileIcon,
  FolderIcon,
  SparkIcon,
} from "../OnboardingIcons";
import { buildDemoWheelPayload } from "../onboarding-demo";
import { useElementSize } from "../useElementSize";

type Phase =
  | "idle"
  | "picking"
  | "selected"
  | "converting"
  | "completed"
  | "failed";

interface ActiveConversion {
  sourcePath: string;
  targetFormat: string;
  id: string | null;
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 10 || unit === 0 ? 0 : 1).replace(".", ",")} ${
    units[unit]
  }`;
}

function fileName(path: string | null): string {
  if (!path) return "";
  return path.split(/[\\/]/).pop() ?? path;
}

export function TryItStep() {
  const [phase, setPhase] = useState<Phase>("idle");
  const [file, setFile] = useState<FileInfo | null>(null);
  const [targets, setTargets] = useState<TargetFormat[]>([]);
  const [selected, setSelected] = useState<TargetFormat | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const [progress, setProgress] = useState(0);
  const [message, setMessage] = useState("");
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [wheelRef, wheelSize] = useElementSize<HTMLDivElement>();

  const activeRef = useRef<ActiveConversion | null>(null);
  const cancelRequested = useRef(false);
  const startingRef = useRef(false);

  const size = Math.min(wheelSize.width, wheelSize.height);
  const payload = useMemo(() => {
    if (!file || size <= 0) return null;
    return buildDemoWheelPayload({
      file,
      targets,
      size,
      mode: phase === "converting" ? "drag" : "picker",
    });
  }, [file, targets, size, phase]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<Promise<UnlistenFn>> = [];

    const handle = (update: ConversionProgress) => {
      if (disposed) return;
      const active = activeRef.current;
      if (!active) return;
      if (update.sourcePath !== active.sourcePath) return;
      if (update.targetFormat !== active.targetFormat) return;
      if (active.id && active.id !== update.conversionId) return;

      active.id = update.conversionId;
      setProgress(update.progress);
      setMessage(update.message);

      if (update.status === "completed") {
        activeRef.current = null;
        setPhase("completed");
        setOutputPath(update.outputPath);
        setError(null);
      } else if (update.status === "failed" || update.status === "cancelled") {
        activeRef.current = null;
        setPhase("failed");
        setError(update.error ?? update.message);
      }
    };

    for (const name of [
      "onboarding-conversion-progress",
      "onboarding-conversion-complete",
      "onboarding-conversion-error",
    ]) {
      unlisteners.push(
        listen<ConversionProgress>(name, (event) => handle(event.payload))
      );
    }

    return () => {
      disposed = true;
      unlisteners.forEach((promise) => {
        promise.then((unlisten) => unlisten()).catch(() => undefined);
      });
      const active = activeRef.current;
      if (active?.id) {
        void invoke("cancel_conversion", { conversionId: active.id });
      }
      activeRef.current = null;
    };
  }, []);

  const cancelActive = () => {
    const active = activeRef.current;
    if (!active) return;
    cancelRequested.current = true;
    if (active.id) {
      void invoke("cancel_conversion", { conversionId: active.id });
    }
  };

  const pickFile = async () => {
    cancelActive();
    activeRef.current = null;
    cancelRequested.current = false;
    setPhase("picking");
    setError(null);

    try {
      const result = await invoke<OnboardingFileInfo | null>(
        "pick_onboarding_file"
      );
      if (result) {
        setFile(result.file);
        setTargets(result.targets);
        setSelected(null);
        setHovered(null);
        setOutputPath(null);
        setProgress(0);
        setMessage("");
        setPhase("selected");
      } else {
        setPhase(file ? "selected" : "idle");
      }
    } catch (reason) {
      setError(String(reason));
      setPhase(file ? "selected" : "idle");
    }
  };

  const startConversion = async () => {
    if (!file || !selected || phase === "converting" || startingRef.current) {
      return;
    }

    startingRef.current = true;
    setPhase("converting");
    setProgress(0.04);
    setMessage(`Convertendo para ${selected.label}.`);
    setError(null);
    setOutputPath(null);
    cancelRequested.current = false;
    activeRef.current = {
      sourcePath: file.path,
      targetFormat: selected.id,
      id: null,
    };

    try {
      const id = await invoke<string>("start_onboarding_conversion", {
        request: {
          sourcePath: file.path,
          targetFormat: selected.id,
        },
      });

      const active = activeRef.current;
      if (
        !active ||
        active.sourcePath !== file.path ||
        active.targetFormat !== selected.id
      ) {
        void invoke("cancel_conversion", { conversionId: id });
        return;
      }

      active.id = id;
      if (cancelRequested.current) {
        void invoke("cancel_conversion", { conversionId: id });
      }
    } catch (reason) {
      activeRef.current = null;
      setPhase("failed");
      setError(String(reason));
    } finally {
      startingRef.current = false;
    }
  };

  const cancelConversion = () => {
    cancelRequested.current = true;
    const active = activeRef.current;
    if (active?.id) {
      void invoke("cancel_conversion", { conversionId: active.id });
    }
    setMessage("Cancelando conversão…");
  };

  const chooseAnother = () => {
    setSelected(null);
    setHovered(null);
    setError(null);
    setOutputPath(null);
    setProgress(0);
    setMessage("");
    setPhase("selected");
  };

  const reveal = () => {
    if (outputPath) {
      void invoke("reveal_output", { path: outputPath });
    }
  };

  const focusedTarget =
    hovered !== null ? targets[hovered] ?? null : selected;
  const percent = Math.round(Math.max(0, Math.min(1, progress)) * 100);

  return (
    <div className="onboarding-step">
      <StepHeader
        eyebrow="Experimente"
        title={
          <>
            Agora é <span className="onboarding-title-accent">sua vez.</span>
          </>
        }
        description="Selecione um arquivo real para testar a conversão funcionando. O arquivo original nunca é alterado."
      />

      {!file ? (
        <div className="try-panel try-panel-idle">
          <div className="try-idle-mark">
            <FileIcon size={28} />
          </div>
          <h2 className="try-idle-title">Escolha um arquivo real</h2>
          <p className="try-idle-copy">
           A conversão ocorre de verdade, salvando o resultado logo ao lado do original. Você escolhe o formato e confirma antes de prosseguir.
          </p>
          <button
            type="button"
            className="btn-primary try-pick-button"
            onClick={() => void pickFile()}
            disabled={phase === "picking"}
          >
            {phase === "picking" ? "Abrindo seletor…" : "Escolher arquivo"}
          </button>
          <p className="try-idle-note">
            Prefere o gesto? Segure Shift e arraste um arquivo no Explorer.
          </p>
          {error ? (
            <p className="try-error-line" role="alert">
              {error}
            </p>
          ) : null}
        </div>
      ) : (
        <div className="try-panel try-panel-active">
          <div className="try-grid">
            <div className="try-visual">
              {phase === "completed" ? (
                <div className="try-success-visual" aria-hidden="true">
                  <span className="try-success-ring" />
                  <span className="try-success-icon">
                    <CheckIcon size={30} />
                  </span>
                  <span className="try-success-file">{file.fileName}</span>
                </div>
              ) : (
                <div className="try-wheel-frame" ref={wheelRef}>
                  {payload ? (
                    <FormatWheel
                      payload={payload}
                      hovered={hovered}
                      onHover={setHovered}
                      onSelect={(target) => {
                        if (phase === "converting") return;
                        setSelected(target);
                        setPhase("selected");
                        setError(null);
                        setOutputPath(null);
                      }}
                    />
                  ) : null}
                </div>
              )}
            </div>

            <div className="try-content">
              <div className="try-file-row">
                <span className="try-file-icon">
                  <FileIcon size={17} />
                </span>
                <span className="try-file-copy">
                  <span className="try-file-name" title={file.path}>
                    {file.fileName}
                  </span>
                  <span className="try-file-meta">
                    {file.kindLabel || file.category || "Arquivo"}
                    {formatBytes(file.sizeBytes)
                      ? ` · ${formatBytes(file.sizeBytes)}`
                      : ""}
                  </span>
                </span>
                <button
                  type="button"
                  className="try-link"
                  onClick={() => void pickFile()}
                  disabled={phase === "converting"}
                >
                  Trocar
                </button>
              </div>

              {phase === "converting" ? (
                <div className="try-progress">
                  <div className="try-progress-head">
                    <span>
                      Convertendo para{" "}
                      <strong>{selected?.label ?? "…"}</strong>
                    </span>
                    <span>{percent}%</span>
                  </div>
                  <div
                    className="try-progress-track"
                    role="progressbar"
                    aria-valuenow={percent}
                    aria-valuemin={0}
                    aria-valuemax={100}
                  >
                    <span
                      className="try-progress-fill"
                      style={{ width: `${percent}%` }}
                    />
                  </div>
                  <p className="try-progress-message">{message}</p>
                  <button
                    type="button"
                    className="btn-ghost try-small-button"
                    onClick={cancelConversion}
                  >
                    Cancelar
                  </button>
                </div>
              ) : null}

              {phase === "completed" ? (
                <div className="try-success">
                  <span className="try-success-kicker">Tudo certo</span>
                  <h2 className="try-result-title">Conversão concluída.</h2>
                  <p className="try-output" title={outputPath ?? undefined}>
                    <FolderIcon size={14} />
                    <span>{fileName(outputPath)}</span>
                  </p>
                  <p className="try-safe">
                    O original foi preservado. O novo arquivo está ao lado dele.
                  </p>
                  <div className="try-actions">
                    <button
                      type="button"
                      className="btn-primary try-small-button"
                      onClick={reveal}
                    >
                      Abrir pasta
                    </button>
                    <button
                      type="button"
                      className="btn-ghost try-small-button"
                      onClick={() => void pickFile()}
                    >
                      Escolher outro
                    </button>
                  </div>
                </div>
              ) : null}

              {phase === "failed" ? (
                <div className="try-failure">
                  <span className="try-failure-kicker">Não concluído</span>
                  <h2 className="try-result-title">Não foi possível converter.</h2>
                  <p className="try-error-line" role="alert">
                    {error ?? "A conversão foi interrompida."}
                  </p>
                  <div className="try-actions">
                    <button
                      type="button"
                      className="btn-primary try-small-button"
                      onClick={() => void startConversion()}
                      disabled={!selected}
                    >
                      Tentar novamente
                    </button>
                    <button
                      type="button"
                      className="btn-ghost try-small-button"
                      onClick={chooseAnother}
                    >
                      Escolher formato
                    </button>
                  </div>
                </div>
              ) : null}

              {phase === "selected" ? (
                selected ? (
                  <div className="try-confirm">
                    <span className="try-confirm-kicker">Formato selecionado</span>
                    <div className="try-target-line">
                      <strong>{selected.label}</strong>
                      <span>{selected.description}</span>
                    </div>
                    <p className="try-safe">
                      Converter <strong>{file.fileName}</strong> para{" "}
                      <strong>{selected.label}</strong>? O resultado será salvo
                      ao lado do original, sem alterar o arquivo atual.
                    </p>
                    <div className="try-actions">
                      <button
                        type="button"
                        className="btn-primary try-small-button"
                        onClick={() => void startConversion()}
                      >
                        Converter agora
                      </button>
                      <button
                        type="button"
                        className="btn-ghost try-small-button"
                        onClick={chooseAnother}
                      >
                        Escolher outro
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="try-prompt">
                    <span className="try-prompt-kicker">
                      <SparkIcon size={13} /> Orbit pronto
                    </span>
                    <h2 className="try-result-title">Escolha um formato.</h2>
                    <p className="try-safe">
                      Passe o mouse sobre um formato para ver os detalhes. Nada
                      é convertido até você confirmar.
                    </p>
                    {focusedTarget ? (
                      <p className="try-hover-detail">
                        <strong>{focusedTarget.label}</strong>{" "}
                        {focusedTarget.description}
                      </p>
                    ) : null}
                  </div>
                )
              ) : null}

              {error && phase !== "failed" && phase !== "idle" ? (
                <p className="try-error-line" role="alert">
                  {error}
                </p>
              ) : null}
            </div>
          </div>
        </div>
      )}

      <p className="onboarding-footnote">
        Esta etapa é opcional. Você pode continuar sem converter nada.
      </p>
    </div>
  );
}
