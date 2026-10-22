//! Engine ZIP real. Compacta um único arquivo usando a crate `zip`.
//! A origem é apenas lida; nada é movido ou apagado.

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use log::info;
use zip::write::FileOptions;

pub fn compress(
    source_path: &Path,
    output_path: &Path,
    cancel: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Compactando: {:?} -> {:?}", source_path, output_path);

    if !source_path.exists() {
        return Err(format!("arquivo de origem não encontrado: {:?}", source_path).into());
    }
    if !source_path.is_file() {
        return Err("a origem do ZIP precisa ser um arquivo".into());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "nome do arquivo de origem inválido".to_string())?;

    let source = File::open(source_path)?;
    let mut reader = BufReader::new(source);

    let out_file = File::create(output_path)?;
    let mut zip = zip::ZipWriter::new(out_file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    zip.start_file(file_name, options)?;

    let mut buffer = [0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(zip);
            let _ = std::fs::remove_file(output_path);
            return Err("compactação cancelada".into());
        }

        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        zip.write_all(&buffer[..bytes_read])?;
    }

    if cancel.load(Ordering::SeqCst) {
        drop(zip);
        let _ = std::fs::remove_file(output_path);
        return Err("compactação cancelada".into());
    }

    zip.finish()?;

    let metadata = std::fs::metadata(output_path)?;
    if metadata.len() == 0 {
        let _ = std::fs::remove_file(output_path);
        return Err("arquivo ZIP gerado está vazio".into());
    }

    info!(
        "ZIP criado: {:?} ({} bytes)",
        output_path,
        metadata.len()
    );
    Ok(())
}
