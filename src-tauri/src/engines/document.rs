//! Engine de conversão de documentos.
//! Fase 1/2: placeholder. A conversão real de documentos (PDF, DOCX, etc.)
//! requer LibreOffice headless ou uma biblioteca dedicada.
//! Valida a entrada e gera um arquivo de saída vazio como comprovante de
//! conceito, claramente identificado como simulação.

use log::{info, warn};
use std::path::Path;

pub fn convert(
    source_path: &str,
    target_format: &str,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "[SIMULAÇÃO] Conversão de documento: {} -> {} -> {:?}",
        source_path, target_format, output_path
    );

    let source = std::path::Path::new(source_path);
    if !source.exists() {
        return Err(format!("arquivo de origem não encontrado: {}", source_path).into());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // SIMULAÇÃO: cria arquivo de saída vazio.
    std::fs::write(output_path, b"")?;

    warn!(
        "Conversão de documento é SIMULADA. LibreOffice headless não está integrado nesta fase. \
         Arquivo de saída criado como placeholder: {:?}",
        output_path
    );

    Ok(())
}
