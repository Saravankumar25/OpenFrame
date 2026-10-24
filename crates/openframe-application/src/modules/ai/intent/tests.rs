use openframe_search::Embedder;
use openframe_test_support::embedder::BandEmbedder;

use super::super::understand::{Depth, Entity, Intent as Kind};
use super::*;

fn name_of(m: &Scored) -> &'static str {
    &m.intent().name
}

// ------------------------------------------------------------------ the data file

#[test]
fn the_intent_file_is_complete() {
    let list = intents();
    assert!(
        (25..=40).contains(&list.len()),
        "about thirty intents, found {}",
        list.len()
    );
    let mut names = HashSet::new();
    for i in list {
        assert!(names.insert(i.name.as_str()), "{} is listed twice", i.name);
        assert!(
            (6..=10).contains(&i.examples.len()),
            "{}: {} English examples",
            i.name,
            i.examples.len()
        );
        for l in LANGS {
            assert_eq!(
                i.other.get(l).map_or(0, Vec::len),
                2,
                "{}: two examples in {l}",
                i.name
            );
        }
        assert_eq!(
            i.other.len(),
            LANGS.len(),
            "{}: only the supported languages",
            i.name
        );
        assert!(
            i.examples
                .iter()
                .chain(i.other.values().flatten())
                .all(|t| !t.trim().is_empty() && t.chars().count() <= 90)
        );
    }
    assert!(list.iter().any(|i| i.target == Target::Chat));
    assert!(list.iter().any(|i| i.target == Target::Recipe));
}

#[test]
fn every_tool_exists_in_the_toolbox_and_every_fact_is_one_it_accepts() {
    let known = super::super::toolbox::all_tool_names();
    for i in intents() {
        if let Target::Tool(t) = &i.target {
            assert!(known.contains(&t.as_str()), "{}: no tool {t}", i.name);
        }
    }
    for i in intents().iter().filter(|i| i.fact.is_some()) {
        let Target::Tool(t) = &i.target else {
            panic!("{} has a fact but no tool", i.name)
        };
        let s = (super::super::tools::spec(t).unwrap().schema)();
        let allowed = s["properties"]["fact"]["enum"].as_array().unwrap();
        assert!(
            allowed.contains(&json!(i.fact.clone().unwrap())),
            "{}: fact {:?} is not accepted by {t}",
            i.name,
            i.fact
        );
    }
}

#[test]
fn no_example_is_a_request_of_the_gold_tool_set() {
    // Only the open gold set: the held-out paraphrase and conversation files are never read here.
    let gold = include_str!("../../../../../openframe-ai-eval/gold/tools.en.jsonl");
    let asked: HashSet<String> = gold
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| v["request"].as_str().map(|r| r.trim().to_lowercase()))
        .collect();
    assert!(asked.len() > 100);
    for i in intents() {
        for t in i.examples.iter().chain(i.other.values().flatten()) {
            assert!(
                !asked.contains(&t.trim().to_lowercase()),
                "{}: {t:?} is a gold request",
                i.name
            );
        }
    }
}

// ----------------------------------------------------------------------- scoring

/// A question that names everything any intent needs.
const ANY: Have = Have {
    scenes: 9,
    characters: 9,
};

fn best(e: &BandEmbedder, text: &str) -> Scored {
    classify(e, text, ANY).unwrap_or_else(|| panic!("no score for {text}"))
}

/// Rewordings written for this test (none is an example) that the stand-in reads the way a search
/// model would. Each must be routed to its intent with the score and the lead the gates ask for.
const ROUTED: &[(&str, &str)] = &[
    ("How many scenes make up my screenplay?", "count_scenes"),
    (
        "Which character has the most dialogue overall?",
        "dialogue_most",
    ),
    ("What is the location of scene 21?", "scene_setting"),
    ("Is scene 9 a night scene or a day scene?", "scene_time"),
    ("What are all the tasks that remain?", "project_tasks"),
    (
        "How many places are used in the script?",
        "location_statistics",
    ),
    ("What themes does the film have?", "theme"),
    (
        "What is the relationship between Ravi and Meena?",
        "relationship",
    ),
    ("Describe the character of Ravi", "describe_character"),
    ("Summarise the whole film for me", "whole_screenplay"),
    ("Give honest feedback on act two", "critique"),
    ("Show me the shooting schedule", "schedule_overview"),
    ("Who are the crew members?", "cast_and_crew"),
    ("What drafts are there?", "list_drafts"),
    (
        "How many acts and beats are on the beat board?",
        "story_statistics",
    ),
    (
        "How many cast and crew members are on record?",
        "production_statistics",
    ),
];

#[test]
fn rewordings_are_routed_and_chosen_for_a_clear_lead() {
    let e = BandEmbedder::new("rewordings");
    let mut wrong = Vec::new();
    for (text, want) in ROUTED {
        let m = best(&e, text);
        if name_of(&m) != *want || !m.accepted() {
            wrong.push(format!(
                "{text:?}: got {} score {:.3} margin {:.3}, wanted {want}",
                name_of(&m),
                m.score,
                m.margin
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Rewordings the stand-in cannot tell apart from a neighbour (it reads words, not meaning). The
/// gates must never turn one of them into the WRONG intent: each is left to the rules, or routed
/// right. A real search model is measured on the bench, not here.
#[test]
fn a_hard_reworded_question_is_never_routed_to_the_wrong_intent() {
    let e = BandEmbedder::new("hard");
    let two = Have {
        scenes: 0,
        characters: 2,
    };
    let scene = Have {
        scenes: 1,
        characters: 0,
    };
    let nothing = Have {
        scenes: 0,
        characters: 0,
    };
    for (text, want, have) in [
        ("Who talks in scene 17?", "scene_speakers", scene),
        (
            "Find the scenes in which Ravi and Meena talk",
            "scenes_with_characters",
            two,
        ),
        (
            "Which scenes mention the kettle?",
            "find_scenes_mentioning",
            nothing,
        ),
        ("List the people who speak", "list_characters", nothing),
        ("Give me ideas for a better ending", "ideas", nothing),
        ("How do I export a PDF?", "product_help", nothing),
        ("How far is the breakdown?", "breakdown_status", nothing),
        ("Go to scene 25", "open_scene", scene),
        ("Overview of the project", "project_overview", nothing),
    ] {
        let m = classify(&e, text, have).unwrap();
        assert!(
            !m.accepted() || name_of(&m) == want,
            "{text:?} was routed to {} ({:.3} / {:.3}), wanted {want} or nothing",
            name_of(&m),
            m.score,
            m.margin
        );
    }
}

/// An intent about a scene or a person is a candidate only when the question names one.
#[test]
fn an_intent_that_needs_a_scene_or_a_name_is_not_a_candidate_without_one() {
    let e = BandEmbedder::new("needs");
    let nothing = Have {
        scenes: 0,
        characters: 0,
    };
    let needing: HashSet<&str> = [
        "scene_speakers",
        "scene_setting",
        "scene_time",
        "open_scene",
        "suggest_breakdown",
        "dialogue_scenes",
        "scenes_with_characters",
        "describe_character",
        "relationship",
    ]
    .into();
    for i in intents() {
        assert_eq!(
            i.needs.is_some(),
            needing.contains(i.name.as_str()),
            "{}",
            i.name
        );
    }
    // Every question below is closest to a scene intent in words, and nothing names a scene.
    for text in [
        "Who talks in scene 17?",
        "What is the location of scene 21?",
        "Is scene 9 a night scene or a day scene?",
        "Go to scene 25",
    ] {
        let m = classify(&e, text, nothing).unwrap();
        assert!(
            !needing.contains(name_of(&m)),
            "{text:?} became {} with no scene named",
            name_of(&m)
        );
    }
    let with_scene = classify(
        &e,
        "What is the location of scene 21?",
        Have {
            scenes: 1,
            characters: 0,
        },
    )
    .unwrap();
    assert_eq!(name_of(&with_scene), "scene_setting");
    // One name is not enough for a pair, and two are not needed for one person.
    let one = Have {
        scenes: 0,
        characters: 1,
    };
    let pair = classify(&e, "What is the relationship between Ravi and Meena?", one).unwrap();
    assert_ne!(name_of(&pair), "relationship");
    let person = classify(&e, "Describe the character of Ravi", one).unwrap();
    assert_eq!(name_of(&person), "describe_character");
}

/// Texts that belong to no intent score in the band of unrelated texts and stay with the rules.
#[test]
fn unrelated_and_empty_texts_are_not_routed() {
    let e = BandEmbedder::new("unrelated");
    for text in [
        "What is the weather like today?",
        "Write me a poem about the sea",
        "Tell me a joke",
        "asdf qwer zxcv",
        "Who won the football match?",
        "hello",
        "",
    ] {
        let Some(m) = classify(&e, text, ANY) else {
            continue;
        };
        assert!(
            !m.accepted(),
            "{text:?} was routed to {} at {:.3} / {:.3}",
            name_of(&m),
            m.score,
            m.margin
        );
    }
}

#[test]
fn a_question_between_two_readings_is_left_to_the_rules() {
    let list = intents();
    let at = |name: &str| list.iter().position(|i| i.name == name).unwrap();
    let (a, b) = (at("count_scenes"), at("count_characters"));
    // The question sits halfway between a count_scenes example and a count_characters example.
    let half = unit(vec![1.0, 0.98, 0.0]);
    let protos = vec![(a, vec![1.0, 0.0, 0.0]), (b, vec![0.0, 1.0, 0.0])];
    let m = rank(&half, &protos, ANY).unwrap();
    assert!(m.margin < MARGIN_MIN, "margin {}", m.margin);
    assert!(!m.accepted());
}

#[test]
fn the_margin_ignores_intents_that_ask_for_the_same_action() {
    let list = intents();
    let at = |name: &str| list.iter().position(|i| i.name == name).unwrap();
    let (explain, ideas, count) = (at("explain_scene"), at("ideas"), at("count_scenes"));
    // Both chat intents are almost as close as each other; the nearest other action is far away.
    let protos = vec![
        (explain, unit(vec![1.0, 0.0, 0.0])),
        (ideas, unit(vec![1.0, 0.05, 0.0])),
        (count, unit(vec![0.4, 0.0, 1.0])),
    ];
    let m = rank(&unit(vec![1.0, 0.0, 0.0]), &protos, ANY).unwrap();
    assert_eq!(m.intent().target, Target::Chat);
    assert!(m.margin > 0.5, "margin {}", m.margin);
}

#[test]
fn a_recipe_needs_a_clearer_match_than_a_tool() {
    let list = intents();
    let at = |name: &str| list.iter().position(|i| i.name == name).unwrap();
    let just_enough = SCORE_MIN + 0.01;
    let recipe = Scored {
        index: at("whole_screenplay"),
        score: just_enough,
        margin: 0.2,
        second: None,
    };
    let tool = Scored {
        index: at("count_scenes"),
        score: just_enough,
        margin: 0.2,
        second: None,
    };
    assert!(tool.accepted());
    assert!(!recipe.accepted());
    assert!(
        Scored {
            score: SCORE_MIN + RECIPE_EXTRA,
            ..recipe
        }
        .accepted()
    );
}

#[test]
fn the_gates_are_direct_top_two_or_none() {
    let list = intents();
    let at = |name: &str| list.iter().position(|i| i.name == name).unwrap();
    let with = |name: &str, score: f32, margin: f32| Scored {
        index: at(name),
        score,
        margin,
        second: None,
    };
    assert_eq!(SCORE_MIN, 0.90);
    assert_eq!(MARGIN_MIN, 0.03);
    assert_eq!(with("count_scenes", 0.90, 0.03).gate(), Gate::Direct);
    // Below the direct score, from 0.88 up: the two closest tools.
    assert_eq!(with("count_scenes", 0.899, 0.2).gate(), Gate::TopTwo);
    assert_eq!(with("count_scenes", 0.88, 0.2).gate(), Gate::TopTwo);
    // A strong score without a lead is the same band.
    assert_eq!(with("count_scenes", 0.95, 0.029).gate(), Gate::TopTwo);
    assert_eq!(with("count_scenes", 0.879, 0.5).gate(), Gate::None);
    // A recipe keeps its extra margin on the direct gate.
    assert_eq!(
        with("whole_screenplay", SCORE_MIN + 0.01, 0.2).gate(),
        Gate::TopTwo
    );
    assert_eq!(
        with("whole_screenplay", SCORE_MIN + RECIPE_EXTRA, 0.2).gate(),
        Gate::Direct
    );
}

#[test]
fn rank_keeps_the_runner_up_that_asks_for_another_action() {
    let list = intents();
    let at = |name: &str| list.iter().position(|i| i.name == name).unwrap();
    let (a, b, c) = (
        at("scene_speakers"),
        at("scene_setting"),
        at("count_scenes"),
    );
    // scene_speakers and scene_setting share the tool but not the fact: they are different actions.
    let protos = vec![
        (a, unit(vec![1.0, 0.0, 0.0])),
        (b, unit(vec![1.0, 0.3, 0.0])),
        (c, unit(vec![0.0, 0.0, 1.0])),
    ];
    let m = rank(&unit(vec![1.0, 0.0, 0.0]), &protos, ANY).unwrap();
    assert_eq!((m.index, m.second), (a, Some(b)));
}

/// Night, day, interior and exterior counts are about locations and time, not the scene total, and
/// "who speaks in scene N" is the speakers fact, in whatever words.
#[test]
fn location_counts_and_scene_speakers_are_not_taken_for_their_neighbours() {
    let e = BandEmbedder::new("neighbours");
    let scene = Have {
        scenes: 1,
        characters: 0,
    };
    for text in [
        "How many scenes happen at night?",
        "How many night scenes are there?",
        "Count the interior scenes",
        "What number of scenes take place at night?",
    ] {
        let m = classify(&e, text, ANY).unwrap();
        assert_ne!(
            (m.gate(), name_of(&m)),
            (Gate::Direct, "count_scenes"),
            "{text:?} must not be a direct scene total"
        );
        assert_eq!(name_of(&m), "location_statistics", "{text:?}");
    }
    for text in [
        "Which characters speak in scene 5?",
        "Which characters speak in scene 11?",
        "Who has lines in scene 8?",
    ] {
        let m = classify(&e, text, scene).unwrap();
        assert_eq!(name_of(&m), "scene_speakers", "{text:?}");
        assert_eq!(m.intent().target, Target::Tool("scene_fact".into()));
        assert_eq!(m.intent().fact.as_deref(), Some("speakers"));
    }
}

// ----------------------------------------------------------------- cache and waits

#[test]
fn the_examples_are_embedded_once_per_model_and_language() {
    let e = BandEmbedder::new("once");
    let english: usize = intents().iter().map(|i| i.examples.len()).sum();
    best(&e, "How many scenes make up my screenplay?");
    assert_eq!(e.texts(), english + 1);
    best(&e, "Who are the crew members?");
    assert_eq!(e.texts(), english + 2);
    // Another model starts again.
    let other = BandEmbedder::new("once-other");
    best(&other, "Who are the crew members?");
    assert_eq!(other.texts(), english + 1);
}

#[test]
fn a_question_in_another_language_also_uses_that_languages_examples() {
    let e = BandEmbedder::new("spanish");
    let spanish: usize = intents().iter().map(|i| i.other["es"].len()).sum();
    let english: usize = intents().iter().map(|i| i.examples.len()).sum();
    let m = best(&e, "¿Cuántas escenas tiene el guion con los actos?");
    assert_eq!(e.texts(), english + spanish + 1);
    assert_eq!(name_of(&m), "count_scenes");
}

#[test]
fn a_failing_embedder_gives_nothing_and_never_panics() {
    let e = BandEmbedder::new("failing");
    e.set_failing(true);
    assert!(classify(&e, "How many scenes are there?", ANY).is_none());
    // A later success is not blocked by the failure.
    e.set_failing(false);
    assert!(classify(&e, "How many scenes are there?", ANY).is_some());
}

#[test]
fn a_slow_embedder_is_skipped_and_its_late_result_helps_the_next_request() {
    let e = Arc::new(BandEmbedder::new("slow"));
    e.set_delay(Some(Duration::from_millis(300)));
    let started = std::time::Instant::now();
    let first = classify_within(
        e.clone(),
        "Who are the crew members?",
        ANY,
        Duration::from_millis(40),
    );
    assert!(first.is_none());
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "the request must not wait"
    );
    // The worker thread finishes the examples in the background.
    let mut ready = false;
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        if CACHE.lock().contains_key(&(e.model_id(), "en")) {
            ready = true;
            break;
        }
    }
    assert!(ready, "the examples were never cached");
    let again = classify_within(e, "Who are the crew members?", ANY, Duration::from_secs(5));
    assert!(again.is_some());
}

// ----------------------------------------------------------------- direct calls

fn understood(characters: &[&str], scenes: &[usize]) -> Understood {
    Understood {
        entities: characters
            .iter()
            .map(|c| Entity {
                kind: EntityKind::Character,
                id: c.to_uppercase(),
                label: (*c).to_string(),
            })
            .collect(),
        scenes: scenes.to_vec(),
        missing_scenes: Vec::new(),
        acts: Vec::new(),
        depth: Depth::Quick,
        intent: Kind::Other,
        fuzzy: Vec::new(),
        pronoun_unresolved: false,
        whole_range: false,
    }
}

fn intent(name: &str) -> &'static Intent {
    intents().iter().find(|i| i.name == name).unwrap()
}

fn schema_of(tool: &str) -> Value {
    super::super::tools::spec(tool)
        .map(|t| (t.schema)())
        .or_else(|| super::super::toolbox::reads::spec(tool).map(|r| (r.schema)()))
        .unwrap()
}

fn required_of(tool: &str) -> Vec<String> {
    schema_of(tool)["required"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn call(name: &str, u: Option<&Understood>, text: &str) -> Option<StepCall> {
    let i = intent(name);
    let Target::Tool(t) = &i.target else { panic!() };
    direct_call(i, u, text, &required_of(t))
}

/// Whatever a direct call holds passes the tool's own strict schema.
fn valid(c: &StepCall) {
    super::super::toolbox::schema::validate(&c.tool, &schema_of(&c.tool), &c.arguments)
        .unwrap_or_else(|e| panic!("{} {}: {e:?}", c.tool, c.arguments));
}

#[test]
fn tools_without_required_arguments_are_called_at_once() {
    for name in [
        "count_scenes",
        "list_characters",
        "project_tasks",
        "idea_vault",
        "schedule_overview",
    ] {
        let c = call(name, None, "anything").unwrap_or_else(|| panic!("{name}"));
        assert_eq!(c.arguments, json!({}));
        assert!(c.done);
        valid(&c);
    }
}

#[test]
fn a_scene_fact_needs_exactly_one_scene_the_draft_has() {
    let one = understood(&[], &[3]);
    let c = call("scene_speakers", Some(&one), "Who talks in scene 3?").unwrap();
    assert_eq!(c.tool, "scene_fact");
    assert_eq!(c.arguments, json!({"scene": 3, "fact": "speakers"}));
    valid(&c);
    let none = understood(&[], &[]);
    assert!(call("scene_speakers", Some(&none), "Who talks in scene 99?").is_none());
    let two = understood(&[], &[2, 3]);
    assert!(call("scene_speakers", Some(&two), "Who talks in scenes 2 and 3?").is_none());
    assert!(call("scene_speakers", None, "Who talks in scene 3?").is_none());
    let t = call("scene_time", Some(&one), "day or night in scene 3?").unwrap();
    assert_eq!(t.arguments["fact"], "time");
    valid(&t);
}

#[test]
fn a_dialogue_fact_names_a_character_only_when_the_fact_is_about_one() {
    let most = call("dialogue_most", None, "who talks most").unwrap();
    assert_eq!(most.arguments, json!({"fact": "most"}));
    valid(&most);
    assert!(call("dialogue_scenes", None, "where does he talk").is_none());
    let ravi = understood(&["Ravi"], &[]);
    let c = call("dialogue_scenes", Some(&ravi), "Where does Ravi talk?").unwrap();
    assert_eq!(c.arguments, json!({"fact": "scenes", "character": "Ravi"}));
    valid(&c);
    let two = understood(&["Ravi", "Meena"], &[]);
    assert!(
        call(
            "dialogue_scenes",
            Some(&two),
            "Where do Ravi and Meena talk?"
        )
        .is_none()
    );
}

#[test]
fn scenes_with_characters_needs_two_known_names() {
    let two = understood(&["Ravi", "Meena"], &[]);
    let c = call(
        "scenes_with_characters",
        Some(&two),
        "Scenes with Ravi and Meena",
    )
    .unwrap();
    assert_eq!(c.arguments, json!({"characters": ["Ravi", "Meena"]}));
    valid(&c);
    let one = understood(&["Ravi"], &[]);
    assert!(
        call(
            "scenes_with_characters",
            Some(&one),
            "Scenes with Ravi and Zed"
        )
        .is_none()
    );
}

#[test]
fn a_search_needs_a_term_that_is_clear() {
    let u = understood(&[], &[]);
    for (text, term) in [
        ("Which scenes mention the lantern?", "lantern"),
        ("Find every scene that talks about the umbrella", "umbrella"),
        ("Search everything for lantern oil", "lantern oil"),
        ("Find scenes where the old radio appears", "old radio"),
        ("Scenes that contain the word storm", "storm"),
        ("Look for \"hidden key\" in the project", "hidden key"),
        ("Find 'returned to owner' anywhere", "returned to owner"),
    ] {
        assert_eq!(search_term(text).as_deref(), Some(term), "{text}");
    }
    for text in [
        "Which scenes mention it?",
        "Find scenes about anything",
        "How many scenes are there?",
        "Search",
    ] {
        assert_eq!(search_term(text), None, "{text}");
    }
    let c = call(
        "find_scenes_mentioning",
        Some(&u),
        "Which scenes mention the lantern?",
    )
    .unwrap();
    assert_eq!(c.arguments, json!({"text": "lantern"}));
    valid(&c);
    assert!(
        call(
            "find_scenes_mentioning",
            Some(&u),
            "Which scenes are about it"
        )
        .is_none()
    );
}

#[test]
fn a_product_question_passes_the_text_and_an_unfillable_tool_is_left_to_the_model() {
    let c = call("product_help", None, "  How can I back up my project?  ").unwrap();
    assert_eq!(
        c.arguments,
        json!({"question": "How can I back up my project?"})
    );
    valid(&c);
    // A workspace is an enum the model picks; a chat intent is no tool.
    assert!(call("open_workspace", None, "open the vault").is_none());
    assert!(direct_call(intent("ideas"), None, "ideas", &[]).is_none());
}

#[test]
fn a_breakdown_suggestion_names_its_scene_although_the_schema_leaves_it_optional() {
    let one = understood(&[], &[14]);
    let c = call(
        "suggest_breakdown",
        Some(&one),
        "Recommend breakdown items for scene 14",
    )
    .unwrap();
    assert_eq!(c.tool, "suggest_breakdown");
    assert_eq!(c.arguments, json!({"sceneNumber": 14}));
    valid(&c);
    assert!(
        call(
            "suggest_breakdown",
            Some(&understood(&[], &[])),
            "breakdown items"
        )
        .is_none()
    );
}
