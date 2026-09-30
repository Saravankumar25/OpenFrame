//! Format-neutral screenplay document model shared by every parser and writer.
//!
//! The model is deliberately close to OpenFrame's screenplay hub tables
//! (`screenplay_scene` / `screenplay_element`) but also carries a few
//! interchange-only constructs (centered text, lyrics, sections, synopses,
//! forced page breaks) so that Fountain/FDX round trips stay faithful. The
//! application layer decides how those map onto stored elements.

use serde::Serialize;

/// Kind of a screenplay element. Scene headings are not elements: they live on
/// [`Scene::heading`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Action,
    Character,
    Dialogue,
    Parenthetical,
    Transition,
    Shot,
    /// Writer's note (Fountain `[[ ]]`, FDX ScriptNote-like paragraphs). Never printed unless requested.
    Note,
    /// Centered action text (Fountain `>text<`).
    Centered,
    /// Lyrics (Fountain `~`).
    Lyric,
    /// Outline section (Fountain `#`). Not screenplay text.
    Section,
    /// Scene synopsis (Fountain `=`). Not screenplay text.
    Synopsis,
    /// Forced page break (Fountain `===`).
    PageBreak,
}

impl ElementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ElementKind::Action => "action",
            ElementKind::Character => "character",
            ElementKind::Dialogue => "dialogue",
            ElementKind::Parenthetical => "parenthetical",
            ElementKind::Transition => "transition",
            ElementKind::Shot => "shot",
            ElementKind::Note => "note",
            ElementKind::Centered => "centered",
            ElementKind::Lyric => "lyric",
            ElementKind::Section => "section",
            ElementKind::Synopsis => "synopsis",
            ElementKind::PageBreak => "page_break",
        }
    }

    /// Parse an OpenFrame stored element type (`screenplay_element.element_type`).
    pub fn from_stored(value: &str) -> Option<ElementKind> {
        Some(match value {
            "action" => ElementKind::Action,
            "character" => ElementKind::Character,
            "dialogue" => ElementKind::Dialogue,
            "parenthetical" => ElementKind::Parenthetical,
            "transition" => ElementKind::Transition,
            "shot" => ElementKind::Shot,
            "note" => ElementKind::Note,
            _ => return None,
        })
    }

    /// Part of a speech block (cue, parenthetical, dialogue, lyric).
    pub fn is_speech(self) -> bool {
        matches!(
            self,
            ElementKind::Character
                | ElementKind::Dialogue
                | ElementKind::Parenthetical
                | ElementKind::Lyric
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    pub kind: ElementKind,
    pub text: String,
    /// Part of a dual-dialogue block. Set on every element (cue, parenthetical,
    /// dialogue) of *both* speakers; a new `Character` element inside a dual run
    /// starts the second column.
    pub dual: bool,
    /// Revision mark label (e.g. "Blue"), printed as an asterisk when requested.
    pub revision: Option<String>,
}

impl Element {
    pub fn new(kind: ElementKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            dual: false,
            revision: None,
        }
    }
    pub fn dual(mut self, dual: bool) -> Self {
        self.dual = dual;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    /// Scene heading text. Empty for material that appears before the first heading.
    pub heading: String,
    /// Scene number carried by the source file (display only — never identity).
    pub number: Option<String>,
    pub elements: Vec<Element>,
}

impl Scene {
    pub fn new(heading: impl Into<String>) -> Self {
        Self {
            heading: heading.into(),
            number: None,
            elements: Vec::new(),
        }
    }
}

/// Title page as ordered key/value pairs (order is preserved for round trips).
/// Keys use Fountain's canonical names: Title, Credit, Author, Source,
/// Draft date, Contact, Notes, Copyright, Revision.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct TitlePage(pub Vec<(String, String)>);

impl TitlePage {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.trim().is_empty())
    }
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some(entry) = self.0.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
            entry.1 = value;
        } else {
            self.0.push((key.to_string(), value));
        }
    }
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|(_, v)| v.trim().is_empty())
    }
    pub fn title(&self) -> Option<&str> {
        self.get("Title")
    }
    /// Author: Fountain allows both "Author" and "Authors".
    pub fn author(&self) -> Option<&str> {
        self.get("Author").or_else(|| self.get("Authors"))
    }
}

/// Severity of an interchange warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningLevel {
    /// Needs the user's attention before/after import.
    Attention,
    /// Informational (something was omitted or normalized).
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    /// Stable code, e.g. `uncertain_heading`, `unsupported_fdx_element`.
    pub code: String,
    /// Filmmaker-facing message.
    pub message: String,
    pub level: WarningLevel,
    /// Index into [`ScreenplayDoc::scenes`] when the warning concerns one scene.
    pub scene: Option<usize>,
}

impl Warning {
    pub fn attention(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            level: WarningLevel::Attention,
            scene: None,
        }
    }
    pub fn info(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            level: WarningLevel::Info,
            scene: None,
        }
    }
    pub fn at(mut self, scene: usize) -> Self {
        self.scene = Some(scene);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDoc {
    pub title_page: TitlePage,
    pub scenes: Vec<Scene>,
    pub warnings: Vec<Warning>,
}

impl ScreenplayDoc {
    /// Scenes that carry a heading (material before the first heading is not a scene).
    pub fn headed_scene_count(&self) -> usize {
        self.scenes
            .iter()
            .filter(|s| !s.heading.trim().is_empty())
            .count()
    }

    pub fn element_count(&self) -> usize {
        self.scenes.iter().map(|s| s.elements.len()).sum()
    }

    /// Distinct character names from dialogue cues, in first-appearance order,
    /// with extensions such as `(V.O.)` / `(CONT'D)` removed.
    pub fn characters(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in &self.scenes {
            for e in &s.elements {
                if e.kind == ElementKind::Character {
                    let name = character_name(&e.text);
                    if !name.is_empty() && !out.iter().any(|n| n == &name) {
                        out.push(name);
                    }
                }
            }
        }
        out
    }

    /// Scenes and elements only (ignores warnings) — used by round-trip checks.
    pub fn same_content(&self, other: &ScreenplayDoc) -> bool {
        self.title_page == other.title_page && self.scenes == other.scenes
    }
}

/// Strip extensions like `(V.O.)`, `(O.S.)`, `(CONT'D)` and dual markers from a cue.
pub fn character_name(cue: &str) -> String {
    let mut s = cue.trim().trim_end_matches('^').trim().to_string();
    while let Some(open) = s.rfind('(') {
        if s.ends_with(')') {
            s.truncate(open);
            s = s.trim().to_string();
        } else {
            break;
        }
    }
    s.trim().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_names_drop_extensions() {
        assert_eq!(character_name("MEERA (V.O.)"), "MEERA");
        assert_eq!(character_name("ARJUN (O.S.) (CONT'D)"), "ARJUN");
        assert_eq!(character_name("DR. RAO ^"), "DR. RAO");
    }

    #[test]
    fn title_page_lookup_is_case_insensitive() {
        let mut t = TitlePage::default();
        t.set("Title", "Black Rain");
        t.set("title", "BLACK RAIN");
        assert_eq!(t.0.len(), 1);
        assert_eq!(t.title(), Some("BLACK RAIN"));
        assert!(t.author().is_none());
    }
}
