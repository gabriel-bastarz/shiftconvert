/**
 * Estado de conversão compartilhado entre componentes.
 * Mantido simples para o MVP — sem estado global complexo (Redux, etc.).
 */

export type ConversionStatus = "pending" | "in-progress" | "completed" | "failed" | "cancelled";

export interface ConversionState {
  id: string;
  sourcePath: string;
  targetFormat: string;
  category: string;
  outputPath: string | null;
  status: ConversionStatus;
  progress: number; // 0 .. 1
  error: string | null;
  startedAt: number;
  finishedAt: number | null;
}

export interface FileActivation {
  filePath: string | null;
  cursorX: number;
  cursorY: number;
  screenWidth: number;
  screenHeight: number;
  timestamp: number;
  source: "globalHook" | "manualFallback" | "unknown";
}

/** Metadados retornados pelo seletor de arquivo nativo (comando pick_file). */
export interface PickedFileInfo {
  path: string;
  fileName: string;
  extension: string;
  category: string | null;
  sizeBytes: number;
}

export const emptyConversion: ConversionState = {
  id: "",
  sourcePath: "",
  targetFormat: "",
  category: "",
  outputPath: null,
  status: "pending",
  progress: 0,
  error: null,
  startedAt: 0,
  finishedAt: null,
};

export const emptyActivation: FileActivation = {
  filePath: null,
  cursorX: 0,
  cursorY: 0,
  screenWidth: 0,
  screenHeight: 0,
  timestamp: 0,
  source: "unknown",
};
