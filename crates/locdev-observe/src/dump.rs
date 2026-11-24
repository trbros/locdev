//! Symfony VarDumper's `server` format: one line per dump,
//! `base64(serialize([$data, $context]))` (`Server\Connection::write`).
//! `$data` is a `Cloner\Data`; this rebuilds its tree the way
//! `Data::dumpItem`/`getStub` walk it (symfony/var-dumper 7.x), without
//! running PHP.

use std::collections::HashSet;

use locdev_core::observe::{DumpEntry, DumpNode, KeyKind, RequestInfo};

use crate::php::{self, Doc, Key, Value};

// Stub::TYPE_*
const TYPE_REF: i64 = 1;
const TYPE_STRING: i64 = 2;
const TYPE_ARRAY: i64 = 3;
const TYPE_OBJECT: i64 = 4;
const TYPE_RESOURCE: i64 = 5;
const TYPE_SCALAR: i64 = 6;
const STRING_BINARY: i64 = 1;

/// Nodes kept per dump; past this, the rest shows as truncated.
const MAX_NODES: usize = 20_000;
const MAX_DEPTH: usize = 64;

/// One decoded dump.
#[derive(Debug, Clone, PartialEq)]
pub struct Dump {
    pub value: DumpNode,
    /// `dump($x, label: …)` / `withContext(['label' => …])`.
    pub label: Option<String>,
    /// Seconds since the epoch, from the sender.
    pub timestamp: Option<f64>,
    /// Where `dump()` was called.
    pub file: Option<String>,
    pub line: Option<u32>,
    pub request: Option<RequestInfo>,
    /// The CLI command, for dumps from artisan/console.
    pub command: Option<String>,
}

/// A view of a Stub (properties left at their default aren't serialized).
struct Stub<'a> {
    kind: i64,
    class: Option<&'a Value>,
    value: Option<&'a Value>,
    cut: i64,
    handle: i64,
    ref_count: i64,
    position: i64,
    attr: Option<&'a Value>,
}

fn stub<'a>(doc: &'a Doc, v: &'a Value) -> Option<Stub<'a>> {
    let v = doc.deref(v);
    let Value::Object { class, .. } = v else {
        return None;
    };
    if !class.starts_with("Symfony\\Component\\VarDumper\\") {
        return None;
    }
    let int = |name: &str, default: i64| {
        v.get(name)
            .map(|x| doc.deref(x))
            .and_then(Value::int)
            .unwrap_or(default)
    };
    Some(Stub {
        kind: int("type", TYPE_REF),
        class: v.get("class").map(|x| doc.deref(x)),
        value: v.get("value").map(|x| doc.deref(x)),
        cut: int("cut", 0),
        handle: int("handle", 0),
        ref_count: int("refCount", 0),
        position: int("position", 0),
        attr: v.get("attr").map(|x| doc.deref(x)),
    })
}

/// `getStub()`: a compact array item `[type => position]` (with `[0 => cut]`)
/// as the array stub it stands for.
fn compact(item: &Value) -> Option<(i64, i64, i64)> {
    let Value::Array(entries) = item else {
        return None;
    };
    if entries.is_empty() {
        return None;
    }
    let mut kind = 0;
    let mut position = 0;
    let mut cut = 0;
    for (k, v) in entries {
        if let Key::Int(0) = k {
            cut = v.int().unwrap_or(0);
        }
        kind = match k {
            Key::Int(i) => *i,
            Key::Str(_) => 0,
        };
        position = v.int().unwrap_or(0);
    }
    Some((kind, position, cut))
}

fn text(v: Option<&Value>) -> String {
    v.and_then(Value::text).unwrap_or_default()
}

struct Walker<'a> {
    doc: &'a Doc,
    table: &'a Value,
    nodes: usize,
    /// Handles already expanded (`$refs` / `$firstSeen`).
    seen: HashSet<i64>,
}

impl<'a> Walker<'a> {
    fn children(&self, position: i64) -> Option<&'a Value> {
        if position <= 0 {
            return None;
        }
        self.table.index(position).map(|v| self.doc.deref(v))
    }

    fn entries(&mut self, position: i64, kind: HashKind, depth: usize) -> Vec<DumpEntry> {
        let Some(Value::Array(children)) = self.children(position) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(children.len().min(256));
        for (key, child) in children {
            if self.nodes >= MAX_NODES {
                out.push(DumpEntry {
                    key: "…".into(),
                    kind: KeyKind::Meta,
                    value: DumpNode::Truncated,
                });
                break;
            }
            let (key, key_kind) = decode_key(key, kind);
            let value = self.item(child, depth + 1);
            out.push(DumpEntry {
                key,
                kind: key_kind,
                value,
            });
        }
        out
    }

    fn item(&mut self, raw: &'a Value, depth: usize) -> DumpNode {
        self.nodes += 1;
        if depth > MAX_DEPTH || self.nodes > MAX_NODES {
            return DumpNode::Truncated;
        }
        let raw = self.doc.deref(raw);

        // `$type` for non-stub items: what `gettype()` would say.
        let mut type_name: Option<String> = None;
        let mut current: &'a Value = raw;
        let mut hard_handle: Option<i64> = None;

        if let Some(s) = stub(self.doc, raw)
            && s.kind == TYPE_REF
        {
            if s.handle != 0 {
                hard_handle = Some(s.handle);
            }
            let class = text(s.class);
            type_name = Some(if class.is_empty() {
                gettype(s.value.unwrap_or(&Value::Null)).into()
            } else {
                class
            });
            current = s.value.unwrap_or(&Value::Null);
        }

        if let Some(s) = stub(self.doc, current) {
            return self.stub_node(&s, depth);
        }
        if let Some((kind, position, cut)) = compact(current) {
            // A plain array in compact form: TYPE_ARRAY, class = assoc/indexed.
            let count = cut
                + match self.children(position) {
                    Some(Value::Array(c)) => c.len() as i64,
                    _ => 0,
                };
            let items = self.entries(position, HashKind::Array, depth);
            let _ = kind;
            return DumpNode::Array { count, items, cut };
        }
        let _ = hard_handle;
        match type_name.as_deref() {
            // A meta stub (e.g. a date's text) shows its class as the value.
            Some(t)
                if !matches!(
                    t,
                    "NULL" | "boolean" | "integer" | "double" | "string" | "array"
                ) =>
            {
                let note = current.text();
                DumpNode::Text {
                    v: match note {
                        Some(n) if !n.is_empty() => format!("{t}\n{n}"),
                        _ => t.to_string(),
                    },
                }
            }
            _ => scalar(current),
        }
    }

    fn stub_node(&mut self, s: &Stub<'a>, depth: usize) -> DumpNode {
        // A handle already expanded is a repeat (or a recursion).
        let first = if s.ref_count > 0 || s.kind == TYPE_OBJECT || s.kind == TYPE_RESOURCE {
            s.handle == 0 || self.seen.insert(s.handle)
        } else {
            true
        };
        match s.kind {
            TYPE_STRING => DumpNode::Str {
                v: text(s.value),
                cut: s.cut,
                binary: s.class.and_then(Value::int) == Some(STRING_BINARY),
            },
            TYPE_ARRAY => {
                let count = s.value.and_then(Value::int).unwrap_or(0);
                let items = if first {
                    self.entries(s.position, HashKind::Array, depth)
                } else {
                    Vec::new()
                };
                DumpNode::Array {
                    count,
                    items,
                    cut: s.cut,
                }
            }
            TYPE_OBJECT => {
                let class = text(s.class);
                if !first {
                    return DumpNode::Seen {
                        handle: (s.handle != 0).then_some(s.handle),
                        class: Some(class),
                    };
                }
                let items = self.entries(s.position, HashKind::Object, depth);
                DumpNode::Object {
                    class,
                    handle: (s.handle != 0).then_some(s.handle),
                    items,
                    cut: s.cut,
                }
            }
            TYPE_RESOURCE => DumpNode::Resource {
                kind: text(s.class),
                items: if first {
                    self.entries(s.position, HashKind::Resource, depth)
                } else {
                    Vec::new()
                },
            },
            TYPE_SCALAR => {
                let v = s
                    .attr
                    .and_then(|a| a.get("value"))
                    .map(|v| self.doc.deref(v));
                match v {
                    Some(v) => scalar(v),
                    None => DumpNode::Null,
                }
            }
            _ => DumpNode::Text { v: text(s.class) },
        }
    }
}

#[derive(Clone, Copy)]
enum HashKind {
    Array,
    Object,
    Resource,
}

fn gettype(v: &Value) -> &'static str {
    match v {
        Value::Null => "NULL",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "integer",
        Value::Float(_) => "double",
        Value::Str(_) => "string",
        Value::Array(_) => "array",
        Value::Object { .. } | Value::Enum(_) => "object",
        Value::Ref(_) => "NULL",
    }
}

fn scalar(v: &Value) -> DumpNode {
    match v {
        Value::Null | Value::Ref(_) => DumpNode::Null,
        Value::Bool(b) => DumpNode::Bool { v: *b },
        Value::Int(i) => DumpNode::Int { v: *i },
        Value::Float(f) => DumpNode::Float { v: *f },
        Value::Str(s) => DumpNode::Str {
            v: String::from_utf8_lossy(s).into_owned(),
            cut: 0,
            binary: std::str::from_utf8(s).is_err(),
        },
        Value::Array(e) if e.is_empty() => DumpNode::Array {
            count: 0,
            items: Vec::new(),
            cut: 0,
        },
        Value::Enum(name) => DumpNode::Text { v: name.clone() },
        other => DumpNode::Text {
            v: gettype(other).into(),
        },
    }
}

/// Object keys carry visibility like PHP's mangled names (`CliDumper::dumpKey`).
fn decode_key(key: &Key, hash: HashKind) -> (String, KeyKind) {
    let raw = match key {
        Key::Int(i) => return (i.to_string(), KeyKind::Key),
        Key::Str(s) => s.as_slice(),
    };
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    match hash {
        HashKind::Array => (text(raw), KeyKind::Key),
        HashKind::Object | HashKind::Resource => {
            let raw: Vec<u8> = if matches!(hash, HashKind::Resource) && raw.first() != Some(&0) {
                [b"\0~\0".as_slice(), raw].concat()
            } else {
                raw.to_vec()
            };
            if raw.first() != Some(&0) {
                return (text(&raw), KeyKind::Public);
            }
            let rest = &raw[1..];
            let Some(split) = rest.iter().position(|b| *b == 0) else {
                return (text(rest), KeyKind::Private);
            };
            let (scope, name) = (&rest[..split], &rest[split + 1..]);
            let kind = match scope.first() {
                Some(b'+') => KeyKind::Dynamic,
                Some(b'~') => KeyKind::Meta,
                Some(b'*') => KeyKind::Protected,
                _ => KeyKind::Private,
            };
            (text(name), kind)
        }
    }
}

/// Decodes one line from the dump server. Errors say why, never panic.
pub fn decode(line: &str) -> Result<Dump, String> {
    let bytes = base64(line.trim()).ok_or("not base64")?;
    let doc = php::parse_doc(&bytes).map_err(|e| e.to_string())?;
    let root = &doc.root;
    let data = root.index(0).map(|v| doc.deref(v)).ok_or("no data")?;
    let context = root.index(1).map(|v| doc.deref(v));
    let (value, label) = data_tree(&doc, data)?;

    let ctx = |path: &[&str]| -> Option<&Value> {
        let mut v = context?;
        for p in path {
            v = doc.deref(v.get(p)?);
        }
        Some(v)
    };
    let request = ctx(&["request"]).map(|_| RequestInfo {
        method: ctx(&["request", "method"]).and_then(Value::text),
        url: ctx(&["request", "uri"]).and_then(Value::text),
        controller: ctx(&["request", "controller"]).and_then(|c| match c {
            Value::Object { .. } => data_tree(&doc, c).ok().map(|(n, _)| summary(&n)),
            other => other.text(),
        }),
        ..Default::default()
    });
    Ok(Dump {
        value,
        label,
        timestamp: ctx(&["timestamp"]).and_then(|v| match v {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }),
        file: ctx(&["source", "file"]).and_then(Value::text),
        line: ctx(&["source", "line"])
            .and_then(Value::int)
            .map(|l| l as u32),
        request,
        command: ctx(&["cli", "command_line"]).and_then(Value::text),
    })
}

/// A `Cloner\Data` object as a tree, plus its label.
fn data_tree(doc: &Doc, data: &Value) -> Result<(DumpNode, Option<String>), String> {
    let table = data
        .get("data")
        .map(|v| doc.deref(v))
        .ok_or("no data table")?;
    let position = data.get("position").and_then(Value::int).unwrap_or(0);
    let key = data
        .get("key")
        .map(|v| doc.deref(v))
        .cloned()
        .unwrap_or(Value::Int(0));
    let label = data
        .get("context")
        .map(|c| doc.deref(c))
        .and_then(|c| c.get("label"))
        .and_then(Value::text)
        .filter(|l| !l.is_empty());
    let row = table
        .index(position)
        .map(|v| doc.deref(v))
        .ok_or("no root row")?;
    let root = match &key {
        Value::Int(i) => row.index(*i),
        Value::Str(s) => row.get(&String::from_utf8_lossy(s)),
        _ => None,
    }
    .ok_or("no root item")?;
    let mut walker = Walker {
        doc,
        table,
        nodes: 0,
        seen: HashSet::new(),
    };
    Ok((walker.item(root, 0), label))
}

/// One line for lists: `array:3`, `App\Models\User {#12}`, `"text"`, `42`.
pub fn summary(node: &DumpNode) -> String {
    match node {
        DumpNode::Null => "null".into(),
        DumpNode::Bool { v } => v.to_string(),
        DumpNode::Int { v } => v.to_string(),
        DumpNode::Float { v } => v.to_string(),
        DumpNode::Str { v, cut, .. } => {
            let mut s: String = v.chars().take(120).collect();
            if v.chars().count() > 120 || *cut > 0 {
                s.push('…');
            }
            format!("\"{s}\"")
        }
        DumpNode::Text { v } => v.lines().next().unwrap_or_default().to_string(),
        DumpNode::Array { count, .. } => format!("array:{count}"),
        DumpNode::Object { class, handle, .. } => match handle {
            Some(h) => format!("{class} {{#{h}}}"),
            None => class.clone(),
        },
        DumpNode::Resource { kind, .. } => format!("{kind} resource"),
        DumpNode::Seen { class, handle } => format!(
            "{} {{#{}}} (seen above)",
            class.as_deref().unwrap_or("object"),
            handle.unwrap_or(0)
        ),
        DumpNode::Redacted => "[redacted]".into(),
        DumpNode::Truncated => "…".into(),
    }
}

/// Standard base64 (what PHP's `base64_encode` writes).
fn base64(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    }
    let s = s.trim_end_matches('=').as_bytes();
    if s.len() > php::MAX_BYTES * 4 / 3 + 4 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for chunk in s.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
        if chunk.len() == 1 {
            return None;
        }
    }
    Some(out)
}
