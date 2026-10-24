use super::*;

fn fixed(text: &str) -> String {
    correct(text, &[])
}

fn fixed_with(text: &str, names: &[&str]) -> String {
    correct(
        text,
        &names.iter().map(|n| (*n).to_string()).collect::<Vec<_>>(),
    )
}

#[test]
fn a_misspelt_count_question_reads_like_the_route_expects() {
    assert_eq!(
        fixed("how mny seens are there"),
        "how many scenes are there"
    );
    assert_eq!(
        fixed("How many sceens does the script have?"),
        "How many scenes does the script have?"
    );
}

#[test]
fn near_misses_of_one_keyword_are_fixed_in_each_shape() {
    // swapped letters, a missing letter, a doubled letter, a wrong letter
    assert_eq!(
        fixed("Whihc scenes mention the lantern"),
        "Which scenes mention the lantern"
    );
    assert_eq!(fixed("list the chracters"), "list the characters");
    assert_eq!(fixed("who speks in scene 4"), "who speaks in scene 4");
    assert_eq!(fixed("what is the scedule"), "what is the schedule");
    assert_eq!(fixed("find the locatoin list"), "find the location list");
    assert_eq!(fixed("show the dialouge"), "show the dialogue");
    assert_eq!(fixed("how many nigth scenes"), "how many night scenes");
}

#[test]
fn the_case_of_the_original_is_kept() {
    assert_eq!(fixed("Chracters in the film"), "Characters in the film");
    assert_eq!(fixed("SCENEE COUNT"), "SCENEE COUNT");
    assert_eq!(fixed("CHRACTERS"), "CHARACTERS");
}

#[test]
fn a_vocabulary_word_and_a_common_word_are_never_changed() {
    for t in [
        "how many scenes are there",
        "the witch speaks",
        "shoot the scene",
        "give me the gist of it",
        "where was it lost",
        "scene 12 and scene 4",
        "the 2nd draft of 3 scripts",
    ] {
        assert_eq!(fixed(t), t);
    }
}

#[test]
fn a_word_close_to_two_keywords_is_left_alone() {
    // "shote" is one edit from both "shot" and "shots".
    assert_eq!(fixed("a shote of tea"), "a shote of tea");
}

#[test]
fn quoted_text_and_other_scripts_are_left_alone() {
    assert_eq!(
        fixed("find \"seens\" and chracters"),
        "find \"seens\" and characters"
    );
    assert_eq!(
        fixed("Search for 'returned to owner' in chracters"),
        "Search for 'returned to owner' in characters"
    );
    assert_eq!(fixed("कितने सीन हैं"), "कितने सीन हैं");
    assert_eq!(
        fixed("scenes with Devi's chracters"),
        "scenes with Devi's characters"
    );
}

#[test]
fn short_words_are_not_guessed() {
    assert_eq!(fixed("hwo sceens"), "hwo scenes");
    assert_eq!(fixed("cat dog"), "cat dog");
}

#[test]
fn a_lower_case_near_miss_of_a_project_name_is_fixed_but_a_capitalised_one_is_not() {
    let names = ["kiran", "lantern", "junction"];
    assert_eq!(
        fixed_with("what does kirna say at the junciton", &names),
        "what does kiran say at the junction"
    );
    // "Kirna" may be a different person: left for the name check to report.
    assert_eq!(
        fixed_with("What does Kirna say", &names),
        "What does Kirna say"
    );
}

#[test]
fn text_without_a_near_miss_comes_back_unchanged() {
    for t in [
        "",
        "   ",
        "Open Scene 21.",
        "Add a task: rent a marine battery",
        "¿Cuántas escenas hay?",
    ] {
        assert_eq!(fixed(t), t);
    }
}

/// Every listed keyword is still a word the routes read. A keyword the routes dropped would
/// otherwise stay in the list and "correct" words toward nothing.
#[test]
fn every_keyword_appears_in_the_routing_code() {
    let sources = [
        include_str!("../commands.rs"),
        include_str!("../lanes.rs"),
        include_str!("../understand.rs"),
        include_str!("../toolbox.rs"),
        include_str!("../tools.rs"),
    ]
    .join("\n")
    .to_lowercase();
    let appears = |w: &str| {
        sources.match_indices(w).any(|(i, _)| {
            i == 0
                || !sources[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_alphabetic())
        })
    };
    for w in KEYWORDS {
        assert!(w.len() >= 4, "{w} is too short to be corrected toward");
        assert!(appears(w), "{w} appears in no route");
    }
}

#[test]
fn a_keyword_is_not_also_a_common_word() {
    for w in KEYWORDS {
        assert!(!COMMON.contains(w), "{w} is listed as both");
    }
}
