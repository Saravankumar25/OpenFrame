//! Fountain (https://fountain.io/syntax) import and export.
//!
//! Supported: title page (multi-line values), scene headings (INT/EXT/EST/I/E
//! and forced `.`), scene numbers `#12A#`, action (forced `!`), character cues
//! (forced `@`, extensions, dual dialogue `^`), parentheticals, dialogue (with
//! intentional blank lines "  "), transitions (`TO:` and forced `>`), centered
//! text `>…<`, lyrics `~`, notes `[[…]]` (standalone, multi-line and inline),
//! boneyard `/* … */`, sections `#`, synopses `=`, page breaks `===`.
//! Emphasis markers (`*`, `_`) are kept verbatim as part of the text.

use crate::heuristics::Confidence;
use crate::heuristics::{heading_prefix, is_all_caps, split_scene_number};
use crate::model::{Element, ElementKind, Scene, ScreenplayDoc, TitlePage, Warning};
use crate::{ImportOutcome, SourceFormat};
use openframe_domain::AppResult;

const TITLE_KEYS: &[&str] = &[
    "title",
    "credit",
    "author",
    "authors",
    "source",
    "draft date",
    "date",
    "contact",
    "notes",
    "copyright",
    "revision",
    "draft",
    "format",
];

fn remove_boneyard(src: &str, warnings: &mut Vec<Warning>) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => {
                warnings.push(Warning::info(
                    "unterminated_boneyard",
                    "A hidden “/* … */” section was never closed; the text after it was ignored.",
                ));
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Extract inline `[[notes]]` from a line. Returns the remaining text and the notes.
fn split_inline_notes(line: &str) -> (String, Vec<String>) {
    let mut text = String::new();
    let mut notes = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        match rest[start + 2..].find("]]") {
            Some(end) => {
                text.push_str(&rest[..start]);
                notes.push(rest[start + 2..start + 2 + end].trim().to_string());
                rest = &rest[start + 2 + end + 2..];
            }
            None => break,
        }
    }
    text.push_str(rest);
    let collapsed = if notes.is_empty() {
        text
    } else {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    (collapsed, notes)
}

fn parse_title_page(lines: &[&str]) -> (TitlePage, usize) {
    let mut tp = TitlePage::default();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let Some(first) = first else { return (tp, 0) };
    let key_of = |l: &str| -> Option<(String, String)> {
        let (k, v) = l.split_once(':')?;
        if l.starts_with(' ') || l.starts_with('\t') {
            return None;
        }
        let key = k.trim();
        if TITLE_KEYS.contains(&key.to_lowercase().as_str()) {
            Some((key.to_string(), v.trim().to_string()))
        } else {
            None
        }
    };
    if key_of(lines[first]).is_none() {
        return (tp, 0);
    }
    let mut i = first;
    let mut current: Option<(String, String)> = None;
    while i < lines.len() {
        let l = lines[i];
        if l.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = key_of(l) {
            if let Some((ck, cv)) = current.take() {
                tp.0.push((ck, cv));
            }
            current = Some((k, v));
        } else if let Some((_, cv)) = current.as_mut() {
            if !cv.is_empty() {
                cv.push('\n');
            }
            cv.push_str(l.trim());
        } else {
            break;
        }
        i += 1;
    }
    if let Some(c) = current {
        tp.0.push(c);
    }
    (tp, i)
}

struct P {
    doc: ScreenplayDoc,
}

impl P {
    fn scene(&mut self) -> &mut Scene {
        if self.doc.scenes.is_empty() {
            self.doc.scenes.push(Scene::default());
        }
        self.doc.scenes.last_mut().unwrap()
    }
    fn push(&mut self, e: Element) {
        self.scene().elements.push(e);
    }
    fn last_kind(&self) -> Option<ElementKind> {
        self.doc
            .scenes
            .last()
            .and_then(|s| s.elements.last())
            .map(|e| e.kind)
    }
    /// Mark the speech that precedes the current one as dual.
    fn mark_previous_speech_dual(&mut self) {
        let Some(scene) = self.doc.scenes.last_mut() else {
            return;
        };
        let n = scene.elements.len();
        // The current cue was just pushed at n-1; walk back over the previous speech.
        let mut k = n.saturating_sub(1);
        while k > 0 {
            k -= 1;
            let kind = scene.elements[k].kind;
            if !kind.is_speech() {
                break;
            }
            scene.elements[k].dual = true;
            if kind == ElementKind::Character {
                break;
            }
        }
    }
}

pub fn parse(src: &str) -> AppResult<ImportOutcome> {
    let mut warnings = Vec::new();
    let src = src
        .strip_prefix('\u{FEFF}')
        .unwrap_or(src)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let src = remove_boneyard(&src, &mut warnings);
    let all: Vec<&str> = src.split('\n').collect();
    let (title_page, body_start) = parse_title_page(&all);
    let lines = &all[body_start..];

    let mut p = P {
        doc: ScreenplayDoc {
            title_page,
            scenes: Vec::new(),
            warnings: Vec::new(),
        },
    };
    let mut i = 0;
    let mut in_speech = false;
    let n = lines.len();
    let blank = |l: &str| l.trim().is_empty();
    while i < n {
        let raw = lines[i].trim_end_matches(['\r']);
        let prev_blank = i == 0 || blank(lines[i - 1]);
        let next_blank = i + 1 >= n || blank(lines[i + 1]);

        if in_speech && raw.len() >= 2 && raw.trim().is_empty() && raw.starts_with("  ") {
            // Intentional blank line inside dialogue.
            if let Some(last) = p.doc.scenes.last_mut().and_then(|s| s.elements.last_mut())
                && last.kind == ElementKind::Dialogue
            {
                last.text.push('\n');
            }
            i += 1;
            continue;
        }
        if blank(raw) {
            in_speech = false;
            i += 1;
            continue;
        }
        let t = raw.trim();

        // Multi-line notes [[ ... ]]
        if t.starts_with("[[") && !t.contains("]]") {
            let mut note = t[2..].to_string();
            let mut j = i + 1;
            let mut closed = false;
            while j < n {
                let l = lines[j];
                if let Some(end) = l.find("]]") {
                    note.push('\n');
                    note.push_str(&l[..end]);
                    closed = true;
                    break;
                }
                note.push('\n');
                note.push_str(l);
                j += 1;
            }
            if !closed {
                p.doc.warnings.push(Warning::info(
                    "unterminated_note",
                    "A note “[[ …” was never closed; it was kept as a note.",
                ));
            }
            p.push(Element::new(ElementKind::Note, note.trim()));
            in_speech = false;
            i = j + 1;
            continue;
        }

        // Inline / standalone notes.
        let (text, notes) = split_inline_notes(t);
        let t = text.trim();
        if t.is_empty() {
            for note in notes {
                p.push(Element::new(ElementKind::Note, note));
            }
            i += 1;
            continue;
        }

        if in_speech {
            // Inside a speech block: parentheticals, dialogue and lyrics.
            if let Some(l) = t.strip_prefix('~') {
                push_lyric(&mut p, l.trim(), true);
            } else if t.starts_with('(') && t.ends_with(')') {
                p.push(Element::new(ElementKind::Parenthetical, t).dual(dual_of_current(&p)));
            } else if p.last_kind() == Some(ElementKind::Dialogue) {
                let last = p
                    .doc
                    .scenes
                    .last_mut()
                    .unwrap()
                    .elements
                    .last_mut()
                    .unwrap();
                last.text.push('\n');
                last.text.push_str(t);
            } else {
                p.push(Element::new(ElementKind::Dialogue, t).dual(dual_of_current(&p)));
            }
            for note in notes {
                p.push(Element::new(ElementKind::Note, note));
            }
            i += 1;
            continue;
        }

        let chars: Vec<char> = t.chars().collect();
        let second_alnum = chars.get(1).map(|c| c.is_alphanumeric()).unwrap_or(false);
        let is_page_break = t.len() >= 3 && t.chars().all(|c| c == '=');

        if is_page_break {
            p.push(Element::new(ElementKind::PageBreak, ""));
        } else if let Some(s) = t.strip_prefix('#') {
            p.push(Element::new(
                ElementKind::Section,
                s.trim_start_matches('#').trim(),
            ));
        } else if let Some(s) = t.strip_prefix('=') {
            p.push(Element::new(ElementKind::Synopsis, s.trim()));
        } else if t.starts_with('.') && !t.starts_with("..") && second_alnum {
            let (heading, number) = split_scene_number(&t[1..]);
            p.doc.scenes.push(Scene {
                heading,
                number,
                elements: Vec::new(),
            });
        } else if prev_blank && heading_prefix(t) {
            let (heading, number) = split_scene_number(t);
            p.doc.scenes.push(Scene {
                heading,
                number,
                elements: Vec::new(),
            });
        } else if let Some(s) = t.strip_prefix('!') {
            action_paragraph(&mut p, lines, &mut i, s, raw);
            continue;
        } else if t.starts_with('>') && t.ends_with('<') && t.len() >= 2 {
            p.push(Element::new(
                ElementKind::Centered,
                t[1..t.len() - 1].trim(),
            ));
        } else if let Some(s) = t.strip_prefix('>') {
            p.push(Element::new(ElementKind::Transition, s.trim()));
        } else if let Some(s) = t.strip_prefix('~') {
            push_lyric(&mut p, s.trim(), !prev_blank);
        } else if prev_blank && next_blank && is_all_caps(t) && t.ends_with("TO:") {
            p.push(Element::new(ElementKind::Transition, t));
        } else if let Some(s) = t.strip_prefix('@').filter(|_| !next_blank) {
            push_cue(&mut p, s.trim());
            in_speech = true;
        } else if prev_blank && !next_blank && is_fountain_cue(t) {
            push_cue(&mut p, t);
            in_speech = true;
        } else {
            action_paragraph(&mut p, lines, &mut i, t, raw);
            for note in notes {
                p.push(Element::new(ElementKind::Note, note));
            }
            continue;
        }
        for note in notes {
            p.push(Element::new(ElementKind::Note, note));
        }
        i += 1;
    }

    // Empty first scene with no heading (e.g. only notes removed) is dropped.
    p.doc
        .scenes
        .retain(|s| !(s.heading.is_empty() && s.elements.is_empty()));
    let mut doc = p.doc;
    doc.warnings.splice(0..0, warnings);
    if doc.scenes.is_empty() && doc.title_page.is_empty() {
        return Err(crate::not_a_screenplay(SourceFormat::Fountain));
    }
    for (idx, s) in doc.scenes.iter().enumerate() {
        if !s.heading.is_empty() && s.elements.is_empty() {
            doc.warnings.push(
                Warning::attention(
                    "empty_scene",
                    format!("“{}” has no content under its heading.", s.heading),
                )
                .at(idx),
            );
        }
    }
    Ok(ImportOutcome {
        doc,
        confidence: Confidence::high(),
        format: SourceFormat::Fountain,
    })
}

/// Fountain cue: the name is entirely upper case; an extension in parentheses
/// may be any case, but nothing may follow it.
fn is_fountain_cue(t: &str) -> bool {
    let t = t.trim().trim_end_matches('^').trim();
    match t.find('(') {
        Some(i) => t.ends_with(')') && is_all_caps(t[..i].trim()),
        None => is_all_caps(t),
    }
}

/// Consecutive `~` lines form one lyric element.
fn push_lyric(p: &mut P, text: &str, may_continue: bool) {
    if may_continue && p.last_kind() == Some(ElementKind::Lyric) {
        let last = p
            .doc
            .scenes
            .last_mut()
            .unwrap()
            .elements
            .last_mut()
            .unwrap();
        last.text.push('\n');
        last.text.push_str(text);
        return;
    }
    let dual = dual_of_current(p) && p.last_kind().map(|k| k.is_speech()).unwrap_or(false);
    p.push(Element::new(ElementKind::Lyric, text).dual(dual));
}

fn dual_of_current(p: &P) -> bool {
    p.doc
        .scenes
        .last()
        .and_then(|s| {
            s.elements
                .iter()
                .rev()
                .find(|e| e.kind == ElementKind::Character)
        })
        .map(|e| e.dual)
        .unwrap_or(false)
}

fn push_cue(p: &mut P, t: &str) {
    let dual = t.trim_end().ends_with('^');
    let name = t.trim_end().trim_end_matches('^').trim();
    p.push(Element::new(ElementKind::Character, name).dual(dual));
    if dual {
        p.mark_previous_speech_dual();
    }
}

/// Action paragraph: this line and following non-blank lines (full-line notes excepted).
fn action_paragraph(p: &mut P, lines: &[&str], i: &mut usize, first: &str, raw_first: &str) {
    // Keep leading indentation of the raw line for action (Fountain retains it).
    let lead: String = raw_first
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let mut text = if raw_first.trim_start().starts_with('!') {
        first.to_string()
    } else {
        format!("{lead}{first}")
    };
    let mut j = *i + 1;
    let mut trailing_notes = Vec::new();
    while j < lines.len() && !lines[j].trim().is_empty() {
        let l = lines[j].trim_end();
        let lt = l.trim();
        if lt.starts_with("[[") && lt.ends_with("]]") {
            trailing_notes.push(lt[2..lt.len() - 2].trim().to_string());
        } else {
            let (clean, notes) = split_inline_notes(l);
            trailing_notes.extend(notes);
            text.push('\n');
            text.push_str(clean.strip_prefix('!').unwrap_or(&clean));
        }
        j += 1;
    }
    p.push(Element::new(ElementKind::Action, text));
    for n in trailing_notes {
        p.push(Element::new(ElementKind::Note, n));
    }
    *i = j;
}

// ------------------------------------------------------------------- writer

fn needs_forced_action(text: &str) -> bool {
    let first = text.split('\n').next().unwrap_or("");
    let t = first.trim();
    let multi = text.contains('\n');
    t.is_empty()
        || heading_prefix(t)
        || (is_all_caps(t) && t.ends_with("TO:"))
        || (is_fountain_cue(t) && multi)
        || t.starts_with(['!', '@', '#', '~', '=', '.', '>', '['])
        || t.chars().all(|c| c == '=')
}

fn write_title_page(tp: &TitlePage, out: &mut String) {
    for (k, v) in &tp.0 {
        if v.contains('\n') {
            out.push_str(k);
            out.push_str(":\n");
            for l in v.split('\n') {
                out.push_str("    ");
                out.push_str(l.trim());
                out.push('\n');
            }
        } else {
            out.push_str(&format!("{k}: {}\n", v.trim()));
        }
    }
}

/// Write a Fountain document. Internal metadata is never inserted; notes are
/// written only when `include_notes` is set.
pub fn write(doc: &ScreenplayDoc, include_notes: bool, scene_numbers: bool) -> String {
    let mut out = String::new();
    if !doc.title_page.is_empty() {
        write_title_page(&doc.title_page, &mut out);
        out.push('\n');
    }
    let mut number = 0usize;
    for scene in &doc.scenes {
        if !scene.heading.trim().is_empty() {
            number += 1;
            let h = scene.heading.trim();
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            if !heading_prefix(h) {
                out.push('.');
            }
            out.push_str(h);
            let num = scene
                .number
                .clone()
                .or_else(|| scene_numbers.then(|| number.to_string()));
            if let Some(n) = num.filter(|n| !n.trim().is_empty()) {
                out.push_str(&format!(" #{}#", n.trim()));
            }
            out.push_str("\n\n");
        }
        let els: Vec<&Element> = scene
            .elements
            .iter()
            .filter(|e| {
                include_notes
                    || !matches!(
                        e.kind,
                        ElementKind::Note | ElementKind::Section | ElementKind::Synopsis
                    )
            })
            .collect();
        let mut cues_in_dual = 0usize;
        for (idx, e) in els.iter().copied().enumerate() {
            let prev = if idx > 0 { Some(els[idx - 1]) } else { None };
            let continues_speech = matches!(
                e.kind,
                ElementKind::Parenthetical | ElementKind::Dialogue | ElementKind::Lyric
            ) && prev.map(|p| p.kind.is_speech()).unwrap_or(false);
            if !continues_speech && !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            match e.kind {
                ElementKind::Action => {
                    if needs_forced_action(&e.text) {
                        out.push('!');
                    }
                    out.push_str(e.text.trim_end());
                    out.push('\n');
                }
                ElementKind::Character => {
                    if e.dual {
                        cues_in_dual =
                            if prev.map(|p| p.dual && p.kind.is_speech()).unwrap_or(false) {
                                cues_in_dual + 1
                            } else {
                                1
                            };
                    } else {
                        cues_in_dual = 0;
                    }
                    let name = e.text.trim();
                    if !is_fountain_cue(name) || heading_prefix(name) || name.ends_with("TO:") {
                        out.push('@');
                    }
                    out.push_str(name);
                    if e.dual && cues_in_dual == 2 {
                        out.push_str(" ^");
                    }
                    out.push('\n');
                }
                ElementKind::Parenthetical => {
                    let t = e.text.trim();
                    if t.starts_with('(') && t.ends_with(')') {
                        out.push_str(t);
                    } else {
                        out.push_str(&format!("({})", t.trim_matches(['(', ')'])));
                    }
                    out.push('\n');
                }
                ElementKind::Dialogue => {
                    let body = e.text.trim_end_matches('\n');
                    for l in body.split('\n') {
                        if l.trim().is_empty() {
                            out.push_str("  \n");
                        } else {
                            out.push_str(l.trim_end());
                            out.push('\n');
                        }
                    }
                }
                ElementKind::Lyric => {
                    for l in e.text.split('\n') {
                        out.push('~');
                        out.push_str(l.trim());
                        out.push('\n');
                    }
                }
                ElementKind::Transition => {
                    let t = e.text.trim();
                    if !(is_all_caps(t) && t.ends_with("TO:")) {
                        out.push('>');
                    }
                    out.push_str(t);
                    out.push('\n');
                }
                ElementKind::Shot => {
                    // Fountain has no shot element; shots are upper-case action lines.
                    out.push('!');
                    out.push_str(e.text.trim());
                    out.push('\n');
                }
                ElementKind::Centered => {
                    out.push_str(&format!("> {} <\n", e.text.trim()));
                }
                ElementKind::Note => out.push_str(&format!("[[{}]]\n", e.text.trim())),
                ElementKind::Section => out.push_str(&format!("# {}\n", e.text.trim())),
                ElementKind::Synopsis => out.push_str(&format!("= {}\n", e.text.trim())),
                ElementKind::PageBreak => out.push_str("===\n"),
            }
        }
    }
    let mut s = out.trim_end().to_string();
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_elements() {
        let src = "Title: Black Rain\nAuthor: Nisha Verma\n\nFADE IN:\n\nINT. POLICE STATION - NIGHT #1#\n\nArjun enters.\n\nMEERA\n(quietly)\nYou came back.\n\nARJUN ^\nI did.\n\nCUT TO:\n\n.RAILWAY PLATFORM\n\n> THE END <\n";
        let out = parse(src).unwrap();
        let d = out.doc;
        assert_eq!(d.title_page.title(), Some("Black Rain"));
        assert_eq!(d.scenes.len(), 3);
        assert_eq!(d.scenes[1].heading, "INT. POLICE STATION - NIGHT");
        assert_eq!(d.scenes[1].number.as_deref(), Some("1"));
        let kinds: Vec<_> = d.scenes[1]
            .elements
            .iter()
            .map(|e| (e.kind, e.dual))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (ElementKind::Action, false),
                (ElementKind::Character, true),
                (ElementKind::Parenthetical, true),
                (ElementKind::Dialogue, true),
                (ElementKind::Character, true),
                (ElementKind::Dialogue, true),
                (ElementKind::Transition, false),
            ]
        );
        assert_eq!(d.scenes[2].heading, "RAILWAY PLATFORM");
        assert_eq!(d.scenes[2].elements[0].kind, ElementKind::Centered);
    }

    #[test]
    fn boneyard_and_notes() {
        let src = "INT. A - DAY\n\nShe waits. [[fix this]]\n\n/* cut\nthis */\nHe leaves.\n";
        let d = parse(src).unwrap().doc;
        let s = &d.scenes[0];
        assert_eq!(s.elements[0].text, "She waits.");
        assert_eq!(s.elements[1].kind, ElementKind::Note);
        assert_eq!(s.elements[2].text, "He leaves.");
    }
}
