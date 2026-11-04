//! The sections a case without `sections` is asked under (slice R2, spec
//! `ai-section-scoped-assistant` section 12.6). The 274-case set of the conversational harness was
//! written before the person picked sections; to keep it comparable, each of its cases is asked
//! under the leaves the old pipeline would have read for the same message: the topic words of the
//! question (`turn::domains`, rules only, no model) mapped to leaves by the table below. A message
//! that names no topic gets the two leaves the old "wider round" always reached first.
//!
//! The table is the whole mapping. It never looks at a case's expected answer.

use straft_application::modules::ai::turn::Domain;

/// What a topic of the old router reads, as leaves in the order they matter. A leaf is listed only
/// where the old fetcher for the topic read that screen's data.
pub const TOPIC_LEAVES: &[(Domain, &[&str])] = &[
    (Domain::Story, &["story/characters", "story/beat-board"]),
    (Domain::Screenplay, &["screenplay/scenes"]),
    (Domain::Breakdown, &["breakdown/breakdown"]),
    (Domain::People, &["production/cast-crew"]),
    (
        Domain::Production,
        &["production/locations", "notes-tasks/tasks"],
    ),
    (
        Domain::Schedule,
        &["production/schedule", "call-sheets/call-sheets"],
    ),
    (Domain::Budget, &["production/budget"]),
    (
        Domain::Visual,
        &[
            "production/shot-lists",
            "production/storyboards",
            "production/moodboards",
            "idea-vault/this-project",
        ],
    ),
    (Domain::Files, &["files/files", "notes-tasks/notes"]),
    (Domain::Settings, &["help/settings"]),
    (Domain::Product, &["help/product-guide"]),
];

/// Picked when the message names no topic.
pub const NO_TOPIC: &[&str] = &["screenplay/scenes", "story/characters"];

/// Writing cases ask for new text over the story, so they read the script, its people and its board.
pub const WRITING: &[&str] = &["screenplay/scenes", "story/characters", "story/beat-board"];

/// The poisoned-retrieval cases plant text in a scene, a note and a file: all three are picked.
pub const POISONED: &[&str] = &["screenplay/scenes", "notes-tasks/notes", "files/files"];

/// The most sections one question may read.
const CAP: usize = 3;

/// The leaves a legacy case is asked under: the first leaf of each topic, in the order the topics
/// were read, then the second leaves, up to three.
pub fn leaves_for(kind: &str, id: &str, domains: &[Domain]) -> Vec<&'static str> {
    if kind == "writing" {
        return WRITING.to_vec();
    }
    if id.starts_with("poison-") {
        return POISONED.to_vec();
    }
    let rows: Vec<&&[&str]> = domains
        .iter()
        .filter_map(|d| TOPIC_LEAVES.iter().find(|(t, _)| t == d).map(|(_, l)| l))
        .collect();
    if rows.is_empty() {
        return NO_TOPIC.to_vec();
    }
    let mut out: Vec<&'static str> = Vec::new();
    let deepest = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    'depth: for depth in 0..deepest {
        for row in &rows {
            if let Some(l) = row.get(depth)
                && !out.contains(l)
            {
                out.push(l);
                if out.len() == CAP {
                    break 'depth;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use straft_application::modules::ai::sections;

    #[test]
    fn every_leaf_in_the_table_exists() {
        for (_, leaves) in TOPIC_LEAVES {
            for l in *leaves {
                assert!(sections::leaf(l).is_some(), "{l}");
            }
        }
        for l in NO_TOPIC.iter().chain(WRITING).chain(POISONED) {
            assert!(sections::leaf(l).is_some(), "{l}");
        }
    }

    #[test]
    fn a_money_question_reads_the_budget_and_never_more_than_three_leaves() {
        assert_eq!(
            leaves_for("conversation", "conv-lt-money-001", &[Domain::Budget]),
            vec!["production/budget"]
        );
        let many = [
            Domain::Story,
            Domain::Screenplay,
            Domain::People,
            Domain::Schedule,
            Domain::Budget,
        ];
        let got = leaves_for("conversation", "x", &many);
        assert_eq!(got.len(), 3);
        // The first leaf of each of the first three topics, before any second leaf.
        assert_eq!(
            got,
            vec![
                "story/characters",
                "screenplay/scenes",
                "production/cast-crew"
            ]
        );
    }

    #[test]
    fn no_topic_reads_the_script_and_its_characters() {
        assert_eq!(
            leaves_for("conversation", "x", &[]),
            vec!["screenplay/scenes", "story/characters"]
        );
        assert_eq!(
            leaves_for("writing", "x", &[Domain::Budget]),
            WRITING.to_vec()
        );
        assert_eq!(
            leaves_for("safety", "poison-en-001", &[]),
            POISONED.to_vec()
        );
    }
}
