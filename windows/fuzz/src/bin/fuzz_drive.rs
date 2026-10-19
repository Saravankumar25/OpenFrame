//! Seeded mutation driver for the fuzz targets (Windows phase W3), for toolchains where libFuzzer
//! does not link (the Windows development machine). It has no coverage feedback: it mutates the
//! corpus seeds at random (bit flips, inserted, deleted and repeated bytes, splices between seeds,
//! truncation) for a fixed time per target, and fails on any panic or any input slower than the
//! hang limit. Failing inputs are saved under fuzz/artifacts/<target>/ so they replay exactly. A
//! watchdog thread saves the running input and ends the process when one input never returns.
//!
//!   cargo run --release --manifest-path fuzz/Cargo.toml --bin fuzz_drive -- <seconds> [target]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use openframe_fuzz::{TARGETS, seeds};

const HANG: Duration = Duration::from_secs(5);
const MAX_LEN: usize = 1 << 20;

static LAST_PANIC: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// The input being run and when it started (milliseconds since the run began; 0 when idle).
static CURRENT: std::sync::Mutex<(Vec<u8>, &str)> = std::sync::Mutex::new((Vec::new(), ""));
static STARTED_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

fn mutate(rng: &mut Rng, corpus: &[Vec<u8>]) -> Vec<u8> {
    let mut d = corpus[rng.below(corpus.len())].clone();
    for _ in 0..=rng.below(8) {
        match rng.below(7) {
            0 if !d.is_empty() => {
                let i = rng.below(d.len());
                d[i] ^= 1 << rng.below(8);
            }
            1 => {
                let i = rng.below(d.len() + 1);
                d.insert(i, rng.next() as u8);
            }
            2 if !d.is_empty() => {
                let i = rng.below(d.len());
                let n = 1 + rng.below((d.len() - i).min(64));
                d.drain(i..i + n);
            }
            3 if !d.is_empty() => {
                let i = rng.below(d.len());
                let n = 1 + rng.below((d.len() - i).min(256));
                let chunk = d[i..i + n].to_vec();
                let at = rng.below(d.len() + 1);
                for _ in 0..=rng.below(16) {
                    d.splice(at..at, chunk.iter().copied());
                }
            }
            4 => {
                let other = &corpus[rng.below(corpus.len())];
                if !other.is_empty() {
                    let i = rng.below(other.len());
                    let n = 1 + rng.below((other.len() - i).min(512));
                    let at = rng.below(d.len() + 1);
                    d.splice(at..at, other[i..i + n].iter().copied());
                }
            }
            5 if !d.is_empty() => d.truncate(rng.below(d.len())),
            _ => {
                let i = rng.below(d.len() + 1);
                let interesting: [&[u8]; 8] = [
                    b"<",
                    b">",
                    b"&",
                    b"\"",
                    b"\r\n",
                    b"PK\x03\x04",
                    &[0xff, 0xfe],
                    &[0],
                ];
                d.splice(i..i, interesting[rng.below(interesting.len())].iter().copied());
            }
        }
    }
    d.truncate(MAX_LEN);
    d
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seconds: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(60);
    let only = args.get(1).cloned();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    // Panics are expected findings here: the hook keeps the last message for the report.
    std::panic::set_hook(Box::new(|info| {
        *LAST_PANIC.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = info.to_string();
    }));
    let epoch = Instant::now();
    {
        let root = root.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let started = STARTED_MS.load(std::sync::atomic::Ordering::SeqCst);
                let now = epoch.elapsed().as_millis() as u64;
                if started != 0 && now.saturating_sub(started) > 6 * HANG.as_millis() as u64 {
                    let (input, name) = CURRENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
                    let dir = root.join("fuzz").join("artifacts").join(name);
                    let _ = std::fs::create_dir_all(&dir);
                    let file = dir.join("hang.bin");
                    let _ = std::fs::write(&file, &input);
                    eprintln!("{name}: an input never returned; saved as {}", file.display());
                    std::process::exit(2);
                }
            }
        });
    }
    let mut failed = false;
    for target in TARGETS.iter().filter(|t| only.as_deref().is_none_or(|o| o == t.name)) {
        let corpus = seeds(&root, target);
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ target.name.len() as u64);
        let (mut runs, mut panics, mut slow, mut slowest) = (0u64, 0u64, 0u64, Duration::ZERO);
        let deadline = Instant::now() + Duration::from_secs(seconds);
        // Every seed once as it is, then mutations until the time is up.
        let mut queue: Vec<Vec<u8>> = corpus.clone();
        while Instant::now() < deadline {
            let input = queue.pop().unwrap_or_else(|| mutate(&mut rng, &corpus));
            {
                let mut cur = CURRENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                cur.0.clear();
                cur.0.extend_from_slice(&input);
                cur.1 = target.name;
            }
            STARTED_MS.store((epoch.elapsed().as_millis() as u64).max(1), std::sync::atomic::Ordering::SeqCst);
            let started = Instant::now();
            let ok = catch_unwind(AssertUnwindSafe(|| (target.run)(&input))).is_ok();
            let took = started.elapsed();
            STARTED_MS.store(0, std::sync::atomic::Ordering::SeqCst);
            runs += 1;
            slowest = slowest.max(took);
            let problem = if !ok {
                panics += 1;
                Some("panic")
            } else if took > HANG {
                slow += 1;
                Some("slow")
            } else {
                None
            };
            if let Some(kind) = problem {
                let dir = root.join("fuzz").join("artifacts").join(target.name);
                std::fs::create_dir_all(&dir).expect("artifacts folder");
                let file = dir.join(format!("{kind}-{runs}.bin"));
                std::fs::write(&file, &input).expect("save input");
                let why = LAST_PANIC.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
                eprintln!("{}: {kind} on input saved as {} ({why})", target.name, file.display());
            }
        }
        failed |= panics > 0 || slow > 0;
        println!(
            "{{\"target\":\"{}\",\"seconds\":{seconds},\"seeds\":{},\"runs\":{runs},\"panics\":{panics},\"slow\":{slow},\"slowest_ms\":{}}}",
            target.name,
            corpus.len(),
            slowest.as_millis()
        );
    }
    std::process::exit(if failed { 1 } else { 0 });
}
