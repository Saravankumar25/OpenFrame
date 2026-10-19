//! hostile file collection hostile-input corpus for the screenplay parsers (FDX, Fountain, DOCX, PDF).
//!
//! Every committed file under `tests/fixtures/hostile/<format>/` is listed in that folder's
//! README.md and is run here through the public entry point (`import_file`, the path the
//! application uses) with an explicit expectation: a correct parse for `normal`/Unicode
//! cases, otherwise a human `import.*` error. Every case also asserts no panic, a wall-time
//! bound, and that nothing was written next to the input (parsers never extract or write).
//! Large and huge inputs are generated at test time instead of being committed.
//!
//! The binary corpus files are produced by `regenerate_hostile_corpus` (ignored; run it with
//! `cargo test -p openframe-import-export --test hostile_corpus -- --ignored` after changing
//! a generator, then review and commit the files). ZIP and package files in the same folder
//! are exercised by `openframe-security/tests/hostile_archives.rs` and
//! `openframe-application/tests/hostile_packages.rs`.
//!
//! Threats: doc 20 T6 (malicious ZIP), T6a (hostile documents), T7 (oversized input), T3
//! (traversal/symlink).

#[path = "support/hostile.rs"]
mod hostile;

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use hostile::{
    ESCAPE_MARKER, RawEntry, assert_no_escape, bounded, build_zip, corpus, corpus_root, crc32, tree,
};
use openframe_domain::AppError;
use openframe_import_export::{
    ElementKind, ExportFormat, ExportOptions, ImportOutcome, MAX_IMPORT_BYTES, MAX_TEXT_BYTES,
    SourceFormat, docx, import_bytes, import_file, import_text, pdf_import, render_bytes,
};

/// Bound for a committed (small) corpus case in a debug build.
const CASE_BOUND: Duration = Duration::from_secs(10);
/// Bound for decompression bombs and multi-megabyte generated inputs in a debug build.
const HEAVY_BOUND: Duration = Duration::from_secs(45);

// ================================================================== expectations

#[derive(Clone, Copy)]
enum Expect {
    /// Parses; the check verifies the content.
    Parses(fn(&ImportOutcome)),
    /// Fails with exactly this error code and a human message.
    Fails(&'static str),
}

/// A human error: OpenFrame code family, a sentence for the user, no parser internals.
#[track_caller]
fn assert_human(e: &AppError, case: &str) {
    let code = e.code_str();
    assert!(
        code.starts_with("import.") || code.starts_with("validation."),
        "{case}: unexpected error family {code}"
    );
    let m = &e.message;
    assert!(
        m.len() >= 20 && m.ends_with('.'),
        "{case}: not a sentence: {m:?}"
    );
    for jargon in [
        "panick",
        "unwrap",
        "os error",
        "lopdf",
        "quick_xml",
        "quick-xml",
        "ZipError",
        "{",
        "Error(",
        "0x",
    ] {
        assert!(
            !m.contains(jargon),
            "{case}: message leaks internals ({jargon}): {m:?}"
        );
    }
}

fn all_text(o: &ImportOutcome) -> String {
    let mut s = String::new();
    for sc in &o.doc.scenes {
        s.push_str(&sc.heading);
        s.push('\n');
        for e in &sc.elements {
            s.push_str(&e.text);
            s.push('\n');
        }
    }
    s
}

fn headings(o: &ImportOutcome) -> Vec<String> {
    o.doc.scenes.iter().map(|s| s.heading.clone()).collect()
}

/// Copy a corpus file into a fresh folder, import it, and check the expectation plus the
/// shared invariants (no panic, bounded time, nothing written, nothing escaped).
fn run_case(format: &str, file: &str, expect: Expect, limit: Duration) {
    let src = corpus_root().join(format).join(file);
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join(file);
    std::fs::copy(&src, &input).unwrap();
    let before = tree(dir.path());
    let case = format!("{format}/{file}");
    let r = bounded(&case, limit, || import_file(&input, None));
    assert_eq!(tree(dir.path()), before, "{case}: the parser wrote files");
    assert_no_escape(dir.path());
    match (expect, r) {
        (Expect::Parses(check), Ok(o)) => check(&o),
        (Expect::Parses(_), Err(e)) => panic!(
            "{case}: expected a parse, got {} ({}) {:?}",
            e.code_str(),
            e.message,
            e.detail
        ),
        (Expect::Fails(code), Err(e)) => {
            assert_eq!(e.code_str(), code, "{case}: {} {:?}", e.message, e.detail);
            assert_human(&e, &case);
        }
        (Expect::Fails(code), Ok(o)) => panic!(
            "{case}: expected {code}, but it parsed {} scenes",
            o.doc.scenes.len()
        ),
    }
}

/// Run every case of a format and fail if a committed file has no expectation (or the
/// table names a file that is not committed), so the corpus can never silently go unused.
fn run_format(format: &str, cases: &[(&str, Expect, Duration)]) {
    let committed: BTreeSet<String> = corpus(format).into_iter().map(|(n, _)| n).collect();
    let tabled: BTreeSet<String> = cases.iter().map(|(n, _, _)| n.to_string()).collect();
    assert_eq!(
        committed, tabled,
        "{format}: every committed corpus file needs exactly one expectation"
    );
    for (file, expect, limit) in cases {
        run_case(format, file, *expect, *limit);
    }
}

// ================================================================== FDX

fn fdx_normal(o: &ImportOutcome) {
    assert_eq!(o.format, SourceFormat::Fdx);
    assert_eq!(
        headings(o),
        [
            "INT. RAILWAY PLATFORM - NIGHT",
            "EXT. STATION ROAD - CONTINUOUS"
        ]
    );
    assert!(all_text(o).contains("MEERA steps off & looks around."));
}

fn no_entity_expansion(o: &ImportOutcome) {
    let t = all_text(o);
    assert!(
        t.len() < 2_000,
        "entities were expanded ({} bytes)",
        t.len()
    );
    assert!(!t.contains("lollol"), "billion-laughs entity expanded");
    // win.ini always has a [fonts] or "for 16-bit app support" line; none of it may appear.
    assert!(
        !t.contains("[fonts]") && !t.contains("16-bit"),
        "external entity resolved"
    );
    assert_eq!(headings(o), ["INT. LAB - NIGHT"]);
}

fn fdx_unicode(o: &ImportOutcome) {
    assert_eq!(headings(o), ["INT. CAFÉ ÉTOILE - NUIT", "EXT. 東京駅 - 夜"]);
    let t = all_text(o);
    for s in [
        "抹茶",
        "שלום",
        "مرحبا",
        "e\u{301}",
        "\u{200B}",
        "🎬",
        "Ça va? 你好吗?",
        "ZOË",
        "\u{202E}",
    ] {
        assert!(t.contains(s), "lost {s:?}");
    }
}

fn fdx_control_chars(o: &ImportOutcome) {
    // Control characters inside text never break structure.
    assert_eq!(headings(o).len(), 1);
}

#[test]
fn m1_15_fdx_hostile_corpus() {
    use Expect::*;
    run_format(
        "fdx",
        &[
            ("normal.fdx", Parses(fdx_normal), CASE_BOUND),
            (
                "malformed_unclosed.fdx",
                Fails("import.fdx_malformed"),
                CASE_BOUND,
            ),
            (
                "malformed_mismatched_tags.fdx",
                Fails("import.fdx_malformed"),
                CASE_BOUND,
            ),
            (
                "malformed_wrong_root.fdx",
                Fails("import.fdx_invalid"),
                CASE_BOUND,
            ),
            (
                "malformed_binary.fdx",
                Fails("import.fdx_malformed"),
                CASE_BOUND,
            ),
            (
                "malformed_utf16.fdx",
                Fails("import.fdx_malformed"),
                CASE_BOUND,
            ),
            ("empty_content.fdx", Fails("import.fdx_empty"), CASE_BOUND),
            (
                "entity_billion_laughs.fdx",
                Parses(no_entity_expansion),
                CASE_BOUND,
            ),
            (
                "entity_external_xxe.fdx",
                Parses(no_entity_expansion),
                CASE_BOUND,
            ),
            (
                "entity_parameter.fdx",
                Parses(no_entity_expansion),
                CASE_BOUND,
            ),
            ("nested_deep.fdx", Fails("import.fdx_malformed"), CASE_BOUND),
            ("unicode.fdx", Parses(fdx_unicode), CASE_BOUND),
            ("control_chars.fdx", Parses(fdx_control_chars), CASE_BOUND),
        ],
    );
}

// ================================================================== Fountain

fn fountain_normal(o: &ImportOutcome) {
    assert_eq!(o.format, SourceFormat::Fountain);
    assert_eq!(o.doc.title_page.title(), Some("The Last Train"));
    assert_eq!(
        headings(o),
        [
            "INT. RAILWAY PLATFORM - NIGHT",
            "EXT. STATION ROAD - CONTINUOUS"
        ]
    );
    let kinds: Vec<ElementKind> = o.doc.scenes[0].elements.iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&ElementKind::Character) && kinds.contains(&ElementKind::Dialogue));
}

fn fountain_unicode(o: &ImportOutcome) {
    assert_eq!(o.doc.title_page.title(), Some("Étoile ✦ 星"));
    assert_eq!(headings(o), ["INT. CAFÉ ÉTOILE - NUIT", "EXT. 東京駅 - 夜"]);
    let t = all_text(o);
    for s in [
        "抹茶",
        "שלום",
        "مرحبا",
        "\u{200B}",
        "🎬",
        "Ça va? 你好吗?",
        "ZOË",
        "\u{202E}",
    ] {
        assert!(t.contains(s), "lost {s:?}");
    }
}

fn boneyard_hidden(o: &ImportOutcome) {
    let t = all_text(o);
    assert!(t.contains("Visible action."));
    assert!(
        !t.contains("INT. SECRET"),
        "text after an unclosed boneyard must stay hidden"
    );
    assert!(!t.contains("innermost"));
    assert!(
        o.doc
            .warnings
            .iter()
            .any(|w| w.code == "unterminated_boneyard"),
        "the unclosed boneyard is reported"
    );
}

fn notes_bounded(o: &ImportOutcome) {
    assert_eq!(headings(o), ["INT. HOUSE - DAY"]);
}

fn legacy_encoding(o: &ImportOutcome) {
    assert!(o.doc.warnings.iter().any(|w| w.code == "legacy_encoding"));
    assert!(all_text(o).contains("Café"), "Windows-1252 é decoded");
}

fn utf16(o: &ImportOutcome) {
    assert_eq!(headings(o), ["INT. CAFÉ - DAY"]);
    assert!(all_text(o).contains("Zoë"));
}

fn storm(o: &ImportOutcome) {
    assert_eq!(headings(o), ["INT. HOUSE - DAY"]);
}

fn control_chars(o: &ImportOutcome) {
    assert!(!o.doc.scenes.is_empty());
}

fn title_only(o: &ImportOutcome) {
    // Fountain allows a title page on its own: it imports as metadata with no scenes.
    assert_eq!(o.doc.title_page.title(), Some("Only A Title Page"));
    assert!(o.doc.scenes.is_empty());
}

#[test]
fn m1_15_fountain_hostile_corpus() {
    use Expect::*;
    run_format(
        "fountain",
        &[
            ("normal.fountain", Parses(fountain_normal), CASE_BOUND),
            ("unicode.fountain", Parses(fountain_unicode), CASE_BOUND),
            (
                "nested_boneyard.fountain",
                Parses(boneyard_hidden),
                CASE_BOUND,
            ),
            ("nested_notes.fountain", Parses(notes_bounded), CASE_BOUND),
            ("emphasis_storm.fountain", Parses(storm), CASE_BOUND),
            (
                "legacy_encoding.fountain",
                Parses(legacy_encoding),
                CASE_BOUND,
            ),
            ("utf16le_bom.fountain", Parses(utf16), CASE_BOUND),
            ("control_chars.fountain", Parses(control_chars), CASE_BOUND),
            (
                "malformed_binary.fountain",
                Fails("import.not_a_screenplay"),
                CASE_BOUND,
            ),
            ("title_only.fountain", Parses(title_only), CASE_BOUND),
            ("empty.fountain", Fails("import.empty_source"), CASE_BOUND),
            (
                "whitespace_only.fountain",
                Fails("import.not_a_screenplay"),
                CASE_BOUND,
            ),
        ],
    );
}

// ================================================================== DOCX

fn docx_normal(o: &ImportOutcome) {
    assert_eq!(o.format, SourceFormat::Docx);
    assert_eq!(
        headings(o),
        [
            "INT. RAILWAY PLATFORM - NIGHT",
            "EXT. STATION ROAD - CONTINUOUS"
        ]
    );
    assert!(all_text(o).contains("Nobody came."));
}

fn docx_case_collision(o: &ImportOutcome) {
    // Only the exact part name `word/document.xml` is read, never `WORD/DOCUMENT.XML`.
    docx_normal(o);
    assert!(!all_text(o).contains("IMPOSTOR"));
}

fn docx_unicode(o: &ImportOutcome) {
    docx_normal(o);
}

#[test]
fn m1_15_docx_hostile_corpus() {
    use Expect::*;
    run_format(
        "docx",
        &[
            ("normal.docx", Parses(docx_normal), CASE_BOUND),
            ("unicode_names.docx", Parses(docx_unicode), CASE_BOUND),
            // Nothing is ever extracted from a DOCX: traversal names are inert and the
            // document still reads (assert_no_escape + the unchanged folder prove it).
            ("zip_slip.docx", Parses(docx_normal), CASE_BOUND),
            ("absolute_paths.docx", Parses(docx_normal), CASE_BOUND),
            (
                "case_collision.docx",
                Parses(docx_case_collision),
                CASE_BOUND,
            ),
            (
                "duplicate_entry.docx",
                Fails("import.unsafe_archive"),
                CASE_BOUND,
            ),
            // A symlink entry is read as its stored bytes (the link text), never followed.
            (
                "symlink_document.docx",
                Fails("import.docx_invalid"),
                CASE_BOUND,
            ),
            (
                "malformed_not_zip.docx",
                Fails("import.unsafe_archive"),
                CASE_BOUND,
            ),
            (
                "malformed_truncated.docx",
                Fails("import.unsafe_archive"),
                CASE_BOUND,
            ),
            (
                "malformed_xml.docx",
                Fails("import.docx_invalid"),
                CASE_BOUND,
            ),
            (
                "missing_document.docx",
                Fails("import.docx_invalid"),
                CASE_BOUND,
            ),
            ("nested_deep.docx", Fails("import.docx_invalid"), CASE_BOUND),
            (
                "entity_billion_laughs.docx",
                Fails("import.not_a_screenplay"),
                CASE_BOUND,
            ),
            (
                "entity_external_xxe.docx",
                Fails("import.not_a_screenplay"),
                CASE_BOUND,
            ),
            (
                "bomb_ratio.docx",
                Fails("import.unsafe_archive"),
                HEAVY_BOUND,
            ),
            (
                "bomb_lying_header.docx",
                Fails("import.unsafe_archive"),
                HEAVY_BOUND,
            ),
            (
                "directory_flood.docx",
                Fails("import.unsafe_archive"),
                CASE_BOUND,
            ),
        ],
    );
}

// ================================================================== PDF

fn pdf_normal(o: &ImportOutcome) {
    assert_eq!(o.format, SourceFormat::Pdf);
    assert_eq!(
        headings(o),
        [
            "INT. RAILWAY PLATFORM - NIGHT",
            "EXT. STATION ROAD - CONTINUOUS"
        ]
    );
    assert!(all_text(o).contains("Nobody came."));
}

fn pdf_one_heading(o: &ImportOutcome) {
    assert_eq!(headings(o), ["INT. HOUSE - DAY"]);
}

#[test]
fn m1_15_pdf_hostile_corpus() {
    use Expect::*;
    run_format(
        "pdf",
        &[
            ("normal.pdf", Parses(pdf_normal), CASE_BOUND),
            (
                "malformed_header_only.pdf",
                Fails("import.pdf_unreadable"),
                CASE_BOUND,
            ),
            (
                "malformed_truncated.pdf",
                Fails("import.pdf_unreadable"),
                CASE_BOUND,
            ),
            // Plain text with a .pdf extension: the extension picks the PDF reader, which
            // finds no %PDF header and says the PDF could not be read.
            (
                "malformed_no_header.pdf",
                Fails("import.pdf_unreadable"),
                CASE_BOUND,
            ),
            ("malformed_bad_xref.pdf", Parses(pdf_normal), CASE_BOUND),
            ("nested_arrays.pdf", Fails("import.too_large"), CASE_BOUND),
            (
                "nested_object_stream.pdf",
                Fails("import.too_large"),
                CASE_BOUND,
            ),
            ("bomb_flate.pdf", Fails("import.too_large"), HEAVY_BOUND),
            ("bomb_lzw.pdf", Fails("import.too_large"), CASE_BOUND),
            // The reader cuts the page-tree loop (no hang, no stack overflow); a PDF with no
            // page left is reported as damaged.
            (
                "cycle_page_tree.pdf",
                Fails("import.pdf_unreadable"),
                CASE_BOUND,
            ),
            (
                "cycle_self_reference.pdf",
                Fails("import.pdf_unreadable"),
                CASE_BOUND,
            ),
            // The `/Prev` chain points at itself: it is followed once, then the one page reads.
            ("cycle_xref_prev.pdf", Parses(pdf_one_heading), CASE_BOUND),
            (
                "huge_length.pdf",
                Fails("import.pdf_not_screenplay"),
                CASE_BOUND,
            ),
            ("encrypted.pdf", Fails("import.pdf_unreadable"), CASE_BOUND),
        ],
    );
}

// ================================================================== README sync

/// Every committed corpus file (all six formats) is documented in the README table, and
/// every file the README names exists.
#[test]
fn m1_15_corpus_and_readme_list_the_same_cases() {
    let readme = std::fs::read_to_string(corpus_root().join("README.md")).unwrap();
    let mut listed = BTreeSet::new();
    for line in readme.lines() {
        // Table rows: | `format/file` | ...
        if let Some(rest) = line.strip_prefix("| `")
            && let Some((name, _)) = rest.split_once('`')
            && name.contains('/')
        {
            listed.insert(name.to_string());
        }
    }
    let mut committed = BTreeSet::new();
    for format in ["fdx", "fountain", "docx", "pdf", "zip", "packages"] {
        for (name, _) in corpus(format) {
            committed.insert(format!("{format}/{name}"));
        }
    }
    let undocumented: Vec<_> = committed.difference(&listed).collect();
    let missing: Vec<_> = listed.difference(&committed).collect();
    assert!(
        undocumented.is_empty() && missing.is_empty(),
        "README.md out of sync: undocumented {undocumented:?}, missing files {missing:?}"
    );
}

// ================================================================== generated (huge) inputs

/// A file just over the import cap is refused from its size alone (it is never read).
#[test]
fn m1_15_huge_files_are_refused_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    for ext in ["fdx", "fountain", "pdf", "docx", "txt"] {
        let p = dir.path().join(format!("huge.{ext}"));
        let f = std::fs::File::create(&p).unwrap();
        f.set_len(MAX_IMPORT_BYTES + 1).unwrap();
        drop(f);
        let e = bounded(&format!("huge.{ext}"), CASE_BOUND, || import_file(&p, None))
            .expect_err("over the cap");
        assert_eq!(
            e.code_str(),
            "validation.invalid_input",
            "{ext}: {}",
            e.message
        );
        assert!(e.message.contains("too large"), "{ext}: {}", e.message);
        std::fs::remove_file(&p).unwrap();
    }
    // Text over the in-memory cap.
    let big = vec![b'a'; MAX_TEXT_BYTES + 1];
    for f in [SourceFormat::Fountain, SourceFormat::Txt] {
        let e = bounded("huge text", HEAVY_BOUND, || import_bytes(&big, f)).unwrap_err();
        assert_eq!(e.code_str(), "import.too_large");
    }
    let s = "a".repeat(MAX_TEXT_BYTES + 1);
    assert_eq!(
        import_text(&s, false).unwrap_err().code_str(),
        "import.too_large"
    );
}

/// Multi-megabyte Fountain/TXT inputs built from hostile patterns parse in bounded time
/// (any quadratic step would blow the bound by orders of magnitude).
#[test]
fn m1_15_huge_text_patterns_parse_in_bounded_time() {
    const N: usize = 4 << 20;
    let fill = |unit: &str| -> String { unit.repeat(N / unit.len()) };
    let mut cases: Vec<(&str, String)> = vec![
        ("200k scenes", fill("INT. HOUSE - DAY\n\nAction.\n\n")),
        ("dual dialogue chain", {
            let mut s = String::from("INT. HOUSE - DAY\n\n");
            s.push_str(&fill("BOB\nHi.\n\nANN ^\nHo.\n\n"));
            s
        }),
        ("one 4 MiB emphasis line", {
            let mut s = String::from("INT. HOUSE - DAY\n\n");
            s.push_str(&fill("*_"));
            s
        }),
        ("unclosed note openers", {
            let mut s = String::from("INT. HOUSE - DAY\n\n");
            s.push_str(&fill("[[ "));
            s
        }),
        ("note openers per line", {
            let mut s = String::from("INT. HOUSE - DAY\n\n");
            s.push_str(&fill("x [[\n"));
            s
        }),
        ("boneyard openers", {
            let mut s = String::from("INT. HOUSE - DAY\n\nA.\n\n");
            s.push_str(&fill("/*"));
            s
        }),
        ("boneyard pairs", {
            let mut s = String::from("INT. HOUSE - DAY\n\nA.\n\n");
            s.push_str(&fill("/**/"));
            s
        }),
        ("title page flood", {
            let mut s = fill("Key: value\n");
            s.push_str("\nINT. HOUSE - DAY\n\nA.\n");
            s
        }),
        ("cue flood", fill("BOB\n")),
        ("parenthetical flood", {
            let mut s = String::from("INT. HOUSE - DAY\n\nBOB\n");
            s.push_str(&fill("(beat)\n"));
            s
        }),
        ("one 4 MiB line", {
            let mut s = String::from("INT. HOUSE - DAY\n\n");
            s.push_str(&fill("word "));
            s
        }),
        ("blank lines", {
            let mut s = String::from("INT. HOUSE - DAY\n");
            s.push_str(&fill("\n"));
            s
        }),
        ("carriage returns", {
            let mut s = String::from("INT. HOUSE - DAY\r\r");
            s.push_str(&fill("A.\r"));
            s
        }),
        ("scene numbers", fill("INT. HOUSE - DAY #1A#\n\n")),
        ("centered and transitions", {
            let mut s = String::from("INT. HOUSE - DAY\n\n");
            s.push_str(&fill(">CUT TO:<\n\nCUT TO:\n\n"));
            s
        }),
    ];
    // Indented plain text exercises the TXT heuristics (margin histogram, wrapping).
    cases.push((
        "indented txt",
        fill("                         BOB\n          Hello there.\n\n"),
    ));
    for (name, text) in &cases {
        for (label, f) in [
            ("fountain", SourceFormat::Fountain),
            ("txt", SourceFormat::Txt),
        ] {
            let what = format!("{name} as {label}");
            let started = std::time::Instant::now();
            let r = bounded(&what, HEAVY_BOUND, || import_bytes(text.as_bytes(), f));
            println!("{what}: {:?}", started.elapsed());
            if let Err(e) = r {
                assert_human(&e, &what);
            }
        }
    }
}

/// Found by hostile file collection: every `(MORE)` page marker looked back over the whole
/// scene for the last character cue (and every Fountain parenthetical did the same for its
/// cue's dual flag, see "parenthetical flood" above). A long scene with many of them was
/// quadratic.
#[test]
fn m1_15_more_marker_flood_is_linear() {
    const N: usize = 4 << 20;
    let mut more = String::from("INT. HOUSE - DAY\n\n");
    more.push_str(&"A.\n\n(MORE)\n\n".repeat(N / 12));
    for (label, f) in [
        ("fountain", SourceFormat::Fountain),
        ("txt", SourceFormat::Txt),
    ] {
        let what = format!("(MORE) flood as {label}");
        if let Err(e) = bounded(&what, HEAVY_BOUND, || import_bytes(more.as_bytes(), f)) {
            assert_human(&e, &what);
        }
    }
}

/// Huge and amplified FDX/XML: node floods and deep nesting are refused within the bound.
#[test]
fn m1_15_huge_fdx_inputs_are_bounded() {
    let wrap = |body: &str| {
        format!(
            "<?xml version=\"1.0\"?><FinalDraft DocumentType=\"Script\"><Content>{body}</Content></FinalDraft>"
        )
    };
    // Over the node budget (2,000,000 events): refused, not materialised.
    let flood = wrap(&"<a/>".repeat(2_100_000));
    let e = bounded("fdx node flood", HEAVY_BOUND, || {
        import_bytes(flood.as_bytes(), SourceFormat::Fdx)
    })
    .unwrap_err();
    assert_eq!(e.code_str(), "import.fdx_malformed");
    // Very deep nesting (100k levels): refused at depth 257, no stack overflow.
    let deep = wrap(&format!(
        "{}{}",
        "<p>".repeat(100_000),
        "</p>".repeat(100_000)
    ));
    let e = bounded("fdx deep nesting", HEAVY_BOUND, || {
        import_bytes(deep.as_bytes(), SourceFormat::Fdx)
    })
    .unwrap_err();
    assert_eq!(e.code_str(), "import.fdx_malformed");
    // 50k real scenes: a legitimate (if huge) script still parses.
    let scenes = wrap(&"<Paragraph Type=\"Scene Heading\"><Text>INT. HOUSE - DAY</Text></Paragraph><Paragraph Type=\"Action\"><Text>A.</Text></Paragraph>".repeat(50_000));
    let o = bounded("fdx 50k scenes", HEAVY_BOUND, || {
        import_bytes(scenes.as_bytes(), SourceFormat::Fdx)
    })
    .unwrap();
    assert_eq!(o.doc.scenes.len(), 50_000);
    // One 8 MiB text node and one 8 MiB attribute value.
    let text = wrap(&format!(
        "<Paragraph Type=\"Scene Heading\"><Text>INT. HOUSE - DAY</Text></Paragraph><Paragraph Type=\"Action\"><Text>{}</Text></Paragraph>",
        "x".repeat(8 << 20)
    ));
    bounded("fdx 8 MiB text node", HEAVY_BOUND, || {
        import_bytes(text.as_bytes(), SourceFormat::Fdx)
    })
    .unwrap();
    let attr = wrap(&format!(
        "<Paragraph Type=\"{}\"><Text>INT. HOUSE - DAY</Text></Paragraph>",
        "S".repeat(8 << 20)
    ));
    if let Err(e) = bounded("fdx 8 MiB attribute", HEAVY_BOUND, || {
        import_bytes(attr.as_bytes(), SourceFormat::Fdx)
    }) {
        assert_human(&e, "fdx 8 MiB attribute");
    }
    // 1M character references: bounded, never expanded into anything but the characters.
    let refs = wrap(&format!(
        "<Paragraph Type=\"Scene Heading\"><Text>INT. HOUSE - DAY</Text></Paragraph><Paragraph Type=\"Action\"><Text>{}</Text></Paragraph>",
        "&#65;&amp;".repeat(500_000)
    ));
    if let Err(e) = bounded("fdx 1M references", HEAVY_BOUND, || {
        import_bytes(refs.as_bytes(), SourceFormat::Fdx)
    }) {
        assert_human(&e, "fdx 1M references");
    }
}

/// Huge PDFs built at test time: too many pages, many streams sharing one dictionary
/// window, and a stream-keyword flood are refused or parsed within the bound.
#[test]
fn m1_15_huge_pdf_inputs_are_bounded() {
    use lopdf::{Document, Object, Stream, dictionary};
    // 2,001 pages: over MAX_PAGES.
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 72 700 Td (INT. HOUSE - DAY) Tj ET".to_vec(),
    ));
    let kids: Vec<Object> = (0..pdf_import::MAX_PAGES + 1)
        .map(|_| {
            doc.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id, "Contents" => content,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            })
            .into()
        })
        .collect();
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", cat);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let e = bounded("pdf 2001 pages", HEAVY_BOUND, || {
        import_bytes(&bytes, SourceFormat::Pdf)
    })
    .unwrap_err();
    assert_eq!(e.code_str(), "import.too_large");
    assert_human(&e, "pdf 2001 pages");
}

/// Found by hostile file collection: one top-level dictionary followed by many streams made the pre-flight
/// rescan the whole file for every stream (O(n²): a 2 MiB file took minutes).
#[test]
fn m1_15_pdf_stream_flood_preflight_is_linear() {
    // One top-level dictionary followed by 100k bare streams: each stream's dictionary
    // window must not stretch back to the start of the file (quadratic pre-flight).
    let mut flood = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n".to_vec();
    for _ in 0..100_000 {
        flood.extend_from_slice(b" stream\nxx\nendstream\n");
    }
    flood.extend_from_slice(b"%%EOF\n");
    let r = bounded("pdf stream flood", HEAVY_BOUND, || {
        pdf_import::preflight(&flood)
    });
    assert!(r.is_ok(), "plain streams are not refused");
    if let Err(e) = bounded("pdf stream flood import", HEAVY_BOUND, || {
        import_bytes(&flood, SourceFormat::Pdf)
    }) {
        assert_human(&e, "pdf stream flood import");
    }
}

/// A Unicode file name and folder (accents, CJK, emoji, bidi-neutral) import like any other.
#[test]
fn m1_15_unicode_paths_import_correctly() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("Scénarios 台本 🎬");
    std::fs::create_dir_all(&dir).unwrap();
    for (format, file, check) in [
        (
            "fountain",
            "normal.fountain",
            fountain_normal as fn(&ImportOutcome),
        ),
        ("fdx", "normal.fdx", fdx_normal),
        ("docx", "normal.docx", docx_normal),
        ("pdf", "normal.pdf", pdf_normal),
    ] {
        let ext = Path::new(file).extension().unwrap().to_str().unwrap();
        let dest = dir.join(format!("Été – ドラフト ✦ 1.{ext}"));
        std::fs::copy(corpus_root().join(format).join(file), &dest).unwrap();
        let o = bounded(&format!("unicode path {format}"), CASE_BOUND, || {
            import_file(&dest, None)
        })
        .unwrap();
        check(&o);
    }
}

/// Symlinks and junctions on Windows (created at test time): a junction is a folder and
/// is refused as an import source; a file symlink to a real script is resolved and read
/// (the target is validated like any picked file); a dangling symlink is a clear error.
#[test]
fn m1_15_symlinked_and_junctioned_import_sources() {
    let root = tempfile::tempdir().unwrap();
    let target_dir = root.path().join("real");
    std::fs::create_dir_all(&target_dir).unwrap();
    let script = target_dir.join("script.fountain");
    std::fs::copy(corpus_root().join("fountain/normal.fountain"), &script).unwrap();

    let junction = root.path().join("junction.fountain");
    match hostile::make_junction(&junction, &target_dir) {
        Ok(()) => {
            let e = bounded("junction source", CASE_BOUND, || {
                import_file(&junction, None)
            })
            .unwrap_err();
            assert_eq!(e.code_str(), "validation.invalid_input");
            assert!(e.message.contains("not a folder"), "{}", e.message);
            // Through a junctioned folder the file itself is an ordinary pick.
            let via = junction.join("script.fountain");
            fountain_normal(
                &bounded("via junction", CASE_BOUND, || import_file(&via, None)).unwrap(),
            );
        }
        Err(why) => println!("SKIPPED junction source case: {why}"),
    }

    let link = root.path().join("link.fountain");
    match hostile::make_file_symlink(&link, &script) {
        Ok(()) => {
            fountain_normal(
                &bounded("symlink source", CASE_BOUND, || import_file(&link, None)).unwrap(),
            );
            let dangling = root.path().join("dangling.fountain");
            hostile::make_file_symlink(&dangling, &root.path().join("gone.fountain")).unwrap();
            let r = bounded("dangling symlink", CASE_BOUND, || {
                import_file(&dangling, None)
            });
            assert!(r.is_err(), "a dangling link is never read");
        }
        Err(why) => println!("SKIPPED file symlink cases: {why}"),
    }
    assert_no_escape(root.path());
}

// ================================================================== corpus generator

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn deflated(name: &str, data: &[u8]) -> RawEntry {
    RawEntry::deflated(name, deflate(data), crc32(data), data.len() as u32)
}

/// A deflated entry of `real` zero bytes whose headers claim `declared` bytes.
fn lying(name: &str, real: usize, declared: u32) -> RawEntry {
    let data = vec![0u8; real];
    RawEntry::deflated(name, deflate(&data), crc32(&data), declared)
}

const CT: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#;

fn docx_xml(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>"
    )
}

fn normal_doc() -> openframe_import_export::ScreenplayDoc {
    let src = std::fs::read_to_string(corpus_root().join("fountain/normal.fountain")).unwrap();
    import_text(&src, true).unwrap().doc
}

fn normal_docx_entries() -> Vec<RawEntry> {
    let opts = docx::DocxOptions {
        title_page: false,
        ..Default::default()
    };
    docx::package_parts(&normal_doc(), &opts)
        .into_iter()
        .map(|(n, b)| deflated(&n, &b))
        .collect()
}

fn with_entries(mut base: Vec<RawEntry>, extra: Vec<RawEntry>) -> Vec<u8> {
    base.extend(extra);
    build_zip(&base, None)
}

fn replace_document(xml: &str) -> Vec<u8> {
    let mut e: Vec<RawEntry> = normal_docx_entries()
        .into_iter()
        .filter(|e| e.name != b"word/document.xml")
        .collect();
    e.push(deflated("word/document.xml", xml.as_bytes()));
    build_zip(&e, None)
}

/// PDF with a correct xref table. `trailer` receives the xref offset.
fn pdf_doc(objects: &[Vec<u8>], trailer: impl Fn(usize) -> String) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![];
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend(body);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for o in offsets {
        out.extend(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {} >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1,
            trailer(xref)
        )
        .as_bytes(),
    );
    out
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< /Length {} {dict} >>\nstream\n", data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

fn one_page(content: Vec<u8>) -> Vec<Vec<u8>> {
    vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),
        content,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".to_vec(),
    ]
}

const PKG_ID: &str = "01926f6e-0000-7000-8000-00000000a001";
const SRC_ID: &str = "01926f6e-0000-7000-8000-00000000a002";
const USER_ID: &str = "01926f6e-0000-7000-8000-00000000a003";

fn manifest(package_id: &str) -> Vec<u8> {
    format!(
        r#"{{"format":"openframe-package","packageType":"story","formatVersion":1,"packageId":"{package_id}","sourceProjectId":"{SRC_ID}","sourceProjectTitle":"Hostile","exportedAt":1,"appVersion":"1","scope":{{"kind":"full","label":"Story"}},"commentsIncluded":false,"attachmentsIncluded":false,"privateNotesIncluded":false,"originatingUser":{{"userId":"{USER_ID}","displayName":"Mallory"}}}}"#
    )
    .into_bytes()
}

/// An exchange package whose checksums list every entry (so validation reaches the hostile
/// part). `entries` are added after manifest.json and content.json.
fn package(manifest_bytes: &[u8], content: &[u8], extra: Vec<RawEntry>) -> Vec<u8> {
    let mut entries = vec![
        deflated("manifest.json", manifest_bytes),
        deflated("content.json", content),
    ];
    entries.extend(extra);
    let mut files = Vec::new();
    for e in &entries {
        // Checksums are over the *uncompressed* bytes; recover them for stored entries,
        // and for deflated ones by inflating.
        let plain = if e.method == 0 {
            e.data.clone()
        } else {
            let mut d = flate2::read::DeflateDecoder::new(e.data.as_slice());
            let mut v = Vec::new();
            std::io::Read::read_to_end(&mut d, &mut v).unwrap();
            v
        };
        files.push(format!(
            "\"{}\":\"{}\"",
            String::from_utf8_lossy(&e.name).replace('\\', "\\\\"),
            openframe_security::sha256_bytes(&plain)
        ));
    }
    let sums = format!(
        r#"{{"algorithm":"sha256","files":{{{}}}}}"#,
        files.join(",")
    );
    entries.push(deflated("checksums.json", sums.as_bytes()));
    build_zip(&entries, None)
}

/// Writes the binary corpus files. Ignored: run explicitly, then review and commit.
#[test]
#[ignore = "regenerates committed corpus files; run explicitly after changing a generator"]
fn regenerate_hostile_corpus() {
    let root = corpus_root();
    let w = |rel: &str, bytes: &[u8]| {
        let p: PathBuf = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, bytes).unwrap();
    };
    let escape = format!("../../{ESCAPE_MARKER}");

    // ---------------------------------------------------------- FDX
    let mut bin = b"<?xml version=\"1.0\"?>\n<FinalDraft><Content>".to_vec();
    bin.extend((0u8..=255).cycle().take(2048).map(|b| b.wrapping_mul(37)));
    w("fdx/malformed_binary.fdx", &bin);
    let utf16: Vec<u8> = std::iter::once([0xFFu8, 0xFE])
        .chain(
            std::fs::read_to_string(root.join("fdx/normal.fdx"))
                .unwrap()
                .encode_utf16()
                .map(u16::to_le_bytes),
        )
        .flatten()
        .collect();
    w("fdx/malformed_utf16.fdx", &utf16);
    w(
        "fdx/nested_deep.fdx",
        format!(
            "<?xml version=\"1.0\"?><FinalDraft DocumentType=\"Script\"><Content><Paragraph Type=\"Scene Heading\"><Text>INT. HOUSE - DAY</Text></Paragraph>{}{}</Content></FinalDraft>",
            "<Paragraph Type=\"Action\">".repeat(300),
            "</Paragraph>".repeat(300)
        )
        .as_bytes(),
    );
    let mut ctl = b"<?xml version=\"1.0\"?><FinalDraft DocumentType=\"Script\"><Content><Paragraph Type=\"Scene Heading\"><Text>INT. HOUSE - DAY</Text></Paragraph><Paragraph Type=\"Action\"><Text>".to_vec();
    ctl.extend((0u8..32).filter(|b| !matches!(b, b'\t' | b'\n' | b'\r')));
    ctl.extend(b"\x7F &#0; &#1; &#xFFFE; &#x10FFFF; &#99999999; end</Text></Paragraph></Content></FinalDraft>");
    w("fdx/control_chars.fdx", &ctl);

    // ---------------------------------------------------------- Fountain
    let mut fbin = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    fbin.extend((0u16..2048).map(|i| (i.wrapping_mul(97) % 251) as u8));
    w("fountain/malformed_binary.fountain", &fbin);
    w(
        "fountain/legacy_encoding.fountain",
        b"INT. CAF\xC9 - DAY\n\nThe Caf\xE9 is \x93quiet\x94 \x96 almost.\n\nZO\xCB\nCa va?\n",
    );
    let u: Vec<u8> = [0xFFu8, 0xFE]
        .into_iter()
        .chain(
            "INT. CAFÉ - DAY\n\nZoë waits.\n\nZOË\nHello.\n"
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        )
        .collect();
    w("fountain/utf16le_bom.fountain", &u);
    let mut storm = String::from("INT. HOUSE - DAY\n\n");
    storm.push_str(&"***___**_*_".repeat(1_500));
    storm.push_str("\n\n");
    storm.push_str(&"\\*".repeat(2_000));
    storm.push('\n');
    w("fountain/emphasis_storm.fountain", storm.as_bytes());
    let mut fctl = b"INT. HOUSE - DAY\n\nNUL\0 BEL\x07 FF\x0c VT\x0b ESC\x1b DEL\x7f.\n\n".to_vec();
    fctl.extend(
        "BOB\u{0000}\nHi\u{2028}there\u{2029}.\n\n\u{FEFF}INT. ROOF - NIGHT\n\nA.\n".as_bytes(),
    );
    w("fountain/control_chars.fountain", &fctl);

    // ---------------------------------------------------------- DOCX
    let normal = normal_docx_entries();
    w("docx/normal.docx", &build_zip(&normal, None));
    w(
        "docx/unicode_names.docx",
        &with_entries(
            normal.clone(),
            vec![
                deflated("word/média/été 台本 🎬.xml", b"<x/>"),
                deflated("customXml/\u{202E}lmx.exe", b"<x/>"),
            ],
        ),
    );
    w(
        "docx/zip_slip.docx",
        &with_entries(
            normal.clone(),
            vec![
                RawEntry::stored(&escape, b"pwned"),
                RawEntry::stored(&format!("..\\..\\{ESCAPE_MARKER}"), b"pwned"),
            ],
        ),
    );
    w(
        "docx/absolute_paths.docx",
        &with_entries(
            normal.clone(),
            vec![
                RawEntry::stored(&format!("C:/{ESCAPE_MARKER}"), b"pwned"),
                RawEntry::stored(&format!("/{ESCAPE_MARKER}"), b"pwned"),
                RawEntry::stored(&format!("\\\\?\\C:\\{ESCAPE_MARKER}"), b"pwned"),
            ],
        ),
    );
    w(
        "docx/case_collision.docx",
        &with_entries(
            normal.clone(),
            vec![deflated(
                "WORD/DOCUMENT.XML",
                docx_xml("<w:p><w:r><w:t>INT. IMPOSTOR - DAY</w:t></w:r></w:p>").as_bytes(),
            )],
        ),
    );
    w(
        "docx/duplicate_entry.docx",
        &with_entries(
            normal.clone(),
            vec![deflated(
                "word/document.xml",
                docx_xml("<w:p><w:r><w:t>INT. IMPOSTOR - DAY</w:t></w:r></w:p>").as_bytes(),
            )],
        ),
    );
    let mut sym: Vec<RawEntry> = normal
        .iter()
        .filter(|e| e.name != b"word/document.xml")
        .cloned()
        .collect();
    sym.push(RawEntry::symlink("word/document.xml", "C:/Windows/win.ini"));
    w("docx/symlink_document.docx", &build_zip(&sym, None));
    w(
        "docx/malformed_not_zip.docx",
        b"PK\x03\x04 this is not really a zip archive, just a header",
    );
    let full = build_zip(&normal, None);
    w("docx/malformed_truncated.docx", &full[..full.len() * 3 / 5]);
    w(
        "docx/malformed_xml.docx",
        &replace_document("<w:document><w:body><w:p><w:r><w:t>INT. HOUSE"),
    );
    w(
        "docx/missing_document.docx",
        &build_zip(&[deflated("[Content_Types].xml", CT)], None),
    );
    w(
        "docx/nested_deep.docx",
        &replace_document(&docx_xml(&format!(
            "{}{}",
            "<w:tbl><w:tr><w:tc>".repeat(120),
            "</w:tc></w:tr></w:tbl>".repeat(120)
        ))),
    );
    w(
        "docx/entity_billion_laughs.docx",
        &replace_document(
            r#"<?xml version="1.0"?><!DOCTYPE d [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;"><!ENTITY d "&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;"><!ENTITY e "&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;"><!ENTITY f "&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;"><!ENTITY g "&f;&f;&f;&f;&f;&f;&f;&f;&f;&f;"><!ENTITY h "&g;&g;&g;&g;&g;&g;&g;&g;&g;&g;">]><w:document xmlns:w="w"><w:body><w:p><w:r><w:t>&h;</w:t></w:r></w:p></w:body></w:document>"#,
        ),
    );
    w(
        "docx/entity_external_xxe.docx",
        &replace_document(
            r#"<?xml version="1.0"?><!DOCTYPE d [<!ENTITY x SYSTEM "file:///C:/Windows/win.ini">]><w:document xmlns:w="w"><w:body><w:p><w:r><w:t>&x;</w:t></w:r></w:p></w:body></w:document>"#,
        ),
    );
    let mut bomb: Vec<RawEntry> = normal
        .iter()
        .filter(|e| e.name != b"word/document.xml")
        .cloned()
        .collect();
    bomb.push(deflated("word/document.xml", &vec![b' '; 40 << 20]));
    w("docx/bomb_ratio.docx", &build_zip(&bomb, None));
    let mut liar: Vec<RawEntry> = normal
        .iter()
        .filter(|e| e.name != b"word/document.xml")
        .cloned()
        .collect();
    liar.push(lying("word/document.xml", 40 << 20, 900));
    w("docx/bomb_lying_header.docx", &build_zip(&liar, None));
    w(
        "docx/directory_flood.docx",
        &build_zip(&normal, Some(60_000)),
    );

    // ---------------------------------------------------------- PDF
    let (bytes, _, _) = render_bytes(
        &normal_doc(),
        ExportFormat::Pdf,
        &ExportOptions {
            title_page: false,
            ..Default::default()
        },
    )
    .unwrap();
    w("pdf/normal.pdf", &bytes);
    w("pdf/malformed_header_only.pdf", b"%PDF-1.7\n%%EOF\n");
    w("pdf/malformed_truncated.pdf", &bytes[..bytes.len() / 3]);
    w(
        "pdf/malformed_no_header.pdf",
        b"This file has a .pdf extension but is plain text.\n",
    );
    // Valid objects, every xref offset wrong: lopdf rebuilds the table from the objects.
    let mut bad_xref = bytes.clone();
    if let Some(x) = bad_xref.windows(5).rposition(|w| w == b"xref\n") {
        let mut i = x + 5;
        // Skip the subsection header line, then zero every 10-digit offset.
        while i < bad_xref.len() && bad_xref[i] != b'\n' {
            i += 1;
        }
        i += 1;
        while i + 20 <= bad_xref.len() && bad_xref[i].is_ascii_digit() {
            for b in &mut bad_xref[i..i + 10] {
                *b = b'0';
            }
            i += 20;
        }
    }
    w("pdf/malformed_bad_xref.pdf", &bad_xref);
    w(
        "pdf/nested_arrays.pdf",
        &pdf_doc(
            &[format!("{}{}", "[".repeat(1_000), "]".repeat(1_000)).into_bytes()],
            |_| String::new(),
        ),
    );
    let inner = format!("{}{}", "[".repeat(5_000), "]".repeat(5_000));
    w(
        "pdf/nested_object_stream.pdf",
        &pdf_doc(
            &[
                b"<< /Type /Catalog >>".to_vec(),
                stream_obj(
                    "/Type /ObjStm /N 1 /First 0 /Filter /FlateDecode",
                    &zlib(inner.as_bytes()),
                ),
            ],
            |_| String::new(),
        ),
    );
    let flate_bomb = zlib(&vec![
        0u8;
        (pdf_import::MAX_STREAM_INFLATED + (1 << 20)) as usize
    ]);
    w(
        "pdf/bomb_flate.pdf",
        &pdf_doc(
            &one_page(stream_obj("/Filter /FlateDecode", &flate_bomb)),
            |_| String::new(),
        ),
    );
    w(
        "pdf/bomb_lzw.pdf",
        &pdf_doc(
            &one_page(stream_obj(
                "/Filter /LZWDecode",
                b"\x80\x0b\x60\x50\x22\x0c\x0c\x85\x01",
            )),
            |_| String::new(),
        ),
    );
    w(
        "pdf/cycle_page_tree.pdf",
        &pdf_doc(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
                b"<< /Type /Pages /Kids [2 0 R 3 0 R] /Count 1 /Parent 2 0 R >>".to_vec(),
            ],
            |_| String::new(),
        ),
    );
    w(
        "pdf/cycle_self_reference.pdf",
        &pdf_doc(&[b"1 0 R".to_vec(), b"2 0 R".to_vec()], |_| String::new()),
    );
    let content = stream_obj("", b"BT /F1 12 Tf 72 700 Td (INT. HOUSE - DAY) Tj ET");
    w(
        "pdf/cycle_xref_prev.pdf",
        &pdf_doc(&one_page(content.clone()), |xref| format!("/Prev {xref}")),
    );
    let mut huge_len = one_page(Vec::new());
    huge_len[3] = b"<< /Length 99999999999999 >>\nstream\nBT ET\nendstream".to_vec();
    w(
        "pdf/huge_length.pdf",
        &pdf_doc(&huge_len, |_| String::new()),
    );
    let mut enc = one_page(content);
    enc.push(
        format!(
            "<< /Filter /Standard /V 1 /R 2 /O <{}> /U <{}> /P -44 >>",
            "28bf4e5e4e758a4164004e56fffa0108", "c4e1ab8f3d0b1b3d2a51e2d7c3b0a0a1"
        )
        .into_bytes(),
    );
    w(
        "pdf/encrypted.pdf",
        &pdf_doc(&enc, |_| {
            "/Encrypt 6 0 R /ID [<00112233445566778899aabbccddeeff><00112233445566778899aabbccddeeff>]".into()
        }),
    );

    // ---------------------------------------------------------- ZIP (archive layer)
    let ok = || {
        vec![
            RawEntry::stored("docs/readme.txt", b"hello"),
            deflated("notes.txt", b"plain notes"),
        ]
    };
    w("zip/normal.zip", &build_zip(&ok(), None));
    let one = |name: &str| with_entries(ok(), vec![RawEntry::stored(name, b"pwned")]);
    w("zip/slip_dotdot.zip", &one(&escape));
    w(
        "zip/slip_backslash.zip",
        &one(&format!("..\\..\\{ESCAPE_MARKER}")),
    );
    w(
        "zip/slip_nested.zip",
        &one(&format!("a/b/../../../{ESCAPE_MARKER}")),
    );
    w(
        "zip/slip_dot_segments.zip",
        &one(&format!("./././../{ESCAPE_MARKER}")),
    );
    w("zip/abs_unix.zip", &one(&format!("/tmp/{ESCAPE_MARKER}")));
    w("zip/abs_drive.zip", &one(&format!("C:/{ESCAPE_MARKER}")));
    w(
        "zip/abs_drive_backslash.zip",
        &one(&format!("C:\\{ESCAPE_MARKER}")),
    );
    w(
        "zip/drive_relative.zip",
        &one(&format!("C:{ESCAPE_MARKER}")),
    );
    w(
        "zip/unc.zip",
        &one(&format!("\\\\server\\share\\{ESCAPE_MARKER}")),
    );
    w(
        "zip/device_verbatim.zip",
        &one(&format!("\\\\?\\C:\\{ESCAPE_MARKER}")),
    );
    w("zip/device_namespace.zip", &one("\\\\.\\PhysicalDrive0"));
    w("zip/reserved_con.zip", &one("CON"));
    w("zip/reserved_nul_ext.zip", &one("docs/nul.txt"));
    w("zip/reserved_com_superscript.zip", &one("COM\u{00B9}.txt"));
    w("zip/ads_stream.zip", &one("notes.txt:hidden"));
    w("zip/trailing_dot.zip", &one("notes.txt."));
    w("zip/trailing_space.zip", &one("docs /readme.txt"));
    w("zip/empty_name.zip", &one(""));
    w(
        "zip/nul_in_name.zip",
        &with_entries(
            ok(),
            vec![RawEntry::stored_bytes(b"safe.txt\0../../x", b"pwned")],
        ),
    );
    w(
        "zip/symlink_entry.zip",
        &with_entries(
            ok(),
            vec![RawEntry::symlink("docs/link", "../../../../Windows")],
        ),
    );
    w(
        "zip/symlink_then_write.zip",
        &with_entries(
            ok(),
            vec![
                RawEntry::symlink("out", "C:/"),
                RawEntry::stored(&format!("out/{ESCAPE_MARKER}"), b"pwned"),
            ],
        ),
    );
    w(
        "zip/case_collision.zip",
        &build_zip(
            &[
                RawEntry::stored("Readme.txt", b"first: the real readme"),
                RawEntry::stored("README.TXT", b"second: overwrites the first on Windows"),
            ],
            None,
        ),
    );
    w(
        "zip/case_collision_folder.zip",
        &build_zip(
            &[
                RawEntry::stored("Assets/a.txt", b"one"),
                RawEntry::stored("assets/A.TXT", b"two"),
            ],
            None,
        ),
    );
    w(
        "zip/short_name_alias.zip",
        &build_zip(
            &[
                RawEntry::stored("content.json", b"{\"real\":true}"),
                RawEntry::stored("CONTEN~1.JSO", b"{\"alias\":true}"),
            ],
            None,
        ),
    );
    w(
        "zip/duplicate_name.zip",
        &build_zip(
            &[
                RawEntry::stored("same.txt", b"first"),
                RawEntry::stored("same.txt", b"second"),
            ],
            None,
        ),
    );
    w(
        "zip/unicode_names.zip",
        &build_zip(
            &[
                RawEntry::stored("scènes/été.txt", b"nfc"),
                RawEntry::stored("scènes/e\u{301}te\u{301}.txt", b"nfd"),
                RawEntry::stored("日本語/台本.txt", b"cjk"),
                RawEntry::stored("emoji 🎬.txt", b"emoji"),
                RawEntry::stored("fullwidth\u{FF0F}..\u{FF0F}x.txt", b"fullwidth solidus"),
                RawEntry::stored("division\u{2215}slash.txt", b"division slash"),
                RawEntry::stored("bidi \u{202E}txt.exe", b"bidi override"),
            ],
            None,
        ),
    );
    w(
        "zip/bomb_ratio.zip",
        &with_entries(ok(), vec![deflated("zeros.bin", &vec![0u8; 20 << 20])]),
    );
    w(
        "zip/bomb_lying_header.zip",
        &with_entries(ok(), vec![lying("zeros.bin", 40 << 20, 1_000)]),
    );
    w(
        "zip/bomb_total_size.zip",
        &build_zip(
            &(0..520)
                .map(|i| RawEntry {
                    declared_size: 1 << 20,
                    ..RawEntry::stored(&format!("f/{i}.bin"), b"x")
                })
                .collect::<Vec<_>>(),
            None,
        ),
    );
    w("zip/directory_flood.zip", &build_zip(&ok(), Some(65_535)));
    let inner = with_entries(ok(), vec![RawEntry::stored(&escape, b"pwned")]);
    w(
        "zip/nested_zip.zip",
        &with_entries(ok(), vec![RawEntry::stored("inner/evil.zip", &inner)]),
    );
    let z = build_zip(&ok(), None);
    w("zip/malformed_truncated.zip", &z[..z.len() - 10]);
    w(
        "zip/malformed_garbage.zip",
        b"PK\x05\x06 not an archive at all",
    );

    // ---------------------------------------------------------- packages (exchange)
    let m = manifest(PKG_ID);
    w(
        "packages/slip_dotdot.ofstory",
        &package(&m, b"{}", vec![RawEntry::stored(&escape, b"pwned")]),
    );
    w(
        "packages/abs_drive.ofstory",
        &package(
            &m,
            b"{}",
            vec![RawEntry::stored(&format!("C:/{ESCAPE_MARKER}"), b"pwned")],
        ),
    );
    w(
        "packages/symlink_entry.ofstory",
        &package(&m, b"{}", vec![RawEntry::symlink("assets", "../../..")]),
    );
    w(
        "packages/case_collision.ofstory",
        &package(
            &m,
            b"{}",
            vec![RawEntry::stored("Content.json", b"{\"hostile\":true}")],
        ),
    );
    w(
        "packages/short_name_alias.ofstory",
        &package(
            &m,
            b"{}",
            vec![RawEntry::stored("CONTEN~1.JSO", b"{\"hostile\":true}")],
        ),
    );
    w("packages/duplicate_manifest.ofstory", &{
        let mut p = package(&m, b"{}", vec![]);
        // Append nothing: rebuild with a second manifest entry instead.
        let mut entries = hostile::parse_raw_zip(&p).unwrap();
        entries.push(deflated(
            "manifest.json",
            &manifest("01926f6e-0000-7000-8000-00000000beef"),
        ));
        p = build_zip(&entries, None);
        p
    });
    w(
        "packages/bomb_ratio.ofstory",
        &package(
            &m,
            b"{}",
            vec![deflated("assets/zeros.bin", &vec![0u8; 20 << 20])],
        ),
    );
    w("packages/bomb_lying_header.ofstory", &{
        let mut entries = hostile::parse_raw_zip(&package(
            &m,
            b"{}",
            vec![deflated("assets/zeros.bin", &vec![0u8; 40 << 20])],
        ))
        .unwrap();
        for e in &mut entries {
            if e.name == b"assets/zeros.bin" {
                e.declared_size = 1_000;
            }
        }
        build_zip(&entries, None)
    });
    w(
        "packages/manifest_deep.ofstory",
        &package(
            &format!("{}{}", "[".repeat(100_000), "]".repeat(100_000)).into_bytes(),
            b"{}",
            vec![],
        ),
    );
    w(
        "packages/manifest_huge.ofstory",
        &package(
            &{
                let mut v = vec![b' '; 5 << 20];
                v.extend_from_slice(&m);
                v
            },
            b"{}",
            vec![],
        ),
    );
    w(
        "packages/manifest_traversal_id.ofstory",
        &package(&manifest("../../../reviews-escape"), b"{}", vec![]),
    );
    w("packages/checksums_traversal.ofstory", &{
        let mut entries = hostile::parse_raw_zip(&package(&m, b"{}", vec![])).unwrap();
        entries.retain(|e| e.name != b"checksums.json");
        let sums = format!(
            r#"{{"algorithm":"sha256","files":{{"manifest.json":"{}","content.json":"{}","../{ESCAPE_MARKER}":"{}"}}}}"#,
            openframe_security::sha256_bytes(&m),
            openframe_security::sha256_bytes(b"{}"),
            openframe_security::sha256_bytes(b"pwned"),
        );
        entries.push(deflated("checksums.json", sums.as_bytes()));
        build_zip(&entries, None)
    });
    w(
        "packages/content_deep.ofstory",
        &package(
            &m,
            format!("{}{}", "[".repeat(100_000), "]".repeat(100_000)).as_bytes(),
            vec![],
        ),
    );
    w(
        "packages/nested_package.ofstory",
        &package(
            &m,
            b"{}",
            vec![RawEntry::stored(
                "attachments/inner.ofstory",
                &package(&m, b"{}", vec![RawEntry::stored(&escape, b"pwned")]),
            )],
        ),
    );
    w(
        "packages/unicode_names.ofstory",
        &package(
            &m,
            b"{}",
            vec![
                RawEntry::stored("assets/été 台本 🎬.txt", b"unicode"),
                RawEntry::stored("assets/e\u{301}te\u{301}.txt", b"nfd"),
            ],
        ),
    );
    w("packages/directory_flood.ofstory", &{
        let entries = hostile::parse_raw_zip(&package(&m, b"{}", vec![])).unwrap();
        build_zip(&entries, Some(65_535))
    });
    let p = package(&m, b"{}", vec![]);
    w("packages/malformed_truncated.ofstory", &p[..p.len() / 2]);
}
