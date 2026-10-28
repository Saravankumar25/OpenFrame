//! Appearance and themes (build plan area "Appearance and themes"; ADR-0014; features THEME-001
//! to THEME-008): preferences in the app database, My Themes, and the theme importer as an
//! untrusted parser. Every hostile input must change nothing: no preference, no saved theme, no
//! project.

use openframe_application::modules::appearance::theme::{self, MAX_THEME_BYTES};
use openframe_application::registry::{self, AiExposure};
use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const VALID: &str = r#"{
  "formatVersion": 1,
  "id": "harbour-night",
  "name": "Harbour Night",
  "description": "Blue-grey surfaces with a lamp-orange accent.",
  "author": "Test Author",
  "seeds": { "neutralHue": 230, "neutralTint": 0.02, "accentHue": 55, "accentChroma": 0.14, "accentLightnessNudge": 0 },
  "overrides": { "dark": { "accent": "oklch(74% 0.14 55)" } }
}"#;

fn state(env: &TestEnv) -> Value {
    env.ok("appearance.get", json!({}))
}

fn my_ids(env: &TestEnv) -> Vec<String> {
    state(env)["myThemes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect()
}

/// Everything a failed import must leave unchanged.
fn snapshot(env: &TestEnv) -> (Value, i64) {
    let rows = env
        .core
        .with_app_db(|c| Ok(c.query_row("SELECT COUNT(*) FROM sys_user_theme", [], |r| r.get(0))?))
        .unwrap();
    (state(env), rows)
}

fn with_seed(field: &str, value: &str) -> String {
    VALID.replacen(
        &format!("\"{field}\": "),
        &format!("\"{field}\": {value}, \"_was\": "),
        1,
    )
}

#[test]
fn a_new_installation_shows_openframe_default_in_system_mode() {
    let env = TestEnv::new();
    let s = state(&env);
    assert_eq!(s["mode"], "system");
    assert_eq!(s["themeId"], "openframe-default");
    assert!(s["fallbackNotice"].is_null());
    assert_eq!(s["myThemes"], json!([]));
}

#[test]
fn mode_and_theme_survive_a_restart() {
    let env = TestEnv::new();
    env.ok("appearance.set_mode", json!({ "mode": "dark" }));
    env.ok(
        "appearance.set_theme",
        json!({ "themeId": "midnight-reel" }),
    );
    let env = env.restart();
    let s = state(&env);
    assert_eq!(s["mode"], "dark");
    assert_eq!(s["themeId"], "midnight-reel");
    env.ok("appearance.set_mode", json!({ "mode": "light" }));
    env.ok("appearance.set_mode", json!({ "mode": "system" }));
    assert_eq!(state(&env.restart())["mode"], "system");
}

#[test]
fn every_builtin_theme_can_be_applied() {
    let env = TestEnv::new();
    for t in theme::builtin_themes() {
        let s = env.ok("appearance.set_theme", json!({ "themeId": t.id }));
        assert_eq!(s["themeId"], t.id.as_str());
    }
}

#[test]
fn an_unknown_theme_or_mode_is_refused_and_changes_nothing() {
    let env = TestEnv::new();
    env.ok("appearance.set_theme", json!({ "themeId": "slate-noir" }));
    let before = snapshot(&env);
    for id in ["no-such-theme", "", "SLATE-NOIR", "../x", "a"] {
        assert_eq!(
            env.err("appearance.set_theme", json!({ "themeId": id })),
            "not_found.theme",
            "{id}"
        );
    }
    assert_eq!(
        env.err("appearance.set_mode", json!({ "mode": "sepia" })),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err("appearance.set_mode", json!({ "mode": "dark", "x": 1 })),
        "validation.invalid_input"
    );
    assert_eq!(snapshot(&env), before);
}

#[test]
fn import_from_text_preview_save_apply_and_restart() {
    let env = TestEnv::new();
    let check = env.ok("appearance.check_theme", json!({ "text": VALID }));
    assert_eq!(check["status"], "ok", "{check}");
    assert_eq!(check["theme"]["name"], "Harbour Night");
    assert_eq!(check["theme"]["dark"]["accent"], "oklch(74% 0.14 55)");
    assert_eq!(
        check["contrast"].as_array().unwrap().len(),
        2 * theme::PAIRS.len()
    );
    // Checking saves nothing.
    assert!(my_ids(&env).is_empty());
    let s = env.ok(
        "appearance.import_theme",
        json!({ "text": VALID, "apply": true }),
    );
    assert_eq!(s["themeId"], "harbour-night");
    let env = env.restart();
    let s = state(&env);
    assert_eq!(s["themeId"], "harbour-night");
    assert_eq!(s["myThemes"][0]["theme"]["seeds"]["neutralHue"], 230.0);
    assert!(s["myThemes"][0]["problem"].is_null());
}

#[test]
fn import_from_a_file_with_a_byte_order_mark() {
    let env = TestEnv::new();
    let mut bytes = "\u{feff}".as_bytes().to_vec();
    bytes.extend_from_slice(VALID.as_bytes());
    let path = env.write_file("harbour.json", &bytes);
    let check = env.ok("appearance.check_theme", json!({ "path": path }));
    assert_eq!(check["status"], "ok", "{check}");
    env.ok(
        "appearance.import_theme",
        json!({ "path": path, "apply": false }),
    );
    assert_eq!(my_ids(&env), ["harbour-night"]);
    assert_eq!(
        state(&env)["themeId"],
        "openframe-default",
        "saving without Apply keeps the theme"
    );
}

#[test]
fn the_ten_kilobyte_limit_is_exact() {
    let env = TestEnv::new();
    let pad = MAX_THEME_BYTES - VALID.len();
    let at_limit = format!("{VALID}{}", " ".repeat(pad));
    assert_eq!(at_limit.len(), MAX_THEME_BYTES);
    assert_eq!(
        env.ok("appearance.check_theme", json!({ "text": at_limit }))["status"],
        "ok"
    );
    let over = format!("{at_limit} ");
    let c = env.ok("appearance.check_theme", json!({ "text": over }));
    assert_eq!(c["status"], "rejected");
    assert!(c["problems"][0].as_str().unwrap().contains("10 KB"), "{c}");
    // A large file is refused without reading it whole.
    let big = env.write_file("big.json", &vec![b' '; 5 * 1024 * 1024]);
    let c = env.ok("appearance.check_theme", json!({ "path": big }));
    assert_eq!(c["status"], "rejected");
    assert!(c["problems"][0].as_str().unwrap().contains("10 KB"), "{c}");
}

/// Hostile and malformed inputs (docs/engineering/30-appearance-themes.md section 6).
#[test]
fn security_hostile_theme_files_are_refused_and_change_nothing() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    env.ok(
        "appearance.import_theme",
        json!({ "text": VALID, "apply": false }),
    );
    env.ok("appearance.set_theme", json!({ "themeId": "cutting-room" }));
    env.ok("appearance.set_mode", json!({ "mode": "dark" }));
    let project_before = env.ok("project.current", json!({}));
    let before = snapshot(&env);

    let deep = format!("{}1{}", "[".repeat(5000), "]".repeat(5000));
    let cases: Vec<(&str, String)> = vec![
        (
            "unknown key",
            VALID.replacen("\"author\"", "\"script\": \"alert(1)\", \"author\"", 1),
        ),
        (
            "unknown seed",
            VALID.replacen("\"neutralHue\"", "\"glow\": 1, \"neutralHue\"", 1),
        ),
        (
            "unknown override mode",
            VALID.replacen("\"dark\":", "\"dim\":", 1),
        ),
        (
            "bad id",
            VALID.replacen("harbour-night", "Harbour Night", 1),
        ),
        ("short id", VALID.replacen("harbour-night", "hn", 1)),
        (
            "long id",
            VALID.replacen("harbour-night", &"h".repeat(41), 1),
        ),
        (
            "id with path",
            VALID.replacen("harbour-night", "../../etc", 1),
        ),
        (
            "id with quote",
            VALID.replacen("harbour-night", "x\\\"]{}", 1),
        ),
        (
            "built-in id",
            VALID.replacen("harbour-night", "openframe-default", 1),
        ),
        (
            "built-in id 2",
            VALID.replacen("harbour-night", "high-contrast", 1),
        ),
        ("duplicate custom id", VALID.to_string()),
        ("invalid json", VALID.replacen('}', "", 1)),
        ("not an object", "[1,2,3]".into()),
        ("string", "\"theme\"".into()),
        ("empty", "".into()),
        ("whitespace", "   \n ".into()),
        ("deep nesting", deep),
        ("css text", ":root{--bg:red}".into()),
        ("non-numeric seed", VALID.replacen("230", "\"230\"", 1)),
        ("seed out of range", VALID.replacen("230", "361", 1)),
        ("negative tint", VALID.replacen("0.02", "-0.02", 1)),
        ("huge chroma", VALID.replacen("0.14,", "4,", 1)),
        (
            "nudge out of range",
            VALID.replacen(
                "\"accentLightnessNudge\": 0",
                "\"accentLightnessNudge\": 40",
                1,
            ),
        ),
        ("NaN", VALID.replacen("230", "NaN", 1)),
        ("Infinity", VALID.replacen("230", "Infinity", 1)),
        ("overflow", VALID.replacen("230", "1e999", 1)),
        ("null seed", VALID.replacen("230", "null", 1)),
        (
            "missing seed",
            VALID.replacen("\"neutralHue\": 230, ", "", 1),
        ),
        ("missing seeds", with_seed("seeds", "null")),
        (
            "format version 2",
            VALID.replacen("\"formatVersion\": 1", "\"formatVersion\": 2", 1),
        ),
        (
            "format version text",
            VALID.replacen("\"formatVersion\": 1", "\"formatVersion\": \"1\"", 1),
        ),
        (
            "format version 0",
            VALID.replacen("\"formatVersion\": 1", "\"formatVersion\": 0", 1),
        ),
        (
            "url()",
            VALID.replacen("oklch(74% 0.14 55)", "url(https://example.com/x.png)", 1),
        ),
        (
            "@import",
            VALID.replacen("oklch(74% 0.14 55)", "@import url(x.css)", 1),
        ),
        (
            "semicolon",
            VALID.replacen("oklch(74% 0.14 55)", "#fff; background: red", 1),
        ),
        (
            "braces",
            VALID.replacen("oklch(74% 0.14 55)", "#fff} body{display:none", 1),
        ),
        (
            "var()",
            VALID.replacen("oklch(74% 0.14 55)", "var(--ink)", 1),
        ),
        (
            "expression",
            VALID.replacen("oklch(74% 0.14 55)", "expression(alert(1))", 1),
        ),
        (
            "calc in oklch",
            VALID.replacen("oklch(74% 0.14 55)", "oklch(calc(50%) 0.1 10)", 1),
        ),
        (
            "named colour",
            VALID.replacen("oklch(74% 0.14 55)", "red", 1),
        ),
        (
            "rgb()",
            VALID.replacen("oklch(74% 0.14 55)", "rgb(255,0,0)", 1),
        ),
        (
            "html in value",
            VALID.replacen("oklch(74% 0.14 55)", "<script>alert(1)</script>", 1),
        ),
        (
            "number value",
            VALID.replacen("\"oklch(74% 0.14 55)\"", "12", 1),
        ),
        (
            "not allowed token",
            VALID.replacen("\"accent\": \"oklch", "\"paper\": \"oklch", 1),
        ),
        (
            "unknown token",
            VALID.replacen("\"accent\": \"oklch", "\"font-family\": \"oklch", 1),
        ),
        (
            "builtin-only token",
            VALID.replacen("\"accent\": \"oklch", "\"backdrop\": \"oklch", 1),
        ),
        (
            "role token",
            VALID.replacen("\"accent\": \"oklch", "\"beat-ink\": \"oklch", 1),
        ),
        (
            "eight-digit hex",
            VALID.replacen("oklch(74% 0.14 55)", "#ff000080", 1),
        ),
        (
            "html name",
            VALID.replacen("Harbour Night", "<img src=x onerror=alert(1)>", 1),
        ),
        (
            "control character name",
            VALID.replacen("Harbour Night", "Harbour\\u0007Night", 1),
        ),
        (
            "bidi name",
            VALID.replacen("Harbour Night", "Harbour \u{202e}thgiN", 1),
        ),
        (
            "long name",
            VALID.replacen("Harbour Night", &"N".repeat(61), 1),
        ),
        ("empty name", VALID.replacen("Harbour Night", "  ", 1)),
        (
            "name not text",
            VALID.replacen("\"Harbour Night\"", "[\"Harbour\"]", 1),
        ),
        (
            "long description",
            VALID.replacen(
                "Blue-grey surfaces with a lamp-orange accent.",
                &"d".repeat(201),
                1,
            ),
        ),
        (
            "contrast failure",
            VALID.replacen(
                "{ \"dark\": { \"accent\": \"oklch(74% 0.14 55)\" } }",
                "{ \"light\": { \"muted\": \"#d0d0d0\" } }",
                1,
            ),
        ),
        (
            "unreadable text",
            VALID.replacen(
                "{ \"dark\": { \"accent\": \"oklch(74% 0.14 55)\" } }",
                "{ \"dark\": { \"ink\": \"#333333\" } }",
                1,
            ),
        ),
    ];
    for (what, text) in &cases {
        let c = env.ok("appearance.check_theme", json!({ "text": text }));
        assert_eq!(c["status"], "rejected", "{what}: {c}");
        assert!(!c["problems"].as_array().unwrap().is_empty(), "{what}");
        let e = env.err(
            "appearance.import_theme",
            json!({ "text": text, "apply": true }),
        );
        assert_eq!(e, "validation.invalid_input", "{what}");
        assert_eq!(snapshot(&env), before, "{what} changed something");
    }
    // Problems never echo control or direction characters back.
    let c = env.ok(
        "appearance.check_theme",
        json!({ "text": VALID.replacen("harbour-night", "x\u{202e}y", 1) }),
    );
    assert!(!c.to_string().contains('\u{202e}'));
    // Hostile files through the file path too.
    for (i, (what, text)) in cases.iter().enumerate().take(12) {
        let path = env.write_file(&format!("hostile-{i}.json"), text.as_bytes());
        assert_eq!(
            env.ok("appearance.check_theme", json!({ "path": path }))["status"],
            "rejected",
            "{what}"
        );
    }
    let binary = env.write_file("binary.json", &[0xff, 0xfe, 0x00, 0x7b, 0x00]);
    assert_eq!(
        env.ok("appearance.check_theme", json!({ "path": binary }))["status"],
        "rejected"
    );
    let folder = env.dir.path().join("inputs").to_string_lossy().into_owned();
    assert_eq!(
        env.ok("appearance.check_theme", json!({ "path": folder }))["status"],
        "rejected"
    );
    let missing = env
        .dir
        .path()
        .join("nope.json")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        env.ok("appearance.check_theme", json!({ "path": missing }))["status"],
        "rejected"
    );
    // Both or neither source.
    assert_eq!(
        env.ok("appearance.check_theme", json!({}))["status"],
        "rejected"
    );
    assert_eq!(
        env.ok(
            "appearance.check_theme",
            json!({ "text": VALID, "path": binary })
        )["status"],
        "rejected"
    );
    assert_eq!(
        env.err("appearance.import_theme", json!({ "text": VALID })),
        "validation.invalid_input",
        "apply is required"
    );
    assert_eq!(snapshot(&env), before);
    assert_eq!(
        env.ok("project.current", json!({})),
        project_before,
        "the open project is untouched"
    );
}

#[test]
fn security_review_findings_names_paths_actors_and_stale_previews() {
    let env = TestEnv::new();
    let before = snapshot(&env);
    // Invisible, filler or built-in names.
    for name in [
        "\u{200b}\u{2060}",
        "\u{3164}",
        "Harbour\u{feff}Night",
        "Harbour\u{061c}Night",
        "Night\u{2028}Bus",
        "***",
        "OpenFrame Default",
        "high contrast",
    ] {
        let text = VALID.replacen("Harbour Night", name, 1);
        let c = env.ok("appearance.check_theme", json!({ "text": text }));
        assert_eq!(c["status"], "rejected", "{name:?}: {c}");
    }
    // A theme file on a network share is refused without opening it.
    let c = env.ok(
        "appearance.check_theme",
        json!({ "path": r"\\nas.example\themes\t.json" }),
    );
    assert_eq!(c["status"], "rejected");
    assert!(
        c["problems"][0]
            .as_str()
            .unwrap()
            .contains("Copy the theme file")
    );
    // Only the person at this computer reads or changes themes.
    let other = env.other_user(Role::Owner);
    for (op, args) in [
        ("appearance.check_theme", json!({ "text": VALID })),
        (
            "appearance.rename_theme",
            json!({ "id": "harbour-night", "name": "X" }),
        ),
        ("appearance.delete_theme", json!({ "id": "harbour-night" })),
        (
            "appearance.export_theme",
            json!({ "id": "harbour-night", "path": "C:/x.json" }),
        ),
    ] {
        let e = env.call_as(&other, op, args).unwrap_err();
        assert_eq!(e.code_str(), "permission.denied", "{op}");
    }
    // A long list of problems is bounded.
    let many: String = (0..400).map(|i| format!("\"k{i}\":1,")).collect();
    let c = env.ok(
        "appearance.check_theme",
        json!({ "text": format!("{{{many}\"formatVersion\":1}}") }),
    );
    assert!(c["problems"].as_array().unwrap().len() <= 21, "{c}");
    assert_eq!(snapshot(&env), before);
    // The saved theme is the one previewed: a changed source is refused.
    let path = env.write_file("harbour.json", VALID.as_bytes());
    let digest = env.ok("appearance.check_theme", json!({ "path": path }))["digest"].clone();
    std::fs::write(&path, VALID.replacen("Harbour Night", "Harbour Day", 1)).unwrap();
    let e = env.err(
        "appearance.import_theme",
        json!({ "path": path, "apply": true, "expectedDigest": digest }),
    );
    assert_eq!(e, "validation.invalid_input");
    assert_eq!(snapshot(&env), before);
    let digest = env.ok("appearance.check_theme", json!({ "path": path }))["digest"].clone();
    env.ok(
        "appearance.import_theme",
        json!({ "path": path, "apply": true, "expectedDigest": digest }),
    );
    assert_eq!(state(&env)["myThemes"][0]["name"], "Harbour Day");
}

#[test]
fn a_rename_to_a_built_in_name_is_refused_and_the_theme_stays_usable() {
    // Found in the security review: a rename to a built-in name used to succeed, and the next read
    // then marked the theme as damaged (and fell back to OpenFrame Default when it was active).
    let env = TestEnv::new();
    env.ok(
        "appearance.import_theme",
        json!({ "text": VALID, "apply": true }),
    );
    for name in ["High Contrast", "  openframe   DEFAULT ", "Studio Amber"] {
        let e = env.err(
            "appearance.rename_theme",
            json!({ "id": "harbour-night", "name": name }),
        );
        assert_eq!(e, "validation.invalid_input", "{name:?}");
    }
    let s = state(&env.restart());
    assert_eq!(s["themeId"], "harbour-night");
    assert_eq!(s["myThemes"][0]["name"], "Harbour Night");
    assert!(s["myThemes"][0]["problem"].is_null(), "{s}");
}

#[test]
fn a_corrected_import_always_survives_being_read_back() {
    // Sweep seeds: whatever the check accepts (after corrections) must still pass when the stored,
    // rounded values are validated again on the next read.
    let env = TestEnv::new();
    let mut accepted = 0;
    for (i, hue) in (0..360).step_by(23).enumerate() {
        for nudge in [-3.3333, 0.0, 4.75, 9.1] {
            let text = format!(
                r#"{{"formatVersion":1,"id":"sweep-{i}-{n}","name":"Sweep {i}","seeds":{{"neutralHue":{hue},"neutralTint":0.0213,"accentHue":{h2},"accentChroma":0.1777,"accentLightnessNudge":{nudge}}}}}"#,
                n = (nudge * 10.0_f64).abs() as i64,
                h2 = (hue * 7) % 360,
            );
            if env.ok("appearance.check_theme", json!({ "text": text }))["status"] != "rejected" {
                env.ok(
                    "appearance.import_theme",
                    json!({ "text": text, "apply": false }),
                );
                accepted += 1;
            }
        }
    }
    assert!(
        accepted > 10,
        "the sweep must exercise saved themes ({accepted})"
    );
    let s = state(&env.restart());
    for t in s["myThemes"].as_array().unwrap() {
        assert!(t["problem"].is_null(), "{t}");
    }
}

#[test]
fn small_readability_problems_are_corrected_and_said() {
    let env = TestEnv::new();
    // White button text on this accent is 4.43 to 1 in Light mode.
    let text = r#"{"formatVersion":1,"id":"deep-navy","name":"Deep Navy","seeds":{"neutralHue":255,"neutralTint":0.03,"accentHue":238,"accentChroma":0.15,"accentLightnessNudge":0}}"#;
    let c = env.ok("appearance.check_theme", json!({ "text": text }));
    assert_eq!(c["status"], "corrected", "{c}");
    assert_eq!(c["notes"].as_array().unwrap().len(), 1);
    assert_eq!(c["theme"]["light"]["accent"], "oklch(55% 0.15 238)");
    env.ok(
        "appearance.import_theme",
        json!({ "text": text, "apply": true }),
    );
    let s = state(&env);
    assert_eq!(
        s["myThemes"][0]["theme"]["light"]["accent"],
        "oklch(55% 0.15 238)"
    );
}

#[test]
fn indian_script_names_are_kept() {
    let env = TestEnv::new();
    let te = VALID.replacen("Harbour Night", "రాత్రి రైలు", 1);
    env.ok(
        "appearance.import_theme",
        json!({ "text": te, "apply": false }),
    );
    env.ok(
        "appearance.rename_theme",
        json!({ "id": "harbour-night", "name": "रात की बस" }),
    );
    let env = env.restart();
    assert_eq!(state(&env)["myThemes"][0]["name"], "रात की बस");
}

#[test]
fn rename_export_reimport_and_delete() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    env.ok(
        "appearance.import_theme",
        json!({ "text": VALID, "apply": true }),
    );
    env.ok(
        "appearance.rename_theme",
        json!({ "id": "harbour-night", "name": "  Harbour Lamps " }),
    );
    assert_eq!(state(&env)["myThemes"][0]["name"], "Harbour Lamps");
    for bad in ["", "   ", "<b>x</b>", &"x".repeat(61)] {
        assert_eq!(
            env.err(
                "appearance.rename_theme",
                json!({ "id": "harbour-night", "name": bad })
            ),
            "validation.invalid_input",
            "{bad}"
        );
    }
    assert_eq!(
        env.err(
            "appearance.rename_theme",
            json!({ "id": "studio-amber", "name": "Mine" })
        ),
        "not_found.theme"
    );

    let out = env.dir.path().join("exports").join("Harbour Lamps");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let r = env.ok(
        "appearance.export_theme",
        json!({ "id": "harbour-night", "path": out.to_string_lossy() }),
    );
    assert_eq!(r["fileName"], "Harbour Lamps.json");
    let written = std::fs::read_to_string(r["path"].as_str().unwrap()).unwrap();
    let file: Value = serde_json::from_str(&written).unwrap();
    assert_eq!(file["name"], "Harbour Lamps");
    assert_eq!(file["formatVersion"], 1);
    let mut keys: Vec<&str> = file
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "author",
            "description",
            "formatVersion",
            "id",
            "name",
            "overrides",
            "seeds"
        ]
    );
    // Exporting into OpenFrame's own folders is refused.
    let inside = env.dir.path().join("appdata").join("theme.json");
    assert!(
        env.call(
            "appearance.export_theme",
            json!({ "id": "harbour-night", "path": inside.to_string_lossy() })
        )
        .is_err()
    );
    // A folder that does not exist: nothing is written.
    let nowhere = env.dir.path().join("missing").join("t.json");
    assert!(
        env.call(
            "appearance.export_theme",
            json!({ "id": "harbour-night", "path": nowhere.to_string_lossy() })
        )
        .is_err()
    );
    assert!(!nowhere.exists());

    // Deleting the active theme falls back to OpenFrame Default at once; the project is untouched.
    let s = env.ok("appearance.delete_theme", json!({ "id": "harbour-night" }));
    assert_eq!(s["themeId"], "openframe-default");
    assert!(s["fallbackNotice"].is_null());
    assert_eq!(s["myThemes"], json!([]));
    assert_eq!(
        env.err("appearance.delete_theme", json!({ "id": "harbour-night" })),
        "not_found.theme"
    );
    assert_eq!(env.ok("project.current", json!({}))["title"], "Night Bus");

    // The exported file imports again to the same theme.
    let s = env.ok(
        "appearance.import_theme",
        json!({ "path": r["path"], "apply": true }),
    );
    assert_eq!(s["themeId"], "harbour-night");
    assert_eq!(s["myThemes"][0]["name"], "Harbour Lamps");
    assert_eq!(
        s["myThemes"][0]["theme"]["dark"]["accent"],
        "oklch(74% 0.14 55)"
    );
}

#[test]
fn deleting_an_inactive_theme_keeps_the_active_one() {
    let env = TestEnv::new();
    env.ok(
        "appearance.import_theme",
        json!({ "text": VALID, "apply": false }),
    );
    let other = VALID.replacen("harbour-night", "harbour-day", 1);
    env.ok(
        "appearance.import_theme",
        json!({ "text": other, "apply": true }),
    );
    let s = env.ok("appearance.delete_theme", json!({ "id": "harbour-night" }));
    assert_eq!(s["themeId"], "harbour-day");
    assert_eq!(my_ids(&env), ["harbour-day"]);
}

#[test]
fn a_damaged_or_missing_saved_theme_falls_back_to_default() {
    let env = TestEnv::new();
    env.ok(
        "appearance.import_theme",
        json!({ "text": VALID, "apply": true }),
    );
    // Damage the stored row the way an edit outside OpenFrame could.
    env.core
        .with_app_db(|c| {
            c.execute(
                "UPDATE sys_user_theme SET overrides_json='{\"dark\":{\"accent\":\"url(x)\"}}'",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let env = env.restart();
    let s = state(&env);
    assert_eq!(s["themeId"], "openframe-default");
    assert!(
        s["fallbackNotice"]
            .as_str()
            .unwrap()
            .contains("OpenFrame Default")
    );
    assert!(s["myThemes"][0]["theme"].is_null());
    assert!(s["myThemes"][0]["problem"].is_string());
    assert_eq!(
        env.err(
            "appearance.set_theme",
            json!({ "themeId": "harbour-night" })
        ),
        "not_found.theme"
    );
    assert!(env.call("appearance.export_theme", json!({ "id": "harbour-night", "path": env.dir.path().join("x.json").to_string_lossy() })).is_err());
    // It can still be deleted, which clears the notice.
    let s = env.ok("appearance.delete_theme", json!({ "id": "harbour-night" }));
    assert!(s["fallbackNotice"].is_null());

    // A preference naming a theme that no longer exists.
    env.core
        .with_app_db(|c| {
            c.execute("UPDATE sys_appearance_pref SET value='gone-theme' WHERE key='theme'", [])?;
            c.execute("INSERT OR REPLACE INTO sys_appearance_pref(key, value, updated_at) VALUES ('mode', 'sepia', 0)", [])?;
            Ok(())
        })
        .unwrap();
    let s = state(&env);
    assert_eq!(s["themeId"], "openframe-default");
    assert_eq!(
        s["mode"], "system",
        "an unknown stored mode reads as System"
    );
    assert!(s["fallbackNotice"].is_string());
}

#[test]
fn only_the_person_at_this_computer_changes_the_appearance() {
    let env = TestEnv::new();
    let other = env.other_user(Role::Owner);
    for (op, args) in [
        ("appearance.set_mode", json!({ "mode": "dark" })),
        ("appearance.set_theme", json!({ "themeId": "slate-noir" })),
        (
            "appearance.import_theme",
            json!({ "text": VALID, "apply": true }),
        ),
    ] {
        let e = env.call_as(&other, op, args).unwrap_err();
        assert_eq!(e.code_str(), "permission.denied", "{op}");
    }
    assert_eq!(state(&env)["mode"], "system");
}

#[test]
fn my_themes_has_a_size_limit() {
    let env = TestEnv::new();
    for i in 0..openframe_application::modules::appearance::MAX_USER_THEMES {
        let t = VALID.replacen("harbour-night", &format!("theme-{i:03}"), 1);
        env.ok(
            "appearance.import_theme",
            json!({ "text": t, "apply": false }),
        );
    }
    let t = VALID.replacen("harbour-night", "one-too-many", 1);
    assert_eq!(
        env.err(
            "appearance.import_theme",
            json!({ "text": t, "apply": false })
        ),
        "validation.invalid_input"
    );
}

#[test]
fn appearance_operations_are_hidden_from_the_assistant() {
    let cat = registry::catalog();
    let ops: Vec<&str> = cat
        .op_names()
        .into_iter()
        .filter(|n| n.starts_with("appearance."))
        .collect();
    assert_eq!(ops.len(), 8);
    for op in ops {
        let m = cat
            .metadata(op)
            .unwrap_or_else(|| panic!("{op} has metadata"));
        // Reading the mode and the theme is exposed (`app_appearance`, slice S2); every command that
        // changes them stays hidden.
        if op == "appearance.get" {
            assert!(!matches!(m.ai_exposure, AiExposure::Hidden(_)), "{op}");
        } else {
            assert!(matches!(m.ai_exposure, AiExposure::Hidden(_)), "{op}");
        }
        assert!(m.required_capability.is_none(), "{op} is application level");
    }
    let del = cat.metadata("appearance.delete_theme").unwrap();
    assert!(del.destructive && del.irreversible && del.confirmation);
}
