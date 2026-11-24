//! JavaScript and TypeScript config files, read statically with oxc and
//! never executed. The config object is found where the file exports it
//! (`export default`, `module.exports`, `defineConfig(...)`, a `const` it
//! names, an arrow function returning it). Only literal values are edited,
//! by replacing their exact span; anything computed stays as written.

use locdev_core::config::OptionState;
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionBody, Expression, ObjectExpression, ObjectProperty, ObjectPropertyKind, Program,
    PropertyKind, Statement, UnaryOperator,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::Value;

/// Code shown for computed values is cut at this length.
const SHOWN: usize = 160;

fn source_type(name: &str) -> SourceType {
    SourceType::from_path(name).unwrap_or_else(|_| SourceType::mjs())
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, col)
}

/// The first syntax error, with its line and column.
pub fn check(text: &str, name: &str) -> Option<String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, text, source_type(name)).parse();
    let error = parsed.diagnostics.errors().next()?;
    let at = error.labels.iter().next().map(|l| l.offset());
    Some(match at {
        Some(offset) => {
            let (line, col) = line_col(text, offset as usize);
            format!("{} (line {line}, column {col})", error.message)
        }
        None => error.message.to_string(),
    })
}

fn top_level_init<'a>(program: &'a Program<'a>, name: &str) -> Option<&'a Expression<'a>> {
    for statement in program.body.iter().rev() {
        let declaration = match statement {
            Statement::VariableDeclaration(d) => d,
            Statement::ExportDeclaration(e) => match &e.declaration {
                oxc_ast::ast::Declaration::VariableDeclaration(d) => d,
                _ => continue,
            },
            _ => continue,
        };
        for declarator in &declaration.declarations {
            if declarator
                .id
                .get_identifier_name()
                .is_some_and(|n| n.as_str() == name)
            {
                return declarator.init.as_ref();
            }
        }
    }
    None
}

/// The object a config expression stands for, following calls, arrow
/// functions and top-level constants.
fn resolve<'a>(
    program: &'a Program<'a>,
    expr: &'a Expression<'a>,
    depth: u8,
) -> Option<&'a ObjectExpression<'a>> {
    if depth > 8 {
        return None;
    }
    match expr.get_inner_expression() {
        Expression::ObjectExpression(object) => Some(object),
        // defineConfig({...}), defineNuxtConfig({...}), withX(config)…
        Expression::CallExpression(call) => {
            let first = call.arguments.first()?.as_expression()?;
            resolve(program, first, depth + 1)
        }
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => {
                body.statements.iter().rev().find_map(|s| match s {
                    Statement::ReturnStatement(r) => r
                        .argument
                        .as_ref()
                        .and_then(|a| resolve(program, a, depth + 1)),
                    _ => None,
                })
            }
            body => resolve(program, body.as_expression()?, depth + 1),
        },
        Expression::Identifier(id) => resolve(
            program,
            top_level_init(program, id.name.as_str())?,
            depth + 1,
        ),
        _ => None,
    }
}

fn exported<'a>(program: &'a Program<'a>) -> Option<&'a Expression<'a>> {
    program.body.iter().find_map(|statement| match statement {
        Statement::ExportDefaultDeclaration(d) => d.declaration.as_expression(),
        Statement::ExpressionStatement(s) => match &s.expression {
            Expression::AssignmentExpression(a)
                if a.left
                    .as_member_expression()
                    .is_some_and(|m| m.is_specific_member_access("module", "exports")) =>
            {
                Some(&a.right)
            }
            _ => None,
        },
        _ => None,
    })
}

fn config<'a>(program: &'a Program<'a>) -> Result<&'a ObjectExpression<'a>, String> {
    let expr = exported(program)
        .ok_or("the file doesn't export a config (`export default` or `module.exports`)")?;
    resolve(program, expr, 0).ok_or_else(|| {
        "the exported config isn't an object literal LocDev can read; edit the text instead".into()
    })
}

fn property<'a>(object: &'a ObjectExpression<'a>, name: &str) -> Option<&'a ObjectProperty<'a>> {
    // The last definition wins in JavaScript.
    object.properties.iter().rev().find_map(|p| match p {
        ObjectPropertyKind::ObjectProperty(p)
            if !p.key.is_computed()
                && p.kind == PropertyKind::Init
                && p.key
                    .static_name()
                    .is_some_and(|n| n.as_str() == Some(name)) =>
        {
            Some(&**p)
        }
        _ => None,
    })
}

fn literal(expr: &Expression) -> Option<Value> {
    match expr.get_inner_expression() {
        Expression::BooleanLiteral(b) => Some(Value::Bool(b.value)),
        Expression::NullLiteral(_) => Some(Value::Null),
        Expression::NumericLiteral(n) => number(n.value),
        Expression::StringLiteral(s) => s.value.as_str().map(|s| Value::String(s.into())),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => t
            .quasis[0]
            .value
            .cooked
            .as_ref()?
            .as_str()
            .map(|s| Value::String(s.into())),
        Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => {
            match &u.argument {
                Expression::NumericLiteral(n) => number(-n.value),
                _ => None,
            }
        }
        _ => None,
    }
}

fn number(value: f64) -> Option<Value> {
    if value.fract() == 0.0 && value.abs() < 9e15 {
        Some(Value::from(value as i64))
    } else {
        serde_json::Number::from_f64(value).map(Value::Number)
    }
}

fn code(text: &str, expr: &Expression) -> String {
    let span = expr.span();
    let raw = &text[span.start as usize..span.end as usize];
    let flat: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > SHOWN {
        format!("{}…", flat.chars().take(SHOWN).collect::<String>())
    } else {
        flat
    }
}

/// Facts about a parsed config, read once for every option.
pub struct Read {
    pub values: Vec<(String, OptionState)>,
    pub notes: Vec<String>,
}

/// What the file says for each key, plus read-only notes (the plugins).
pub fn read(text: &str, name: &str, keys: &[&str]) -> Result<Read, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, text, source_type(name)).parse();
    if parsed.diagnostics.has_errors() {
        return Err(check(text, name).unwrap_or_else(|| "syntax error".into()));
    }
    let root = config(&parsed.program)?;
    let mut values = Vec::new();
    for key in keys {
        values.push((key.to_string(), read_key(&parsed.program, text, root, key)));
    }
    let mut notes = Vec::new();
    if let Some(plugins) = property(root, "plugins").map(|p| p.value.get_inner_expression())
        && let Expression::ArrayExpression(array) = plugins
    {
        let names: Vec<String> = array
            .elements
            .iter()
            .filter_map(|e| e.as_expression())
            .map(|e| code(text, e))
            .collect();
        notes.push(format!(
            "plugins: {}",
            if names.is_empty() {
                "none".into()
            } else {
                names.join(", ")
            }
        ));
    }
    if root
        .properties
        .iter()
        .any(|p| matches!(p, ObjectPropertyKind::SpreadProperty(_)))
    {
        notes.push(
            "the config spreads another object (`...`); options it sets aren't shown here".into(),
        );
    }
    Ok(Read { values, notes })
}

fn read_key<'a>(
    program: &'a Program<'a>,
    text: &str,
    root: &'a ObjectExpression<'a>,
    key: &str,
) -> OptionState {
    let parts: Vec<&str> = key.split('.').collect();
    let mut object = root;
    for (i, part) in parts.iter().enumerate() {
        let Some(prop) = property(object, part) else {
            return OptionState::Missing;
        };
        if i + 1 == parts.len() {
            return match literal(&prop.value) {
                Some(value) if !prop.shorthand => OptionState::Set { value },
                _ => OptionState::Code {
                    code: code(text, &prop.value),
                },
            };
        }
        match resolve_object(program, &prop.value) {
            Some(next) => object = next,
            None => {
                return OptionState::Code {
                    code: code(text, &prop.value),
                };
            }
        }
    }
    OptionState::Missing
}

/// A nested value that is itself an object literal (directly, not through a call).
fn resolve_object<'a>(
    _program: &'a Program<'a>,
    expr: &'a Expression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match expr.get_inner_expression() {
        Expression::ObjectExpression(o) => Some(o),
        _ => None,
    }
}

fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The quote the file uses most.
fn quote_of(text: &str) -> char {
    if text.matches('"').count() > text.matches('\'').count() {
        '"'
    } else {
        '\''
    }
}

fn js_string(value: &str, quote: char) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push(quote);
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn js_literal(value: &Value, quote: char) -> Result<String, String> {
    Ok(match value {
        Value::String(s) => js_string(s, quote),
        Value::Bool(_) | Value::Number(_) | Value::Null => value.to_string(),
        _ => return Err(
            "only plain values (text, numbers, true/false) are edited here; edit the text instead"
                .into(),
        ),
    })
}

fn js_key(key: &str, quote: char) -> String {
    if is_identifier(key) {
        key.to_string()
    } else {
        js_string(key, quote)
    }
}

/// Smallest indentation step used in the file (two spaces when unknown).
fn indent_unit(text: &str) -> String {
    let mut best: Option<usize> = None;
    for line in text.lines() {
        if line.starts_with('\t') {
            return "\t".into();
        }
        let spaces = line.len() - line.trim_start_matches(' ').len();
        if spaces > 0 && !line.trim().is_empty() {
            best = Some(best.map_or(spaces, |b| b.min(spaces)));
        }
    }
    " ".repeat(best.unwrap_or(2))
}

fn line_indent(text: &str, pos: usize) -> String {
    let start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    text[start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// Offset just after a trailing comma that follows `from`, skipping blanks
/// and comments, if the next significant character is a comma.
fn comma_after(text: &str, from: usize, until: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < until {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < until && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = text[i + 2..until]
                    .find("*/")
                    .map_or(until, |e| i + 2 + e + 2);
            }
            b',' => return Some(i + 1),
            _ => return None,
        }
    }
    None
}

/// `entry` (built for the indentation it lands at) added as the object's last property.
fn insert(
    text: &str,
    object: &ObjectExpression,
    unit: &str,
    entry: impl Fn(&str) -> String,
) -> String {
    let open = object.span.start as usize;
    let close = object.span.end as usize - 1;
    let mut out = text.to_string();
    let single_line = !text[open..close].contains('\n');
    match object.properties.last() {
        Some(last) => {
            let last_end = last.span().end as usize;
            let indent = line_indent(text, last.span().start as usize);
            match (comma_after(text, last_end, close), single_line) {
                (Some(after), true) => out.insert_str(after, &format!(" {},", entry(&indent))),
                (None, true) => out.insert_str(last_end, &format!(", {}", entry(&indent))),
                (Some(after), false) => {
                    out.insert_str(after, &format!("\n{indent}{},", entry(&indent)))
                }
                (None, false) => {
                    out.insert_str(last_end, &format!(",\n{indent}{}", entry(&indent)))
                }
            }
        }
        None => {
            let outer = line_indent(text, open);
            let inner = format!("{outer}{unit}");
            if text[open + 1..close].trim().is_empty() {
                out.replace_range(
                    open + 1..close,
                    &format!("\n{inner}{},\n{outer}", entry(&inner)),
                );
            } else {
                out.insert_str(open + 1, &format!("\n{inner}{},", entry(&inner)));
            }
        }
    }
    out
}

/// `key: value` text, wrapping the remaining path in objects.
fn nested(parts: &[&str], value: &str, quote: char, unit: &str, indent: &str) -> String {
    match parts {
        [last] => format!("{}: {value}", js_key(last, quote)),
        [first, rest @ ..] => {
            let inner = format!("{indent}{unit}");
            format!(
                "{}: {{\n{inner}{},\n{indent}}}",
                js_key(first, quote),
                nested(rest, value, quote, unit, &inner)
            )
        }
        [] => String::new(),
    }
}

/// The text with `key` set to `value`. Refuses when the value (or an object
/// on the way) is computed in code; the result must parse again.
pub fn set(text: &str, name: &str, key: &str, value: &Value) -> Result<String, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, text, source_type(name)).parse();
    if parsed.diagnostics.has_errors() {
        return Err(check(text, name).unwrap_or_else(|| "syntax error".into()));
    }
    let program = &parsed.program;
    let quote = quote_of(text);
    let unit = indent_unit(text);
    let literal_text = js_literal(value, quote)?;
    let parts: Vec<&str> = key.split('.').collect();
    let mut object = config(program)?;
    let mut out = None;
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        match property(object, part) {
            Some(prop) if last => {
                if prop.shorthand || literal(&prop.value).is_none() {
                    return Err(format!(
                        "`{key}` is computed in code (`{}`); edit the text instead",
                        code(text, &prop.value)
                    ));
                }
                // Replace the literal itself, keeping `as const` and the like around it.
                let span = prop.value.get_inner_expression().span();
                let mut next = text.to_string();
                next.replace_range(span.start as usize..span.end as usize, &literal_text);
                out = Some(next);
                break;
            }
            Some(prop) => match resolve_object(program, &prop.value) {
                Some(next) => object = next,
                None => {
                    return Err(format!(
                        "`{part}` is computed in code (`{}`); edit the text instead",
                        code(text, &prop.value)
                    ));
                }
            },
            None => {
                let rest = &parts[i..];
                out = Some(insert(text, object, &unit, |indent| {
                    nested(rest, &literal_text, quote, &unit, indent)
                }));
                break;
            }
        }
    }
    let out = out.ok_or("empty key")?;
    if let Some(error) = check(&out, name) {
        return Err(format!(
            "the change wouldn't parse ({error}); edit the text instead"
        ));
    }
    Ok(out)
}
