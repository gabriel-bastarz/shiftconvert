//! Engine de documentos: DOCX e PDF.
//!
//! - DOCX -> TXT/HTML/PDF: lê o `word/document.xml` do pacote OOXML.
//! - PDF -> TXT/HTML/DOCX: extrai o texto com `pdf-extract`.
//!
//! A conversão é textual. Imagens, colunas, tabelas complexas e o layout
//! original não são preservados; PDFs digitalizados precisam de OCR.

use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use log::info;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::engines::text;

type BoxError = Box<dyn Error + Send + Sync>;

const PDF_PAGE_WIDTH: f32 = 595.0;
const PDF_PAGE_HEIGHT: f32 = 842.0;
const PDF_MARGIN: f32 = 56.0;
const PDF_FONT_SIZE: f32 = 11.0;
const PDF_LINE_HEIGHT: f32 = 15.0;
const PDF_MAX_CHARS: usize = 86;

#[derive(Debug, Clone, Default)]
struct Paragraph {
    text: String,
    heading: Option<u8>,
}

/// Converte um DOCX ou PDF para um dos formatos de documento oferecidos.
pub fn convert(source_path: &str, target_format: &str, output_path: &Path) -> Result<(), BoxError> {
    let source_format = extension_of(source_path);
    let target = target_format.to_lowercase();

    info!(
        "Convertendo documento: {} ({}) -> {}",
        source_path, source_format, target
    );

    match source_format.as_str() {
        "docx" => convert_docx(source_path, &target, output_path),
        "pdf" => convert_pdf(source_path, &target, output_path),
        other => Err(format!("formato de documento de origem não suportado: {other}").into()),
    }
}

fn convert_docx(source_path: &str, target: &str, output_path: &Path) -> Result<(), BoxError> {
    let paragraphs = read_docx(source_path)?;
    let title = title_for(source_path);

    match target {
        "txt" => {
            let text = paragraphs_to_text(&paragraphs);
            write_text(output_path, &text)
        }
        "html" => {
            let body = paragraphs_to_html_body(&paragraphs);
            if body.trim().is_empty() {
                return Err("o documento não contém texto para gerar o HTML".into());
            }
            write_text(output_path, &text::wrap_html(&body, &title))
        }
        "pdf" => write_pdf(&paragraphs, output_path),
        _ => Err(format!("conversão de DOCX para {target} não suportada").into()),
    }
}

fn convert_pdf(source_path: &str, target: &str, output_path: &Path) -> Result<(), BoxError> {
    let text = extract_pdf_text(source_path)?;
    let title = title_for(source_path);

    match target {
        "txt" => write_text(output_path, &normalize_text(&text)),
        "html" => {
            let blocks = blocks_from_text(&text);
            if blocks.is_empty() {
                return Err("o PDF não gerou parágrafos de texto".into());
            }
            let mut body = String::new();
            for block in blocks {
                body.push_str(&format!("<p>{}</p>\n", text::escape_html(&block)));
            }
            write_text(output_path, &text::wrap_html(&body, &title))
        }
        "docx" => {
            let blocks = blocks_from_text(&text);
            if blocks.is_empty() {
                return Err("o PDF não gerou parágrafos de texto".into());
            }
            write_docx(&blocks, output_path)
        }
        _ => Err(format!("conversão de PDF para {target} não suportada").into()),
    }
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn title_for(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Documento")
        .to_string()
}

fn write_text(output_path: &Path, content: &str) -> Result<(), BoxError> {
    if content.trim().is_empty() {
        return Err("o documento não contém texto para converter".into());
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, content.as_bytes())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// DOCX -> texto
// ---------------------------------------------------------------------------

fn read_docx(source_path: &str) -> Result<Vec<Paragraph>, BoxError> {
    let file = File::open(source_path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("DOCX inválido ou corrompido: {error}"))?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|_| "DOCX inválido: word/document.xml não encontrado".to_string())?;

    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .map_err(|error| format!("não foi possível ler o conteúdo do DOCX: {error}"))?;

    parse_docx_xml(&xml)
}

fn parse_docx_xml(xml: &str) -> Result<Vec<Paragraph>, BoxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut buffer = Vec::new();
    let mut paragraphs = Vec::new();
    let mut current = Paragraph::default();
    let mut current_style: Option<String> = None;
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut deleted_depth = 0usize;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => match local_name(event.name().as_ref()) {
                "p" => {
                    current = Paragraph::default();
                    current_style = None;
                    in_paragraph = true;
                }
                "pStyle" => {
                    current_style = attribute_value(&event, "val");
                }
                "t" => in_text = true,
                "tab" if in_paragraph => current.text.push('\t'),
                "br" | "cr" if in_paragraph => current.text.push('\n'),
                "noBreakHyphen" if in_paragraph => current.text.push('-'),
                "del" => deleted_depth += 1,
                _ => {}
            },
            Ok(Event::Empty(event)) => match local_name(event.name().as_ref()) {
                "p" => paragraphs.push(Paragraph::default()),
                "pStyle" => {
                    current_style = attribute_value(&event, "val");
                }
                "tab" if in_paragraph => current.text.push('\t'),
                "br" | "cr" if in_paragraph => current.text.push('\n'),
                "noBreakHyphen" if in_paragraph => current.text.push('-'),
                _ => {}
            },
            Ok(Event::Text(event)) if in_text && deleted_depth == 0 => {
                current
                    .text
                    .push_str(&event.xml_content(XmlVersion::Implicit1_0));
            }
            Ok(Event::CData(event)) if in_text && deleted_depth == 0 => {
                current.text.push_str(event.as_ref());
            }
            Ok(Event::GeneralRef(event)) if in_text && deleted_depth == 0 => {
                if let Ok(Some(character)) = event.resolve_char_ref() {
                    current.text.push(character);
                } else {
                    match event.as_ref() {
                        "amp" => current.text.push('&'),
                        "lt" => current.text.push('<'),
                        "gt" => current.text.push('>'),
                        "quot" => current.text.push('"'),
                        "apos" => current.text.push('\''),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => match local_name(event.name().as_ref()) {
                "p" if in_paragraph => {
                    current.heading = current_style.as_deref().and_then(heading_level);
                    paragraphs.push(current.clone());
                    current = Paragraph::default();
                    in_paragraph = false;
                }
                "t" => in_text = false,
                "del" => deleted_depth = deleted_depth.saturating_sub(1),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(format!("XML do DOCX inválido: {error}").into());
            }
            _ => {}
        }
        buffer.clear();
    }

    if in_paragraph && !current.text.trim().is_empty() {
        current.heading = current_style.as_deref().and_then(heading_level);
        paragraphs.push(current);
    }

    Ok(paragraphs)
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn attribute_value(event: &BytesStart<'_>, name: &str) -> Option<String> {
    event
        .attributes()
        .with_checks(false)
        .flatten()
        .find(|attribute| local_name(attribute.key.as_ref()) == name)
        .map(|attribute| attribute.value.to_string())
}

fn heading_level(style: &str) -> Option<u8> {
    let normalized: String = style
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    if normalized == "title" {
        return Some(1);
    }

    for prefix in ["heading", "titulo", "ttulo"] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            if let Some(level) = rest.chars().next().and_then(|value| value.to_digit(10)) {
                return u8::try_from(level)
                    .ok()
                    .filter(|level| (1..=6).contains(level));
            }
        }
    }
    None
}

fn paragraphs_to_text(paragraphs: &[Paragraph]) -> String {
    let mut output = paragraphs
        .iter()
        .map(|paragraph| paragraph.text.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn paragraphs_to_html_body(paragraphs: &[Paragraph]) -> String {
    let mut body = String::new();

    for paragraph in paragraphs {
        let trimmed = paragraph.text.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tag = match paragraph.heading {
            Some(level) => format!("h{level}"),
            None => "p".to_string(),
        };
        let content = text::escape_html(trimmed).replace('\n', "<br>\n");
        body.push_str(&format!("<{tag}>{content}</{tag}>\n"));
    }

    body
}

// ---------------------------------------------------------------------------
// PDF -> texto
// ---------------------------------------------------------------------------

fn extract_pdf_text(source_path: &str) -> Result<String, BoxError> {
    let bytes = fs::read(source_path)?;
    let extraction = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem(&bytes)
    }));

    let raw = match extraction {
        Ok(Ok(text)) => text,
        Ok(Err(error)) => {
            return Err(format!("não foi possível extrair texto do PDF: {error}").into());
        }
        Err(_) => {
            return Err("não foi possível extrair texto do PDF: o parser falhou".into());
        }
    };

    let normalized = normalize_text(&raw);
    if normalized.trim().is_empty() {
        return Err(
            "o PDF não contém texto extraível (pode ser digitalizado); OCR não está disponível"
                .into(),
        );
    }

    Ok(normalized)
}

fn normalize_text(value: &str) -> String {
    let value = value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{000C}', "\n\n");

    let mut normalized = String::with_capacity(value.len());
    let mut previous_blank = false;

    for line in value.lines() {
        let line = line.trim_end();
        let blank = line.is_empty();
        if blank {
            if previous_blank {
                continue;
            }
            previous_blank = true;
        } else {
            previous_blank = false;
        }
        normalized.push_str(line);
        normalized.push('\n');
    }

    while normalized.ends_with("\n\n") {
        normalized.pop();
    }
    if !normalized.is_empty() && !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

/// Junta linhas de um PDF em parágrafos separados por linhas em branco.
/// Quando o PDF não traz linhas em branco, cada linha vira um parágrafo para
/// evitar um único bloco gigante no DOCX/HTML.
fn blocks_from_text(value: &str) -> Vec<String> {
    let normalized = normalize_text(value);
    let mut blocks = Vec::new();

    if normalized.contains("\n\n") {
        for block in normalized.split("\n\n") {
            let joined = block
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !joined.is_empty() {
                blocks.push(joined);
            }
        }
    } else {
        for line in normalized.lines() {
            let line = line.trim();
            if !line.is_empty() {
                blocks.push(line.to_string());
            }
        }
    }

    blocks
}

// ---------------------------------------------------------------------------
// PDF de saída
// ---------------------------------------------------------------------------

fn write_pdf(paragraphs: &[Paragraph], output_path: &Path) -> Result<(), BoxError> {
    let mut lines = Vec::new();

    for paragraph in paragraphs {
        let text = paragraph.text.trim_end();
        if text.is_empty() {
            lines.push(String::new());
            continue;
        }
        for line in text.lines() {
            lines.extend(wrap_line(line));
        }
    }

    if lines.iter().all(|line| line.trim().is_empty()) {
        return Err("o documento não contém texto para gerar o PDF".into());
    }

    write_pdf_lines(&lines, output_path)
}

fn wrap_line(line: &str) -> Vec<String> {
    let line = line.replace('\t', "    ");
    if line.trim().is_empty() {
        return vec![String::new()];
    }

    let mut output = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > PDF_MAX_CHARS {
            if !current.is_empty() {
                output.push(std::mem::take(&mut current));
            }
            let mut chunk = String::new();
            for character in word.chars() {
                if chunk.chars().count() >= PDF_MAX_CHARS {
                    output.push(std::mem::take(&mut chunk));
                }
                chunk.push(character);
            }
            if !chunk.is_empty() {
                current = chunk;
            }
            continue;
        }

        let current_len = current.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if current_len + 1 + word_len <= PDF_MAX_CHARS {
            current.push(' ');
            current.push_str(word);
        } else {
            output.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        output.push(current);
    }
    output
}

fn write_pdf_lines(lines: &[String], output_path: &Path) -> Result<(), BoxError> {
    let max_lines = ((PDF_PAGE_HEIGHT - PDF_MARGIN * 2.0) / PDF_LINE_HEIGHT).floor() as usize;
    let pages = lines.chunks(max_lines.max(1)).collect::<Vec<_>>();
    let page_count = pages.len().max(1);

    let contents = pages
        .iter()
        .map(|page| {
            let mut content = Vec::new();
            let start_y = PDF_PAGE_HEIGHT - PDF_MARGIN;
            content.extend_from_slice(
                format!(
                    "BT\n/F1 {PDF_FONT_SIZE} Tf\n{PDF_LINE_HEIGHT} TL\n1 0 0 1 {PDF_MARGIN} {start_y} Tm\n"
                )
                .as_bytes(),
            );
            for line in *page {
                content.extend_from_slice(&pdf_literal(line));
                content.extend_from_slice(b" Tj\nT*\n");
            }
            content.extend_from_slice(b"ET\n");
            content
        })
        .collect::<Vec<_>>();

    let first_page_object = 4;
    let first_content_object = first_page_object + page_count;
    let mut objects = Vec::new();

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids = (0..page_count)
        .map(|index| format!("{} 0 R", first_page_object + index))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>").into_bytes());
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );

    for index in 0..page_count {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PDF_PAGE_WIDTH} {PDF_PAGE_HEIGHT}] \
                 /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                first_content_object + index
            )
            .into_bytes(),
        );
    }

    for content in &contents {
        objects.push(
            format!("<< /Length {} >>\nstream\n", content.len())
                .into_bytes()
                .into_iter()
                .chain(content.iter().copied())
                .chain(b"\nendstream".iter().copied())
                .collect::<Vec<u8>>(),
        );
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    fs::write(output_path, pdf)?;
    Ok(())
}

fn pdf_literal(value: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(value.len() + 2);
    output.push(b'(');
    for character in value.chars() {
        let byte = winansi_byte(character);
        match byte {
            b'(' | b')' | b'\\' => {
                output.push(b'\\');
                output.push(byte);
            }
            0x20..=0x7e => output.push(byte),
            _ => {
                output.extend_from_slice(format!("\\{byte:03o}").as_bytes());
            }
        }
    }
    output.push(b')');
    output
}

fn winansi_byte(character: char) -> u8 {
    let code = character as u32;
    if (0x20..=0x7e).contains(&code) || (0xa0..=0xff).contains(&code) {
        return code as u8;
    }

    match character {
        '\u{20ac}' => 0x80,
        '\u{201a}' => 0x82,
        '\u{0192}' => 0x83,
        '\u{201e}' => 0x84,
        '\u{2026}' => 0x85,
        '\u{2020}' => 0x86,
        '\u{2021}' => 0x87,
        '\u{02c6}' => 0x88,
        '\u{2030}' => 0x89,
        '\u{0160}' => 0x8a,
        '\u{2039}' => 0x8b,
        '\u{0152}' => 0x8c,
        '\u{017d}' => 0x8e,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201c}' => 0x93,
        '\u{201d}' => 0x94,
        '\u{2022}' => 0x95,
        '\u{2013}' => 0x96,
        '\u{2014}' => 0x97,
        '\u{02dc}' => 0x98,
        '\u{2122}' => 0x99,
        '\u{0161}' => 0x9a,
        '\u{203a}' => 0x9b,
        '\u{0153}' => 0x9c,
        '\u{017e}' => 0x9e,
        '\u{0178}' => 0x9f,
        '\t' => b' ',
        _ => b'?',
    }
}

// ---------------------------------------------------------------------------
// DOCX de saída
// ---------------------------------------------------------------------------

fn write_docx(paragraphs: &[String], output_path: &Path) -> Result<(), BoxError> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>
"#;

    let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"#;

    let document_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>
"#;

    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/>
        <w:sz w:val="22"/>
        <w:szCs w:val="22"/>
      </w:rPr>
    </w:rPrDefault>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="120" w:line="276" w:lineRule="auto"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:qFormat/>
  </w:style>
</w:styles>
"#;

    let mut document = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
"#,
    );

    for paragraph in paragraphs {
        if paragraph.trim().is_empty() {
            document.push_str("    <w:p/>\n");
            continue;
        }

        document.push_str("    <w:p><w:r>");
        for (index, line) in paragraph.lines().enumerate() {
            if index > 0 {
                document.push_str("<w:br/>");
            }
            document.push_str(&format!(
                "<w:t xml:space=\"preserve\">{}</w:t>",
                xml_escape(line)
            ));
        }
        document.push_str("</w:r></w:p>\n");
    }

    document.push_str(
        r#"    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1134" w:header="709" w:footer="709" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>
"#,
    );

    for (name, content) in [
        ("[Content_Types].xml", content_types),
        ("_rels/.rels", root_rels),
        ("word/_rels/document.xml.rels", document_rels),
        ("word/styles.xml", styles),
    ] {
        zip.start_file(name, options)?;
        zip.write_all(content.as_bytes())?;
    }

    zip.start_file("word/document.xml", options)?;
    zip.write_all(document.as_bytes())?;
    zip.finish()?;

    Ok(())
}

fn xml_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
    output
}

#[cfg(test)]
pub(crate) fn write_test_docx(path: &Path) -> Result<(), BoxError> {
    write_docx(
        &[
            "Documento de teste".to_string(),
            "Segunda linha do documento.".to_string(),
        ],
        path,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "shiftconvert-doc-{label}-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("criar pasta de teste");
        dir
    }

    #[test]
    fn docx_xml_extracts_paragraphs_and_headings() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Título</w:t></w:r></w:p>
    <w:p><w:r><w:t>Tom &amp; Jerry</w:t></w:r></w:p>
    <w:p><w:r><w:t>linha 1</w:t></w:r><w:br/><w:r><w:t>linha 2</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let paragraphs = parse_docx_xml(xml).expect("parsear docx");
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0].text, "Título");
        assert_eq!(paragraphs[0].heading, Some(1));
        assert_eq!(paragraphs[1].text, "Tom & Jerry");
        assert_eq!(paragraphs[2].text, "linha 1\nlinha 2");
    }

    #[test]
    fn writes_valid_docx_that_can_be_read_back() {
        let dir = scratch_dir("docx");
        let path = dir.join("saida.docx");
        let paragraphs = vec![
            "Primeiro parágrafo".to_string(),
            "Segundo <teste> & fim".to_string(),
        ];

        write_docx(&paragraphs, &path).expect("gerar docx");
        let read_back = read_docx(path.to_string_lossy().as_ref()).expect("ler docx");
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].text, "Primeiro parágrafo");
        assert_eq!(read_back[1].text, "Segundo <teste> & fim");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn writes_pdf_that_can_be_extracted_back() {
        let dir = scratch_dir("pdf");
        let path = dir.join("saida.pdf");
        let paragraphs = vec![
            Paragraph {
                text: "Título do documento".to_string(),
                heading: Some(1),
            },
            Paragraph {
                text: "Olá, mundo! Acentuação: ação, coração e informação.".to_string(),
                heading: None,
            },
        ];

        write_pdf(&paragraphs, &path).expect("gerar pdf");
        let bytes = fs::read(&path).expect("ler pdf");
        assert!(bytes.starts_with(b"%PDF-1.4"));

        let text = extract_pdf_text(path.to_string_lossy().as_ref()).expect("extrair pdf");
        assert!(text.contains("Título do documento"));
        assert!(text.contains("Olá, mundo!"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn writes_multipage_pdf_and_extracts_last_line() {
        let dir = scratch_dir("pdf-multipage");
        let path = dir.join("saida.pdf");
        let paragraphs = (0..60)
            .map(|index| Paragraph {
                text: format!("Linha {index} do documento."),
                heading: None,
            })
            .collect::<Vec<_>>();

        write_pdf(&paragraphs, &path).expect("gerar pdf multipágina");
        let text = extract_pdf_text(path.to_string_lossy().as_ref()).expect("extrair pdf");
        assert!(text.contains("Linha 0 do documento."));
        assert!(text.contains("Linha 59 do documento."));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn converts_docx_to_txt_html_and_pdf() {
        let dir = scratch_dir("docx-targets");
        let source = dir.join("origem.docx");
        write_docx(
            &[
                "Capítulo 1".to_string(),
                "Conteúdo do documento.".to_string(),
            ],
            &source,
        )
        .expect("gerar docx");

        for (target, expected_extension) in [("txt", "txt"), ("html", "html"), ("pdf", "pdf")] {
            let output = dir.join(format!("saida.{expected_extension}"));
            convert(source.to_string_lossy().as_ref(), target, &output)
                .unwrap_or_else(|error| panic!("converter docx para {target}: {error}"));
            assert!(output.exists(), "saída {target} ausente");
            assert!(fs::metadata(&output).unwrap().len() > 0);
        }

        let html = fs::read_to_string(dir.join("saida.html")).expect("ler html");
        assert!(html.contains("Capítulo 1"));
        assert!(html.contains("Conteúdo do documento."));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn converts_pdf_to_txt_html_and_docx() {
        let dir = scratch_dir("pdf-targets");
        let source = dir.join("origem.pdf");
        write_pdf(
            &[Paragraph {
                text: "Texto extraído do PDF.".to_string(),
                heading: None,
            }],
            &source,
        )
        .expect("gerar pdf");

        for (target, expected_extension) in [("txt", "txt"), ("html", "html"), ("docx", "docx")] {
            let output = dir.join(format!("saida.{expected_extension}"));
            convert(source.to_string_lossy().as_ref(), target, &output)
                .unwrap_or_else(|error| panic!("converter pdf para {target}: {error}"));
            assert!(output.exists(), "saída {target} ausente");
            assert!(fs::metadata(&output).unwrap().len() > 0);
        }

        let docx = read_docx(dir.join("saida.docx").to_string_lossy().as_ref()).expect("ler docx");
        assert!(docx
            .iter()
            .any(|paragraph| paragraph.text.contains("Texto extraído")));

        let _ = fs::remove_dir_all(dir);
    }
}
