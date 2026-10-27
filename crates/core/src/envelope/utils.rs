//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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

use std::borrow::Cow;

use mail_parser::{HeaderForm, HeaderName, Message, MessagePart, PartKind};
use regex::{Captures, Regex};

fn merge_contiguous_encoded_words(input: &str) -> String {
    let block_re =
        Regex::new(r"(?:=\?[^?]+\?[bBqQ]\?[^?]+\?=)(?:\s+(?:=\?[^?]+\?[bBqQ]\?[^?]+\?=))+")
            .unwrap();

    let word_re = Regex::new(r"=\?([^?]+)\?([bBqQ])\?([^?]+)\?=").unwrap();

    block_re
        .replace_all(input, |caps: &Captures| {
            let whole = caps.get(0).unwrap().as_str();

            let mut charset: Option<String> = None;
            let mut encoding: Option<String> = None;
            let mut combined = String::new();
            let mut ok = true;

            for cap in word_re.captures_iter(whole) {
                let cs = &cap[1];
                let enc = cap[2].to_ascii_uppercase();
                let text = &cap[3];

                if let Some(ref c) = charset {
                    if c != cs {
                        ok = false;
                        break;
                    }
                } else {
                    charset = Some(cs.to_string());
                }

                if let Some(ref e) = encoding {
                    if e != &enc {
                        ok = false;
                        break;
                    }
                } else {
                    encoding = Some(enc);
                }

                combined.push_str(text);
            }

            if ok {
                format!(
                    "=?{}?{}?{}?=",
                    charset.unwrap(),
                    encoding.unwrap(),
                    combined
                )
            } else {
                whole.to_string()
            }
        })
        .to_string()
}

pub fn normalize_subject(raw_subject: Option<&str>) -> String {
    let subject = match raw_subject {
        Some(subject) => merge_contiguous_encoded_words(subject),
        None => return String::new(),
    };

    HeaderForm::Text
        .parse(subject.as_bytes())
        .value()
        .as_text()
        .map(String::from)
        .unwrap_or_default()
}

/// Returns the raw value of the first top-level header with the given name:
/// the bytes after the colon up to and including the final line break.
pub fn header_raw<'m>(
    message: &'m Message<'_>,
    name: impl Into<HeaderName<'m>>,
) -> Option<&'m str> {
    message
        .headers()
        .get(name)
        .and_then(|header| std::str::from_utf8(header.raw_value()).ok())
}

/// Returns the attachments of the top-level message: every non-text leaf
/// part, plus every text or HTML part that is not part of the body of its
/// own type.
pub fn attachment_parts<'m>(
    message: &'m Message<'_>,
) -> impl Iterator<Item = MessagePart<'m>> + 'm {
    message.root().parts().filter(|part| match part.kind() {
        PartKind::Text => !part.in_text_body(),
        PartKind::Html => !part.in_html_body(),
        PartKind::Binary | PartKind::InlineBinary | PartKind::Message(_) => true,
        PartKind::Multipart => false,
    })
}

/// Returns the contents of a part: the charset-decoded text of text parts,
/// the transfer-decoded bytes of binary parts, the raw bytes of a nested
/// message and nothing for multipart containers.
pub fn part_contents<'m>(part: &MessagePart<'m>) -> Cow<'m, [u8]> {
    match part.kind() {
        PartKind::Text | PartKind::Html => match part.text().unwrap_or_default() {
            Cow::Borrowed(text) => Cow::Borrowed(text.as_bytes()),
            Cow::Owned(text) => Cow::Owned(text.into_bytes()),
        },
        PartKind::Binary | PartKind::InlineBinary => part.decoded(),
        PartKind::Message(nested) => Cow::Borrowed(nested.raw()),
        PartKind::Multipart => Cow::Borrowed(&[]),
    }
}

#[cfg(test)]
mod tests {
    use crate::envelope::utils::{merge_contiguous_encoded_words, normalize_subject};
    use mail_parser::MessageParser;

    // ── merge_contiguous_encoded_words ──────────────────────────────

    #[test]
    fn merge_basic_utf8_b() {
        let s = "Hello =?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?= !!!";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "Hello =?UTF-8?B?SGVsbG8=V29ybGQ=?= !!!"
        );
    }

    #[test]
    fn merge_three_blocks() {
        let s = "=?UTF-8?B?QQ==?= =?UTF-8?B?Qg==?= =?UTF-8?B?Qw==?=";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "=?UTF-8?B?QQ==Qg==Qw==?="
        );
    }

    #[test]
    fn merge_noncontiguous_blocks() {
        let s = "=?UTF-8?B?QQ==?= =?UTF-8?B?Qg==?= test =?UTF-8?B?Qw==?= =?UTF-8?B?RA==?=";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "=?UTF-8?B?QQ==Qg==?= test =?UTF-8?B?Qw==RA==?="
        );
    }

    #[test]
    fn reject_different_charsets() {
        let s = "=?UTF-8?B?QQ==?= =?GBK?B?Qg==?=";
        assert_eq!(merge_contiguous_encoded_words(s), s);
    }

    #[test]
    fn reject_different_encodings() {
        let s = "=?UTF-8?B?QQ==?= =?UTF-8?Q?Qg?=";
        assert_eq!(merge_contiguous_encoded_words(s), s);
    }

    #[test]
    fn merge_case_insensitive_encoding() {
        let s = "=?UTF-8?b?QQ==?= =?UTF-8?B?Qg==?=";
        assert_eq!(merge_contiguous_encoded_words(s), "=?UTF-8?B?QQ==Qg==?=");
    }

    #[test]
    fn single_encoded_word_unchanged() {
        let s = "Hello =?UTF-8?B?SGVsbG8=?= !!!";
        assert_eq!(merge_contiguous_encoded_words(s), s);
    }

    #[test]
    fn multiple_spaces_between_words() {
        let s = "=?UTF-8?B?QQ==?=    =?UTF-8?B?Qg==?=";
        assert_eq!(merge_contiguous_encoded_words(s), "=?UTF-8?B?QQ==Qg==?=");
    }

    #[test]
    fn plain_subject_line() {
        let s = "Just a normal subject line";
        assert_eq!(merge_contiguous_encoded_words(s), s);
    }

    #[test]
    fn merge_quoted_printable() {
        let s = "=?UTF-8?Q?Hello_?= =?UTF-8?Q?World?=";
        assert_eq!(
            merge_contiguous_encoded_words(s),
            "=?UTF-8?Q?Hello_World?="
        );
    }

    // ── normalize_subject ───────────────────────────────────────────

    #[test]
    fn normalize_subject_none() {
        assert_eq!(normalize_subject(None), "");
    }

    #[test]
    fn cp1253_subject_and_base64_body() {
        let eml = concat!(
            "Subject: =?Cp1253?B?1/Hd+fPnIOvv4+Hx6eHz7O/9?=\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: text/plain; charset=\"Cp1253\"\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "wePh8Of03SDw5evc9Oc=\r\n",
        );

        let message = MessageParser::default().parse(eml).unwrap();
        assert_eq!(message.subject(), Some("Χρέωση λογαριασμού"));
        assert_eq!(message.body_text(0).as_deref(), Some("Αγαπητέ πελάτη"));
    }
}
