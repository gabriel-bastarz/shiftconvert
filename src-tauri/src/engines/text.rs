//! Engine de texto.
//! Conversões reais entre TXT, Markdown e HTML:
//! TXT -> MD/HTML, MD -> TXT/HTML e HTML -> TXT.

use std::error::Error;
use std::fs;
use std::path::Path;

use html2text::from_read;
use log::info;
use pulldown_cmark::{html, Options, Parser};

type BoxError = Box<dyn Error + Send + Sync>;

const HTML_STYLE: &str = r#"
    :root { color-scheme: light; }
    body {
      max-width: 74ch;
      margin: 2.5rem auto;
      padding: 0 1.25rem;
      font-family: "Segoe UI", system-ui, sans-serif;
      font-size: 16px;
      line-height: 1.65;
      color: #1f2937;
      background: #ffffff;
    }
    h1, h2, h3, h4, h5, h6 { line-height: 1.25; }
    p { white-space: pre-wrap; }
    pre {
      overflow-x: auto;
      padding: 1rem;
      border-radius: 8px;
      background: #f3f4f6;
    }
    code { font-family: "Cascadia Code", Consolas, monospace; }
    blockquote {
      margin-left: 0;
      padding-left: 1rem;
      border-left: 3px solid #cbd5e1;
      color: #475569;
    }
    table { border-collapse: collapse; }
    th, td { border: 1px solid #cbd5e1; padding: 0.4rem 0.6rem; }
"#;

/// Converte um arquivo de texto de acordo com a extensão de origem.
pub fn convert(source_path: &str, target_format: &str, output_path: &Path) -> Result<(), BoxError> {
    let bytes = fs::read(source_path)?;
    let text = decode_text(&bytes)?;
    let source_format = extension_of(source_path);
    let target = target_format.to_lowercase();

    info!(
        "Convertendo texto: {} ({}) -> {}",
        source_path, source_format, target
    );

    let converted = match source_format.as_str() {
        "txt" => match target.as_str() {
            "md" => text_to_markdown(&text),
            "html" => text_to_html(&text, &title_for(source_path)),
            _ => {
                return Err(format!("conversão de TXT para {target} não suportada").into());
            }
        },
        "md" | "markdown" => match target.as_str() {
            "txt" => html_to_text(&markdown_to_html_body(&text))?,
            "html" => wrap_html(&markdown_to_html_body(&text), &title_for(source_path)),
            _ => {
                return Err(format!("conversão de Markdown para {target} não suportada").into());
            }
        },
        "html" | "htm" => match target.as_str() {
            "txt" => html_to_text(&text)?,
            _ => {
                return Err(format!("conversão de HTML para {target} não suportada").into());
            }
        },
        other => {
            return Err(format!("formato de texto de origem não suportado: {other}").into());
        }
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, converted.as_bytes())?;

    let metadata = fs::metadata(output_path)?;
    if metadata.len() == 0 {
        return Err("arquivo de texto gerado está vazio; conversão descartada".into());
    }

    info!(
        "Texto convertido com sucesso: {:?} ({} bytes)",
        output_path,
        metadata.len()
    );
    Ok(())
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn title_for(source_path: &str) -> String {
    Path::new(source_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Documento")
        .to_string()
}

/// TXT e Markdown são texto puro; a extensão .md mantém o conteúdo válido e
/// garante uma quebra de linha final, como pede a convenção do formato.
fn text_to_markdown(text: &str) -> String {
    let mut output = normalize_newlines(text);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn text_to_html(text: &str, title: &str) -> String {
    let normalized = normalize_newlines(text);
    let mut body = String::new();

    for block in normalized.split("\n\n") {
        if block.trim().is_empty() {
            continue;
        }
        body.push_str("<p>");
        body.push_str(&escape_html(block).replace('\n', "<br>\n"));
        body.push_str("</p>\n");
    }

    wrap_html(&body, title)
}

fn markdown_to_html_body(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

fn html_to_text(html_source: &str) -> Result<String, BoxError> {
    let rendered = from_read(html_source.as_bytes(), 100)
        .map_err(|error| format!("não foi possível ler o HTML: {error}"))?;
    let trimmed = rendered.trim_end();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("{trimmed}\n"))
}

fn wrap_html(body: &str, title: &str) -> String {
    format!(
        "<!doctype html>\n\
         <html lang=\"pt-BR\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n\
         <style>{HTML_STYLE}</style>\n\
         </head>\n\
         <body>\n\
         {body}\
         </body>\n\
         </html>\n",
        escape_html(title)
    )
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn escape_html(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
    output
}

fn decode_text(bytes: &[u8]) -> Result<String, BoxError> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return utf8(rest);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, false);
    }
    utf8(bytes)
}

fn utf8(bytes: &[u8]) -> Result<String, BoxError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("o arquivo de texto não está em UTF-8: {error}"))?;
    Ok(text.to_string())
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Result<String, BoxError> {
    if bytes.len() % 2 != 0 {
        return Err("arquivo UTF-16 com quantidade ímpar de bytes".into());
    }

    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect();

    String::from_utf16(&units)
        .map_err(|error| format!("o arquivo de texto UTF-16 é inválido: {error}").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_removes_markup_and_entities() {
        let text = html_to_text("<h1>Notícia</h1><p>Tom &amp; Jerry<br>linha 2</p>").expect("html");
        assert!(text.contains("Notícia"));
        assert!(text.contains("Tom & Jerry"));
        assert!(text.contains("linha 2"));
        assert!(!text.contains("<h1>"));
    }

    #[test]
    fn text_to_html_escapes_special_characters() {
        let html = text_to_html("a < b & c", "teste");
        assert!(html.contains("a &lt; b &amp; c"));
        assert!(html.contains("<title>teste</title>"));
    }

    #[test]
    fn markdown_to_html_renders_common_markup() {
        let html = markdown_to_html_body("# Título\n\n**forte**");
        assert!(html.contains("<h1>Título</h1>"));
        assert!(html.contains("<strong>forte</strong>"));
    }
}
