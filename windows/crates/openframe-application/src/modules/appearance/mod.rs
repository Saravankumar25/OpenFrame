//! Appearance: Light, Dark and System modes, the built-in themes and My Themes (build plan area
//! "Appearance and themes"; ADR-0014; Screenplay and Production Depth Specification section 47A;
//! docs/engineering/30-appearance-themes.md).
//!
//! Preferences and imported themes live in the application database (`sys_appearance_pref`,
//! `sys_user_theme`, app migration 0002), never in a project. The operations are application
//! level, hidden from the assistant and need no undo step: they are personal preferences, not
//! project content. Theme files are untrusted and go through [`theme::parse_theme`] and the
//! contrast check before anything is stored; stored rows are validated again on every read, so a
//! damaged or edited row can never be applied and the active theme falls back to OpenFrame
//! Default.

pub mod color;
pub mod theme;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::core::AppCore;
use crate::events::{AppEvent, StoreKind};
use crate::registry::Registry;

use theme::{ContrastRow, DEFAULT_THEME_ID, MAX_NAME_CHARS, MAX_THEME_BYTES, Origin, Seeds, Theme};

/// Why the assistant never sees these operations (published in the AI tool coverage matrix).
pub const HIDDEN_REASON: &str = "Appearance (mode, theme and My Themes) is a personal preference of the person at this computer, not project content; the assistant never reads or changes how the app looks.";

/// The table name the UI listens to for appearance changes.
pub const TABLE: &str = "sys_appearance";

/// Most themes My Themes keeps.
pub const MAX_USER_THEMES: usize = 100;

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OpClass, OperationMetadata as M};
    r.module("Appearance");
    r.query("appearance.get", get).meta(
        M::app_query("The appearance mode, the active theme and the themes the user can choose.")
            .hidden(HIDDEN_REASON),
    );
    r.command("appearance.set_mode", set_mode).meta(
        M::app_command("Switch between Light, Dark and System appearance.").hidden(HIDDEN_REASON),
    );
    r.command("appearance.set_theme", set_theme).meta(
        M::app_command("Apply a built-in theme or one from My Themes.").hidden(HIDDEN_REASON),
    );
    r.query("appearance.check_theme", check_theme).meta(
        M::app_query(
            "Read and check a theme file or pasted theme text and show its preview, without saving anything.",
        )
        .class(OpClass::Compute)
        .fs(Fs::ReadsUserFile)
        .hidden(HIDDEN_REASON),
    );
    r.command("appearance.import_theme", import_theme).meta(
        M::app_command(
            "Save a checked theme file or pasted theme text to My Themes, and apply it if asked.",
        )
        .fs(Fs::ReadsUserFile)
        .hidden(HIDDEN_REASON),
    );
    r.command("appearance.rename_theme", rename_theme)
        .meta(M::app_command("Rename a theme in My Themes.").hidden(HIDDEN_REASON));
    r.command("appearance.export_theme", export_theme).meta(
        M::app_command(
            "Save a theme from My Themes as a theme file at a location the user picked.",
        )
        .fs(Fs::WritesUserFile)
        .hidden(HIDDEN_REASON),
    );
    // A theme is a small preference file, not project content: removing it has no undo step and no
    // Recently Deleted entry, so the UI confirms first. Projects and other themes are untouched.
    r.command("appearance.delete_theme", delete_theme).meta(
        M::app_command(
            "Delete a theme from My Themes. If it is active, OpenFrame Default is applied.",
        )
        .destructive()
        .irreversible()
        .confirm()
        .hidden(HIDDEN_REASON),
    );
}

// ------------------------------------------------------------------ DTOs

/// The user's mode preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum AppearanceMode {
    Light,
    Dark,
    System,
}

impl AppearanceMode {
    fn as_str(self) -> &'static str {
        match self {
            AppearanceMode::Light => "light",
            AppearanceMode::Dark => "dark",
            AppearanceMode::System => "system",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "light" => Some(AppearanceMode::Light),
            "dark" => Some(AppearanceMode::Dark),
            "system" => Some(AppearanceMode::System),
            _ => None,
        }
    }
}

/// A theme as the picker and the style layer use it. Every colour is canonical text rebuilt from
/// parsed numbers (`#rrggbb` or `oklch(L% C H)`).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ThemeDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub built_in: bool,
    pub seeds: Seeds,
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}

impl ThemeDto {
    fn from_theme(t: &Theme, built_in: bool) -> Self {
        let side = |m: &BTreeMap<String, color::ColorValue>| {
            m.iter().map(|(k, v)| (k.clone(), v.css())).collect()
        };
        ThemeDto {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            author: t.author.clone(),
            built_in,
            seeds: t.seeds,
            light: side(&t.light),
            dark: side(&t.dark),
        }
    }
}

/// One entry of My Themes. `problem` is set when the stored theme no longer passes the checks;
/// such a theme is listed (so it can be deleted) but cannot be applied.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MyTheme {
    pub id: String,
    pub name: String,
    pub theme: Option<ThemeDto>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub mode: AppearanceMode,
    /// The theme actually shown (always usable).
    pub theme_id: String,
    /// Set when the saved theme could not be used and OpenFrame Default is shown instead.
    pub fallback_notice: Option<String>,
    /// Imported themes. Built-in themes come from the shared list the webview reads directly.
    pub my_themes: Vec<MyTheme>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppearanceGetArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetModeArgs {
    pub mode: AppearanceMode,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetThemeArgs {
    pub theme_id: String,
}

/// A theme to check or import: exactly one of a file the user picked or pasted text.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeSourceArgs {
    #[ts(optional)]
    pub path: Option<String>,
    #[ts(optional)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportThemeArgs {
    #[ts(optional)]
    pub path: Option<String>,
    #[ts(optional)]
    pub text: Option<String>,
    /// Apply the theme after saving it.
    pub apply: bool,
    /// `digest` of the checked preview: the save is refused when the source changed since.
    #[ts(optional)]
    pub expected_digest: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameThemeArgs {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportThemeArgs {
    pub id: String,
    /// Destination chosen in the Save dialog (`.json` is added when missing).
    pub path: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteThemeArgs {
    pub id: String,
}

/// Result of checking a theme before import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ThemeCheckStatus {
    /// Valid and readable as it is.
    Ok,
    /// Valid; small automatic readability corrections were made (listed in `notes`).
    Corrected,
    /// Cannot be used. `problems` says why. Nothing was changed.
    Rejected,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ThemeCheck {
    pub status: ThemeCheckStatus,
    /// The validated (and corrected) theme, for the preview. None when the file could not be read.
    pub theme: Option<ThemeDto>,
    pub problems: Vec<String>,
    pub notes: Vec<String>,
    pub contrast: Vec<ContrastRow>,
    /// SHA-256 of the checked text, passed back on save so the saved theme is the one previewed.
    pub digest: Option<String>,
}

/// Most problems listed for one theme (a hostile file could otherwise produce hundreds).
const MAX_PROBLEMS: usize = 20;

fn bounded(mut problems: Vec<String>) -> Vec<String> {
    problems.dedup();
    if problems.len() > MAX_PROBLEMS {
        let more = problems.len() - MAX_PROBLEMS;
        problems.truncate(MAX_PROBLEMS);
        problems.push(format!("And {more} more problems."));
    }
    problems
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ExportThemeResult {
    pub path: String,
    pub file_name: String,
}

// ------------------------------------------------------------------ storage

const KEY_MODE: &str = "mode";
const KEY_THEME: &str = "theme";

fn read_pref(c: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT value FROM sys_appearance_pref WHERE key=?1",
        [key],
        |r| r.get(0),
    )
    .optional()?)
}

fn write_pref(c: &Connection, key: &str, value: &str) -> AppResult<()> {
    c.execute(
        "INSERT INTO sys_appearance_pref(key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
        params![key, value, now_ms()],
    )?;
    Ok(())
}

struct Row {
    id: String,
    name: String,
    description: String,
    author: String,
    seeds_json: String,
    overrides_json: String,
    format_version: i64,
}

fn rows(c: &Connection, id: Option<&str>) -> AppResult<Vec<Row>> {
    let mut st = c.prepare(
        "SELECT id, name, description, author, seeds_json, overrides_json, format_version
         FROM sys_user_theme WHERE (?1 IS NULL OR id = ?1) ORDER BY created_at, id",
    )?;
    let out = st
        .query_map([id], |r| {
            Ok(Row {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                author: r.get(3)?,
                seeds_json: r.get(4)?,
                overrides_json: r.get(5)?,
                format_version: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(out)
}

/// Validate a stored row exactly like an imported file (it may have been damaged or edited
/// outside OpenFrame). Returns the checked theme or the reason it cannot be used.
fn revalidate(row: &Row) -> Result<Theme, String> {
    let seeds: Value = serde_json::from_str(&row.seeds_json)
        .map_err(|_| "Its saved data is damaged.".to_string())?;
    let overrides: Value = serde_json::from_str(&row.overrides_json)
        .map_err(|_| "Its saved data is damaged.".to_string())?;
    let mut doc = serde_json::Map::new();
    doc.insert("formatVersion".into(), row.format_version.into());
    doc.insert("id".into(), row.id.clone().into());
    doc.insert("name".into(), row.name.clone().into());
    doc.insert("description".into(), row.description.clone().into());
    doc.insert("author".into(), row.author.clone().into());
    doc.insert("seeds".into(), seeds);
    if overrides.as_object().is_some_and(|o| !o.is_empty()) {
        doc.insert("overrides".into(), overrides);
    }
    let text = Value::Object(doc).to_string();
    let parsed = theme::parse_theme(&text, Origin::Imported).map_err(|p| p.join(" "))?;
    let checked = theme::check(parsed);
    if checked.failures.is_empty() {
        Ok(checked.theme)
    } else {
        Err("It no longer passes the readability check.".into())
    }
}

fn my_themes(c: &Connection) -> AppResult<Vec<(MyTheme, Option<Theme>)>> {
    Ok(rows(c, None)?
        .into_iter()
        .map(|row| match revalidate(&row) {
            Ok(t) => (
                MyTheme {
                    id: row.id.clone(),
                    name: t.name.clone(),
                    theme: Some(ThemeDto::from_theme(&t, false)),
                    problem: None,
                },
                Some(t),
            ),
            Err(problem) => (
                MyTheme {
                    id: row.id.clone(),
                    name: theme::clean_text(&row.name, "theme name", MAX_NAME_CHARS, true)
                        .unwrap_or_else(|_| row.id.clone()),
                    theme: None,
                    problem: Some(problem),
                },
                None,
            ),
        })
        .collect())
}

fn state(core: &AppCore) -> AppResult<AppearanceState> {
    core.with_app_db(|c| {
        let mode = read_pref(c, KEY_MODE)?
            .and_then(|m| AppearanceMode::parse(&m))
            .unwrap_or(AppearanceMode::System);
        let mine = my_themes(c)?;
        let wanted = read_pref(c, KEY_THEME)?.unwrap_or_else(|| DEFAULT_THEME_ID.to_string());
        let usable = theme::builtin(&wanted).is_some()
            || mine.iter().any(|(m, t)| m.id == wanted && t.is_some());
        let (theme_id, fallback_notice) = if usable {
            (wanted, None)
        } else {
            (
                DEFAULT_THEME_ID.to_string(),
                Some(
                    "Your saved theme could not be used, so OpenFrame Default is shown. Your projects are not affected."
                        .to_string(),
                ),
            )
        };
        Ok(AppearanceState {
            mode,
            theme_id,
            fallback_notice,
            my_themes: mine.into_iter().map(|(m, _)| m).collect(),
        })
    })
}

fn changed(core: &AppCore) {
    core.emit(AppEvent::DataChanged {
        store: StoreKind::Project,
        tables: vec![TABLE.to_string()],
        ids: Vec::new(),
        origin: "local".to_string(),
    });
}

/// Appearance belongs to the person at this computer: never to package changes or the assistant.
fn require_local(actor: &Actor) -> AppResult<()> {
    if matches!(actor.origin, ActorOrigin::Local) {
        Ok(())
    } else {
        Err(AppError::new(
            "permission.denied",
            "Only the person using this computer can change the appearance.",
        ))
    }
}

// ------------------------------------------------------------------ handlers

fn get(core: &AppCore, _a: &Actor, _: AppearanceGetArgs) -> AppResult<AppearanceState> {
    state(core)
}

fn set_mode(core: &AppCore, actor: &Actor, args: SetModeArgs) -> AppResult<AppearanceState> {
    require_local(actor)?;
    core.with_app_db(|c| write_pref(c, KEY_MODE, args.mode.as_str()))?;
    changed(core);
    state(core)
}

fn set_theme(core: &AppCore, actor: &Actor, args: SetThemeArgs) -> AppResult<AppearanceState> {
    require_local(actor)?;
    let id = args.theme_id;
    let usable = theme::valid_id(&id)
        && (theme::builtin(&id).is_some()
            || core.with_app_db(|c| {
                Ok(rows(c, Some(&id))?
                    .first()
                    .is_some_and(|r| revalidate(r).is_ok()))
            })?);
    if !usable {
        return Err(AppError::new(
            "not_found.theme",
            "That theme is not available. Choose another theme.",
        ));
    }
    core.with_app_db(|c| write_pref(c, KEY_THEME, &id))?;
    changed(core);
    state(core)
}

/// Read the theme text from exactly one source, bounded to 10 KB.
fn source_text(path: Option<&str>, text: Option<&str>) -> Result<String, String> {
    match (path, text) {
        (Some(p), None) => read_theme_file(Path::new(p)),
        (None, Some(t)) => {
            if t.len() > MAX_THEME_BYTES {
                Err(format!(
                    "The theme is larger than 10 KB ({} KB). Theme files are small; this is not one.",
                    t.len().div_ceil(1024)
                ))
            } else {
                Ok(t.to_string())
            }
        }
        _ => Err("Choose a theme file or paste theme text.".into()),
    }
}

fn read_theme_file(path: &Path) -> Result<String, String> {
    // Theme files are tiny: a file on a network share is refused (no SMB connection is made for a
    // theme), then the usual rules (no device names or streams, symlink target checked).
    openframe_security::check_user_path(path, openframe_security::NetworkPaths::Refuse).map_err(
        |_| {
            "Copy the theme file to this computer first (for example to Downloads), then import it."
                .to_string()
        },
    )?;
    openframe_security::validate_input_file(path, u64::MAX).map_err(|e| e.message.clone())?;
    let file =
        std::fs::File::open(path).map_err(|_| "The theme file could not be opened.".to_string())?;
    // Check the opened handle too: the path could have been swapped after the checks above.
    if !file.metadata().map(|m| m.is_file()).unwrap_or(false) {
        return Err("Please choose a theme file, not a folder or device.".into());
    }
    // Read at most one byte past the limit, so a file that grows after the check is still refused.
    let mut buf = Vec::with_capacity(MAX_THEME_BYTES + 1);
    file.take(MAX_THEME_BYTES as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|_| "The theme file could not be read.".to_string())?;
    if buf.len() > MAX_THEME_BYTES {
        return Err(
            "The theme file is larger than 10 KB. Theme files are small; this is not one.".into(),
        );
    }
    let text = String::from_utf8(buf).map_err(|_| {
        "This is not a theme file. OpenFrame theme files are small JSON documents.".to_string()
    })?;
    Ok(text
        .strip_prefix('\u{feff}')
        .map(str::to_string)
        .unwrap_or(text))
}

/// Parse, check ids against My Themes and run the contrast check. Returns the preview and, when
/// the theme may be saved, the checked theme itself (from this one read of the source).
fn evaluate(
    core: &AppCore,
    path: Option<&str>,
    text: Option<&str>,
) -> AppResult<(ThemeCheck, Option<Theme>)> {
    let rejected =
        |problems: Vec<String>, theme: Option<ThemeDto>, contrast: Vec<ContrastRow>| ThemeCheck {
            status: ThemeCheckStatus::Rejected,
            theme,
            problems: bounded(problems),
            notes: Vec::new(),
            contrast,
            digest: None,
        };
    let text = match source_text(path, text) {
        Ok(t) => t,
        Err(p) => return Ok((rejected(vec![p], None, Vec::new()), None)),
    };
    let digest = openframe_security::sha256_bytes(text.as_bytes());
    let parsed = match theme::parse_theme(&text, Origin::Imported) {
        Ok(t) => t,
        Err(problems) => return Ok((rejected(problems, None, Vec::new()), None)),
    };
    let exists = core.with_app_db(|c| Ok(!rows(c, Some(&parsed.id))?.is_empty()))?;
    let checked = theme::check(parsed);
    let dto = ThemeDto::from_theme(&checked.theme, false);
    let mut problems = Vec::new();
    if exists {
        problems.push(format!(
            "A theme with the id \"{}\" is already in My Themes. Delete that theme first, or change the id in the file.",
            checked.theme.id
        ));
    }
    for f in &checked.failures {
        problems.push(format!(
            "{} mode: {} is too hard to read ({:.2} to 1; at least 4.5 to 1 is needed).",
            if f.mode == theme::Mode::Light {
                "Light"
            } else {
                "Dark"
            },
            f.label,
            f.ratio
        ));
    }
    if !problems.is_empty() {
        let check = ThemeCheck {
            notes: checked.notes,
            ..rejected(problems, Some(dto), checked.rows)
        };
        return Ok((check, None));
    }
    let check = ThemeCheck {
        status: if checked.notes.is_empty() {
            ThemeCheckStatus::Ok
        } else {
            ThemeCheckStatus::Corrected
        },
        theme: Some(dto),
        problems: Vec::new(),
        notes: checked.notes,
        contrast: checked.rows,
        digest: Some(digest),
    };
    Ok((check, Some(checked.theme)))
}

fn check_theme(core: &AppCore, actor: &Actor, args: ThemeSourceArgs) -> AppResult<ThemeCheck> {
    // It reads a file the user picked: only the person at this computer may ask.
    require_local(actor)?;
    Ok(evaluate(core, args.path.as_deref(), args.text.as_deref())?.0)
}

fn import_theme(
    core: &AppCore,
    actor: &Actor,
    args: ImportThemeArgs,
) -> AppResult<AppearanceState> {
    require_local(actor)?;
    // The source is read and checked again here: nothing from an earlier preview is trusted.
    let (check, theme) = evaluate(core, args.path.as_deref(), args.text.as_deref())?;
    let Some(t) = theme else {
        let first = check.problems.first().cloned().unwrap_or_default();
        return Err(AppError::invalid_input(format!(
            "This theme cannot be used. {first} Nothing was changed."
        )));
    };
    let stale = args
        .expected_digest
        .as_deref()
        .is_some_and(|expected| check.digest.as_deref() != Some(expected));
    if stale {
        return Err(AppError::invalid_input(
            "The theme file changed after it was checked. Check it again. Nothing was changed.",
        ));
    }
    let file = t.to_file_json();
    let seeds = file["seeds"].to_string();
    let overrides = file
        .get("overrides")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()))
        .to_string();
    let now = now_ms();
    core.with_app_db(|c| {
        let count: i64 = c.query_row("SELECT COUNT(*) FROM sys_user_theme", [], |r| r.get(0))?;
        if count as usize >= MAX_USER_THEMES {
            return Err(AppError::invalid_input(format!(
                "My Themes is full ({MAX_USER_THEMES} themes). Delete a theme first. Nothing was changed."
            )));
        }
        let inserted = c.execute(
            "INSERT INTO sys_user_theme(id, name, description, author, seeds_json, overrides_json, format_version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8) ON CONFLICT(id) DO NOTHING",
            params![t.id, t.name, t.description, t.author, seeds, overrides, theme::FORMAT_VERSION as i64, now],
        )?;
        if inserted == 0 {
            return Err(AppError::invalid_input(format!(
                "A theme with the id \"{}\" is already in My Themes. Nothing was changed.",
                t.id
            )));
        }
        if args.apply {
            write_pref(c, KEY_THEME, &t.id).map_err(|e| {
                AppError::new(
                    e.code_str().to_string(),
                    "The theme was saved to My Themes but could not be applied. Apply it from My Themes.",
                )
            })?;
        }
        Ok(())
    })?;
    changed(core);
    state(core)
}

fn rename_theme(
    core: &AppCore,
    actor: &Actor,
    args: RenameThemeArgs,
) -> AppResult<AppearanceState> {
    require_local(actor)?;
    let name = theme::clean_text(&args.name, "theme name", MAX_NAME_CHARS, true)
        .map_err(AppError::invalid_input)?;
    // The same rule as on import: a renamed theme must still pass the check on every read.
    theme::not_a_builtin_name(&name).map_err(AppError::invalid_input)?;
    let n = core.with_app_db(|c| {
        Ok(c.execute(
            "UPDATE sys_user_theme SET name=?1, updated_at=?2 WHERE id=?3",
            params![name, now_ms(), args.id],
        )?)
    })?;
    if n == 0 {
        return Err(AppError::not_found("theme"));
    }
    changed(core);
    state(core)
}

fn export_theme(
    core: &AppCore,
    actor: &Actor,
    args: ExportThemeArgs,
) -> AppResult<ExportThemeResult> {
    require_local(actor)?;
    let row = core
        .with_app_db(|c| rows(c, Some(&args.id)))?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("theme"))?;
    let t = revalidate(&row)
        .map_err(|p| AppError::invalid_input(format!("This theme cannot be exported. {p}")))?;
    let mut path = PathBuf::from(args.path.trim());
    let has_json = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if !has_json {
        let mut s = path.into_os_string();
        s.push(".json");
        path = PathBuf::from(s);
    }
    crate::util::check_output_file(core, &path)?;
    let mut text = serde_json::to_string_pretty(&t.to_file_json())
        .map_err(|e| AppError::internal(e.to_string()))?;
    text.push('\n');
    openframe_platform::atomic::replace_bytes(&path, text.as_bytes())?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(ExportThemeResult {
        path: path.to_string_lossy().into_owned(),
        file_name,
    })
}

fn delete_theme(
    core: &AppCore,
    actor: &Actor,
    args: DeleteThemeArgs,
) -> AppResult<AppearanceState> {
    require_local(actor)?;
    core.with_app_db(|c| {
        if rows(c, Some(&args.id))?.is_empty() {
            return Err(AppError::not_found("theme"));
        }
        // Fall back first: if the delete were interrupted, the theme is merely unused.
        if read_pref(c, KEY_THEME)?.as_deref() == Some(args.id.as_str()) {
            write_pref(c, KEY_THEME, DEFAULT_THEME_ID)?;
        }
        c.execute("DELETE FROM sys_user_theme WHERE id=?1", [&args.id])?;
        Ok(())
    })?;
    changed(core);
    state(core)
}
