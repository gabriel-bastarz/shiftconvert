/**
 * Tipos de arquivo suportados pelo ShiftConvert.
 * Usado para filtrar formatos de destino compatíveis com cada categoria de origem.
 */

export type FileCategory =
  | "document"
  | "image"
  | "audio"
  | "video"
  | "compression"
  | "unknown";

export interface FileTypeInfo {
  category: FileCategory;
  extension: string;
  mimeType?: string;
}

/**
 * Classifica a categoria de um arquivo com base na extensão.
 * Corresponde à lógica do backend (src-tauri/src/file_info.rs).
 */
export function categorizeByExtension(extension: string): FileCategory {
  const ext = extension.toLowerCase().replace(/^\./, "");

  const imageExtensions = [
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico", "svg",
  ];
  const audioExtensions = [
    "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus",
  ];
  const videoExtensions = [
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
  ];
  const documentExtensions = [
    "pdf", "doc", "docx", "odt", "txt", "rtf", "xls", "xlsx", "ods", "ppt",
    "pptx", "odp", "md", "html", "htm",
  ];
  const compressionExtensions = ["zip", "rar", "7z", "tar", "gz", "bz2", "xz"];

  if (imageExtensions.includes(ext)) return "image";
  if (audioExtensions.includes(ext)) return "audio";
  if (videoExtensions.includes(ext)) return "video";
  if (documentExtensions.includes(ext)) return "document";
  if (compressionExtensions.includes(ext)) return "compression";
  return "unknown";
}

/**
 * Extrai a extensão de um caminho de arquivo.
 */
export function getExtension(filePath: string): string {
  const parts = filePath.split(".");
  if (parts.length < 2) return "";
  return parts[parts.length - 1].toLowerCase();
}
