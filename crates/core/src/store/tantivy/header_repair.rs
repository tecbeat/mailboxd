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

//! Repair of subjects and attachment filenames garbled by a legacy charset.
//!
//! Mail indexed before headers in GBK, Big5 or Shift_JIS were decoded (see
//! [`crate::envelope::header_text`]) has an empty subject or replacement
//! characters in its stored subject and filenames. The headers are still
//! intact in the stored EML, so a rebuild of the search index decodes them
//! again with [`repair_envelope_headers`] and carries the result over to the
//! attachment documents with [`repair_attachment_headers`].

use std::collections::HashMap;

use mail_parser::{MessageParser, MessagePart};
use tantivy::{schema::Value, TantivyDocument};

use crate::{
    envelope::{
        extractor::DETACH_PLACEHOLDER_PREFIX,
        header_text::{attachment_name, message_subject},
        utils::attachment_parts,
    },
    store::tantivy::{
        rebuild::{copy_stored_fields, stored_attachments},
        schema::SchemaTools,
    },
};

const REPLACEMENT: char = '\u{FFFD}';

/// What [`repair_envelope_headers`] changed in an envelope document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeaderRepair {
    /// The new subject, if the stored one was replaced.
    pub subject: Option<String>,
    /// The new filenames, by attachment content hash.
    pub filenames: HashMap<String, String>,
}

/// `doc`, an envelope document, with a garbled subject and garbled
/// attachment filenames decoded again from `eml`, its stored EML, and what
/// changed. A subject is garbled when it is empty or holds a replacement
/// character, a filename when it holds one. Only a non-empty value without
/// replacement characters replaces a stored one. `None` when nothing
/// changed.
pub fn repair_envelope_headers(
    doc: &TantivyDocument,
    eml: &[u8],
) -> Option<(TantivyDocument, HeaderRepair)> {
    let f = SchemaTools::email_fields();
    let subject = doc
        .get_first(f.f_subject)
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let mut attachments = stored_attachments(doc).unwrap_or_default();
    let garbled_subject = subject.is_empty() || subject.contains(REPLACEMENT);
    let garbled_name = |name: &Option<String>| name.as_deref().is_some_and(is_garbled);
    if !garbled_subject && !attachments.iter().any(|att| garbled_name(&att.filename)) {
        return None;
    }

    let message = MessageParser::new().parse(eml)?;
    let mut repair = HeaderRepair::default();
    if garbled_subject {
        let decoded = message_subject(&message);
        if is_clean(&decoded) && decoded != subject {
            repair.subject = Some(decoded);
        }
    }
    for part in attachment_parts(&message) {
        let Some(hash) = placeholder_hash(eml, &part) else {
            continue;
        };
        let Some(att) = attachments
            .iter_mut()
            .find(|att| att.storage_hash() == hash && garbled_name(&att.filename))
        else {
            continue;
        };
        if let Some(name) = attachment_name(&message, &part).filter(|name| is_clean(name)) {
            repair
                .filenames
                .insert(att.content_hash.clone(), name.clone());
            att.filename = Some(name);
        }
    }

    let attachments_json = if repair.filenames.is_empty() {
        None
    } else {
        match serde_json::to_string(&attachments) {
            Ok(json) => Some(json),
            Err(error) => {
                tracing::warn!(%error, "Cannot store repaired attachment filenames");
                repair.filenames.clear();
                None
            }
        }
    };
    if repair.subject.is_none() && attachments_json.is_none() {
        return None;
    }

    let mut skip = Vec::new();
    if repair.subject.is_some() {
        skip.push(f.f_subject);
    }
    if attachments_json.is_some() {
        skip.push(f.f_attachments);
    }
    let mut repaired = copy_stored_fields(doc, &skip);
    if let Some(subject) = &repair.subject {
        repaired.add_text(f.f_subject, subject);
    }
    if let Some(json) = attachments_json {
        repaired.add_text(f.f_attachments, json);
    }
    Some((repaired, repair))
}

/// `doc`, an attachment document of the envelope `repair` was made for,
/// with the repaired subject and filename. `None` when neither applies.
pub fn repair_attachment_headers(
    doc: &TantivyDocument,
    repair: &HeaderRepair,
) -> Option<TantivyDocument> {
    let f = SchemaTools::attachment_fields();
    let filename = doc
        .get_first(f.f_content_hash)
        .and_then(|v| v.as_str())
        .and_then(|hash| repair.filenames.get(hash));
    if repair.subject.is_none() && filename.is_none() {
        return None;
    }

    let mut skip = Vec::new();
    if repair.subject.is_some() {
        skip.push(f.f_subject);
    }
    if filename.is_some() {
        skip.push(f.f_name_exact);
    }
    let mut repaired = copy_stored_fields(doc, &skip);
    if let Some(subject) = &repair.subject {
        repaired.add_text(f.f_subject, subject);
    }
    if let Some(filename) = filename {
        repaired.add_text(f.f_name_exact, filename);
    }
    Some(repaired)
}

fn is_garbled(text: &str) -> bool {
    text.contains(REPLACEMENT)
}

fn is_clean(text: &str) -> bool {
    !text.is_empty() && !is_garbled(text)
}

/// The storage hash in the detach placeholder that replaced the body of
/// `part` in the stored `eml`, or `None` if the body was kept.
fn placeholder_hash<'e>(eml: &'e [u8], part: &MessagePart<'_>) -> Option<&'e str> {
    let start = (part.offset_body() as usize).min(eml.len());
    let end = (part.offset_end() as usize).min(eml.len());
    let body = eml.get(start..end)?;
    let prefix = DETACH_PLACEHOLDER_PREFIX.as_bytes();
    let hash_start = body
        .windows(prefix.len())
        .position(|window| window == prefix)?
        + prefix.len();
    let rest = &body[hash_start..];
    let hash_len = rest.windows(2).position(|window| window == b">>")?;
    std::str::from_utf8(&rest[..hash_len]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{envelope::extractor::strip_attachment_bodies, message::content::AttachmentInfo};
    use encoding_rs::GBK;

    const SUBJECT: &str = "测试邮件主题";
    const FILENAME: &str = "附件名称.rar";

    /// A GBK message with raw 8-bit Subject and filename, and a second
    /// attachment with an ASCII name.
    fn original_eml() -> Vec<u8> {
        let gbk = |text: &str| GBK.encode(text).0.into_owned();
        let mut eml = b"Subject: ".to_vec();
        eml.extend_from_slice(&gbk(SUBJECT));
        eml.extend_from_slice(
            b"\r\nMIME-Version: 1.0\r\n\
              Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n\
              --b\r\nContent-Type: text/plain; charset=gbk\r\n\r\nhi\r\n\
              --b\r\nContent-Type: application/octet-stream\r\n\
              Content-Disposition: attachment; filename=\"",
        );
        eml.extend_from_slice(&gbk(FILENAME));
        eml.extend_from_slice(
            b"\"\r\nContent-Transfer-Encoding: base64\r\n\r\nAAAA\r\n\
              --b\r\nContent-Type: text/plain\r\n\
              Content-Disposition: attachment; filename=\"plain.txt\"\r\n\r\nplain\r\n\
              --b--\r\n",
        );
        eml
    }

    /// The stored EML and the attachment metadata as written before the
    /// headers were decoded: the GBK filename garbled.
    fn stored_message() -> (Vec<u8>, Vec<AttachmentInfo>) {
        let eml = original_eml();
        let message = MessageParser::new().parse(&eml[..]).unwrap();
        let mut detached = strip_attachment_bodies(&eml, &message, None);
        for att in &mut detached.infos {
            if att.filename.as_deref() == Some(FILENAME) {
                att.filename = Some("\u{FFFD}\u{FFFD}\u{FFFD}.rar".into());
            }
        }
        (detached.stripped_eml, detached.infos)
    }

    fn envelope(subject: &str, attachments: &[AttachmentInfo]) -> TantivyDocument {
        let f = SchemaTools::email_fields();
        let mut doc = TantivyDocument::new();
        doc.add_text(f.f_id, "eid-1");
        doc.add_text(f.f_subject, subject);
        doc.add_text(f.f_attachments, serde_json::to_string(attachments).unwrap());
        doc.add_u64(f.f_size, 42);
        doc
    }

    fn subject_of(doc: &TantivyDocument) -> Vec<&str> {
        let f = SchemaTools::email_fields();
        doc.get_all(f.f_subject)
            .filter_map(|v| v.as_str())
            .collect()
    }

    fn filenames(doc: &TantivyDocument) -> Vec<Option<String>> {
        stored_attachments(doc)
            .unwrap()
            .into_iter()
            .map(|att| att.filename)
            .collect()
    }

    fn hash_of(attachments: &[AttachmentInfo], name: &str) -> String {
        attachments
            .iter()
            .find(|att| att.filename.as_deref().is_some_and(|n| n.ends_with(name)))
            .unwrap()
            .content_hash
            .clone()
    }

    #[test]
    fn a_garbled_subject_and_filename_are_decoded_again() {
        let (eml, attachments) = stored_message();
        let (doc, repair) = repair_envelope_headers(&envelope("", &attachments), &eml).unwrap();

        assert_eq!(subject_of(&doc), [SUBJECT]);
        let mut names = filenames(&doc);
        names.sort();
        assert_eq!(
            names,
            [Some("plain.txt".to_string()), Some(FILENAME.to_string())]
        );
        assert_eq!(
            doc.get_first(SchemaTools::email_fields().f_size)
                .and_then(|v| v.as_u64()),
            Some(42)
        );
        assert_eq!(repair.subject.as_deref(), Some(SUBJECT));
        assert_eq!(
            repair.filenames,
            HashMap::from([(hash_of(&attachments, ".rar"), FILENAME.to_string())])
        );
    }

    #[test]
    fn a_legacy_placeholder_with_the_content_hash_is_matched() {
        let (mut eml, mut attachments) = stored_message();
        for att in &mut attachments {
            let raw = att.raw_content_hash.take().unwrap();
            let at = eml
                .windows(raw.len())
                .position(|w| w == raw.as_bytes())
                .unwrap();
            eml.splice(at..at + raw.len(), att.content_hash.bytes());
        }

        let (doc, _) = repair_envelope_headers(&envelope("Kept", &attachments), &eml).unwrap();

        assert_eq!(subject_of(&doc), ["Kept"]);
        assert!(filenames(&doc).contains(&Some(FILENAME.to_string())));
    }

    #[test]
    fn clean_headers_are_left_alone() {
        let (eml, mut attachments) = stored_message();
        for att in &mut attachments {
            att.filename = att
                .filename
                .take()
                .map(|name| name.replace('\u{FFFD}', "x"));
        }
        assert!(repair_envelope_headers(&envelope("Report", &attachments), &eml).is_none());
    }

    #[test]
    fn a_message_without_a_subject_is_left_alone() {
        let eml = b"From: a@example.com\r\n\r\nhi\r\n";
        assert!(repair_envelope_headers(&envelope("", &[]), eml).is_none());
    }

    #[test]
    fn attachment_documents_take_the_repaired_subject_and_name() {
        let f = SchemaTools::attachment_fields();
        let (eml, attachments) = stored_message();
        let (_, repair) = repair_envelope_headers(&envelope("", &attachments), &eml).unwrap();
        let attachment = |hash: &str| {
            let mut doc = TantivyDocument::new();
            doc.add_text(f.f_id, "aid-1");
            doc.add_text(f.f_subject, "");
            doc.add_text(f.f_content_hash, hash);
            doc.add_text(f.f_name_exact, "\u{FFFD}.rar");
            doc
        };
        let texts = |doc: &TantivyDocument, field| -> Vec<String> {
            doc.get_all(field)
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        };

        let doc = repair_attachment_headers(&attachment(&hash_of(&attachments, ".rar")), &repair)
            .unwrap();
        assert_eq!(texts(&doc, f.f_subject), [SUBJECT]);
        assert_eq!(texts(&doc, f.f_name_exact), [FILENAME]);
        assert_eq!(texts(&doc, f.f_id), ["aid-1"]);

        let doc = repair_attachment_headers(&attachment("other"), &repair).unwrap();
        assert_eq!(texts(&doc, f.f_subject), [SUBJECT]);
        assert_eq!(texts(&doc, f.f_name_exact), ["\u{FFFD}.rar"]);

        assert!(
            repair_attachment_headers(&attachment("other"), &HeaderRepair::default()).is_none()
        );
    }
}
