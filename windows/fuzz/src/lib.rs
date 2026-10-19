//! The fuzz targets as plain functions (Windows phase W3). `fuzz_targets/*.rs` hand them to
//! libFuzzer (`cargo +nightly fuzz run <target>`, where libFuzzer links: Linux and macOS);
//! `src/bin/fuzz_drive.rs` runs them under a seeded mutation loop on any toolchain, which is how
//! they run on the Windows development machine (libFuzzer does not link there).
//! Every target must return for any input: a document or a human error, never a panic or a hang.

use openframe_project_format::{ProjectLayout, read_manifest};
use openframe_security::archive::{self, ArchiveLimits};

/// Final Draft (.fdx) import.
pub fn fdx(data: &[u8]) {
    let _ = openframe_import_export::fdx::parse(data);
}

/// Fountain import.
pub fn fountain(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let _ = openframe_import_export::fountain::parse(&text);
}

/// The bounded XML reader under FDX and DOCX import.
pub fn xml(data: &[u8]) {
    let _ = openframe_import_export::xml::parse(data);
}

/// Word (.docx) import from a file: zip container, then XML.
pub fn docx(data: &[u8]) {
    let dir = tempfile::tempdir().expect("temp folder");
    let path = dir.path().join("input.docx");
    std::fs::write(&path, data).expect("write input");
    let _ = openframe_import_export::docx::parse(&path);
}

// The package limits allow gigabytes; the fuzzer uses the same checks with small numbers so a
// crafted archive cannot fill the disk during a run.
const LIMITS: ArchiveLimits = ArchiveLimits {
    max_entries: 1_000,
    max_total_bytes: 32 << 20,
    max_entry_bytes: 8 << 20,
    max_ratio: ArchiveLimits::PACKAGE.max_ratio,
};

/// The zip reader that opens project, backup and exchange packages: list, read one entry and
/// extract everything under limits, never writing outside the destination.
pub fn package_archive(data: &[u8]) {
    let dir = tempfile::tempdir().expect("temp folder");
    let path = dir.path().join("input.ofproject");
    std::fs::write(&path, data).expect("write input");
    let _ = archive::declared_directory(&path);
    if let Ok(names) = archive::inspect(&path, LIMITS)
        && let Some(first) = names.first()
    {
        let _ = archive::read_entry(&path, first, 1 << 20);
    }
    let dest = dir.path().join("out");
    if let Ok(files) = archive::extract_all(&path, &dest, LIMITS) {
        for f in files {
            assert!(
                f.is_relative() && dest.join(&f).starts_with(&dest),
                "unsafe extracted name: {}",
                f.display()
            );
        }
    }
    // Whatever happened, nothing was written next to the destination (zip slip).
    for e in std::fs::read_dir(dir.path()).expect("temp folder").flatten() {
        let name = e.file_name();
        assert!(name == "input.ofproject" || name == "out", "wrote outside the destination: {name:?}");
    }
}

/// The project manifest (openframe.json) read when a project or a package opens.
pub fn project_manifest(data: &[u8]) {
    let dir = tempfile::tempdir().expect("temp folder");
    let layout = ProjectLayout::new(dir.path().join("Fuzz.openframe"));
    std::fs::create_dir_all(layout.root()).expect("project folder");
    std::fs::write(layout.manifest(), data).expect("write manifest");
    let _ = read_manifest(&layout);
}

/// A target: its name, its function, and the corpus folders (relative to the repository) and file
/// endings it is seeded from.
pub struct Target {
    pub name: &'static str,
    pub run: fn(&[u8]),
    pub seeds: &'static [(&'static str, &'static [&'static str])],
}

pub const TARGETS: &[Target] = &[
    Target { name: "fdx", run: fdx, seeds: &[("crates/openframe-import-export/tests/fixtures/hostile/fdx", &["fdx"]), ("crates/openframe-import-export/tests/fixtures/fdx-suite", &["fdx"]), ("crates/openframe-import-export/tests/fixtures/screenplay", &["fdx"])] },
    Target { name: "fountain", run: fountain, seeds: &[("crates/openframe-import-export/tests/fixtures/hostile/fountain", &["fountain"]), ("crates/openframe-import-export/tests/fixtures/screenplay", &["fountain", "txt"])] },
    Target { name: "xml", run: xml, seeds: &[("crates/openframe-import-export/tests/fixtures/hostile/fdx", &["fdx"]), ("crates/openframe-import-export/tests/fixtures/fdx-suite", &["fdx"])] },
    Target { name: "docx", run: docx, seeds: &[("crates/openframe-import-export/tests/fixtures/hostile/docx", &["docx"])] },
    Target { name: "package_archive", run: package_archive, seeds: &[("crates/openframe-import-export/tests/fixtures/hostile/zip", &["zip"]), ("crates/openframe-import-export/tests/fixtures/hostile/packages", &["ofproject", "ofbackup", "zip"]), ("crates/openframe-application/tests/fixtures/packages", &["ofproject", "ofbackup", "ofresponse", "ofscriptreview", "ofstory"])] },
    Target { name: "project_manifest", run: project_manifest, seeds: &[("crates/openframe-import-export/tests/fixtures/hostile/packages", &["json"])] },
];

/// Every seed file of a target, read from the repository at `root`.
pub fn seeds(root: &std::path::Path, target: &Target) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (dir, endings) in target.seeds {
        let mut stack = vec![root.join(dir)];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| endings.iter().any(|end| x.eq_ignore_ascii_case(end)))
                    && let Ok(bytes) = std::fs::read(&p)
                {
                    out.push(bytes);
                }
            }
        }
    }
    if target.name == "project_manifest" {
        out.push(br#"{"format":"openframe-project","formatVersion":1,"projectId":"p","title":"T","projectType":"Feature Film","schemaVersion":16,"createdAt":0,"createdWith":"0.1.0"}"#.to_vec());
    }
    if out.is_empty() {
        out.push(Vec::new());
    }
    out
}
