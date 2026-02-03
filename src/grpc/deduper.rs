use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use solana_sdk::signature::Signature;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TxKey {
    signature: Signature,
    slot: u64,
}

#[derive(Debug)]
pub struct TxDeduper {
    ttl_us: i64,
    log_label: Option<String>,
    entries: DashMap<TxKey, i64>,
    last_cleanup_us: AtomicI64,
}

impl TxDeduper {
    pub fn new(ttl: Duration, log_label: Option<String>) -> Self {
        let ttl_us = ttl.as_micros().min(i64::MAX as u128) as i64;
        let ttl_us = ttl_us.max(1);
        Self {
            ttl_us,
            log_label,
            entries: DashMap::new(),
            last_cleanup_us: AtomicI64::new(0),
        }
    }

    #[inline]
    pub fn log_label(&self) -> Option<&str> {
        self.log_label.as_deref()
    }

    #[inline]
    pub fn check(&self, signature: Signature, slot: u64, now_us: i64) -> bool {
        let key = TxKey { signature, slot };
        let accepted = match self.entries.entry(key) {
            Entry::Occupied(mut entry) => {
                let age_us = now_us.saturating_sub(*entry.get());
                if age_us < self.ttl_us {
                    return false;
                }
                *entry.get_mut() = now_us;
                true
            }
            Entry::Vacant(entry) => {
                entry.insert(now_us);
                true
            }
        };
        if accepted {
            self.maybe_cleanup(now_us);
        }
        accepted
    }

    #[inline]
    fn maybe_cleanup(&self, now_us: i64) {
        let last = self.last_cleanup_us.load(Ordering::Relaxed);
        if now_us.saturating_sub(last) < self.ttl_us {
            return;
        }
        if self
            .last_cleanup_us
            .compare_exchange(last, now_us, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
        let cutoff = now_us.saturating_sub(self.ttl_us);
        self.entries.retain(|_, ts| *ts >= cutoff);
    }
}
