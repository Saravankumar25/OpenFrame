use std::sync::Arc;

use openframe_ai::{ChatRequest, InferenceProvider, ProviderCapabilities};
use openframe_domain::{AppError, AppResult};
use openframe_search::{Embedder, SearchError};
use parking_lot::Mutex;

use super::AnswerType::*;
use super::*;
use crate::modules::ai::index::{Hit, How as Mention, Kind, Resolved, SceneHit};

fn read_with(message: &str, previous: Option<AnswerType>, r: &Resolved) -> TurnRead {
    read(
        &TurnInput {
            message,
            previous_message: None,
            previous_type: previous,
            resolved: r,
        },
        &Aids::none(),
    )
}

fn plain(message: &str) -> TurnRead {
    read_with(message, None, &Resolved::default())
}

fn named(name: &str) -> Hit {
    Hit {
        id: name.to_lowercase(),
        name: name.into(),
        how: Mention::Exact,
    }
}

fn scene_hit(n: u32) -> SceneHit {
    SceneHit {
        screenplay_id: "sp".into(),
        ordinal: n,
        scene_id: Some(format!("s{n}")),
        heading: None,
        how: Mention::Exact,
    }
}

fn with_scene(n: u32) -> Resolved {
    Resolved {
        scenes: vec![scene_hit(n)],
        ..Resolved::default()
    }
}

fn with_title(title: &str) -> Resolved {
    Resolved {
        screenplays: vec![named(title)],
        ..Resolved::default()
    }
}

#[test]
fn strong_rules_read_the_shape_of_the_question() {
    let cases: &[(&str, AnswerType)] = &[
        ("how many props are in act two", Count),
        ("what is the number of shoot days", Count),
        ("how are Dev and his sister related", Relation),
        ("the bond between the captain and the cook", Relation),
        ("planned versus actual for catering", Compare),
        ("are we over budget on transport", Compare),
        ("compare act one and act three", Compare),
        ("go to the budget screen", Navigate),
        ("could you take me to the call sheets", Navigate),
        ("ok bro open the settings", Navigate),
        ("open the file map.png", Navigate),
        ("explain the opening", Explain),
        ("I need an explanation of the ending", Explain),
        ("can you describe the mood of the harbour", Explain),
        ("what happens at the end of act two", Explain),
        ("how do I export a PDF", Help),
        ("where do I find the backups", Help),
        ("can I use this offline", Help),
        ("is there a way to print the schedule", Help),
        ("add a task to scout the pier", Change),
        ("please rename the teaser", Change),
        ("move scene 9 after scene 12", Change),
        ("give me three ideas for the finale", Brainstorm),
        ("how could I make the chase more tense", Brainstorm),
        ("what if the sister never leaves", Brainstorm),
        ("make it more dramatic", Brainstorm),
        ("who plays the baker", Lookup),
        ("what is the phone of the gaffer", Lookup),
        ("how much is the van hire", Lookup),
        ("list every prop in the kitchen", List),
        ("which scenes take place at sea", List),
    ];
    for (m, want) in cases {
        let got = plain(m);
        assert_eq!((*m, got.answer_type), (*m, *want), "{got:?}");
        assert_eq!(got.how, ReadHow::Rule, "{m}");
    }
}

#[test]
fn how_many_times_is_a_complaint_not_a_count() {
    let r = read_with(
        "bro how many times do I have to say it is the feature",
        Some(Navigate),
        &Resolved::default(),
    );
    assert_eq!((r.answer_type, r.how), (Navigate, ReadHow::Repair));
}

#[test]
fn a_scene_number_opens_only_with_an_open_word() {
    let r = with_scene(42);
    assert_eq!(read_with("open scene 42", None, &r).answer_type, Navigate);
    assert_eq!(
        read_with("scene 42 of the teaser, open it", None, &r).answer_type,
        Navigate
    );
    assert_eq!(
        read_with("what is the location in scene 42", None, &r).answer_type,
        Lookup
    );
}

#[test]
fn a_correction_takes_the_type_it_states_or_the_previous_one() {
    let none = Resolved::default();
    // States what was wanted.
    assert_eq!(
        read_with(
            "I did not ask you to open it, I asked you to explain scene 35",
            Some(Navigate),
            &none
        )
        .answer_type,
        Explain
    );
    assert_eq!(
        read_with(
            "I asked for scene 35 explanation and you gave me a list",
            Some(List),
            &none
        )
        .answer_type,
        Explain
    );
    // Names no new request: the previous type, marked as inherited.
    let r = read_with("I said the feature, not the teaser", Some(Count), &none);
    assert_eq!((r.answer_type, r.how), (Count, ReadHow::Repair));
    let r = read_with("again bro, you opened the wrong one", Some(Lookup), &none);
    assert_eq!((r.answer_type, r.how), (Lookup, ReadHow::Repair));
    // Nothing to repair after small talk.
    let r = read_with("I said the feature", Some(Chat), &none);
    assert_ne!(r.how, ReadHow::Repair);
}

#[test]
fn a_bare_title_answers_the_last_question() {
    let r = read_with("LOW TIDE RADIO", Some(Count), &with_title("Low Tide Radio"));
    assert_eq!((r.answer_type, r.how), (Count, ReadHow::Repair));
    // A title with a question word is a new question, not an answer.
    let r = read_with(
        "what is LOW TIDE RADIO about",
        Some(Count),
        &with_title("Low Tide Radio"),
    );
    assert_ne!(r.how, ReadHow::Repair);
    // Without a previous message there is nothing to inherit.
    let r = read_with("LOW TIDE RADIO", None, &with_title("Low Tide Radio"));
    assert_ne!(r.how, ReadHow::Repair);
}

#[test]
fn anger_and_small_talk_are_chat_when_nothing_is_asked() {
    for m in [
        "fuck you",
        "GO TO HELL",
        "thanks, that is all",
        "you are the worst",
        "WOW THIS IS RUBBISH I AM UNINSTALLING IT NEVER OPENING THIS APP AGAIN",
        "what the hell, who asked for that",
        "ok cool",
        "wow bro you ask again and again, you dont open the right scenes, you dont explain what I ask",
    ] {
        let r = plain(m);
        assert_eq!((m, r.answer_type, r.how), (m, Chat, ReadHow::Rule), "{m}");
        assert_eq!(r.length_class, LengthClass::Brief);
    }
    // Thanks in front of a real request is the request.
    assert_eq!(plain("thanks, now list the props").answer_type, List);
    // Anger that repeats a request takes the last type and is never a chat.
    let r = read_with(
        "what the fuck, I literally told you it is LOW TIDE RADIO",
        Some(Navigate),
        &with_title("Low Tide Radio"),
    );
    assert_eq!((r.answer_type, r.how), (Navigate, ReadHow::Repair));
    // A name in the message is a request even beside swearing.
    let r = read_with("fuck LOW TIDE RADIO", None, &with_title("Low Tide Radio"));
    assert_ne!(r.how, ReadHow::Rule);
}

#[test]
fn plural_nouns_contents_and_two_scenes_have_their_own_rules() {
    for (m, want) in [
        ("which budget lines have no actual yet", List),
        ("which locations are still unconfirmed", List),
        ("what is in the Permits folder", List),
        ("what is on day 10", List),
        ("who are the speaking roles", List),
        ("total planned budget of all lines", Lookup),
        ("what does it cost", Lookup),
        ("does the AI need the internet", Help),
        ("is my project sent anywhere", Help),
        ("what is the Production Source", Help),
    ] {
        assert_eq!(plain(m).answer_type, want, "{m}");
    }
    for (m, want) in [
        ("who is in scene 1", List),
        ("when do we shoot scene 2", Lookup),
        ("and which shoot day is that", Lookup),
    ] {
        assert_eq!(plain(m).answer_type, want, "{m}");
    }
    assert_ne!(plain("what special effects does it need").answer_type, Help);
    // "which" before a singular word, a verb or "has" is not a plural noun.
    assert_ne!(plain("which scene has the most lines").answer_type, List);
    assert_ne!(plain("which day is that").answer_type, List);
    // Small talk with "who are" is not a list.
    assert_eq!(plain("who are you").how, ReadHow::Fallback);
    // Two scenes named together ask how they relate; one scene does not.
    let two = Resolved {
        scenes: vec![scene_hit(3), scene_hit(41)],
        ..Resolved::default()
    };
    assert_eq!(
        read_with("does scene 3 share a place with scene 41", None, &two).answer_type,
        Compare
    );
    assert_eq!(
        read_with("list the props in scene 3 and scene 41", None, &two).answer_type,
        List
    );
    assert_eq!(
        read_with("open scene 3 and scene 41", None, &two).answer_type,
        Navigate
    );
    // Two scenes inside a correction are not a comparison.
    let r = read_with(
        "You have incorrectly opened Scene 3, I asked for Scene 41",
        Some(Navigate),
        &two,
    );
    assert_eq!(r.answer_type, Navigate);
    let r = read_with("I asked for scene 41 not scene 3", Some(Lookup), &two);
    assert_eq!((r.answer_type, r.how), (Lookup, ReadHow::Repair));
    assert_eq!(
        read_with("show me scene 41", None, &with_scene(41)).answer_type,
        Navigate
    );
}

#[test]
fn a_complaint_is_not_a_request_and_the_wish_comes_before_the_but() {
    // The verbs after "you dont" are things the assistant failed to do.
    let r = plain(
        "wow bro you dont open the right scenes you dont explain what I ask you dont help me",
    );
    assert_eq!((r.answer_type, r.how), (Chat, ReadHow::Rule));
    // "I asked for X ... but you opened Y": the wish is X.
    let r = read_with(
        "I asked for the character relationships but you opened scene 13 in the screenplay",
        Some(Navigate),
        &with_scene(13),
    );
    assert_eq!(r.answer_type, Relation);
    // Anger without a name or a request is chat, even when it repeats "I asked".
    let r = read_with(
        "I asked three times and you still got it wrong. Useless.",
        Some(Navigate),
        &Resolved::default(),
    );
    assert_eq!((r.answer_type, r.how), (Chat, ReadHow::Rule));
}

#[test]
fn groups_follow_the_spec() {
    let g = |t: AnswerType| t.group();
    for t in [Lookup, List, Count, Compare] {
        assert_eq!(g(t), Group::Facts);
    }
    for t in [Explain, Relation] {
        assert_eq!(g(t), Group::Narrative);
    }
    assert_eq!(g(Brainstorm), Group::Brainstorm);
    assert_eq!(g(Navigate), Group::Navigate);
    assert_eq!(g(Change), Group::Change);
    assert_eq!(g(Help), Group::Help);
    assert_eq!(g(Chat), Group::Chat);
    for t in ANSWER_TYPES {
        assert_eq!(AnswerType::parse(t.code()), Some(t));
    }
}

#[test]
fn the_length_class_follows_the_type_and_moves_with_the_words() {
    let class = |m: &str, t: AnswerType| rules::length_class(m, t);
    assert_eq!(
        class("who plays the baker", Lookup),
        LengthClass::OneSentence
    );
    assert_eq!(class("list the props", List), LengthClass::Short);
    assert_eq!(class("explain scene 4", Explain), LengthClass::Paragraph);
    assert_eq!(class("hello", Chat), LengthClass::Brief);
    // Shorter.
    assert_eq!(
        class("explain scene 4 briefly", Explain),
        LengthClass::Short
    );
    assert_eq!(
        class("tell the story in brief", Explain),
        LengthClass::Short
    );
    assert_eq!(
        class("explain scene 4 in one line", Explain),
        LengthClass::OneSentence
    );
    assert_eq!(
        class("how many scenes, one sentence", Count),
        LengthClass::OneSentence
    );
    // Longer.
    assert_eq!(
        class("explain it in detail", Explain),
        LengthClass::Paragraph
    );
    assert_eq!(
        class("list the props in detail", List),
        LengthClass::Paragraph
    );
    assert_eq!(
        class("who plays the baker in detail", Lookup),
        LengthClass::Short
    );
    assert_eq!(
        class("explain at least one long paragraph", Explain),
        LengthClass::Paragraph
    );
    // Both: no move. Chat never grows.
    assert_eq!(class("briefly, in detail", Explain), LengthClass::Paragraph);
    assert_eq!(class("thanks in detail", Chat), LengthClass::Brief);
    // The words of the classes are the ones the evaluation scores.
    assert_eq!(LengthClass::OneSentence.words(), (1, 30));
    assert_eq!(LengthClass::Paragraph.words(), (40, 200));
}

#[test]
fn domains_come_from_topic_words_and_resolved_names() {
    let d = |m: &str, r: &Resolved, t: AnswerType| {
        domains::find(m, r, t)
            .into_iter()
            .map(Domain::code)
            .collect::<Vec<_>>()
    };
    let none = Resolved::default();
    assert_eq!(
        d("planned and actual budget for wardrobe", &none, Compare),
        ["breakdown", "budget"]
    );
    assert_eq!(
        d("which day is the car chase shot", &none, Lookup),
        ["schedule"]
    );
    assert_eq!(
        d("how do I change the language", &none, Help),
        ["settings", "product"]
    );
    assert_eq!(d("hello", &none, Chat), Vec::<&str>::new());
    // Resolved names add domains the words do not carry.
    let r = Resolved {
        characters: vec![named("Anya")],
        persons: vec![named("Ravi")],
        days: vec![named("Day 4")],
        others: vec![(Kind::VaultItem, named("palette"))],
        scenes: vec![scene_hit(3)],
        ..Resolved::default()
    };
    assert_eq!(
        d("tell me about her", &r, Explain),
        ["story", "screenplay", "people", "schedule", "visual"]
    );
    // A screenplay taken from the focus is not named by the message.
    let implied = Resolved {
        screenplays: vec![Hit {
            id: "sp".into(),
            name: "Low Tide Radio".into(),
            how: Mention::Focus,
        }],
        ..Resolved::default()
    };
    assert!(d("hello", &implied, Chat).is_empty());
}

#[test]
fn without_an_embedder_the_previous_type_or_a_default_answers() {
    let none = Resolved::default();
    let r = read_with("the thing about the second one", Some(Explain), &none);
    assert_eq!((r.answer_type, r.how), (Explain, ReadHow::Fallback));
    let r = read_with("could that be done differently?", None, &none);
    assert_eq!((r.answer_type, r.how), (Lookup, ReadHow::Fallback));
    let r = read_with("the second one then", None, &none);
    assert_eq!((r.answer_type, r.how), (Chat, ReadHow::Fallback));
    // Small talk before is not a type to inherit.
    let r = read_with("the second one then", Some(Chat), &none);
    assert_eq!(r.how, ReadHow::Fallback);
    assert_ne!(r.answer_type, Explain);
}

// ------------------------------------------------------------------ nearest neighbours

/// Every example is its own dimension plus a dimension for its type, so a test can build a message
/// that sits exactly on chosen examples: `votes: a | b | c` is the sum of those examples.
struct ExampleEmbedder {
    fail: bool,
    /// Each embedder has its own id, so tests running side by side never share a vector cache.
    id: u64,
}

impl ExampleEmbedder {
    const TYPES: usize = 11;

    fn new(fail: bool) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self {
            fail,
            id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    fn vector_of(&self, text: &str) -> Vec<f32> {
        let ex = neighbour::examples();
        let mut v = vec![0.0; Self::TYPES + ex.len()];
        let mut put = |i: usize| {
            v[Self::TYPES + i] += 2.0;
            v[ex[i].t as usize] += 1.0;
        };
        if let Some(list) = text.strip_prefix("zz ") {
            for i in list
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter_map(|n| n.parse::<usize>().ok())
                .filter(|i| *i < ex.len())
            {
                put(i);
            }
        } else if let Some(i) = ex.iter().position(|x| x.text == text) {
            put(i);
        }
        v
    }
}

impl Embedder for ExampleEmbedder {
    fn model_id(&self) -> String {
        format!("test-example-embedder-{}", self.id)
    }
    fn dim(&self) -> usize {
        Self::TYPES + neighbour::examples().len()
    }
    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, SearchError> {
        if self.fail {
            return Err(SearchError::Embedding("down".into()));
        }
        Ok(inputs.iter().map(|t| self.vector_of(t)).collect())
    }
}

struct Scripted {
    reply: &'static str,
    calls: Mutex<Vec<String>>,
}

impl InferenceProvider for Scripted {
    fn chat(&self, req: &ChatRequest) -> AppResult<String> {
        self.calls.lock().push(
            req.messages
                .last()
                .map(|m| m.content.clone())
                .unwrap_or_default(),
        );
        if self.reply == "error" {
            return Err(AppError::internal("down"));
        }
        Ok(self.reply.to_string())
    }
    fn model_reference(&self) -> String {
        "scripted".into()
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            context_tokens: 4096,
            languages: None,
            grammar: false,
        }
    }
}

/// A message that sits exactly on the first unused examples of the given types: `zz 4,31,60`. It has
/// no English word, so no rule reads it and only the vote decides.
fn voting(types: &[AnswerType]) -> String {
    let ex = neighbour::examples();
    let mut taken = Vec::new();
    for t in types {
        let i = (0..ex.len())
            .find(|i| ex[*i].t == *t && !taken.contains(i))
            .unwrap();
        taken.push(i);
    }
    let list: Vec<String> = taken.iter().map(usize::to_string).collect();
    format!("zz {}", list.join(","))
}

fn read_aided(message: &str, previous: Option<AnswerType>, aids: &Aids<'_>) -> TurnRead {
    read(
        &TurnInput {
            message,
            previous_message: Some("an earlier message"),
            previous_type: previous,
            resolved: &Resolved::default(),
        },
        aids,
    )
}

fn aids<'a>(e: Option<ExampleEmbedder>, model: Option<&'a Scripted>) -> Aids<'a> {
    Aids {
        embedder: e.map(|e| Arc::new(e) as Arc<dyn Embedder>),
        model: model.map(|m| m as &dyn InferenceProvider),
        stopped: &|| false,
        wait: Duration::from_secs(30),
    }
}

#[test]
fn a_clear_vote_decides_without_the_model() {
    let model = Scripted {
        reply: "chat",
        calls: Mutex::default(),
    };
    let msg = voting(&[Compare, Compare, Compare, List, Lookup]);
    let r = read_aided(
        &msg,
        None,
        &aids(Some(ExampleEmbedder::new(false)), Some(&model)),
    );
    assert_eq!((r.answer_type, r.how), (Compare, ReadHow::Neighbour));
    assert!((r.vote - 0.6).abs() < 1e-6);
    assert!(model.calls.lock().is_empty());
}

#[test]
fn a_weak_vote_asks_the_model_with_the_closed_list() {
    let model = Scripted {
        reply: "Relation.",
        calls: Mutex::default(),
    };
    let msg = voting(&[Compare, Compare, List, List, Lookup]);
    let r = read_aided(
        &msg,
        None,
        &aids(Some(ExampleEmbedder::new(false)), Some(&model)),
    );
    assert_eq!((r.answer_type, r.how), (Relation, ReadHow::Model));
    let asked = model.calls.lock();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("lookup, list, count, explain, relation"));
    assert!(asked[0].contains("Previous message: an earlier message"));
}

#[test]
fn a_model_that_fails_or_answers_off_the_list_leaves_the_vote() {
    for reply in ["error", "banana", ""] {
        let model = Scripted {
            reply,
            calls: Mutex::default(),
        };
        let msg = voting(&[Compare, Compare, List, List, Lookup]);
        let r = read_aided(
            &msg,
            None,
            &aids(Some(ExampleEmbedder::new(false)), Some(&model)),
        );
        assert_eq!(r.how, ReadHow::Neighbour, "{reply}");
        assert_eq!(model.calls.lock().len(), 1);
    }
    assert_eq!(model::parse_reply("  help\n"), Some(Help));
    assert_eq!(model::parse_reply("1. lookup"), None);
}

#[test]
fn a_stopped_request_starts_no_model_call() {
    let model = Scripted {
        reply: "chat",
        calls: Mutex::default(),
    };
    let msg = voting(&[Compare, Compare, List, List, Lookup]);
    let a = Aids {
        stopped: &|| true,
        ..aids(Some(ExampleEmbedder::new(false)), Some(&model))
    };
    let r = read_aided(&msg, None, &a);
    assert_eq!(r.how, ReadHow::Neighbour);
    assert!(model.calls.lock().is_empty());
}

#[test]
fn a_rule_never_embeds_and_never_calls_the_model() {
    let model = Scripted {
        reply: "chat",
        calls: Mutex::default(),
    };
    let r = read_aided(
        "how many scenes are there",
        None,
        &aids(Some(ExampleEmbedder::new(true)), Some(&model)),
    );
    assert_eq!((r.answer_type, r.how), (Count, ReadHow::Rule));
    assert!(model.calls.lock().is_empty());
}

#[test]
fn a_failing_embedder_falls_back_without_calling_the_model() {
    let model = Scripted {
        reply: "chat",
        calls: Mutex::default(),
    };
    let r = read_aided(
        "something unclear here",
        Some(List),
        &aids(Some(ExampleEmbedder::new(true)), Some(&model)),
    );
    assert_eq!((r.answer_type, r.how), (List, ReadHow::Fallback));
    assert!(model.calls.lock().is_empty());
}

#[test]
fn anger_is_never_read_as_a_change_or_a_count_by_the_neighbours() {
    let msg = voting(&[Change, Change, Change, Change, Change]);
    // "props" keeps the chat gate shut; "fuck" makes it angry.
    let angry = format!("{msg} fuck props");
    // The message has no rule hit, the vote is clear for "change", and it is angry.
    let r = read_aided(&angry, None, &aids(Some(ExampleEmbedder::new(false)), None));
    assert_eq!(r.answer_type, Chat);
    let calm = read_aided(&msg, None, &aids(Some(ExampleEmbedder::new(false)), None));
    assert_eq!(calm.answer_type, Change);
}

#[test]
fn leave_one_out_reads_every_example_without_itself() {
    let loo = leave_one_out(&ExampleEmbedder::new(false)).unwrap();
    assert_eq!(loo.len(), neighbour::examples().len());
}

// ------------------------------------------------------------------ the example file

#[test]
fn the_example_file_covers_every_type_in_all_twelve_languages() {
    let ex = neighbour::examples();
    for t in ANSWER_TYPES {
        let of: Vec<_> = ex.iter().filter(|x| x.t == t).collect();
        assert!(
            (20..=30).contains(&of.len()),
            "{}: {} examples",
            t.code(),
            of.len()
        );
        for lang in [
            "en", "hi", "te", "ta", "kn", "ml", "es", "fr", "de", "zh", "ja", "ko",
        ] {
            assert!(
                of.iter().any(|x| x.lang == lang),
                "{} has no {lang} example",
                t.code()
            );
        }
    }
    let mut seen = std::collections::HashSet::new();
    for x in ex {
        assert!(seen.insert(x.text.to_lowercase()), "duplicate: {}", x.text);
        assert!(!x.text.contains('\u{2014}'), "em dash in: {}", x.text);
        assert!(x.text.chars().count() <= 80, "too long: {}", x.text);
    }
}

// ------------------------------------------------------------------ warm-up (slice S8)

#[test]
fn the_warm_up_prompt_is_the_label_prompt_with_one_token_asked() {
    let prompts = warm::prompts();
    assert_eq!(prompts.len(), 1);
    let p = &prompts[0];
    assert_eq!(p.max_tokens, 1);
    // Its start is the text every real read starts with, so the engine's stored work serves both.
    let real = model::request("how many scenes", Some("an earlier message"));
    let (a, b) = (&p.messages[0].content, &real.messages[0].content);
    let start = a.find("\n\nMessage:").unwrap();
    assert!(b.starts_with(&a[..start]));
    assert_eq!(real.max_tokens, 6);
}

#[test]
fn the_warm_up_embeds_the_examples_once_and_a_failing_embedder_leaves_nothing_behind() {
    let ok = ExampleEmbedder::new(false);
    assert!(warm::examples(&ok));
    // The second call finds the vectors of this embedder in the cache: a failing one is not asked.
    assert!(warm::examples(&ok));
    assert!(!warm::examples(&ExampleEmbedder::new(true)));
}
