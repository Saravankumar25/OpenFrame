//! The nearest-neighbour read: the labelled example messages of `turn_examples.jsonl`, embedded once
//! with the app's search model, vote on the type of a message. Deterministic, multilingual (the
//! examples are in twelve languages and the search model is multilingual) and cheap: no model writes
//! anything here.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use openframe_search::{Embedder, l2_normalize};
use parking_lot::Mutex;
use serde::Deserialize;

use super::AnswerType;

/// Examples that vote.
const K: usize = 5;
/// The winner needs this many of the [`K`] votes and this many more than the runner-up, otherwise
/// the vote is weak and the model is asked. Tuned with the leave-one-out run on the example file
/// (`ai-eval --turn-only`), never on the gold sets.
const CLEAR_VOTES: usize = 3;
const CLEAR_LEAD: usize = 2;

#[derive(Deserialize)]
struct Raw {
    t: String,
    l: String,
    m: String,
}

pub(super) struct Example {
    pub t: AnswerType,
    pub lang: String,
    pub text: String,
}

static EXAMPLES: LazyLock<Vec<Example>> = LazyLock::new(|| {
    include_str!("../turn_examples.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let r: Raw = serde_json::from_str(l).expect("turn_examples.jsonl is valid");
            Example {
                t: AnswerType::parse(&r.t).expect("turn_examples.jsonl names a known type"),
                lang: r.l,
                text: r.m,
            }
        })
        .collect()
});

pub(super) fn examples() -> &'static [Example] {
    &EXAMPLES
}

type Prototypes = Arc<Vec<(AnswerType, Vec<f32>)>>;

static CACHE: LazyLock<Mutex<HashMap<String, Prototypes>>> = LazyLock::new(Default::default);
static BUILDING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

fn unit(mut v: Vec<f32>) -> Vec<f32> {
    l2_normalize(&mut v);
    v
}

/// The examples, embedded once per search model. None while another thread is still embedding them
/// or when the model fails: the caller skips this step.
pub(super) fn prototypes(e: &dyn Embedder) -> Option<Prototypes> {
    let key = e.model_id();
    if let Some(p) = CACHE.lock().get(&key) {
        return Some(p.clone());
    }
    if !BUILDING.lock().insert(key.clone()) {
        return None;
    }
    let made = (|| {
        let texts: Vec<String> = examples().iter().map(|x| x.text.clone()).collect();
        let vectors = e.embed_queries(&texts).ok()?;
        if vectors.len() != texts.len() {
            return None;
        }
        Some(Arc::new(
            examples()
                .iter()
                .map(|x| x.t)
                .zip(vectors.into_iter().map(unit))
                .collect::<Vec<_>>(),
        ))
    })();
    BUILDING.lock().remove(&key);
    let p = made?;
    CACHE.lock().insert(key, p.clone());
    Some(p)
}

/// The result of one vote.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Vote {
    pub winner: AnswerType,
    /// Votes for the winner and for the runner-up, of [`K`].
    pub votes: usize,
    pub runner_up: usize,
    /// Share of the votes the winner got, 0 to 1.
    pub share: f32,
}

impl Vote {
    /// The winner has enough votes and enough of a lead to be trusted without the model.
    pub fn clear(&self) -> bool {
        self.votes >= CLEAR_VOTES && self.votes - self.runner_up >= CLEAR_LEAD
    }
}

/// The nearest `K` examples vote; `skip` leaves one example out (leave-one-out).
fn vote(query: &[f32], protos: &[(AnswerType, Vec<f32>)], skip: Option<usize>) -> Option<Vote> {
    let mut sims: Vec<(f32, AnswerType)> = protos
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != skip)
        .map(|(_, (t, v))| (v.iter().zip(query).map(|(a, b)| a * b).sum::<f32>(), *t))
        .collect();
    sims.sort_by(|a, b| b.0.total_cmp(&a.0));
    sims.truncate(K);
    let mut tally: BTreeMap<AnswerType, (usize, f32)> = BTreeMap::new();
    for (s, t) in &sims {
        let e = tally.entry(*t).or_default();
        e.0 += 1;
        e.1 += s;
    }
    let mut ranked: Vec<_> = tally.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.total_cmp(&a.1.1)));
    let (winner, (votes, _)) = *ranked.first()?;
    let runner_up = ranked.get(1).map_or(0, |r| r.1.0);
    Some(Vote {
        winner,
        votes,
        runner_up,
        share: votes as f32 / sims.len() as f32,
    })
}

fn vote_now(e: &dyn Embedder, text: &str) -> Option<Vote> {
    let protos = prototypes(e)?;
    let q = e.embed_queries(&[text.to_string()]).ok()?.pop()?;
    vote(&unit(q), &protos, None)
}

/// Embed the message and vote, on a worker thread, waiting at most `wait`. A late result still fills
/// the example cache, so the next message is fast.
pub(super) fn vote_within(e: Arc<dyn Embedder>, text: &str, wait: Duration) -> Option<Vote> {
    let text = text.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(vote_now(&*e, &text));
    });
    rx.recv_timeout(wait).ok().flatten()
}

/// One example read by the others (leave-one-out): how well the examples predict themselves. It
/// tunes [`CLEAR_VOTES`] without touching any gold set.
#[derive(Debug, Clone)]
pub struct LooRead {
    pub expected: AnswerType,
    pub got: AnswerType,
    pub lang: String,
    pub text: String,
    pub clear: bool,
}

pub fn leave_one_out(e: &dyn Embedder) -> Option<Vec<LooRead>> {
    let protos = prototypes(e)?;
    Some(
        examples()
            .iter()
            .zip(protos.iter())
            .enumerate()
            .filter_map(|(i, (x, (_, v)))| {
                let v = vote(v, &protos, Some(i))?;
                Some(LooRead {
                    expected: x.t,
                    got: v.winner,
                    lang: x.lang.clone(),
                    text: x.text.clone(),
                    clear: v.clear(),
                })
            })
            .collect(),
    )
}
