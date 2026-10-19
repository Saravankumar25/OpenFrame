//! Safe ZIP handling for packages and DOCX (Threat model: zip-slip, zip bombs,
//! symlinks, oversized entries). All package/document parsers must use these
//! helpers instead of touching `zip` directly.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult};
use zip::write::SimpleFileOptions;

#[derive(Debug, Clone, Copy)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_total_bytes: u64,
    pub max_entry_bytes: u64,
    /// Maximum uncompressed/compressed ratio for any entry larger than 1 MiB.
    pub max_ratio: u64,
}

impl ArchiveLimits {
    /// Project/backup packages: large media allowed.
    pub const PACKAGE: ArchiveLimits = ArchiveLimits {
        max_entries: 200_000,
        max_total_bytes: 64 << 30,
        max_entry_bytes: 16 << 30,
        max_ratio: 200,
    };
    /// Office documents (DOCX) and exchange packages.
    pub const DOCUMENT: ArchiveLimits = ArchiveLimits {
        max_entries: 5_000,
        max_total_bytes: 512 << 20,
        max_entry_bytes: 256 << 20,
        max_ratio: 200,
    };
}

fn rejected(detail: impl Into<String>) -> AppError {
    AppError::import(
        "unsafe_archive",
        "This file isn't a valid OpenFrame package or document, so nothing was imported.",
    )
    .with_detail(detail)
}

/// Hard ceiling for any archive OpenFrame opens (the largest `max_entries` of all limit sets).
const ABSOLUTE_MAX_ENTRIES: u64 = ArchiveLimits::PACKAGE.max_entries as u64;
/// Hard ceiling for the central directory (it is read fully into memory by `zip`).
const ABSOLUTE_MAX_CENTRAL_DIR: u64 = 128 << 20;

fn le_u16(b: &[u8], at: usize) -> Option<u64> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as u64)
}
fn le_u32(b: &[u8], at: usize) -> Option<u64> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as u64)
}
fn le_u64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Entry count and central-directory size as *declared* by the end-of-central-directory
/// record (ZIP64 aware), read without parsing the directory. `None` if not found.
pub fn declared_directory(path: &Path) -> AppResult<Option<(u64, u64)>> {
    use std::io::{Seek, SeekFrom};
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let tail_len = len.min(65_557 + 20);
    f.seek(SeekFrom::Start(len - tail_len))?;
    let mut tail = vec![0u8; tail_len as usize];
    f.read_exact(&mut tail)?;
    let Some(eocd) = tail.windows(4).rposition(|w| w == b"PK\x05\x06") else {
        return Ok(None);
    };
    let (mut entries, mut cd_size) = match (le_u16(&tail, eocd + 10), le_u32(&tail, eocd + 12)) {
        (Some(e), Some(s)) => (e, s),
        _ => return Ok(None),
    };
    // ZIP64: the locator sits 20 bytes before the classic record.
    if (entries == 0xFFFF || cd_size == 0xFFFF_FFFF)
        && eocd >= 20
        && &tail[eocd - 20..eocd - 16] == b"PK\x06\x07"
        && let Some(rec_off) = le_u64(&tail, eocd - 20 + 8)
        && rec_off + 56 <= len
    {
        let mut rec = [0u8; 56];
        f.seek(SeekFrom::Start(rec_off))?;
        f.read_exact(&mut rec)?;
        if &rec[..4] == b"PK\x06\x06" {
            entries = le_u64(&rec, 32).unwrap_or(u64::MAX);
            cd_size = le_u64(&rec, 40).unwrap_or(u64::MAX);
        }
    }
    Ok(Some((entries, cd_size)))
}

/// Refuse archives whose declared directory exceeds `max_entries` *before* `zip` parses it
/// (the whole central directory is materialised in memory; millions of tiny entries in a
/// 100 MB file would cost gigabytes).
pub fn check_directory(path: &Path, max_entries: u64) -> AppResult<()> {
    if let Some((entries, cd_size)) = declared_directory(path)? {
        if entries > max_entries {
            return Err(rejected(format!("{entries} entries exceeds limit")));
        }
        if cd_size > ABSOLUTE_MAX_CENTRAL_DIR {
            return Err(rejected("central directory too large"));
        }
    }
    Ok(())
}

fn open(path: &Path) -> AppResult<zip::ZipArchive<File>> {
    check_directory(path, ABSOLUTE_MAX_ENTRIES)?;
    let f = File::open(path)?;
    let zip = zip::ZipArchive::new(f).map_err(|e| rejected(e.to_string()))?;
    // `zip` keys entries by name and keeps only the last of two equal names, so its count
    // drops below the directory's. Readers disagree about which duplicate is real (one
    // tool shows one document, OpenFrame imports another): refuse the archive.
    if let Some((declared, _)) = declared_directory(path)?
        && declared != zip.len() as u64
    {
        return Err(rejected(format!(
            "directory lists {declared} entries but {} distinct names",
            zip.len()
        )));
    }
    Ok(zip)
}

/// True if one path component looks like a Windows 8.3 short-name alias (`CONTEN~1.JSO`,
/// `PROGRA~1`): up to six characters, `~`, digits, and an extension of up to three
/// characters. Windows resolves such a name to an existing long name in the same folder,
/// so writing it could overwrite or shadow a different file.
fn is_short_name_alias(component: &str) -> bool {
    let (base, ext) = match component.rsplit_once('.') {
        Some((b, e)) => (b, Some(e)),
        None => (component, None),
    };
    let Some((stem, digits)) = base.rsplit_once('~') else {
        return false;
    };
    !stem.is_empty()
        && stem.chars().count() <= 6
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && base.chars().count() <= 8
        && ext.is_none_or(|e| e.chars().count() <= 3 && !e.is_empty())
}

/// Case-insensitive key for collision checks (Windows and default macOS volumes ignore
/// letter case). Unicode normalisation is not applied: NTFS keeps NFC and NFD names apart.
fn fold(name: &str) -> String {
    name.to_lowercase()
}

/// Refuse names that would land on the same file or folder on a case-insensitive file
/// system, and 8.3 short-name aliases. `names` are validated relative paths with `/`.
fn check_name_collisions<'a>(names: impl IntoIterator<Item = &'a str>) -> AppResult<()> {
    use std::collections::HashMap;
    // Folded path -> true for a file, false for a folder implied by a file below it.
    let mut seen: HashMap<String, bool> = HashMap::new();
    for name in names {
        let name = name.replace('\\', "/");
        if name.split('/').any(is_short_name_alias) {
            return Err(rejected(format!("short-name alias entry: {name}")));
        }
        let parts: Vec<&str> = name.split('/').collect();
        for depth in 1..=parts.len() {
            let is_file = depth == parts.len();
            let key = fold(&parts[..depth].join("/"));
            match seen.get(&key) {
                // Two files, or a file and a folder, share one name.
                Some(&existing) if existing || is_file => {
                    return Err(rejected(format!("entry names collide: {name}")));
                }
                Some(_) => {}
                None => {
                    seen.insert(key, is_file);
                }
            }
        }
    }
    Ok(())
}

/// Pre-flight every entry against the limits without extracting anything.
pub fn inspect(path: &Path, limits: ArchiveLimits) -> AppResult<Vec<String>> {
    let mut zip = open(path)?;
    if zip.len() > limits.max_entries {
        return Err(rejected(format!("{} entries exceeds limit", zip.len())));
    }
    let mut total: u64 = 0;
    let mut names = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i).map_err(|e| rejected(e.to_string()))?;
        let name = entry
            .name()
            .map_err(|_| rejected("entry name is not valid text"))?
            .to_string();
        if entry.is_dir() {
            continue;
        }
        crate::validate_relative(&name)
            .map_err(|_| rejected(format!("unsafe entry name: {name}")))?;
        if let Some(mode) = entry.unix_mode()
            && mode & 0o170000 == 0o120000
        {
            return Err(rejected(format!("symlink entry: {name}")));
        }
        let size = entry.size();
        if size > limits.max_entry_bytes {
            return Err(rejected(format!("entry too large: {name}")));
        }
        let compressed = entry.compressed_size().max(1);
        if size > (1 << 20) && size / compressed > limits.max_ratio {
            return Err(rejected(format!("suspicious compression ratio: {name}")));
        }
        total = total.saturating_add(size);
        if total > limits.max_total_bytes {
            return Err(rejected("archive too large"));
        }
        names.push(name);
    }
    check_name_collisions(names.iter().map(String::as_str))?;
    Ok(names)
}

/// Read one entry fully, enforcing `max_bytes` on the *actual* stream (headers may lie).
pub fn read_entry(path: &Path, name: &str, max_bytes: u64) -> AppResult<Option<Vec<u8>>> {
    let mut zip = open(path)?;
    let entry = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(rejected(e.to_string())),
    };
    let mut buf = Vec::new();
    let read = entry
        .take(max_bytes + 1)
        .read_to_end(&mut buf)
        .map_err(|e| rejected(format!("entry {name} can't be read: {e}")))?;
    if read as u64 > max_bytes {
        return Err(rejected(format!("entry {name} exceeds limit")));
    }
    Ok(Some(buf))
}

/// Extract into `dest` (which must not exist yet). Validates every entry first,
/// then streams with hard byte limits. On any failure the partial directory is removed.
pub fn extract_all(path: &Path, dest: &Path, limits: ArchiveLimits) -> AppResult<Vec<PathBuf>> {
    if dest.exists() {
        return Err(AppError::internal("extraction destination already exists"));
    }
    inspect(path, limits)?;
    fs::create_dir_all(dest)?;
    let result = (|| -> AppResult<Vec<PathBuf>> {
        let mut zip = open(path)?;
        let mut written = Vec::new();
        let mut total: u64 = 0;
        for i in 0..zip.len() {
            let entry = zip.by_index(i).map_err(|e| rejected(e.to_string()))?;
            if entry.is_dir() {
                continue;
            }
            let name = entry
                .name()
                .map_err(|_| rejected("entry name is not valid text"))?;
            let rel = crate::validate_relative(&name).map_err(|_| rejected("unsafe entry name"))?;
            let out_path = dest.join(&rel);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            // `create_new`: two entries never write the same file (defence in depth behind
            // the duplicate and case-collision checks).
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out_path)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::AlreadyExists => rejected("two entries share one name"),
                    _ => e.into(),
                })?;
            // The declared size was checked by `inspect`; the real stream may not exceed it
            // (a "lying header" declares 1 KB and inflates to gigabytes). Bytes are counted
            // while inflating, so the excess is never written.
            let remaining = limits.max_total_bytes.saturating_sub(total);
            let cap = entry.size().min(limits.max_entry_bytes).min(remaining);
            // A read error is the archive's fault (damaged data, a CRC that does not match):
            // report a damaged package, not a failed save. Write errors stay storage errors.
            let mut reader = entry.take(cap + 1);
            let mut buf = vec![0u8; 64 * 1024];
            let mut copied: u64 = 0;
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        return Err(rejected(format!("entry can't be read: {e}")));
                    }
                };
                out.write_all(&buf[..n])?;
                copied += n as u64;
            }
            if copied > cap {
                return Err(rejected(
                    "entry inflated past its declared size during extraction",
                ));
            }
            total += copied;
            out.sync_all()?;
            written.push(rel);
        }
        Ok(written)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(dest);
    }
    result
}

/// Source for an archive entry.
pub enum EntrySource<'a> {
    Bytes(&'a [u8]),
    File(&'a Path),
}

/// Write a ZIP atomically through the shared atomic replace (`openframe_platform::atomic`):
/// the archive is staged under a unique temp name in the destination folder, then renamed
/// over `dest` (never deleted first). Entry names are validated. On any failure the staging
/// file is removed and `dest` is left as it was.
pub fn write_zip<'a>(
    dest: &Path,
    entries: impl IntoIterator<Item = (String, EntrySource<'a>)>,
) -> AppResult<()> {
    use openframe_platform::atomic;
    let parent = dest.parent().unwrap_or(Path::new(""));
    let staged = atomic::unique_temp_path(parent, dest)?;
    let result = (|| -> AppResult<()> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true)
            .unix_permissions(0o644);
        for (name, source) in entries {
            crate::validate_relative(&name)?;
            zip.start_file(name.replace('\\', "/"), opts)
                .map_err(|e| AppError::storage(e.to_string()))?;
            match source {
                EntrySource::Bytes(b) => zip.write_all(b)?,
                EntrySource::File(p) => {
                    let mut f = File::open(p)?;
                    std::io::copy(&mut f, &mut zip)?;
                }
            }
        }
        zip.finish().map_err(|e| AppError::storage(e.to_string()))?;
        Ok(())
    })();
    match result {
        // Syncs, then renames over `dest`; removes the staging file if that fails.
        Ok(()) => Ok(atomic::replace_from_staged(&staged, dest)?),
        Err(e) => {
            let _ = fs::remove_file(&staged);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let f = File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        for (n, b) in entries {
            z.start_file(*n, SimpleFileOptions::default()).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("p.zip");
        write_zip(
            &zp,
            vec![("a/b.txt".to_string(), EntrySource::Bytes(b"hello"))],
        )
        .unwrap();
        assert_eq!(read_entry(&zp, "a/b.txt", 100).unwrap().unwrap(), b"hello");
        let out = dir.path().join("out");
        let files = extract_all(&zp, &out, ArchiveLimits::DOCUMENT).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(fs::read(out.join("a").join("b.txt")).unwrap(), b"hello");
    }

    /// Bytes damaged inside an entry's data (a bad disk or a partial download) are reported as
    /// an unsafe archive, never as a failed save, and the partial extraction is removed.
    #[test]
    fn damaged_entry_data_is_a_rejected_archive_not_a_storage_error() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("p.zip");
        let text: Vec<u8> = (0..20_000u32)
            .map(|i| b"abcdefgh"[(i % 7) as usize])
            .collect();
        {
            let mut z = zip::ZipWriter::new(File::create(&zp).unwrap());
            let stored =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            z.start_file("story.txt", stored).unwrap();
            z.write_all(&text).unwrap();
            z.finish().unwrap();
        }
        let mut bytes = fs::read(&zp).unwrap();
        // Local header (30 bytes) + name (9 bytes), then the stored data: change a byte in it.
        bytes[30 + 9 + 5000] ^= 0x20;
        fs::write(&zp, &bytes).unwrap();
        let out = dir.path().join("out");
        let err = extract_all(&zp, &out, ArchiveLimits::DOCUMENT).unwrap_err();
        assert_eq!(err.code_str(), "import.unsafe_archive", "{err:?}");
        assert!(!out.exists());
        let err = read_entry(&zp, "story.txt", 1 << 20).unwrap_err();
        assert_eq!(err.code_str(), "import.unsafe_archive", "{err:?}");
    }

    #[test]
    fn writing_over_an_existing_archive_replaces_it_atomically() {
        // `rename` replaces the destination in one step; no delete-then-rename window.
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("p.zip");
        write_zip(
            &zp,
            vec![("a.txt".to_string(), EntrySource::Bytes(b"first"))],
        )
        .unwrap();
        write_zip(
            &zp,
            vec![("a.txt".to_string(), EntrySource::Bytes(b"second"))],
        )
        .unwrap();
        assert_eq!(read_entry(&zp, "a.txt", 100).unwrap().unwrap(), b"second");
        // Nothing but the archive is left in the folder (no staging file).
        let left: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, [std::ffi::OsString::from("p.zip")]);
    }

    #[test]
    fn a_failed_zip_write_leaves_the_old_archive_and_no_staging_file() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("p.zip");
        write_zip(&zp, vec![("a.txt".to_string(), EntrySource::Bytes(b"old"))]).unwrap();
        let err = write_zip(
            &zp,
            vec![("../escape.txt".to_string(), EntrySource::Bytes(b"new"))],
        );
        assert!(err.is_err());
        assert_eq!(read_entry(&zp, "a.txt", 100).unwrap().unwrap(), b"old");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn zip_slip_rejected_and_nothing_written() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("evil.zip");
        raw_zip(&zp, &[("ok.txt", b"x"), ("../../escape.txt", b"pwned")]);
        let out = dir.path().join("out");
        assert!(extract_all(&zp, &out, ArchiveLimits::DOCUMENT).is_err());
        assert!(!out.exists());
        assert!(!dir.path().join("escape.txt").exists());
    }

    #[test]
    fn absolute_and_drive_paths_rejected() {
        let dir = tempfile::tempdir().unwrap();
        for bad in ["/abs.txt", "C:/win.txt"] {
            let zp = dir.path().join("b.zip");
            raw_zip(&zp, &[(bad, b"x")]);
            assert!(inspect(&zp, ArchiveLimits::DOCUMENT).is_err(), "{bad}");
        }
    }

    #[test]
    fn oversized_entry_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("big.zip");
        let big = vec![0u8; 3 << 20];
        raw_zip(&zp, &[("big.bin", &big)]);
        let limits = ArchiveLimits {
            max_entry_bytes: 1 << 20,
            ..ArchiveLimits::DOCUMENT
        };
        assert!(inspect(&zp, limits).is_err());
        let ratio = ArchiveLimits {
            max_ratio: 10,
            ..ArchiveLimits::DOCUMENT
        };
        assert!(inspect(&zp, ratio).is_err());
    }

    /// A deflated entry whose headers declare `declared` bytes while the stream inflates to
    /// `real` bytes (a "lying header" decompression bomb): written by `zip::ZipWriter`, then
    /// the uncompressed-size fields of the local and central headers are overwritten.
    fn lying_zip(path: &Path, name: &str, real: usize, declared: u32) {
        let mut a = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut a));
            let opts =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            z.start_file(name, opts).unwrap();
            z.write_all(&vec![0u8; real]).unwrap();
            z.finish().unwrap();
        }
        assert_eq!(&a[..4], b"PK\x03\x04");
        a[22..26].copy_from_slice(&declared.to_le_bytes());
        let eocd = a.windows(4).rposition(|w| w == b"PK\x05\x06").unwrap();
        let cd = u32::from_le_bytes(a[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        assert_eq!(&a[cd..cd + 4], b"PK\x01\x02");
        a[cd + 24..cd + 28].copy_from_slice(&declared.to_le_bytes());
        fs::write(path, a).unwrap();
    }

    #[test]
    fn lying_header_bomb_stops_at_the_declared_size() {
        // Declares 1 KB, inflates to 8 MiB. Below the 1 MiB ratio threshold the declared
        // size is all `inspect` can see, so extraction must count the real bytes and stop
        // at the declared size (with PACKAGE limits the old cap was 16 GiB per entry).
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("liar.zip");
        lying_zip(&zp, "zeros.bin", 8 << 20, 1_000);
        assert!(
            inspect(&zp, ArchiveLimits::PACKAGE).is_ok(),
            "headers look small"
        );
        let out = dir.path().join("out");
        let err = extract_all(&zp, &out, ArchiveLimits::PACKAGE).unwrap_err();
        assert_eq!(err.code_str(), "import.unsafe_archive");
        assert!(!out.exists(), "the partial extraction is removed");
    }

    #[test]
    fn honest_entry_of_exactly_the_declared_size_extracts() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("honest.zip");
        lying_zip(&zp, "zeros.bin", 4_096, 4_096);
        let out = dir.path().join("out");
        extract_all(&zp, &out, ArchiveLimits::DOCUMENT).unwrap();
        assert_eq!(fs::read(out.join("zeros.bin")).unwrap().len(), 4_096);
    }

    #[test]
    fn duplicate_entry_names_are_refused() {
        // `zip` keeps only the last of two equal names, so readers would disagree about
        // which part is real. The directory count no longer matches: refuse.
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("dup.zip");
        let mut a = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut a));
            z.start_file("same.txt", SimpleFileOptions::default())
                .unwrap();
            z.write_all(b"first").unwrap();
            z.finish().unwrap();
        }
        // Splice: local header + data of the single entry twice, central record twice.
        let eocd = a.windows(4).rposition(|w| w == b"PK\x05\x06").unwrap();
        let cd_off = u32::from_le_bytes(a[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        let local = a[..cd_off].to_vec();
        let mut central = a[cd_off..eocd].to_vec();
        let mut second = central.clone();
        second[42..46].copy_from_slice(&(local.len() as u32).to_le_bytes());
        central.extend(second);
        let mut out = local.clone();
        out.extend(&local);
        let new_cd = out.len() as u32;
        out.extend(&central);
        out.extend(0x0605_4b50u32.to_le_bytes());
        out.extend([0u8; 4]);
        out.extend(2u16.to_le_bytes());
        out.extend(2u16.to_le_bytes());
        out.extend((central.len() as u32).to_le_bytes());
        out.extend(new_cd.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        fs::write(&zp, out).unwrap();
        for e in [
            inspect(&zp, ArchiveLimits::DOCUMENT).unwrap_err(),
            read_entry(&zp, "same.txt", 100).unwrap_err(),
        ] {
            assert_eq!(e.code_str(), "import.unsafe_archive");
        }
        assert!(!dir.path().join("out").exists());
    }

    #[test]
    fn case_colliding_names_are_refused() {
        // On Windows (and default macOS volumes) these name the same file, so the second
        // entry would silently overwrite the first.
        let dir = tempfile::tempdir().unwrap();
        for (i, pair) in [
            [("Readme.txt", b"one"), ("README.TXT", b"two")],
            [("Assets/a.txt", b"one"), ("assets/A.TXT", b"two")],
            [("folder", b"one"), ("FOLDER/x.txt", b"two")],
        ]
        .iter()
        .enumerate()
        {
            let zp = dir.path().join(format!("c{i}.zip"));
            raw_zip(&zp, &[(pair[0].0, pair[0].1), (pair[1].0, pair[1].1)]);
            let e = inspect(&zp, ArchiveLimits::PACKAGE).unwrap_err();
            assert_eq!(e.code_str(), "import.unsafe_archive", "{pair:?}");
            let out = dir.path().join(format!("out{i}"));
            assert!(extract_all(&zp, &out, ArchiveLimits::PACKAGE).is_err());
            assert!(!out.exists());
        }
        // Different names that only look alike stay allowed.
        let zp = dir.path().join("ok.zip");
        raw_zip(
            &zp,
            &[("a.txt", b"1"), ("a.txt.bak", b"2"), ("b/a.txt", b"3")],
        );
        inspect(&zp, ArchiveLimits::PACKAGE).unwrap();
    }

    #[test]
    fn short_name_aliases_are_refused() {
        // `CONTEN~1.JSO` is the 8.3 alias Windows gives `content.json`: writing it would
        // overwrite (or shadow) the real file.
        let dir = tempfile::tempdir().unwrap();
        for bad in [
            "CONTEN~1.JSO",
            "assets/PROGRA~1/x.txt",
            "a~12",
            "ABCDEF~9.TXT",
        ] {
            let zp = dir.path().join("s.zip");
            raw_zip(&zp, &[("content.json", b"{}"), (bad, b"alias")]);
            let e = inspect(&zp, ArchiveLimits::PACKAGE).unwrap_err();
            assert_eq!(e.code_str(), "import.unsafe_archive", "{bad}");
        }
        // A tilde that cannot be an 8.3 alias is an ordinary character.
        for ok in [
            "notes~draft.txt",
            "~temp.txt",
            "backup~.txt",
            "a~1b.txt",
            "longname~1.txt",
        ] {
            let zp = dir.path().join("t.zip");
            raw_zip(&zp, &[(ok, b"x")]);
            inspect(&zp, ArchiveLimits::PACKAGE).unwrap_or_else(|e| panic!("{ok}: {e:?}"));
        }
    }

    #[test]
    fn garbage_is_rejected_humanely() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("x.zip");
        fs::write(&zp, b"not a zip").unwrap();
        let err = inspect(&zp, ArchiveLimits::DOCUMENT).unwrap_err();
        assert_eq!(err.code_str(), "import.unsafe_archive");
    }
}
