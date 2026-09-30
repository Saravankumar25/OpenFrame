//! Regression tests for the 2026-09-30 security review (docs/engineering/SECURITY_REVIEW_2026-09-30.md):
//! user-chosen path validation (PATH-01..04), shell-open safety (PKG-01), web links (WEB-01),
//! log redaction (LOG-01) and archive directory limits (PARSE-03).

use std::path::{Path, PathBuf};

use openframe_security::{
    NetworkPaths, archive, check_user_path, is_dangerous_to_open, is_local_disk_path,
    is_reserved_name, panic_summary, sanitize_file_name, validate_input_file, validate_output_dir,
    validate_output_file, web_link,
};

#[cfg(windows)]
#[test]
fn device_verbatim_and_relative_paths_are_rejected_before_any_filesystem_access() {
    for bad in [
        r"\\.\PhysicalDrive0",
        r"\\.\pipe\openframe",
        r"\\?\GLOBALROOT\Device\Mup\attacker\share\x.png",
        r"\\?\Volume{00000000-0000-0000-0000-000000000000}\x",
        r"C:\Users\x\NUL",
        r"C:\Users\x\com1.txt",
        r"C:\Users\x\CON .log",
        r"C:\Users\x\CONIN$",
        r"C:\Users\x\notes.txt:secret",
        r"C:\Users\x\..\y\a.txt",
        r"relative\a.txt",
        r"\rooted-without-drive.txt",
        "C:\\Users\\x\\a\u{7}.txt",
        "",
    ] {
        assert!(
            check_user_path(Path::new(bad), NetworkPaths::Allow).is_err(),
            "{bad} must be rejected"
        );
    }
    assert!(
        check_user_path(
            Path::new(r"C:\Users\x\Documents\Script.pdf"),
            NetworkPaths::Refuse
        )
        .is_ok()
    );
    assert!(check_user_path(Path::new(r"\\?\C:\Users\x\a.pdf"), NetworkPaths::Refuse).is_ok());
}

#[cfg(windows)]
#[test]
fn network_paths_follow_the_policy_including_slash_variants() {
    // Rust (and Win32) treat '/' like '\' in the prefix: all of these are UNC shares.
    for unc in [
        r"\\attacker\share\x.png",
        r"\/attacker/share/x.png",
        r"//attacker/share/x.png",
        r"\\?\UNC\attacker\share\x.png",
    ] {
        assert!(
            check_user_path(Path::new(unc), NetworkPaths::Refuse).is_err(),
            "{unc} must be refused"
        );
        assert!(!is_local_disk_path(Path::new(unc)), "{unc} is not local");
    }
    // A user may deliberately pick a file on a NAS in a dialog.
    assert!(check_user_path(Path::new(r"\\nas\share\script.fdx"), NetworkPaths::Allow).is_ok());
    assert!(is_local_disk_path(Path::new(r"D:\Media\a.png")));
}

#[test]
fn reserved_names_cover_all_windows_devices() {
    for n in [
        "CON", "nul.txt", "COM9.log", "lpt1", "CONOUT$", "clock$", "COM¹", "AUX .x", "com0",
    ] {
        assert!(is_reserved_name(n), "{n}");
        assert_ne!(sanitize_file_name(n), n, "{n} is sanitized");
    }
    for n in ["CONSOLE.txt", "Nullable.pdf", "COM10", "Script.pdf"] {
        assert!(!is_reserved_name(n), "{n}");
    }
}

#[test]
fn output_files_are_never_written_into_protected_folders() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("Film.openframe");
    let appdata = dir.path().join("appdata");
    let docs = dir.path().join("Documents");
    for d in [&project, &appdata, &docs, &project.join("assets")] {
        std::fs::create_dir_all(d).unwrap();
    }
    let protected: [&Path; 2] = [&project, &appdata];
    assert!(validate_output_file(&docs.join("Film.pdf"), &protected).is_ok());
    for bad in [
        project.join("openframe.json"),
        project.join("assets").join("x.pdf"),
        appdata.join("app.sqlite"),
    ] {
        let e = validate_output_file(&bad, &protected).unwrap_err();
        assert_eq!(
            e.code_str(),
            "validation.invalid_input",
            "{}",
            bad.display()
        );
    }
    // Missing folder, a folder as the target, and non-portable names.
    assert_eq!(
        validate_output_file(&docs.join("nope").join("a.pdf"), &protected)
            .unwrap_err()
            .code_str(),
        "not_found.folder"
    );
    assert!(validate_output_file(&docs, &protected).is_err());
    assert!(validate_output_file(&docs.join("CON.pdf"), &protected).is_err());
    assert!(validate_output_file(&docs.join("a.pdf."), &protected).is_err());
    // New project parent folders: not inside the project, but a new subfolder elsewhere is fine.
    assert!(validate_output_dir(&project.join("nested"), &protected).is_err());
    assert!(validate_output_dir(&docs.join("New Folder"), &protected).is_ok());
}

#[cfg(windows)]
#[test]
fn junction_into_a_protected_folder_is_resolved_before_the_check() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("Film.openframe");
    std::fs::create_dir_all(&project).unwrap();
    let link = dir.path().join("innocent");
    // Directory junctions need no privileges (unlike symlinks).
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&project)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("mklink /J unavailable; skipping");
        return;
    }
    let protected: [&Path; 1] = [&project];
    assert!(validate_output_file(&link.join("openframe.json"), &protected).is_err());
}

#[test]
fn input_files_must_be_regular_files_within_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("a.fdx");
    std::fs::write(&f, b"12345").unwrap();
    assert_eq!(validate_input_file(&f, 10).unwrap(), 5);
    assert!(validate_input_file(&f, 4).is_err());
    assert!(validate_input_file(dir.path(), 10).is_err());
    assert!(
        validate_input_file(Path::new("a.fdx"), 10).is_err(),
        "relative"
    );
    #[cfg(windows)]
    assert!(validate_input_file(Path::new(r"\\.\pipe\x"), 10).is_err());
}

#[test]
fn executables_are_never_shell_opened() {
    for bad in [
        "photo.exe",
        "photo.jpg.exe",
        "a.HTA",
        "run.bat",
        "x.cmd",
        "s.ps1",
        "l.lnk",
        "u.url",
        "i.msi",
        "j.js",
        "v.vbs",
        "a.scr",
        "noextension",
        "a.exe.",
        "a.exe  ",
        "c.cpl",
        "r.reg",
        "d.library-ms",
        "s.settingcontent-ms",
        "p.appref-ms",
    ] {
        assert!(is_dangerous_to_open(&PathBuf::from(bad)), "{bad}");
    }
    for ok in [
        "photo.jpg",
        "script.pdf",
        "memo.m4a",
        "clip.mp4",
        "notes.txt",
        "sheet.xlsx",
        "a.docx",
    ] {
        assert!(!is_dangerous_to_open(&PathBuf::from(ok)), "{ok}");
    }
}

#[test]
fn only_plain_web_links_reach_the_browser() {
    assert_eq!(
        web_link(" https://example.com/a?b=1 "),
        Some("https://example.com/a?b=1")
    );
    assert!(web_link("HTTP://Example.com").is_some());
    for bad in [
        "javascript:alert(1)",
        "file:///C:/Windows/System32/calc.exe",
        "ms-settings:privacy",
        "search-ms:query=x",
        "https://",
        "https:///path",
        "https://user:pw@example.com/",
        "https://example.com/a b",
        "https://example.com/\"--x",
        "https://example.com\\@evil",
        "\\\\attacker\\share",
    ] {
        assert!(web_link(bad).is_none(), "{bad}");
    }
    assert!(web_link(&format!("https://e.com/{}", "a".repeat(5000))).is_none());
}

#[test]
fn panic_log_lines_are_redacted_and_bounded() {
    let s = panic_summary(
        &format!(
            "called unwrap on C:\\Users\\Ravi Kumar\\Film.openframe token=abc123 {}",
            "x".repeat(1000)
        ),
        r"C:\Users\Ravi Kumar\.cargo\registry\src\index\lopdf-0.36.0\src\parser.rs:12:5",
    );
    assert!(!s.contains("Ravi"), "{s}");
    assert!(!s.contains("abc123"), "{s}");
    assert!(s.len() < 450, "{}", s.len());
    assert!(s.contains("parser.rs"));
}

#[test]
fn archives_declaring_too_many_entries_are_refused_before_parsing() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("many.docx");
    {
        let f = std::fs::File::create(&p).unwrap();
        let mut z = zip::ZipWriter::new(f);
        for i in 0..40 {
            z.start_file(
                format!("f{i}.xml"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            z.write_all(b"x").unwrap();
        }
        z.finish().unwrap();
    }
    assert_eq!(archive::declared_directory(&p).unwrap().unwrap().0, 40);
    assert!(archive::check_directory(&p, 50).is_ok());
    let e = archive::check_directory(&p, 10).unwrap_err();
    assert_eq!(e.code_str(), "import.unsafe_archive");
    // A lying end-of-central-directory record is caught too (declares 65535 entries).
    let mut bytes = std::fs::read(&p).unwrap();
    let eocd = bytes.windows(4).rposition(|w| w == b"PK\x05\x06").unwrap();
    bytes[eocd + 10] = 0xFE;
    bytes[eocd + 11] = 0xFF;
    std::fs::write(&p, &bytes).unwrap();
    assert!(archive::check_directory(&p, 5_000).is_err());
}
