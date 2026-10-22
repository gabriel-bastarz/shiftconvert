use std::fs;
use std::path::Path;

use log::{debug, warn};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub exists: bool,
    pub is_readable: bool,
    pub category: FileCategory,
    pub kind_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileCategory {
    Image,
    Document,
    Audio,
    Video,
    Archive,
    Other,
}

impl FileCategory {
    fn label(self) -> &'static str {
        match self {
            Self::Image => "Imagem",
            Self::Document => "Documento",
            Self::Audio => "Áudio",
            Self::Video => "Vídeo",
            Self::Archive => "Arquivo compactado",
            Self::Other => "Arquivo",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetFormat {
    /// Identificador estável usado pelo backend (jpg, png, webp, zip...).
    pub id: String,
    /// Texto curto exibido diretamente na roda.
    pub label: String,
    /// Extensão do arquivo de saída.
    pub extension: String,
    /// Descrição curta para leitores de tela e para o rodapé.
    pub description: String,
    /// "image", "text", "document", "video" ou "archive".
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Categorização
// ---------------------------------------------------------------------------

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico",
];
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus"];
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
];
const DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "odt", "txt", "rtf", "xls", "xlsx", "ods", "ppt", "pptx", "odp", "md",
    "html", "htm", "csv", "json", "xml",
];
const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "rar", "7z", "tar", "gz", "bz2", "xz"];

fn categorize(extension: &str) -> FileCategory {
    if IMAGE_EXTENSIONS.contains(&extension) {
        FileCategory::Image
    } else if AUDIO_EXTENSIONS.contains(&extension) {
        FileCategory::Audio
    } else if VIDEO_EXTENSIONS.contains(&extension) {
        FileCategory::Video
    } else if DOCUMENT_EXTENSIONS.contains(&extension) {
        FileCategory::Document
    } else if ARCHIVE_EXTENSIONS.contains(&extension) {
        FileCategory::Archive
    } else {
        FileCategory::Other
    }
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .trim_start_matches('.')
        .to_lowercase()
}

/// Normaliza aliases para o identificador canônico usado pela roda.
pub fn canonical_extension(extension: &str) -> &str {
    match extension {
        "jpeg" => "jpg",
        "tif" => "tiff",
        "markdown" => "md",
        "htm" => "html",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Inspeção
// ---------------------------------------------------------------------------

pub fn inspect_file(path: &str) -> Result<FileInfo, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("caminho vazio".to_string());
    }

    let path_buf = std::path::absolute(Path::new(trimmed))
        .map_err(|error| format!("caminho inválido: {error}"))?;

    let metadata = fs::metadata(&path_buf).map_err(|error| {
        format!(
            "não foi possível acessar '{}': {error}",
            path_buf.display()
        )
    })?;

    if !metadata.is_file() {
        return Err(format!(
            "'{}' não é um arquivo (pastas não são aceitas)",
            path_buf.display()
        ));
    }

    // Uma leitura curta confirma permissão sem carregar o arquivo inteiro.
    let is_readable = fs::File::open(&path_buf).is_ok();
    if !is_readable {
        warn!("Arquivo sem permissão de leitura: {}", path_buf.display());
    }

    let file_name = path_buf
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("arquivo")
        .to_string();
    let extension = extension_of(&path_buf);
    let category = categorize(&extension);

    let info = FileInfo {
        path: path_buf.to_string_lossy().to_string(),
        file_name,
        extension,
        size_bytes: metadata.len(),
        exists: true,
        is_readable,
        category,
        kind_label: category.label().to_string(),
    };

    debug!(
        "Arquivo inspecionado: {} ({}, {} bytes, {:?})",
        info.file_name, info.extension, info.size_bytes, info.category
    );

    Ok(info)
}

// ---------------------------------------------------------------------------
// Formatos de destino realmente implementados
// ---------------------------------------------------------------------------

/// Lista apenas conversões executadas de verdade pelo backend.
///
/// - Imagens decodificáveis pelo crate `image`: JPG, PNG, WEBP, BMP, GIF e TIFF.
/// - Texto: TXT, Markdown e HTML entre si, conforme o formato de origem.
/// - Documentos: DOCX e PDF, com conversão textual entre TXT, HTML e o outro formato.
/// - Vídeo: MP4, WEBM, MKV e GIF por FFmpeg.
/// - Qualquer arquivo (exceto ZIP): compactação ZIP real.
/// - O formato de origem é omitido; não há conversão "para o mesmo formato".
/// - Formatos sem engine real (DOC, ODT, áudio etc.) não entram na lista.
pub fn supported_targets(path: &Path) -> Vec<TargetFormat> {
    let source_extension = extension_of(path);
    let canonical_source = canonical_extension(&source_extension).to_string();
    let source_category = categorize(&source_extension);
    let decode_as_image = source_category == FileCategory::Image
        && matches!(
            canonical_source.as_str(),
            "jpg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "ico"
        );
    let decode_as_text = matches!(canonical_source.as_str(), "txt" | "md" | "html");
    let decode_as_document = matches!(canonical_source.as_str(), "docx" | "pdf");
    let decode_as_video =
        source_category == FileCategory::Video && crate::engines::video::is_available();

    let mut targets = Vec::new();

    if decode_as_image {
        let image_targets = [
            ("jpg", "JPG", "Foto compacta"),
            ("png", "PNG", "Transparência sem perdas"),
            ("webp", "WEBP", "Imagem leve para web"),
            ("bmp", "BMP", "Bitmap sem compressão"),
            ("gif", "GIF", "GIF estático (1º quadro)"),
            ("tiff", "TIFF", "Alta qualidade"),
        ];

        for (id, label, description) in image_targets {
            if id == canonical_source {
                continue;
            }
            targets.push(TargetFormat {
                id: id.to_string(),
                label: label.to_string(),
                extension: id.to_string(),
                description: description.to_string(),
                kind: "image".to_string(),
            });
        }
    }

    if decode_as_text {
        let text_targets: &[(&str, &str, &str)] = match canonical_source.as_str() {
            "txt" => &[
                ("md", "MD", "Markdown editável"),
                ("html", "HTML", "Página HTML"),
            ],
            "md" => &[
                ("txt", "TXT", "Texto simples"),
                ("html", "HTML", "Página HTML"),
            ],
            "html" => &[("txt", "TXT", "Texto simples")],
            _ => &[],
        };

        for (id, label, description) in text_targets {
            if *id == canonical_source {
                continue;
            }
            targets.push(TargetFormat {
                id: id.to_string(),
                label: label.to_string(),
                extension: id.to_string(),
                description: description.to_string(),
                kind: "text".to_string(),
            });
        }
    }

    if decode_as_document {
        let document_targets: &[(&str, &str, &str)] = match canonical_source.as_str() {
            "docx" => &[
                ("pdf", "PDF", "PDF de texto (sem layout original)"),
                ("html", "HTML", "HTML com o conteúdo do documento"),
                ("txt", "TXT", "Texto simples"),
            ],
            "pdf" => &[
                ("docx", "DOCX", "Documento Word editável (texto)"),
                ("html", "HTML", "HTML com o texto extraído"),
                ("txt", "TXT", "Texto extraído"),
            ],
            _ => &[],
        };

        for (id, label, description) in document_targets {
            if *id == canonical_source {
                continue;
            }
            targets.push(TargetFormat {
                id: id.to_string(),
                label: label.to_string(),
                extension: id.to_string(),
                description: description.to_string(),
                kind: "document".to_string(),
            });
        }
    }

    if decode_as_video {
        let video_targets = [
            ("mp4", "MP4", "Vídeo H.264 + AAC"),
            ("webm", "WEBM", "Vídeo VP9 para web"),
            ("mkv", "MKV", "Contêiner Matroska"),
            ("gif", "GIF", "GIF animado"),
        ];

        for (id, label, description) in video_targets {
            if id == canonical_source {
                continue;
            }
            targets.push(TargetFormat {
                id: id.to_string(),
                label: label.to_string(),
                extension: id.to_string(),
                description: description.to_string(),
                kind: "video".to_string(),
            });
        }
    }

    // ZIP é real para qualquer arquivo, mas não faz sentido recomprimir um ZIP.
    if source_extension != "zip" {
        targets.push(TargetFormat {
            id: "zip".to_string(),
            label: "ZIP".to_string(),
            extension: "zip".to_string(),
            description: "Compactar em um arquivo ZIP".to_string(),
            kind: "archive".to_string(),
        });
    }

    targets
}

/// Versão que valida o caminho antes de listar os formatos.
pub fn output_extension(target_id: &str) -> Option<&'static str> {
    match target_id {
        "jpg" => Some("jpg"),
        "png" => Some("png"),
        "webp" => Some("webp"),
        "bmp" => Some("bmp"),
        "gif" => Some("gif"),
        "tiff" => Some("tiff"),
        "txt" => Some("txt"),
        "md" => Some("md"),
        "html" => Some("html"),
        "docx" => Some("docx"),
        "pdf" => Some("pdf"),
        "mp4" => Some("mp4"),
        "webm" => Some("webm"),
        "mkv" => Some("mkv"),
        "zip" => Some("zip"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_targets_are_real_and_exclude_source() {
        let targets = supported_targets(Path::new("C:/fotos/imagem.png"));
        let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
        assert!(ids.contains(&"jpg"));
        assert!(ids.contains(&"webp"));
        assert!(ids.contains(&"zip"));
        assert!(!ids.contains(&"png"));
        assert!(!ids.contains(&"mp3"));
        assert_eq!(ids.len(), 6);
    }

    #[test]
    fn txt_offers_real_text_targets_and_zip() {
        let targets = supported_targets(Path::new("C:/notas/arquivo.txt"));
        let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
        assert_eq!(ids, ["md", "html", "zip"]);
        assert_eq!(
            targets
                .iter()
                .find(|target| target.id == "html")
                .map(|target| target.kind.as_str()),
            Some("text")
        );
    }

    #[test]
    fn html_only_offers_txt_and_zip() {
        let targets = supported_targets(Path::new("C:/notas/pagina.html"));
        let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
        assert_eq!(ids, ["txt", "zip"]);
    }

    #[test]
    fn docx_offers_pdf_html_txt_and_zip() {
        let targets = supported_targets(Path::new("C:/docs/relatorio.docx"));
        let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
        assert_eq!(ids, ["pdf", "html", "txt", "zip"]);
        assert!(targets
            .iter()
            .all(|target| target.id == "zip" || target.kind == "document"));
    }

    #[test]
    fn pdf_offers_docx_html_txt_and_zip() {
        let targets = supported_targets(Path::new("C:/docs/relatorio.pdf"));
        let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
        assert_eq!(ids, ["docx", "html", "txt", "zip"]);
        assert!(targets
            .iter()
            .all(|target| target.id == "zip" || target.kind == "document"));
    }

    #[test]
    fn video_offers_real_targets_and_excludes_source() {
        if !crate::engines::video::is_available() {
            return;
        }
        let targets = supported_targets(Path::new("C:/videos/clipe.mp4"));
        let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
        assert_eq!(ids, ["webm", "mkv", "gif", "zip"]);
        assert!(targets
            .iter()
            .all(|target| target.id == "zip" || target.kind == "video"));
    }

    #[test]
    fn zip_offers_nothing_because_extraction_is_not_implemented() {
        let targets = supported_targets(Path::new("C:/notas/pacote.zip"));
        assert!(targets.is_empty());
    }

    #[test]
    fn svg_is_not_pretended_as_convertible_image() {
        let targets = supported_targets(Path::new("C:/vetores/logo.svg"));
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "zip");
    }
}
