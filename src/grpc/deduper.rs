use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use solana_sdk::signature::Signature;

const UNINITIALIZED_US: i64 = i64::MIN;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TxKey {
    signature: Signature,
    slot: u64,
}

#[derive(Debug)]
struct Generations {
    current: Arc<DashMap<TxKey, i64>>,
    previous: Arc<DashMap<TxKey, i64>>,
    current_dirty: AtomicBool,
    previous_dirty: bool,
}

impl Generations {
    fn new() -> Self {
        Self {
            current: Arc::new(DashMap::new()),
            previous: Arc::new(DashMap::new()),
            current_dirty: AtomicBool::new(false),
            previous_dirty: false,
        }
    }
}

#[derive(Debug)]
pub struct TxDeduper {
    ttl_us: i64,
    log_label: Option<String>,
    generations: RwLock<Generations>,
    last_rotate_us: AtomicI64,
}

impl TxDeduper {
    pub fn new(ttl: Duration, log_label: Option<String>) -> Self {
        let ttl_us = ttl.as_micros().min(i64::MAX as u128) as i64;
        let ttl_us = ttl_us.max(1);
        Self {
            ttl_us,
            log_label,
            generations: RwLock::new(Generations::new()),
            last_rotate_us: AtomicI64::new(UNINITIALIZED_US),
        }
    }

    #[inline]
    pub fn log_label(&self) -> Option<&str> {
        self.log_label.as_deref()
    }

    #[inline]
    pub fn check(&self, signature: Signature, slot: u64, now_us: i64) -> bool {
        self.maybe_rotate(now_us);
        let key = TxKey { signature, slot };
        let gens = self.generations.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        if gens.previous_dirty {
            if let Some(prev_ts) = gens.previous.get(&key) {
                if now_us.saturating_sub(*prev_ts) < self.ttl_us {
                    return false;
                }
            }
        }
        let accepted = match gens.current.entry(key) {
            Entry::Occupied(mut entry) => {
                if now_us.saturating_sub(*entry.get()) < self.ttl_us {
                    false
                } else {
                    *entry.get_mut() = now_us;
                    gens.current_dirty.store(true, Ordering::Relaxed);
                    true
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(now_us);
                gens.current_dirty.store(true, Ordering::Relaxed);
                true
            }
        };
        accepted
    }

    /// Rotates `current` into `previous` in O(1) once per `ttl_us` and drops
    /// the expired generation off the hot gRPC path when inside a Tokio runtime.
    #[inline]
    fn maybe_rotate(&self, now_us: i64) {
        let last = self.last_rotate_us.load(Ordering::Relaxed);
        if last == UNINITIALIZED_US {
            let _ = self.last_rotate_us.compare_exchange(
                UNINITIALIZED_US,
                now_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            return;
        }
        if now_us.saturating_sub(last) < self.ttl_us {
            return;
        }
        let expired = {
            let mut gens =
                self.generations.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            let current_last = self.last_rotate_us.load(Ordering::Relaxed);
            if now_us.saturating_sub(current_last) < self.ttl_us {
                return;
            }
            self.last_rotate_us.store(now_us, Ordering::Release);
            let current_dirty = gens.current_dirty.swap(false, Ordering::Relaxed);
            let old_prev_dirty = gens.previous_dirty;
            if current_dirty {
                let old_curr = std::mem::replace(&mut gens.current, Arc::new(DashMap::new()));
                let old_prev = std::mem::replace(&mut gens.previous, old_curr);
                gens.previous_dirty = true;
                old_prev_dirty.then_some(old_prev)
            } else if old_prev_dirty {
                let old_prev = std::mem::replace(&mut gens.previous, Arc::new(DashMap::new()));
                gens.previous_dirty = false;
                Some(old_prev)
            } else {
                None
            }
        };
        drop_off_hot_path(expired);
    }

    #[cfg(test)]
    fn total_entries(&self) -> usize {
        let gens = self.generations.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        gens.current.len() + gens.previous.len()
    }
}

fn drop_off_hot_path(map: Option<Arc<DashMap<TxKey, i64>>>) {
    let Some(map) = map else { return };
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn_blocking(move || drop(map));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn rotation_preserves_recent_entries_and_evicts_expired_generation() {
        let deduper = TxDeduper::new(Duration::from_secs(5), None);
        let sig_early = Signature::new_unique();
        let sig_late = Signature::new_unique();
        let sig_trigger = Signature::new_unique();

        assert!(deduper.check(sig_early, 1, 0));
        assert!(deduper.check(sig_late, 1, 4_000_000));
        assert_eq!(deduper.total_entries(), 2);

        // Rotate at t = 5s: early (age 5s) expired, late (age 1s) stays deduplicated via previous.
        assert!(deduper.check(sig_trigger, 1, 5_000_000));
        assert!(!deduper.check(sig_late, 1, 6_000_000));
        assert!(deduper.check(sig_early, 1, 6_000_000));
        assert!(!deduper.check(sig_early, 1, 6_000_001));

        // At t = 9s, sig_late reaches exact TTL (9s - 4s = 5s) and is allowed again.
        assert!(deduper.check(sig_late, 1, 9_000_000));

        // Second rotation drops the original generation so memory stays bounded.
        let sig_after = Signature::new_unique();
        assert!(deduper.check(sig_after, 2, 10_000_000));
        let sig_final = Signature::new_unique();
        assert!(deduper.check(sig_final, 3, 15_000_000));
        assert_eq!(deduper.total_entries(), 2);
    }

    #[test]
    fn concurrent_checks_across_rotations_accept_each_key_once_per_ttl() {
        let deduper = Arc::new(TxDeduper::new(Duration::from_millis(10), None));
        let keys: Vec<(Signature, u64, i64)> =
            (0..200).map(|i| (Signature::new_unique(), i, (i as i64) * 500)).collect();
        let accepted = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(4));

        std::thread::scope(|scope| {
            for _ in 0..4 {
                let deduper = Arc::clone(&deduper);
                let keys = &keys;
                let accepted = Arc::clone(&accepted);
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    for &(sig, slot, ts) in keys {
                        barrier.wait();
                        if deduper.check(sig, slot, ts) {
                            accepted.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        assert_eq!(accepted.load(Ordering::Relaxed), keys.len());
    }
}
