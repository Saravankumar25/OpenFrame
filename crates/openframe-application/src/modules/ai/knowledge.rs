//! Versioned product knowledge (AI spec §3.1, AI-AC-023): curated with the
//! application, retrieved deterministically, never taken from model memory alone.

pub const GUIDE: &str = include_str!("knowledge.md");
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

/// Words that say nothing about which section is meant.
const STOP: &[&str] = &[
    "does", "what", "this", "that", "with", "have", "from", "need", "needs", "will", "your", "when",
    "where", "which", "there", "about", "into", "they", "them", "then", "than", "would", "could",
    "should", "like", "once", "work", "thing", "things", "instead",
];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4 && !STOP.contains(w))
        .map(|w| w.trim_end_matches('s').to_string())
        .collect()
}

/// The `n` sections most relevant to `question` (keyword overlap; titles weigh more).
pub fn relevant(question: &str, n: usize) -> Vec<(&'static str, &'static str)> {
    let mut q = words(question);
    let asked = question.to_lowercase();
    // "Who can do this" asks about roles, whatever the verb is.
    if asked.contains("who can") {
        q.extend(["role".to_string(), "permission".to_string()]);
    }
    let all = sections();
    // A word few sections hold says more than one most of them hold.
    let parts: Vec<Vec<String>> = all
        .iter()
        .map(|(t, b)| words(&format!("{t} {b}")))
        .collect();
    let weight = |w: &String| -> usize {
        let held = parts.iter().filter(|p| p.contains(w)).count().max(1);
        1 + (usize::BITS - (parts.len() / held).leading_zeros()) as usize
    };
    let mut scored: Vec<(usize, (&str, &str))> = all
        .into_iter()
        .map(|(t, b)| {
            let tw = words(t);
            let bw = words(b);
            // The question names a term of the section by its words ("Day Out of Days", "Parking
            // Lot"). One-word parts such as "Screenplay" are too common to count.
            let named = t
                .to_lowercase()
                .split(" and ")
                .map(|p| p.trim().trim_start_matches("the ").to_string())
                .any(|p| p.contains(' ') && asked.contains(&p));
            let overlap: usize = q
                .iter()
                .map(|w| {
                    (tw.iter().filter(|x| *x == w).count() * 3
                        + bw.iter().filter(|x| *x == w).count())
                        * weight(w)
                })
                .sum();
            let score = overlap + usize::from(named) * 200;
            (score, (t, b))
        })
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    // Nothing matching means the guide does not cover the question. No unrelated section is
    // offered instead: that is how a Telugu question about a character was answered with the
    // guide's Production Source text (OpenFrameAI.docx exhibit E07).
    scored.into_iter().take(n).map(|(_, s)| s).collect()
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

    #[test]
    fn a_rare_word_decides_and_the_stop_words_do_not_count() {
        let r = relevant("does the AI need the internet", 2);
        assert!(
            r.iter().any(|(t, _)| *t == "Getting OpenFrame AI"
                || *t == "Saving and working offline"),
            "{r:?}"
        );
        let r = relevant("is my project sent anywhere", 2);
        assert!(r.iter().any(|(_, b)| b.contains("never sent")), "{r:?}");
        let r = relevant("Who can change things in a project?", 2);
        assert!(r.iter().any(|(t, _)| *t == "Roles and permissions"), "{r:?}");
    }

    #[test]
    fn product_terms_find_their_own_section() {
        for (question, title) in [
            ("What is a Change Set?", "Proposed Changes and approval"),
            (
                "What is the Parking Lot?",
                "Scene Cards and the Parking Lot",
            ),
            ("What is the Beat Board?", "Beat Board and Screenplay"),
            ("What are Sides?", "Sides and Day Out of Days"),
            ("What is Day Out of Days?", "Sides and Day Out of Days"),
            ("What does the Production Source mean?", "Production Source"),
            ("What is Quick Capture?", "Quick Capture"),
            (
                "How do I change the theme?",
                "Change the theme and dark mode",
            ),
            (
                "How do I switch to dark mode?",
                "Change the theme and dark mode",
            ),
            ("What is a Beat?", "Story Board, Outline and Beats"),
            (
                "What does the Story Timeline show?",
                "Characters and Story Timeline",
            ),
            ("What does Difference mean in the Budget?", "Budget"),
            ("How are shot letters like 12A made?", "Shot lists"),
            (
                "How do I link a file instead of copying it?",
                "Project Files",
            ),
            ("Which tabs does Settings have?", "Settings"),
        ] {
            assert_eq!(relevant(question, 1)[0].0, title, "{question}");
        }
    }
}
