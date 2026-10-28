use super::*;

fn change(text: &str) -> Option<(String, serde_json::Value)> {
    change_command(text, |_| true).map(|c| (c.tool, c.arguments))
}

fn nav(text: &str) -> Option<(String, serde_json::Value)> {
    navigation_call(text, |_| true).map(|c| (c.tool, c.arguments))
}

#[test]
fn licence_and_payment_requests_get_a_plain_refusal() {
    for text in [
        "Change the project licence so I do not need to pay.",
        "Skip the activation.",
        "Remove the licence check.",
        "Extend my free trial.",
        "How can I use OpenFrame for free?",
    ] {
        assert!(not_the_assistants(text).is_some(), "{text}");
    }
    // Story content that uses the same words is the project's, not the program's.
    for text in [
        "Change the licence plate on the bus in scene 4 to AP 39 1989.",
        "Remove the driving licence from the props.",
        "Which scenes mention the payment to Farooq?",
        "Does Ravi pay the driver?",
        "Summarise my project notes.",
    ] {
        assert!(not_the_assistants(text).is_none(), "{text}");
    }
}

#[test]
fn a_plain_create_task_request_is_the_task_proposal() {
    for (text, title) in [
        (
            "Create a task to scout the railway station.",
            "Scout the railway station",
        ),
        (
            "create a task to book the bus for the night shoot",
            "Book the bus for the night shoot",
        ),
        (
            "Please add a new task: check the bus timetable!",
            "Check the bus timetable",
        ),
        (
            "Add a task called “Confirm pier permit”.",
            "Confirm pier permit",
        ),
        (
            "Remind me to call the art department.",
            "Call the art department",
        ),
        ("Task: hire a generator", "Hire a generator"),
    ] {
        assert_eq!(
            change(text),
            Some(("propose_task".to_string(), json!({ "title": title }))),
            "{text}"
        );
    }
}

#[test]
fn a_plain_mark_done_request_is_the_done_proposal() {
    assert_eq!(
        change("Mark the task Confirm pier permit as done."),
        Some((
            "propose_task_done".to_string(),
            json!({ "task": "Confirm pier permit", "done": true })
        ))
    );
}

#[test]
fn navigation_is_the_open_tools() {
    assert_eq!(
        nav("Jump to scene 12."),
        Some(("open_scene".into(), json!({ "sceneNumber": 12 })))
    );
    assert_eq!(
        nav("take me to scene 3"),
        Some(("open_scene".into(), json!({ "sceneNumber": 3 })))
    );
    for (text, w) in [
        ("Open the Idea Vault.", "vault"),
        ("Go to the story board", "story"),
        ("Go to the Beat Board", "story"),
        ("Open Call Sheets.", "callsheets"),
        ("Show my notes and tasks.", "notes"),
        ("Open Recently Deleted.", "trash"),
        ("Take me to Breakdown.", "breakdown"),
    ] {
        assert_eq!(
            nav(text),
            Some(("open_workspace".into(), json!({ "workspace": w }))),
            "{text}"
        );
    }
}

#[test]
fn plain_additions_are_their_proposals() {
    assert_eq!(
        change("Add the brass lamp to props."),
        Some((
            "propose_catalog_item".into(),
            json!({ "name": "brass lamp", "category": "Props" })
        ))
    );
    assert_eq!(
        change("Add Meena's silk saree to wardrobe"),
        Some((
            "propose_catalog_item".into(),
            json!({ "name": "Meena's silk saree", "category": "Wardrobe" })
        ))
    );
    assert_eq!(
        change("Add an auto rickshaw as a vehicle."),
        Some((
            "propose_catalog_item".into(),
            json!({ "name": "auto rickshaw", "category": "Vehicles" })
        ))
    );
    assert_eq!(
        change("Create a character named Inspector Rao, the station's new officer."),
        Some((
            "propose_character".into(),
            json!({ "name": "Inspector Rao", "description": "The station's new officer" })
        ))
    );
    assert_eq!(
        change("Add Lata as a character."),
        Some(("propose_character".into(), json!({ "name": "Lata" })))
    );
    assert_eq!(
        change("Replace 'Madurai' with 'Trichy' throughout the script."),
        Some((
            "propose_replace_text".into(),
            json!({ "find": "Madurai", "replace": "Trichy" })
        ))
    );
    assert_eq!(
        change("Make a note that the bridge scenes moved to Friday."),
        Some((
            "propose_project_note".into(),
            json!({ "body": "The bridge scenes moved to Friday" })
        ))
    );
    assert_eq!(
        change("Save a note titled Rain with the body: book two water tankers."),
        Some((
            "propose_project_note".into(),
            json!({ "title": "Rain", "body": "Book two water tankers" })
        ))
    );
    assert_eq!(
        tool("Create a moodboard called Harbour Dusk.").as_deref(),
        Some("propose_moodboard")
    );
    assert_eq!(
        tool("Create a shooting schedule called Unit B.").as_deref(),
        Some("propose_schedule")
    );
    assert_eq!(
        change("Create a storyboard for scene 7."),
        Some(("propose_storyboard".into(), json!({ "sceneNumber": 7 })))
    );
    assert_eq!(
        change("Add a location: Marina pier."),
        Some(("propose_location".into(), json!({ "name": "Marina pier" })))
    );
}

#[test]
fn production_and_story_actions_name_their_proposal_tool() {
    assert_eq!(
        change("Add a shot to scene 21: wide shot of Lakshmi on the tower."),
        Some((
            "propose_shot".into(),
            json!({ "sceneNumber": 21, "description": "Wide shot of Lakshmi on the tower" })
        ))
    );
    assert_eq!(
        change("Make a moodboard for the station at night."),
        Some((
            "propose_moodboard".into(),
            json!({ "name": "Station at night" })
        ))
    );
    assert_eq!(
        change("Make a moodboard for scene 12."),
        Some((
            "propose_moodboard".into(),
            json!({ "name": "Scene 12", "sceneNumber": 12 })
        ))
    );
    assert_eq!(
        change("Add three desks as set dressing."),
        Some((
            "propose_catalog_item".into(),
            json!({
                "name": "desks",
                "category": "Props",
                "notes": "Quantity: 3",
                "description": "Set dressing"
            })
        ))
    );
    // One item and a plain category keep the name whole and add no quantity.
    assert_eq!(
        change("Add the red kite to props."),
        Some((
            "propose_catalog_item".into(),
            json!({ "name": "red kite", "category": "Props" })
        ))
    );
    assert_eq!(
        change("Leave a comment on the draft: tighten act two."),
        Some((
            "propose_comment".into(),
            json!({ "draft": "current", "text": "Tighten act two." })
        ))
    );
    assert_eq!(
        change("Pin a note saying the cyclone scenes need rain machines."),
        Some((
            "propose_project_note".into(),
            json!({ "body": "The cyclone scenes need rain machines", "pinned": true })
        ))
    );
    assert_eq!(
        change("Create characters for this scene: Nandini."),
        Some(("propose_character".into(), json!({ "name": "Nandini" })))
    );
    // Several names are the model's to plan.
    assert_eq!(
        change("Create characters for this scene: Nandini and Rohan."),
        None
    );
}

fn tool(text: &str) -> Option<String> {
    change(text).map(|c| c.0)
}

#[test]
fn questions_are_not_commands() {
    for q in [
        "How many scenes are in the script?",
        "Who plays the clerk?",
        "Describe Karthik.",
        "What is a Change Set?",
        "Which scenes mention the lantern?",
        "How do I export a PDF?",
        "Explain scene 35.",
    ] {
        assert_eq!(change(q), None, "{q}");
        assert_eq!(nav(q), None, "{q}");
    }
}

#[test]
fn a_command_must_be_offered_to_run_and_is_one_short_line() {
    assert!(change_command("Create a task to scout the station.", |_| false).is_none());
    assert!(navigation_call("Open scene 4", |_| false).is_none());
    assert!(change_command("Create a task to scout\nthe station.", |_| true).is_none());
    // A breakdown request is a command of the suggestion tool.
    assert_eq!(
        change("Suggest breakdown elements for scene 22."),
        Some((
            "suggest_breakdown".to_string(),
            json!({ "sceneNumber": 22 })
        ))
    );
    // A navigation command is not a change.
    assert_eq!(change("Open scene 4"), None);
}
