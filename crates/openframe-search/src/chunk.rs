//! Text packing helpers for domain-aware chunking (§14). The application picks
//! the policy per domain (scene-first, card, paragraph, section…); these
//! functions only pack natural units into bounded chunks without splitting a
//! unit unless it alone exceeds the bound.

/// Rough token estimate for budgeting (≈4 characters per token for English prose).
pub fn estimate_tokens(text: &str) -> u32 {
    (text.chars().count().div_ceil(4)).max(1) as u32
}

/// Truncate to at most `max` characters on a char boundary, adding an ellipsis.
pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Split one oversized unit at sentence boundaries, then hard-cut as a last resort.
fn split_long(unit: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut sentences = Vec::new();
    let mut start = 0;
    for (i, ch) in unit.char_indices() {
        if matches!(ch, '.' | '!' | '?' | '…') {
            let end = i + ch.len_utf8();
            sentences.push(&unit[start..end]);
            start = end;
        }
    }
    if start < unit.len() {
        sentences.push(&unit[start..]);
    }
    for s in sentences {
        let s_len = s.chars().count();
        if s_len > max {
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_string());
            }
            cur = String::new();
            let chars: Vec<char> = s.chars().collect();
            for piece in chars.chunks(max) {
                let p: String = piece.iter().collect();
                if !p.trim().is_empty() {
                    out.push(p.trim().to_string());
                }
            }
            continue;
        }
        if cur.chars().count() + s_len > max && !cur.trim().is_empty() {
            out.push(cur.trim().to_string());
            cur = String::new();
        }
        cur.push_str(s);
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Pack `units` (paragraphs, screenplay elements, sections…) in order into
/// chunks of at most `max_chars` characters, joined by `sep`.
pub fn pack_units<'a>(
    units: impl IntoIterator<Item = &'a str>,
    max_chars: usize,
    sep: &str,
) -> Vec<String> {
    let max = max_chars.max(64);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cur_len = 0usize;
    let sep_len = sep.chars().count();
    for unit in units {
        let unit = unit.trim();
        if unit.is_empty() {
            continue;
        }
        let len = unit.chars().count();
        if len > max {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
                cur_len = 0;
            }
            out.extend(split_long(unit, max));
            continue;
        }
        if cur_len > 0 && cur_len + sep_len + len > max {
            out.push(std::mem::take(&mut cur));
            cur_len = 0;
        }
        if cur_len > 0 {
            cur.push_str(sep);
            cur_len += sep_len;
        }
        cur.push_str(unit);
        cur_len += len;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Paragraph chunks (blank-line separated; single lines when a paragraph is too long).
pub fn paragraphs(text: &str, max_chars: usize) -> Vec<String> {
    let paras: Vec<&str> = text.split("\n\n").collect();
    let mut units: Vec<&str> = Vec::new();
    for p in paras {
        if p.chars().count() > max_chars {
            units.extend(p.lines());
        } else {
            units.push(p);
        }
    }
    pack_units(units, max_chars, "\n\n")
}

/// Line-unit chunks (screenplay elements, list-like bodies).
pub fn lines(text: &str, max_chars: usize) -> Vec<String> {
    pack_units(text.lines(), max_chars, "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_one_chunk() {
        assert_eq!(
            paragraphs("One.\n\nTwo.", 500),
            vec!["One.\n\nTwo.".to_string()]
        );
        assert!(paragraphs("   ", 500).is_empty());
    }

    #[test]
    fn units_are_never_split_unless_oversized() {
        let text = (0..10)
            .map(|i| format!("Line {i} of the scene."))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = lines(&text, 70);
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert!(c.chars().count() <= 70, "{c}");
            for l in c.lines() {
                assert!(l.starts_with("Line "), "unit split: {l}");
            }
        }
        assert_eq!(chunks.join("\n"), text);
    }

    #[test]
    fn oversized_units_split_on_sentences_then_hard_cut() {
        let long = "A short one. ".repeat(20);
        let chunks = paragraphs(&long, 64);
        assert!(chunks.iter().all(|c| c.chars().count() <= 64));
        let word = "x".repeat(200);
        let chunks = paragraphs(&word, 64);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks.concat(), word);
    }

    #[test]
    fn helpers() {
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(truncate_chars("héllo world", 5), "héll…");
        assert_eq!(truncate_chars("hi", 5), "hi");
    }
}
