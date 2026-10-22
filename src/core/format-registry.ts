/**
 * Registro central de formatos suportados por categoria.
 * Cada entrada indica se o formato está implementado (real) ou simulado.
 * Usado pelo frontend para exibir apenas opções disponíveis e filtrar
 * conversões incompatíveis.
 */

import { FileCategory } from "./file-types";

export interface FormatOption {
  value: string;
  label: string;
  /** Se verdadeiro, a conversão está implementada de fato. Se falso, é simulação. */
  implemented: boolean;
  /** MIME type de saída sugerido. */
  mimeType?: string;
}

export interface FormatRegistryEntry {
  category: FileCategory;
  sourceExtensions: string[];
  targetFormats: FormatOption[];
}

/**
 * Registro de formatos. Fonte única da verdade para formatos disponíveis.
 * Formatos marcados como implemented=false são simulações (Fase 2).
 */
export const FORMAT_REGISTRY: FormatRegistryEntry[] = [
  {
    category: "image",
    sourceExtensions: ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif"],
    targetFormats: [
      { value: "png", label: "PNG", implemented: true, mimeType: "image/png" },
      { value: "jpg", label: "JPEG", implemented: true, mimeType: "image/jpeg" },
      { value: "webp", label: "WebP", implemented: true, mimeType: "image/webp" },
      { value: "bmp", label: "BMP", implemented: true, mimeType: "image/bmp" },
      { value: "gif", label: "GIF", implemented: true, mimeType: "image/gif" },
    ],
  },
  {
    category: "compression",
    sourceExtensions: ["zip", "rar", "7z", "tar", "gz"],
    targetFormats: [
      { value: "zip", label: "ZIP", implemented: true, mimeType: "application/zip" },
    ],
  },
  {
    category: "audio",
    sourceExtensions: ["mp3", "wav", "flac", "aac", "ogg", "m4a"],
    targetFormats: [
      { value: "mp3", label: "MP3", implemented: false, mimeType: "audio/mpeg" },
      { value: "wav", label: "WAV", implemented: false, mimeType: "audio/wav" },
    ],
  },
  {
    category: "video",
    sourceExtensions: ["mp4", "mkv", "avi", "mov", "webm"],
    targetFormats: [
      { value: "mp4", label: "MP4", implemented: false, mimeType: "video/mp4" },
      { value: "webm", label: "WebM", implemented: false, mimeType: "video/webm" },
    ],
  },
  {
    category: "document",
    sourceExtensions: ["pdf", "docx", "txt", "odt"],
    targetFormats: [
      { value: "pdf", label: "PDF", implemented: false, mimeType: "application/pdf" },
      { value: "txt", label: "TXT", implemented: false, mimeType: "text/plain" },
    ],
  },
];

/**
 * Retorna os formatos de destino disponíveis para uma categoria de origem.
 * Filtra apenas formatos compatíveis (mesma categoria).
 */
export function getTargetFormatsForCategory(
  category: FileCategory
): FormatOption[] {
  const entry = FORMAT_REGISTRY.find((e) => e.category === category);
  return entry ? entry.targetFormats : [];
}

/**
 * Verifica se uma conversão de origem para destino é suportada pelo registro.
 */
export function isConversionSupported(
  sourceCategory: FileCategory,
  targetFormat: string
): boolean {
  const formats = getTargetFormatsForCategory(sourceCategory);
  return formats.some((f) => f.value === targetFormat.toLowerCase());
}

/**
 * Retorna a categoria de um formato de destino (para validação cruzada).
 */
export function getCategoryForFormat(format: string): FileCategory | null {
  const entry = FORMAT_REGISTRY.find((e) =>
    e.targetFormats.some((f) => f.value === format.toLowerCase())
  );
  return entry ? entry.category : null;
}
