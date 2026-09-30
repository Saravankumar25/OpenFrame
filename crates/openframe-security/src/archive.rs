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
    zip::ZipArchive::new(f).map_err(|e| rejected(e.to_string()))
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
        let name = entry.name().to_string();
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
    let read = entry.take(max_bytes + 1).read_to_end(&mut buf)?;
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
            let rel = crate::validate_relative(entry.name())
                .map_err(|_| rejected("unsafe entry name"))?;
            let out_path = dest.join(&rel);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&out_path)?;
            let remaining = limits.max_total_bytes.saturating_sub(total);
            let cap = remaining.min(limits.max_entry_bytes);
            let copied = std::io::copy(&mut entry.take(cap + 1), &mut out)?;
            if copied > cap {
                return Err(rejected("entry exceeded declared limits during extraction"));
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

/// Write a ZIP atomically (temp file + rename). Entry names are validated.
pub fn write_zip<'a>(
    dest: &Path,
    entries: impl IntoIterator<Item = (String, EntrySource<'a>)>,
) -> AppResult<()> {
    let tmp = dest.with_extension("partial");
    let result = (|| -> AppResult<()> {
        let file = File::create(&tmp)?;
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
        let file = zip.finish().map_err(|e| AppError::storage(e.to_string()))?;
        file.sync_all()?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            if dest.exists() {
                fs::remove_file(dest)?;
            }
            fs::rename(&tmp, dest)?;
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
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

    #[test]
    fn garbage_is_rejected_humanely() {
        let dir = tempfile::tempdir().unwrap();
        let zp = dir.path().join("x.zip");
        fs::write(&zp, b"not a zip").unwrap();
        let err = inspect(&zp, ArchiveLimits::DOCUMENT).unwrap_err();
        assert_eq!(err.code_str(), "import.unsafe_archive");
    }
}
