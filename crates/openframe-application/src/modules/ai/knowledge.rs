//! Versioned product knowledge (AI spec §3.1, AI-AC-023): curated with the
//! application, retrieved deterministically, never taken from model memory alone.

pub const GUIDE: &str = include_str!("knowledge.md");
pub const GUIDE_LABEL: &str = "OpenFrame product guide v1";

pub fn sections() -> Vec<(&'static str, &'static str)> {
    GUIDE
        .split("\n## ")
        .skip(1)
        .filter_map(|s| {
            let (title, body) = s.split_once('\n')?;
            Some((title.trim(), body.trim()))
        })
        .collect()
}

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .map(|w| w.trim_end_matches('s').to_string())
        .collect()
}

/// The `n` sections most relevant to `question` (keyword overlap; titles weigh more).
pub fn relevant(question: &str, n: usize) -> Vec<(&'static str, &'static str)> {
    let q = words(question);
    let mut scored: Vec<(usize, (&str, &str))> = sections()
        .into_iter()
        .map(|(t, b)| {
            let tw = words(t);
            let bw = words(b);
            let score: usize = q
                .iter()
                .map(|w| {
                    tw.iter().filter(|x| *x == w).count() * 3
                        + bw.iter().filter(|x| *x == w).count()
                })
                .sum();
            (score, (t, b))
        })
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    let mut out: Vec<(&str, &str)> = scored.into_iter().take(n).map(|(_, s)| s).collect();
    if out.is_empty() {
        // Always ground in the assistant-behaviour section rather than nothing.
        out.extend(
            sections()
                .into_iter()
                .filter(|(t, _)| t.starts_with("How the assistant")),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_has_sections_and_retrieves_by_topic() {
        assert!(sections().len() >= 10);
        let r = relevant("Why is my call sheet marked stale?", 2);
        assert!(r.iter().any(|(t, _)| t.contains("Call sheets")));
        let r = relevant("What is the Production Source?", 1);
        assert_eq!(r[0].0, "Production Source");
    }
}
