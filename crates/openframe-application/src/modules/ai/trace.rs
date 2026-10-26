//! The local AI trace and the AI health card (ADR-0028 decision 8; Windows phase W5).
//!
//! One row per assistant request in the app database (`sys_ai_trace`), numbers and ids only: never
//! the request, the answer, project text or a path. Nothing leaves the computer on its own
//! (ADR-0021); `ai.export_report` writes the rows and the health figures to a file the person
//! chose. A failure to record never fails the request it describes.

use std::path::PathBuf;

use openframe_ai::Usage;
use openframe_domain::{Actor, AppError, AppResult, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::dossier;
use super::turn::TurnRead;
use crate::core::AppCore;

/// What one request did, in numbers.
pub struct TraceRow<'a> {
    pub request_id: &'a str,
    pub project_id: Option<&'a str>,
    pub scope_kind: &'a str,
    pub model_reference: Option<&'a str>,
    pub context_tokens: Option<u32>,
    pub tools_offered: usize,
    pub tools_chosen: Vec<String>,
    pub steps: usize,
    pub sources: usize,
    pub usage: Option<Usage>,
    pub total_ms: u64,
    pub passes: u32,
    pub checks_failed: Vec<String>,
    pub result_kind: &'a str,
    pub error_code: Option<&'a str>,
    pub proposed: bool,
    /// The path the turn took (write, navigate, change or fixed). Stored in the `lane` column, which
    /// predates the conversational path and keeps its name because the trace table has no migration.
    pub path: &'a str,
    /// The fallback the answer stage used (last_resort or abstained), empty when the model's answer
    /// was shown. Stored in the `runner_up_lane` column.
    pub fallback: &'a str,
    /// The person pressed Stop and the request ended early.
    pub cancelled: bool,
    /// What the answer-type stage read from the message, and how long it took.
    pub turn: Option<&'a TurnRead>,
    pub turn_ms: u64,
    /// What the fetch stage gathered (counts only), and how long it took.
    pub dossier: Option<dossier::Summary>,
    pub dossier_ms: u64,
}

pub fn record(core: &AppCore, r: &TraceRow<'_>) {
    let u = r.usage.unwrap_or_default();
    let fill = match r.context_tokens {
        Some(ctx) if ctx > 0 => {
            (u64::from(u.max_prompt_tokens) * 100 / u64::from(ctx)).min(100) as i64
        }
        _ => 0,
    };
    let done = core.with_app_db(|c| {
        c.execute(
            "INSERT INTO sys_ai_trace(id, request_id, project_id, created_at, scope_kind, model_reference, context_tokens,
                                      tools_offered, tools_chosen, steps, sources, model_calls, tokens_in, tokens_out,
                                      context_fill_pct, prompt_ms, generation_ms, total_ms, passes, checks_failed,
                                      result_kind, error_code, proposed, cached_tokens, lane, runner_up_lane, cancelled,
                                      answer_type, answer_group, turn_domains, length_class, turn_how, turn_vote, turn_ms,
                                      dossier_facts, dossier_tokens, dossier_missing, dossier_required, dossier_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27,
                     ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39)",
            params![
                new_id(),
                r.request_id,
                r.project_id,
                now_ms(),
                r.scope_kind,
                r.model_reference,
                r.context_tokens,
                r.tools_offered as i64,
                r.tools_chosen.join(","),
                r.steps as i64,
                r.sources as i64,
                u.calls,
                u.prompt_tokens,
                u.completion_tokens,
                fill,
                u.prompt_ms,
                u.generation_ms,
                r.total_ms.min(i64::MAX as u64) as i64,
                r.passes,
                r.checks_failed.join(","),
                r.result_kind,
                r.error_code,
                r.proposed,
                u.cached_tokens,
                r.path,
                r.fallback,
                r.cancelled,
                r.turn.map_or("", |t| t.answer_type.code()),
                r.turn.map_or("", |t| t.group.code()),
                r.turn.map(TurnRead::domain_codes).unwrap_or_default(),
                r.turn.map_or("", |t| t.length_class.code()),
                r.turn.map_or("", |t| t.how.code()),
                r.turn.map_or(0.0, |t| f64::from(t.vote)),
                r.turn_ms.min(i64::MAX as u64) as i64,
                r.dossier.map_or(0, |d| d.facts as i64),
                r.dossier.map_or(0, |d| d.tokens as i64),
                r.dossier.map_or(0, |d| d.missing as i64),
                r.dossier.map_or(0, |d| d.required as i64),
                r.dossier_ms.min(i64::MAX as u64) as i64,
            ],
        )?;
        Ok(())
    });
    if let Err(e) = done {
        tracing::warn!(code = e.code_str(), "the AI trace row could not be written");
    }
}

/// What the person did with the proposal of a request: `applied`, `rejected` or `undone`.
pub fn set_outcome(core: &AppCore, request_id: &str, outcome: &str) {
    let done = core.with_app_db(|c| {
        c.execute(
            "UPDATE sys_ai_trace SET outcome = ?2, outcome_at = ?3 WHERE request_id = ?1",
            params![request_id, outcome, now_ms()],
        )?;
        Ok(())
    });
    if let Err(e) = done {
        tracing::warn!(
            code = e.code_str(),
            "the AI trace outcome could not be written"
        );
    }
}

/// The request a Change Set belongs to, when the open project has it.
pub fn request_of_change_set(core: &AppCore, change_set_id: &str) -> Option<String> {
    let s = core.project_opt()?;
    s.store
        .read(|c| {
            Ok(c.query_row(
                "SELECT ai_request_id FROM change_set WHERE id = ?1",
                [change_set_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten())
        })
        // Best effort like every trace write, but a database error is logged.
        .unwrap_or_else(|e| {
            tracing::warn!(
                code = e.code_str(),
                "the AI trace could not find the request of a proposal"
            );
            None
        })
}

// ------------------------------------------------------------------ health card

/// The AI health card in Settings, from the most recent rows.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiHealthDto {
    /// Requests counted (the most recent, at most `HEALTH_WINDOW`).
    pub requests: u32,
    pub median_total_ms: Option<u32>,
    pub median_tokens_in: Option<u32>,
    /// Share of all prompt tokens the engine served from stored work, in percent.
    pub cache_hit_pct: Option<u32>,
    pub median_context_fill_pct: Option<u32>,
    pub proposals: u32,
    pub applied: u32,
    pub rejected: u32,
    pub undone: u32,
    /// Applied out of the proposals the person acted on, in percent.
    pub apply_rate_pct: Option<u32>,
    pub average_passes: Option<f64>,
    pub errors: u32,
    pub weakest_tool: Option<WeakTool>,
    /// Where the chat engine runs, in one plain sentence (ADR-0029 decision 6).
    pub runs_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WeakTool {
    pub name: String,
    pub uses: u32,
    /// Requests with this tool that ended in an error or a rejected or undone proposal, in percent.
    pub failure_pct: u32,
}

pub const HEALTH_WINDOW: u32 = 500;
/// A tool needs this many uses before it can be called the weakest.
const WEAK_TOOL_MIN_USES: u32 = 3;

struct Row {
    total_ms: i64,
    tokens_in: i64,
    cached_tokens: i64,
    fill: i64,
    passes: i64,
    tools: String,
    error: bool,
    proposed: bool,
    outcome: Option<String>,
}

fn rows(c: &Connection, limit: u32) -> AppResult<Vec<Row>> {
    let mut st = c.prepare(
        "SELECT total_ms, tokens_in, context_fill_pct, passes, tools_chosen, error_code IS NOT NULL, proposed, outcome,
                cached_tokens
         FROM sys_ai_trace ORDER BY created_at DESC LIMIT ?1",
    )?;
    let it = st.query_map([limit], |r| {
        Ok(Row {
            total_ms: r.get(0)?,
            tokens_in: r.get(1)?,
            fill: r.get(2)?,
            passes: r.get(3)?,
            tools: r.get(4)?,
            error: r.get(5)?,
            proposed: r.get(6)?,
            outcome: r.get(7)?,
            cached_tokens: r.get(8)?,
        })
    })?;
    Ok(it.collect::<Result<Vec<_>, _>>()?)
}

fn median(mut v: Vec<i64>) -> Option<u32> {
    v.sort_unstable();
    (!v.is_empty()).then(|| v[v.len() / 2].clamp(0, u32::MAX as i64) as u32)
}

fn cache_hit_pct(rows: &[Row]) -> Option<u32> {
    let prompt: i64 = rows.iter().map(|r| r.tokens_in).sum();
    let cached: i64 = rows.iter().map(|r| r.cached_tokens).sum();
    (prompt > 0).then(|| (cached * 100 / prompt).clamp(0, 100) as u32)
}

fn health_from(rows: &[Row]) -> AiHealthDto {
    let mut h = AiHealthDto {
        requests: rows.len() as u32,
        median_total_ms: median(rows.iter().map(|r| r.total_ms).collect()),
        median_tokens_in: median(
            rows.iter()
                .map(|r| r.tokens_in)
                .filter(|t| *t > 0)
                .collect(),
        ),
        median_context_fill_pct: median(rows.iter().map(|r| r.fill).filter(|f| *f > 0).collect()),
        cache_hit_pct: cache_hit_pct(rows),
        ..AiHealthDto::default()
    };
    let mut by_tool: std::collections::BTreeMap<&str, (u32, u32)> = Default::default();
    for r in rows {
        h.errors += u32::from(r.error);
        if r.proposed {
            h.proposals += 1;
        }
        match r.outcome.as_deref() {
            Some("applied") => h.applied += 1,
            Some("rejected") => h.rejected += 1,
            Some("undone") => h.undone += 1,
            _ => {}
        }
        let failed = r.error || matches!(r.outcome.as_deref(), Some("rejected" | "undone"));
        for t in r.tools.split(',').filter(|t| !t.is_empty()) {
            let e = by_tool.entry(t).or_default();
            e.0 += 1;
            e.1 += u32::from(failed);
        }
    }
    let decided = h.applied + h.rejected + h.undone;
    h.apply_rate_pct = (decided > 0).then(|| h.applied * 100 / decided);
    h.average_passes = (!rows.is_empty())
        .then(|| rows.iter().map(|r| r.passes as f64).sum::<f64>() / rows.len() as f64);
    h.weakest_tool = by_tool
        .into_iter()
        .filter(|(_, (uses, failed))| *uses >= WEAK_TOOL_MIN_USES && *failed > 0)
        .map(|(name, (uses, failed))| WeakTool {
            name: name.to_string(),
            uses,
            failure_pct: failed * 100 / uses,
        })
        .max_by_key(|w| (w.failure_pct, w.uses));
    h
}

pub fn health(core: &AppCore, _actor: &Actor, _args: AiHealthArgs) -> AppResult<AiHealthDto> {
    let mut h = core.with_app_db(|c| Ok(health_from(&rows(c, HEALTH_WINDOW)?)))?;
    h.runs_on = super::service::service(core)
        .manager()
        .ok()
        .map(|m| m.placement_text());
    Ok(h)
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiHealthArgs {}

// ------------------------------------------------------------------ export

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiReportArgs {
    /// Destination chosen in the Save dialog (`.json` is added when missing).
    pub path: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiReportResult {
    pub file_name: String,
    pub rows: u32,
}

/// Every trace row (numbers and ids only) and the health figures, to a file the person chose.
pub fn export_report(core: &AppCore, _actor: &Actor, a: AiReportArgs) -> AppResult<AiReportResult> {
    let dest = destination(&a.path)?;
    let (rows, health) =
        core.with_app_db(|c| Ok((trace_json(c, None)?, health_from(&rows(c, HEALTH_WINDOW)?))))?;
    let doc = json!({
        "kind": "openframe-ai-report",
        "version": 1,
        "exportedAt": now_ms(),
        "appVersion": core.config.app_version,
        "note": "Numbers and ids only. No request text, answer, project text or path is recorded.",
        "health": health,
        "rows": rows,
    });
    let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| AppError::internal(e.to_string()))?;
    openframe_project_format::atomic_write(&dest, &bytes)?;
    tracing::info!(rows = rows.len(), "AI report exported");
    Ok(AiReportResult {
        file_name: dest
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        rows: rows.len() as u32,
    })
}

/// The trace rows as JSON objects (numbers and ids only), oldest first; one request's when given.
fn trace_json(c: &Connection, request_id: Option<&str>) -> AppResult<Vec<Value>> {
    let mut st = c.prepare(
        "SELECT request_id, project_id, created_at, scope_kind, model_reference, context_tokens, tools_offered,
                tools_chosen, steps, sources, model_calls, tokens_in, tokens_out, cached_tokens, context_fill_pct,
                prompt_ms, generation_ms, total_ms, passes, checks_failed, result_kind, error_code, proposed, outcome,
                lane, runner_up_lane, cancelled,
                answer_type, answer_group, turn_domains, length_class, turn_how, turn_vote, turn_ms,
                dossier_facts, dossier_tokens, dossier_missing, dossier_required, dossier_ms
         FROM sys_ai_trace WHERE (?1 IS NULL OR request_id = ?1) ORDER BY created_at",
    )?;
    let names: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
    let it = st.query_map([request_id], |r| {
        let mut m = serde_json::Map::new();
        for (i, n) in names.iter().enumerate() {
            let v: rusqlite::types::Value = r.get(i)?;
            m.insert(
                n.clone(),
                match v {
                    rusqlite::types::Value::Null => Value::Null,
                    rusqlite::types::Value::Integer(x) => json!(x),
                    rusqlite::types::Value::Real(x) => json!(x),
                    rusqlite::types::Value::Text(x) => json!(x),
                    rusqlite::types::Value::Blob(_) => Value::Null,
                },
            );
        }
        Ok(Value::Object(m))
    })?;
    Ok(it.collect::<Result<Vec<_>, _>>()?)
}

/// The numbers of one request for a saved conversation, without the ids; null when none were kept.
pub(super) fn request_numbers(c: &Connection, request_id: &str) -> AppResult<Value> {
    Ok(trace_json(c, Some(request_id))?
        .into_iter()
        .next()
        .map_or(Value::Null, |mut row| {
            if let Value::Object(m) = &mut row {
                m.remove("request_id");
                m.remove("project_id");
            }
            row
        }))
}

pub(super) fn destination(raw: &str) -> AppResult<PathBuf> {
    let raw = raw.trim();
    let mut p = PathBuf::from(raw);
    if raw.is_empty() || !p.is_absolute() {
        return Err(AppError::invalid_input(
            "Choose where to save the file with the Save dialog.",
        ));
    }
    if !p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        let mut s = p.into_os_string();
        s.push(".json");
        p = PathBuf::from(s);
    }
    openframe_security::check_user_path(&p, openframe_security::NetworkPaths::Allow)?;
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.is_empty() || openframe_security::sanitize_file_name(name) != name {
        return Err(AppError::invalid_input(
            "That file name contains characters Windows doesn't allow.",
        ));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ms: i64, tools: &str, error: bool, proposed: bool, outcome: Option<&str>) -> Row {
        Row {
            total_ms: ms,
            tokens_in: 900,
            cached_tokens: 450,
            fill: 20,
            passes: 1,
            tools: tools.into(),
            error,
            proposed,
            outcome: outcome.map(str::to_string),
        }
    }

    #[test]
    fn the_health_card_reports_medians_apply_rate_and_the_weakest_tool() {
        let rows = vec![
            row(1000, "count_scenes", false, false, None),
            row(3000, "propose_character", false, true, Some("applied")),
            row(5000, "propose_character", false, true, Some("rejected")),
            row(7000, "propose_character", false, true, Some("applied")),
            row(9000, "propose_task", true, false, None),
            row(2000, "propose_task", false, true, Some("undone")),
            row(4000, "propose_task", false, true, Some("applied")),
        ];
        let h = health_from(&rows);
        assert_eq!(h.requests, 7);
        assert_eq!(h.median_total_ms, Some(4000));
        assert_eq!(
            (h.proposals, h.applied, h.rejected, h.undone, h.errors),
            (5, 3, 1, 1, 1)
        );
        assert_eq!(h.apply_rate_pct, Some(60));
        assert_eq!(h.average_passes, Some(1.0));
        assert_eq!(h.cache_hit_pct, Some(50));
        assert_eq!(health_from(&[]).cache_hit_pct, None);
        let w = h.weakest_tool.unwrap();
        assert_eq!(
            (w.name.as_str(), w.uses, w.failure_pct),
            ("propose_task", 3, 66)
        );
        assert!(health_from(&[]).weakest_tool.is_none());
    }

    #[test]
    fn rows_written_before_the_cache_column_read_as_zero() {
        let mut c = openframe_persistence::open_in_memory().unwrap();
        let apply = openframe_persistence::migrate::apply;
        apply(&mut c, &crate::schema::APP_MIGRATIONS[..3]).unwrap();
        c.execute(
            "INSERT INTO sys_ai_trace(id, request_id, created_at, scope_kind, result_kind)
             VALUES ('a', 'r1', 1, 'WholeProject', 'Answer')",
            [],
        )
        .unwrap();
        apply(&mut c, crate::schema::APP_MIGRATIONS).unwrap();
        assert_eq!(rows(&c, 10).unwrap()[0].cached_tokens, 0);
        let numbers = request_numbers(&c, "r1").unwrap();
        assert_eq!(numbers["cached_tokens"], 0);
        assert!(numbers.get("request_id").is_none() && numbers.get("project_id").is_none());
        assert!(request_numbers(&c, "missing").unwrap().is_null());
        assert_eq!(trace_json(&c, None).unwrap().len(), 1);
    }

    #[test]
    fn the_report_goes_only_to_an_absolute_json_path() {
        assert!(destination("relative/report.json").is_err());
        assert!(destination("").is_err());
        let base = std::env::temp_dir();
        let p = destination(&base.join("ai report").to_string_lossy()).unwrap();
        assert_eq!(p.extension().unwrap(), "json");
    }
}
