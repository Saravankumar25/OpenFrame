//! Regression tests for the 2026-10-05 security baseline (docs/release/evidence/M0/M0.28-security-baseline.md):
//! E3 / link and junction safety (write destinations resolve the nearest existing ancestor through junctions and
//! symlinks) and E4 / network drive refusal (mapped network drives and paths that resolve to a share are refused).

use std::path::{Path, PathBuf};

use openframe_security::{
    DriveKind, NETWORK_LOCATION_CODE, NetworkPaths, check_user_path, confine, drive_kind,
    is_local_disk_path, is_network_location, validate_output_dir,
};

/// `mklink /J` (directory junctions need no privileges). False if the system refuses.
fn junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A project folder and an unrelated folder outside it, both inside one temp dir.
fn project_and_outside() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("Film.openframe");
    let outside = dir.path().join("Elsewhere");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    (dir, project, outside)
}

fn assert_escape(r: openframe_domain::AppResult<PathBuf>, what: &str) {
    match r {
        Ok(p) => panic!("{what}: confined to {} instead of refusing", p.display()),
        Err(e) => assert_eq!(e.code_str(), "security.path_escape", "{what}"),
    }
}

#[cfg(windows)]
#[test]
fn new_files_below_a_junctioned_assets_folder_are_refused() {
    let (_dir, project, outside) = project_and_outside();
    if !junction(&project.join("assets"), &outside) {
        eprintln!("SKIPPED: mklink /J is unavailable on this system");
        return;
    }
    // The destination does not exist yet: before link and junction safety only existing paths were resolved, so
    // a new managed file was written into `Elsewhere`.
    assert_escape(
        confine(&project, "assets/ab/new.png"),
        "new file in a new shard",
    );
    assert_escape(
        confine(&project, "assets/new.png"),
        "new file in the junction",
    );
    assert_escape(confine(&project, "assets"), "the junction itself");
    // An existing file behind the junction stays refused, as before.
    std::fs::write(outside.join("old.png"), b"x").unwrap();
    assert_escape(confine(&project, "assets/old.png"), "existing file");
    // Nothing was created outside by the checks.
    assert!(!outside.join("ab").exists());
}

#[cfg(windows)]
#[test]
fn a_junction_deeper_inside_assets_is_refused_too() {
    let (_dir, project, outside) = project_and_outside();
    std::fs::create_dir_all(project.join("assets")).unwrap();
    if !junction(&project.join("assets").join("ab"), &outside) {
        eprintln!("SKIPPED: mklink /J is unavailable on this system");
        return;
    }
    assert_escape(confine(&project, "assets/ab/cd/new.png"), "nested junction");
    // A sibling shard without a link is fine.
    let ok = confine(&project, "assets/cd/new.png").unwrap();
    assert!(ok.starts_with(&project));
}

#[cfg(windows)]
#[test]
fn a_junction_that_stays_inside_the_project_is_allowed() {
    let (_dir, project, _outside) = project_and_outside();
    let real = project.join("media");
    std::fs::create_dir_all(&real).unwrap();
    if !junction(&project.join("assets"), &real) {
        eprintln!("SKIPPED: mklink /J is unavailable on this system");
        return;
    }
    assert!(confine(&project, "assets/ab/new.png").is_ok());
}

#[cfg(windows)]
#[test]
fn a_project_opened_through_a_junction_still_confines_normally() {
    let (dir, project, outside) = project_and_outside();
    std::fs::create_dir_all(project.join("assets")).unwrap();
    let alias = dir.path().join("Alias.openframe");
    if !junction(&alias, &project) {
        eprintln!("SKIPPED: mklink /J is unavailable on this system");
        return;
    }
    // The root itself is a link: both sides resolve, so ordinary files are accepted...
    let p = confine(&alias, "assets/ab/new.png").unwrap();
    assert!(p.starts_with(&alias));
    // ...and an escape below it is still caught.
    assert!(junction(&project.join("assets").join("zz"), &outside));
    assert_escape(
        confine(&alias, "assets/zz/new.png"),
        "escape below aliased root",
    );
}

#[test]
fn plain_destinations_are_unchanged() {
    let (_dir, project, _outside) = project_and_outside();
    // Nothing below the root exists yet, or only real folders: allowed, and nothing is created.
    let p = confine(&project, "assets/ab/new.png").unwrap();
    assert_eq!(p, project.join("assets").join("ab").join("new.png"));
    assert!(!project.join("assets").exists());
    std::fs::create_dir_all(project.join("assets").join("ab")).unwrap();
    std::fs::write(project.join("assets").join("ab").join("old.png"), b"x").unwrap();
    assert!(confine(&project, "assets/ab/old.png").is_ok());
    assert!(confine(&project, "assets/ab/new.png").is_ok());
    // A root that does not exist yet cannot contain a link.
    assert!(confine(&project.join("missing"), "assets/x.png").is_ok());
    // Syntax is still refused first.
    assert!(confine(&project, "../x").is_err());
    assert!(confine(&project, "assets/../../x").is_err());
}

#[cfg(windows)]
#[test]
fn a_dangling_symlink_is_refused_when_symlinks_can_be_created() {
    let (_dir, project, outside) = project_and_outside();
    std::fs::create_dir_all(project.join("assets")).unwrap();
    let link = project.join("assets").join("new.png");
    // Symlinks need Developer Mode or elevation; junctions cover the default case above.
    if std::os::windows::fs::symlink_file(outside.join("created-by-write.png"), &link).is_err() {
        eprintln!("SKIPPED: file symlinks need Developer Mode or elevation on this system");
        return;
    }
    assert_escape(confine(&project, "assets/new.png"), "dangling symlink");
}

// ------------------------------------------------------------------ network drive refusal network locations

#[cfg(windows)]
#[test]
fn the_local_system_drive_is_not_refused() {
    let temp = std::env::temp_dir().canonicalize().unwrap();
    let shown = openframe_security::display_path(&temp);
    let letter = shown.to_string_lossy().as_bytes()[0];
    assert_eq!(
        drive_kind(letter),
        DriveKind::Other,
        "the temp folder is on a local drive"
    );
    assert!(check_user_path(&shown, NetworkPaths::Refuse).is_ok());
    assert!(is_local_disk_path(&shown.join("x.png")));
    assert!(!is_network_location(&shown));
    assert!(!is_network_location(
        &shown.join("Not Yet").join("Film.openframe")
    ));
    // Not a drive letter at all.
    assert_eq!(drive_kind(b'1'), DriveKind::Other);
}

#[cfg(windows)]
#[test]
fn unc_project_locations_are_network_locations() {
    for unc in [
        r"\\server\share\Film.openframe",
        r"\\?\UNC\server\share\Film.openframe",
        "//server/share/Film.openframe",
    ] {
        assert!(is_network_location(Path::new(unc)), "{unc}");
    }
}

#[cfg(windows)]
#[test]
fn a_local_folder_that_resolves_to_a_share_is_refused_when_symlinks_can_be_created() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("Projects");
    // Junctions cannot point at a share; a directory symlink can (Developer Mode or elevation).
    if std::os::windows::fs::symlink_dir(r"\\localhost\C$\Windows", &link).is_err() {
        eprintln!("SKIPPED: directory symlinks need Developer Mode or elevation on this system");
        return;
    }
    if !link.join("System32").exists() {
        eprintln!("SKIPPED: the administrative share \\\\localhost\\C$ is not reachable");
        return;
    }
    assert!(is_network_location(&link.join("Film.openframe")));
    let e = validate_output_dir(&link.join("New"), &[]).unwrap_err();
    assert_eq!(e.code_str(), NETWORK_LOCATION_CODE);
}
