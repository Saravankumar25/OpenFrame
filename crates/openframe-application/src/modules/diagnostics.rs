//! Support diagnostics bundle (build plan item export diagnostics, feature CORE-015, ADR-0021,
//! docs/engineering/26-observability.md §3).
//!
//! | op                           | what                                                        |
//! |------------------------------|-------------------------------------------------------------|
//! | `diagnostics.preview`        | the exact files a bundle would contain, for the user to read |
//! | `diagnostics.export_bundle`  | write that bundle as a ZIP to a location the user picked     |
//!
//! Policy (ADR-0021): nothing is sent anywhere. The user starts the export, can read every
//! file before saving, and decides whether to attach the ZIP to a support request. This
//! module performs no network I/O.
//!
//! Privacy is enforced by construction, not by after-the-fact redaction:
//! - The summary holds only fixed facts (app/OS version, architecture, channel, schema
//!   versions). No project id, title, path, user name or content is read into it.
//! - The log tail is rebuilt field by field from the local JSON log. A message is kept only
//!   when it is one of OpenFrame's own fixed log messages; any other text becomes
//!   "[message omitted]". String fields are kept only for a short allow-list of
//!   identifier-shaped fields (`op` must be a registered operation, `code` an error code);
//!   every other string field is replaced by "[omitted]". Numbers and booleans are kept.
//! - The AI section copies states, versions and sizes only; the runtime folder, the
//!   runtime log path and free-text runtime messages are never copied.
//! - Defence in depth: OpenFrame's own folders (project, app data, Global Idea Vault,
//!   projects folder, user profile) are scrubbed from every file and the result goes through
//!   `openframe_security::redact` (home paths, e-mail addresses, key/token values).
//!
//! The bundle is written atomically (`archive::write_zip`: temporary file + rename) and
//! the destination follows the export rules: an absolute path in an existing folder, a name
//! Windows accepts, never inside the open project, OpenFrame's app folder or the Global
//! Idea Vault. Exporting changes nothing in the project, so there is no undo step; when a
//! project is open, one Activity entry records the export. Diagnostics are not project
//! content and have no search document.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_security::archive::{EntrySource, write_zip};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::core::AppCore;
use crate::events::{AppEvent, StoreKind};
use crate::registry::Registry;
use crate::store::MutationMeta;

/// Why the assistant never reaches these operations (coverage matrix).
pub const HIDDEN_REASON: &str = "Support diagnostics are user-initiated only (ADR-0021): the user reviews the bundle and saves it to a location they pick; the assistant never gathers, exports or sends diagnostics.";

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M};
    r.module("Diagnostics");
    r.query("diagnostics.preview", preview).meta(
        M::query(
            Capability::Export,
            "The files a support diagnostics bundle would contain, shown before saving it.",
        )
        .hidden(HIDDEN_REASON),
    );
    // Not undoable: it writes a file outside the project and changes nothing in it, so
    // there is nothing to undo (the user deletes the file in File Explorer if unwanted).
    r.command("diagnostics.export_bundle", export_bundle).meta(
        M::command(
            Capability::Export,
            "Save a redacted support diagnostics bundle to a location the user picked.",
        )
        .fs(Fs::WritesUserFile)
        .hidden(HIDDEN_REASON),
    );
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticsPreviewArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticsExportArgs {
    /// Destination chosen in the Save dialog (`.zip` is added when missing).
    pub path: String,
}

/// One file of the bundle, exactly as it will be written.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsFile {
    pub name: String,
    /// What the file contains, in plain language.
    pub description: String,
    #[ts(type = "number")]
    pub bytes: u64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsPreview {
    /// "OpenFrame Diagnostics 2026-10-05.zip" (no project name).
    pub suggested_file_name: String,
    pub files: Vec<DiagnosticsFile>,
    /// What a bundle never contains.
    pub never_included: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsExportResult {
    pub path: String,
    pub file_name: String,
    #[ts(type = "number")]
    pub bytes_written: u64,
    pub files: Vec<String>,
    pub summary: String,
}

// ------------------------------------------------------------------ handlers

fn require(actor: &Actor) -> AppResult<()> {
    actor.require(Capability::Export, "export from this project")
}

fn preview(
    core: &AppCore,
    actor: &Actor,
    _: DiagnosticsPreviewArgs,
) -> AppResult<DiagnosticsPreview> {
    require(actor)?;
    let bundle = collect(core);
    Ok(DiagnosticsPreview {
        suggested_file_name: suggested_file_name(&bundle.created_at),
        files: bundle.files,
        never_included: NEVER_INCLUDED.iter().map(|s| s.to_string()).collect(),
    })
}

fn export_bundle(
    core: &AppCore,
    actor: &Actor,
    a: DiagnosticsExportArgs,
) -> AppResult<DiagnosticsExportResult> {
    require(actor)?;
    let dest = destination(core, &a.path)?;
    let bundle = collect(core);
    let entries = bundle
        .files
        .iter()
        .map(|f| (f.name.clone(), EntrySource::Bytes(f.content.as_bytes())));
    // `write_zip` stages through the shared atomic replace: a unique hidden staging name in the
    // destination folder, removed on any failure, never a delete before the rename.
    if let Err(e) = write_zip(&dest, entries) {
        return Err(write_error(e));
    }
    let bytes_written = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let files: Vec<String> = bundle.files.iter().map(|f| f.name.clone()).collect();
    record_activity(core, actor);
    tracing::info!(
        files = files.len(),
        bytes = bytes_written,
        "diagnostics bundle exported"
    );
    let file_name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    Ok(DiagnosticsExportResult {
        path: dest.to_string_lossy().into_owned(),
        summary: format!(
            "Saved {file_name}. Nothing was sent. Review it before you attach it to a support request."
        ),
        file_name,
        bytes_written,
        files,
    })
}

/// Validate the destination chosen in the Save dialog. The path is checked syntactically
/// (no device, alternate-stream or relative paths) before the filesystem is touched.
fn destination(core: &AppCore, raw: &str) -> AppResult<PathBuf> {
    let raw = raw.trim();
    let mut p = PathBuf::from(raw);
    if raw.is_empty() || !p.is_absolute() {
        return Err(AppError::invalid_input(
            "Choose where to save the diagnostics bundle with the Save dialog.",
        ));
    }
    let is_zip = p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    if !is_zip {
        let mut s = p.into_os_string();
        s.push(".zip");
        p = PathBuf::from(s);
    }
    openframe_security::check_user_path(&p, openframe_security::NetworkPaths::Allow)?;
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.is_empty() || openframe_security::sanitize_file_name(name) != name {
        return Err(AppError::invalid_input(
            "That file name contains characters Windows doesn't allow.",
        ));
    }
    let parent = p
        .parent()
        .filter(|x| !x.as_os_str().is_empty())
        .ok_or_else(|| {
            AppError::invalid_input("Choose a folder to save the diagnostics bundle in.")
        })?;
    if !parent.is_dir() {
        return Err(AppError::export(
            "folder_missing",
            "That folder can't be found. If it's on an external drive, reconnect it, or choose another location.",
        ));
    }
    if p.is_dir() {
        return Err(AppError::invalid_input("Choose a file name, not a folder."));
    }
    if let Some(session) = core.project_opt()
        && openframe_security::is_within(session.layout.root(), parent)
    {
        return Err(AppError::export(
            "inside_project",
            "The diagnostics bundle can't be saved inside the project's own folder. Choose another location, such as Documents.",
        ));
    }
    if openframe_security::is_within(&core.config.app_data_dir, parent) {
        return Err(AppError::export(
            "inside_app_folder",
            "The diagnostics bundle can't be saved inside OpenFrame's own app folder. Choose another location, such as Documents.",
        ));
    }
    // Protected folders resolved through links, the Global Idea Vault; remembers the path
    // so "Reveal in File Manager" may show it.
    crate::util::check_output_file(core, &p)?;
    Ok(p)
}

/// Keep specific, actionable storage errors; anything else is a plain "could not be saved".
fn write_error(e: AppError) -> AppError {
    if e.is("storage.disk_full")
        || e.is("storage.access_denied")
        || e.is("storage.drive_unavailable")
    {
        return e;
    }
    AppError::export(
        "write_failed",
        "The diagnostics bundle could not be saved. Nothing was changed.",
    )
    .with_detail(e.code_str().to_string())
}

/// One Activity entry in the open project (not undoable: the project is unchanged). A
/// failure to record it never fails an export that was written.
fn record_activity(core: &AppCore, actor: &Actor) {
    let Some(session) = core.project_opt() else {
        return;
    };
    let origin = match &actor.origin {
        openframe_domain::auth::ActorOrigin::Local => "local",
        openframe_domain::auth::ActorOrigin::Exchange { .. } => "exchange",
        openframe_domain::auth::ActorOrigin::Ai { .. } => "ai",
    };
    const SUMMARY: &str = "Exported a diagnostics bundle for support";
    let res = session.store.mutate(
        actor,
        MutationMeta::new("diagnostics.export_bundle", SUMMARY, Capability::Export)
            .not_undoable()
            .quiet(),
        |tx| {
            tx.conn().execute(
                "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary, origin)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    new_id(),
                    now_ms(),
                    actor.user_id,
                    actor.display_name,
                    "diagnostics.export_bundle",
                    SUMMARY,
                    origin
                ],
            )?;
            Ok(())
        },
    );
    match res {
        Ok(()) => core.emit(AppEvent::DataChanged {
            store: StoreKind::Project,
            tables: vec!["sys_activity".into()],
            ids: Vec::new(),
            origin: "local".into(),
        }),
        Err(e) => tracing::warn!(
            code = e.code_str(),
            "could not record diagnostics export activity"
        ),
    }
}

// ------------------------------------------------------------------ bundle

const NEVER_INCLUDED: [&str; 6] = [
    "Screenplay text, notes, comments and any other project content",
    "Project titles, file names and folder paths",
    "Names, contacts and e-mail addresses",
    "Assistant questions and answers",
    "Passwords, keys and tokens",
    "Nothing is sent automatically. The bundle stays on this computer until you share it.",
];

const FILES: [(&str, &str); 6] = [
    (
        "manifest.json",
        "List of every file in this bundle, what it contains and its SHA-256 checksum.",
    ),
    (
        "summary.json",
        "OpenFrame version, Windows version, architecture, distribution channel and database schema versions.",
    ),
    (
        "errors.json",
        "Error codes from recent logs, with how often and when they occurred.",
    ),
    (
        "timings.json",
        "How long slow or failed operations took, by operation name.",
    ),
    (
        "log-tail.txt",
        "The most recent log entries, rebuilt from fixed messages, operation names, error codes and numbers only.",
    ),
    (
        "ai-status.json",
        "Offline AI status: available or not, bundle check, engine state and component versions. No folders or file paths.",
    ),
];

pub(crate) struct Bundle {
    pub created_at: String,
    pub files: Vec<DiagnosticsFile>,
}

fn collect(core: &AppCore) -> Bundle {
    let created_at = now_rfc3339();
    let scan = scan_logs(core);
    let contents = [
        pretty(&summary_json(core, &created_at, &scan)),
        pretty(&errors_json(&scan)),
        pretty(&timings_json(&scan)),
        log_tail(&scan),
        pretty(&ai_json(core)),
    ];
    let secrets = sensitive_paths(core);
    let mut files: Vec<DiagnosticsFile> = FILES[1..]
        .iter()
        .zip(contents)
        .map(|((name, description), content)| {
            let content = scrub(&content, &secrets);
            DiagnosticsFile {
                name: name.to_string(),
                description: description.to_string(),
                bytes: content.len() as u64,
                content,
            }
        })
        .collect();
    let manifest = json!({
        "format": "openframe-diagnostics",
        "formatVersion": 1,
        "createdAt": created_at,
        "sentAutomatically": false,
        "files": std::iter::once(json!({ "name": FILES[0].0, "contains": FILES[0].1 }))
            .chain(files.iter().map(|f| json!({
                "name": f.name,
                "contains": f.description,
                "bytes": f.bytes,
                "sha256": openframe_security::sha256_bytes(f.content.as_bytes()),
            })))
            .collect::<Vec<_>>(),
        "neverIncluded": NEVER_INCLUDED,
    });
    let manifest = scrub(&pretty(&manifest), &secrets);
    files.insert(
        0,
        DiagnosticsFile {
            name: FILES[0].0.to_string(),
            description: FILES[0].1.to_string(),
            bytes: manifest.len() as u64,
            content: manifest,
        },
    );
    Bundle { created_at, files }
}

fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".into());
    s.push('\n');
    s
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .ok()
        .and_then(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_default()
}

/// "OpenFrame Diagnostics 2026-10-05.zip".
fn suggested_file_name(created_at: &str) -> String {
    let date: String = created_at.chars().take(10).collect();
    if date.len() == 10 {
        format!("OpenFrame Diagnostics {date}.zip")
    } else {
        "OpenFrame Diagnostics.zip".into()
    }
}

// ------------------------------------------------------------------ summary

fn summary_json(core: &AppCore, created_at: &str, scan: &LogScan) -> Value {
    let startup = scan
        .entries
        .iter()
        .rev()
        .find(|e| e.message == "OpenFrame Studio starting");
    let webview = scan
        .entries
        .iter()
        .rev()
        .find(|e| e.message == "WebView2 runtime available")
        .and_then(|e| e.get("webview2"));
    let channel = startup
        .and_then(|e| e.get("channel"))
        .filter(|c| *c == "direct" || *c == "store");
    let flag = |k: &str| -> Value {
        match startup.and_then(|e| e.get(k)) {
            Some("true") => Value::Bool(true),
            Some("false") => Value::Bool(false),
            _ => Value::Null,
        }
    };
    let latest = |m: &[openframe_persistence::migrate::Migration]| {
        m.iter().map(|m| m.version).max().unwrap_or(0)
    };
    let app_db = core
        .with_app_db(openframe_persistence::migrate::current_version)
        .ok();
    let project = match core.project_opt() {
        Some(s) => {
            let current = s
                .store
                .read(openframe_persistence::migrate::current_version)
                .ok();
            let m = s.manifest.lock();
            json!({
                "open": true,
                "current": current,
                "supported": latest(crate::schema::PROJECT_MIGRATIONS),
                "projectFormatVersion": m.format_version,
                "lastSavedByAppVersion": safe(&m.app_version, is_version),
            })
        }
        None => json!({
            "open": false,
            "supported": latest(crate::schema::PROJECT_MIGRATIONS),
        }),
    };
    json!({
        "createdAt": created_at,
        "app": {
            "name": "OpenFrame Studio",
            "version": safe(&core.config.app_version, is_version),
        },
        "system": {
            "os": os_name(),
            "osVersion": os_version(),
            "architecture": std::env::consts::ARCH,
        },
        "distribution": {
            "channel": channel.unwrap_or("not recorded"),
            "packaged": flag("packaged"),
            "inAppUpdates": flag("in_app_updates"),
            "source": "Most recent startup entry in the local log",
        },
        "webView2Version": webview,
        "schemaVersions": {
            "appDatabase": { "current": app_db, "supported": latest(crate::schema::APP_MIGRATIONS) },
            "project": project,
            "globalIdeaVault": { "supported": latest(crate::schema::GLOBAL_MIGRATIONS) },
        },
        "logs": {
            "filesRead": scan.files_read,
            "entriesRead": scan.entries.len(),
            "linesSkipped": scan.skipped,
            "from": scan.entries.first().map(|e| e.ts.clone()),
            "to": scan.entries.last().map(|e| e.ts.clone()),
        },
    })
}

fn os_name() -> String {
    sysinfo::System::name()
        .map(|s| safe(&s, is_label))
        .unwrap_or_else(|| openframe_platform::current_os().name().to_string())
}

/// "11 (26200)" on Windows 11 (registry `CurrentBuildNumber`, read by sysinfo).
fn os_version() -> String {
    sysinfo::System::os_version()
        .map(|s| safe(&s, is_label))
        .unwrap_or_else(|| "unknown".into())
}

// ------------------------------------------------------------------ AI status

fn ai_json(core: &AppCore) -> Value {
    use crate::modules::ai::{AiEmptyArgs, offline};
    let actor = core.local_actor();
    let status = match offline::status(core, &actor, AiEmptyArgs {}) {
        Ok(s) => json!({
            "available": s.available,
            "bundleState": safe(&s.bundle_state, is_camel),
            "problemCode": s.problem.as_ref().map(|e| safe(e.code_str(), is_code)),
            "runtimeState": safe(&s.runtime_state, is_camel),
            "localOnly": s.local_only,
        }),
        Err(e) => json!({ "available": false, "errorCode": safe(e.code_str(), is_code) }),
    };
    let details = match offline::diagnostics(core, &actor, AiEmptyArgs {}) {
        Ok(d) => json!({
            "bundleId": safe(&d.bundle_id, is_token),
            "bundleVersion": safe(&d.bundle_version, is_token),
            "bundleState": safe(&d.bundle_state, is_camel),
            "bundleBytes": d.bundle_bytes,
            "chatEngineState": safe(&d.chat_state, is_camel),
            "searchEngineState": safe(&d.search_state, is_camel),
            "components": d.components.iter().map(|c| json!({
                "role": safe(&c.role, is_label),
                "id": safe(&c.id, is_token),
                "version": safe(&c.version, is_token),
                "bytes": c.bytes,
            })).collect::<Vec<_>>(),
        }),
        Err(e) => json!({ "available": false, "errorCode": safe(e.code_str(), is_code) }),
    };
    json!({
        "status": status,
        "details": details,
        "omitted": "Folders, file paths and free-text runtime messages are never included.",
    })
}

// ------------------------------------------------------------------ logs

/// Newest daily log files read (rotation keeps 14).
const MAX_LOG_FILES: usize = 3;
/// Bytes read from the end of the logs, newest file first.
const MAX_LOG_BYTES: u64 = 2 << 20;
/// Longer lines are skipped, never parsed.
const MAX_LINE_BYTES: usize = 16 << 10;
const MAX_ENTRIES: usize = 20_000;
const TAIL_ENTRIES: usize = 300;
const MAX_GROUPS: usize = 100;
const OMITTED: &str = "[omitted]";
const MESSAGE_OMITTED: &str = "[message omitted]";

/// OpenFrame's own fixed log messages (tracing call sites). Only these are copied; any
/// other message text is replaced, so a message can never carry content into the bundle.
/// A new log message shows as "[message omitted]" until it is added here.
const KNOWN_MESSAGES: &[&str] = &[
    "a model file could not be held during its check",
    "an interrupted AI change set could not be fully undone",
    "an old OpenFrame AI set could not be removed yet",
    "applied migration checksum differs from shipped migration",
    "background task failed",
    "could not group AI-applied undo steps",
    "could not index one item",
    "could not open the WebView2 download page",
    "could not record an automatic history point",
    "could not record diagnostics export activity",
    "could not record export activity",
    "could not record the Offline AI check",
    "could not recover an interrupted AI change set",
    "could not remove purged file; it was left in place",
    "could not roll back a partially applied AI change set",
    "could not start the intelligence indexer",
    "diagnostics bundle exported",
    "embedding runtime failed; semantic search stays unavailable for now",
    "error while closing project at shutdown",
    "failed to deliver event to UI",
    "failed to start application core",
    "graph expansion unavailable",
    "intelligence index discarded; rebuilding",
    "intelligence index unavailable; AI uses search and tools only",
    "intelligence indexer did not stop in time; it will exit on its own",
    "intelligence indexing failed; retrieval falls back to search",
    "journal closed",
    "lexical retrieval failed; using scope and tools only",
    "mutation failed to persist",
    "Offline AI could not initialise; the rest of OpenFrame is unaffected",
    "offline AI runtime could not be placed in a kill-on-close job",
    "offline AI runtime could not be restarted",
    "offline AI runtime could not use its loopback port; starting again on a new port",
    "offline AI runtime exited and will not be restarted automatically",
    "offline AI runtime exited unexpectedly; restarting",
    "OpenFrame Studio starting",
    "operation",
    "operation failed",
    "project did not close cleanly; recovery will be offered next time",
    "project lock released",
    "recovered an AI change set interrupted while applying",
    "retrieval candidates no longer valid (deleted/private/stale index)",
    "retrieval unavailable for this request",
    "search index rebuild failed; search may be incomplete",
    "semantic retrieval failed; using search",
    "slow operation",
    "the AI engine on the graphics card stopped; going on on the processor",
    "the AI engine place was chosen",
    "the AI trace could not find the request of a proposal",
    "the AI trace could not read the undo step",
    "the bundled Offline AI failed its check; AI stays off",
    "the engine did not start for the graphics card check",
    "the graphics card check could not be recorded",
    "the graphics card check could not run fully; the processor is used",
    "the graphics card check ran",
    "the graphics card check request failed",
    "the graphics card engine could not list devices",
    "the graphics card record could not be read; it starts again",
    "the model host or manifest key this build was made with is not valid; OpenFrame AI cannot be downloaded",
    "the Offline AI is not usable",
    "the OpenFrame AI download stopped",
    "the previous OpenFrame AI set could not be removed yet",
    "unloading offline AI model after inactivity",
    "WAL journal mode unavailable; continuing with fallback mode",
    "WebView2 runtime available",
    "WebView2 runtime not available",
    "worker could not start",
];

/// `std::io::ErrorKind` names logged as `kind = ?e.kind()`.
const IO_KINDS: &[&str] = &[
    "NotFound",
    "PermissionDenied",
    "AlreadyExists",
    "WouldBlock",
    "InvalidInput",
    "InvalidData",
    "TimedOut",
    "WriteZero",
    "Interrupted",
    "Unsupported",
    "UnexpectedEof",
    "OutOfMemory",
    "StorageFull",
    "ReadOnlyFilesystem",
    "ResourceBusy",
    "DirectoryNotEmpty",
    "IsADirectory",
    "NotADirectory",
    "Other",
];

#[derive(Debug, Default)]
struct LogScan {
    entries: Vec<Entry>,
    files_read: usize,
    skipped: usize,
}

/// One log line, rebuilt from allow-listed parts only.
#[derive(Debug)]
struct Entry {
    ts: String,
    level: &'static str,
    target: String,
    message: String,
    fields: Vec<(String, String)>,
}

impl Entry {
    fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .filter(|v| *v != OMITTED)
    }
    fn ms(&self) -> Option<u64> {
        self.get("ms").and_then(|v| v.parse().ok())
    }
}

fn scan_logs(core: &AppCore) -> LogScan {
    let dir = core.config.app_data_dir.join("logs");
    let mut scan = LogScan::default();
    let mut per_file: Vec<Vec<String>> = Vec::new();
    let mut budget = MAX_LOG_BYTES;
    for path in log_files(&dir).into_iter().rev() {
        if budget == 0 {
            break;
        }
        let Ok((lines, read)) = read_tail(&path, budget) else {
            continue;
        };
        budget = budget.saturating_sub(read);
        scan.files_read += 1;
        per_file.push(lines);
    }
    // Oldest first.
    for lines in per_file.into_iter().rev() {
        for line in lines {
            match parse_line(core, &line) {
                Some(e) => scan.entries.push(e),
                None => scan.skipped += 1,
            }
        }
    }
    if scan.entries.len() > MAX_ENTRIES {
        let extra = scan.entries.len() - MAX_ENTRIES;
        scan.entries.drain(..extra);
    }
    scan
}

/// Daily logs `openframe.YYYY-MM-DD.log` (regular files only, never links), oldest first,
/// at most the newest [`MAX_LOG_FILES`].
fn log_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<(String, PathBuf)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if !is_log_name(&name) {
                return None;
            }
            let md = std::fs::symlink_metadata(e.path()).ok()?;
            md.is_file().then(|| (name, e.path()))
        })
        .collect();
    v.sort();
    let skip = v.len().saturating_sub(MAX_LOG_FILES);
    v.into_iter().skip(skip).map(|(_, p)| p).collect()
}

fn is_log_name(name: &str) -> bool {
    let Some(date) = name
        .strip_prefix("openframe.")
        .and_then(|s| s.strip_suffix(".log"))
    else {
        return false;
    };
    date.len() == 10
        && date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
}

/// The last `budget` bytes of `path` as complete lines (a cut first line is dropped).
fn read_tail(path: &Path, budget: u64) -> std::io::Result<(Vec<String>, u64)> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let start = len.saturating_sub(budget);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.take(budget).read_to_end(&mut buf)?;
    let read = buf.len() as u64;
    let mut slice = buf.as_slice();
    if start > 0 {
        match slice.iter().position(|b| *b == b'\n') {
            Some(i) => slice = &slice[i + 1..],
            None => slice = &[],
        }
    }
    let lines = slice
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty() && l.len() <= MAX_LINE_BYTES)
        .filter_map(|l| std::str::from_utf8(l).ok())
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();
    Ok((lines, read))
}

fn parse_line(core: &AppCore, line: &str) -> Option<Entry> {
    let v: Value = serde_json::from_str(line).ok()?;
    let obj = v.as_object()?;
    let level = match obj.get("level")?.as_str()? {
        "ERROR" => "ERROR",
        "WARN" => "WARN",
        "INFO" => "INFO",
        "DEBUG" => "DEBUG",
        "TRACE" => "TRACE",
        _ => return None,
    };
    let ts = obj
        .get("timestamp")
        .and_then(Value::as_str)
        .filter(|t| is_timestamp(t))
        .unwrap_or("")
        .to_string();
    let target = obj
        .get("target")
        .and_then(Value::as_str)
        .filter(|t| is_target(t))
        .unwrap_or("other")
        .to_string();
    let empty = serde_json::Map::new();
    let fields = obj
        .get("fields")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let raw_message = fields.get("message").and_then(Value::as_str).unwrap_or("");
    let message = sanitize_message(&target, raw_message);
    let mut out = Vec::new();
    for (k, v) in fields {
        if k == "message" || !is_field_key(k) {
            continue;
        }
        out.push((k.clone(), sanitize_field(core, k, v)));
        if out.len() >= 16 {
            break;
        }
    }
    Some(Entry {
        ts,
        level,
        target,
        message,
        fields: out,
    })
}

fn sanitize_message(target: &str, raw: &str) -> String {
    if KNOWN_MESSAGES.contains(&raw) {
        return raw.to_string();
    }
    // The panic hook logs "panic at <file>:<line>: <message>"; keep the source location
    // only (panic messages can quote values).
    if target.starts_with("openframe")
        && let Some(rest) = raw.strip_prefix("panic at ")
    {
        let loc = rest.split(": ").next().unwrap_or("");
        return if is_location(loc) {
            format!("panic at {loc} (message omitted)")
        } else {
            "panic (details omitted)".into()
        };
    }
    MESSAGE_OMITTED.into()
}

fn sanitize_field(core: &AppCore, key: &str, v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            let ok = match key {
                // An operation name is kept only if OpenFrame registers it (the webview
                // could send any string as an op name).
                "op" => core.registry.get(s).is_some(),
                "code" | "action" => is_code(s),
                "kind" => is_code(s) || IO_KINDS.contains(&s.as_str()),
                "channel" => s == "direct" || s == "store",
                "version" | "webview2" => is_version(s),
                "state" | "phase" => is_camel(s) || is_code(s),
                _ => false,
            };
            if ok { s.clone() } else { OMITTED.into() }
        }
        _ => OMITTED.into(),
    }
}

// ------------------------------------------------------------------ errors and timings

fn errors_json(scan: &LogScan) -> Value {
    #[derive(Default)]
    struct G {
        level: &'static str,
        count: u64,
        first: String,
        last: String,
    }
    let mut groups: BTreeMap<(String, String, String), G> = BTreeMap::new();
    for e in &scan.entries {
        let code = e.get("code");
        if code.is_none() && e.level != "ERROR" && e.level != "WARN" {
            continue;
        }
        let key = (
            code.unwrap_or("-").to_string(),
            e.get("op").unwrap_or("-").to_string(),
            e.message.clone(),
        );
        let g = groups.entry(key).or_insert_with(|| G {
            level: e.level,
            first: e.ts.clone(),
            ..G::default()
        });
        g.count += 1;
        g.last = e.ts.clone();
        if e.level == "ERROR" {
            g.level = "ERROR";
        }
    }
    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(b.1.last.cmp(&a.1.last)));
    let total = rows.len();
    rows.truncate(MAX_GROUPS);
    json!({
        "note": "Error codes and operation names from the local log, grouped. Messages are OpenFrame's own fixed texts.",
        "groups": total,
        "errors": rows.into_iter().map(|((code, op, message), g)| json!({
            "code": code,
            "operation": op,
            "message": message,
            "level": g.level,
            "count": g.count,
            "firstSeen": g.first,
            "lastSeen": g.last,
        })).collect::<Vec<_>>(),
    })
}

fn timings_json(scan: &LogScan) -> Value {
    let mut by_op: BTreeMap<&str, (Vec<u64>, u64)> = BTreeMap::new();
    for e in &scan.entries {
        let (Some(op), Some(ms)) = (e.get("op"), e.ms()) else {
            continue;
        };
        let slot = by_op.entry(op).or_default();
        slot.0.push(ms);
        if e.get("code").is_some() {
            slot.1 += 1;
        }
    }
    let mut rows: Vec<Value> = by_op
        .into_iter()
        .map(|(op, (mut ms, failed))| {
            ms.sort_unstable();
            json!({
                "operation": op,
                "count": ms.len(),
                "failed": failed,
                "slowerThan250ms": ms.iter().filter(|m| **m > 250).count(),
                "medianMs": ms[ms.len() / 2],
                "maxMs": ms[ms.len() - 1],
            })
        })
        .collect();
    rows.sort_by(|a, b| b["maxMs"].as_u64().cmp(&a["maxMs"].as_u64()));
    rows.truncate(MAX_GROUPS * 2);
    json!({
        "note": "At the default log level only slow (over 250 ms) and failed operations are logged.",
        "operations": rows,
    })
}

fn log_tail(scan: &LogScan) -> String {
    let mut out = String::from(
        "# OpenFrame log tail (most recent entries, oldest first)\n\
         # Rebuilt from fixed messages, operation names, error codes and numbers only.\n\
         # Any other text is shown as [message omitted] or [omitted].\n",
    );
    let skip = scan.entries.len().saturating_sub(TAIL_ENTRIES);
    for e in scan.entries.iter().skip(skip) {
        let mut line = format!("{} {:<5} {}: {}", e.ts, e.level, e.target, e.message);
        // The log fields arrive in key order; show the operation, its error code and its
        // duration first so a support reader sees "op=… code=… ms=…" together.
        let rank = |k: &str| match k {
            "op" => 0,
            "code" => 1,
            "ms" => 2,
            _ => 3,
        };
        let mut fields: Vec<_> = e.fields.iter().collect();
        fields.sort_by_key(|(k, _)| rank(k));
        for (k, v) in fields {
            line.push_str(&format!(" {k}={v}"));
        }
        let line: String = line.chars().take(400).collect();
        out.push_str(&line);
        out.push('\n');
    }
    out
}

// ------------------------------------------------------------------ defence in depth

/// OpenFrame's folders and the user profile: removed from every file even though the
/// collectors never copy a path.
fn sensitive_paths(core: &AppCore) -> Vec<String> {
    let mut v: Vec<PathBuf> = vec![
        core.config.app_data_dir.clone(),
        core.config.projects_dir.clone(),
        core.config.global_vault_dir.clone(),
    ];
    if let Some(s) = core.project_opt() {
        v.push(s.layout.root().to_path_buf());
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        v.push(PathBuf::from(home));
    }
    let mut out = Vec::new();
    for p in v {
        let s = p.to_string_lossy().into_owned();
        if s.len() < 4 {
            continue;
        }
        out.push(s.replace('\\', "\\\\"));
        out.push(s.replace('\\', "/"));
        out.push(s);
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.len()));
    out.dedup();
    out
}

fn scrub(text: &str, paths: &[String]) -> String {
    let mut s = text.to_string();
    for p in paths {
        if let Ok(re) = regex::RegexBuilder::new(&regex::escape(p))
            .case_insensitive(true)
            .build()
        {
            s = re.replace_all(&s, "[path removed]").into_owned();
        }
    }
    openframe_security::redact(&s)
}

// ------------------------------------------------------------------ shapes

fn safe(s: &str, ok: fn(&str) -> bool) -> String {
    if ok(s) { s.to_string() } else { OMITTED.into() }
}

/// `validation.invalid_input`, `story.create_act`, `packages.backup`.
fn is_code(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
}

/// `0.1.0`, `154.0.4258.37`, `0.1.0-test`.
fn is_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.starts_with(|c: char| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
}

/// Debug names of state enums: `Ready`, `NotInstalled`, `LoadingModel`.
fn is_camel(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.starts_with(|c: char| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphabetic())
        && !s
            .as_bytes()
            .windows(2)
            .any(|w| w[0].is_ascii_uppercase() && w[1].is_ascii_uppercase())
}

/// Catalog identifiers and versions: `gemma-3-1b-it`, `2026.10.1`.
fn is_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

/// Short fixed labels: `Windows`, `11 (26200)`, `Graphics card`.
fn is_label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 40
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '(' | ')' | '.' | '-'))
}

fn is_field_key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// `openframe_application::store`, `reqwest::connect`.
fn is_target(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':'))
}

/// `2026-09-30T06:08:22.082235Z`.
fn is_timestamp(s: &str) -> bool {
    (20..=40).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '-' | ':' | '.' | 'T' | 'Z' | '+'))
}

/// `src/modules/story.rs:120` (the panic hook keeps the last three path components).
fn is_location(s: &str) -> bool {
    let Some((file, line)) = s.rsplit_once(':') else {
        return false;
    };
    !file.is_empty()
        && file.len() <= 120
        && file.ends_with(".rs")
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
        && !line.is_empty()
        && line.len() <= 6
        && line.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_accept_identifiers_and_refuse_text() {
        assert!(is_code("storage.disk_full"));
        assert!(!is_code("OFMARKER.code"));
        assert!(!is_code("a b"));
        assert!(is_version("154.0.4258.37"));
        assert!(!is_version("v1 beta"));
        assert!(is_camel("NotInstalled"));
        assert!(!is_camel("OFMARKERKEY"));
        assert!(!is_camel("Ready7"));
        assert!(is_location("src/modules/story.rs:120"));
        assert!(!is_location("C:\\Users\\x\\a.rs:1"));
        assert!(!is_location("src/a.rs: secret"));
        assert!(is_log_name("openframe.2026-09-30.log"));
        assert!(!is_log_name("openframe.log"));
        assert!(!is_log_name("openframe.2026-09-30.log.bak"));
        assert!(is_timestamp("2026-09-30T06:08:22.082235Z"));
        assert!(!is_timestamp("yesterday at noon, honestly"));
    }

    #[test]
    fn unknown_messages_and_panic_text_are_dropped() {
        assert_eq!(
            sanitize_message("openframe_application", "operation failed"),
            "operation failed"
        );
        assert_eq!(
            sanitize_message("openframe_application", "INT. HOUSE - NIGHT"),
            MESSAGE_OMITTED
        );
        assert_eq!(
            sanitize_message(
                "openframe_desktop_lib",
                "panic at src/modules/story.rs:12: called unwrap on JANE'S LINE"
            ),
            "panic at src/modules/story.rs:12 (message omitted)"
        );
        assert_eq!(
            sanitize_message("reqwest::connect", "panic at x.rs:1: y"),
            MESSAGE_OMITTED
        );
    }

    #[test]
    fn scrub_removes_folders_in_every_spelling() {
        let paths = vec![
            "C:\\\\Films\\\\Rain".to_string(),
            "C:/Films/Rain".to_string(),
            "C:\\Films\\Rain".to_string(),
        ];
        let s = scrub(
            "a C:\\films\\rain\\x b \"C:\\\\Films\\\\Rain\" c C:/Films/Rain d",
            &paths,
        );
        assert!(!s.to_lowercase().contains("rain"), "{s}");
    }
}
