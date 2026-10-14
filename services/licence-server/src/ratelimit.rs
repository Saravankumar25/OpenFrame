//! A small per-address token bucket. Kept in memory only and never written anywhere. Every
//! call is O(1) amortised, so a flood of new addresses cannot turn the limiter itself into
//! a CPU cost (two-generation table, see `Generations`).
//!
//! IPv6 addresses are grouped by their first 64 bits, because one home or office usually holds a
//! whole /64 and could otherwise rotate addresses to get a fresh bucket each time.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

pub struct RateLimiter {
    gens: Mutex<Generations>,
    capacity: f64,
    refill_per_second: f64,
    /// Each generation holds at most this many addresses, so the table holds at most twice it.
    half: usize,
}

/// Two generations of buckets. A bucket that is touched moves to the current generation; when
/// the current generation is full it becomes the previous one and a fresh one starts, so the
/// previous generation holds only addresses not seen since the swap. Every operation is O(1)
/// amortised: a flood of new addresses costs no scan, and a throttled address that keeps
/// knocking stays current and keeps its empty bucket.
#[derive(Default)]
struct Generations {
    current: HashMap<IpAddr, Bucket>,
    previous: HashMap<IpAddr, Bucket>,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

impl RateLimiter {
    /// `capacity` requests at once, then `refill_per_second` on average. At most `max_entries`
    /// addresses are tracked in two generations; the older generation is dropped whole when the
    /// newer one fills, and no bucket of a recently seen address ever changes because of it.
    pub fn new(capacity: u32, refill_per_second: f64, max_entries: usize) -> Self {
        Self {
            gens: Mutex::new(Generations::default()),
            capacity: f64::from(capacity),
            refill_per_second,
            half: max_entries.div_ceil(2).max(1),
        }
    }

    /// Take one token for `ip`. Returns false when the caller must wait.
    pub fn allow(&self, ip: IpAddr, now: Instant) -> bool {
        let key = group(ip);
        let mut gens = self.gens.lock().unwrap_or_else(|p| p.into_inner());
        let mut bucket = match gens.current.remove(&key) {
            Some(b) => b,
            None => gens.previous.remove(&key).unwrap_or(Bucket {
                tokens: self.capacity,
                last: now,
            }),
        };
        let elapsed = now.saturating_duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_per_second).min(self.capacity);
        bucket.last = now;
        let allowed = if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        };
        if gens.current.len() >= self.half {
            gens.previous = std::mem::take(&mut gens.current);
        }
        gens.current.insert(key, bucket);
        allowed
    }

    #[cfg(test)]
    fn tracked(&self) -> usize {
        let gens = self.gens.lock().unwrap();
        gens.current.len() + gens.previous.len()
    }
}

fn group(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let s = v6.segments();
                IpAddr::V6(std::net::Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_burst_is_cut_off_and_the_bucket_refills() {
        let limiter = RateLimiter::new(3, 1.0, 100);
        let ip: IpAddr = "203.0.113.7".parse().unwrap();
        let t0 = Instant::now();
        assert!((0..3).all(|_| limiter.allow(ip, t0)));
        assert!(!limiter.allow(ip, t0));
        // Another address has its own bucket.
        assert!(limiter.allow("203.0.113.8".parse().unwrap(), t0));
        assert!(limiter.allow(ip, t0 + Duration::from_millis(1100)));
        assert!(!limiter.allow(ip, t0 + Duration::from_millis(1100)));
    }

    #[test]
    fn ipv6_addresses_in_one_slash_64_share_a_bucket() {
        let limiter = RateLimiter::new(1, 0.0, 100);
        let t0 = Instant::now();
        assert!(limiter.allow("2001:db8:1:2::1".parse().unwrap(), t0));
        assert!(!limiter.allow("2001:db8:1:2::ffff".parse().unwrap(), t0));
        assert!(limiter.allow("2001:db8:1:3::1".parse().unwrap(), t0));
    }

    #[test]
    fn the_table_is_bounded() {
        let limiter = RateLimiter::new(1, 0.0, 2);
        let t0 = Instant::now();
        for i in 0..50u8 {
            limiter.allow(IpAddr::from([198, 51, 100, i]), t0);
        }
        assert!(limiter.tracked() <= 2);
    }

    #[test]
    fn a_flood_from_many_addresses_does_not_reset_a_throttled_address() {
        let limiter = RateLimiter::new(1, 0.0, 4);
        let t0 = Instant::now();
        let attacker: IpAddr = "203.0.113.7".parse().unwrap();
        assert!(limiter.allow(attacker, t0));
        assert!(!limiter.allow(attacker, t0), "throttled");
        // Fill and overflow the table from other addresses while the attacker keeps knocking, as a
        // real attacker would; the attacker's bucket must stay throttled through every swap.
        for i in 0..200u8 {
            let t = t0 + Duration::from_millis(u64::from(i) + 1);
            limiter.allow(IpAddr::from([198, 51, 100, i]), t);
            assert!(!limiter.allow(attacker, t), "still throttled at step {i}");
        }
        assert!(limiter.tracked() <= 4);
    }

    #[test]
    fn eviction_drops_only_the_least_recently_seen_address() {
        let limiter = RateLimiter::new(1, 0.0, 2);
        let t0 = Instant::now();
        let a: IpAddr = "198.51.100.1".parse().unwrap();
        let b: IpAddr = "198.51.100.2".parse().unwrap();
        let c: IpAddr = "198.51.100.3".parse().unwrap();
        assert!(limiter.allow(a, t0));
        assert!(limiter.allow(b, t0 + Duration::from_millis(10)));
        // c is new and the table is full: a (oldest) goes, b keeps its empty bucket.
        assert!(limiter.allow(c, t0 + Duration::from_millis(20)));
        assert!(
            !limiter.allow(b, t0 + Duration::from_millis(30)),
            "b still throttled"
        );
        assert!(
            limiter.allow(a, t0 + Duration::from_millis(40)),
            "a starts fresh after eviction"
        );
    }
}
