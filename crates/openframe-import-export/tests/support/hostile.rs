//! Shared support for the hostile-input corpus and the seeded mutation fuzzer (hostile file collection).
//!
//! Included by `tests/hostile_corpus.rs` through `#[path]`, so it depends only on `std`.
//! No fuzzing crate is used (adding one needs owner approval): the fuzzer is a
//! deterministic xorshift mutator over the committed corpus.
//!
//! Environment overrides for longer local runs:
//! - `OPENFRAME_FUZZ_ITERS`: iterations per parser (default: per-parser CI budget)
//! - `OPENFRAME_FUZZ_SEED`: base seed (decimal or 0x-hex; default fixed so CI is reproducible)
//! - `OPENFRAME_FUZZ_TRACE=1`: write every input to the reproducer folder *before* running
//!   it, so an abort (stack overflow, out of memory) that kills the process leaves the input.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const DEFAULT_SEED: u64 = 0x0F_EA_11_5E_ED_15_00_15;

// ------------------------------------------------------------------ corpus

/// `crates/openframe-import-export/tests/fixtures/hostile` (works from any including crate).
pub fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("openframe-import-export")
        .join("tests")
        .join("fixtures")
        .join("hostile")
}

/// Every committed corpus file of one format, sorted by name: `(file name, bytes)`.
pub fn corpus(format: &str) -> Vec<(String, Vec<u8>)> {
    let dir = corpus_root().join(format);
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("corpus folder {} missing: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let bytes = std::fs::read(e.path()).unwrap();
            (name, bytes)
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "corpus folder {} is empty", dir.display());
    out
}

/// File name a hostile archive tries to write outside its destination. Tests check that
/// no file of this name appears anywhere near the extraction.
pub const ESCAPE_MARKER: &str = "OPENFRAME_HOSTILE_ESCAPE.txt";

/// Fail if [`ESCAPE_MARKER`] exists in `dir` or any of its ancestors (up to 4 levels),
/// the system temp folder or the current directory.
#[track_caller]
pub fn assert_no_escape(dir: &Path) {
    let mut places: Vec<PathBuf> = dir.ancestors().take(5).map(Path::to_path_buf).collect();
    places.push(std::env::temp_dir());
    if let Ok(cwd) = std::env::current_dir() {
        places.push(cwd);
    }
    for p in places {
        let marker = p.join(ESCAPE_MARKER);
        assert!(
            std::fs::symlink_metadata(&marker).is_err(),
            "a hostile entry escaped to {}",
            marker.display()
        );
    }
}

/// Relative paths of everything below `dir` (links are listed, never followed).
pub fn tree(dir: &Path) -> BTreeSet<PathBuf> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            out.insert(p.strip_prefix(root).unwrap().to_path_buf());
            let is_real_dir = std::fs::symlink_metadata(&p)
                .map(|m| m.is_dir() && !m.file_type().is_symlink())
                .unwrap_or(false);
            if is_real_dir && !is_reparse_point(&p) {
                walk(root, &p, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(dir, dir, &mut out);
    out
}

#[cfg(windows)]
pub fn is_reparse_point(p: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    std::fs::symlink_metadata(p)
        .map(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn is_reparse_point(p: &Path) -> bool {
    std::fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Create a directory junction `link` → `target` (`mklink /J`). `Err` carries the reason
/// so callers can print a clear skip message.
#[cfg(windows)]
pub fn make_junction(link: &Path, target: &Path) -> Result<(), String> {
    let out = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map_err(|e| format!("could not run mklink: {e}"))?;
    if out.status.success() && is_reparse_point(link) {
        Ok(())
    } else {
        Err(format!(
            "mklink /J failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(not(windows))]
pub fn make_junction(link: &Path, target: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link).map_err(|e| e.to_string())
}

/// Create a file symbolic link (needs Developer Mode or elevation on Windows).
#[cfg(windows)]
pub fn make_file_symlink(link: &Path, target: &Path) -> Result<(), String> {
    std::os::windows::fs::symlink_file(target, link)
        .map_err(|e| format!("file symlinks need Developer Mode or elevation on Windows ({e})"))
}

#[cfg(not(windows))]
pub fn make_file_symlink(link: &Path, target: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ guarded execution

/// Aborts the whole test process if the guarded work is still running at the deadline, so a
/// hang (infinite loop, quadratic blow-up) fails loudly with the case name instead of
/// stalling CI. Dropping the guard disarms it.
pub struct Watchdog {
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Watchdog {
    pub fn arm(what: &str, deadline: Duration, on_fire: Option<(String, Vec<u8>)>) -> Watchdog {
        use std::sync::atomic::Ordering;
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = done.clone();
        let what = what.to_string();
        std::thread::spawn(move || {
            let started = Instant::now();
            while started.elapsed() < deadline {
                if flag.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if flag.load(Ordering::Relaxed) {
                return;
            }
            let saved = on_fire.map(|(name, bytes)| save_reproducer(&name, &bytes));
            eprintln!(
                "HANG: {what} still running after {deadline:?}: unbounded work on hostile input \
                 is a product defect{}",
                saved
                    .map(|p| format!("; input saved to {}", p.display()))
                    .unwrap_or_default()
            );
            std::process::abort();
        });
        Watchdog { done }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Run `f`, converting a panic into a test failure, and fail if it took longer than `limit`.
/// A hang past twice the limit aborts the process with the case name ([`Watchdog`]).
#[track_caller]
pub fn bounded<T>(what: &str, limit: Duration, f: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let dog = Watchdog::arm(what, limit * 2 + Duration::from_secs(5), None);
    let r = catch_unwind(AssertUnwindSafe(f));
    drop(dog);
    let took = started.elapsed();
    match r {
        Ok(v) => {
            assert!(
                took <= limit,
                "{what}: took {took:?}, bound is {limit:?} (unbounded work on hostile input)"
            );
            v
        }
        Err(p) => panic!(
            "{what}: PANICKED ({}); a panic is a product defect",
            panic_text(&p)
        ),
    }
}

pub fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "non-string panic".into())
}

// ------------------------------------------------------------------ deterministic PRNG

/// xorshift64*: tiny, fast, deterministic across platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    pub fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

fn fnv1a(s: &str) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    h
}

fn env_u64(name: &str) -> Option<u64> {
    let v = std::env::var(name).ok()?;
    let v = v.trim();
    if let Some(hex) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        v.parse().ok()
    }
}

/// (base seed, iterations) after environment overrides.
pub fn fuzz_config(default_iters: u64) -> (u64, u64) {
    let seed = env_u64("OPENFRAME_FUZZ_SEED").unwrap_or(DEFAULT_SEED);
    let iters = env_u64("OPENFRAME_FUZZ_ITERS").unwrap_or(default_iters);
    (seed, iters)
}

// ------------------------------------------------------------------ mutation

const INTERESTING: &[u8] = &[
    0x00, 0x01, 0x7F, 0x80, 0xFF, b'<', b'>', b'[', b']', b'(', b')', b'{', b'}', b'"', b'\'',
    b'&', b';', b'/', b'\\', b'%', b'\n', b'\r', b' ', b':', b'.', b'*', b'_', b'#', b'!', b'@',
    b'0', b'9',
];

const NUMBERS: &[&[u8]] = &[
    b"0",
    b"-1",
    b"65535",
    b"65536",
    b"2147483647",
    b"4294967295",
    b"4294967296",
    b"9223372036854775807",
    b"18446744073709551616",
    b"99999999999999999999999999",
    b"1e308",
    b"-1e308",
    b"NaN",
    b"0.000000000000000000001",
];

const RUN_LENGTHS: &[usize] = &[2, 16, 255, 256, 4_096, 65_535, 65_536, 300_000];

/// One mutated input derived from `base`. `pool` provides splice material (other corpus
/// files of the same format), `dict` format tokens. The result never exceeds `max_len`.
pub fn mutate(
    rng: &mut Rng,
    base: &[u8],
    pool: &[Vec<u8>],
    dict: &[&[u8]],
    max_len: usize,
) -> Vec<u8> {
    let mut v = base.to_vec();
    let rounds = 1 + rng.below(4);
    for _ in 0..rounds {
        let len = v.len();
        match rng.below(11) {
            // Bit flips.
            0 if len > 0 => {
                for _ in 0..1 + rng.below(8) {
                    let i = rng.below(len);
                    v[i] ^= 1 << rng.below(8);
                }
            }
            // Interesting bytes.
            1 if len > 0 => {
                let i = rng.below(len);
                v[i] = INTERESTING[rng.below(INTERESTING.len())];
            }
            // Truncation.
            2 => {
                let at = rng.below(len + 1);
                v.truncate(at);
            }
            // Delete a range.
            3 if len > 0 => {
                let a = rng.below(len);
                let b = (a + 1 + rng.below(256)).min(len);
                v.drain(a..b);
            }
            // Splice a slice of another corpus file.
            4 if !pool.is_empty() => {
                let src = &pool[rng.below(pool.len())];
                if !src.is_empty() {
                    let a = rng.below(src.len());
                    let b = (a + 1 + rng.below(4_096)).min(src.len());
                    let at = rng.below(len + 1);
                    v.splice(at..at, src[a..b].iter().copied());
                }
            }
            // Repeated nesting: duplicate a short slice many times in place.
            5 if len > 0 => {
                let a = rng.below(len);
                let b = (a + 1 + rng.below(32)).min(len);
                let piece = v[a..b].to_vec();
                let times = [2, 10, 100, 300, 1_000, 5_000][rng.below(6)];
                let mut grown = Vec::with_capacity(piece.len() * times);
                for _ in 0..times {
                    grown.extend_from_slice(&piece);
                    if grown.len() > max_len {
                        break;
                    }
                }
                v.splice(a..a, grown);
            }
            // Oversized field: a long run of one byte.
            6 => {
                let n = RUN_LENGTHS[rng.below(RUN_LENGTHS.len())];
                let c = if rng.chance(2) {
                    b'A'
                } else {
                    INTERESTING[rng.below(INTERESTING.len())]
                };
                let at = rng.below(len + 1);
                v.splice(at..at, std::iter::repeat_n(c, n));
            }
            // Format tokens, sometimes repeated (nesting openers, entity refs, filters).
            7 if !dict.is_empty() => {
                let tok = dict[rng.below(dict.len())];
                let times = if rng.chance(3) { 1 + rng.below(500) } else { 1 };
                let at = rng.below(len + 1);
                let mut ins = Vec::with_capacity(tok.len() * times);
                for _ in 0..times {
                    ins.extend_from_slice(tok);
                }
                v.splice(at..at, ins);
            }
            // Replace a digit run with an extreme number.
            8 if len > 0 => {
                let start = rng.below(len);
                if let Some(p) = v[start..].iter().position(u8::is_ascii_digit) {
                    let a = start + p;
                    let b = a + v[a..].iter().take_while(|c| c.is_ascii_digit()).count();
                    let n = NUMBERS[rng.below(NUMBERS.len())];
                    v.splice(a..b, n.iter().copied());
                }
            }
            // Swap two ranges (reorders structure).
            9 if len > 8 => {
                let a = rng.below(len / 2);
                let b = len / 2 + rng.below(len / 2);
                let n = (1 + rng.below(64)).min(len - b).min(b - a);
                for k in 0..n {
                    v.swap(a + k, b + k);
                }
            }
            // Duplicate the tail (doubled trailers / directories).
            _ => {
                if len > 0 {
                    let a = rng.below(len);
                    let tail = v[a..].to_vec();
                    v.extend_from_slice(&tail);
                }
            }
        }
        if v.len() > max_len {
            v.truncate(max_len);
        }
    }
    v
}

/// Shrink a failing input (ddmin-style chunk removal, bounded number of attempts).
pub fn minimize(input: &[u8], mut fails: impl FnMut(&[u8]) -> bool) -> Vec<u8> {
    let mut cur = input.to_vec();
    let mut chunk = cur.len() / 2;
    let mut attempts = 0usize;
    while chunk >= 1 && attempts < 4_000 {
        let mut i = 0;
        let mut removed_any = false;
        while i < cur.len() && attempts < 4_000 {
            let end = (i + chunk).min(cur.len());
            let mut cand = Vec::with_capacity(cur.len() - (end - i));
            cand.extend_from_slice(&cur[..i]);
            cand.extend_from_slice(&cur[end..]);
            attempts += 1;
            if fails(&cand) {
                cur = cand;
                removed_any = true;
            } else {
                i += chunk;
            }
        }
        if !removed_any {
            chunk /= 2;
        }
    }
    cur
}

/// Folder where reproducers are written (outside the repository; commit the minimal one
/// to the corpus as part of the defect fix).
pub fn reproducer_dir() -> PathBuf {
    std::env::temp_dir().join("openframe-fuzz-reproducers")
}

pub fn save_reproducer(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = reproducer_dir();
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(name);
    let _ = std::fs::write(&p, bytes);
    p
}

/// Outcome label of one fuzz input: "ok" or an error code.
pub type Outcome = String;

pub struct FuzzSpec<'a> {
    /// Parser name (also the per-parser seed salt and reproducer file prefix).
    pub name: &'a str,
    pub default_iters: u64,
    /// Wall-time bound for any single input (debug build).
    pub per_input: Duration,
    pub corpus: Vec<Vec<u8>>,
    pub dict: &'a [&'a [u8]],
    pub max_len: usize,
    /// Reproducer file extension.
    pub ext: &'a str,
}

/// Run the seeded mutation fuzzer: first every corpus file unmodified, then `iters`
/// mutated inputs. `target` must return an outcome label and must not panic. A panic or an
/// input over the time bound is minimized, saved and reported with seed and iteration.
/// Returns the outcome histogram (callers assert the fuzzer reached deep code).
pub fn fuzz(spec: FuzzSpec<'_>, target: impl Fn(&[u8]) -> Outcome) -> BTreeMap<Outcome, u64> {
    let (base_seed, iters) = fuzz_config(spec.default_iters);
    let seed = base_seed ^ fnv1a(spec.name);
    let trace = std::env::var("OPENFRAME_FUZZ_TRACE").is_ok_and(|v| v == "1");
    println!(
        "fuzz[{}]: base seed {base_seed:#x} (OPENFRAME_FUZZ_SEED), parser seed {seed:#x}, \
         {iters} iterations (OPENFRAME_FUZZ_ITERS), {} corpus files, per-input bound {:?}",
        spec.name,
        spec.corpus.len(),
        spec.per_input
    );
    let started = Instant::now();
    let mut rng = Rng::new(seed);
    let mut hist: BTreeMap<Outcome, u64> = BTreeMap::new();
    let run_one = |input: &[u8]| -> Result<(Outcome, Duration), String> {
        let t = Instant::now();
        match catch_unwind(AssertUnwindSafe(|| target(input))) {
            Ok(o) => Ok((o, t.elapsed())),
            Err(p) => Err(panic_text(&p)),
        }
    };
    let total = spec.corpus.len() as u64 + iters;
    for i in 0..total {
        let input = if (i as usize) < spec.corpus.len() {
            spec.corpus[i as usize].clone()
        } else {
            let base = &spec.corpus[rng.below(spec.corpus.len())];
            mutate(&mut rng, base, &spec.corpus, spec.dict, spec.max_len)
        };
        if trace {
            save_reproducer(&format!("{}-last.{}", spec.name, spec.ext), &input);
        }
        // A hang never returns, so the watchdog saves the input and aborts with its location.
        let dog = Watchdog::arm(
            &format!(
                "fuzz[{}] iteration {i} (OPENFRAME_FUZZ_SEED={base_seed:#x} OPENFRAME_FUZZ_ITERS={})",
                spec.name,
                i + 1
            ),
            spec.per_input * 4 + Duration::from_secs(10),
            Some((
                format!("{}-{seed:x}-{i}.hang.{}", spec.name, spec.ext),
                input.clone(),
            )),
        );
        let outcome = run_one(&input);
        drop(dog);
        let failure = match outcome {
            Ok((o, took)) if took <= spec.per_input => {
                *hist.entry(o).or_default() += 1;
                None
            }
            Ok((_, took)) => Some(format!("took {took:?} (bound {:?})", spec.per_input)),
            Err(msg) => Some(format!("panicked: {msg}")),
        };
        if let Some(why) = failure {
            let raw = save_reproducer(&format!("{}-{seed:x}-{i}.{}", spec.name, spec.ext), &input);
            let min = minimize(&input, |cand| match run_one(cand) {
                Ok((_, took)) => took > spec.per_input,
                Err(_) => true,
            });
            let min_path = save_reproducer(
                &format!("{}-{seed:x}-{i}.min.{}", spec.name, spec.ext),
                &min,
            );
            panic!(
                "fuzz[{}]: DEFECT at iteration {i} (base seed {base_seed:#x}): {why}\n  \
                 input ({} bytes): {}\n  minimized ({} bytes): {}\n  \
                 reproduce: OPENFRAME_FUZZ_SEED={base_seed:#x} OPENFRAME_FUZZ_ITERS={}",
                spec.name,
                input.len(),
                raw.display(),
                min.len(),
                min_path.display(),
                i + 1
            );
        }
    }
    println!(
        "fuzz[{}]: {total} inputs in {:?}, no panic, every input within {:?}; outcomes {hist:?}",
        spec.name,
        started.elapsed(),
        spec.per_input
    );
    hist
}

// ------------------------------------------------------------------ raw ZIP writer

/// A ZIP entry for the raw writer. The writer never validates names, so it can build
/// zip-slip, absolute, device and colliding names that `zip::ZipWriter` users would avoid.
#[derive(Clone)]
pub struct RawEntry {
    pub name: Vec<u8>,
    /// Bytes stored in the archive (already compressed when `method` is 8).
    pub data: Vec<u8>,
    /// 0 = stored, 8 = deflate.
    pub method: u16,
    /// CRC-32 of the *uncompressed* data.
    pub crc: u32,
    /// Uncompressed size written to the headers (may lie).
    pub declared_size: u32,
    /// Unix mode (e.g. `0o120777` for a symlink); sets "version made by" to Unix.
    pub unix_mode: Option<u32>,
}

impl RawEntry {
    pub fn stored(name: &str, data: &[u8]) -> RawEntry {
        RawEntry::stored_bytes(name.as_bytes(), data)
    }
    pub fn stored_bytes(name: &[u8], data: &[u8]) -> RawEntry {
        RawEntry {
            name: name.to_vec(),
            data: data.to_vec(),
            method: 0,
            crc: crc32(data),
            declared_size: data.len() as u32,
            unix_mode: None,
        }
    }
    /// `compressed` is raw deflate of `plain_len` bytes whose CRC-32 is `crc`.
    pub fn deflated(name: &str, compressed: Vec<u8>, crc: u32, plain_len: u32) -> RawEntry {
        RawEntry {
            name: name.as_bytes().to_vec(),
            data: compressed,
            method: 8,
            crc,
            declared_size: plain_len,
            unix_mode: None,
        }
    }
    pub fn symlink(name: &str, target: &str) -> RawEntry {
        RawEntry {
            unix_mode: Some(0o120777),
            ..RawEntry::stored(name, target.as_bytes())
        }
    }
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Deterministic ZIP (no timestamps). `declared_entries` overrides the end-of-central-directory
/// entry count (to build archives that lie about their directory).
pub fn build_zip(entries: &[RawEntry], declared_entries: Option<u16>) -> Vec<u8> {
    let mut out: Vec<u8> = vec![];
    let mut central: Vec<u8> = vec![];
    for e in entries {
        let offset = out.len() as u32;
        let csize = e.data.len() as u32;
        let flags: u16 = if std::str::from_utf8(&e.name).is_ok_and(|s| !s.is_ascii()) {
            0x0800 // names are UTF-8
        } else {
            0
        };
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend(20u16.to_le_bytes());
        out.extend(flags.to_le_bytes());
        out.extend(e.method.to_le_bytes());
        out.extend([0u8; 4]); // time, date
        out.extend(e.crc.to_le_bytes());
        out.extend(csize.to_le_bytes());
        out.extend(e.declared_size.to_le_bytes());
        out.extend((e.name.len() as u16).to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(&e.name);
        out.extend(&e.data);

        let made_by: u16 = if e.unix_mode.is_some() {
            (3 << 8) | 20
        } else {
            20
        };
        let external: u32 = e.unix_mode.map(|m| m << 16).unwrap_or(0);
        central.extend(0x0201_4b50u32.to_le_bytes());
        central.extend(made_by.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        central.extend(flags.to_le_bytes());
        central.extend(e.method.to_le_bytes());
        central.extend([0u8; 4]);
        central.extend(e.crc.to_le_bytes());
        central.extend(csize.to_le_bytes());
        central.extend(e.declared_size.to_le_bytes());
        central.extend((e.name.len() as u16).to_le_bytes());
        central.extend([0u8; 8]); // extra, comment, disk, internal attrs
        central.extend(external.to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(&e.name);
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    let count = declared_entries.unwrap_or(entries.len() as u16);
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(cd_size.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out
}

/// Read the entries of a (well-formed, stored or deflated) ZIP back as raw entries, so
/// structure-aware fuzzing can mutate names and data and rebuild the archive. Returns
/// `None` for anything the simple local-header walk does not understand.
pub fn parse_raw_zip(bytes: &[u8]) -> Option<Vec<RawEntry>> {
    let u16_at = |at: usize| -> Option<u16> {
        Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
    };
    let u32_at = |at: usize| -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
    };
    let mut out = Vec::new();
    let mut at = 0usize;
    while u32_at(at)? == 0x0403_4b50 {
        let method = u16_at(at + 8)?;
        let crc = u32_at(at + 14)?;
        let csize = u32_at(at + 18)? as usize;
        let usize_ = u32_at(at + 22)?;
        let nlen = u16_at(at + 26)? as usize;
        let xlen = u16_at(at + 28)? as usize;
        let name = bytes.get(at + 30..at + 30 + nlen)?.to_vec();
        let start = at + 30 + nlen + xlen;
        let data = bytes.get(start..start + csize)?.to_vec();
        out.push(RawEntry {
            name,
            data,
            method,
            crc,
            declared_size: usize_,
            unix_mode: None,
        });
        at = start + csize;
    }
    Some(out)
}
