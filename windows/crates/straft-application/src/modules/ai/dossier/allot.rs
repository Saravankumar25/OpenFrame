//! Splitting the dossier's token budget across the picked leaves (spec 3.4, round R3).
//!
//! The dossier cap is shared by the picked leaves by weight. Each leaf has a minimum share, so one
//! large leaf cannot crowd out the rest, and a leaf that needs less than its share passes the spare
//! on to the leaves that need more. A fact that carries a value the answer must state is never cut
//! (that rule lives in `trim`; a share only says how much of the rest may stay).

/// What one picked leaf asks for.
#[derive(Debug, Clone, Copy)]
pub struct Want {
    /// The leaf's weight in the registry (1 to 5).
    pub weight: u32,
    /// The tokens of its facts after the gate and the guards.
    pub demand: usize,
}

/// The least a leaf may keep when it has that much: a quarter to half of an even split, never less
/// than this many tokens.
pub const MIN_SHARE_FLOOR: usize = 120;

/// The tokens each leaf may keep, in the order of `wants`. The sum never passes `total`, a leaf
/// never gets more than it asked for, and every leaf that asks for its minimum share gets it.
pub fn shares(total: usize, wants: &[Want]) -> Vec<usize> {
    let n = wants.len();
    if n == 0 {
        return Vec::new();
    }
    let mut min_share = MIN_SHARE_FLOOR.max(total / (2 * n));
    if min_share * n > total {
        min_share = total / n;
    }
    let mut got: Vec<usize> = wants.iter().map(|w| w.demand.min(min_share)).collect();
    let mut pool = total.saturating_sub(got.iter().sum());
    // Spare tokens go to the leaves that still want more, by weight; a few rounds settle it.
    for _ in 0..=n {
        let unmet: Vec<usize> = (0..n).filter(|i| got[*i] < wants[*i].demand).collect();
        let weights: u32 = unmet.iter().map(|i| wants[*i].weight.max(1)).sum();
        if unmet.is_empty() || pool == 0 {
            break;
        }
        let round = pool;
        for i in unmet {
            let part = (round * wants[i].weight.max(1) as usize).div_ceil(weights as usize);
            let give = part.min(wants[i].demand - got[i]).min(pool);
            got[i] += give;
            pool -= give;
        }
    }
    got
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(weight: u32, demand: usize) -> Want {
        Want { weight, demand }
    }

    #[test]
    fn one_huge_leaf_cannot_starve_the_others() {
        let got = shares(1_500, &[w(5, 40_000), w(2, 600), w(2, 600)]);
        assert!(got.iter().sum::<usize>() <= 1_500, "{got:?}");
        // The small leaves keep at least their minimum share, whatever the big one asks.
        assert!(got[1] >= 250 && got[2] >= 250, "{got:?}");
        // And the big one takes what is left over.
        assert!(got[0] > got[1], "{got:?}");
    }

    #[test]
    fn a_leaf_that_needs_less_passes_its_spare_share_on() {
        let got = shares(1_500, &[w(3, 100), w(3, 5_000)]);
        assert_eq!(got[0], 100);
        assert_eq!(got[1], 1_400, "{got:?}");
    }

    #[test]
    fn nothing_is_given_beyond_what_was_asked() {
        let got = shares(1_500, &[w(3, 200), w(3, 300)]);
        assert_eq!(got, vec![200, 300]);
    }

    #[test]
    fn weight_decides_how_the_rest_is_split() {
        let got = shares(1_500, &[w(4, 9_000), w(1, 9_000)]);
        assert!(got[0] > got[1], "{got:?}");
        assert!(got.iter().sum::<usize>() <= 1_500);
        assert!(
            got[1] >= 375,
            "the lighter leaf keeps its minimum share: {got:?}"
        );
    }

    #[test]
    fn many_leaves_still_fit_the_cap() {
        let wants = vec![w(2, 10_000); 12];
        let got = shares(1_500, &wants);
        assert!(got.iter().sum::<usize>() <= 1_500, "{got:?}");
        assert!(got.iter().all(|g| *g > 0));
    }
}
