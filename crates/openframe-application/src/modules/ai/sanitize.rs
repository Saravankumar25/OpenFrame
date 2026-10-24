//! The last wash of model-written text (ADR-0028: the person never sees our own markup).
//!
//! A small model sometimes copies the prompt into its answer: our delimiter tags, the source labels
//! in front of each piece of evidence, the metadata lines of a briefing, or a sentence about the
//! reply itself ("here is the revised response"). `clean` removes exactly those and nothing else,
//! so a sentence that carries a fact stays. It runs on every model-written text before the
//! verifier sees it and before it is stored (`orchestrator::plain_text_with`).

use regex::Regex;
use std::sync::LazyLock;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid pattern")
}

/// Our delimiter tags with their attributes, opening or closing, even when cut off.
static TAG: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)</?\s*(?:project_data|observation|conversation_history|product_guide)\b[^>\n]*>?")
});
/// Source labels printed in front of evidence: "[Project map]", "[Briefing: Scene 4]",
/// "[Screenplay · Scene 2: INT. SHOP - DAY]", "[Act 2 of 3: The Chase]".
static BRACKET: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\[(?:Briefing:[^\]\n]*|Project map|Act \d+ of \d+[^\]\n]*|[^\]\n]*\u{00b7}[^\]\n]*)\]")
});
/// A briefing heading line: "## Scene 4 (Draft)", "## Project map (Draft)", "## Character: X (Draft)".
static HEADING_LINE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?m)^[ \t]*##[ \t]+(?:Scenes? \d+|Project map|Character:)[^\n]*\n?"));
/// The labels of a briefing line: "Scene 4: synopsis: ...". Only the label goes.
static SCENE_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"Scene \d+: (?:(?:heading|placement on the Beat Board|synopsis|writer's scene note|characters present|dialogue blocks per speaker|first line|last line|longest line of [^:\n]{1,60}|first action|last action|action|beat|key line, [^:\n]{1,60}): |(?:the scene before is|the scene after is) )",
    )
});
/// Labels whose value is evidence, and so goes with them; and labels that only end such a value.
static LABEL: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?P<drop>Characters:|Location:|Time:|Record:|Note:|Scene:|Scene \d+ \u{00b7} )|Story day:|Story:|Synopsis:|Role:|Played by:|Act:|Sequence:|Scene Card:|Beat:",
    )
});
static META: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:(?:okay|ok|sure|alright|certainly)[,.!]?\s+)?(?:here(?:'s|\u{2019}s| is| are)\s+(?:the|a|your)\s+(?:\w+\s+){0,2}?(?:revised|updated|corrected|new|rewritten|fixed|improved)\b|as requested,?\s+i(?:'ve|\u{2019}ve| have)\b|i(?:'ve|\u{2019}ve| have)\s+(?:removed|revised|rewritten|corrected)\b|removing the\s+[^.!?]*\b(?:response|answer|reply|version|records?|repetition|duplicates?|suggestion|feedback)\b)",
    )
});

/// Pieces of a line, each with the spaces that follow it, split after . ! ? and a space.
fn sentences(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    for i in 0..b.len() {
        if matches!(b[i], b'.' | b'!' | b'?') && b.get(i + 1).is_some_and(u8::is_ascii_whitespace) {
            let mut end = i + 1;
            while b.get(end).is_some_and(u8::is_ascii_whitespace) {
                end += 1;
            }
            out.push(&line[start..end]);
            start = end;
        }
    }
    if start < line.len() {
        out.push(&line[start..]);
    }
    out
}

/// A sentence that talks about the reply, not the story. "Here is the revised answer: Venkat..."
/// keeps what follows the colon.
fn without_meta(line: &str) -> String {
    let mut out = String::new();
    for s in sentences(line) {
        let t = s.trim_start();
        if META.is_match(t) {
            if let Some(i) = t.find(':') {
                out.push_str(t[i + 1..].trim_start());
            }
            continue;
        }
        out.push_str(s);
    }
    out
}

/// Remove "Characters: ..." and the like up to the next label or the end of the line.
fn without_labels(line: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    let mut dropping = false;
    for m in LABEL.captures_iter(line) {
        let whole = m.get(0).expect("whole match");
        let before = line[..whole.start()].chars().next_back();
        if before.is_some_and(|c| !c.is_whitespace() && c != '(') {
            continue;
        }
        if !dropping {
            out.push_str(&line[at..whole.start()]);
        }
        dropping = m.name("drop").is_some();
        at = if dropping { whole.end() } else { whole.start() };
    }
    if !dropping {
        out.push_str(&line[at..]);
    }
    out
}

/// A heading or label run twice in a row: "Scene 4: INT. KITCHEN - NIGHT Scene 4: INT. KITCHEN -
/// NIGHT" becomes one. Only runs of three words or more that hold a scene number or INT/EXT.
fn without_repeated_heading(line: &str) -> String {
    let w: Vec<&str> = line.split(' ').collect();
    let mut keep: Vec<&str> = Vec::with_capacity(w.len());
    let mut i = 0;
    'outer: while i < w.len() {
        for k in (3..=(w.len() - i) / 2).rev() {
            let run = &w[i..i + k];
            let heading = run
                .iter()
                .any(|x| matches!(*x, "INT." | "EXT." | "INT/EXT." | "I/E."))
                || (run[0] == "Scene" && run[1].trim_end_matches(':').parse::<u32>().is_ok());
            if heading && run == &w[i + k..i + 2 * k] {
                keep.extend_from_slice(run);
                i += 2 * k;
                continue 'outer;
            }
        }
        keep.push(w[i]);
        i += 1;
    }
    keep.join(" ")
}

/// Briefing heading lines ("## Scene 4 (Draft)"), removed before markdown marks are stripped.
pub(super) fn without_briefing_headings(text: &str) -> String {
    HEADING_LINE.replace_all(text, "").into_owned()
}

/// The model copied our markup or the labels of its notes (not merely a sentence about the reply).
/// `raw` is the reply as written, `unwashed` the same without markdown marks.
pub(super) fn copies_markup(raw: &str, unwashed: &str) -> bool {
    HEADING_LINE.is_match(raw)
        || unwashed.lines().any(|l| {
            let l = l.trim_start();
            TAG.is_match(l)
                || BRACKET.is_match(l)
                || SCENE_LABEL.is_match(l)
                || without_labels(l) != l
        })
}

/// The text without prompt markup, evidence labels and sentences about the reply. Line layout
/// stays; a line that only held removed material goes.
pub(super) fn clean(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let indent = &line[..line.len() - line.trim_start().len()];
        let body = TAG.replace_all(line.trim_start(), "");
        let body = BRACKET.replace_all(&body, "");
        let body = SCENE_LABEL.replace_all(&body, "");
        let body = without_labels(&body);
        let body = without_repeated_heading(&body);
        let body = without_meta(&body);
        let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
        // A bullet marker alone is not a line.
        if body.is_empty() || body == "-" {
            continue;
        }
        lines.push(format!("{indent}{body}"));
    }
    let mut out = lines.join("\n");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests;
