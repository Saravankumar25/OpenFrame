//! The element rule table is shared with the editor: this test writes
//! `apps/desktop/src/workspaces/screenplay/editor/elementRules.json` from
//! [`openframe_import_export::rules::RULES`], so the editor's labels,
//! shortcuts, line-break rules and Enter progression can never drift from the
//! page layout and the FDX mapping (contract doc 32 §2.2). The editor's own
//! test checks its element metadata against the same file.

use openframe_import_export::rules::RULES;

#[test]
fn editor_rule_file_is_generated_from_the_rule_table() {
    let mut json = serde_json::to_string_pretty(RULES).expect("rules serialise");
    json.push('\n');
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src/workspaces/screenplay/editor/elementRules.json");
    let current = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    if current != json {
        std::fs::write(&path, json.as_bytes()).expect("write elementRules.json");
    }
    let back: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(back.as_array().unwrap().len(), RULES.len());
}
