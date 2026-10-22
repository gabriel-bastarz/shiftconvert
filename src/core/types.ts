/**
 * Tipos compartilhados entre o backend Rust e a interface.
 * Os payloads usam camelCase, igual ao serde do backend.
 */

export interface FileInfo {
  path: string;
  fileName: string;
  extension: string;
  sizeBytes: number;
  exists: boolean;
  isReadable: boolean;
  category: string;
  kindLabel: string;
}

export interface TargetFormat {
  id: string;
  label: string;
  extension: string;
  description: string;
  kind: "image" | "text" | "video" | "archive" | string;
}

export interface WheelLayout {
  centerX: number;
  centerY: number;
  outerRadius: number;
  innerRadius: number;
  rotation: number;
  seamBends: number[];
  outerWave: number[];
  outerPhase: number[];
  innerWave: number[];
  innerPhase: number[];
}

export interface Backdrop {
  dataUrl: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OverlayContext {
  windowX: number;
  windowY: number;
  windowWidth: number;
  windowHeight: number;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
  scale: number;
}

export interface WheelPayload {
  mode: "drag" | "picker";
  file: FileInfo;
  targets: TargetFormat[];
  layout: WheelLayout;
  backdrop: Backdrop | null;
  overlay: OverlayContext;
}

export interface DragOverPayload {
  cursorX: number;
  cursorY: number;
  regionIndex: number | null;
  targetId: string | null;
  angle: number;
  distance: number;
  overCenter: boolean;
}

export interface DropPayload {
  file: FileInfo;
  target: TargetFormat;
  conversionId: string;
  layout: WheelLayout;
  overlay: OverlayContext;
  cursorX: number;
  cursorY: number;
}

export interface DragMessage {
  message: string;
  cursorX: number;
  cursorY: number;
}

export interface PickerResponse {
  cancelled: boolean;
  wheel: WheelPayload | null;
  message: string | null;
}

export interface HudInfo {
  width: number;
  height: number;
}

export interface StartupInfo {
  mode: string;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
  scale: number;
}

export type ConversionStatus =
  | "in-progress"
  | "completed"
  | "failed"
  | "cancelled";

export interface ConversionProgress {
  conversionId: string;
  sourcePath: string;
  targetFormat: string;
  status: ConversionStatus;
  progress: number;
  message: string;
  outputPath: string | null;
  error: string | null;
}

export interface HudState {
  file: FileInfo;
  target: TargetFormat;
  conversionId: string;
  status: ConversionStatus;
  progress: number;
  message: string;
  outputPath: string | null;
  error: string | null;
}

export interface ToastState {
  id: number;
  message: string;
  x: number;
  y: number;
}
