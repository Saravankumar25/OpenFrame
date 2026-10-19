//! Strict argument schemas for AI tools (spec §39: `additionalProperties = false`).
//!
//! Every tool's schema is hand-written with these builders. Strings always carry
//! `maxLength`, arrays `maxItems`, integers `minimum`/`maximum`, so the argument
//! shape is bounded before any tool code runs. `validate` enforces the subset of
//! JSON Schema the builders produce; model output that does not match is refused
//! (and never "repaired" into something else).

use openframe_domain::{AppError, AppResult};
use serde_json::{Map, Value, json};

/// Upper bound for the serialized arguments of one tool call.
pub const MAX_ARGS_BYTES: usize = 8 * 1024;
/// Upper bound for nesting of tool arguments.
pub const MAX_ARGS_DEPTH: usize = 6;

/// An object with the given properties; `required` lists mandatory keys.
pub fn obj(props: &[(&str, Value)], required: &[&str]) -> Value {
    let mut p = Map::new();
    for (k, v) in props {
        p.insert((*k).to_string(), v.clone());
    }
    let mut o =
        json!({"type": "object", "properties": Value::Object(p), "additionalProperties": false});
    if !required.is_empty() {
        o["required"] = json!(required);
    }
    o
}

/// No arguments.
pub fn none() -> Value {
    obj(&[], &[])
}

/// A bounded string.
pub fn s(max: usize) -> Value {
    json!({"type": "string", "maxLength": max})
}

/// A bounded string with a short hint for the model.
pub fn sd(max: usize, description: &str) -> Value {
    json!({"type": "string", "maxLength": max, "description": description})
}

/// A bounded integer.
pub fn int(min: i64, max: i64) -> Value {
    json!({"type": "integer", "minimum": min, "maximum": max})
}

pub fn boolean() -> Value {
    json!({"type": "boolean"})
}

/// One of a fixed set of strings.
pub fn en(values: &[&str]) -> Value {
    json!({"type": "string", "enum": values})
}

/// A bounded array.
pub fn arr(items: Value, max: usize) -> Value {
    json!({"type": "array", "items": items, "minItems": 1, "maxItems": max})
}

/// Reference to a project object: its id (from an earlier tool result) or its name.
pub fn reference() -> Value {
    sd(200, "id from an earlier result, or the exact name")
}

/// Screenplay scene number in a draft.
pub fn scene_number() -> Value {
    int(1, 100_000)
}

fn invalid(detail: String) -> AppError {
    AppError::ai(
        "tool_arguments",
        "I couldn't understand the details of that request.",
    )
    .with_detail(detail)
}

fn depth(v: &Value) -> usize {
    match v {
        Value::Array(a) => 1 + a.iter().map(depth).max().unwrap_or(0),
        Value::Object(o) => 1 + o.values().map(depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// Check size and shape limits, then validate `args` against `schema`.
pub fn validate(tool: &str, schema: &Value, args: &Value) -> AppResult<()> {
    let text = args.to_string();
    if text.len() > MAX_ARGS_BYTES {
        return Err(AppError::ai(
            "tool_arguments",
            "That request had more detail than one step can take. Nothing was changed.",
        )
        .with_detail(format!("{tool}: {} bytes", text.len())));
    }
    if depth(args) > MAX_ARGS_DEPTH {
        return Err(invalid(format!("{tool}: arguments nested too deeply")));
    }
    check(schema, args, tool)
}

fn check(schema: &Value, v: &Value, path: &str) -> AppResult<()> {
    if let Some(allowed) = schema.get("enum").and_then(|e| e.as_array()) {
        if !allowed.contains(v) {
            return Err(invalid(format!("{path}: value not allowed")));
        }
        return Ok(());
    }
    match schema.get("type").and_then(|t| t.as_str()) {
        Some("object") => {
            let o = v
                .as_object()
                .ok_or_else(|| invalid(format!("{path}: expected an object")))?;
            let props = schema
                .get("properties")
                .and_then(|p| p.as_object())
                .cloned()
                .unwrap_or_default();
            for (k, val) in o {
                let Some(ps) = props.get(k) else {
                    return Err(invalid(format!("{path}: unknown field `{k}`")));
                };
                if val.is_null() {
                    // Explicit null = "not given" for optional fields.
                    continue;
                }
                check(ps, val, &format!("{path}.{k}"))?;
            }
            for r in schema
                .get("required")
                .and_then(|r| r.as_array())
                .into_iter()
                .flatten()
            {
                let key = r.as_str().unwrap_or("");
                if o.get(key).is_none_or(|x| x.is_null()) {
                    return Err(invalid(format!("{path}: missing `{key}`")));
                }
            }
            Ok(())
        }
        Some("string") => {
            let t = v
                .as_str()
                .ok_or_else(|| invalid(format!("{path}: expected text")))?;
            if let Some(max) = schema.get("maxLength").and_then(|m| m.as_u64())
                && t.chars().count() as u64 > max
            {
                return Err(AppError::ai(
                    "tool_arguments",
                    "That request had more detail than one step can take. Nothing was changed.",
                )
                .with_detail(format!("{path}: longer than {max} characters")));
            }
            Ok(())
        }
        Some("integer") => {
            let n = v
                .as_i64()
                .ok_or_else(|| invalid(format!("{path}: expected a whole number")))?;
            if schema
                .get("minimum")
                .and_then(|m| m.as_i64())
                .is_some_and(|min| n < min)
                || schema
                    .get("maximum")
                    .and_then(|m| m.as_i64())
                    .is_some_and(|max| n > max)
            {
                return Err(invalid(format!("{path}: number out of range")));
            }
            Ok(())
        }
        Some("boolean") => v
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| invalid(format!("{path}: expected true or false"))),
        Some("array") => {
            let a = v
                .as_array()
                .ok_or_else(|| invalid(format!("{path}: expected a list")))?;
            let max = schema.get("maxItems").and_then(|m| m.as_u64()).unwrap_or(0);
            if a.len() as u64 > max {
                return Err(AppError::ai(
                    "tool_arguments",
                    "That request had more detail than one step can take. Nothing was changed.",
                )
                .with_detail(format!("{path}: more than {max} items")));
            }
            if schema
                .get("minItems")
                .and_then(|m| m.as_u64())
                .is_some_and(|min| (a.len() as u64) < min)
            {
                return Err(invalid(format!("{path}: list is empty")));
            }
            let items = schema.get("items").cloned().unwrap_or(Value::Null);
            for (i, x) in a.iter().enumerate() {
                check(&items, x, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        _ => Err(invalid(format!("{path}: schema without a type"))),
    }
}

/// Every string/array/integer in `schema` is bounded and every object is closed
/// (used by the coverage tests; returns the first offending path).
pub fn unbounded_part(schema: &Value, path: &str) -> Option<String> {
    if schema.get("enum").is_some() {
        return None;
    }
    match schema.get("type").and_then(|t| t.as_str()) {
        Some("object") => {
            if schema.get("additionalProperties") != Some(&Value::Bool(false)) {
                return Some(format!("{path}: object is not closed"));
            }
            for (k, p) in schema
                .get("properties")
                .and_then(|p| p.as_object())
                .into_iter()
                .flatten()
            {
                if let Some(bad) = unbounded_part(p, &format!("{path}.{k}")) {
                    return Some(bad);
                }
            }
            None
        }
        Some("string") => schema
            .get("maxLength")
            .is_none()
            .then(|| format!("{path}: string without maxLength")),
        Some("integer") => (schema.get("minimum").is_none() || schema.get("maximum").is_none())
            .then(|| format!("{path}: integer without bounds")),
        Some("array") => {
            if schema.get("maxItems").is_none() {
                return Some(format!("{path}: array without maxItems"));
            }
            unbounded_part(
                schema.get("items").unwrap_or(&Value::Null),
                &format!("{path}[]"),
            )
        }
        Some("boolean") => None,
        _ => Some(format!("{path}: schema without a type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        obj(
            &[
                ("name", s(10)),
                ("n", int(1, 5)),
                ("tags", arr(s(4), 2)),
                ("kind", en(&["a", "b"])),
            ],
            &["name"],
        )
    }

    #[test]
    fn strict_objects_refuse_unknown_fields() {
        assert!(validate("t", &sample(), &json!({"name": "x"})).is_ok());
        let e = validate("t", &sample(), &json!({"name": "x", "sql": "DROP"})).unwrap_err();
        assert_eq!(e.code_str(), "ai.tool_arguments");
        assert!(validate("t", &sample(), &json!({})).is_err(), "required");
    }

    #[test]
    fn bounds_are_enforced() {
        assert!(validate("t", &sample(), &json!({"name": "01234567890"})).is_err());
        assert!(validate("t", &sample(), &json!({"name": "x", "n": 9})).is_err());
        assert!(
            validate(
                "t",
                &sample(),
                &json!({"name": "x", "tags": ["a", "b", "c"]})
            )
            .is_err()
        );
        assert!(validate("t", &sample(), &json!({"name": "x", "kind": "c"})).is_err());
        assert!(validate("t", &sample(), &json!({"name": "x", "n": "2"})).is_err());
        let huge = "x".repeat(MAX_ARGS_BYTES + 1);
        assert!(validate("t", &sample(), &json!({ "name": huge })).is_err());
    }

    #[test]
    fn builders_produce_bounded_schemas() {
        assert_eq!(unbounded_part(&sample(), "t"), None);
        let open = json!({"type": "object", "properties": {}});
        assert!(unbounded_part(&open, "t").is_some());
        let free = obj(&[("x", json!({"type": "string"}))], &[]);
        assert!(unbounded_part(&free, "t").is_some());
    }
}
