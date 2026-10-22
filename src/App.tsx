import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { FormatWheel } from "./components/FormatWheel";
import { StartupPrompt } from "./components/StartupPrompt";
import { StatusHud } from "./components/StatusHud";
import type {
  ConversionProgress,
  DragMessage,
  DragOverPayload,
  DropPayload,
  HudState,
  PickerResponse,
  StartupInfo,
  ToastState,
  WheelPayload,
} from "./core/types";
import "./styles/orbit.css";

type Mode = "boot" | "prompt" | "loading" | "armed" | "wheel" | "hud" | "hidden";

function fallbackFile(path: string) {
  const name = path.split(/[\\/]/).pop() ?? "arquivo";
  return {
    path,
    fileName: name,
    extension: name.includes(".") ? name.split(".").pop() ?? "" : "",
    sizeBytes: 0,
    exists: true,
    isReadable: true,
    category: "",
    kindLabel: "",
  };
}

export default function App() {
  const [mode, setMode] = useState<Mode>("boot");
  const [message, setMessage] = useState<string | null>(null);
  const [wheel, setWheel] = useState<WheelPayload | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const [hud, setHud] = useState<HudState | null>(null);
  const [toast, setToast] = useState<ToastState | null>(null);

  const modeRef = useRef(mode);
  const wheelRef = useRef(wheel);
  const hudRef = useRef(hud);
  const toastTimer = useRef<number | null>(null);
  const progressCache = useRef(new Map<string, ConversionProgress>());

  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);
  useEffect(() => {
    wheelRef.current = wheel;
  }, [wheel]);
  useEffect(() => {
    hudRef.current = hud;
  }, [hud]);

  const showToast = useCallback((message: string, x: number, y: number, duration = 1900) => {
    const clampedX = Math.min(Math.max(x, 60), Math.max(60, window.innerWidth - 60));
    const clampedY = Math.min(Math.max(y, 60), Math.max(60, window.innerHeight - 60));
    setToast({ id: Date.now(), message, x: clampedX, y: clampedY });
    if (toastTimer.current !== null) {
      window.clearTimeout(toastTimer.current);
    }
    toastTimer.current = window.setTimeout(() => setToast(null), duration);
  }, []);

  const handlePick = useCallback(async () => {
    setMessage(null);
    setMode("loading");
    try {
      const response = await invoke<PickerResponse>("pick_file");
      if (response.cancelled) {
        setMode("prompt");
        return;
      }
      if (response.wheel) {
        setWheel(response.wheel);
        setHovered(null);
        setHud(null);
        setMode("wheel");
        return;
      }
      setMessage(response.message ?? "Não foi possível usar este arquivo.");
      setMode("prompt");
    } catch (error) {
      setMessage(String(error));
      setMode("prompt");
    }
  }, []);

  const handleSelect = useCallback(
    (index: number) => {
      const current = wheelRef.current;
      if (!current) return;
      const target = current.targets[index];
      if (!target) return;

      setHud({
        file: current.file,
        target,
        conversionId: "",
        status: "in-progress",
        progress: 0.02,
        message: `Convertendo para ${target.label}…`,
        outputPath: null,
        error: null,
      });
      setMode("hud");
      setWheel(null);
      setHovered(null);

      void invoke("show_hud", {
        anchorX: current.layout.centerX,
        anchorY: current.layout.centerY,
      }).catch(() => undefined);

      invoke<string>("start_conversion", {
        request: {
          sourcePath: current.file.path,
          targetFormat: target.id,
        },
      })
        .then((conversionId) => {
          progressCache.current.set(conversionId, {
            conversionId,
            sourcePath: current.file.path,
            targetFormat: target.id,
            status: "in-progress",
            progress: 0.02,
            message: `Convertendo para ${target.label}…`,
            outputPath: null,
            error: null,
          });
          setHud((value) => (value ? { ...value, conversionId } : value));
        })
        .catch((error) => {
          setHud((value) =>
            value
              ? {
                  ...value,
                  status: "failed",
                  message: String(error),
                  error: String(error),
                }
              : value
          );
        });
    },
    []
  );

  const handleCancel = useCallback(() => {
    const current = hudRef.current;
    if (!current?.conversionId) return;
    void invoke("cancel_conversion", { conversionId: current.conversionId });
    setHud((value) =>
      value ? { ...value, message: "Cancelando…", error: null } : value
    );
  }, []);

  const handleClose = useCallback(() => {
    void invoke("hide_overlay");
    setMode("hidden");
    setMessage(null);
    setWheel(null);
    setHovered(null);
    setHud(null);
  }, []);

  const handleReveal = useCallback(() => {
    const current = hudRef.current;
    if (!current?.outputPath) return;
    void invoke("reveal_output", { path: current.outputPath });
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<Promise<UnlistenFn>> = [];

    const applyProgress = (progress: ConversionProgress) => {
      progressCache.current.set(progress.conversionId, progress);
      setHud((value) => {
        if (!value) {
          return {
            file: fallbackFile(progress.sourcePath),
            target: {
              id: progress.targetFormat,
              label: progress.targetFormat.toUpperCase(),
              extension: progress.targetFormat,
              description: "",
              kind: "",
            },
            conversionId: progress.conversionId,
            status: progress.status,
            progress: progress.progress,
            message: progress.message,
            outputPath: progress.outputPath,
            error: progress.error,
          };
        }
        if (value.conversionId && value.conversionId !== progress.conversionId) {
          return value;
        }
        return {
          ...value,
          conversionId: value.conversionId || progress.conversionId,
          status: progress.status,
          progress: progress.progress,
          message: progress.message,
          outputPath: progress.outputPath,
          error: progress.error,
        };
      });
      setMode("hud");
    };

    const setup = async () => {
      try {
        await invoke<StartupInfo>("ui_ready");
        if (!disposed) setMode("prompt");
      } catch (error) {
        if (!disposed) {
          setMessage(String(error));
          setMode("prompt");
        }
      }

      const add = <T,>(name: string, handler: (payload: T) => void) => {
        unlisteners.push(
          listen<T>(name, (event) => {
            if (!disposed) handler(event.payload);
          })
        );
      };

      add<StartupInfo>("show-prompt", () => {
        setMessage(null);
        setWheel(null);
        setHovered(null);
        setHud(null);
        setMode("prompt");
      });

      add("tray-pick", () => {
        void handlePick();
      });

      add("drag-armed", () => {
        setWheel(null);
        setHovered(null);
        setMode("armed");
      });

      add<WheelPayload>("drag-enter", (payload) => {
        setWheel(payload);
        setHovered(null);
        setHud(null);
        setMode("wheel");
      });

      add<DragOverPayload>("drag-over", (payload) => {
        if (modeRef.current === "wheel") {
          setHovered(payload.regionIndex);
        }
      });

      add("drag-leave", () => {
        setHovered(null);
      });

      add<DragMessage>("drag-invalid", (payload) => {
        showToast(payload.message, payload.cursorX, payload.cursorY, 2400);
      });

      add<DropPayload>("drop", (payload) => {
        const progress = progressCache.current.get(payload.conversionId);
        setWheel(null);
        setHovered(null);
        setHud({
          file: payload.file,
          target: payload.target,
          conversionId: payload.conversionId,
          status: progress?.status ?? "in-progress",
          progress: progress?.progress ?? 0.04,
          message: progress?.message ?? "Conversão iniciada…",
          outputPath: progress?.outputPath ?? null,
          error: progress?.error ?? null,
        });
        setMode("hud");
      });

      add<DragMessage>("drop-invalid", (payload) => {
        showToast(payload.message, payload.cursorX, payload.cursorY, 1600);
        window.setTimeout(() => {
          void invoke("hide_overlay");
          setMode("hidden");
        }, 1500);
      });

      add<DragMessage>("drop-error", (payload) => {
        showToast(payload.message, payload.cursorX, payload.cursorY, 2400);
        window.setTimeout(() => {
          void invoke("hide_overlay");
          setMode("hidden");
        }, 2200);
      });

      add("drag-cancel", () => {
        setWheel(null);
        setHovered(null);
        setMode("hidden");
      });

      add<ConversionProgress>("conversion-progress", applyProgress);
      add<ConversionProgress>("conversion-complete", applyProgress);
      add<ConversionProgress>("conversion-error", applyProgress);
    };

    void setup();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      const currentMode = modeRef.current;
      const isDragMode =
        currentMode === "armed" ||
        (currentMode === "wheel" && wheelRef.current?.mode === "drag");
      if (isDragMode) {
        // O watcher nativo cuida do cancelamento com segurança: ele mantém o
        // escudo até o botão ser solto, evitando que o Explorer mova o arquivo.
        return;
      }
      void invoke("hide_overlay");
      setMode("hidden");
    };

    window.addEventListener("keydown", onKeyDown);

    return () => {
      disposed = true;
      window.removeEventListener("keydown", onKeyDown);
      unlisteners.forEach((promise) => {
        promise.then((unlisten) => unlisten()).catch(() => undefined);
      });
      if (toastTimer.current !== null) {
        window.clearTimeout(toastTimer.current);
      }
    };
  }, [handlePick, showToast]);

  return (
    <div className={`app-root mode-${mode}`}>
      {mode === "prompt" ? (
        <StartupPrompt message={message} onPick={handlePick} onClose={handleClose} />
      ) : null}

      {mode === "loading" ? (
        <div className="loading-root">
          <div className="menu-card loading-card">
            <span className="spinner" aria-hidden="true" />
            <span>Preparando a roda…</span>
          </div>
        </div>
      ) : null}

      {mode === "wheel" && wheel ? (
        <FormatWheel
          payload={wheel}
          hovered={hovered}
          onHover={setHovered}
          onSelect={(target) => {
            const index = wheelRef.current?.targets.findIndex(
              (item) => item.id === target.id
            );
            if (index !== undefined && index >= 0) {
              handleSelect(index);
            }
          }}
        />
      ) : null}

      {mode === "hud" && hud ? (
        <StatusHud
          hud={hud}
          onCancel={handleCancel}
          onClose={handleClose}
          onReveal={handleReveal}
        />
      ) : null}

      {mode === "armed" ? (
        <div className="armed-hint" aria-hidden="true">
          Solte sobre um formato
        </div>
      ) : null}

      {toast ? (
        <div
          className="cursor-toast"
          style={{ left: toast.x, top: toast.y }}
          key={toast.id}
          role="status"
        >
          {toast.message}
        </div>
      ) : null}
    </div>
  );
}
