//! The four sample scripts that ship inside the app (owner decision 2026-10-07; masterplan area
//! "Call sheets, budget, templates, samples and exports").
//!
//! OpenFrame Studio wrote and owns every sample; each file carries its own copyright note
//! (`templates/samples/README.md`). The files are compiled into the binary, so every package
//! (`.msix`, `.pkg`, `.exe`, `.dmg`) has them without a download and without a file the user
//! could lose. New users open one to learn the app (`screenplay.import_preview` with a sample
//! id), and the test suites parse them as regression fixtures (`tests/samples.rs` here and in
//! the application crate).
//!
//! The bytes are parsed by the same importer as a file the user picks, so a sample can never
//! take a path a real script cannot.

use crate::{ImportOutcome, SourceFormat, import_bytes};
use openframe_domain::{AppError, AppResult};

/// One embedded sample script.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Stable id used by the app ("example-feature").
    pub id: &'static str,
    /// Name shown in the app.
    pub title: &'static str,
    /// File name used for the imported draft and the import report.
    pub file_name: &'static str,
    pub format: SourceFormat,
    /// One line for the sample picker.
    pub description: &'static str,
    /// Scene count of the file (kept in step with `templates/samples/README.md`).
    pub scenes: usize,
    pub bytes: &'static [u8],
}

/// The copyright note every sample carries.
pub const COPYRIGHT_NOTE: &str = "Copyright 2026 OpenFrame Studio";

pub const SAMPLES: &[Sample] = &[
    Sample {
        id: "example-feature",
        title: "Example Feature",
        file_name: "Example-Feature.fountain",
        format: SourceFormat::Fountain,
        description: "A feature film in three acts with many locations, a company move and dialogue in Telugu and Hindi.",
        scenes: 36,
        bytes: include_bytes!("../../../templates/samples/Example-Feature.fountain"),
    },
    Sample {
        id: "example-short",
        title: "Example Short",
        file_name: "Example-Short.fountain",
        format: SourceFormat::Fountain,
        description: "A complete short film around one main location.",
        scenes: 10,
        bytes: include_bytes!("../../../templates/samples/Example-Short.fountain"),
    },
    Sample {
        id: "example-scene",
        title: "Example Scene",
        file_name: "Example-Scene.fountain",
        format: SourceFormat::Fountain,
        description: "One scene that uses every standard screenplay element.",
        scenes: 1,
        bytes: include_bytes!("../../../templates/samples/Example-Scene.fountain"),
    },
    Sample {
        id: "fdx-interchange-test",
        title: "FDX Interchange Test",
        file_name: "FDX-Interchange-Test.fdx",
        format: SourceFormat::Fdx,
        description: "A Final Draft file with revisions, lettered scene numbers and Indian scripts.",
        scenes: 6,
        bytes: include_bytes!("../../../templates/samples/FDX-Interchange-Test.fdx"),
    },
];

/// The sample with this id.
pub fn find(id: &str) -> Option<&'static Sample> {
    SAMPLES.iter().find(|s| s.id == id)
}

/// Parse a sample with the regular importer.
pub fn import(id: &str) -> AppResult<(&'static Sample, ImportOutcome)> {
    let sample = find(id)
        .ok_or_else(|| AppError::invalid_input("Choose one of the sample scripts in the list."))?;
    Ok((sample, import_bytes(sample.bytes, sample.format)?))
}
