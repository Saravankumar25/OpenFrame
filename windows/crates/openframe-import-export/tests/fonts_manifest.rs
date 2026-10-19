//! Supply chain of the bundled PDF fonts (ADR-0016): every font file the app compiles in is
//! listed in `fonts/fonts.json` with its SHA-256, size, version and licence, ships with its
//! SIL Open Font License text, and nothing unlisted sits in the folder.

use std::path::Path;

use openframe_import_export::fonts::ALL_FACES;
use serde_json::Value;

fn fonts_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts"))
}

fn manifest() -> Vec<Value> {
    let text = std::fs::read_to_string(fonts_dir().join("fonts.json")).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    v["fonts"].as_array().unwrap().clone()
}

#[test]
fn every_bundled_font_matches_its_manifest_hash_and_licence() {
    let entries = manifest();
    assert_eq!(entries.len(), ALL_FACES.len(), "one entry per bundled face");
    for face in ALL_FACES {
        let e = entries
            .iter()
            .find(|e| e["file"] == face.file_name())
            .unwrap_or_else(|| panic!("{} is not in fonts.json", face.file_name()));
        let data = face.data();
        assert_eq!(
            openframe_security::sha256_bytes(data),
            e["sha256"].as_str().unwrap(),
            "{}: the compiled font differs from the reviewed file",
            face.file_name()
        );
        assert_eq!(data.len() as u64, e["bytes"].as_u64().unwrap());
        assert_eq!(e["license"], "OFL-1.1");
        let licence =
            std::fs::read_to_string(fonts_dir().join(e["licenseFile"].as_str().unwrap())).unwrap();
        assert!(licence.contains("SIL OPEN FONT LICENSE Version 1.1"));
        assert!(
            e["source"]
                .as_str()
                .unwrap()
                .starts_with("https://github.com/"),
            "{e}"
        );
    }
}

#[test]
fn the_fonts_folder_holds_only_listed_files() {
    let entries = manifest();
    let mut listed: Vec<String> = entries
        .iter()
        .flat_map(|e| {
            [
                e["file"].as_str().unwrap().to_string(),
                e["licenseFile"].as_str().unwrap().to_string(),
            ]
        })
        .collect();
    listed.push("fonts.json".into());
    for entry in std::fs::read_dir(fonts_dir()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(listed.contains(&name), "unlisted file in fonts/: {name}");
    }
}
