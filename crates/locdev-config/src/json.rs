//! JSON and JSONC through a concrete syntax tree: reading a dotted path and
//! changing one value keep every comment, blank line and indentation.

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstNode, CstObject, CstRootNode};
use locdev_core::config::OptionState;
use serde_json::Value;

fn options(strict: bool) -> ParseOptions {
    if strict {
        ParseOptions {
            allow_comments: false,
            allow_loose_object_property_names: false,
            allow_trailing_commas: false,
            allow_missing_commas: false,
            allow_single_quoted_strings: false,
            allow_hexadecimal_numbers: false,
            allow_unary_plus_numbers: false,
            allow_bare_decimal_point_numbers: false,
            allow_extended_string_escapes: false,
            allow_non_finite_numbers: false,
        }
    } else {
        ParseOptions::default()
    }
}

/// The syntax error, if any. `strict` is plain JSON (no comments, no trailing commas).
pub fn check(text: &str, strict: bool) -> Option<String> {
    CstRootNode::parse(text, &options(strict))
        .err()
        .map(|e| e.to_string())
}

fn root_object(text: &str, strict: bool) -> Result<CstObject, String> {
    let root = CstRootNode::parse(text, &options(strict)).map_err(|e| e.to_string())?;
    root.object_value()
        .ok_or_else(|| "the file isn't a JSON object".to_string())
}

fn node_state(node: &CstNode) -> OptionState {
    match node.to_serde_value() {
        Some(value) => OptionState::Set { value },
        None => OptionState::Code {
            code: node.to_string(),
        },
    }
}

/// What the file says at `key` (`compilerOptions.strict`).
pub fn read(text: &str, strict: bool, key: &str) -> Result<OptionState, String> {
    let mut object = root_object(text, strict)?;
    let parts: Vec<&str> = key.split('.').collect();
    for (i, part) in parts.iter().enumerate() {
        let Some(prop) = object.get(part) else {
            return Ok(OptionState::Missing);
        };
        let Some(value) = prop.value() else {
            return Ok(OptionState::Missing);
        };
        if i + 1 == parts.len() {
            return Ok(node_state(&value));
        }
        match prop.object_value() {
            Some(next) => object = next,
            // `"compilerOptions": null` or a string where an object belongs.
            None => {
                return Ok(OptionState::Code {
                    code: value.to_string(),
                });
            }
        }
    }
    Ok(OptionState::Missing)
}

fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(b) => CstInputValue::Bool(*b),
        Value::Number(n) => CstInputValue::Number(n.to_string()),
        Value::String(s) => CstInputValue::String(s.clone()),
        Value::Array(items) => CstInputValue::Array(items.iter().map(input).collect()),
        Value::Object(map) => {
            CstInputValue::Object(map.iter().map(|(k, v)| (k.clone(), input(v))).collect())
        }
    }
}

/// The text with `key` set to `value`; objects on the way are created.
pub fn set(text: &str, strict: bool, key: &str, value: &Value) -> Result<String, String> {
    let root = CstRootNode::parse(text, &options(strict)).map_err(|e| e.to_string())?;
    let mut object = root
        .object_value()
        .ok_or_else(|| "the file isn't a JSON object".to_string())?;
    let parts: Vec<&str> = key.split('.').collect();
    let (last, path) = parts.split_last().ok_or_else(|| "empty key".to_string())?;
    for part in path {
        object = match object.get(part) {
            Some(prop) => prop
                .object_value()
                .ok_or_else(|| format!("`{part}` isn't an object here; edit the text instead"))?,
            None => object
                .append(part, CstInputValue::Object(Vec::new()))
                .object_value_or_set(),
        };
    }
    match object.get(last) {
        Some(prop) => prop.set_value(input(value)),
        None => {
            object.append(last, input(value));
        }
    }
    Ok(root.to_string())
}
