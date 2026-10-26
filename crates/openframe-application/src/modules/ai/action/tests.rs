use super::*;

/// The rendering used before the compact catalogue: the full JSON schema of every tool.
fn old_catalog_text(tools: &[ToolDef]) -> String {
    let mut s = String::from("Tools:\n");
    for t in tools {
        s.push_str(&format!(
            "- {}: {} Arguments: {}\n",
            t.name, t.description, t.schema
        ));
    }
    s.push_str(&format!(
        "- {FINAL_TOOL}: Finish with a short answer written only from the tool results above. Arguments: {}\n",
        final_schema()
    ));
    s
}

fn real(names: &[&str]) -> Vec<ToolDef> {
    let all = toolbox::all_defs_unchecked();
    names
        .iter()
        .map(|n| {
            all.iter()
                .find(|t| t.name == *n)
                .unwrap_or_else(|| panic!("{n} is not a tool"))
                .clone()
        })
        .collect()
}

#[test]
fn a_signature_lists_required_arguments_first_then_optional_ones() {
    let tools = real(&["retrieve_context", "count_scenes"]);
    assert_eq!(
        signature(&tools[0].schema),
        "query: string, limit?: integer"
    );
    assert_eq!(signature(&json!({"type": "object", "properties": {}})), "");
    assert_eq!(signature(&final_schema()), "text: string");
    let s = json!({"type": "object", "additionalProperties": false,
        "properties": {
            "tags": {"type": "array", "items": {"type": "string", "maxLength": 4}, "maxItems": 2},
            "kind": {"type": "string", "enum": ["a", "b"]},
            "kinds": {"type": "array", "items": {"type": "string", "enum": ["a", "b"]}, "maxItems": 2},
            "where": {"type": "object", "properties": {}},
            "n": {"type": "integer", "minimum": 1, "maximum": 5},
            "on": {"type": "boolean"}},
        "required": ["on", "kind"]});
    assert_eq!(
        signature(&s),
        r#"on: boolean, kind: "a"|"b", kinds?: ("a"|"b")[], n?: integer, tags?: string[], where?: object"#
    );
    // Every real tool renders without panicking and never prints a raw schema keyword.
    for t in toolbox::all_defs_unchecked() {
        let sig = signature(&t.schema);
        assert!(
            !sig.contains("maxLength") && !sig.contains("additionalProperties"),
            "{}: {sig}",
            t.name
        );
        assert_eq!(
            t.schema["properties"].as_object().map_or(0, |p| p.len()),
            if sig.is_empty() {
                0
            } else {
                sig.matches(": ").count()
            },
            "{}: {sig}",
            t.name
        );
    }
}

fn known(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn def(name: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        class: OperationClass::Read,
        description: "test".into(),
        schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        terminal: false,
        mutating: false,
    }
}
#[test]
fn step_schema_offers_only_known_tools_and_final_answer() {
    let tools = vec![def("count_scenes"), def("open_scene")];
    let schema = step_schema(&tools);
    let variants = schema["oneOf"].as_array().unwrap();
    assert_eq!(variants.len(), 3);
    for v in variants {
        assert_eq!(v["additionalProperties"], false);
        assert_eq!(v["required"], json!(["action", "arguments", "done"]));
    }
    let k = known(&["count_scenes"]);
    assert!(parse_step(r#"{"tool":"drop_database","arguments":{}}"#, &k).is_err());
    assert!(
        parse_step(
            r#"{"tool":"ai.change_set.accept","arguments":{"id":"x"}}"#,
            &k
        )
        .is_err()
    );
    let fenced = parse_step(
        "```json\n{\"tool\":\"count_scenes\",\"arguments\":{}}\n```",
        &k,
    )
    .unwrap();
    assert!(fenced.done, "a reply without done is final");
    let more = parse_step(r#"{"tool":"count_scenes","arguments":{},"done":false}"#, &k).unwrap();
    assert!(!more.done);
    assert!(
        parse_step(
            r#"{"tool":"final_answer","arguments":{"text":"x"},"done":true}"#,
            &k
        )
        .is_ok()
    );
    assert!(parse_step("not json", &k).is_err());
    assert!(
        parse_step(
            &format!(
                "{{\"tool\":\"count_scenes\",\"arguments\":{{\"a\":\"{}\"}}}}",
                "x".repeat(5_000)
            ),
            &k
        )
        .is_err(),
        "oversized model output is rejected"
    );
}

#[test]
fn observations_are_bounded_and_older_ones_shortened() {
    let mut t = Transcript {
        base: vec![ChatMessage::system("s"), ChatMessage::user("u")],
        turns: Vec::new(),
    };
    for i in 0..8 {
        let call = StepCall {
            tool: format!("tool_{i}"),
            arguments: json!({}),
            done: false,
        };
        t.push(&call, i + 1, format!("Result {i}\n{}", "x".repeat(3_000)));
    }
    let msgs = t.messages();
    assert_eq!(msgs.len(), 2 + 16);
    let total: usize = msgs[2..].iter().map(|m| m.content.len()).sum();
    assert!(
        total < OBSERVATION_BUDGET + 16 * 400,
        "observations stay within budget: {total}"
    );
    assert!(
        msgs.last().unwrap().content.contains(&"x".repeat(1_000)),
        "the newest observation is complete"
    );
    assert!(
        !msgs[3].content.contains(&"x".repeat(500)),
        "the oldest one is shortened"
    );
    assert!(msgs[3].content.contains("trust=\"untrusted\""));
}

#[test]
fn composed_answers_keep_exact_results_and_mark_written_text_inferred() {
    let mut a = ToolOutput::exact("Draft 1 contains 3 scenes.");
    a.provenance.push(Provenance::new("Draft", "Draft 1"));
    let b = ToolOutput::exact("In Draft 1 there are 3 distinct character cues.");
    let both = compose_reads(&[a.clone(), b.clone()], None, None).unwrap();
    assert_eq!(
        both.content,
        "Draft 1 contains 3 scenes. In Draft 1 there are 3 distinct character cues."
    );
    assert_eq!(both.confidence, Some(Confidence::Exact));
    assert_eq!(both.provenance.len(), 1);
    let written =
        compose_reads(&[a, b], Some("Three scenes, three speakers."), Some("note")).unwrap();
    assert_eq!(written.confidence, Some(Confidence::Inferred));
    // The written answer is one clean reply: the tool sentences are not repeated under it
    // (OpenFrameAI.docx section 2.4); only notes the person needs remain.
    assert_eq!(written.content, "Three scenes, three speakers.");
    assert!(
        written
            .details
            .iter()
            .all(|d| !d.starts_with("From project data"))
    );
    assert_eq!(written.details.last().unwrap(), "note");
    assert!(compose_reads(&[], None, None).is_none());
}

#[test]
fn retrieved_passages_are_sources_of_a_written_answer_not_a_list() {
    let mut passages = ToolOutput::exact("2 passages related to “kite”.");
    passages.items.push(ResultItem {
        label: "Screenplay · Scene 4".into(),
        detail: None,
        nav: None,
    });
    passages
        .provenance
        .push(Provenance::new("Scene", "Scene 4"));
    passages.structured = Some(json!({ "indexState": "Ready" }));
    let mut list = ToolOutput::exact("1 scene mentions the kite.");
    list.items.push(ResultItem {
        label: "Scene 9".into(),
        detail: None,
        nav: None,
    });
    let reads = [passages, list];
    let written = compose_reads(&reads, Some("The kite stays home."), None).unwrap();
    let labels: Vec<&str> = written.items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, ["Scene 9"]);
    assert_eq!(written.provenance.len(), 1, "the passage is still cited");
    // With no written answer the passages are what was found, so they stay listed.
    assert_eq!(compose_reads(&reads, None, None).unwrap().items.len(), 2);
}

#[test]
fn items_are_the_answer_only_for_a_list_read_without_written_text() {
    let item = |l: &str| ResultItem {
        label: l.into(),
        detail: None,
        nav: None,
    };
    let mut count = ToolOutput::exact("Draft 1 has 2 scenes.");
    count.items.push(item("Scene 1"));
    let mut list = ToolOutput::exact("2 tasks are open.");
    list.items.push(item("Book the bus"));
    list.items_are_answer = true;
    // A count's items are evidence: they stay out of the answer.
    assert!(
        !compose_reads(&[count.clone()], None, None)
            .unwrap()
            .items_are_answer
    );
    // A list read alone is the answer.
    let one = compose_reads(&[list.clone()], None, None).unwrap();
    assert!(one.items_are_answer);
    // Next to another read only the list's rows are carried, and they are the answer.
    let both = compose_reads(&[count.clone(), list.clone()], None, None).unwrap();
    assert!(both.items_are_answer);
    assert_eq!(both.items.len(), 1);
    assert_eq!(both.items[0].label, "Book the bus");
    // Two reads without a list keep their evidence, flagged as not the answer.
    let facts = compose_reads(&[count.clone(), count.clone()], None, None).unwrap();
    assert!(!facts.items_are_answer);
    // Text the model wrote makes every item a source.
    let written = compose_reads(&[list], Some("Two tasks."), None).unwrap();
    assert!(!written.items_are_answer);
}

#[test]
fn background_tasks_are_reported_by_long_running_tools() {
    let mut o = ToolOutput::exact("Started.");
    assert_eq!(task_of(&o), None);
    o.structured = Some(json!({"taskId": "01J0TASK"}));
    assert_eq!(task_of(&o).as_deref(), Some("01J0TASK"));
    o.structured = Some(json!({"taskId": "x".repeat(200)}));
    assert_eq!(task_of(&o), None, "implausible task ids are ignored");
}

#[test]
fn the_catalogue_puts_the_fixed_tools_first_and_keeps_final_answer() {
    let a = tool_catalog_text(&real(&[
        "count_scenes",
        "retrieve_context",
        "search_project",
        "open_scene",
    ]));
    let b = tool_catalog_text(&real(&[
        "list_characters",
        "open_scene",
        "search_project",
        "retrieve_context",
    ]));
    let names = |s: &str| {
        s.lines()
            .skip(1)
            .map(|l| l[2..l.find('(').unwrap()].to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&a),
        [
            "retrieve_context",
            "search_project",
            "open_scene",
            "count_scenes",
            FINAL_TOOL
        ]
    );
    assert_eq!(
        names(&b),
        [
            "retrieve_context",
            "search_project",
            "open_scene",
            "list_characters",
            FINAL_TOOL
        ]
    );
    // The fixed tools are an identical start, so the engine can reuse its work for them.
    let start = |s: &str| s.lines().take(4).collect::<Vec<_>>().join("\n");
    assert_eq!(start(&a), start(&b));
    assert!(a.contains("- final_answer(text: string): Finish"));
}

#[test]
fn the_compact_catalogue_is_at_most_60_percent_of_the_old_one() {
    // A typical change request: the fixed tools plus the few that match it.
    let tools = real(&[
        "retrieve_context",
        "search_project",
        "project_overview",
        "open_object",
        "open_workspace",
        "open_scene",
        "private_information",
        "count_scenes",
        "scene_fact",
        "list_characters",
        "find_scenes_mentioning",
        "propose_task",
    ]);
    let (old, new) = (
        old_catalog_text(&tools).len(),
        tool_catalog_text(&tools).len(),
    );
    assert!(new * 100 <= old * 60, "old {old}, new {new}");
    let all = toolbox::all_defs_unchecked();
    assert!(tool_catalog_text(&all).len() < old_catalog_text(&all).len());
}

#[test]
fn a_navigation_message_opens_what_it_names_and_nothing_else() {
    use crate::modules::ai::index::{Hit, How, SceneHit};
    let scene = Resolved {
        scenes: vec![SceneHit {
            screenplay_id: "sp".into(),
            ordinal: 42,
            scene_id: Some("s".into()),
            heading: None,
            how: How::Exact,
        }],
        ..Resolved::default()
    };
    let c = target_of(&scene).unwrap();
    assert_eq!(
        (c.tool.as_str(), c.arguments.clone()),
        ("open_scene", json!({"sceneNumber": 42}))
    );
    let person = Resolved {
        persons: vec![Hit {
            id: "p".into(),
            name: "Rekha Thomas".into(),
            how: How::Exact,
        }],
        ..Resolved::default()
    };
    let c = target_of(&person).unwrap();
    assert_eq!(
        (c.tool.as_str(), c.arguments.clone()),
        ("open_object", json!({"text": "Rekha Thomas"}))
    );
    // A scene number the screenplay does not have is not a target, and the screenplay beside it is
    // not opened in its place.
    let missing = Resolved {
        screenplays: vec![Hit {
            id: "sp".into(),
            name: "Lost Property".into(),
            how: How::Exact,
        }],
        scenes: vec![SceneHit {
            screenplay_id: "sp".into(),
            ordinal: 46,
            scene_id: None,
            heading: None,
            how: How::Exact,
        }],
        ..Resolved::default()
    };
    assert!(target_of(&missing).is_none());
    assert!(is_navigation("open_scene") && !is_navigation("propose_task"));
}
