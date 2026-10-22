/**
 * RadialMenu.tsx
 * Menu radial legado mantido apenas para referência (a interface ativa usa FormatWheel).
 * Controla abertura, fechamento, seleção de categoria e formato,
 * reposicionamento em relação às bordas da tela e cancelamento.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { RadialPetal } from "./RadialPetal";
import { ConversionStatus as ConversionStatusPanel } from "./ConversionStatus";
import {
  FORMAT_REGISTRY,
  getTargetFormatsForCategory,
} from "@/core/format-registry";
import {
  ConversionState,
  ConversionStatus,
  emptyConversion,
  FileActivation,
  PickedFileInfo,
} from "@/core/conversion-state";
import { categorizeByExtension, FileCategory } from "@/core/file-types";

// ---------------------------------------------------------------------------
// Configuração das cinco pétalas
// ---------------------------------------------------------------------------

const PETALS = [
  { index: 0, angle: -90, category: "document", icon: "📄", label: "Documentos" },
  { index: 1, angle: -18, category: "image", icon: "🖼️", label: "Imagens" },
  { index: 2, angle: 54, category: "audio", icon: "🎵", label: "Áudio" },
  { index: 3, angle: 126, category: "video", icon: "🎬", label: "Vídeo" },
  { index: 4, angle: 198, category: "compression", icon: "📦", label: "Compactação" },
] as const;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface RadialMenuProps {
  filePath: string | null;
  cursorX: number;
  cursorY: number;
  screenWidth: number;
  screenHeight: number;
  /** Chamado após o usuário escolher um arquivo no seletor nativo. */
  onFileSelected: (file: PickedFileInfo) => void;
  onClose: () => void;
}

interface MenuPosition {
  x: number;
  y: number;
  edge: string | null;
}

/**
 * Calcula a posição do menu ajustando para não ultrapassar as bordas da tela.
 */
function computeMenuPosition(
  cursorX: number,
  cursorY: number,
  screenWidth: number,
  screenHeight: number,
  menuSize: number = 340
): MenuPosition {
  let x = cursorX - menuSize / 2;
  let y = cursorY - menuSize / 2;
  let edge: string | null = null;

  if (x < 0) {
    x = 10;
    edge = "left";
  } else if (x + menuSize > screenWidth) {
    x = screenWidth - menuSize - 10;
    edge = "right";
  }

  if (y < 0) {
    y = 10;
    if (!edge) edge = "top";
  } else if (y + menuSize > screenHeight) {
    y = screenHeight - menuSize - 10;
    if (!edge) edge = "bottom";
  }

  return { x, y, edge };
}

/**
 * Extrai a categoria do arquivo com base na extensão.
 */
function fileCategoryFromPath(path: string | null): FileCategory {
  if (!path) return "unknown";
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return categorizeByExtension(ext);
}

export const RadialMenu: React.FC<RadialMenuProps> = ({
  filePath,
  cursorX,
  cursorY,
  screenWidth,
  screenHeight,
  onFileSelected,
  onClose,
}) => {
  const [isOpen, setIsOpen] = useState(true);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [, setSelectedFormat] = useState<string | null>(null);
  const [conversion, setConversion] = useState<ConversionState>(emptyConversion);
  const [showConversionStatus, setShowConversionStatus] = useState(false);
  const [conversionError, setConversionError] = useState<string | null>(null);
  const [pickingFile, setPickingFile] = useState(false);

  const menuRef = useRef<HTMLDivElement>(null);

  // O reposicionamento em relação às bordas da tela é feito no backend,
  // que move a janela overlay para não sair do monitor.
  const position = useMemo(
    () => computeMenuPosition(cursorX, cursorY, screenWidth, screenHeight),
    [cursorX, cursorY, screenWidth, screenHeight]
  );

  // Os formatos exibidos pertencem à pétala selecionada. Isso permite, por
  // exemplo, compactar um TXT em ZIP (pétala Compactação) sem confundir com
  // os formatos de documento.
  const availableFormats = useMemo(() => {
    if (!activeCategory) return [];
    return getTargetFormatsForCategory(activeCategory as FileCategory);
  }, [activeCategory]);

  // -------------------------------------------------------------------------
  // Event listeners do Tauri
  // -------------------------------------------------------------------------

  useEffect(() => {
    const unlisten = listen<FileActivation>("shiftconvert:activation", (event) => {
      const payload = event.payload;
      console.log("[ShiftConvert] Evento de ativação recebido:", payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // -------------------------------------------------------------------------
  // Handlers
  // -------------------------------------------------------------------------

  const handlePetalSelect = useCallback((category: string): void => {
    setActiveCategory(category);
    setSelectedFormat(null);
    setConversionError(null);
  }, []);

  const handlePickFile = useCallback(async (): Promise<void> => {
    setPickingFile(true);
    setConversionError(null);
    try {
      const info = await invoke<PickedFileInfo | null>("pick_file");
      if (info) {
        onFileSelected(info);
        setActiveCategory(null);
        setSelectedFormat(null);
      }
    } catch (err: unknown) {
      const message =
        err instanceof Error
          ? err.message
          : typeof err === "string"
            ? err
            : "Não foi possível abrir o seletor de arquivos.";
      setConversionError(message);
      console.error("[ShiftConvert] Erro ao escolher arquivo:", err);
    } finally {
      setPickingFile(false);
    }
  }, [onFileSelected]);

  const handleFormatSelect = useCallback(
    async (format: string): Promise<void> => {
      if (!filePath) {
        setConversionError("Nenhum arquivo selecionado.");
        return;
      }

      const formatEntry = FORMAT_REGISTRY.flatMap((entry) => entry.targetFormats).find(
        (f) => f.value === format
      );

      if (!formatEntry || !formatEntry.implemented) {
        setSelectedFormat(format);
        setConversionError(
          `Conversão para ${format.toUpperCase()} está em simulação. Ainda não implementada.`
        );
        return;
      }

      setSelectedFormat(format);

      // A categoria do engine é a pétala selecionada (ex.: Compactação),
      // não necessariamente a categoria do arquivo de origem.
      const engineCategory = activeCategory ?? fileCategoryFromPath(filePath);

      try {
        const result = await invoke<ConversionState>("convert_file", {
          sourcePath: filePath,
          targetFormat: format,
          category: engineCategory,
        });

        setConversion(result);
        setShowConversionStatus(true);

        if (result.status === "completed") {
          setTimeout(() => onClose(), 1500);
        }
      } catch (err: unknown) {
        const message =
          err instanceof Error ? err.message : typeof err === "string" ? err : "Erro ao iniciar conversão.";
        setConversionError(message);
        console.error("[ShiftConvert] Erro na conversão:", err);
      }
    },
    [filePath, activeCategory, onClose]
  );

  const handleCancelConversion = useCallback((): void => {
    if (conversion.id) {
      invoke("cancel_conversion_cmd", { conversionId: conversion.id }).catch(
        (err: unknown) => {
          console.error("[ShiftConvert] Erro ao cancelar:", err);
        }
      );
    }
    setShowConversionStatus(false);
    setConversion(emptyConversion);
  }, [conversion.id]);

  const handleClose = useCallback((): void => {
    setIsOpen(false);
    setActiveCategory(null);
    setSelectedFormat(null);
    onClose();
  }, [onClose]);

  const handleBackdropClick = useCallback((): void => {
    handleClose();
  }, [handleClose]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        handleClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleClose]);

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  const categoryOfFile = fileCategoryFromPath(filePath);

  return (
    <div
      className={`radial-overlay ${isOpen ? "visible" : ""}`}
      data-visible={isOpen ? "true" : "false"}
      onClick={handleBackdropClick}
    >
      <div className="radial-backdrop" />

      <div
        ref={menuRef}
        className="radial-menu"
        data-edge={position.edge ?? undefined}
        style={{
          width: "340px",
          height: "340px",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="radial-outer-ring" />

        <div className="radial-center">
          {filePath ? (
            <span className="center-glyph">SC</span>
          ) : (
            <button
              type="button"
              className="center-picker"
              onClick={handlePickFile}
              disabled={pickingFile}
              title="Escolher um arquivo para converter"
              aria-label="Escolher arquivo"
            >
              <span className="center-picker-icon" aria-hidden="true">
                +
              </span>
              <span className="center-picker-label">
                {pickingFile ? "abrindo" : "arquivo"}
              </span>
            </button>
          )}
        </div>

        {filePath && (
          <button
            type="button"
            className="overlay-file-chip"
            title={`${filePath} — clique para trocar de arquivo`}
            onClick={(e) => {
              e.stopPropagation();
              void handlePickFile();
            }}
            disabled={pickingFile}
          >
            {filePath.split(/[\\/]/).pop() ?? filePath}
          </button>
        )}

        {PETALS.map((petal) => {
          const formats = activeCategory === petal.category ? availableFormats : [];
          return (
            <RadialPetal
              key={petal.index}
              angle={petal.angle}
              index={petal.index}
              categoryName={petal.label}
              category={petal.category}
              icon={petal.icon}
              isActive={activeCategory === petal.category}
              formats={formats}
              onSelect={handlePetalSelect}
              onFormatSelect={handleFormatSelect}
            />
          );
        })}
      </div>

      {showConversionStatus && conversion.id && (
        <ConversionStatusPanel
          conversion={conversion}
          visible={true}
          onCancel={handleCancelConversion}
        />
      )}

      {conversionError && !showConversionStatus && (
        <div
          className="conversion-status visible"
          style={{
            borderColor: "#f87171",
            position: "fixed",
            bottom: 32,
            right: 32,
          }}
        >
          <div className="status-header">
            <span className="status-icon">❌</span>
            <div>
              <div className="status-title">Erro na conversão</div>
              <div className="status-detail">{conversionError}</div>
            </div>
          </div>
          <div className="status-actions">
            <button
              type="button"
              className="btn-cancel"
              onClick={() => setConversionError(null)}
            >
              Fechar
            </button>
          </div>
        </div>
      )}

      {isOpen && !activeCategory && (
        <div className="sr-only" aria-live="polite">
          Menu radial aberto. Selecione uma categoria com o mouse ou teclado.
          Pressione Escape para cancelar.
        </div>
      )}

      <div className="sr-only">Categoria do arquivo: {categoryOfFile}</div>
      <div className="sr-only">Status: {conversion.status as ConversionStatus}</div>
    </div>
  );
};
