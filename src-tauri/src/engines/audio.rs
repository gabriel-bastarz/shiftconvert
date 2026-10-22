//! Engine de conversão de áudio.
//! Fase 1/2: placeholder. A conversão real de áudio requer FFmpeg ou uma
//! biblioteca de decodificação/encode de áudio (ex: rodio, cpal, ffmpeg-next).
//! Por enquanto, valida a entrada e gera um arquivo de saída vazio como
//! comprovante de conceito, claramente identificado como simulação.

use log::{info, warn};
use std::path::Path;

pub fn convert(
    source_path: &str,
    target_format: &str,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[SIMULAÇÃO] Conversão de áudio: {} -> {} -> {:?}",
        source_path, target_format, output_path
    );

    // Valida que o arquivo de origem existe
    let source = std::path::Path::new(source_path);
    if !source.exists() {
        return Err(format!("arquivo de origem não encontrado: {}", source_path).into());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // SIMULAÇÃO: cria um arquivo de saída vazio para validar o fluxo.
    // Em produção, aqui seria invocado o FFmpeg com argumentos separados
    // (não concatenação de strings de caminho).
    std::fs::write(output_path, b"")?;

    warn!(
        "Conversão de áudio é SIMULADA. FFmpeg não está integrado nesta fase. \
         Arquivo de saída criado como placeholder: {:?}",
        output_path
    );

    Ok(())
}
