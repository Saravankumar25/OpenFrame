//! The last wash of model-written text (ADR-0028: the person never sees our own markup).
//!
//! A small model sometimes copies the prompt into its answer: our delimiter tags, or a sentence
//! about the reply itself ("here is the revised response"). `clean` removes exactly those and
//! nothing else, so a sentence that carries a fact stays. It runs on every model-written text before
//! the checker sees it and before it is stored (`text::plain_text`).

use regex::Regex;
use std::sync::LazyLock;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid pattern")
}

/// Our delimiter tags with their attributes, opening or closing, even when cut off.
static TAG: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)</?\s*(?:project_data|observation|conversation(?:_history)?|product_guide|facts|memory|focus)\b[^>\n]*>?",
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

/// The text without prompt markup and sentences about the reply. Line layout stays; a line that
/// only held removed material goes.
pub(super) fn clean(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let indent = &line[..line.len() - line.trim_start().len()];
        let body = TAG.replace_all(line.trim_start(), "");
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
