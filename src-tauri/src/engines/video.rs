//! Engine de vídeo via FFmpeg.
//! O FFmpeg é executado como processo externo, sempre com argumentos separados
//! (sem shell). A origem é apenas lida; o arquivo temporário é removido em caso
//! de erro ou cancelamento.

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use log::{info, warn};

type BoxError = Box<dyn Error + Send + Sync>;

static FFMPEG: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Informa se o FFmpeg está disponível para oferecer destinos de vídeo na roda.
pub fn is_available() -> bool {
    ffmpeg_path().is_some()
}

pub fn convert(
    source_path: &str,
    target_format: &str,
    output_path: &Path,
    cancel: &AtomicBool,
) -> Result<(), BoxError> {
    let source = Path::new(source_path);
    if !source.is_file() {
        return Err(format!("arquivo de vídeo não encontrado: {source_path}").into());
    }

    let target = target_format.to_lowercase();
    if !matches!(target.as_str(), "mp4" | "webm" | "mkv" | "gif") {
        return Err(format!("formato de vídeo de destino não suportado: {target}").into());
    }

    let ffmpeg = ffmpeg_path().ok_or_else(|| {
        "FFmpeg não encontrado. Instale com 'winget install Gyan.FFmpeg' \
         ou defina SHIFTCONVERT_FFMPEG com o caminho do ffmpeg.exe."
            .to_string()
    })?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = fs::remove_file(output_path);

    let mut args: Vec<OsString> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-y",
        "-i",
        source_path,
    ]
    .iter()
    .map(|value| OsString::from(*value))
    .collect();

    match target.as_str() {
        "mp4" => {
            args.extend(
                [
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a?",
                    "-sn",
                    "-dn",
                    "-vf",
                    "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "21",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    "-movflags",
                    "+faststart",
                ]
                .iter()
                .map(|value| OsString::from(*value)),
            );
        }
        "webm" => {
            args.extend(
                [
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a?",
                    "-sn",
                    "-dn",
                    "-vf",
                    "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                    "-c:v",
                    "libvpx-vp9",
                    "-crf",
                    "32",
                    "-b:v",
                    "0",
                    "-deadline",
                    "realtime",
                    "-cpu-used",
                    "6",
                    "-row-mt",
                    "1",
                    "-c:a",
                    "libopus",
                    "-b:a",
                    "128k",
                ]
                .iter()
                .map(|value| OsString::from(*value)),
            );
        }
        "mkv" => {
            // Matroska aceita os codecs usuais; remuxar evita perda e espera.
            args.extend(
                ["-map", "0:v:0", "-map", "0:a?", "-sn", "-dn", "-c", "copy"]
                    .iter()
                    .map(|value| OsString::from(*value)),
            );
        }
        "gif" => {
            args.extend(
                [
                    "-map",
                    "0:v:0",
                    "-an",
                    "-sn",
                    "-dn",
                    "-vf",
                    "fps=12,scale=480:-1:flags=lanczos,split[s0][s1];\
                     [s0]palettegen=max_colors=192[p];[s1][p]paletteuse=dither=bayer",
                    "-loop",
                    "0",
                ]
                .iter()
                .map(|value| OsString::from(*value)),
            );
        }
        _ => unreachable!(),
    }

    args.push(output_path.as_os_str().to_os_string());

    info!(
        "Convertendo vídeo com FFmpeg: {} -> {} ({:?})",
        source_path, target, output_path
    );

    run_ffmpeg(&ffmpeg, &args, output_path, cancel)?;

    let metadata = fs::metadata(output_path)?;
    if metadata.len() == 0 {
        let _ = fs::remove_file(output_path);
        return Err("arquivo de vídeo gerado está vazio; conversão descartada".into());
    }

    info!(
        "Vídeo convertido com sucesso: {:?} ({} bytes)",
        output_path,
        metadata.len()
    );
    Ok(())
}

fn run_ffmpeg(
    ffmpeg: &Path,
    args: &[OsString],
    output_path: &Path,
    cancel: &AtomicBool,
) -> Result<(), BoxError> {
    let mut command = Command::new(ffmpeg);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| format!("não foi possível iniciar o FFmpeg: {error}"))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "não foi possível ler a saída de erro do FFmpeg".to_string())?;
    let stderr_reader = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    });

    let mut failure: Option<String> = None;
    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let code = status
                        .code()
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "desconhecido".to_string());
                    failure = Some(format!("FFmpeg terminou com código {code}"));
                }
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(60)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                failure = Some(format!("falha ao acompanhar o FFmpeg: {error}"));
                break;
            }
        }
    }

    let stderr_text = stderr_reader.join().unwrap_or_default();

    if cancel.load(Ordering::SeqCst) {
        let _ = fs::remove_file(output_path);
        return Err("conversão de vídeo cancelada".into());
    }

    if let Some(message) = failure {
        let _ = fs::remove_file(output_path);
        let details = tail(&stderr_text);
        if details.is_empty() {
            return Err(message.into());
        }
        return Err(format!("{message}\n{details}").into());
    }

    Ok(())
}

fn tail(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    lines[lines.len().saturating_sub(8)..].join("\n")
}

fn ffmpeg_path() -> Option<PathBuf> {
    FFMPEG
        .get_or_init(|| {
            if let Some(configured) = std::env::var_os("SHIFTCONVERT_FFMPEG") {
                let path = PathBuf::from(configured);
                if path.is_file() {
                    info!("FFmpeg configurado por SHIFTCONVERT_FFMPEG: {:?}", path);
                    return Some(path);
                }
                warn!(
                    "SHIFTCONVERT_FFMPEG aponta para um arquivo inexistente: {:?}",
                    path
                );
            }

            if command_available("ffmpeg") {
                info!("FFmpeg encontrado no PATH.");
                return Some(PathBuf::from("ffmpeg"));
            }

            if let Some(path) = find_in_winget() {
                info!("FFmpeg encontrado no pacote do WinGet: {:?}", path);
                return Some(path);
            }

            if let Some(path) = find_common_install() {
                info!("FFmpeg encontrado em local comum: {:?}", path);
                return Some(path);
            }

            warn!("FFmpeg não foi encontrado.");
            None
        })
        .clone()
}

fn command_available(name: &str) -> bool {
    let mut command = Command::new(name);
    command
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console(&mut command);
    command
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn find_in_winget() -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")?;
    let winget = PathBuf::from(local_app_data)
        .join("Microsoft")
        .join("WinGet");

    let links = winget.join("Links").join("ffmpeg.exe");
    if links.is_file() {
        return Some(links);
    }

    let packages = winget.join("Packages");
    let entries = fs::read_dir(packages).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("Gyan.FFmpeg") {
            continue;
        }

        let direct = entry.path().join("bin").join("ffmpeg.exe");
        if direct.is_file() {
            return Some(direct);
        }

        if let Ok(builds) = fs::read_dir(entry.path()) {
            for build in builds.flatten() {
                let candidate = build.path().join("bin").join("ffmpeg.exe");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn find_common_install() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("ffmpeg").join("ffmpeg.exe"));
            candidates.push(dir.join("resources").join("ffmpeg").join("ffmpeg.exe"));
            candidates.push(dir.join("ffmpeg.exe"));
        }
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(
            PathBuf::from(program_files)
                .join("ffmpeg")
                .join("bin")
                .join("ffmpeg.exe"),
        );
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(
            PathBuf::from(program_files_x86)
                .join("ffmpeg")
                .join("bin")
                .join("ffmpeg.exe"),
        );
    }
    candidates.push(PathBuf::from(r"C:\ffmpeg\bin\ffmpeg.exe"));

    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("shiftconvert-video-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).expect("criar pasta de teste");
        dir
    }

    #[test]
    fn converts_generated_video_with_ffmpeg() {
        let Some(ffmpeg) = ffmpeg_path() else {
            return;
        };

        let dir = scratch_dir();
        let source = dir.join("vídeo-teste.mp4");
        let source_arg = source.to_string_lossy().to_string();

        let mut generator = Command::new(&ffmpeg);
        generator.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x120:rate=12",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            source_arg.as_str(),
        ]);
        hide_console(&mut generator);
        let status = generator.status().expect("gerar vídeo de teste");
        assert!(status.success(), "não foi possível gerar o vídeo de teste");

        let before = fs::read(&source).expect("ler origem");
        let cancel = AtomicBool::new(false);

        let mkv = dir.join("saida.mkv");
        convert(&source_arg, "mkv", &mkv, &cancel).expect("converter para MKV");
        assert!(fs::metadata(&mkv).unwrap().len() > 0);

        let gif = dir.join("saida.gif");
        convert(&source_arg, "gif", &gif, &cancel).expect("converter para GIF");
        assert!(fs::metadata(&gif).unwrap().len() > 0);

        let mp4 = dir.join("saida.mp4");
        convert(&source_arg, "mp4", &mp4, &cancel).expect("converter para MP4");
        assert!(fs::metadata(&mp4).unwrap().len() > 0);

        let webm = dir.join("saida.webm");
        convert(&source_arg, "webm", &webm, &cancel).expect("converter para WEBM");
        assert!(fs::metadata(&webm).unwrap().len() > 0);

        assert_eq!(fs::read(&source).unwrap(), before, "origem foi alterada");
        let _ = fs::remove_dir_all(dir);
    }
}
