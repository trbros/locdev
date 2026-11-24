//! A reader for PHP's `serialize()` format. It only reads data: nothing is
//! instantiated and no code runs, so a payload can't do more than be wrong.
//! Limits on size, depth and item count keep a hostile one cheap.

use std::fmt;

/// A deserialized PHP value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// PHP strings are bytes.
    Str(Vec<u8>),
    Array(Vec<(Key, Value)>),
    /// Properties keep PHP's names: private ones as `\0Class\0name`,
    /// protected as `\0*\0name`.
    Object {
        class: String,
        props: Vec<(Key, Value)>,
    },
    /// `r:`/`R:` back-references (not followed).
    Ref(u64),
    /// `E:` enum case (`Class:Case`).
    Enum(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Int(i64),
    Str(Vec<u8>),
}

impl Key {
    pub fn text(&self) -> String {
        match self {
            Key::Int(i) => i.to_string(),
            Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
        }
    }
}

impl Value {
    pub fn text(&self) -> Option<String> {
        match self {
            Value::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
            Value::Int(i) => Some(i.to_string()),
            Value::Float(f) => Some(f.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Str(s) => std::str::from_utf8(s).ok()?.parse().ok(),
            Value::Bool(b) => Some(*b as i64),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// An array entry or object property by name (object names are matched
    /// without their visibility prefix).
    pub fn get(&self, name: &str) -> Option<&Value> {
        let entries = match self {
            Value::Array(entries) => entries,
            Value::Object { props, .. } => props,
            _ => return None,
        };
        entries.iter().find_map(|(k, v)| match k {
            Key::Str(s) => {
                let bare = match s.iter().rposition(|b| *b == 0) {
                    Some(at) => &s[at + 1..],
                    None => &s[..],
                };
                (bare == name.as_bytes()).then_some(v)
            }
            Key::Int(i) => (i.to_string() == name).then_some(v),
        })
    }

    pub fn index(&self, i: i64) -> Option<&Value> {
        match self {
            Value::Array(entries) => entries
                .iter()
                .find_map(|(k, v)| (*k == Key::Int(i)).then_some(v)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unserialize: {}", self.0)
    }
}

/// Larger payloads are refused outright.
pub const MAX_BYTES: usize = 8 << 20;
const MAX_DEPTH: usize = 256;
const MAX_ITEMS: usize = 500_000;

pub fn parse(input: &[u8]) -> Result<Value, Error> {
    Ok(parse_doc(input)?.root)
}

/// A payload and what its `r:N` references point to.
pub struct Doc {
    pub root: Value,
    /// PHP numbers every value except keys and `R:` from 1, in the order
    /// they start; objects and scalars are kept here (arrays aren't, they
    /// can't be `r:` targets in practice and would cost a copy).
    slots: Vec<Option<Value>>,
}

impl Doc {
    /// `v`, or what it refers to when it's a reference.
    pub fn deref<'a>(&'a self, v: &'a Value) -> &'a Value {
        match v {
            Value::Ref(n) => self
                .slots
                .get((*n as usize).wrapping_sub(1))
                .and_then(Option::as_ref)
                .unwrap_or(v),
            _ => v,
        }
    }
}

pub fn parse_doc(input: &[u8]) -> Result<Doc, Error> {
    if input.len() > MAX_BYTES {
        return Err(Error(format!("payload over {} bytes", MAX_BYTES)));
    }
    let mut p = Parser {
        s: input,
        at: 0,
        items: 0,
        slots: Vec::new(),
    };
    let root = p.value(0, true)?;
    Ok(Doc {
        root,
        slots: p.slots,
    })
}

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
    items: usize,
    slots: Vec<Option<Value>>,
}

impl Parser<'_> {
    fn err<T>(&self, what: &str) -> Result<T, Error> {
        Err(Error(format!("{what} at byte {}", self.at)))
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    fn expect(&mut self, b: u8) -> Result<(), Error> {
        if self.peek() == Some(b) {
            self.at += 1;
            Ok(())
        } else {
            self.err(&format!("expected '{}'", b as char))
        }
    }

    /// Bytes up to (not including) `end`, which is consumed.
    fn until(&mut self, end: u8) -> Result<&[u8], Error> {
        let start = self.at;
        let len = self.s[start..]
            .iter()
            .position(|b| *b == end)
            .ok_or_else(|| Error(format!("unterminated value at byte {start}")))?;
        self.at = start + len + 1;
        Ok(&self.s[start..start + len])
    }

    fn number<T: std::str::FromStr>(&mut self, end: u8) -> Result<T, Error> {
        let raw = self.until(end)?;
        std::str::from_utf8(raw)
            .ok()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| Error(format!("bad number before byte {}", self.at)))
    }

    /// `len:"bytes"` (the quotes around exactly `len` bytes).
    fn sized(&mut self) -> Result<Vec<u8>, Error> {
        let len: usize = self.number(b':')?;
        self.expect(b'"')?;
        if self.at + len > self.s.len() {
            return self.err("string runs past the end");
        }
        let bytes = self.s[self.at..self.at + len].to_vec();
        self.at += len;
        self.expect(b'"')?;
        Ok(bytes)
    }

    fn entries(&mut self, depth: usize) -> Result<Vec<(Key, Value)>, Error> {
        let n: usize = self.number(b':')?;
        if n > MAX_ITEMS {
            return self.err("too many items");
        }
        self.expect(b'{')?;
        let mut out = Vec::with_capacity(n.min(1024));
        for _ in 0..n {
            let key = match self.value(depth + 1, false)? {
                Value::Int(i) => Key::Int(i),
                Value::Str(s) => Key::Str(s),
                _ => return self.err("array key isn't an int or a string"),
            };
            let value = self.value(depth + 1, true)?;
            out.push((key, value));
        }
        self.expect(b'}')?;
        Ok(out)
    }

    /// One value; `numbered` is false for keys (PHP doesn't count them).
    fn value(&mut self, depth: usize, numbered: bool) -> Result<Value, Error> {
        let tag = self.peek().ok_or_else(|| Error("unexpected end".into()))?;
        let slot = (numbered && tag != b'R').then(|| {
            self.slots.push(None);
            self.slots.len() - 1
        });
        let value = self.read(depth, tag)?;
        if let Some(slot) = slot
            && !matches!(value, Value::Array(_))
        {
            self.slots[slot] = Some(value.clone());
        }
        Ok(value)
    }

    fn read(&mut self, depth: usize, tag: u8) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return self.err("nested too deep");
        }
        self.items += 1;
        if self.items > MAX_ITEMS {
            return self.err("too many items");
        }
        self.at += 1;
        match tag {
            b'N' => {
                self.expect(b';')?;
                Ok(Value::Null)
            }
            b'b' => {
                self.expect(b':')?;
                let v: u8 = self.number(b';')?;
                Ok(Value::Bool(v != 0))
            }
            b'i' => {
                self.expect(b':')?;
                Ok(Value::Int(self.number(b';')?))
            }
            b'd' => {
                self.expect(b':')?;
                let raw = self.until(b';')?;
                let text = std::str::from_utf8(raw).unwrap_or("NAN");
                let v = match text {
                    "INF" => f64::INFINITY,
                    "-INF" => f64::NEG_INFINITY,
                    "NAN" => f64::NAN,
                    t => t.parse().map_err(|_| Error(format!("bad float {t}")))?,
                };
                Ok(Value::Float(v))
            }
            b's' => {
                self.expect(b':')?;
                let s = self.sized()?;
                self.expect(b';')?;
                Ok(Value::Str(s))
            }
            b'a' => {
                self.expect(b':')?;
                Ok(Value::Array(self.entries(depth)?))
            }
            b'O' => {
                self.expect(b':')?;
                let class = String::from_utf8_lossy(&self.sized()?).into_owned();
                self.expect(b':')?;
                let props = self.entries(depth)?;
                Ok(Value::Object { class, props })
            }
            b'r' | b'R' => {
                self.expect(b':')?;
                Ok(Value::Ref(self.number(b';')?))
            }
            b'E' => {
                self.expect(b':')?;
                let name = String::from_utf8_lossy(&self.sized()?).into_owned();
                self.expect(b';')?;
                Ok(Value::Enum(name))
            }
            // `C:` (Serializable) carries an opaque payload: kept as text.
            b'C' => {
                self.expect(b':')?;
                let class = String::from_utf8_lossy(&self.sized()?).into_owned();
                self.expect(b':')?;
                let len: usize = self.number(b':')?;
                self.expect(b'{')?;
                if self.at + len > self.s.len() {
                    return self.err("payload runs past the end");
                }
                self.at += len;
                self.expect(b'}')?;
                Ok(Value::Object {
                    class,
                    props: Vec::new(),
                })
            }
            other => self.err(&format!("unknown type '{}'", other as char)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_type() {
        let raw = r#"a:6:{i:0;N;i:1;b:1;i:2;i:-7;i:3;d:1.5;s:3:"key";s:7:"h\"llo!";i:4;O:3:"Foo":1:{s:8:" Foo bar";a:0:{}}}"#
            .replace(" Foo ", "\0Foo\0");
        let v = parse(raw.as_bytes()).unwrap();
        assert_eq!(v.index(0), Some(&Value::Null));
        assert_eq!(v.index(1), Some(&Value::Bool(true)));
        assert_eq!(v.index(2), Some(&Value::Int(-7)));
        assert_eq!(v.index(3), Some(&Value::Float(1.5)));
        assert_eq!(
            v.get("key").and_then(Value::text).as_deref(),
            Some("h\\\"llo!")
        );
        let Some(Value::Object { class, .. }) = v.index(4) else {
            panic!()
        };
        assert_eq!(class, "Foo");
        assert!(
            v.index(4).unwrap().get("bar").is_some(),
            "private names match bare"
        );
    }

    #[test]
    fn strings_count_bytes_not_characters() {
        let v = parse("s:6:\"ação\";".as_bytes()).unwrap();
        assert_eq!(v.text().as_deref(), Some("ação"));
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        for bad in [
            &b"s:10:\"short\";"[..],
            b"a:2:{i:0;N;",
            b"x:1;",
            b"i:abc;",
            b"",
            b"a:99999999:{}",
        ] {
            assert!(parse(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
        }
        let deep = "a:1:{i:0;".repeat(400) + "N;" + &"}".repeat(400);
        assert!(parse(deep.as_bytes()).is_err());
    }
}
