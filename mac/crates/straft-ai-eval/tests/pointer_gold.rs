//! The out-of-section pointer (ADR-0041) on the section bench, with no model: the answer type comes
//! from the rules, the facts from the picked sections, and the pointer is code. These are the same
//! numbers `ai-eval --dossier-only` reports, kept here so they hold without the engine. With a search
//! model the type read can only get better; the writer plays no part.

use std::path::PathBuf;
use std::sync::Arc;

use straft_ai_eval::turn::{from_cases, index_of, picks_of, read_conv_resolved};
use straft_ai_eval::{Case, PointerCheck, PointerMetrics, load_cases};
use straft_application::core::{AppConfig, AppCore};
use straft_application::events::RecordingSink;
use straft_application::modules::ai::turn::Aids;
use straft_application::modules::ai::{dossier, orchestrator, pointer, sections};

fn checks() -> Vec<(String, String, PointerCheck)> {
    let dir = tempfile::tempdir().unwrap();
    let core = AppCore::new(
        AppConfig::isolated(dir.path()),
        Arc::new(RecordingSink::default()),
    )
    .unwrap();
    let gold = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gold");
    let cases: Vec<Case> = load_cases(&gold)
        .unwrap()
        .into_iter()
        .filter(|c| c.id.starts_with("sec-"))
        .collect();
    let convs = from_cases(&cases);
    // Every case of the bench reads one fixture, built once.
    let project = &convs[0].project;
    assert!(convs.iter().all(|c| c.project == *project));
    let ix = index_of(&core, project).unwrap();
    let session = core.project().unwrap();
    let actor = core.local_actor();
    let mut out = Vec::new();
    for conv in &convs {
        let reads = read_conv_resolved(&ix, conv, &Aids::none());
        for m in &conv.measured {
            let case = cases.iter().find(|c| c.id == m.id).unwrap();
            let (resolved, read, focus) = &reads[m.turn];
            let read = &orchestrator::hinted(read.clone(), &conv.turns[m.turn]);
            let message = &conv.turns[m.turn];
            // Episodes are refused on a feature film: a request with that pick runs nothing.
            if case
                .sections
                .as_ref()
                .unwrap()
                .iter()
                .any(|p| p.leaf == "story/episodes")
            {
                continue;
            }
            let sel = picks_of(&core, case, &ix).unwrap();
            let retrieved = sections::retrieve(&core, &session, &actor, &sel, message);
            let leaf_read = dossier::LeafRead {
                selection: &sel,
                passages: &retrieved.passages,
            };
            let got = session
                .store
                .read(|c| {
                    let input = dossier::Input {
                        conn: c,
                        actor: &actor,
                        index: &ix,
                        core: Some(&core),
                        message,
                        read,
                        resolved,
                        focus: Some(focus),
                        previous_message: m.turn.checked_sub(1).map(|i| conv.turns[i].as_str()),
                        sections: &leaf_read,
                    };
                    let d = dossier::build(&input);
                    let v = pointer::decide(&input, &sel, &d);
                    if std::env::var_os("POINTER_DEBUG").is_some() {
                        let kinds: Vec<&str> = d.facts.iter().map(|f| f.kind).collect();
                        eprintln!(
                            "{} {:?} {:?} {:?} facts {:?} outside {:?} missing {:?}\n   {}",
                            case.id,
                            sel.codes(),
                            message,
                            v,
                            kinds,
                            d.outside.iter().map(|o| o.name.clone()).collect::<Vec<_>>(),
                            d.missing,
                            d.render_plain()
                                .chars()
                                .take(500)
                                .collect::<String>()
                                .replace('\n', " // ")
                        );
                    }
                    Ok(v)
                })
                .unwrap();
            out.push((
                case.id.clone(),
                format!("{} [{}]", message, read.answer_type.code()),
                PointerCheck {
                    want: case.expect.pointer.clone(),
                    got: got.pointer().map(|p| p.leaf.id.to_string()),
                },
            ));
        }
    }
    out
}

#[test]
fn the_pointer_names_the_right_section_and_stays_quiet_when_the_pick_fits() {
    let checks = checks();
    let mut m = PointerMetrics::default();
    for (id, message, c) in &checks {
        m.add(c);
        if c.want != c.got {
            eprintln!("{id}: {message}: want {:?}, got {:?}", c.want, c.got);
        }
    }
    eprintln!("{}", m.table());
    assert_eq!(m.expected, 60);
    assert!(m.quiet_cases >= 300);
    assert!(m.recall().unwrap() >= 90.0, "recall {:?}", m.recall());
    assert!(
        m.precision().unwrap() >= 90.0,
        "precision {:?}",
        m.precision()
    );
    assert!(
        m.false_rate().unwrap() <= 2.0,
        "false rate {:?}",
        m.false_rate()
    );
}
