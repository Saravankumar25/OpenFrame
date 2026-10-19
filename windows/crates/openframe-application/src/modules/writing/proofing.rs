//! Proofing (SCRIPT-020): project spelling languages, the on and off switch, checking text
//! against the operating system's dictionaries, and the personal dictionary.
//!
//! - The project keeps a main language and an optional second language (for mixed-language
//!   scripts) in `project.settings_json` under `proofing`. Changing them is one undoable step.
//! - Text is split into runs by writing system. A run is checked only with the selected
//!   languages written in that system and installed on this computer. A run in a writing system
//!   with no installed dictionary (for example Telugu on most computers) is never underlined.
//! - When two selected languages share a writing system (English and French, say), a word is
//!   underlined only when neither dictionary knows it.
//! - Words in the personal dictionary are never underlined. The personal dictionary belongs to
//!   the person at this computer and lives in the application database.
//! - Nothing leaves the computer: the operating system's local dictionaries do the checking.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use openframe_domain::{Actor, AppError, AppResult, Capability};
use openframe_platform::spelling::{self, Misspelling, SpellEngine, SpellError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{read_app_setting, require_local, write_app_setting};
use crate::core::AppCore;
use crate::events::AppEvent;
use crate::registry::Registry;
use crate::store::MutationMeta;

/// `sys_settings` key of the personal dictionary (a JSON array of words).
pub const DICTIONARY_KEY: &str = "writing.personal_dictionary";
/// Project settings key of the proofing settings.
const SETTINGS_KEY: &str = "proofing";
/// The language used when a project has not chosen one.
pub const DEFAULT_LANGUAGE: &str = "en-US";

pub const MAX_DICTIONARY_WORDS: usize = 5_000;
pub const MAX_WORD_CHARS: usize = 64;
const MAX_ITEMS: usize = 400;
const MAX_ITEM_BYTES: usize = crate::modules::screenplay::edit::MAX_ELEMENT_BYTES;
const MAX_TOTAL_BYTES: usize = 512 * 1024;
const MAX_KEY_CHARS: usize = 64;
/** One check request never takes longer than this, even when the system service is slow. */
const CHECK_DEADLINE: Duration = Duration::from_secs(8);

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("writing.proofing", proofing).meta(
        M::read("Proofing settings of the project and the spelling languages installed on this computer.")
            .hidden(h::UI_FLOW),
    );
    r.command("writing.set_proofing", set_proofing).meta(
        M::edit("Turn spell checking on or off and choose the project's main and second spelling language.")
            .hidden(h::PREFERENCE),
    );
    r.query("writing.check_spelling", check_spelling).meta(
        M::compute("Words the spelling dictionaries do not know in the given editor text.")
            .hidden(h::UI_FLOW),
    );
    r.query("writing.spelling_suggestions", spelling_suggestions)
        .meta(M::compute("Spelling suggestions for one word.").hidden(h::UI_FLOW));
    r.query("writing.dictionary", dictionary).meta(
        M::app_query("Words in the personal dictionary of the person at this computer.")
            .hidden(h::PREFERENCE),
    );
    r.command("writing.dictionary_add", dictionary_add)
        .meta(M::app_command("Add a word to the personal dictionary.").hidden(h::PREFERENCE));
    r.command("writing.dictionary_remove", dictionary_remove)
        .meta(M::app_command("Remove a word from the personal dictionary.").hidden(h::PREFERENCE));
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProofingSettings {
    pub enabled: bool,
    /// BCP 47 tag such as `en-US`.
    pub main_language: String,
    pub second_language: Option<String>,
}

impl Default for ProofingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            main_language: DEFAULT_LANGUAGE.into(),
            second_language: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WritingProofingState {
    pub settings: ProofingSettings,
    /// Spelling languages installed on this computer.
    pub installed: Vec<String>,
    /// False when this system has no spelling adapter (macOS until the Mac adapter is built).
    pub adapter_available: bool,
    /// The spelling service could not answer just now (the installed list may be incomplete).
    pub service_error: bool,
    pub can_edit: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingProofingArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingSetProofingArgs {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub main_language: Option<String>,
    /// An empty string clears the second language.
    #[serde(default)]
    pub second_language: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpellingItem {
    /// Caller's key for this text (an element id).
    pub key: String,
    pub text: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingCheckSpellingArgs {
    pub items: Vec<SpellingItem>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SpellingMiss {
    /// UTF-16 offset in the item text (what the editor uses).
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SpellingResult {
    pub key: String,
    pub misses: Vec<SpellingMiss>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WritingSpellingCheck {
    pub enabled: bool,
    pub results: Vec<SpellingResult>,
    /// Selected languages that are not installed on this computer.
    pub missing_languages: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingWordArgs {
    pub word: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WritingSpellingSuggestions {
    pub suggestions: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingDictionaryArgs {}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WritingDictionary {
    pub words: Vec<String>,
}

// ------------------------------------------------------------------ settings

/// A BCP 47 language tag of the shape the operating systems use (`en-US`, `te-IN`, `sr-Latn-RS`).
pub fn valid_language_tag(tag: &str) -> bool {
    let parts: Vec<&str> = tag.split('-').collect();
    if tag.len() > 35 || parts.is_empty() || parts.len() > 4 {
        return false;
    }
    let first = parts[0];
    if !(2..=3).contains(&first.len()) || !first.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    parts[1..]
        .iter()
        .all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

fn language_or_error(tag: &str) -> AppResult<String> {
    let t = tag.trim();
    if !valid_language_tag(t) {
        return Err(AppError::validation(
            "language",
            "Choose a language from the list.",
        ));
    }
    Ok(t.to_string())
}

pub(crate) fn read_settings(c: &rusqlite::Connection) -> AppResult<ProofingSettings> {
    let raw: String = c.query_row("SELECT settings_json FROM project", [], |r| r.get(0))?;
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    Ok(v.get(SETTINGS_KEY)
        .and_then(|p| serde_json::from_value::<ProofingSettings>(p.clone()).ok())
        .filter(|p| valid_language_tag(&p.main_language))
        .unwrap_or_default())
}

fn proofing(
    core: &AppCore,
    actor: &Actor,
    _a: WritingProofingArgs,
) -> AppResult<WritingProofingState> {
    actor.require(Capability::View, "view this project")?;
    let settings = core.project()?.store.read(read_settings)?;
    let installed = spelling::system().languages();
    Ok(WritingProofingState {
        settings,
        service_error: installed.is_err(),
        installed: installed.unwrap_or_default(),
        adapter_available: spelling::adapter_available(),
        can_edit: actor.can(Capability::Edit),
    })
}

fn set_proofing(
    core: &AppCore,
    actor: &Actor,
    a: WritingSetProofingArgs,
) -> AppResult<ProofingSettings> {
    let main = a
        .main_language
        .as_deref()
        .map(language_or_error)
        .transpose()?;
    let second = match a.second_language.as_deref().map(str::trim) {
        Some("") => Some(None),
        Some(t) => Some(Some(language_or_error(t)?)),
        None => None,
    };
    let s = core.project()?;
    let summary = match (a.enabled, &main, &second) {
        (Some(true), None, None) => "Turned spell checking on",
        (Some(false), None, None) => "Turned spell checking off",
        _ => "Changed the spelling languages",
    };
    s.store.mutate(
        actor,
        MutationMeta::new("writing.set_proofing", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let mut p = read_settings(c)?;
            if let Some(e) = a.enabled {
                p.enabled = e;
            }
            if let Some(m) = main {
                p.main_language = m;
            }
            if let Some(sl) = second {
                p.second_language = sl;
            }
            if p.second_language.as_deref() == Some(p.main_language.as_str()) {
                p.second_language = None;
            }
            let (id, raw): (String, String) =
                c.query_row("SELECT id, settings_json FROM project", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            let mut merged: Value =
                serde_json::from_str(&raw).unwrap_or_else(|_| Value::Object(Default::default()));
            if !merged.is_object() {
                merged = json!({});
            }
            merged[SETTINGS_KEY] =
                serde_json::to_value(&p).map_err(|e| AppError::internal(e.to_string()))?;
            openframe_persistence::rows::update_fields(
                c,
                "project",
                &id,
                &[(
                    "settings_json",
                    openframe_persistence::rows::text(merged.to_string()),
                )],
                &["settings_json"],
                None,
                "project",
            )?;
            Ok(p)
        },
    )
}

// ------------------------------------------------------------------ checking

/// The writing system a character belongs to. Spaces, digits and punctuation have none and
/// stay with the run they are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script {
    Latin,
    Greek,
    Cyrillic,
    Hebrew,
    Arabic,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    /// Any other writing system (Chinese, Thai, ...). Never checked.
    Other,
}

pub fn script_of(c: char) -> Option<Script> {
    let u = c as u32;
    let s = match u {
        0x41..=0x5A | 0x61..=0x7A | 0xC0..=0x24F | 0x1E00..=0x1EFF => Script::Latin,
        0x370..=0x3FF | 0x1F00..=0x1FFF => Script::Greek,
        0x400..=0x52F => Script::Cyrillic,
        0x590..=0x5FF => Script::Hebrew,
        0x600..=0x6FF | 0x750..=0x77F => Script::Arabic,
        0x900..=0x97F | 0xA8E0..=0xA8FF => Script::Devanagari,
        0x980..=0x9FF => Script::Bengali,
        0xA00..=0xA7F => Script::Gurmukhi,
        0xA80..=0xAFF => Script::Gujarati,
        0xB00..=0xB7F => Script::Oriya,
        0xB80..=0xBFF => Script::Tamil,
        0xC00..=0xC7F => Script::Telugu,
        0xC80..=0xCFF => Script::Kannada,
        0xD00..=0xD7F => Script::Malayalam,
        _ if c.is_alphabetic() => Script::Other,
        _ => return None,
    };
    Some(s)
}

/// The writing system a language is written in.
pub fn script_of_language(tag: &str) -> Script {
    let lower = tag.to_ascii_lowercase();
    if lower.contains("-latn") {
        return Script::Latin;
    }
    if lower.contains("-cyrl") {
        return Script::Cyrillic;
    }
    match lower.split('-').next().unwrap_or("") {
        "hi" | "mr" | "ne" | "sa" | "kok" | "mai" => Script::Devanagari,
        "bn" | "as" => Script::Bengali,
        "pa" => Script::Gurmukhi,
        "gu" => Script::Gujarati,
        "or" => Script::Oriya,
        "ta" => Script::Tamil,
        "te" => Script::Telugu,
        "kn" => Script::Kannada,
        "ml" => Script::Malayalam,
        "ru" | "uk" | "bg" | "be" | "mk" | "sr" | "kk" | "ky" | "mn" | "tg" => Script::Cyrillic,
        "el" => Script::Greek,
        "he" | "yi" => Script::Hebrew,
        "ar" | "fa" | "ur" | "ps" | "sd" | "ug" => Script::Arabic,
        "af" | "az" | "bs" | "ca" | "cs" | "cy" | "da" | "de" | "en" | "es" | "et" | "eu"
        | "fi" | "fil" | "fr" | "ga" | "gl" | "hr" | "hu" | "id" | "is" | "it" | "lb" | "lt"
        | "lv" | "ms" | "mt" | "nb" | "nl" | "nn" | "no" | "pl" | "pt" | "ro" | "sk" | "sl"
        | "sq" | "sv" | "sw" | "tl" | "tr" | "uz" | "vi" | "zu" | "xh" | "rm" | "fo" | "fy" => {
            Script::Latin
        }
        _ => Script::Other,
    }
}

/// Runs of one writing system: (script, start, end) in UTF-16 units.
pub fn script_runs(text: &str) -> Vec<(Script, usize, usize)> {
    let mut runs: Vec<(Script, usize, usize)> = Vec::new();
    let mut pos = 0usize;
    for ch in text.chars() {
        let w = ch.len_utf16();
        // Combining marks (Indic vowel signs and viramas sit inside their block, so they are
        // matched above) and joiners belong to the run they follow.
        let is_joiner = matches!(ch, '\u{200C}' | '\u{200D}');
        match (script_of(ch), runs.last_mut()) {
            (Some(s), Some(last)) if last.0 == s => last.2 = pos + w,
            (Some(s), _) => runs.push((s, pos, pos + w)),
            (None, Some(last)) => {
                if is_joiner || ch == '\'' || ch == '\u{2019}' {
                    last.2 = pos + w;
                }
            }
            (None, None) => {}
        }
        pos += w;
    }
    runs
}

fn utf16_slice(units: &[u16], start: usize, end: usize) -> String {
    String::from_utf16_lossy(&units[start..end])
}

/// Unknown words in `text`, never underlining a word in the personal dictionary or a run with
/// no installed dictionary for its writing system.
pub fn check_text(
    engine: &dyn SpellEngine,
    languages: &[String],
    personal: &HashSet<String>,
    text: &str,
    deadline: Instant,
) -> Result<Vec<SpellingMiss>, SpellError> {
    let units: Vec<u16> = text.encode_utf16().collect();
    // Runs of each writing system are checked together in one call per language (joined by a
    // space), so mixed text such as "a \u{0431} a \u{0431}" never turns into thousands of calls.
    let mut by_script: Vec<(Script, Vec<(usize, usize)>)> = Vec::new();
    for (script, start, end) in script_runs(text) {
        if script == Script::Other {
            continue;
        }
        match by_script.iter_mut().find(|(s, _)| *s == script) {
            Some((_, runs)) => runs.push((start, end)),
            None => by_script.push((script, vec![(start, end)])),
        }
    }
    let mut out: Vec<SpellingMiss> = Vec::new();
    for (script, runs) in by_script {
        let langs: Vec<&String> = languages
            .iter()
            .filter(|l| script_of_language(l) == script)
            .collect();
        if langs.is_empty() {
            continue;
        }
        // Joined text and, per run, (offset in the joined text, start, end in the item text).
        let mut joined: Vec<u16> = Vec::new();
        let mut map: Vec<(usize, usize, usize)> = Vec::with_capacity(runs.len());
        for (start, end) in runs {
            if !joined.is_empty() {
                joined.push(u16::from(b' '));
            }
            map.push((joined.len(), start, end));
            joined.extend_from_slice(&units[start..end]);
        }
        let joined = String::from_utf16_lossy(&joined);
        // A word is unknown only if every selected dictionary for this script rejects it.
        let mut unknown: Option<HashSet<(u32, u32)>> = None;
        for l in langs {
            if Instant::now() > deadline {
                return Err(SpellError::Unavailable("spell check took too long".into()));
            }
            let found: HashSet<(u32, u32)> = engine
                .check(l, &joined)?
                .into_iter()
                .map(|Misspelling { start, len }| (start, len))
                .collect();
            unknown = Some(match unknown {
                None => found,
                Some(prev) => prev.intersection(&found).copied().collect(),
            });
        }
        let mut misses: Vec<(u32, u32)> = unknown.unwrap_or_default().into_iter().collect();
        misses.sort_unstable();
        for (s, len) in misses {
            let s = s as usize;
            // The run this miss starts in (the last run starting at or before it).
            let i = map.partition_point(|(at, _, _)| *at <= s);
            if i == 0 {
                continue;
            }
            let (at, start, end) = map[i - 1];
            let a = start + (s - at);
            let b = (a + len as usize).min(end);
            if a >= b || b > units.len() {
                continue;
            }
            let word = utf16_slice(&units, a, b);
            if personal.contains(&dictionary_key(&word)) {
                continue;
            }
            out.push(SpellingMiss {
                start: a as u32,
                end: b as u32,
            });
        }
    }
    out.sort_by_key(|m| m.start);
    Ok(out)
}

/// The selected languages that are installed, main language first.
fn active_languages(
    settings: &ProofingSettings,
    installed: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut active = Vec::new();
    let mut missing = Vec::new();
    let wanted = std::iter::once(&settings.main_language).chain(settings.second_language.iter());
    for l in wanted {
        if installed.iter().any(|i| i.eq_ignore_ascii_case(l)) {
            active.push(l.clone());
        } else {
            missing.push(l.clone());
        }
    }
    (active, missing)
}

fn check_spelling(
    core: &AppCore,
    actor: &Actor,
    a: WritingCheckSpellingArgs,
) -> AppResult<WritingSpellingCheck> {
    actor.require(Capability::View, "view this project")?;
    if a.items.len() > MAX_ITEMS {
        return Err(AppError::invalid_input("Too much text to check at once."));
    }
    let mut total = 0usize;
    for it in &a.items {
        if it.key.chars().count() > MAX_KEY_CHARS || it.text.len() > MAX_ITEM_BYTES {
            return Err(AppError::invalid_input("Too much text to check at once."));
        }
        total += it.text.len();
    }
    if total > MAX_TOTAL_BYTES {
        return Err(AppError::invalid_input("Too much text to check at once."));
    }
    let settings = core.project()?.store.read(read_settings)?;
    if !settings.enabled {
        return Ok(WritingSpellingCheck {
            enabled: false,
            results: Vec::new(),
            missing_languages: Vec::new(),
        });
    }
    let engine = spelling::system();
    let installed = engine.languages().map_err(spell_error)?;
    let (active, missing) = active_languages(&settings, &installed);
    let deadline = Instant::now() + CHECK_DEADLINE;
    let personal = personal_set(core)?;
    let mut results = Vec::with_capacity(a.items.len());
    for it in a.items {
        let misses =
            check_text(engine, &active, &personal, &it.text, deadline).map_err(spell_error)?;
        results.push(SpellingResult {
            key: it.key,
            misses,
        });
    }
    Ok(WritingSpellingCheck {
        enabled: true,
        results,
        missing_languages: missing,
    })
}

fn spell_error(e: SpellError) -> AppError {
    match e {
        SpellError::TooLarge => AppError::invalid_input("Too much text to check at once."),
        SpellError::LanguageNotInstalled => AppError::new(
            "proofing.language_not_installed",
            "That spelling language is not installed on this computer.",
        ),
        SpellError::Unavailable(d) => AppError::new(
            "proofing.unavailable",
            "Spell checking is not available right now. Your writing is not affected.",
        )
        .retryable()
        .with_detail(d),
    }
}

fn spelling_suggestions(
    core: &AppCore,
    actor: &Actor,
    a: WritingWordArgs,
) -> AppResult<WritingSpellingSuggestions> {
    actor.require(Capability::View, "view this project")?;
    let word = a.word.trim();
    if word.is_empty() || word.chars().count() > MAX_WORD_CHARS {
        return Ok(WritingSpellingSuggestions {
            suggestions: Vec::new(),
        });
    }
    let settings = core.project()?.store.read(read_settings)?;
    let engine = spelling::system();
    let installed = engine.languages().map_err(spell_error)?;
    let (active, _) = active_languages(&settings, &installed);
    let script = word.chars().find_map(script_of);
    let mut suggestions: Vec<String> = Vec::new();
    for l in active
        .iter()
        .filter(|l| Some(script_of_language(l)) == script)
    {
        for s in engine.suggest(l, word).map_err(spell_error)? {
            if !suggestions.contains(&s) && suggestions.len() < 6 {
                suggestions.push(s);
            }
        }
    }
    Ok(WritingSpellingSuggestions { suggestions })
}

// ------------------------------------------------------------------ personal dictionary

/// Case-insensitive identity of a dictionary word.
pub fn dictionary_key(word: &str) -> String {
    word.trim().to_lowercase()
}

fn valid_word(word: &str) -> AppResult<String> {
    let w = word.trim();
    if w.is_empty() {
        return Err(AppError::required("Word"));
    }
    if w.chars().count() > MAX_WORD_CHARS {
        return Err(AppError::validation(
            "word",
            format!("A dictionary word can have at most {MAX_WORD_CHARS} characters."),
        ));
    }
    if w.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AppError::validation(
            "word",
            "Add one word at a time, without spaces.",
        ));
    }
    if !w.chars().any(char::is_alphabetic) {
        return Err(AppError::validation(
            "word",
            "A word needs at least one letter.",
        ));
    }
    Ok(w.to_string())
}

pub(crate) fn personal_words(core: &AppCore) -> AppResult<Vec<String>> {
    Ok(read_app_setting(core, DICTIONARY_KEY)?
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|w| valid_word(w).is_ok())
        .take(MAX_DICTIONARY_WORDS)
        .collect())
}

fn personal_set(core: &AppCore) -> AppResult<HashSet<String>> {
    Ok(personal_words(core)?
        .iter()
        .map(|w| dictionary_key(w))
        .collect())
}

fn save_words(core: &AppCore, words: &[String]) -> AppResult<()> {
    write_app_setting(core, DICTIONARY_KEY, Some(&json!(words)))?;
    core.emit(AppEvent::Module {
        module: "writing".into(),
        name: "dictionaryChanged".into(),
        payload: json!({ "count": words.len() }),
    });
    Ok(())
}

fn dictionary(
    core: &AppCore,
    _a: &Actor,
    _: WritingDictionaryArgs,
) -> AppResult<WritingDictionary> {
    Ok(WritingDictionary {
        words: personal_words(core)?,
    })
}

fn dictionary_add(
    core: &AppCore,
    actor: &Actor,
    a: WritingWordArgs,
) -> AppResult<WritingDictionary> {
    require_local(actor, "the personal dictionary")?;
    let word = valid_word(&a.word)?;
    let _guard = super::settings_lock();
    let mut words = personal_words(core)?;
    let key = dictionary_key(&word);
    if !words.iter().any(|w| dictionary_key(w) == key) {
        if words.len() >= MAX_DICTIONARY_WORDS {
            return Err(AppError::invalid_input(format!(
                "The personal dictionary is full ({MAX_DICTIONARY_WORDS} words). Remove words you no longer need first."
            )));
        }
        words.push(word);
        words.sort_by_key(|w| dictionary_key(w));
        save_words(core, &words)?;
    }
    Ok(WritingDictionary { words })
}

fn dictionary_remove(
    core: &AppCore,
    actor: &Actor,
    a: WritingWordArgs,
) -> AppResult<WritingDictionary> {
    require_local(actor, "the personal dictionary")?;
    let key = dictionary_key(&a.word);
    let _guard = super::settings_lock();
    let mut words = personal_words(core)?;
    let before = words.len();
    words.retain(|w| dictionary_key(w) != key);
    if words.len() != before {
        save_words(core, &words)?;
    }
    Ok(WritingDictionary { words })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A dictionary per language: a word is known when it is in the list.
    struct Fake(HashMap<&'static str, Vec<&'static str>>);

    impl SpellEngine for Fake {
        fn languages(&self) -> Result<Vec<String>, SpellError> {
            Ok(self.0.keys().map(|k| k.to_string()).collect())
        }
        fn check(&self, language: &str, text: &str) -> Result<Vec<Misspelling>, SpellError> {
            let known = self
                .0
                .get(language)
                .ok_or(SpellError::LanguageNotInstalled)?;
            let mut out = Vec::new();
            let mut pos = 0u32;
            let mut word = String::new();
            let mut start = 0u32;
            let flush = |word: &mut String, start: u32, out: &mut Vec<Misspelling>| {
                if !word.is_empty() && !known.iter().any(|k| k.eq_ignore_ascii_case(word)) {
                    out.push(Misspelling {
                        start,
                        len: word.encode_utf16().count() as u32,
                    });
                }
                word.clear();
            };
            for ch in text.chars() {
                if ch.is_alphanumeric()
                    || ch == '\u{0C4D}'
                    || script_of(ch).is_some() && !ch.is_whitespace()
                {
                    if word.is_empty() {
                        start = pos;
                    }
                    word.push(ch);
                } else {
                    flush(&mut word, start, &mut out);
                }
                pos += ch.len_utf16() as u32;
            }
            flush(&mut word, start, &mut out);
            Ok(out)
        }
        fn suggest(&self, _language: &str, _word: &str) -> Result<Vec<String>, SpellError> {
            Ok(vec!["the".into()])
        }
    }

    fn far() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    fn fake() -> Fake {
        Fake(HashMap::from([
            (
                "en-US",
                vec!["meera", "walks", "into", "the", "room", "and", "says"],
            ),
            ("fr-FR", vec!["bonjour", "the"]),
        ]))
    }

    fn spans(text: &str, misses: &[SpellingMiss]) -> Vec<String> {
        let u: Vec<u16> = text.encode_utf16().collect();
        misses
            .iter()
            .map(|m| String::from_utf16_lossy(&u[m.start as usize..m.end as usize]))
            .collect()
    }

    #[test]
    fn fsd_script_020_unknown_words_are_reported_with_utf16_offsets() {
        let t = "Meera walks into teh room.";
        let m = check_text(&fake(), &["en-US".into()], &HashSet::new(), t, far()).unwrap();
        assert_eq!(spans(t, &m), vec!["teh"]);
    }

    #[test]
    fn fsd_script_020_personal_dictionary_words_are_never_underlined() {
        let t = "Meera walks into Kothapeta.";
        let none = check_text(&fake(), &["en-US".into()], &HashSet::new(), t, far()).unwrap();
        assert_eq!(spans(t, &none), vec!["Kothapeta"]);
        let personal = HashSet::from([dictionary_key("KOTHAPETA")]);
        assert!(
            check_text(&fake(), &["en-US".into()], &personal, t, far())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn fsd_script_020_indian_script_without_a_dictionary_is_not_underlined() {
        // Telugu and Hindi text with only English installed: no false underlines, and the
        // English words around them are still checked with correct offsets.
        let t = "Meera says నమస్తే అండి and नमस्ते teh room.";
        let m = check_text(
            &fake(),
            &["en-US".into(), "te-IN".into()],
            &HashSet::new(),
            t,
            far(),
        );
        // te-IN is not installed in the fake, so the caller would not pass it; check that an
        // installed-only list never touches the Telugu run.
        assert!(m.is_err(), "the engine refuses languages it does not have");
        let m = check_text(&fake(), &["en-US".into()], &HashSet::new(), t, far()).unwrap();
        assert_eq!(spans(t, &m), vec!["teh"]);
    }

    #[test]
    fn fsd_script_020_second_language_accepts_its_words() {
        let t = "Meera says bonjour.";
        let only_en = check_text(&fake(), &["en-US".into()], &HashSet::new(), t, far()).unwrap();
        assert_eq!(spans(t, &only_en), vec!["bonjour"]);
        let both = check_text(
            &fake(),
            &["en-US".into(), "fr-FR".into()],
            &HashSet::new(),
            t,
            far(),
        )
        .unwrap();
        assert!(
            both.is_empty(),
            "a word either dictionary knows is not underlined"
        );
    }

    #[test]
    fn fsd_script_020_mixed_scripts_map_offsets_back_and_stay_bounded() {
        // Alternating scripts: one engine call per language, offsets still exact.
        let t = "teh \u{0431} Meera \u{0431} teh \u{0C2E} walks";
        let m = check_text(&fake(), &["en-US".into()], &HashSet::new(), t, far()).unwrap();
        assert_eq!(spans(t, &m), vec!["teh", "teh"]);
        let units: Vec<u16> = t.encode_utf16().collect();
        assert_eq!(
            m[1].start as usize,
            units.len() - "teh \u{0C2E} walks".encode_utf16().count()
        );
        // An emoji (two UTF-16 units) before the word keeps the offset right.
        let t = "\u{1F3AC} teh";
        let m = check_text(&fake(), &["en-US".into()], &HashSet::new(), t, far()).unwrap();
        assert_eq!(m[0].start, 3);
        // Past the deadline nothing more is sent to the service.
        let late = Instant::now() - Duration::from_secs(1);
        assert!(check_text(&fake(), &["en-US".into()], &HashSet::new(), "teh", late).is_err());
    }

    #[test]
    fn script_runs_follow_writing_systems() {
        let runs = script_runs("ab నమ cd");
        assert_eq!(
            runs.iter().map(|r| r.0).collect::<Vec<_>>(),
            vec![Script::Latin, Script::Telugu, Script::Latin]
        );
        // Emoji and other symbols are neutral; offsets are UTF-16.
        let runs = script_runs("\u{1F3AC}ab");
        assert_eq!(runs, vec![(Script::Latin, 2, 4)]);
        assert!(script_runs("").is_empty());
    }

    #[test]
    fn language_tags_map_to_scripts_and_are_validated() {
        assert_eq!(script_of_language("en-US"), Script::Latin);
        assert_eq!(script_of_language("te-IN"), Script::Telugu);
        assert_eq!(script_of_language("hi-IN"), Script::Devanagari);
        assert_eq!(script_of_language("sr-Latn-RS"), Script::Latin);
        assert_eq!(script_of_language("ja-JP"), Script::Other);
        assert!(valid_language_tag("en-US"));
        assert!(valid_language_tag("sr-Latn-RS"));
        assert!(!valid_language_tag("EN-us"));
        assert!(!valid_language_tag("en_US"));
        assert!(!valid_language_tag("e"));
        assert!(!valid_language_tag(&"a-".repeat(40)));
    }

    #[test]
    fn dictionary_words_are_validated() {
        assert!(valid_word("Kothapeta").is_ok());
        assert!(valid_word("నమస్తే").is_ok());
        assert!(valid_word("two words").is_err());
        assert!(valid_word("1234").is_err());
        assert!(valid_word("").is_err());
        assert!(valid_word(&"a".repeat(MAX_WORD_CHARS + 1)).is_err());
    }

    #[test]
    fn active_languages_report_missing_dictionaries() {
        let s = ProofingSettings {
            enabled: true,
            main_language: "en-US".into(),
            second_language: Some("te-IN".into()),
        };
        let (active, missing) = active_languages(&s, &["en-US".into()]);
        assert_eq!(active, vec!["en-US"]);
        assert_eq!(missing, vec!["te-IN"]);
    }
}
