//! Theme data, the strict theme-file parser and the contrast check (ADR-0014;
//! docs/engineering/30-appearance-themes.md sections 3 to 5).
//!
//! A theme is pure data: five seed numbers plus, optionally, a few token overrides per mode. The
//! built-in list and the engine constants live in `themes.json`, which the webview reads too, so
//! there is exactly one list. Imported files are untrusted: [`parse_theme`] accepts only the
//! documented keys, bounded numbers and two colour forms, and everything it returns was rebuilt
//! from parsed numbers. The original text is never stored or shown as CSS.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

use super::color::{ColorValue, Lab, contrast};

/// Largest theme file or pasted text accepted (bytes).
pub const MAX_THEME_BYTES: usize = 10 * 1024;
/// The only file format major version this build reads.
pub const FORMAT_VERSION: u64 = 1;
/// WCAG AA for normal text.
pub const MIN_CONTRAST: f64 = 4.5;
/// The theme a new installation and every fallback uses.
pub const DEFAULT_THEME_ID: &str = "openframe-default";

pub const MAX_NAME_CHARS: usize = 60;
pub const MAX_DESCRIPTION_CHARS: usize = 200;
pub const MAX_AUTHOR_CHARS: usize = 80;

// ------------------------------------------------------------------ engine (themes.json)

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Engine {
    /// Per mode: token name to `[lightness %, chroma multiplier of the neutral tint]`.
    pub ladder: BTreeMap<String, BTreeMap<String, [f64; 2]>>,
    pub accent_lightness: ModePair<f64>,
    pub status_lightness: ModePair<f64>,
    pub soft_mix: ModePair<f64>,
    /// `[lightness %, chroma]` of the dark button text; its hue is the accent hue.
    pub dark_accent_ink: [f64; 2],
    pub light_accent_ink: String,
    pub accent_strong_mix: f64,
    pub accent_line_mix: f64,
    pub accent_ring_mix: f64,
    pub status_text_mix: f64,
    pub warning_text_mix: f64,
    pub status_line_mix: f64,
    pub warning_line_mix: f64,
    /// Status name to `[lightness offset, chroma, hue]`.
    pub status: BTreeMap<String, [f64; 3]>,
    /// Component role tokens: name to the CSS formula used when a theme does not set them.
    pub roles: BTreeMap<String, String>,
    /// Logo gradient stops derived from the accent when a theme does not set them.
    pub logo: BTreeMap<String, String>,
    /// The webview's copy of [`IMPORTABLE_TOKENS`] (a test keeps them equal).
    pub importable_tokens: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModePair<T> {
    pub light: T,
    pub dark: T,
}

impl<T: Copy> ModePair<T> {
    pub fn get(&self, mode: Mode) -> T {
        match mode {
            Mode::Light => self.light,
            Mode::Dark => self.dark,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemesFile {
    engine: Engine,
    themes: Vec<Value>,
}

/// The displayed (resolved) mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ResolvedMode")]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Light,
    Dark,
}

impl Mode {
    pub const BOTH: [Mode; 2] = [Mode::Light, Mode::Dark];
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }
}

const THEMES_JSON: &str = include_str!("themes.json");

struct Catalog {
    engine: Engine,
    builtin: Vec<Theme>,
}

fn catalog() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(|| {
        let file: ThemesFile =
            serde_json::from_str(THEMES_JSON).expect("themes.json is part of the source code");
        let builtin = file
            .themes
            .iter()
            .map(|v| {
                parse_value(v, Origin::BuiltIn)
                    .unwrap_or_else(|p| panic!("built-in theme in themes.json is not valid: {p:?}"))
            })
            .collect();
        Catalog {
            engine: file.engine,
            builtin,
        }
    })
}

pub fn engine() -> &'static Engine {
    &catalog().engine
}

/// The built-in themes, in picker order. The first is OpenFrame Default.
pub fn builtin_themes() -> &'static [Theme] {
    &catalog().builtin
}

pub fn builtin(id: &str) -> Option<&'static Theme> {
    builtin_themes().iter().find(|t| t.id == id)
}

// ------------------------------------------------------------------ tokens

/// Who supplied a theme. Built-ins (reviewed source code) may set any token; imported files only
/// the safe subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    BuiltIn,
    Imported,
}

/// Tokens an imported theme may override (docs/engineering/30-appearance-themes.md section 5).
pub const IMPORTABLE_TOKENS: &[&str] = &[
    "bg",
    "panel",
    "surface2",
    "surface3",
    "hover",
    "well",
    "chrome",
    "ink",
    "ink2",
    "muted",
    "line",
    "line2",
    "side",
    "side2",
    "side-ink",
    "side-mut",
    "accent",
    "accent-ink",
    "accent-d",
    "lg1",
    "lg2",
    "lg3",
];

/// Further tokens a built-in theme may set (OpenFrame Default's exact palette).
const BUILTIN_ONLY_TOKENS: &[&str] = &[
    "backdrop",
    "accent-soft",
    "accent-line",
    "accent-ring",
    "blue",
    "blue-soft",
    "blue-t",
    "blue-line",
    "green",
    "green-soft",
    "green-t",
    "green-line",
    "red",
    "red-soft",
    "red-t",
    "red-line",
    "purple",
    "purple-soft",
    "purple-t",
    "purple-line",
    "teal",
    "teal-soft",
    "teal-t",
    "teal-line",
    "yellow",
    "yellow-soft",
    "yellow-t",
    "yellow-line",
];

fn token_allowed(name: &str, origin: Origin) -> bool {
    IMPORTABLE_TOKENS.contains(&name)
        || (origin == Origin::BuiltIn && (BUILTIN_ONLY_TOKENS.contains(&name) || engine_role(name)))
}

fn engine_role(name: &str) -> bool {
    // During the catalog's own construction the engine is not available yet; roles are read
    // from the raw file then.
    static ROLES: OnceLock<Vec<String>> = OnceLock::new();
    ROLES
        .get_or_init(|| {
            let v: Value = serde_json::from_str(THEMES_JSON).expect("themes.json");
            v["engine"]["roles"]
                .as_object()
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default()
        })
        .iter()
        .any(|r| r == name)
}

// ------------------------------------------------------------------ theme model

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ThemeSeeds")]
#[serde(rename_all = "camelCase")]
pub struct Seeds {
    /// Hue of the neutral surfaces, 0 to 360.
    pub neutral_hue: f64,
    /// How strongly surfaces take the hue, 0 (grey) to 0.04.
    pub neutral_tint: f64,
    /// Accent hue, 0 to 360.
    pub accent_hue: f64,
    /// Accent colourfulness, 0 to 0.25.
    pub accent_chroma: f64,
    /// Accent lightness adjustment in percentage points, -12 to 16.
    pub accent_lightness_nudge: f64,
}

/// Documented seed ranges: (key, min, max).
pub const SEED_RANGES: [(&str, f64, f64); 5] = [
    ("neutralHue", 0.0, 360.0),
    ("neutralTint", 0.0, 0.04),
    ("accentHue", 0.0, 360.0),
    ("accentChroma", 0.0, 0.25),
    ("accentLightnessNudge", -12.0, 16.0),
];

/// A validated theme. Every value was parsed from numbers; nothing here is raw file text.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub seeds: Seeds,
    pub light: BTreeMap<String, ColorValue>,
    pub dark: BTreeMap<String, ColorValue>,
}

impl Theme {
    pub fn overrides(&self, mode: Mode) -> &BTreeMap<String, ColorValue> {
        match mode {
            Mode::Light => &self.light,
            Mode::Dark => &self.dark,
        }
    }

    fn overrides_mut(&mut self, mode: Mode) -> &mut BTreeMap<String, ColorValue> {
        match mode {
            Mode::Light => &mut self.light,
            Mode::Dark => &mut self.dark,
        }
    }

    /// The theme in the file format (what Export writes).
    pub fn to_file_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("formatVersion".into(), FORMAT_VERSION.into());
        o.insert("id".into(), self.id.clone().into());
        o.insert("name".into(), self.name.clone().into());
        o.insert("description".into(), self.description.clone().into());
        o.insert("author".into(), self.author.clone().into());
        o.insert(
            "seeds".into(),
            serde_json::to_value(self.seeds).expect("seeds serialize"),
        );
        if !self.light.is_empty() || !self.dark.is_empty() {
            let side = |m: &BTreeMap<String, ColorValue>| {
                Value::Object(m.iter().map(|(k, v)| (k.clone(), v.css().into())).collect())
            };
            let mut ov = Map::new();
            if !self.light.is_empty() {
                ov.insert("light".into(), side(&self.light));
            }
            if !self.dark.is_empty() {
                ov.insert("dark".into(), side(&self.dark));
            }
            o.insert("overrides".into(), Value::Object(ov));
        }
        Value::Object(o)
    }
}

// ------------------------------------------------------------------ parser

/// Parse untrusted theme text. Returns every problem found, in plain words, or the theme.
/// Never panics on any input.
pub fn parse_theme(text: &str, origin: Origin) -> Result<Theme, Vec<String>> {
    if text.len() > MAX_THEME_BYTES {
        return Err(vec![format!(
            "The theme is larger than 10 KB ({} KB). Theme files are small; this is not one.",
            text.len().div_ceil(1024)
        )]);
    }
    if text.trim().is_empty() {
        return Err(vec!["The theme is empty.".into()]);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| {
        vec![
            "This is not a theme file. OpenFrame theme files are small JSON documents.".to_string(),
        ]
    })?;
    parse_value(&value, origin)
}

fn parse_value(value: &Value, origin: Origin) -> Result<Theme, Vec<String>> {
    let mut problems = Vec::new();
    let Some(obj) = value.as_object() else {
        return Err(vec![
            "This is not a theme file. It must be one JSON object.".into(),
        ]);
    };
    const KEYS: [&str; 7] = [
        "formatVersion",
        "id",
        "name",
        "description",
        "author",
        "seeds",
        "overrides",
    ];
    for k in obj.keys() {
        if !KEYS.contains(&k.as_str()) {
            problems.push(format!(
                "The file has a setting OpenFrame does not know: \"{}\".",
                shown(k)
            ));
        }
    }
    match obj.get("formatVersion") {
        Some(Value::Number(n)) if n.as_u64() == Some(FORMAT_VERSION) => {}
        Some(Value::Number(n)) if n.as_u64().is_some_and(|v| v > FORMAT_VERSION) => problems.push(
            "This theme was made for a newer version of OpenFrame. Update OpenFrame to use it."
                .into(),
        ),
        Some(_) => problems.push("\"formatVersion\" must be 1.".into()),
        None => problems.push("The file has no \"formatVersion\".".into()),
    }
    let id = match obj.get("id").and_then(Value::as_str) {
        Some(id) if valid_id(id) => {
            if origin == Origin::Imported && builtin(id).is_some() {
                problems.push(format!(
                    "The id \"{id}\" belongs to a built-in theme. Change the id in the file."
                ));
            }
            id.to_string()
        }
        Some(id) => {
            problems.push(format!(
                "The id \"{}\" is not allowed. Use 3 to 40 lowercase letters, digits or hyphens.",
                shown(id)
            ));
            String::new()
        }
        None => {
            problems.push("The file has no \"id\".".into());
            String::new()
        }
    };
    let name = text_field(obj, "name", MAX_NAME_CHARS, true, &mut problems);
    if origin == Origin::Imported
        && let Err(p) = not_a_builtin_name(&name)
    {
        problems.push(p);
    }
    let description = text_field(
        obj,
        "description",
        MAX_DESCRIPTION_CHARS,
        false,
        &mut problems,
    );
    let author = text_field(obj, "author", MAX_AUTHOR_CHARS, false, &mut problems);
    let seeds = parse_seeds(obj.get("seeds"), &mut problems);
    let (light, dark) = parse_overrides(obj.get("overrides"), origin, &mut problems);
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(Theme {
        id,
        name,
        description,
        author,
        seeds: seeds.expect("no problems means seeds parsed"),
        light,
        dark,
    })
}

/// `^[a-z0-9-]{3,40}$`
pub fn valid_id(id: &str) -> bool {
    (3..=40).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A short, printable rendering of untrusted text for an error message.
fn shown(s: &str) -> String {
    let clean: String = s
        .chars()
        .filter(|c| !c.is_control() && !is_bidi_control(*c))
        .take(40)
        .collect();
    if s.chars().count() > 40 {
        format!("{clean}...")
    } else {
        clean
    }
}

/// A user theme may not carry a built-in theme's name (ignoring case and repeated spaces), so the
/// theme list never shows two themes that look the same. Used on import and on rename.
pub fn not_a_builtin_name(name: &str) -> Result<(), String> {
    let key = |s: &str| {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let wanted = key(name);
    if builtin_themes().iter().any(|b| key(&b.name) == wanted) {
        Err(format!(
            "\"{}\" is the name of a built-in theme. Give this theme its own name.",
            name.trim()
        ))
    } else {
        Ok(())
    }
}

fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Invisible, filler, separator, private-use and tag characters, which could make a name blank or
/// look like another. Zero-width joiner and non-joiner stay allowed: Indian scripts need them.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00ad}'
            | '\u{115f}'..='\u{1160}'
            | '\u{17b4}'..='\u{17b5}'
            | '\u{180b}'..='\u{180e}'
            | '\u{200b}'
            | '\u{2028}'..='\u{2029}'
            | '\u{2060}'..='\u{2064}'
            | '\u{206a}'..='\u{206f}'
            | '\u{3164}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{feff}'
            | '\u{ffa0}'
            | '\u{fff0}'..='\u{fffb}'
            | '\u{e000}'..='\u{f8ff}'
            | '\u{e0000}'..='\u{e007f}'
            | '\u{f0000}'..='\u{10ffff}'
    )
}

/// Validate a display name or similar text. Shown in the UI as text only, but it must still be
/// plain: no control or direction characters and no markup.
pub fn clean_text(
    raw: &str,
    label: &str,
    max_chars: usize,
    required: bool,
) -> Result<String, String> {
    let t = raw.trim();
    if t.is_empty() {
        return if required {
            Err(format!("The {label} is empty."))
        } else {
            Ok(String::new())
        };
    }
    if t.chars().count() > max_chars {
        return Err(format!(
            "The {label} is longer than {max_chars} characters."
        ));
    }
    if t.chars()
        .any(|c| c.is_control() || is_bidi_control(c) || is_invisible(c) || c == '<' || c == '>')
    {
        return Err(format!(
            "The {label} contains characters that are not allowed (invisible or control characters, or < >)."
        ));
    }
    if !t.chars().any(char::is_alphanumeric) {
        return Err(format!("The {label} needs at least one letter or digit."));
    }
    Ok(t.to_string())
}

fn text_field(
    obj: &Map<String, Value>,
    key: &str,
    max: usize,
    required: bool,
    problems: &mut Vec<String>,
) -> String {
    let label = match key {
        "name" => "theme name",
        "description" => "description",
        _ => "author",
    };
    match obj.get(key) {
        None if required => {
            problems.push(format!("The file has no \"{key}\"."));
            String::new()
        }
        None => String::new(),
        Some(Value::String(s)) => clean_text(s, label, max, required).unwrap_or_else(|e| {
            problems.push(e);
            String::new()
        }),
        Some(_) => {
            problems.push(format!("\"{key}\" must be text."));
            String::new()
        }
    }
}

fn parse_seeds(v: Option<&Value>, problems: &mut Vec<String>) -> Option<Seeds> {
    let Some(obj) = v.and_then(Value::as_object) else {
        problems.push("The file has no \"seeds\" object.".into());
        return None;
    };
    for k in obj.keys() {
        if !SEED_RANGES.iter().any(|(n, _, _)| n == k) {
            problems.push(format!(
                "\"seeds\" has a value OpenFrame does not know: \"{}\".",
                shown(k)
            ));
        }
    }
    let mut vals = [0.0f64; 5];
    let mut ok = true;
    for (i, (key, min, max)) in SEED_RANGES.iter().enumerate() {
        match obj.get(*key) {
            Some(Value::Number(n)) => match n.as_f64() {
                Some(x) if x.is_finite() && (*min..=*max).contains(&x) => vals[i] = x,
                _ => {
                    ok = false;
                    problems.push(format!("Seed \"{key}\" must be between {min} and {max}."));
                }
            },
            Some(_) => {
                ok = false;
                problems.push(format!("Seed \"{key}\" must be a number."));
            }
            None => {
                ok = false;
                problems.push(format!("Seed \"{key}\" is missing."));
            }
        }
    }
    ok.then_some(Seeds {
        neutral_hue: vals[0],
        neutral_tint: vals[1],
        accent_hue: vals[2],
        accent_chroma: vals[3],
        accent_lightness_nudge: vals[4],
    })
}

type Overrides = BTreeMap<String, ColorValue>;

/// Most overrides a built-in theme may carry per mode (an imported one: one per allowed token).
const MAX_OVERRIDES_PER_MODE: usize = 128;

fn parse_overrides(
    v: Option<&Value>,
    origin: Origin,
    problems: &mut Vec<String>,
) -> (Overrides, Overrides) {
    let mut out = (Overrides::new(), Overrides::new());
    let Some(v) = v else { return out };
    let Some(obj) = v.as_object() else {
        problems.push("\"overrides\" must be an object with \"light\" and \"dark\".".into());
        return out;
    };
    for (mode, side) in obj {
        let target = match mode.as_str() {
            "light" => &mut out.0,
            "dark" => &mut out.1,
            other => {
                problems.push(format!(
                    "\"overrides\" may only have \"light\" and \"dark\", not \"{}\".",
                    shown(other)
                ));
                continue;
            }
        };
        let Some(map) = side.as_object() else {
            problems.push(format!("\"overrides.{mode}\" must be an object."));
            continue;
        };
        let limit = match origin {
            Origin::Imported => IMPORTABLE_TOKENS.len(),
            Origin::BuiltIn => MAX_OVERRIDES_PER_MODE,
        };
        if map.len() > limit {
            problems.push(format!("\"overrides.{mode}\" has too many entries."));
            continue;
        }
        for (token, value) in map {
            if !token_allowed(token, origin) {
                problems.push(format!(
                    "\"{}\" cannot be changed by a theme file. Allowed: {}.",
                    shown(token),
                    IMPORTABLE_TOKENS.join(", ")
                ));
                continue;
            }
            let parsed = value.as_str().and_then(|s| {
                if origin == Origin::BuiltIn {
                    builtin_value(s)
                } else {
                    ColorValue::parse(s)
                }
            });
            match parsed {
                Some(c) => {
                    target.insert(token.clone(), c);
                }
                None => problems.push(format!(
                    "The colour for \"{token}\" ({mode}) is not allowed. Use #rrggbb or oklch(L% C H)."
                )),
            }
        }
    }
    out
}

/// Built-in themes may also use `#rrggbbaa` (OpenFrame Default's translucent edges). Stored as the
/// opaque colour for contrast; the webview receives the original eight digits from themes.json.
fn builtin_value(s: &str) -> Option<ColorValue> {
    match s.strip_prefix('#') {
        Some(hex)
            if (hex.len() == 4 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            let rgb = if hex.len() == 4 { &hex[..3] } else { &hex[..6] };
            ColorValue::parse(&format!("#{rgb}"))
        }
        _ => ColorValue::parse(s),
    }
}

// ------------------------------------------------------------------ resolved tokens

/// The colours the contrast check needs, resolved for one mode the way the style layer does.
#[derive(Debug, Clone)]
pub struct Resolved(BTreeMap<String, Lab>);

impl Resolved {
    pub fn get(&self, token: &str) -> Lab {
        self.0[token]
    }
}

pub const STATUS: [&str; 6] = ["blue", "green", "red", "purple", "teal", "yellow"];

pub fn resolve(theme: &Theme, mode: Mode) -> Resolved {
    let e = engine();
    let s = theme.seeds;
    let ov = theme.overrides(mode);
    let pick = |name: &str, computed: Lab| ov.get(name).map_or(computed, |c| c.lab());
    let mut t = BTreeMap::new();
    let ladder = &e.ladder[mode.as_str()];
    for (name, [l, k]) in ladder {
        let c = Lab::from_oklch(*l, s.neutral_tint * k, s.neutral_hue);
        t.insert(name.clone(), pick(name, c));
    }
    let accent = Lab::from_oklch(
        e.accent_lightness.get(mode) + s.accent_lightness_nudge,
        s.accent_chroma,
        s.accent_hue,
    );
    let accent = pick("accent", accent);
    t.insert("accent".into(), accent);
    let ink_default = match mode {
        Mode::Light => ColorValue::parse(&e.light_accent_ink)
            .expect("engine light accent ink")
            .lab(),
        Mode::Dark => Lab::from_oklch(e.dark_accent_ink[0], e.dark_accent_ink[1], s.accent_hue),
    };
    t.insert("accent-ink".into(), pick("accent-ink", ink_default));
    let ink = t["ink"];
    let panel = t["panel"];
    let mix = e.soft_mix.get(mode) / 100.0;
    t.insert(
        "accent-d".into(),
        pick("accent-d", accent.mix(e.accent_strong_mix / 100.0, ink)),
    );
    t.insert(
        "accent-soft".into(),
        pick("accent-soft", accent.mix(mix, panel)),
    );
    let sl = e.status_lightness.get(mode);
    for name in STATUS {
        let [dl, c, h] = e.status[name];
        let base = pick(name, Lab::from_oklch(sl + dl, c, h));
        let text_mix = if name == "yellow" {
            e.warning_text_mix
        } else {
            e.status_text_mix
        };
        t.insert(
            format!("{name}-soft"),
            pick(&format!("{name}-soft"), base.mix(mix, panel)),
        );
        t.insert(
            format!("{name}-t"),
            pick(&format!("{name}-t"), base.mix(text_mix / 100.0, ink)),
        );
        t.insert(name.to_string(), base);
    }
    Resolved(t)
}

// ------------------------------------------------------------------ contrast

/// The text and background pairs every theme must keep readable (spec 47A.13).
pub const PAIRS: &[(&str, &str, &str)] = &[
    ("ink", "bg", "Main text on the window background"),
    ("ink", "panel", "Main text on panels"),
    ("ink2", "panel", "Secondary text on panels"),
    ("muted", "panel", "Muted text on panels"),
    ("muted", "bg", "Muted text on the window background"),
    ("muted", "surface2", "Muted text on table headers"),
    ("side-ink", "side", "Sidebar text"),
    ("side-mut", "side", "Muted sidebar text"),
    ("accent-ink", "accent", "Button text on the accent colour"),
    ("accent-d", "accent-soft", "Selected item text"),
    ("accent-d", "panel", "Accent text on panels"),
    ("panel", "ink", "Tooltip and message text"),
    ("ink", "surface3", "Text on raised areas"),
    ("ink", "hover", "Text on a hovered row"),
    ("ink2", "chrome", "Status bar text"),
    ("blue-t", "blue-soft", "Information labels"),
    ("green-t", "green-soft", "Success labels"),
    ("red-t", "red-soft", "Error labels"),
    ("purple-t", "purple-soft", "Private labels"),
    ("teal-t", "teal-soft", "Teal labels"),
    ("yellow-t", "yellow-soft", "Warning labels"),
];

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, rename = "ThemeContrastRow")]
#[serde(rename_all = "camelCase")]
pub struct ContrastRow {
    pub mode: Mode,
    pub label: String,
    pub foreground: String,
    pub background: String,
    pub ratio: f64,
    pub passes: bool,
}

pub fn contrast_rows(theme: &Theme) -> Vec<ContrastRow> {
    let mut rows = Vec::new();
    for mode in Mode::BOTH {
        let r = resolve(theme, mode);
        for (fg, bg, label) in PAIRS {
            let ratio = contrast(r.get(fg), r.get(bg));
            rows.push(ContrastRow {
                mode,
                label: (*label).to_string(),
                foreground: (*fg).to_string(),
                background: (*bg).to_string(),
                ratio: (ratio * 100.0).floor() / 100.0,
                passes: ratio >= MIN_CONTRAST,
            });
        }
    }
    rows
}

/// Outcome of checking an imported theme.
#[derive(Debug, Clone)]
pub struct Checked {
    pub theme: Theme,
    /// What was adjusted, in plain words. Empty when nothing was.
    pub notes: Vec<String>,
    /// Pairs that still fail after the allowed corrections; non-empty means "cannot apply".
    pub failures: Vec<ContrastRow>,
    pub rows: Vec<ContrastRow>,
}

/// Largest accent lightness change (percentage points) the automatic correction may make.
pub const MAX_ACCENT_CORRECTION: f64 = 4.0;
/// Lowest share of the accent kept in the strong accent text colour by the correction.
pub const MIN_STRONG_MIX: f64 = 0.40;

/// Contrast check with the deterministic corrections that do not change a theme's character
/// (spec 47A.13): button text may switch between white and dark, the accent may move up to
/// 4 lightness points, and the strong accent text may take more of the text colour. Anything
/// else that fails is reported and the theme cannot be applied.
pub fn check(mut theme: Theme) -> Checked {
    // Check exactly what is stored and shown: the canonical text form of every value.
    for mode in Mode::BOTH {
        for v in theme.overrides_mut(mode).values_mut() {
            *v = ColorValue::parse(&v.css()).unwrap_or(*v);
        }
    }
    let mut notes = Vec::new();
    for mode in Mode::BOTH {
        correct_button_text(&mut theme, mode, &mut notes);
        correct_accent_text(&mut theme, mode, &mut notes);
    }
    let rows = contrast_rows(&theme);
    let failures = rows.iter().filter(|r| !r.passes).cloned().collect();
    Checked {
        theme,
        notes,
        failures,
        rows,
    }
}

fn ratio(theme: &Theme, mode: Mode, fg: &str, bg: &str) -> f64 {
    let r = resolve(theme, mode);
    contrast(r.get(fg), r.get(bg))
}

fn correct_button_text(theme: &mut Theme, mode: Mode, notes: &mut Vec<String>) {
    if ratio(theme, mode, "accent-ink", "accent") >= MIN_CONTRAST {
        return;
    }
    let e = engine();
    let s = theme.seeds;
    let white = ColorValue::Hex([255, 255, 255]);
    let dark = ColorValue::Oklch {
        l: e.dark_accent_ink[0],
        c: e.dark_accent_ink[1],
        h: s.accent_hue,
    };
    let accent = resolve(theme, mode).get("accent");
    let best = [white, dark]
        .into_iter()
        .map(|c| (contrast(c.lab(), accent), c))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .expect("two candidates");
    if best.0 >= MIN_CONTRAST {
        theme
            .overrides_mut(mode)
            .insert("accent-ink".into(), best.1);
        notes.push(format!(
            "{} mode: button text was set to {} so it stays readable.",
            title(mode),
            if best.1 == white {
                "white"
            } else {
                "a dark colour"
            }
        ));
        return;
    }
    // Move the accent a little (darker in Light mode for white text, lighter in Dark mode).
    if theme.overrides(mode).contains_key("accent") {
        return;
    }
    let base_l = e.accent_lightness.get(mode) + s.accent_lightness_nudge;
    let step: f64 = match mode {
        Mode::Light => -1.0,
        Mode::Dark => 1.0,
    };
    let ink = match mode {
        Mode::Light => white,
        Mode::Dark => dark,
    };
    let mut delta = step;
    while delta.abs() <= MAX_ACCENT_CORRECTION {
        let l = (base_l + delta).clamp(0.0, 100.0);
        let candidate = ColorValue::Oklch {
            l,
            c: s.accent_chroma,
            h: s.accent_hue,
        };
        if contrast(ink.lab(), candidate.lab()) >= MIN_CONTRAST {
            let o = theme.overrides_mut(mode);
            o.insert("accent".into(), candidate);
            o.insert("accent-ink".into(), ink);
            notes.push(format!(
                "{} mode: the accent colour was made {} by {} so button text stays readable.",
                title(mode),
                if step < 0.0 { "darker" } else { "lighter" },
                if delta.abs() == 1.0 {
                    "1 point".to_string()
                } else {
                    format!("{} points", delta.abs())
                }
            ));
            return;
        }
        delta += step;
    }
}

fn correct_accent_text(theme: &mut Theme, mode: Mode, notes: &mut Vec<String>) {
    let fails = |t: &Theme| {
        ratio(t, mode, "accent-d", "accent-soft") < MIN_CONTRAST
            || ratio(t, mode, "accent-d", "panel") < MIN_CONTRAST
    };
    if !fails(theme) || theme.overrides(mode).contains_key("accent-d") {
        return;
    }
    let e = engine();
    let r = resolve(theme, mode);
    let (accent, ink) = (r.get("accent"), r.get("ink"));
    let mut p = e.accent_strong_mix / 100.0 - 0.04;
    while p >= MIN_STRONG_MIX - 1e-9 {
        let candidate = ColorValue::from_lab(accent.mix(p, ink));
        let mut trial = theme.clone();
        trial
            .overrides_mut(mode)
            .insert("accent-d".into(), candidate);
        if !fails(&trial) {
            *theme = trial;
            notes.push(format!(
                "{} mode: selected and accent text were darkened toward the text colour so they stay readable.",
                title(mode)
            ));
            return;
        }
        p -= 0.04;
    }
}

fn title(mode: Mode) -> &'static str {
    match mode {
        Mode::Light => "Light",
        Mode::Dark => "Dark",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_loads_and_starts_with_openframe_default() {
        let all = builtin_themes();
        assert_eq!(all[0].id, DEFAULT_THEME_ID);
        assert_eq!(all.len(), 16, "OpenFrame Default plus the fifteen themes");
    }

    /// Every built-in theme, OpenFrame Default included, passes every pair in both modes. OpenFrame
    /// Default had six Light-mode pairs below 4.5 to 1 until the owner decision of 2026-10-07
    /// (ADR-0014 amendment) raised them; no exception is allowed any more.
    #[test]
    fn contrast_every_builtin_theme_passes_in_light_and_dark() {
        for t in builtin_themes() {
            for r in contrast_rows(t) {
                assert!(
                    r.passes,
                    "{} {}: {} ({} on {}) is {:.2} to 1, below 4.5",
                    t.id,
                    r.mode.as_str(),
                    r.label,
                    r.foreground,
                    r.background,
                    r.ratio
                );
            }
            // Built-ins carry their corrections in themes.json: the check changes nothing else.
            let c = check(t.clone());
            assert!(c.notes.is_empty(), "{}: {:?}", t.id, c.notes);
        }
    }

    #[test]
    fn contrast_high_contrast_keeps_seven_to_one_for_main_and_muted_text() {
        let hc = builtin("high-contrast").unwrap();
        for mode in Mode::BOTH {
            let r = resolve(hc, mode);
            for (fg, bg) in [
                ("ink", "bg"),
                ("ink", "panel"),
                ("muted", "panel"),
                ("side-ink", "side"),
            ] {
                let ratio = contrast(r.get(fg), r.get(bg));
                assert!(
                    ratio >= 7.0,
                    "high contrast {}: {fg} on {bg} {ratio:.2}",
                    mode.as_str()
                );
            }
        }
    }

    #[test]
    fn the_fifteen_theme_ids_are_the_reference_list() {
        let ids: Vec<&str> = builtin_themes()
            .iter()
            .skip(1)
            .map(|t| t.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "studio-amber",
                "slate-noir",
                "midnight-reel",
                "teal-orange",
                "courier-cream",
                "cutting-room",
                "velvet-curtain",
                "sunset-boulevard",
                "arctic-frame",
                "neon-reel",
                "location-scout",
                "desert-dune",
                "colorist-neutral",
                "silver-screen",
                "high-contrast",
            ]
        );
        for t in builtin_themes() {
            assert!(
                valid_id(&t.id) && !t.name.is_empty() && !t.description.is_empty(),
                "{}",
                t.id
            );
        }
    }

    #[test]
    fn a_failing_import_is_corrected_only_within_the_limits() {
        // Midnight Reel's raw seeds: white button text is 4.43 to 1 in Light mode, so the accent is
        // made one point darker.
        let raw = r#"{"formatVersion":1,"id":"navy-test","name":"Navy","seeds":{"neutralHue":255,"neutralTint":0.03,"accentHue":238,"accentChroma":0.15,"accentLightnessNudge":0}}"#;
        let c = check(parse_theme(raw, Origin::Imported).unwrap());
        assert!(c.failures.is_empty());
        assert_eq!(c.notes.len(), 1, "{:?}", c.notes);
        assert!(c.notes[0].contains("darker by 1 point "), "{}", c.notes[0]);
        // Unreadable muted text cannot be corrected: the theme is refused for use.
        let bad = r##"{"formatVersion":1,"id":"grey-fog","name":"Fog","seeds":{"neutralHue":0,"neutralTint":0,"accentHue":250,"accentChroma":0.1,"accentLightnessNudge":0},"overrides":{"light":{"muted":"#bbbbbb"}}}"##;
        let c = check(parse_theme(bad, Origin::Imported).unwrap());
        assert!(
            c.failures.iter().any(|r| r.foreground == "muted"),
            "{:?}",
            c.failures
        );
    }

    #[test]
    fn the_webview_knows_the_same_importable_tokens() {
        assert_eq!(engine().importable_tokens, IMPORTABLE_TOKENS);
    }

    #[test]
    fn ids_follow_the_rule() {
        for ok in ["abc", "my-theme-2", "a-b", &"x".repeat(40)] {
            assert!(valid_id(ok), "{ok}");
        }
        for bad in [
            "ab",
            "",
            "My-Theme",
            "a_b",
            "a b",
            "a.b",
            "ä-theme",
            &"x".repeat(41),
            "a/b",
            "<x>",
        ] {
            assert!(!valid_id(bad), "{bad}");
        }
    }
}
