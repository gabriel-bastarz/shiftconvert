import type {
  FileInfo,
  TargetFormat,
  WheelLayout,
  WheelPayload,
} from "../core/types";

export type DemoCategoryId =
  | "document"
  | "image"
  | "audio"
  | "video"
  | "archive";

export interface DemoCategory {
  id: DemoCategoryId;
  label: string;
  file: FileInfo;
  targets: TargetFormat[];
  note: string;
}

function target(
  id: string,
  label: string,
  description: string,
  kind: TargetFormat["kind"]
): TargetFormat {
  return {
    id,
    label,
    extension: id,
    description,
    kind,
  };
}

function demoFile(
  fileName: string,
  category: string,
  kindLabel: string,
  sizeBytes: number
): FileInfo {
  const extension = fileName.split(".").pop() ?? "";
  return {
    path: `C:\\ShiftConvert\\${fileName}`,
    fileName,
    extension,
    sizeBytes,
    exists: true,
    isReadable: true,
    category,
    kindLabel,
  };
}

const zip = target(
  "zip",
  "ZIP",
  "Compactar em um arquivo ZIP",
  "archive"
);

export const DEMO_CATEGORIES: DemoCategory[] = [
  {
    id: "document",
    label: "Documentos",
    file: demoFile("contrato.docx", "document", "Documento", 284_000),
    targets: [
      target("pdf", "PDF", "PDF de texto (sem layout original)", "document"),
      target("html", "HTML", "HTML com o conteúdo do documento", "document"),
      target("txt", "TXT", "Texto simples", "text"),
      zip,
    ],
    note: "DOCX e PDF oferecem conversão real de texto.",
  },
  {
    id: "image",
    label: "Imagens",
    file: demoFile("ensaio.png", "image", "Imagem", 1_840_000),
    targets: [
      target("jpg", "JPG", "Foto compacta", "image"),
      target("webp", "WEBP", "Imagem leve para web", "image"),
      target("bmp", "BMP", "Bitmap sem compressão", "image"),
      target("gif", "GIF", "GIF estático (1º quadro)", "image"),
      target("tiff", "TIFF", "Alta qualidade", "image"),
      zip,
    ],
    note: "Seis formatos de imagem disponíveis, todos reais.",
  },
  {
    id: "audio",
    label: "Áudio",
    file: demoFile("trilha.mp3", "audio", "Áudio", 4_600_000),
    targets: [zip],
    note: "Nesta versão, áudio pode ser compactado em ZIP.",
  },
  {
    id: "video",
    label: "Vídeo",
    file: demoFile("cena.mp4", "video", "Vídeo", 18_400_000),
    targets: [
      target("webm", "WEBM", "Vídeo VP9 para web", "video"),
      target("mkv", "MKV", "Contêiner Matroska", "video"),
      target("gif", "GIF", "GIF animado", "video"),
      zip,
    ],
    note: "Vídeo usa o FFmpeg empacotado com o ShiftConvert.",
  },
  {
    id: "archive",
    label: "Compactação",
    file: demoFile("planilha.xlsx", "document", "Documento", 92_000),
    targets: [zip],
    note: "Qualquer arquivo, exceto ZIP, pode ser compactado.",
  },
];

function seamBends(count: number): number[] {
  return Array.from({ length: count }, (_, index) => {
    const value = index;
    const magnitude = 0.068 + 0.026 * Math.abs(Math.sin(value * 1.73 + 0.9));
    const sign = Math.sin(value * 1.7 + 0.6) >= 0 ? 1 : -1;
    return sign * magnitude;
  });
}

export function buildDemoWheelPayload(options: {
  file: FileInfo;
  targets: TargetFormat[];
  size: number;
  mode: "drag" | "picker";
}): WheelPayload {
  const size = Math.max(160, Math.round(options.size));
  const count = Math.max(1, options.targets.length);
  const outer = size * 0.37;
  const inner = outer * 0.35;
  const layout: WheelLayout = {
    centerX: size / 2,
    centerY: size / 2,
    outerRadius: outer,
    innerRadius: inner,
    rotation: -Math.PI / 2 - Math.PI / count,
    seamBends: seamBends(count),
    outerWave: [0, 0, 0],
    outerPhase: [0.7, 2.1, 4.3],
    innerWave: [0, 0],
    innerPhase: [1.3, 3.8],
  };

  return {
    mode: options.mode,
    file: options.file,
    targets: options.targets,
    layout,
    backdrop: null,
    overlay: {
      windowX: 0,
      windowY: 0,
      windowWidth: size,
      windowHeight: size,
      workX: 0,
      workY: 0,
      workWidth: size,
      workHeight: size,
      scale: 1,
    },
  };
}

export const GESTURE_DEMO_FILE = demoFile(
  "relatório.docx",
  "document",
  "Documento",
  284_000
);

export const GESTURE_DEMO_TARGETS: TargetFormat[] = [
  target("pdf", "PDF", "PDF de texto", "document"),
  target("html", "HTML", "HTML com o conteúdo", "document"),
  target("txt", "TXT", "Texto simples", "text"),
  zip,
];
