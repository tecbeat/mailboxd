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

//! Header text written in a legacy charset without a correct label.
//!
//! Some mailers (often for Chinese, Japanese or Korean mail) put raw GBK,
//! Big5 or Shift_JIS bytes into the Subject or a filename parameter, or wrap
//! them in an encoded word labelled UTF-8. Read as UTF-8 that text turns into
//! replacement characters. [`message_subject`] and [`attachment_name`]
//! decode such headers again with the charset the message declares for its
//! text, or the one [`chardetng`] detects, and keep mail-parser's result for
//! every header that decoded cleanly.

use std::sync::LazyLock;

use base64::{
    alphabet,
    engine::{general_purpose::GeneralPurpose, DecodePaddingMode, GeneralPurposeConfig},
    Engine,
};
use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::{Encoding, UTF_8};
use mail_parser::{HeaderForm, HeaderName, Message, MessagePart};
use regex::bytes::{Captures, Regex};

use crate::envelope::utils::normalize_subject;

const REPLACEMENT: char = '\u{FFFD}';

static ENCODED_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"=\?([^?\s]+)\?([bBqQ])\?([^?\s]*)\?=").unwrap());

static BASE64_LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

/// The decoded Subject of `message`, or an empty string without one.
pub fn message_subject(message: &Message<'_>) -> String {
    let subject = message.subject().unwrap_or_default();
    if !subject.contains(REPLACEMENT) {
        return subject.to_string();
    }
    let Some(header) = message.headers().get(HeaderName::Subject) else {
        return subject.to_string();
    };
    let raw = header.raw_value();
    let repaired = match repair_header_value(raw, message) {
        Some(value) => normalize_subject(Some(&value)),
        None => match std::str::from_utf8(raw) {
            Ok(value) => normalize_subject(Some(value)),
            Err(_) => return subject.to_string(),
        },
    };
    if repaired.is_empty() {
        subject.to_string()
    } else {
        repaired
    }
}

/// The filename of `part`, a part of `message`: the Content-Disposition
/// `filename`, else the Content-Type `name`.
pub fn attachment_name(message: &Message<'_>, part: &MessagePart<'_>) -> Option<String> {
    let name = part.attachment_name()?;
    if !name.contains(REPLACEMENT) {
        return Some(name.to_string());
    }
    let repaired = [
        (HeaderName::ContentDisposition, "filename"),
        (HeaderName::ContentType, "name"),
    ]
    .into_iter()
    .find_map(|(header_name, attribute)| {
        let header = part.headers().get(header_name)?;
        let value = repair_header_value(header.raw_value(), message)?;
        let parsed = HeaderForm::ContentType.parse(value.as_bytes());
        let content_type = parsed.value().as_content_type()?;
        content_type
            .attribute(attribute)
            .filter(|name| !name.is_empty() && !name.contains(REPLACEMENT))
            .map(String::from)
    });
    Some(repaired.unwrap_or_else(|| name.to_string()))
}

/// Rewrites a raw header value that is not valid UTF-8 text into one that
/// is: raw 8-bit runs are decoded, and encoded words labelled UTF-8 or
/// US-ASCII whose payload is not UTF-8 are relabelled, both with the
/// fallback charset of `message`. `None` when there is nothing to repair.
fn repair_header_value(raw: &[u8], message: &Message<'_>) -> Option<String> {
    let segments = split_header_value(raw);
    let suspect: Vec<u8> = segments
        .iter()
        .filter_map(Segment::suspect_bytes)
        .flatten()
        .copied()
        .collect();
    if suspect.is_empty() {
        return None;
    }
    let encoding = fallback_encoding(message, &suspect);

    let mut repaired = String::with_capacity(raw.len());
    for segment in &segments {
        match segment {
            Segment::Text(bytes) => match std::str::from_utf8(bytes) {
                Ok(text) => repaired.push_str(text),
                Err(_) => repaired.push_str(&encoding.decode_without_bom_handling(bytes).0),
            },
            Segment::Word {
                mislabelled: Some(word),
                ..
            } => {
                repaired.push_str(&format!(
                    "=?{}?{}?{}?=",
                    encoding.name(),
                    word.transfer as char,
                    String::from_utf8_lossy(word.payload),
                ));
            }
            Segment::Word { raw, .. } => repaired.push_str(&String::from_utf8_lossy(raw)),
        }
    }
    Some(repaired)
}

enum Segment<'a> {
    Text(&'a [u8]),
    Word {
        raw: &'a [u8],
        mislabelled: Option<MislabelledWord<'a>>,
    },
}

struct MislabelledWord<'a> {
    transfer: u8,
    payload: &'a [u8],
    decoded: Vec<u8>,
}

impl Segment<'_> {
    fn suspect_bytes(&self) -> Option<&[u8]> {
        match self {
            Segment::Text(bytes) => std::str::from_utf8(bytes).is_err().then_some(*bytes),
            Segment::Word {
                mislabelled: Some(word),
                ..
            } => Some(&word.decoded),
            Segment::Word { .. } => None,
        }
    }
}

fn split_header_value(raw: &[u8]) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut last = 0;
    for captures in ENCODED_WORD.captures_iter(raw) {
        let whole = captures.get(0).expect("match");
        if whole.start() > last {
            segments.push(Segment::Text(&raw[last..whole.start()]));
        }
        segments.push(Segment::Word {
            raw: whole.as_bytes(),
            mislabelled: mislabelled_word(&captures),
        });
        last = whole.end();
    }
    if last < raw.len() {
        segments.push(Segment::Text(&raw[last..]));
    }
    segments
}

/// The encoded word in `captures` if it claims UTF-8 or US-ASCII but its
/// payload is not UTF-8.
fn mislabelled_word<'a>(captures: &Captures<'a>) -> Option<MislabelledWord<'a>> {
    let label = captures.get(1)?.as_bytes();
    let label = label.split(|&b| b == b'*').next().unwrap_or(label);
    if !claims_utf8(label) {
        return None;
    }
    let transfer = captures.get(2)?.as_bytes()[0];
    let payload = captures.get(3)?.as_bytes();
    let decoded = decode_word_payload(transfer, payload)?;
    if std::str::from_utf8(&decoded).is_ok() {
        return None;
    }
    Some(MislabelledWord {
        transfer,
        payload,
        decoded,
    })
}

fn claims_utf8(label: &[u8]) -> bool {
    label.eq_ignore_ascii_case(b"us-ascii")
        || label.eq_ignore_ascii_case(b"ascii")
        || Encoding::for_label(label) == Some(UTF_8)
}

/// The bytes of an RFC 2047 encoded-word payload.
fn decode_word_payload(transfer: u8, payload: &[u8]) -> Option<Vec<u8>> {
    if transfer.eq_ignore_ascii_case(&b'b') {
        BASE64_LENIENT.decode(payload).ok()
    } else {
        let spaced: Vec<u8> = payload
            .iter()
            .map(|&b| if b == b'_' { b' ' } else { b })
            .collect();
        Some(
            mail_parser::Encoding::QuotedPrintable
                .decode(&spaced)
                .into_owned(),
        )
    }
}

/// The charset `suspect` bytes of `message` are read in: the first legacy
/// charset a part declares, else the one detected from the bytes.
fn fallback_encoding(message: &Message<'_>, suspect: &[u8]) -> &'static Encoding {
    declared_legacy_charset(message).unwrap_or_else(|| {
        let mut detector = EncodingDetector::new(Iso2022JpDetection::Allow);
        detector.feed(suspect, true);
        detector.guess(None, Utf8Detection::Deny)
    })
}

fn declared_legacy_charset(message: &Message<'_>) -> Option<&'static Encoding> {
    message.root().parts().find_map(|part| {
        let label = part.content_type()?.attribute("charset")?;
        if claims_utf8(label.as_bytes()) {
            return None;
        }
        Encoding::for_label(label.trim().as_bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use encoding_rs::{BIG5, GBK, SHIFT_JIS};
    use mail_parser::MessageParser;

    const SUBJECT: &str = "测试邮件主题";
    const FILENAME: &str = "附件名称.rar";

    fn encode(encoding: &'static Encoding, text: &str) -> Vec<u8> {
        encoding.encode(text).0.into_owned()
    }

    /// A multipart message whose Subject and attachment name parameters are
    /// `subject` and `filename` as written, with `charset` on the text part.
    fn message_bytes(subject: &[u8], filename: &[u8], charset: &str) -> Vec<u8> {
        let mut eml = b"Subject: ".to_vec();
        eml.extend_from_slice(subject);
        eml.extend_from_slice(
            format!(
                "\r\nMIME-Version: 1.0\r\n\
                 Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n\
                 --b\r\nContent-Type: text/plain; charset={charset}\r\n\r\nhi\r\n\
                 --b\r\nContent-Type: application/octet-stream; name=\""
            )
            .as_bytes(),
        );
        eml.extend_from_slice(filename);
        eml.extend_from_slice(b"\"\r\nContent-Disposition: attachment; filename=\"");
        eml.extend_from_slice(filename);
        eml.extend_from_slice(b"\"\r\n\r\nAAAA\r\n--b--\r\n");
        eml
    }

    fn decoded(eml: &[u8]) -> (String, Option<String>) {
        let message = MessageParser::new().parse(eml).unwrap();
        let part = message.attachments().next().expect("attachment");
        (message_subject(&message), attachment_name(&message, &part))
    }

    fn utf8_word(bytes: &[u8]) -> Vec<u8> {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(bytes)).into_bytes()
    }

    #[test]
    fn raw_gbk_headers_use_the_declared_body_charset() {
        let eml = message_bytes(&encode(GBK, SUBJECT), &encode(GBK, FILENAME), "gbk");
        assert_eq!(
            decoded(&eml),
            (SUBJECT.to_string(), Some(FILENAME.to_string()))
        );
    }

    #[test]
    fn raw_gbk_headers_are_detected_without_a_declared_charset() {
        let subject = "关于下周项目会议安排的通知";
        let filename = "会议纪要和项目计划.rar";
        let eml = message_bytes(&encode(GBK, subject), &encode(GBK, filename), "utf-8");
        assert_eq!(
            decoded(&eml),
            (subject.to_string(), Some(filename.to_string()))
        );
    }

    #[test]
    fn gbk_in_encoded_words_labelled_utf8_is_relabelled() {
        let eml = message_bytes(
            &utf8_word(&encode(GBK, SUBJECT)),
            &utf8_word(&encode(GBK, FILENAME)),
            "gb2312",
        );
        assert_eq!(
            decoded(&eml),
            (SUBJECT.to_string(), Some(FILENAME.to_string()))
        );
    }

    #[test]
    fn mislabelled_q_encoded_word_is_relabelled() {
        let payload: String = encode(GBK, SUBJECT)
            .iter()
            .map(|b| format!("={b:02X}"))
            .collect();
        let subject = format!("Re:_=?utf-8?Q?{payload}?=");
        let eml = message_bytes(subject.as_bytes(), b"plain.txt", "gbk");
        assert_eq!(decoded(&eml).0, format!("Re:_{SUBJECT}"));
    }

    #[test]
    fn other_legacy_charsets_follow_the_declared_charset() {
        for (encoding, label, text) in [
            (BIG5, "big5", "測試郵件主題"),
            (SHIFT_JIS, "shift_jis", "テストメールの件名"),
        ] {
            let eml = message_bytes(&encode(encoding, text), b"plain.txt", label);
            assert_eq!(decoded(&eml).0, text, "{label}");
        }
    }

    #[test]
    fn correctly_labelled_headers_are_left_alone() {
        let subject = format!("=?GBK?B?{}?=", STANDARD.encode(encode(GBK, SUBJECT)));
        let filename = format!("=?gb2312?B?{}?=", STANDARD.encode(encode(GBK, FILENAME)));
        let eml = message_bytes(subject.as_bytes(), filename.as_bytes(), "utf-8");
        assert_eq!(
            decoded(&eml),
            (SUBJECT.to_string(), Some(FILENAME.to_string()))
        );
    }

    #[test]
    fn utf8_subject_and_filename_are_unchanged() {
        let eml = message_bytes("Grüße 你好".as_bytes(), "报告.pdf".as_bytes(), "gbk");
        assert_eq!(
            decoded(&eml),
            ("Grüße 你好".to_string(), Some("报告.pdf".to_string()))
        );
    }

    #[test]
    fn missing_subject_and_name_stay_missing() {
        let eml = b"From: a@example.com\r\n\r\nbody\r\n";
        let message = MessageParser::new().parse(&eml[..]).unwrap();
        assert_eq!(message_subject(&message), "");
        assert_eq!(attachment_name(&message, &message.root().root_part()), None);
    }
}
