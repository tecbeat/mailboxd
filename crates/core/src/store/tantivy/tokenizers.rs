//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

use deunicode::deunicode;
use tantivy::query::{BooleanQuery, BoostQuery, EmptyQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::tokenizer::*;
use tantivy::{Index, Term};

use crate::store::tantivy::filter::DeunicodeFilter;

/// Deterministic, script-aware word tokenizer.
///
/// Text is split into runs of CJK and of other characters:
/// - Other runs go through SimpleTokenizer → RemoveLongFilter(40) →
///   LowerCaser → DeunicodeFilter.
/// - CJK runs have no word spaces. Every character becomes a unigram and every
///   adjacent pair a bigram at the position of its first character, as in
///   Lucene's CJK analyzer. They are not transliterated, so characters that
///   sound alike in pinyin stay apart.
///
/// The query parser runs the same tokenizer, so a CJK query term becomes a
/// phrase over these tokens: an exact substring match.
///
/// No stemming and no language detection: the same word produces the same token
/// whether it arrives as part of a whole message or as a single query term, so
/// the index side and the query side always agree (the root fix for
/// rustmailer/bichon#361).
#[derive(Clone)]
pub struct EuroTokenizer {
    analyzer: TextAnalyzer,
}

impl EuroTokenizer {
    pub fn new() -> Self {
        Self {
            analyzer: TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .filter(DeunicodeFilter)
                .build(),
        }
    }

    fn tokenize(&mut self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut position = 0;
        for (start, run) in script_runs(text) {
            if run.starts_with(is_cjk) {
                position = push_cjk_grams(&mut tokens, start, run, position);
            } else {
                let mut stream = self.analyzer.token_stream(run);
                let mut next_position = position;
                while stream.advance() {
                    let token = stream.token();
                    next_position = position + token.position + 1;
                    tokens.push(Token {
                        offset_from: start + token.offset_from,
                        offset_to: start + token.offset_to,
                        position: position + token.position,
                        text: token.text.clone(),
                        position_length: token.position_length,
                    });
                }
                position = next_position;
            }
        }
        tokens
    }
}

/// Push the unigrams and bigrams of a CJK run and return the next position.
fn push_cjk_grams(tokens: &mut Vec<Token>, start: usize, run: &str, position: usize) -> usize {
    let chars: Vec<(usize, char)> = run.char_indices().collect();
    for (i, &(offset, c)) in chars.iter().enumerate() {
        let unigram_end = offset + c.len_utf8();
        let bigram_end = chars.get(i + 1).map(|&(next, n)| next + n.len_utf8());
        for end in std::iter::once(unigram_end).chain(bigram_end) {
            tokens.push(Token {
                offset_from: start + offset,
                offset_to: start + end,
                position: position + i,
                text: run[offset..end].to_string(),
                position_length: 1,
            });
        }
    }
    position + chars.len()
}

/// Split text into maximal runs that are either all CJK or all other
/// characters, with the byte offset of each run.
fn script_runs(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut rest = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, first) = rest.next()?;
        let cjk = is_cjk(first);
        let mut end = start + first.len_utf8();
        while let Some(&(offset, c)) = rest.peek() {
            if is_cjk(c) != cjk {
                break;
            }
            end = offset + c.len_utf8();
            rest.next();
        }
        Some((start, &text[start..end]))
    })
}

/// Han ideographs, Japanese kana and Korean Hangul. CJK punctuation is not
/// included, so it separates runs like a space.
pub(crate) fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x11FF // Hangul Jamo
            | 0x3005..=0x3007 // ideographic iteration mark, closing mark, number zero
            | 0x3041..=0x3096 // Hiragana
            | 0x309D..=0x309F // Hiragana iteration marks, digraph
            | 0x30A1..=0x30FA // Katakana
            | 0x30FC..=0x30FF // Katakana prolonged sound mark, iteration marks, digraph
            | 0x3131..=0x318E // Hangul Compatibility Jamo
            | 0x31F0..=0x31FF // Katakana Phonetic Extensions
            | 0x3400..=0x4DBF // CJK Unified Ideographs Extension A
            | 0x4E00..=0x9FFF // CJK Unified Ideographs
            | 0xA960..=0xA97F // Hangul Jamo Extended-A
            | 0xAC00..=0xD7A3 // Hangul Syllables
            | 0xD7B0..=0xD7FF // Hangul Jamo Extended-B
            | 0xF900..=0xFAFF // CJK Compatibility Ideographs
            | 0xFF66..=0xFF9F // Halfwidth Katakana
            | 0xFFA0..=0xFFDC // Halfwidth Hangul
            | 0x20000..=0x3FFFF // CJK Unified Ideographs Extensions B and later
    )
}

impl Default for EuroTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

/// The tokens of one text, computed up front.
pub struct EuroTokenStream {
    tokens: Vec<Token>,
    current: Option<usize>,
}

impl TokenStream for EuroTokenStream {
    fn advance(&mut self) -> bool {
        let next = self.current.map_or(0, |i| i + 1);
        self.current = Some(next);
        next < self.tokens.len()
    }

    fn token(&self) -> &Token {
        &self.tokens[self.current.unwrap_or(0)]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.current.unwrap_or(0)]
    }
}

impl Tokenizer for EuroTokenizer {
    type TokenStream<'a> = EuroTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        EuroTokenStream {
            tokens: self.tokenize(text),
            current: None,
        }
    }
}

/// Register the tokenizers the email and attachment schemas refer to.
pub fn register_tokenizers(index: &Index) {
    index.tokenizers().register("euro", EuroTokenizer::new());
    index.tokenizers().register("tri", tri_analyzer());
}

/// Trigram analyzer: NgramTokenizer(3,3) → LowerCaser → DeunicodeFilter.
///
/// The normalization order must match [`trigrams`] exactly (window first, then
/// lowercase, then deunicode), or the query-side grams will not line up with the
/// terms written at index time.
pub fn tri_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(NgramTokenizer::new(3, 3, false).expect("ngram 3,3 is valid"))
        .filter(LowerCaser)
        .filter(DeunicodeFilter)
        .build()
}

/// Compute the 3-grams of a query term exactly as [`tri_analyzer`] would
/// (lowercase + deunicode). Fewer than 3 characters yields an empty vector — the
/// caller falls back to a pure word-level query in that case.
///
/// A term with CJK characters has no grams either: the word level already
/// matches any CJK substring exactly, and grams would only add noise.
pub fn trigrams(term: &str) -> Vec<String> {
    let chars: Vec<char> = term.chars().collect();
    let mut out = Vec::new();
    if chars.len() < 3 || chars.iter().any(|c| is_cjk(*c)) {
        return out;
    }
    for w in chars.windows(3) {
        let g: String = w.iter().collect();
        let g = g.to_lowercase();
        let g = if g.is_ascii() { g } else { deunicode(&g) };
        out.push(g);
    }
    out
}

/// Thresholded trigram subquery on a single field (leave-one-out):
/// `Should[ all grams Must, one-gram-missing Must × n ]` — i.e. at least
/// (n-1)/n grams must be present.
///
/// Tolerates one level of inflection (kroatiens↔kroatien) while rejecting hits
/// driven by a single common gram ("ers" in "Vers"). Dumping every gram into
/// BM25 as `Should` collapses precision@10, so the threshold is mandatory.
fn tri_field_subquery(field: Field, term: &str) -> Box<dyn Query> {
    let grams = trigrams(term);
    if grams.is_empty() {
        return Box::new(EmptyQuery);
    }
    let gram_terms: Vec<Term> = grams
        .iter()
        .map(|g| Term::from_field_text(field, g))
        .collect();

    let must_clauses = |skip: Option<usize>| -> Vec<(Occur, Box<dyn Query>)> {
        gram_terms
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != skip)
            .map(|(_, t)| {
                (
                    Occur::Must,
                    Box::new(TermQuery::new(t.clone(), IndexRecordOption::Basic)) as Box<dyn Query>,
                )
            })
            .collect()
    };

    let mut variants: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    // Variant 0: every gram Must (pure substring semantics).
    variants.push((
        Occur::Should,
        Box::new(BooleanQuery::new(must_clauses(None))),
    ));
    // Variants 1..n: one gram missing (one level of inflection).
    //
    // Only worth generating from 3 grams up. With exactly 2 grams (a 4-character
    // term) a "one gram missing" variant collapses to a single gram, i.e. "either
    // gram matches" — for `haus` that would match `Mauser` on the shared `aus`
    // gram, and `haus` is not even a substring of `mauser`. Requiring both grams
    // is precisely the 4-character substring test, so nothing real is lost.
    if gram_terms.len() >= 3 {
        for skip in 0..gram_terms.len() {
            let clauses = must_clauses(Some(skip));
            if !clauses.is_empty() {
                variants.push((Occur::Should, Box::new(BooleanQuery::new(clauses))));
            }
        }
    }
    Box::new(BooleanQuery::new(variants))
}

/// Fuzzy subquery for a query text over the given trigram fields.
///
/// Terms are AND-ed and fields are OR-ed, like the word level under
/// [`text_parser`]: every term must match in at least one of the fields.
///
/// A term too short for grams (fewer than 3 characters) would make the
/// conjunction false, so then, as with no fields or no terms, this returns
/// `EmptyQuery` and the word level answers the query alone.
pub fn tri_query(tri_fields: &[Field], text: &str) -> Box<dyn Query> {
    let terms: Vec<&str> = text.split_whitespace().collect();
    if tri_fields.is_empty() || terms.is_empty() || terms.iter().any(|t| trigrams(t).is_empty()) {
        return Box::new(EmptyQuery);
    }

    let mut per_term: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    for term in terms {
        let mut per_field: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for field in tri_fields {
            per_field.push((Occur::Should, tri_field_subquery(*field, term)));
        }
        per_term.push((Occur::Must, Box::new(BooleanQuery::new(per_field))));
    }
    Box::new(BooleanQuery::new(per_term))
}

/// Combine: word-level exact (boost 3.0) OR trigram fuzzy (boost 1.0).
///
/// Word-level hits always rank first; trigram only adds inflection/substring
/// recall, and short terms with no grams degrade to a pure word-level query.
pub fn word_or_tri(word: Box<dyn Query>, tri_fields: &[Field], text: &str) -> Box<dyn Query> {
    let tri = tri_query(tri_fields, text);
    Box::new(BooleanQuery::new(vec![
        (Occur::Should, Box::new(BoostQuery::new(word, 3.0))),
        (Occur::Should, Box::new(BoostQuery::new(tri, 1.0))),
    ]))
}

/// A `QueryParser` for free-text filters: space-separated terms are AND-ed,
/// as in email clients, so another word narrows the result instead of
/// widening it. Terms are still OR-ed across the fields, and explicit `OR`,
/// quoted phrases and `-term` keep working.
pub fn text_parser(index: &Index, fields: Vec<Field>) -> QueryParser {
    let mut parser = QueryParser::for_index(index, fields);
    parser.set_conjunction_by_default();
    parser
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<String> {
        let mut tokenizer = EuroTokenizer::new();
        let mut stream = tokenizer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            out.push(stream.token().text.clone());
        }
        out
    }

    /// rustmailer/bichon#361: a whole subject and a single query term must
    /// produce the same token. Under the old tokenizer the whole string was detected as Danish
    /// (stemmed) while the single word was detected as another language, so the
    /// two tokens differed — this assertion necessarily fails on the old code.
    #[test]
    fn deterministic_across_context() {
        let full = tokens("VS: Sejlerferie i Kroatien med skipper");
        let word = tokens("Sejlerferie");
        assert!(
            full.contains(&"sejlerferie".to_string()),
            "full-text tokens: {full:?}"
        );
        assert!(
            word.contains(&"sejlerferie".to_string()),
            "single-word tokens: {word:?}"
        );
    }

    fn positioned(text: &str) -> Vec<(usize, String)> {
        let mut tokenizer = EuroTokenizer::new();
        let mut stream = tokenizer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            let token = stream.token();
            out.push((token.position, token.text.clone()));
        }
        out
    }

    fn at(position: usize, text: &str) -> (usize, String) {
        (position, text.to_string())
    }

    /// A CJK run has no word spaces, so every character is a unigram and
    /// every adjacent pair a bigram at the position of its first character.
    /// The characters stay as they are instead of becoming pinyin.
    #[test]
    fn cjk_run_becomes_unigrams_and_bigrams() {
        assert_eq!(
            positioned("对讲机"),
            vec![
                at(0, "对"),
                at(0, "对讲"),
                at(1, "讲"),
                at(1, "讲机"),
                at(2, "机"),
            ]
        );
    }

    /// Latin and CJK runs each keep their own processing, positions run on
    /// across them, and no bigram spans a script change or CJK punctuation.
    #[test]
    fn mixed_text_keeps_each_script_and_counts_positions_on() {
        assert_eq!(
            positioned("Re: 回复：灯泡 Été&对讲"),
            vec![
                at(0, "re"),
                at(1, "回"),
                at(1, "回复"),
                at(2, "复"),
                at(3, "灯"),
                at(3, "灯泡"),
                at(4, "泡"),
                at(5, "ete"),
                at(6, "对"),
                at(6, "对讲"),
                at(7, "讲"),
            ]
        );
    }

    /// Kana and Hangul are CJK runs as well; a single character is a unigram.
    #[test]
    fn kana_hangul_and_single_characters_are_grams() {
        assert_eq!(
            tokens("メール 한국 字"),
            ["メ", "メー", "ー", "ール", "ル", "한", "한국", "국", "字"]
        );
    }

    /// Byte offsets point at the original text, so highlighting lines up.
    #[test]
    fn offsets_point_into_the_original_text() {
        let text = "Re 灯泡";
        let mut tokenizer = EuroTokenizer::new();
        let mut stream = tokenizer.token_stream(text);
        let mut spans = Vec::new();
        while stream.advance() {
            let token = stream.token();
            spans.push(&text[token.offset_from..token.offset_to]);
        }
        assert_eq!(spans, ["Re", "灯", "灯泡", "泡"]);
    }

    /// Tokens longer than 40 characters are dropped by RemoveLongFilter.
    #[test]
    fn removes_long_tokens() {
        let long = "a".repeat(41);
        assert!(tokens(&long).is_empty());
    }

    /// Accented characters are normalized by DeunicodeFilter.
    #[test]
    fn deunicode_accents() {
        assert_eq!(tokens("Étoiles"), vec!["etoiles".to_string()]);
    }

    /// Query-side gram computation must match the tri analyzer: window →
    /// lowercase → deunicode for non-ASCII.
    #[test]
    fn trigrams_normalized_like_analyzer() {
        assert!(
            trigrams("ab").is_empty(),
            "shorter than 3 chars has no grams"
        );
        assert_eq!(
            trigrams("Kroatiens"),
            vec!["kro", "roa", "oat", "ati", "tie", "ien", "ens"]
        );
        assert_eq!(trigrams("Été"), vec!["ete"]);
    }

    /// CJK in the trigram fields stays as written instead of pinyin, and a
    /// CJK query term has no grams.
    #[test]
    fn trigram_layer_does_not_transliterate_cjk() {
        let mut analyzer = tri_analyzer();
        let mut stream = analyzer.token_stream("对讲机é");
        let mut grams = Vec::new();
        while stream.advance() {
            grams.push(stream.token().text.clone());
        }
        assert_eq!(grams, ["对讲机", "讲机é"]);
        assert!(trigrams("对讲机").is_empty());
        assert!(trigrams("Re对讲").is_empty());
    }

    #[test]
    fn default_tokenizer_is_the_euro_tokenizer() {
        let mut tokenizer = EuroTokenizer::default();
        let mut stream = tokenizer.token_stream("Étoiles Filantes");
        let mut out = Vec::new();
        while stream.advance() {
            out.push(stream.token().text.clone());
        }
        assert_eq!(out, tokens("Étoiles Filantes"));
        assert_eq!(out, ["etoiles", "filantes"]);
    }

    /// A term too short for grams matches nothing on its own; `tri_query`
    /// steps aside before that, but the field subquery must not match all.
    #[test]
    fn field_subquery_of_a_term_without_grams_matches_nothing() {
        use crate::store::tantivy::schema::SchemaTools;
        use tantivy::{collector::Count, TantivyDocument};

        let index = Index::create_in_ram(SchemaTools::email_schema());
        register_tokenizers(&index);
        let f = SchemaTools::email_fields();
        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
        let mut doc = TantivyDocument::new();
        doc.add_text(f.f_subject_tri, "ab abc");
        writer.add_document(doc).unwrap();
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();

        let count = |term: &str| {
            searcher
                .search(tri_field_subquery(f.f_subject_tri, term).as_ref(), &Count)
                .unwrap()
        };
        assert_eq!(count("ab"), 0);
        assert_eq!(count("abc"), 1);
    }
}
