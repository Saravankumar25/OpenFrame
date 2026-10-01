//! Deterministic embedders for AUTOMATED TESTS ONLY. This crate is a
//! dev-dependency; release builds never contain these types.
//!
//! [`ConceptEmbedder`] hashes stemmed words into a fixed-size vector and adds a
//! shared "concept" feature for words of the same small synonym group
//! ("trust", "betrayal", "suspicion"…). That gives tests a stable, explainable
//! stand-in for meaning-based similarity without downloading a model.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
