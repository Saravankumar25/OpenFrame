//! Embedding abstraction (contract C1/B side, §12).
//!
//! Production embeddings come from the local embedding model managed by
//! `openframe-ai` (adapter in the application layer). Tests use deterministic
//! embedders from `openframe-test-support`; release code never contains one.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::{SearchError, SearchResult};

/// A local text-embedding model.
pub trait Embedder: Send + Sync {
    /// Model identity: id + version + hash prefix. Stored in the index metadata;
    /// a different value invalidates every stored vector.
    fn model_id(&self) -> String;
    fn dim(&self) -> usize;
    /// Embed a batch. Implementations may batch internally; output order matches input.
    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, SearchError>;
}

pub fn l2_normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Check an embedder's output (count, dimension, finiteness) and normalise it.
pub fn validate(
    mut vectors: Vec<Vec<f32>>,
    count: usize,
    dim: usize,
) -> SearchResult<Vec<Vec<f32>>> {
    if vectors.len() != count {
        return Err(SearchError::Embedding(format!(
            "expected {count} vectors, got {}",
            vectors.len()
        )));
    }
    for v in &mut vectors {
        if v.len() != dim {
            return Err(SearchError::Dimension {
                expected: dim,
                got: v.len(),
            });
        }
        if v.iter().any(|x| !x.is_finite()) {
            return Err(SearchError::Embedding("non-finite vector component".into()));
        }
        l2_normalize(v);
    }
    Ok(vectors)
}

/// (model id, query text)
type CacheKey = (String, String);
type LruState = (VecDeque<CacheKey>, HashMap<CacheKey, Vec<f32>>);

/// Small LRU of query embeddings (§34 "cache query embeddings where useful").
pub struct QueryEmbeddingCache {
    capacity: usize,
    inner: Mutex<LruState>,
}

impl QueryEmbeddingCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Mutex::new((VecDeque::new(), HashMap::new())),
        }
    }

    pub fn get_or_embed(&self, embedder: &dyn Embedder, text: &str) -> SearchResult<Vec<f32>> {
        let key = (embedder.model_id(), text.to_string());
        if let Ok(g) = self.inner.lock()
            && let Some(v) = g.1.get(&key)
        {
            return Ok(v.clone());
        }
        let out = embedder.embed(std::slice::from_ref(&key.1))?;
        let mut v = validate(out, 1, embedder.dim())?;
        let v = v.pop().unwrap_or_default();
        if let Ok(mut g) = self.inner.lock() {
            let (order, map) = &mut *g;
            if map.insert(key.clone(), v.clone()).is_none() {
                order.push_back(key);
            }
            while order.len() > self.capacity {
                if let Some(old) = order.pop_front() {
                    map.remove(&old);
                }
            }
        }
        Ok(v)
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.0.clear();
            g.1.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Counting(AtomicUsize);
    impl Embedder for Counting {
        fn model_id(&self) -> String {
            "counting@1".into()
        }
        fn dim(&self) -> usize {
            2
        }
        fn embed(&self, inputs: &[String]) -> SearchResult<Vec<Vec<f32>>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(inputs.iter().map(|s| vec![s.len() as f32, 1.0]).collect())
        }
    }

    #[test]
    fn cache_embeds_each_query_once_and_evicts() {
        let e = Counting(AtomicUsize::new(0));
        let c = QueryEmbeddingCache::new(2);
        let a = c.get_or_embed(&e, "abc").unwrap();
        assert!((a.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
        c.get_or_embed(&e, "abc").unwrap();
        assert_eq!(e.0.load(Ordering::SeqCst), 1);
        c.get_or_embed(&e, "b").unwrap();
        c.get_or_embed(&e, "c").unwrap();
        c.get_or_embed(&e, "abc").unwrap();
        assert_eq!(e.0.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn validation_rejects_bad_output() {
        assert!(validate(vec![vec![1.0]], 2, 1).is_err());
        assert!(matches!(
            validate(vec![vec![1.0, 2.0]], 1, 3),
            Err(SearchError::Dimension {
                expected: 3,
                got: 2
            })
        ));
        assert!(validate(vec![vec![f32::INFINITY]], 1, 1).is_err());
    }
}
