use super::super::scope::{AiScopeKind, ResolvedScope};
use super::super::test_script::{project, sample};
use super::super::{chat, commands, queries, understand};
use super::*;

/// The lane of a request, decided the way the orchestrator decides it, on the Example Short sample
/// (ten scenes, Devi and Kiran, no acts).
fn lane_of(text: &str) -> Decision {
    lane_in(text, None)
}

/// The same with a scene in view (the scope is that scene).
fn lane_in(text: &str, in_view: Option<usize>) -> Decision {
    let c = project();
    let d = sample(&c, "example-short");
    let scene = in_view.map(|n| queries::scenes(&c, &d.id).unwrap()[n - 1].clone());
    let scope = ResolvedScope {
        kind: if scene.is_some() {
            AiScopeKind::CurrentScene
        } else {
            AiScopeKind::WholeProject
        },
        label: String::new(),
        draft: Some(d.clone()),
        scene,
        selection: Vec::new(),
        context: Vec::new(),
    };
    let u = understand::understand(&c, &d, &scope, text, &[]).unwrap();
    let unknown = chat::unknown_names(&c, &d, text, &u).unwrap();
    let resolved = !u.entities.is_empty() || !u.scenes.is_empty() || !u.acts.is_empty();
    let direct = commands::direct_route(text, in_view, |_| true);
    decide(&Inputs {
        text,
        direct: direct.as_ref(),
        understood: Some(&u),
        scene_scope: in_view.is_some(),
        project_scope: in_view.is_none(),
        unknown_name: unknown.first().filter(|_| !resolved).map(String::as_str),
    })
}

#[test]
fn twenty_five_requests_take_the_lane_the_plan_names() {
    let table: &[(&str, Lane)] = &[
        // Direct: a command or an exact fact that names its tool.
        ("Go to scene 3", Lane::Direct),
        ("How many night scenes are there?", Lane::Direct),
        ("Who speaks in this screenplay?", Lane::Direct),
        ("Summarize Scene 3", Lane::Direct),
        ("Describe Devi", Lane::Direct),
        // Action: a change verb.
        ("Rename scene 3 to The Second Knock", Lane::Action),
        ("Add a prop to scene 2", Lane::Action),
        ("Create a task to scout the footbridge", Lane::Action),
        ("Can you add Kiran to the cast list?", Lane::Action),
        ("Delete scene 9", Lane::Action),
        ("Please mark scene 4 as omitted", Lane::Action),
        // Recipe: the whole screenplay, as an explain, summary or theme question.
        ("Explain the whole screenplay", Lane::Recipe),
        ("Summarise the entire script for me", Lane::Recipe),
        ("What is the overall theme of the film?", Lane::Recipe),
        ("Walk me through the story from start to end", Lane::Recipe),
        // Chat: people, scenes, ideas, critique, and the questions code answers itself.
        ("Explain what happens in scene 4", Lane::Chat),
        ("Who is Devi and what does she do?", Lane::Chat),
        (
            "What is the relationship between Devi and Kiran?",
            Lane::Chat,
        ),
        ("Compare Devi and Kiran", Lane::Chat),
        ("Any ideas for scene 4?", Lane::Chat),
        ("Give me notes on scene 6", Lane::Chat),
        ("Who is Marcus and what does he do?", Lane::Chat),
        ("Explain scene 40", Lane::Chat),
        ("Why does Kiran keep coming back to the office?", Lane::Chat),
        // Agent: lists, finds, production records, anything unclear.
        ("Find the scenes set at night", Lane::Agent),
        ("What is the status of the budget?", Lane::Agent),
        ("Show the props for scene 4", Lane::Agent),
    ];
    for (text, want) in table {
        let got = lane_of(text);
        assert_eq!(got.lane, *want, "{text}: {got:?}");
    }
}

#[test]
fn requests_the_agent_has_always_answered_stay_with_the_agent() {
    // A verb of the request is not a name; a place alone is a search; a loose request about the
    // scene in view is for the scene tools; a count is a count even of something unknown.
    for text in [
        "Search everything",
        "Count the scenes for me, please.",
        "Question 3",
        "How many scenes are in Rewrite B?",
        "What happens at the lost property office?",
        "Break down scene 1",
    ] {
        assert_eq!(lane_of(text).lane, Lane::Agent, "{text}");
    }
    for text in ["Summarize", "Give me the gist of this scene"] {
        assert_eq!(
            lane_in(text, Some(2)).lane,
            Lane::Agent,
            "{text} in a scene"
        );
    }
    // The scene in view is enough for a question that is about the scene by nature.
    assert_eq!(lane_in("Any ideas for this?", Some(2)).lane, Lane::Chat);
    assert_eq!(lane_in("What is weak here?", Some(2)).lane, Lane::Chat);
    assert_eq!(lane_in("Explain this scene", Some(2)).lane, Lane::Chat);
}

#[test]
fn the_margin_is_one_for_a_hard_rule_and_half_when_the_question_matched_no_kind() {
    assert_eq!(lane_of("Rename scene 3 to The Second Knock").margin, 1.0);
    assert_eq!(lane_of("Explain what happens in scene 4").margin, 1.0);
    let other = lane_of("Why does Kiran keep coming back to the office?");
    assert_eq!((other.lane, other.margin), (Lane::Chat, 0.5));
    assert_eq!(other.runner_up, Lane::Agent);
    assert_eq!(
        lane_of("Explain the whole screenplay").runner_up,
        Lane::Chat
    );
}

#[test]
fn a_summary_route_steps_aside_only_when_the_person_asks_to_explain() {
    assert_eq!(lane_of("Summarize Scene 3").lane, Lane::Direct);
    assert_eq!(lane_of("Explain Scene 3").lane, Lane::Chat);
    assert_eq!(lane_of("Describe Devi").lane, Lane::Direct);
    assert_eq!(lane_of("Analyze Devi in detail").lane, Lane::Chat);
    // A fact route never steps aside.
    assert_eq!(lane_of("Who speaks in this screenplay?").lane, Lane::Direct);
}

#[test]
fn change_verbs_start_an_action_but_a_question_about_change_does_not() {
    for yes in [
        "Add a prop to scene 2",
        "Please rename Devi to Devika",
        "I want you to move scene 3 after scene 5",
        "Could you schedule scene 2 for Monday?",
        "Make a task to book the station",
        "Set the status of scene 4 to done",
    ] {
        assert!(is_action(yes), "{yes}");
    }
    for no in [
        "Make the opening more tense",
        "What changes in scene 4?",
        "Addressing the theme, what does Devi want?",
        "Who moves the kite?",
        "Does Kiran remove the tag?",
    ] {
        assert!(!is_action(no), "{no}");
    }
}

#[test]
fn a_project_without_a_screenplay_goes_to_the_agent() {
    let d = decide(&Inputs {
        text: "Explain the whole screenplay",
        direct: None,
        understood: None,
        scene_scope: false,
        project_scope: true,
        unknown_name: None,
    });
    assert_eq!(
        (d.lane, d.runner_up, d.margin),
        (Lane::Agent, Lane::Chat, 0.5)
    );
}
