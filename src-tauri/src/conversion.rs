use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::engines::{compression, document, image as image_engine, text, video};
use crate::file_info::{self, canonical_extension, output_extension, TargetFormat};

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionRequest {
    pub source_path: String,
    pub target_format: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionProgress {
    pub conversion_id: String,
    pub source_path: String,
    pub target_format: String,
    /// in-progress | completed | failed | cancelled
    pub status: String,
    pub progress: f64,
    pub message: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

/// Canal de eventos da conversão. O onboarding usa um canal separado para
/// que o HUD global não reaja a uma conversão iniciada dentro do guia.
#[derive(Debug, Clone, Copy)]
pub enum EventChannel {
    Global,
    Onboarding,
}

impl EventChannel {
    fn event_name(self, event: &str) -> String {
        match self {
            Self::Global => format!("conversion-{event}"),
            Self::Onboarding => format!("onboarding-conversion-{event}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreparedJob {
    pub source: PathBuf,
    pub target: TargetFormat,
    pub output: PathBuf,
    pub temp: PathBuf,
}

#[derive(Debug)]
pub enum Failure {
    Failed(String),
    Cancelled,
}

impl Failure {
    fn message(&self) -> String {
        match self {
            Self::Failed(message) => message.clone(),
            Self::Cancelled => "Conversão cancelada.".to_string(),
        }
    }

    fn status(&self) -> &'static str {
        match self {
            Self::Failed(_) => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(1);

fn registry() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("conv-{millis}-{counter}")
}

// ---------------------------------------------------------------------------
// Comando Tauri
// ---------------------------------------------------------------------------

/// Inicia a conversão em uma thread separada e devolve imediatamente o id.
/// O frontend acompanha pelos eventos `conversion-progress`,
/// `conversion-complete` e `conversion-error`.
pub fn start_conversion(app: AppHandle, request: ConversionRequest) -> Result<String, String> {
    start_conversion_with_channel(app, request, EventChannel::Global)
}

/// Variante usada pelo onboarding. Os eventos usam o prefixo
/// `onboarding-conversion-*` e não alimentam o HUD principal.
pub fn start_onboarding_conversion(
    app: AppHandle,
    request: ConversionRequest,
) -> Result<String, String> {
    start_conversion_with_channel(app, request, EventChannel::Onboarding)
}

fn start_conversion_with_channel(
    app: AppHandle,
    request: ConversionRequest,
    channel: EventChannel,
) -> Result<String, String> {
    if request.source_path.trim().is_empty() {
        return Err("caminho de origem vazio".to_string());
    }
    if request.target_format.trim().is_empty() {
        return Err("formato de destino vazio".to_string());
    }

    let id = next_id();
    let cancel = Arc::new(AtomicBool::new(false));
    registry()
        .lock()
        .map_err(|_| "registro de conversões indisponível".to_string())?
        .insert(id.clone(), cancel.clone());

    let thread_id = id.clone();
    let thread_request = request.clone();
    std::thread::Builder::new()
        .name(format!("shiftconvert-{thread_id}"))
        .spawn(move || {
            run_conversion(&app, &thread_id, &thread_request, &cancel, channel);
            if let Ok(mut guard) = registry().lock() {
                guard.remove(&thread_id);
            }
        })
        .map_err(|error| {
            if let Ok(mut guard) = registry().lock() {
                guard.remove(&id);
            }
            format!("não foi possível iniciar a conversão: {error}")
        })?;

    Ok(id)
}

pub fn cancel_conversion(conversion_id: String) -> bool {
    let Ok(guard) = registry().lock() else {
        return false;
    };
    if let Some(flag) = guard.get(&conversion_id) {
        flag.store(true, Ordering::SeqCst);
        info!("Cancelamento solicitado para {}", conversion_id);
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Execução
// ---------------------------------------------------------------------------

fn base_progress(
    id: &str,
    request: &ConversionRequest,
    status: &str,
    progress: f64,
    message: &str,
    output_path: Option<String>,
    error: Option<String>,
) -> ConversionProgress {
    ConversionProgress {
        conversion_id: id.to_string(),
        source_path: request.source_path.clone(),
        target_format: request.target_format.clone(),
        status: status.to_string(),
        progress,
        message: message.to_string(),
        output_path,
        error,
    }
}

fn emit_progress(
    app: &AppHandle,
    channel: EventChannel,
    payload: &ConversionProgress,
) {
    if let Err(error) = app.emit(&channel.event_name("progress"), payload) {
        warn!("Falha ao emitir progresso: {}", error);
    }
}

fn emit_terminal(
    app: &AppHandle,
    channel: EventChannel,
    event: &str,
    payload: &ConversionProgress,
) {
    if let Err(error) = app.emit(&channel.event_name(event), payload) {
        warn!("Falha ao emitir {}: {}", event, error);
    }
}

fn run_conversion(
    app: &AppHandle,
    id: &str,
    request: &ConversionRequest,
    cancel: &AtomicBool,
    channel: EventChannel,
) {
    emit_progress(
        app,
        channel,
        &base_progress(
            id,
            request,
            "in-progress",
            0.04,
            "Validando arquivo…",
            None,
            None,
        ),
    );

    let job = match prepare_job(request, id) {
        Ok(job) => job,
        Err(failure) => {
            let message = failure.message();
            error!("Conversão {} falhou na preparação: {}", id, message);
            emit_terminal(
                app,
                channel,
                "error",
                &base_progress(
                    id,
                    request,
                    failure.status(),
                    0.0,
                    &message,
                    None,
                    Some(message.clone()),
                ),
            );
            return;
        }
    };

    emit_progress(
        app,
        channel,
        &base_progress(
            id,
            request,
            "in-progress",
            0.16,
            "Preparando destino…",
            None,
            None,
        ),
    );

    if cancel.load(Ordering::SeqCst) {
        cleanup(&job.temp);
        emit_terminal(
            app,
            channel,
            "error",
            &base_progress(
                id,
                request,
                "cancelled",
                0.0,
                "Conversão cancelada.",
                None,
                Some("Conversão cancelada.".to_string()),
            ),
        );
        return;
    }

    emit_progress(
        app,
        channel,
        &base_progress(
            id,
            request,
            "in-progress",
            0.30,
            &format!("Convertendo para {}…", job.target.label),
            None,
            None,
        ),
    );

    match execute_job(&job, cancel) {
        Ok(output) => {
            let output_text = output.to_string_lossy().to_string();
            info!("Conversão {} concluída: {}", id, output_text);
            emit_terminal(
                app,
                channel,
                "complete",
                &base_progress(
                    id,
                    request,
                    "completed",
                    1.0,
                    "Conversão concluída.",
                    Some(output_text),
                    None,
                ),
            );
        }
        Err(failure) => {
            let message = failure.message();
            warn!("Conversão {} encerrada: {}", id, message);
            emit_terminal(
                app,
                channel,
                "error",
                &base_progress(
                    id,
                    request,
                    failure.status(),
                    0.0,
                    &message,
                    None,
                    Some(message.clone()),
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Preparação e execução testável
// ---------------------------------------------------------------------------

pub fn prepare_job(request: &ConversionRequest, id: &str) -> Result<PreparedJob, Failure> {
    let file = file_info::inspect_file(&request.source_path)
        .map_err(|message| Failure::Failed(message))?;

    if !file.is_readable {
        return Err(Failure::Failed(
            "sem permissão para ler o arquivo de origem".to_string(),
        ));
    }

    let source = PathBuf::from(&file.path);
    let targets = file_info::supported_targets(&source);
    let target = targets
        .iter()
        .find(|target| target.id == request.target_format.to_lowercase())
        .cloned()
        .ok_or_else(|| {
            Failure::Failed(format!(
                "a conversão para {} não está disponível para este arquivo",
                request.target_format.to_uppercase()
            ))
        })?;

    let extension = output_extension(&target.id)
        .ok_or_else(|| Failure::Failed("formato de destino desconhecido".to_string()))?;

    let output = unique_output_path(&source, extension);
    if output == source {
        return Err(Failure::Failed(
            "a saída seria o próprio arquivo de origem".to_string(),
        ));
    }

    let parent = source
        .parent()
        .ok_or_else(|| Failure::Failed("origem sem pasta de destino".to_string()))?;
    if !parent.exists() {
        return Err(Failure::Failed(
            "a pasta do arquivo de origem não existe".to_string(),
        ));
    }

    // O temporário precisa manter a extensão final porque o FFmpeg escolhe o
    // contêiner de saída a partir dela. O ponto inicial o mantém discreto.
    let temp = parent.join(format!(".shiftconvert-{id}.{extension}"));

    Ok(PreparedJob {
        source,
        target,
        output,
        temp,
    })
}

pub fn execute_job(job: &PreparedJob, cancel: &AtomicBool) -> Result<PathBuf, Failure> {
    if job.source == job.output {
        return Err(Failure::Failed(
            "o destino não pode sobrescrever o original".to_string(),
        ));
    }

    cleanup(&job.temp);

    let result = match job.target.kind.as_str() {
        "archive" => compression::compress(&job.source, &job.temp, cancel),
        "text" => text::convert(
            job.source.to_string_lossy().as_ref(),
            &job.target.id,
            &job.temp,
        ),
        "document" => document::convert(
            job.source.to_string_lossy().as_ref(),
            &job.target.id,
            &job.temp,
        ),
        "video" => video::convert(
            job.source.to_string_lossy().as_ref(),
            &job.target.id,
            &job.temp,
            cancel,
        ),
        _ => image_engine::convert(
            job.source.to_string_lossy().as_ref(),
            &job.target.id,
            &job.temp,
        ),
    };

    if let Err(error) = result {
        let was_cancelled = cancel.load(Ordering::SeqCst);
        cleanup(&job.temp);
        if was_cancelled {
            return Err(Failure::Cancelled);
        }
        return Err(Failure::Failed(format!("falha na conversão: {error}")));
    }

    if cancel.load(Ordering::SeqCst) {
        cleanup(&job.temp);
        return Err(Failure::Cancelled);
    }

    let metadata = fs::metadata(&job.temp)
        .map_err(|error| Failure::Failed(format!("saída temporária ausente: {error}")))?;
    if metadata.len() == 0 {
        cleanup(&job.temp);
        return Err(Failure::Failed(
            "o arquivo gerado ficou vazio; a conversão foi descartada".to_string(),
        ));
    }

    // O rename é atômico dentro da mesma pasta. Se outro processo criou o
    // destino nesse meio-tempo, escolhe um novo sufixo em vez de sobrescrever.
    let mut output = job.output.clone();
    for attempt in 0..8 {
        match fs::rename(&job.temp, &output) {
            Ok(()) => {
                if !output.exists() {
                    return Err(Failure::Failed(
                        "o arquivo de saída não foi encontrado após a gravação".to_string(),
                    ));
                }
                return Ok(output);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                output = unique_output_path(&job.source, &job.target.extension);
                if attempt == 7 {
                    cleanup(&job.temp);
                    return Err(Failure::Failed(
                        "não foi possível escolher um nome de saída livre".to_string(),
                    ));
                }
            }
            Err(error) => {
                cleanup(&job.temp);
                return Err(Failure::Failed(format!(
                    "não foi possível finalizar o arquivo: {error}"
                )));
            }
        }
    }

    cleanup(&job.temp);
    Err(Failure::Failed(
        "não foi possível finalizar o arquivo".to_string(),
    ))
}

fn cleanup(path: &Path) {
    if path.exists() {
        if let Err(error) = fs::remove_file(path) {
            warn!("Não foi possível remover temporário {:?}: {}", path, error);
        }
    }
}

/// Encontra um nome livre ao lado do original. Nunca reutiliza o nome da origem
/// e nunca sobrescreve um arquivo existente.
pub fn unique_output_path(source: &Path, extension: &str) -> PathBuf {
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    let extension = canonical_extension(extension);
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("arquivo");

    let source_extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    let same_extension = canonical_extension(&source_extension) == extension;
    let base = if same_extension {
        format!("{stem} (convertido)")
    } else {
        stem.to_string()
    };

    let mut candidate = parent.join(format!("{base}.{extension}"));
    if candidate != source && !candidate.exists() {
        return candidate;
    }

    for index in 1..10_000 {
        candidate = parent.join(format!("{base} ({index}).{extension}"));
        if candidate != source && !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{base} (convertido).{extension}"))
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_events_use_a_separate_namespace() {
        assert_eq!(
            EventChannel::Global.event_name("progress"),
            "conversion-progress"
        );
        assert_eq!(
            EventChannel::Onboarding.event_name("progress"),
            "onboarding-conversion-progress"
        );
        assert_eq!(
            EventChannel::Onboarding.event_name("complete"),
            "onboarding-conversion-complete"
        );
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "shiftconvert-test-{label}-{}-{}",
            std::process::id(),
            next_id()
        ));
        fs::create_dir_all(&dir).expect("criar pasta de teste");
        dir
    }

    #[test]
    fn unique_output_avoids_collision() {
        let dir = scratch_dir("collision");
        let source = dir.join("foto.png");
        fs::write(&source, b"origem").unwrap();
        let first = unique_output_path(&source, "jpg");
        assert_eq!(first.file_name().unwrap(), "foto.jpg");
        fs::write(&first, b"ocupado").unwrap();
        let second = unique_output_path(&source, "jpg");
        assert_eq!(second.file_name().unwrap(), "foto (1).jpg");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn same_extension_never_uses_source_name() {
        let dir = scratch_dir("same");
        let source = dir.join("foto.png");
        fs::write(&source, b"origem").unwrap();
        let output = unique_output_path(&source, "png");
        assert_ne!(output, source);
        assert!(output.file_name().unwrap().to_string_lossy().contains("convertido"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepares_and_executes_real_image_conversion_without_touching_source() {
        let dir = scratch_dir("image");
        let source = dir.join("imagem.png");
        fs::copy(fixture("imagem-teste.png"), &source).unwrap();
        let before = fs::read(&source).unwrap();

        let request = ConversionRequest {
            source_path: source.to_string_lossy().to_string(),
            target_format: "jpg".to_string(),
        };
        let job = prepare_job(&request, "test-image").expect("preparar");
        assert_eq!(job.target.id, "jpg");
        assert_eq!(job.output.extension().unwrap(), "jpg");

        let cancel = AtomicBool::new(false);
        let output = execute_job(&job, &cancel).expect("converter");
        assert!(output.exists());
        assert!(fs::metadata(&output).unwrap().len() > 0);
        assert_eq!(fs::read(&source).unwrap(), before, "origem foi alterada");
        assert!(!job.temp.exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepares_and_executes_real_text_conversion_without_touching_source() {
        let dir = scratch_dir("text");
        let source = dir.join("documento.txt");
        fs::copy(fixture("documento-teste.txt"), &source).unwrap();
        let before = fs::read(&source).unwrap();

        let request = ConversionRequest {
            source_path: source.to_string_lossy().to_string(),
            target_format: "html".to_string(),
        };
        let job = prepare_job(&request, "test-text").expect("preparar");
        assert_eq!(job.target.id, "html");
        assert_eq!(job.output.extension().unwrap(), "html");
        assert_eq!(job.temp.extension().unwrap(), "html");

        let cancel = AtomicBool::new(false);
        let output = execute_job(&job, &cancel).expect("converter");
        let html = fs::read_to_string(&output).expect("ler html");
        assert!(html.contains("ShiftConvert"));
        assert!(html.contains("&amp;"));
        assert_eq!(fs::read(&source).unwrap(), before, "origem foi alterada");
        assert!(!job.temp.exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepares_and_executes_real_document_conversion_without_touching_source() {
        let dir = scratch_dir("document");
        let source = dir.join("documento.docx");
        document::write_test_docx(&source).expect("gerar docx de teste");
        let before = fs::read(&source).unwrap();

        let request = ConversionRequest {
            source_path: source.to_string_lossy().to_string(),
            target_format: "txt".to_string(),
        };
        let job = prepare_job(&request, "test-document").expect("preparar");
        assert_eq!(job.target.id, "txt");
        assert_eq!(job.target.kind, "document");
        assert_eq!(job.output.extension().unwrap(), "txt");
        assert_eq!(job.temp.extension().unwrap(), "txt");

        let cancel = AtomicBool::new(false);
        let output = execute_job(&job, &cancel).expect("converter documento");
        let text = fs::read_to_string(&output).expect("ler txt");
        assert!(text.contains("Documento de teste"));
        assert!(text.contains("Segunda linha do documento."));
        assert_eq!(fs::read(&source).unwrap(), before, "origem foi alterada");
        assert!(!job.temp.exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prepares_and_executes_real_zip_conversion_without_touching_source() {
        let dir = scratch_dir("zip");
        let source = dir.join("documento.txt");
        fs::copy(fixture("documento-teste.txt"), &source).unwrap();
        let before = fs::read(&source).unwrap();

        let request = ConversionRequest {
            source_path: source.to_string_lossy().to_string(),
            target_format: "zip".to_string(),
        };
        let job = prepare_job(&request, "test-zip").expect("preparar");
        let cancel = AtomicBool::new(false);
        let output = execute_job(&job, &cancel).expect("compactar");
        assert!(output.exists());
        assert_eq!(output.extension().unwrap(), "zip");
        assert_eq!(fs::read(&source).unwrap(), before, "origem foi alterada");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_simulated_or_unknown_targets() {
        let dir = scratch_dir("reject");
        let source = dir.join("documento.txt");
        fs::copy(fixture("documento-teste.txt"), &source).unwrap();
        let request = ConversionRequest {
            source_path: source.to_string_lossy().to_string(),
            target_format: "mp3".to_string(),
        };
        let result = prepare_job(&request, "test-reject");
        assert!(matches!(result, Err(Failure::Failed(_))));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn every_offered_image_target_converts() {
        let dir = scratch_dir("all-image-targets");
        let source = dir.join("imagem.png");
        fs::copy(fixture("imagem-teste.png"), &source).unwrap();
        let before = fs::read(&source).unwrap();

        for (index, target) in ["jpg", "webp", "bmp", "gif", "tiff"].iter().enumerate() {
            let request = ConversionRequest {
                source_path: source.to_string_lossy().to_string(),
                target_format: target.to_string(),
            };
            let job = prepare_job(&request, &format!("test-all-{index}"))
                .unwrap_or_else(|_| panic!("preparar {target}"));
            let cancel = AtomicBool::new(false);
            let output = execute_job(&job, &cancel)
                .unwrap_or_else(|_| panic!("converter para {target}"));
            assert!(output.exists(), "saída {target} ausente");
            assert!(fs::metadata(&output).unwrap().len() > 0, "saída {target} vazia");
        }

        assert_eq!(fs::read(&source).unwrap(), before, "origem foi alterada");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cancellation_removes_temp_and_keeps_source() {
        let dir = scratch_dir("cancel");
        let source = dir.join("documento.txt");
        fs::copy(fixture("documento-teste.txt"), &source).unwrap();
        let before = fs::read(&source).unwrap();

        let request = ConversionRequest {
            source_path: source.to_string_lossy().to_string(),
            target_format: "zip".to_string(),
        };
        let job = prepare_job(&request, "test-cancel").expect("preparar");
        let cancel = AtomicBool::new(true);
        let result = execute_job(&job, &cancel);
        assert!(matches!(result, Err(Failure::Cancelled)));
        assert!(!job.temp.exists());
        assert_eq!(fs::read(&source).unwrap(), before);
        let _ = fs::remove_dir_all(dir);
    }
}
