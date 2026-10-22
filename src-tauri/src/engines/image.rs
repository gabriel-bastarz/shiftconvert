//! Engine de conversão de imagens.
//! Utiliza a biblioteca `image` (Rust) para converter entre formatos de imagem.
//! Fase 1/2: conversões reais entre formatos suportados pelo `image` crate.
//! Formatos suportados: png, jpg, jpeg, webp, bmp, gif, tiff.

use image::ImageFormat;
use log::info;
use std::path::Path;

/// Converte uma imagem de um formato para outro.
pub fn convert(
    source_path: &str,
    target_format: &str,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Convertendo imagem: {} -> {} -> {:?}",
        source_path, target_format, output_path
    );

    // Lê a imagem de origem (decodifica automaticamente o formato)
    let img = image::open(source_path)?;

    // Determina o formato de saída com base na extensão alvo
    let format = match target_format.to_lowercase().as_str() {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "webp" => ImageFormat::WebP,
        "bmp" => ImageFormat::Bmp,
        "gif" => ImageFormat::Gif,
        "tiff" | "tif" => ImageFormat::Tiff,
        _ => {
            return Err(format!(
                "formato de imagem não suportado pelo engine: {}",
                target_format
            )
            .into())
        }
    };

    // Salva no caminho de saída
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    img.save_with_format(output_path, format)?;

    // Valida que o arquivo de saída foi criado e não está vazio
    if !output_path.exists() {
        return Err("arquivo de saída não foi gerado".into());
    }
    let metadata = std::fs::metadata(output_path)?;
    if metadata.len() == 0 {
        return Err("arquivo de saída está vazio; conversão falhou".into());
    }

    info!(
        "Imagem convertida com sucesso: {:?} ({} bytes)",
        output_path,
        metadata.len()
    );

    Ok(())
}
