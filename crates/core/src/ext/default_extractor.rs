//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! The default attachment text extractor shipped with mailboxd.
//!
//! Registered at server startup via [`install`], it replaces the
//! [`NoopExtractor`](super::text_extractor) so attachment contents are
//! full-text indexed out of the box. All backends are pure Rust (no C system
//! dependencies), so the binary builds on the slim Docker image without extra
//! apt packages.
//!
//! Supported types:
//!
//! | Category      | Extensions                     | Backend            |
//! |---------------|--------------------------------|--------------------|
//! | PDF           | `pdf`                          | `pdf-extract`      |
//! | Spreadsheets  | `xls`, `xlsx`, `ods`           | `calamine`         |
//! | Word / slides | `docx`, `pptx`                 | `zip` + `quick-xml`|
//! | OpenDocument  | `odt`, `odp`                   | `zip` + `quick-xml`|
//! | Rich text     | `rtf`                          | `rtf-parser`       |
//! | Plain text    | `txt` and every `text/*` type  | UTF-8 / html2text  |
//!
//! Legacy binary Office formats (`doc`, `ppt`, pre-2007) have no maintained
//! pure-Rust extractor and are skipped; they simply return no text. OCR of
//! image-only PDFs is out of scope, so `is_ocr` is always `false`.

use std::io::{Cursor, Read};
use std::panic::{catch_unwind, AssertUnwindSafe};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::ext::text_extractor::{set_extractor, AttachmentTextExtractor, ExtractedText};

/// The concrete extractor covering the file types listed in the module docs.
pub struct DefaultAttachmentExtractor;

/// Register [`DefaultAttachmentExtractor`] as the process-wide extractor.
///
/// Call once during server startup, before the IMAP sync pipeline runs.
pub fn install() {
    set_extractor(Box::new(DefaultAttachmentExtractor));
}

impl AttachmentTextExtractor for DefaultAttachmentExtractor {
    fn extract(&self, content_type: &str, ext: &str, bytes: &[u8]) -> Option<ExtractedText> {
        // Third-party parsers can panic on malformed input. Extraction runs as
        // a single spawn_blocking batch for the whole message, so an unguarded
        // panic here would drop text for every attachment on that message.
        // Isolate each file behind catch_unwind and degrade to "no text".
        catch_unwind(AssertUnwindSafe(|| extract_inner(content_type, ext, bytes)))
            .ok()
            .flatten()
    }
}

fn extract_inner(content_type: &str, ext: &str, bytes: &[u8]) -> Option<ExtractedText> {
    if bytes.is_empty() {
        return None;
    }

    let raw = match ext {
        "pdf" => extract_pdf(bytes)?,
        "xls" | "xlsx" | "ods" => extract_spreadsheet(bytes)?,
        "docx" => extract_ooxml(bytes, OoxmlKind::Docx)?,
        "pptx" => extract_ooxml(bytes, OoxmlKind::Pptx)?,
        "odt" | "odp" => extract_odf(bytes)?,
        "rtf" => extract_rtf(bytes)?,
        "txt" => decode_text(bytes),
        // Legacy binary Word/PowerPoint (pre-2007): no maintained pure-Rust
        // extractor. Skip gracefully. See issue #4.
        "doc" | "ppt" => return None,
        _ if content_type.starts_with("text/") => {
            if content_type.eq_ignore_ascii_case("text/html") {
                crate::utils::html::extract_text(decode_text(bytes))
            } else {
                decode_text(bytes)
            }
        }
        _ => return None,
    };

    let text = normalize_whitespace(&raw);
    (!text.is_empty()).then_some(ExtractedText {
        text,
        page_count: None,
        is_ocr: false,
    })
}

/// Extract text from a PDF via `pdf-extract` (pure Rust, backed by `lopdf`).
fn extract_pdf(bytes: &[u8]) -> Option<String> {
    pdf_extract::extract_text_from_mem(bytes).ok()
}

/// Extract text from `.xls`/`.xlsx`/`.ods` spreadsheets via `calamine`.
///
/// Cell values are joined with spaces per row and rows with newlines; the
/// output is only used for full-text search, so layout fidelity is irrelevant.
fn extract_spreadsheet(bytes: &[u8]) -> Option<String> {
    use calamine::{Data, Reader};

    let mut workbook = calamine::open_workbook_auto_from_rs(Cursor::new(bytes)).ok()?;
    let mut out = String::new();
    for (_name, range) in workbook.worksheets() {
        for row in range.rows() {
            let mut first = true;
            for cell in row {
                let value = match cell {
                    Data::Empty | Data::Error(_) => continue,
                    Data::String(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(dt) => dt.to_string(),
                    Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
                };
                if value.is_empty() {
                    continue;
                }
                if !first {
                    out.push(' ');
                }
                out.push_str(&value);
                first = false;
            }
            out.push('\n');
        }
    }
    Some(out)
}

enum OoxmlKind {
    Docx,
    Pptx,
}

/// Extract text from OOXML containers (`.docx`, `.pptx`).
///
/// A `.docx` keeps its body in `word/document.xml`; a `.pptx` spreads text
/// across `ppt/slides/slideN.xml`. Both are ZIP archives of XML, so we unzip
/// the relevant parts and collect their text nodes.
fn extract_ooxml(bytes: &[u8], kind: OoxmlKind) -> Option<String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let mut out = String::new();
    match kind {
        OoxmlKind::Docx => {
            let mut xml = Vec::new();
            archive.by_name("word/document.xml").ok()?.read_to_end(&mut xml).ok()?;
            out.push_str(&xml_text(&xml));
        }
        OoxmlKind::Pptx => {
            for i in 0..archive.len() {
                let mut file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let name = file.name().to_string();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    let mut xml = Vec::new();
                    if file.read_to_end(&mut xml).is_ok() {
                        out.push('\n');
                        out.push_str(&xml_text(&xml));
                    }
                }
            }
        }
    }
    Some(out)
}

/// Extract text from OpenDocument text/presentation files (`.odt`, `.odp`),
/// whose body lives in `content.xml`.
fn extract_odf(bytes: &[u8]) -> Option<String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let mut xml = Vec::new();
    archive.by_name("content.xml").ok()?.read_to_end(&mut xml).ok()?;
    Some(xml_text(&xml))
}

/// Collect the character data of an XML document, inserting newlines on
/// paragraph/heading/row boundaries so words from adjacent blocks stay
/// separated. Works uniformly for OOXML (`w:t`, `a:t`) and ODF (`text:p`)
/// because we match on the *local* element name.
fn xml_text(xml: &[u8]) -> String {
    let mut reader = Reader::from_reader(xml);
    let mut out = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                if let Ok(text) = quick_xml::escape::unescape(&e) {
                    out.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                if matches!(e.local_name().as_ref(), "p" | "h" | "tr" | "table-row") {
                    out.push('\n');
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

/// Extract text from a Rich Text Format document via `rtf-parser`.
fn extract_rtf(bytes: &[u8]) -> Option<String> {
    let source = String::from_utf8_lossy(bytes);
    rtf_parser::RtfDocument::try_from(source.as_ref())
        .ok()
        .map(|doc| doc.get_text())
}

/// Decode raw bytes as UTF-8 text, replacing invalid sequences. Good enough
/// for indexing `text/*` payloads whose declared charset we do not trust.
fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Collapse all runs of whitespace into single spaces, matching how the
/// message body text is normalized before indexing.
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut buf));
            for (name, data) in entries {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn extracts_plain_text() {
        let result = DefaultAttachmentExtractor
            .extract("text/plain", "txt", b"plain text content")
            .expect("plain text should extract");
        assert_eq!(result.text, "plain text content");
        assert!(!result.is_ocr);
        assert_eq!(result.page_count, None);
    }

    #[test]
    fn strips_html_attachments() {
        let html = b"<html><body><p>Hello</p><p>World</p></body></html>";
        let result = DefaultAttachmentExtractor
            .extract("text/html", "html", html)
            .expect("html should extract");
        assert!(result.text.contains("Hello"), "got: {}", result.text);
        assert!(result.text.contains("World"), "got: {}", result.text);
    }

    #[test]
    fn extracts_rtf() {
        let rtf = br"{\rtf1\ansi\deff0 Hello RTF world\par}";
        let result = DefaultAttachmentExtractor
            .extract("application/rtf", "rtf", rtf)
            .expect("rtf should extract");
        assert!(result.text.contains("Hello"), "got: {}", result.text);
    }

    #[test]
    fn extracts_docx_body() {
        let document = br#"<?xml version="1.0" encoding="UTF-8"?>
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
              <w:body>
                <w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t xml:space="preserve"> World</w:t></w:r></w:p>
                <w:p><w:r><w:t>Second</w:t></w:r></w:p>
              </w:body>
            </w:document>"#;
        let docx = zip_with(&[("word/document.xml", document)]);
        let result = DefaultAttachmentExtractor
            .extract(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "docx",
                &docx,
            )
            .expect("docx should extract");
        assert!(result.text.contains("Hello World"), "got: {}", result.text);
        assert!(result.text.contains("Second"), "got: {}", result.text);
    }

    #[test]
    fn skips_legacy_binary_office() {
        assert!(DefaultAttachmentExtractor
            .extract("application/msword", "doc", b"\xd0\xcf\x11\xe0anything")
            .is_none());
        assert!(DefaultAttachmentExtractor
            .extract("application/vnd.ms-powerpoint", "ppt", b"\xd0\xcf\x11\xe0anything")
            .is_none());
    }

    #[test]
    fn skips_unknown_and_binary_types() {
        assert!(DefaultAttachmentExtractor
            .extract("image/png", "png", b"\x89PNG\r\n\x1a\n")
            .is_none());
        // Empty input never yields text.
        assert!(DefaultAttachmentExtractor.extract("text/plain", "txt", b"").is_none());
    }
}
