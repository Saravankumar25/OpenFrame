//! Shared screenplay-recognition heuristics (scene headings, character cues,
//! transitions) and the indentation-aware line classifier used by the TXT,
//! pasted-text, PDF and DOCX importers.
//!
//! The classifier works on "raw lines" with an optional indentation measured
//! in inches relative to the action margin (standard screenplay layout: action
//! 0", dialogue 1.0", parenthetical 1.6", character 2.2", transitions far
//! right). TXT indentation in spaces is converted at 10 characters per inch
//! (Courier 12pt), PDF x positions at 72 points per inch, DOCX twips at 1440
//! per inch — so one classifier serves all three.

use std::sync::OnceLock;

use regex::Regex;

use crate::model::{Element, ElementKind, Scene, ScreenplayDoc, Warning};

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("static regex"))
}

/// Result of checking a line for a scene heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingMatch {
    No,
    /// Starts with INT./EXT./EST./INT./EXT./I/E.
    Strong,
    /// Upper-case line ending in a time of day but without INT./EXT.
    Uncertain,
}

/// Standard heading prefix (Fountain rules: followed by '.' or a space).
pub fn heading_prefix(line: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?i)^(int\.?/ext|ext\.?/int|i\.?/e|int|ext|est)[\. ]").is_match(line.trim())
}

pub fn heading_match(line: &str) -> HeadingMatch {
    static TOD: OnceLock<Regex> = OnceLock::new();
    let t = line.trim();
    if t.is_empty() || t.chars().count() > 120 {
        return HeadingMatch::No;
    }
    if heading_prefix(t) {
        return HeadingMatch::Strong;
    }
    let tod = re(
        &TOD,
        r"(?i)\s(-|–|—|--)\s*(day|night|morning|evening|afternoon|dawn|dusk|sunset|sunrise|later|moments later|continuous|same|same time|noon|midnight|magic hour)\.?$",
    );
    if is_all_caps(t) && tod.is_match(t) && !t.ends_with(':') {
        return HeadingMatch::Uncertain;
    }
    HeadingMatch::No
}

/// Split scene numbers printed around a heading: `12 INT. HOUSE - DAY 12`, `12. INT. …`, `INT. … #12A#`.
pub fn split_scene_number(heading: &str) -> (String, Option<String>) {
    static LEAD: OnceLock<Regex> = OnceLock::new();
    static TRAIL: OnceLock<Regex> = OnceLock::new();
    static FOUNTAIN: OnceLock<Regex> = OnceLock::new();
    let mut h = heading.trim().to_string();
    let mut number = None;
    if let Some(c) = re(&FOUNTAIN, r"\s*#([A-Za-z0-9.\-]+)#\s*$").captures(&h) {
        number = Some(c[1].to_string());
        h = h[..c.get(0).unwrap().start()].trim().to_string();
    }
    if let Some(c) = re(&LEAD, r"^(\d{1,4}[A-Z]{0,3})\.?\s+").captures(&h) {
        let rest = h[c.get(0).unwrap().end()..].to_string();
        if heading_match(&rest) != HeadingMatch::No {
            number.get_or_insert_with(|| c[1].to_string());
            h = rest.trim().to_string();
        }
    }
    if let Some(c) = re(&TRAIL, r"\s+\*?(\d{1,4}[A-Z]{0,3})\.?\*?$").captures(&h) {
        let rest = h[..c.get(0).unwrap().start()].to_string();
        if heading_match(&rest) != HeadingMatch::No {
            number.get_or_insert_with(|| c[1].to_string());
            h = rest.trim().to_string();
        }
    }
    (h, number)
}

/// Scene numbers around a heading that is *known* to be a heading (a styled
/// DOCX/FDX paragraph), so INT./EXT. is not required: `12  FLASHBACK  12`,
/// `12A\tMONTAGE`. A single space is not enough on the left ("1917 FLASHBACK"
/// stays intact) unless the rest is itself a recognisable heading.
pub fn split_styled_heading_number(heading: &str) -> (String, Option<String>) {
    static LEAD: OnceLock<Regex> = OnceLock::new();
    let (h, number) = split_scene_number(heading);
    if number.is_some() {
        return (h, number);
    }
    let Some(c) = re(&LEAD, r"^(\d{1,4}[A-Z]{0,3})\.?(?:\s{2,}|\t\s*)(\S.*)$").captures(&h) else {
        return (h, None);
    };
    let num = c[1].to_string();
    let mut rest = c[2].trim_end().to_string();
    // Matching number printed on the right margin (separated by a tab or a
    // run of spaces, so "ROOM 7" keeps its 7).
    if let Some(stripped) = rest.strip_suffix(num.as_str())
        && (stripped.ends_with("  ") || stripped.ends_with('\t'))
        && !stripped.trim().is_empty()
    {
        rest = stripped.trim_end().to_string();
    }
    (rest, Some(num))
}

/// Has at least one upper-case letter and no lower-case letters.
/// Scripts without letter case (e.g. Devanagari) are never "all caps".
pub fn is_all_caps(s: &str) -> bool {
    s.chars().any(|c| c.is_uppercase()) && !s.chars().any(|c| c.is_lowercase())
}

/// Upper-case transition line such as `CUT TO:` or `FADE OUT.`
pub fn is_transition(line: &str) -> bool {
    let t = line.trim();
    if !is_all_caps(t) || t.chars().count() > 40 {
        return false;
    }
    t.ends_with("TO:")
        || matches!(
            t,
            "FADE IN:"
                | "FADE OUT."
                | "FADE OUT"
                | "FADE TO BLACK."
                | "FADE TO BLACK"
                | "CUT TO BLACK."
                | "CUT TO BLACK"
                | "FADE OUT:"
                | "BACK TO SCENE:"
                | "INTERCUT:"
                | "END OF FLASHBACK."
                | "THE END"
        )
}

/// Character-cue shape: upper-case name (extensions in parentheses may be any case),
/// short, not a heading/transition, not ending in punctuation typical of action.
pub fn is_character_cue(line: &str) -> bool {
    let t = line.trim().trim_end_matches('^').trim();
    if t.is_empty() || t.chars().count() > 50 || t.starts_with('(') || t.starts_with('!') {
        return false;
    }
    let name_part = match t.find('(') {
        Some(i) => &t[..i],
        None => t,
    };
    let name = name_part.trim();
    if name.is_empty() || !is_all_caps(name) || name.split_whitespace().count() > 5 {
        return false;
    }
    if name.ends_with(':') || name.ends_with('!') || name.ends_with('?') || name.ends_with(',') {
        return false;
    }
    if name.ends_with('.')
        && !(name.ends_with("V.O.") || name.ends_with("O.S.") || name.ends_with("O.C."))
    {
        // "DR." is fine inside a name, but a line ending in a full stop is usually action.
        let last = name.split_whitespace().last().unwrap_or("");
        if last.len() > 3 {
            return false;
        }
    }
    heading_match(t) == HeadingMatch::No && !is_transition(t)
}

pub fn is_parenthetical(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('(') && t.ends_with(')') && t.len() >= 2
}

// ---------------------------------------------------------------- confidence

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Confidence {
    pub level: ConfidenceLevel,
    /// 0.0 – 1.0
    pub score: f32,
}

impl Confidence {
    pub fn from_score(score: f32) -> Self {
        let score = score.clamp(0.0, 1.0);
        let level = if score >= 0.8 {
            ConfidenceLevel::High
        } else if score >= 0.55 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        };
        Confidence { level, score }
    }
    pub fn high() -> Self {
        Confidence {
            level: ConfidenceLevel::High,
            score: 1.0,
        }
    }
    /// Never report more than `max` (used for PDF: presentation formats are never "certain").
    pub fn capped(self, max: ConfidenceLevel, max_score: f32) -> Self {
        if self.level > max {
            Confidence {
                level: max,
                score: self.score.min(max_score),
            }
        } else {
            self
        }
    }
}

/// Signals gathered while classifying, used for the confidence score.
#[derive(Debug, Clone, Default)]
pub struct ClassifyStats {
    pub strong_headings: usize,
    pub uncertain_headings: usize,
    pub cues: usize,
    pub lines: usize,
    /// Lines whose indentation fell in a recognised screenplay band.
    pub banded_lines: usize,
    pub used_indentation: bool,
}

impl ClassifyStats {
    pub fn confidence(&self) -> Confidence {
        let headings = self.strong_headings + self.uncertain_headings;
        if headings == 0 && self.cues == 0 {
            return Confidence::from_score(0.1);
        }
        let mut score: f32 = 0.35;
        if self.strong_headings > 0 {
            score += 0.3 * (self.strong_headings as f32 / headings.max(1) as f32);
        } else {
            score -= 0.1;
        }
        if self.cues > 0 {
            score += 0.2;
        }
        if self.used_indentation && self.lines > 0 {
            score += 0.15 * (self.banded_lines as f32 / self.lines as f32);
        } else {
            score += 0.1;
        }
        let c = Confidence::from_score(score);
        if self.uncertain_headings > 0 {
            c.capped(ConfidenceLevel::Medium, 0.79)
        } else {
            c
        }
    }
}

// ---------------------------------------------------------------- classifier

/// One physical line of source text.
#[derive(Debug, Clone, Default)]
pub struct RawLine {
    pub text: String,
    /// Indentation relative to the action margin, in inches (None = unknown).
    pub indent: Option<f32>,
    /// Blank line (vertical space) before this line.
    pub gap_before: bool,
    /// Start of a new page (PDF) — wrapped paragraphs never continue across pages
    /// without explicit (MORE)/(CONT'D) handling.
    pub new_page: bool,
    /// A known kind from an explicit style (DOCX paragraph style) — trusted.
    pub hint: Option<LineHint>,
    /// Part of a dual-dialogue block (only meaningful with a hint).
    pub dual: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineHint {
    Heading,
    Kind(ElementKind),
}

#[derive(Debug, Clone, Copy)]
pub struct ClassifyOptions {
    /// Join wrapped lines of one paragraph with a space (PDF, indented TXT) rather
    /// than keeping the line break (pasted text keeps the author's breaks).
    pub join_wrapped: bool,
    pub use_indentation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Band {
    Action,
    Dialogue,
    Paren,
    Character,
    Right,
}

fn band(indent: f32) -> Band {
    if indent < 0.5 {
        Band::Action
    } else if indent < 1.35 {
        Band::Dialogue
    } else if indent < 1.95 {
        Band::Paren
    } else if indent < 3.25 {
        Band::Character
    } else {
        Band::Right
    }
}

struct Builder {
    doc: ScreenplayDoc,
    stats: ClassifyStats,
    join_wrapped: bool,
    /// Kind of the paragraph currently open for continuation (wrapped lines).
    open: Option<ElementKind>,
    in_speech: bool,
}

impl Builder {
    fn scene(&mut self) -> &mut Scene {
        if self.doc.scenes.is_empty() {
            self.doc.scenes.push(Scene::default());
        }
        self.doc.scenes.last_mut().unwrap()
    }
    fn heading(&mut self, text: &str, uncertain: bool) {
        let (heading, number) = split_scene_number(text);
        let mut scene = Scene::new(heading.clone());
        scene.number = number;
        self.doc.scenes.push(scene);
        let idx = self.doc.scenes.len() - 1;
        if uncertain {
            self.stats.uncertain_headings += 1;
            self.doc.warnings.push(
                Warning::attention(
                    "uncertain_heading",
                    format!(
                        "“{heading}” was read as a scene heading, but it has no INT./EXT. prefix."
                    ),
                )
                .at(idx),
            );
        } else {
            self.stats.strong_headings += 1;
        }
        self.open = None;
        self.in_speech = false;
    }
    fn push(&mut self, kind: ElementKind, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if kind == ElementKind::Character {
            self.stats.cues += 1;
        }
        self.scene().elements.push(Element::new(kind, text));
        self.open = Some(kind);
        self.in_speech = kind.is_speech();
    }
    /// Continue the open paragraph of `kind` with a wrapped/next line.
    fn continue_or_push(&mut self, kind: ElementKind, text: &str) {
        let join = self.join_wrapped;
        if self.open == Some(kind)
            && matches!(
                kind,
                ElementKind::Action | ElementKind::Dialogue | ElementKind::Parenthetical
            )
            && let Some(last) = self.scene().elements.last_mut()
            && last.kind == kind
        {
            // A parenthetical that is already closed does not continue.
            if kind == ElementKind::Parenthetical && last.text.trim_end().ends_with(')') {
                self.push(kind, text);
                return;
            }
            last.text.push(if join { ' ' } else { '\n' });
            last.text.push_str(text.trim());
            return;
        }
        self.push(kind, text);
    }
}

/// Merge the dialogue continued across a page break: `(MORE)` at the foot of a
/// page and `NAME (CONT'D)` at the top of the next one.
fn is_more(text: &str) -> bool {
    matches!(text.trim(), "(MORE)" | "(more)" | "(MORE…)")
}

fn is_contd_cue(text: &str) -> bool {
    let t = text.trim().to_uppercase();
    t.ends_with("(CONT'D)")
        || t.ends_with("(CONT’D)")
        || t.ends_with("(CONTD)")
        || t.ends_with("(CONT)")
}

/// Classify raw lines into a screenplay document.
pub fn classify(lines: &[RawLine], opts: ClassifyOptions) -> (ScreenplayDoc, ClassifyStats) {
    let mut b = Builder {
        doc: ScreenplayDoc::default(),
        stats: ClassifyStats {
            used_indentation: opts.use_indentation,
            ..Default::default()
        },
        join_wrapped: opts.join_wrapped,
        open: None,
        in_speech: false,
    };
    let mut pending_more: Option<String> = None; // speaker whose dialogue continues on next page
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let text = line.text.trim_end();
        let t = text.trim();
        if t.is_empty() {
            b.open = None;
            b.in_speech = false;
            i += 1;
            continue;
        }
        b.stats.lines += 1;
        if line.gap_before {
            b.open = None;
            b.in_speech = false;
        }
        if line.new_page {
            // Wrapped paragraphs do not continue across pages unless marked.
            b.open = None;
        }

        // Explicit styles (DOCX) are trusted.
        if let Some(hint) = line.hint {
            b.stats.banded_lines += 1;
            match hint {
                LineHint::Heading => {
                    // Styled headings are trusted, so a printed scene number
                    // ("12  FLASHBACK  12") is removed even without INT./EXT.
                    let (heading, number) = split_styled_heading_number(t);
                    b.heading(&heading, false);
                    if let (Some(n), Some(s)) = (number, b.doc.scenes.last_mut()) {
                        s.number.get_or_insert(n);
                    }
                }
                LineHint::Kind(ElementKind::Character) => {
                    b.push(ElementKind::Character, t);
                }
                LineHint::Kind(
                    k @ (ElementKind::Dialogue | ElementKind::Parenthetical | ElementKind::Lyric),
                ) => {
                    if line.dual && !line.gap_before {
                        // Wrapped lines of one dual-dialogue column (paginated sources).
                        b.continue_or_push(k, t);
                    } else {
                        b.push(k, t);
                    }
                    b.in_speech = true;
                }
                LineHint::Kind(k) => b.push(k, t),
            }
            if line.dual
                && let Some(e) = b.doc.scenes.last_mut().and_then(|s| s.elements.last_mut())
            {
                e.dual = true;
            }
            i += 1;
            continue;
        }

        // Page furniture from paginated sources.
        if is_more(t) {
            if let Some(cue) = last_cue(&b.doc) {
                pending_more = Some(cue);
            }
            b.open = None;
            i += 1;
            continue;
        }
        if t.eq_ignore_ascii_case("CONTINUED:")
            || t.eq_ignore_ascii_case("(CONTINUED)")
            || t.eq_ignore_ascii_case("CONTINUED")
        {
            i += 1;
            continue;
        }

        let band_now = if opts.use_indentation {
            line.indent.map(band)
        } else {
            None
        };
        let hm = heading_match(t);

        // Continued speech after (MORE): "NAME (CONT'D)" re-opens the previous dialogue.
        if let Some(speaker) = pending_more.take()
            && is_contd_cue(t)
            && crate::model::character_name(t) == speaker
        {
            b.open = Some(ElementKind::Dialogue);
            b.in_speech = true;
            if matches!(band_now, Some(Band::Character) | None) {
                b.stats.banded_lines += 1;
            }
            // The next dialogue line continues the element that was split.
            if let Some(next) = lines.get(i + 1) {
                let nt = next.text.trim();
                if !nt.is_empty() && !is_parenthetical(nt) {
                    b.continue_or_push(ElementKind::Dialogue, nt);
                    i += 2;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        let fresh = line.gap_before || b.open.is_none();
        // A standalone upper-case line with a " - " separator at the action
        // margin (e.g. "FLASHBACK - RAILWAY PLATFORM") is probably a heading.
        let hm = if hm == HeadingMatch::No
            && fresh
            && matches!(band_now, None | Some(Band::Action))
            && is_all_caps(t)
            && (t.contains(" - ") || t.contains(" — ") || t.contains(" – "))
            && !is_transition(t)
            && lines
                .get(i + 1)
                .map(|n| n.gap_before || n.new_page)
                .unwrap_or(true)
        {
            HeadingMatch::Uncertain
        } else {
            hm
        };
        let kind: Option<ElementKind> = if b.in_speech && !line.gap_before && hm == HeadingMatch::No
        {
            // Lines directly under a cue are speech, whatever their indentation.
            let paren_open = b.open == Some(ElementKind::Parenthetical)
                && b.doc
                    .scenes
                    .last()
                    .and_then(|s| s.elements.last())
                    .map(|e| !e.text.trim_end().ends_with(')'))
                    .unwrap_or(false);
            if paren_open {
                b.continue_or_push(ElementKind::Parenthetical, t);
                Some(ElementKind::Parenthetical)
            } else if t.starts_with('(') {
                b.push(ElementKind::Parenthetical, t);
                Some(ElementKind::Parenthetical)
            } else {
                b.continue_or_push(ElementKind::Dialogue, t);
                Some(ElementKind::Dialogue)
            }
        } else if hm != HeadingMatch::No
            && (fresh || hm == HeadingMatch::Strong)
            && (hm == HeadingMatch::Strong || matches!(band_now, None | Some(Band::Action)))
        {
            b.heading(t, hm == HeadingMatch::Uncertain);
            None
        } else if is_transition(t) && fresh {
            b.push(ElementKind::Transition, t);
            b.open = None;
            Some(ElementKind::Transition)
        } else if fresh
            && next_is_speech(lines, i)
            && cue_allowed(band_now, lines, i)
            && (is_character_cue(t)
                || (band_now == Some(Band::Character) && !t.chars().any(|c| c.is_lowercase())))
        {
            b.push(ElementKind::Character, t);
            Some(ElementKind::Character)
        } else if band_now == Some(Band::Right) && fresh && is_all_caps(t) {
            b.push(ElementKind::Transition, t);
            b.open = None;
            Some(ElementKind::Transition)
        } else {
            // Indented text outside a speech stays action rather than a guess.
            b.continue_or_push(ElementKind::Action, t);
            Some(ElementKind::Action)
        };
        if let Some(bd) = band_now {
            let consistent = matches!(
                (bd, kind),
                (
                    Band::Action,
                    None | Some(ElementKind::Action | ElementKind::Transition | ElementKind::Shot)
                ) | (Band::Dialogue, Some(ElementKind::Dialogue))
                    | (
                        Band::Paren,
                        Some(ElementKind::Parenthetical | ElementKind::Dialogue)
                    )
                    | (Band::Character, Some(ElementKind::Character))
                    | (Band::Right, Some(ElementKind::Transition))
            );
            if consistent {
                b.stats.banded_lines += 1;
            }
        }
        i += 1;
    }

    // Warn about scenes with no content.
    for (idx, s) in b.doc.scenes.iter().enumerate() {
        if !s.heading.is_empty() && s.elements.is_empty() {
            b.doc.warnings.push(
                Warning::attention(
                    "empty_scene",
                    format!("“{}” has no content under its heading.", s.heading),
                )
                .at(idx),
            );
        }
    }
    if b.doc
        .scenes
        .first()
        .map(|s| s.heading.is_empty() && !s.elements.is_empty())
        .unwrap_or(false)
        && b.doc.scenes.len() > 1
    {
        let only_transitions = b.doc.scenes[0]
            .elements
            .iter()
            .all(|e| e.kind == ElementKind::Transition);
        if !only_transitions {
            b.doc.warnings.push(
                Warning::info("opening_without_heading", "Text before the first scene heading was kept as an opening section without a heading.").at(0),
            );
        }
    }
    (b.doc, b.stats)
}

/// A cue at the action margin is only believable when the speech under it is
/// indented further (otherwise it is an upper-case action line).
fn cue_allowed(band_now: Option<Band>, lines: &[RawLine], i: usize) -> bool {
    match band_now {
        None => true,
        Some(Band::Action) => {
            let here = lines[i].indent.unwrap_or(0.0);
            lines
                .get(i + 1)
                .and_then(|n| n.indent)
                .map(|n| n > here + 0.3)
                .unwrap_or(false)
        }
        Some(_) => true,
    }
}

fn last_cue(doc: &ScreenplayDoc) -> Option<String> {
    doc.scenes
        .last()?
        .elements
        .iter()
        .rev()
        .find(|e| e.kind == ElementKind::Character)
        .map(|e| crate::model::character_name(&e.text))
}

fn next_is_speech(lines: &[RawLine], i: usize) -> bool {
    match lines.get(i + 1) {
        Some(n) => {
            let t = n.text.trim();
            !t.is_empty() && !n.gap_before && heading_match(t) == HeadingMatch::No && !n.new_page
        }
        None => false,
    }
}

/// Approximate page count from content: ~55 lines per page with standard widths.
pub fn estimate_pages(doc: &ScreenplayDoc) -> u32 {
    fn wrapped(text: &str, width: usize) -> usize {
        text.split('\n')
            .map(|l| l.chars().count().max(1).div_ceil(width))
            .sum::<usize>()
            .max(1)
    }
    let mut lines = 0usize;
    for s in &doc.scenes {
        if !s.heading.is_empty() {
            lines += 2;
        }
        for e in &s.elements {
            lines += match e.kind {
                ElementKind::Action | ElementKind::Centered | ElementKind::Shot => {
                    1 + wrapped(&e.text, 60)
                }
                ElementKind::Character | ElementKind::Transition => 2,
                ElementKind::Dialogue | ElementKind::Lyric => wrapped(&e.text, 35),
                ElementKind::Parenthetical => wrapped(&e.text, 25),
                _ => 0,
            };
        }
    }
    ((lines as f32 / 55.0).ceil() as u32).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_detection() {
        assert_eq!(
            heading_match("INT. POLICE STATION — NIGHT"),
            HeadingMatch::Strong
        );
        assert_eq!(heading_match("ext. bus stop - night"), HeadingMatch::Strong);
        assert_eq!(heading_match("INT./EXT. CAR - DAY"), HeadingMatch::Strong);
        assert_eq!(heading_match("I/E CAR - DAY"), HeadingMatch::Strong);
        assert_eq!(heading_match("STREET — DAY"), HeadingMatch::Uncertain);
        assert_eq!(heading_match("INTERIOR DESIGN IS HARD"), HeadingMatch::No);
        assert_eq!(heading_match("She walks in."), HeadingMatch::No);
    }

    #[test]
    fn scene_numbers_are_split() {
        assert_eq!(
            split_scene_number("12 INT. HOUSE - DAY 12"),
            ("INT. HOUSE - DAY".to_string(), Some("12".to_string()))
        );
        assert_eq!(
            split_scene_number("INT. HOUSE - DAY #4A#"),
            ("INT. HOUSE - DAY".to_string(), Some("4A".to_string()))
        );
        assert_eq!(
            split_scene_number("INT. ROOM 101 - DAY"),
            ("INT. ROOM 101 - DAY".to_string(), None)
        );
    }

    #[test]
    fn styled_heading_numbers_are_split_without_prefix() {
        assert_eq!(
            split_styled_heading_number("12  FLASHBACK  12"),
            ("FLASHBACK".to_string(), Some("12".to_string()))
        );
        assert_eq!(
            split_styled_heading_number("4A\tMONTAGE"),
            ("MONTAGE".to_string(), Some("4A".to_string()))
        );
        assert_eq!(
            split_styled_heading_number("1917 FLASHBACK"),
            ("1917 FLASHBACK".to_string(), None)
        );
        assert_eq!(
            split_styled_heading_number("3 INT. HOUSE - DAY 3"),
            ("INT. HOUSE - DAY".to_string(), Some("3".to_string()))
        );
        assert_eq!(
            split_styled_heading_number("7  ROOM 7"),
            ("ROOM 7".to_string(), Some("7".to_string()))
        );
    }

    #[test]
    fn cues_and_transitions() {
        assert!(is_character_cue("MEERA"));
        assert!(is_character_cue("MEERA (V.O.)"));
        assert!(is_character_cue("DR. RAO"));
        assert!(!is_character_cue("She runs."));
        assert!(!is_character_cue("CUT TO:"));
        assert!(!is_character_cue("BANG!"));
        assert!(is_transition("SMASH CUT TO:"));
        assert!(!is_transition("Cut to the chase:"));
        assert!(!is_all_caps("मीरा"));
    }

    #[test]
    fn unindented_text_is_classified() {
        let src = "INT. POLICE STATION — NIGHT\n\nArjun enters carrying a pistol.\n\nMEERA\n(quietly)\nYou said you would never come back.\n\nCUT TO:\n";
        let lines: Vec<RawLine> = crate::text::raw_lines(src).0;
        let (doc, stats) = classify(
            &lines,
            ClassifyOptions {
                join_wrapped: false,
                use_indentation: false,
            },
        );
        assert_eq!(doc.scenes.len(), 1);
        let kinds: Vec<_> = doc.scenes[0].elements.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ElementKind::Action,
                ElementKind::Character,
                ElementKind::Parenthetical,
                ElementKind::Dialogue,
                ElementKind::Transition
            ]
        );
        assert_eq!(stats.strong_headings, 1);
        assert!(stats.confidence().level >= ConfidenceLevel::Medium);
    }
}
