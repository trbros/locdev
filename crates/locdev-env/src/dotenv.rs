//! A `.env` reader that keeps the file exactly as it is. LocDev only ever
//! appends to these files, so the parser only has to find which keys are
//! defined (and where); the text itself is never re-serialized.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
    /// 1-based line where the entry starts.
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// The text after the BOM, untouched.
    pub text: String,
    pub bom: bool,
    /// Lines end in `\r\n`; appended lines follow suit.
    pub crlf: bool,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    NotUtf8,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotUtf8 => f.write_str("not UTF-8 text; LocDev won't read or edit it"),
        }
    }
}

const BOM: &str = "\u{feff}";

pub fn parse(bytes: &[u8]) -> Result<Document, ParseError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ParseError::NotUtf8)?;
    let (bom, text) = match text.strip_prefix(BOM) {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let crlf = text.contains("\r\n");
    let lines: Vec<&str> = text.split('\n').collect();

    let mut entries = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let start = i;
        let line = lines[i].trim_end_matches('\r');
        i += 1;
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let trimmed = trimmed
            .strip_prefix("export ")
            .map(str::trim_start)
            .unwrap_or(trimmed);
        let Some((key, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        {
            continue;
        }
        let rest = rest.trim_start();

        let value = match rest.chars().next() {
            Some(quote @ ('"' | '\'' | '`')) => {
                // Quoted values may span lines; read until the closing quote.
                let mut raw = rest[1..].to_string();
                loop {
                    if let Some(end) = closing(&raw, quote) {
                        raw.truncate(end);
                        break;
                    }
                    if i >= lines.len() {
                        break;
                    }
                    raw.push('\n');
                    raw.push_str(lines[i].trim_end_matches('\r'));
                    i += 1;
                }
                if quote == '"' { unescape(&raw) } else { raw }
            }
            _ => strip_comment(rest).trim_end().to_string(),
        };
        entries.push(Entry {
            key: key.to_string(),
            value,
            line: start + 1,
        });
    }

    Ok(Document {
        text: text.to_string(),
        bom,
        crlf,
        entries,
    })
}

/// Index of the closing `quote` in `raw` (escapes count only inside `"`).
fn closing(raw: &str, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in raw.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' && quote == '"' {
            escaped = true;
        } else if c == quote {
            return Some(i);
        }
    }
    None
}

fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// An unquoted value ends at ` #` (a comment after whitespace).
fn strip_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            return &value[..i];
        }
    }
    value
}

impl Document {
    pub fn keys(&self) -> BTreeSet<&str> {
        self.entries.iter().map(|e| e.key.as_str()).collect()
    }

    pub fn has(&self, key: &str) -> bool {
        self.entries.iter().any(|e| e.key == key)
    }

    /// Keys defined more than once (the loaders disagree on which one wins).
    pub fn duplicates(&self) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut dups = BTreeSet::new();
        for e in &self.entries {
            if !seen.insert(e.key.as_str()) {
                dups.insert(e.key.clone());
            }
        }
        dups.into_iter().collect()
    }

    /// The file with `lines` appended, everything before them byte-for-byte
    /// as it was (BOM, comments, order, line endings).
    pub fn appended(&self, lines: &[String]) -> Vec<u8> {
        let eol = if self.crlf { "\r\n" } else { "\n" };
        let mut out = String::with_capacity(self.text.len() + 64 * lines.len());
        if self.bom {
            out.push_str(BOM);
        }
        out.push_str(&self.text);
        if !self.text.is_empty() {
            if !self.text.ends_with('\n') {
                out.push_str(eol);
            }
            // A blank line sets the new block apart.
            out.push_str(eol);
        }
        for line in lines {
            out.push_str(line);
            out.push_str(eol);
        }
        out.into_bytes()
    }
}

/// `value` written so every common loader (phpdotenv, dotenv, @next/env,
/// Vite, python-dotenv) reads back the same string: bare when it is plain,
/// single-quoted (no interpolation) when it can be, double-quoted otherwise.
pub fn format_value(value: &str) -> String {
    let plain = value.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | ':' | '@' | ',' | '+' | '=')
    });
    if plain {
        return value.to_string();
    }
    if !value.contains('\'') && !value.contains('\n') && !value.contains('\r') {
        return format!("'{value}'");
    }
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '$' => out.push_str("\\$"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_keys_and_values() {
        let doc = parse(
            b"# comment\nAPP_NAME=Shop\nexport DB_HOST = 127.0.0.1\nEMPTY=\nURL=http://x.test # trailing\nHASH=a#b\n\n  INDENTED=yes\n",
        )
        .unwrap();
        let pairs: Vec<(&str, &str)> = doc
            .entries
            .iter()
            .map(|e| (e.key.as_str(), e.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("APP_NAME", "Shop"),
                ("DB_HOST", "127.0.0.1"),
                ("EMPTY", ""),
                ("URL", "http://x.test"),
                ("HASH", "a#b"),
                ("INDENTED", "yes"),
            ]
        );
        assert_eq!(doc.entries[1].line, 3);
    }

    #[test]
    fn quoted_and_multiline() {
        let doc =
            parse(b"A=\"line1\\nline2\"\nB='lit $x'\nKEY=\"-----BEGIN\nabc\n-----END\"\nC=after\n")
                .unwrap();
        assert_eq!(doc.entries[0].value, "line1\nline2");
        assert_eq!(doc.entries[1].value, "lit $x");
        assert_eq!(doc.entries[2].value, "-----BEGIN\nabc\n-----END");
        assert_eq!(doc.entries[3].key, "C");
        assert_eq!(doc.entries[3].line, 6);
    }

    #[test]
    fn duplicates() {
        let doc = parse(b"A=1\nB=2\nA=3\n").unwrap();
        assert_eq!(doc.duplicates(), vec!["A".to_string()]);
    }

    #[test]
    fn rejects_non_utf8() {
        assert_eq!(parse(&[0x41, 0x3d, 0xff, 0xfe]), Err(ParseError::NotUtf8));
    }

    #[test]
    fn append_preserves_everything() {
        let original = "\u{feff}# keep me\r\nA=1   # note\r\nB='x'";
        let doc = parse(original.as_bytes()).unwrap();
        assert!(doc.bom && doc.crlf);
        let out = String::from_utf8(doc.appended(&["C=3".into()])).unwrap();
        assert_eq!(out, format!("{original}\r\n\r\nC=3\r\n"));
        assert!(out.starts_with(original));
    }

    #[test]
    fn append_to_empty() {
        let doc = parse(b"").unwrap();
        assert_eq!(doc.appended(&["A=1".into()]), b"A=1\n");
    }

    #[test]
    fn values_round_trip() {
        for value in [
            "plain",
            "",
            "with space",
            "it's",
            "a\"b",
            "$HOME",
            "multi\nline",
            "back\\slash",
            "sk_test_51ABC",
            "postgres://u:p@127.0.0.1:5432/db",
        ] {
            let line = format!("K={}\n", format_value(value));
            let doc = parse(line.as_bytes()).unwrap();
            assert_eq!(doc.entries[0].value, value, "{line}");
        }
    }
}
