//! Small helpers shared by the proposal builders: typed access to already
//! schema-validated arguments and a Change Set draft builder with previews.

use openframe_domain::{AppError, AppResult};
use serde_json::{Value, json};

use super::super::queries;
use super::super::toolbox::resolve::Found;
use super::super::types::*;
use super::ProposalSpec;

/// Typed reads of tool arguments that already passed the strict schema.
pub struct A<'a>(pub &'a Value);

impl A<'_> {
    /// Trimmed, non-empty text.
    pub fn s(&self, k: &str) -> Option<String> {
        self.0
            .get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    }
    /// Text as given (an empty string means "clear" for optional fields).
    pub fn raw(&self, k: &str) -> Option<String> {
        self.0
            .get(k)
            .and_then(|v| v.as_str())
            .map(|t| t.trim().to_string())
    }
    /// Required non-empty text.
    pub fn req(&self, k: &str, what: &str) -> AppResult<String> {
        self.s(k)
            .ok_or_else(|| queries::ambiguous(format!("What should the {what} be?")))
    }
    pub fn i(&self, k: &str) -> Option<i64> {
        self.0.get(k).and_then(|v| v.as_i64())
    }
    pub fn u(&self, k: &str) -> Option<u32> {
        self.i(k).and_then(|n| u32::try_from(n).ok())
    }
    pub fn b(&self, k: &str) -> Option<bool> {
        self.0.get(k).and_then(|v| v.as_bool())
    }
    pub fn flag(&self, k: &str) -> bool {
        self.b(k).unwrap_or(false)
    }
    /// Non-empty trimmed strings of a list.
    pub fn list(&self, k: &str) -> Vec<String> {
        self.0
            .get(k)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn ints(&self, k: &str) -> Vec<u32> {
        self.0
            .get(k)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_i64())
                    .filter_map(|n| u32::try_from(n).ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A Change Set draft under construction.
pub struct Draft {
    pub d: ChangeSetDraft,
}

impl Draft {
    pub fn new(spec: &ProposalSpec, args: &Value, title: impl Into<String>) -> Draft {
        Draft {
            d: ChangeSetDraft {
                title: title.into(),
                summary: "I prepared this change. Nothing has been changed yet.".into(),
                operations: Vec::new(),
                preview: Vec::new(),
                exclusions: Vec::new(),
                targets: Vec::new(),
                modules: vec![spec.module.to_string()],
                base_rows: Vec::new(),
                source_tool: spec.tool.to_string(),
                source_args: args.clone(),
                sources: Vec::new(),
            },
        }
    }
    pub fn summary(&mut self, s: impl Into<String>) -> &mut Self {
        self.d.summary = s.into();
        self
    }
    /// Add a registry operation (applied only after explicit approval).
    pub fn op(&mut self, op: &str, args: Value, label: impl Into<String>) -> &mut Self {
        self.d.operations.push(OpCall {
            op: op.to_string(),
            args,
            label: label.into(),
        });
        self
    }
    /// A row whose revision guards the proposal against later edits.
    pub fn base(&mut self, table: &str, id: &str) -> &mut Self {
        let key = (table.to_string(), id.to_string());
        if !self.d.base_rows.contains(&key) {
            self.d.base_rows.push(key);
        }
        self
    }
    /// An affected object (also part of the base version).
    pub fn target(&mut self, table: &str, f: &Found) -> &mut Self {
        self.base(table, &f.id);
        if !self.d.targets.iter().any(|t| t.id == f.id) {
            self.d.targets.push(ObjRef {
                table: table.to_string(),
                id: f.id.clone(),
                label: queries::truncate_chars(f.label.trim(), 120),
            });
        }
        self
    }
    pub fn row(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.d.preview.push(PreviewRow::normal(
            label,
            queries::truncate_chars(&value.into(), 400),
        ));
        self
    }
    /// "old → new" preview row.
    pub fn change(&mut self, label: &str, old: &str, new: &str) -> &mut Self {
        let old = if old.trim().is_empty() {
            "(empty)"
        } else {
            old.trim()
        };
        let new = if new.trim().is_empty() {
            "(empty)"
        } else {
            new.trim()
        };
        self.row(
            label,
            format!(
                "{} → {}",
                queries::truncate_chars(old, 160),
                queries::truncate_chars(new, 200)
            ),
        )
    }
    pub fn exclude(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.d.exclusions.push(PreviewRow::excluded(label, value));
        self
    }
    pub fn module(&mut self, m: &str) -> &mut Self {
        if !self.d.modules.iter().any(|x| x == m) {
            self.d.modules.push(m.to_string());
        }
        self
    }
    /// Record a value resolved from context so "Re-check & Review" rebuilds the
    /// same proposal without that context.
    pub fn pin(&mut self, key: &str, v: Value) -> &mut Self {
        if let Some(o) = self.d.source_args.as_object_mut() {
            o.insert(key.to_string(), v);
        }
        self
    }
    pub fn done(self) -> AppResult<ChangeSetDraft> {
        if self.d.operations.is_empty() {
            return Err(queries::ambiguous(
                "There's nothing to change. What would you like me to change?",
            ));
        }
        Ok(self.d)
    }
}

/// Add `key` to the op arguments and an "old → new" row when a new value is given.
/// Returns whether anything was set.
pub fn set_text(
    d: &mut Draft,
    op_args: &mut Value,
    key: &str,
    label: &str,
    new: Option<String>,
    old: &str,
    max: usize,
) -> AppResult<bool> {
    let Some(v) = new else {
        return Ok(false);
    };
    if v.chars().count() > max {
        return Err(AppError::ai(
            "tool_arguments",
            format!("The {} is too long.", label.to_lowercase()),
        ));
    }
    if v.trim() == old.trim() {
        return Ok(false);
    }
    d.change(label, old, &v);
    op_args[key] = json!(v);
    Ok(true)
}

/// Fail with one focused question when an update would change nothing.
pub fn need_change(changed: bool, what: &str) -> AppResult<()> {
    if changed {
        Ok(())
    } else {
        Err(queries::ambiguous(format!(
            "What should I change about {what}? It already has those values."
        )))
    }
}

/// "YYYY-MM-DD" → epoch milliseconds (UTC midnight) for due dates.
pub fn date_ms(date: &str) -> AppResult<i64> {
    let bad = || AppError::ai("tool_arguments", "Dates must look like 2026-10-14.");
    let parts: Vec<&str> = date.trim().split('-').collect();
    if parts.len() != 3 {
        return Err(bad());
    }
    let y: i32 = parts[0].parse().map_err(|_| bad())?;
    let m: u8 = parts[1].parse().map_err(|_| bad())?;
    let d: u8 = parts[2].parse().map_err(|_| bad())?;
    let month = time::Month::try_from(m).map_err(|_| bad())?;
    let day = time::Date::from_calendar_date(y, month, d).map_err(|_| bad())?;
    Ok(day.midnight().assume_utc().unix_timestamp() * 1000)
}

/// Validate "YYYY-MM-DD" and return it normalised.
pub fn date_text(date: &str) -> AppResult<String> {
    date_ms(date)?;
    Ok(date.trim().to_string())
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_parse_strictly() {
        assert_eq!(date_ms("1970-01-02").unwrap(), 86_400_000);
        assert!(date_ms("2026-13-01").is_err());
        assert!(date_ms("tomorrow").is_err());
    }
}
