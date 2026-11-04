//! Typo tolerance for entity resolution (spec `ai-conversational-harness`, sections 4.3 and 5.2).
//!
//! The separate routing copy of the request is retired: a misspelling is handled once, where names
//! are matched against the project index (`index::resolve`), with the two pieces below: an edit
//! distance in which a swap of two neighbouring letters counts as one edit, and the ordinary English
//! words of four to seven letters that sit one letter from a name and must never be matched to it.

/// Ordinary English words of 4 to 7 letters that sit one letter from a keyword. They are never
/// "corrected" ("witch" is not "which", "shoot" is not "shot").
pub(super) const COMMON: &[&str] = &[
    "about", "after", "again", "being", "below", "bring", "built", "called", "chart", "child",
    "clear", "close", "could", "crowd", "dance", "decide", "every", "first", "found", "from",
    "gist", "give", "given", "grant", "great", "group", "happen", "have", "heart", "horse",
    "house", "into", "join", "just", "keep", "kind", "know", "large", "later", "least", "leave",
    "less", "level", "light", "like", "lined", "link", "lost", "made", "make", "mean", "might",
    "mind", "mine", "miss", "more", "most", "move", "much", "must", "name", "near", "need", "next",
    "nice", "none", "noted", "only", "open", "other", "party", "place", "plain", "plant", "point",
    "price", "quiet", "quite", "right", "round", "same", "scale", "scare", "scent", "scone",
    "score", "scope", "scream", "sense", "shall", "shape", "share", "sharp", "shoot", "shop",
    "short", "shout", "show", "shows", "shown", "side", "since", "skill", "small", "some", "soon",
    "sort", "speed", "spoke", "spot", "start", "state", "still", "stone", "stood", "story", "such",
    "sure", "take", "talk", "team", "tell", "than", "that", "their", "them", "then", "there",
    "these", "they", "thing", "think", "this", "those", "three", "through", "tight", "time",
    "told", "took", "tone", "touch", "town", "treat", "tried", "true", "turn", "under", "until",
    "upon", "used", "very", "wait", "walk", "want", "watch", "water", "well", "went", "were",
    "what", "when", "whence", "while", "white", "whom", "whose", "wide", "will", "wish", "with",
    "witch", "woman", "women", "words", "work", "world", "would", "write", "wrong", "years",
    "young", "your",
];

/// Edit distance with a swap of two neighbours counted as one edit; stops past `max`.
pub(super) fn distance(a: &str, b: &str, max: usize) -> Option<usize> {
    let (a, b): (Vec<u8>, Vec<u8>) = (a.bytes().collect(), b.bytes().collect());
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let mut prev2: Vec<usize> = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                cur[j] = cur[j].min(prev2[j - 2] + 1);
            }
        }
        prev2 = prev;
        prev = cur;
    }
    Some(prev[b.len()]).filter(|d| *d <= max)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swap_of_two_neighbours_is_one_edit_and_the_limit_is_kept() {
        assert_eq!(distance("varghese", "vargehse", 2), Some(1));
        assert_eq!(distance("anjali", "anjalli", 1), Some(1));
        assert_eq!(distance("anjali", "mariamma", 2), None);
        assert_eq!(distance("lissy", "lissy", 1), Some(0));
    }

    #[test]
    fn ordinary_words_near_a_name_are_listed() {
        assert!(COMMON.contains(&"house") && COMMON.contains(&"young"));
    }
}
