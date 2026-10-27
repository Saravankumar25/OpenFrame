//! Deterministic embedders for AUTOMATED TESTS ONLY. This crate is a
//! dev-dependency; release builds never contain these types.
//!
//! [`ConceptEmbedder`] hashes stemmed words into a fixed-size vector and adds a
//! shared "concept" feature for words of the same small synonym group
//! ("trust", "betrayal", "suspicion"…). That gives tests a stable, explainable
//! stand-in for meaning-based similarity without downloading a model.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use openframe_search::{Embedder, SearchError, l2_normalize};

const DEFAULT_CONCEPTS: &[&[&str]] = &[
    &[
        "trust",
        "betray",
        "betrayal",
        "suspicion",
        "suspect",
        "suspects",
        "suspicious",
        "doubt",
        "doubts",
        "lie",
        "lies",
        "lying",
        "deceive",
        "distrust",
    ],
    &[
        "guilt", "guilty", "remorse", "shame", "regret", "blame", "atone",
    ],
    &[
        "hospital", "clinic", "doctor", "nurse", "ward", "surgery", "icu",
    ],
    &[
        "confront",
        "confrontation",
        "argue",
        "argument",
        "fight",
        "quarrel",
        "shout",
        "shouts",
        "hostile",
        "anger",
        "furious",
    ],
    &[
        "train",
        "railway",
        "station",
        "platform",
        "locomotive",
        "tracks",
    ],
    &[
        "isolation",
        "alone",
        "lonely",
        "solitude",
        "isolated",
        "abandoned",
    ],
    &["car", "vehicle", "mustang", "jeep", "truck"],
    &["night", "dark", "midnight", "moonlight"],
];

const STOP: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "at", "for", "with", "is", "are", "was",
    "were", "be", "does", "do", "did", "where", "what", "which", "who", "how", "his", "her",
    "their", "its", "it", "this", "that",
];

fn stem(w: &str) -> String {
    for suf in ["ing", "ed", "es", "s"] {
        if w.len() > suf.len() + 3 && w.ends_with(suf) {
            return w[..w.len() - suf.len()].to_string();
        }
    }
    w.to_string()
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub struct ConceptEmbedder {
    dim: usize,
    id: String,
    concepts: Vec<Vec<String>>,
    calls: AtomicUsize,
    texts: AtomicUsize,
    delay: Mutex<Option<Duration>>,
    failing: AtomicBool,
    stalled: (Mutex<bool>, Condvar),
}

impl Default for ConceptEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConceptEmbedder {
    pub fn new() -> Self {
        Self::with_dim(384)
    }

    pub fn with_dim(dim: usize) -> Self {
        Self {
            dim,
            id: format!("test-concept-embedder@1#dim{dim}"),
            concepts: DEFAULT_CONCEPTS
                .iter()
                .map(|g| g.iter().map(|s| s.to_string()).collect())
                .collect(),
            calls: AtomicUsize::new(0),
            texts: AtomicUsize::new(0),
            delay: Mutex::new(None),
            failing: AtomicBool::new(false),
            stalled: (Mutex::new(false), Condvar::new()),
        }
    }

    /// A different model identity (simulates an embedding-model upgrade).
    pub fn with_model_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    /// Sleep this long per call (simulates a slow CPU embedding runtime).
    pub fn set_delay(&self, d: Option<Duration>) {
        *self.delay.lock().unwrap() = d;
    }

    /// Make every call fail (simulates a crashed embedding runtime).
    pub fn set_failing(&self, f: bool) {
        self.failing.store(f, Ordering::SeqCst);
    }

    /// While stalled, every `embed` call blocks until the stall is lifted (simulates an
    /// embedding runtime that stopped answering). `calls()` counts a call as soon as it starts.
    pub fn set_stalled(&self, on: bool) {
        let (lock, cond) = &self.stalled;
        *lock.lock().unwrap() = on;
        cond.notify_all();
    }

    /// Number of `embed` calls so far.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// Number of texts embedded so far.
    pub fn texts(&self) -> usize {
        self.texts.load(Ordering::SeqCst)
    }

    pub fn vector(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0f32; self.dim];
        let lower = text.to_lowercase();
        for raw in lower.split(|c: char| !c.is_alphanumeric()) {
            if raw.len() < 2 || STOP.contains(&raw) {
                continue;
            }
            let s = stem(raw);
            v[(fnv(&format!("w:{s}")) % self.dim as u64) as usize] += 1.0;
            for (gi, g) in self.concepts.iter().enumerate() {
                if g.iter().any(|x| x == raw || stem(x) == s) {
                    v[(fnv(&format!("concept:{gi}")) % self.dim as u64) as usize] += 3.0;
                }
            }
        }
        if v.iter().all(|x| *x == 0.0) {
            v[0] = 1e-3;
        }
        l2_normalize(&mut v);
        v
    }
}

impl Embedder for ConceptEmbedder {
    fn model_id(&self) -> String {
        self.id.clone()
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, SearchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        {
            let (lock, cond) = &self.stalled;
            let mut stalled = lock.lock().unwrap();
            while *stalled {
                stalled = cond.wait(stalled).unwrap();
            }
        }
        if let Some(d) = *self.delay.lock().unwrap() {
            std::thread::sleep(d);
        }
        if self.failing.load(Ordering::SeqCst) {
            return Err(SearchError::Embedding("test embedder is failing".into()));
        }
        self.texts.fetch_add(inputs.len(), Ordering::SeqCst);
        Ok(inputs.iter().map(|t| self.vector(t)).collect())
    }
}

/// Everyday synonyms: words of one group mean about the same to a search model.
const BAND_CONCEPTS: &[&[&str]] = &[
    &[
        "speak",
        "talk",
        "say",
        "dialogue",
        "line",
        "voice",
        "talkative",
        "spoken",
        "utter",
    ],
    &["character", "people", "person", "role", "figure", "cast"],
    &["place", "location", "setting", "site", "locations"],
    &[
        "task",
        "todo",
        "job",
        "remain",
        "pending",
        "outstanding",
        "unfinished",
        "open",
    ],
    &["draft", "version", "revision"],
    &[
        "theme",
        "motif",
        "message",
        "meaning",
        "symbol",
        "symbolise",
        "underlying",
    ],
    &[
        "idea",
        "suggest",
        "brainstorm",
        "pitch",
        "twist",
        "alternative",
        "option",
    ],
    &[
        "story",
        "plot",
        "storyline",
        "film",
        "movie",
        "screenplay",
        "script",
    ],
    &["crew", "team", "staff"],
    &[
        "schedule", "calendar", "plan", "shoot", "shooting", "booked",
    ],
    &[
        "feedback", "critique", "critical", "weak", "improve", "works",
    ],
    &[
        "relationship",
        "dynamic",
        "bond",
        "relate",
        "connect",
        "tension",
    ],
    &[
        "describe",
        "portrait",
        "sketch",
        "characterise",
        "personality",
    ],
    &[
        "summary",
        "summarise",
        "recap",
        "overview",
        "outline",
        "gist",
        "whole",
        "entire",
    ],
    &[
        "explain",
        "walk",
        "understand",
        "retell",
        "happen",
        "happening",
        "unfold",
    ],
    &["many", "count", "total", "number", "tally"],
];

const BAND_STOP: &[&str] = &[
    "the", "a", "an", "of", "to", "in", "is", "are", "do", "does", "it", "this", "that", "me",
    "my", "i", "you", "please", "and", "or", "on", "at", "be", "all", "have", "has", "what", "who",
    "which", "how", "give", "tell", "show",
];

/// A stand-in for an E5 search model, for tests of the intent step. Words, stems, shared meanings
/// and letter triples go into a hashed vector, and the result is put into the band E5 models use:
/// every pair of texts starts at cosine 0.70 and the overlap adds up to 0.30. It checks gates and
/// plumbing against that scale. It does not measure how good the examples are; the bench with the
/// real model does.
pub struct BandEmbedder {
    id: String,
    texts: AtomicUsize,
    delay: Mutex<Option<Duration>>,
    failing: AtomicBool,
}

const BAND_DIM: usize = 512;

impl BandEmbedder {
    pub fn new(id: &str) -> Self {
        Self {
            id: format!("test-band-embedder-{id}"),
            texts: AtomicUsize::new(0),
            delay: Mutex::new(None),
            failing: AtomicBool::new(false),
        }
    }

    pub fn set_failing(&self, on: bool) {
        self.failing.store(on, Ordering::SeqCst);
    }

    pub fn set_delay(&self, d: Option<Duration>) {
        *self.delay.lock().unwrap() = d;
    }

    /// Number of texts embedded so far.
    pub fn texts(&self) -> usize {
        self.texts.load(Ordering::SeqCst)
    }

    pub fn vector(text: &str) -> Vec<f32> {
        let mut v = vec![0f32; BAND_DIM + 1];
        for raw in text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty() && !BAND_STOP.contains(w))
        {
            let w = raw
                .strip_suffix("ing")
                .or_else(|| raw.strip_suffix('s'))
                .filter(|s| s.len() > 2)
                .unwrap_or(raw);
            let slot = |s: &str| (fnv(s) % BAND_DIM as u64) as usize;
            v[slot(w)] += 1.0;
            for (i, g) in BAND_CONCEPTS.iter().enumerate() {
                if g.iter().any(|x| *x == raw || *x == w) {
                    v[slot(&format!("concept:{i}"))] += 1.5;
                }
            }
            let padded: Vec<char> = format!("^{w}$").chars().collect();
            for t in padded.windows(3) {
                v[slot(&t.iter().collect::<String>())] += 0.2;
            }
        }
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        for x in &mut v[..BAND_DIM] {
            *x = *x / n * 0.3f32.sqrt();
        }
        v[BAND_DIM] = 0.7f32.sqrt();
        v
    }
}

impl Embedder for BandEmbedder {
    fn model_id(&self) -> String {
        self.id.clone()
    }
    fn dim(&self) -> usize {
        BAND_DIM + 1
    }
    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, SearchError> {
        if let Some(d) = *self.delay.lock().unwrap() {
            std::thread::sleep(d);
        }
        if self.failing.load(Ordering::SeqCst) {
            return Err(SearchError::Embedding("test embedder is failing".into()));
        }
        self.texts.fetch_add(inputs.len(), Ordering::SeqCst);
        Ok(inputs.iter().map(|t| Self::vector(t)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cos(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn concept_words_are_close_and_deterministic() {
        let e = ConceptEmbedder::new();
        let q = e.vector("losing trust");
        let related = e.vector("She suspects he lies to her.");
        let unrelated = e.vector("The kettle boils in the kitchen.");
        assert!(cos(&q, &related) > cos(&q, &unrelated) + 0.2);
        assert_eq!(e.vector("x y z"), e.vector("x y z"));
    }
}
