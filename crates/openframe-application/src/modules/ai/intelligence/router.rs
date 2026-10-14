//! Deterministic retrieval routing (§21). Not every question needs RAG:
//!
//! | question shape                         | route         |
//! |----------------------------------------|---------------|
//! | product / how-to help                  | ProductHelp   |
//! | exact count / list / status / assignee | Structured    |
//! | cross-module or relationship analysis  | HybridGraph   |
//! | meaning / theme / "scenes where…"      | Hybrid        |
//! | short known-entity lookup, quoted text | Lexical       |
//! | mutation request                       | Hybrid (retrieve required context, then the tools) |
//!
//! Safety never depends on the route: permissions, privacy and the proposal-only
//! mutation boundary are enforced regardless.

use crate::modules::ai::retrieval::RetrievalRoute;

fn words(q: &str) -> Vec<String> {
    q.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn starts_with_any(q: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| q.starts_with(p))
}

fn contains_any(q: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| q.contains(n))
}

/// Product areas; two or more distinct areas in one question → cross-module.
const AREAS: &[(&str, &[&str])] = &[
    (
        "screenplay",
        &[
            "scene",
            "scenes",
            "screenplay",
            "draft",
            "dialogue",
            "script",
        ],
    ),
    (
        "story",
        &[
            "story",
            "card",
            "cards",
            "beat",
            "beats",
            "act",
            "sequence",
            "character",
            "characters",
            "arc",
        ],
    ),
    (
        "production",
        &[
            "prop",
            "props",
            "costume",
            "costumes",
            "vehicle",
            "breakdown",
            "catalog",
            "location",
            "locations",
            "cast",
            "crew",
            "actor",
            "actors",
        ],
    ),
    (
        "schedule",
        &[
            "schedule",
            "shoot",
            "shooting",
            "day",
            "days",
            "call",
            "callsheet",
            "tomorrow",
        ],
    ),
    (
        "visual",
        &["shot", "shots", "storyboard", "moodboard", "panel", "frame"],
    ),
    (
        "notes",
        &[
            "note", "notes", "task", "tasks", "comment", "comments", "idea", "ideas", "vault",
        ],
    ),
];

pub fn route_for(query: &str) -> RetrievalRoute {
    let q = query.trim().to_lowercase();
    let w = words(&q);
    if w.is_empty() {
        return RetrievalRoute::Structured;
    }
    // Product help: how the application works, not what the project contains.
    let help_subject = contains_any(
        &q,
        &[
            "openframe",
            "offline ai",
            "shortcut",
            "keyboard",
            "export",
            "import",
            "package",
            "backup",
            "recently deleted",
            "undo",
            "production source",
            "lock a draft",
            "settings",
            "the app",
        ],
    );
    if (starts_with_any(
        &q,
        &[
            "how do i",
            "how can i",
            "how to",
            "where do i",
            "where can i",
            "can i ",
            "what does",
            "what is the difference",
            "help",
        ],
    ) && help_subject)
        || starts_with_any(&q, &["how do i", "how can i", "how to "])
    {
        return RetrievalRoute::ProductHelp;
    }
    // Mutations: retrieve the context the change needs; the tools do the rest.
    let mutation = starts_with_any(
        &q,
        &[
            "create ",
            "add ",
            "rename ",
            "move ",
            "delete ",
            "remove ",
            "schedule ",
            "assign ",
            "update ",
            "change ",
            "set ",
            "mark ",
            "prepare ",
            "make ",
            "link ",
            "reorder ",
        ],
    );
    if mutation {
        return RetrievalRoute::Hybrid;
    }
    // Exact facts come from canonical SQL tools.
    let structured = starts_with_any(
        &q,
        &[
            "how many",
            "count ",
            "number of",
            "list ",
            "list all",
            "which scenes aren't",
            "which scenes are not",
            "which scenes are unscheduled",
            "what is the status",
            "what's the status",
            "who is assigned",
            "who is the",
            "when is",
            "what day",
            "which day",
            "which shooting day",
            "what shooting day",
            "is scene",
            "open scene",
            "go to",
            "show me scene",
        ],
    ) || (w.len() <= 6 && contains_any(&q, &["unscheduled", "how many", "total"]));
    if structured {
        return RetrievalRoute::Structured;
    }
    // Relationship / cross-module analysis → graph expansion.
    let areas = AREAS
        .iter()
        .filter(|(_, kws)| w.iter().any(|x| kws.contains(&x.as_str())))
        .count();
    let relational = contains_any(
        &q,
        &[
            "related",
            "relationship",
            "connected",
            "connection",
            "involving",
            "between",
            "linked",
            "everything about",
            "what do we need",
            "prepare for",
            "requirements",
            "who appears",
            "where does",
            "who is in",
        ],
    );
    if areas >= 2 || (relational && areas >= 1) {
        return RetrievalRoute::HybridGraph;
    }
    // Semantic concept questions.
    let semantic = starts_with_any(
        &q,
        &[
            "where does",
            "where do",
            "why ",
            "when does",
            "find scenes where",
            "find moments",
            "show moments",
            "what happens",
        ],
    ) || contains_any(
        &q,
        &[
            "similar",
            "like the",
            "feel",
            "feeling",
            "emotion",
            "theme",
            "tone",
            "mood",
            "about ",
            "where ",
            "losing",
            "guilt",
            "trust",
            "conflict",
            "tension",
            "relationship",
        ],
    );
    if semantic {
        return RetrievalRoute::Hybrid;
    }
    // Short entity lookups and quoted phrases: exact terms win.
    if q.contains('"') || w.len() <= 4 || starts_with_any(&q, &["find ", "search ", "show "]) {
        return RetrievalRoute::Lexical;
    }
    RetrievalRoute::Hybrid
}

#[cfg(test)]
mod tests {
    use super::*;
    use RetrievalRoute::*;

    #[test]
    fn spec_examples_route_deterministically() {
        for (q, want) in [
            ("How many scenes are there?", Structured),
            ("Which shooting day contains Scene 42?", Structured),
            ("List unscheduled scenes.", Structured),
            ("Who is assigned as director?", Structured),
            ("How many locations exist?", Structured),
            ("Open Scene 12.", Structured),
            ("red car", Lexical),
            ("Find every scene mentioning the red car", Lexical),
            ("Where does Ravi begin losing trust in Anjali?", Hybrid),
            (
                "Where does Ravi's relationship with Anjali start breaking down in the story cards?",
                HybridGraph,
            ),
            ("Find scenes similar to the hospital confrontation.", Hybrid),
            ("Where is the protagonist acting out of guilt?", Hybrid),
            ("Show ideas related to isolation.", HybridGraph),
            (
                "Prepare tomorrow's shoot for the railway station scenes.",
                Hybrid,
            ),
            (
                "Which props and cast do the railway station scenes need on the shooting day?",
                HybridGraph,
            ),
            ("How do I lock a draft?", ProductHelp),
            ("How do I export a call sheet?", ProductHelp),
            ("Rename Ravi to Raghav throughout the project.", Hybrid),
            ("   ", Structured),
        ] {
            assert_eq!(route_for(q), want, "{q}");
        }
    }

    #[test]
    fn routing_is_stable() {
        let q = "Where does the relationship become hostile?";
        assert_eq!(route_for(q), route_for(q));
    }
}
