use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::metrics;

/// A time-bounded LRU cache for hot DB read queries (#252).
///
/// Each entry carries an expiry timestamp. `get` returns `None` for an entry
/// older than the TTL even if it is still in the LRU, so the caller re-runs
/// the query and `put`s the fresh result. `invalidate_all` is the
/// unconditional wipe used after writes that the cache cannot be sure to
/// have invalidated (e.g. external SQL updates).
///
/// Hit / miss counters are forwarded to `crate::metrics` so the
/// `MetricsSnapshot` reports cache effectiveness.
///
/// # Panic safety
///
/// All `Mutex` lock acquisitions recover from a poisoned mutex via
/// `unwrap_or_else(|e| e.into_inner())` instead of panicking. If the
/// lock-holder panicked, the cache data may be stale, but the app continues
/// to function — stale entries are detected on the next `get` via the TTL.
pub struct QueryCache<T> {
    cache: Mutex<LruCache<String, (T, Instant)>>,
    ttl: Duration,
}

impl<T: Clone> QueryCache<T> {
    pub fn new(cap: usize) -> Self {
        QueryCache::with_ttl(cap, Duration::from_secs(30))
    }

    pub fn with_ttl(cap: usize, ttl: Duration) -> Self {
        // `NonZeroUsize::new` returns `None` when cap is 0, which would be a
        // programming error. Default to capacity 1 instead of panicking.
        let nz = NonZeroUsize::new(cap).unwrap_or(NonZeroUsize::new(1).unwrap());
        QueryCache {
            cache: Mutex::new(LruCache::new(nz)),
            ttl,
        }
    }

    /// Recover a mutex lock guard, handling a potentially poisoned mutex.
    /// A poisoned mutex means the previous lock-holder panicked — the cache
    /// data may be partially inconsistent, but it is still readable.
    /// Recovering rather than panicking keeps the app running; stale entries
    /// are discarded on next `get` via TTL expiry.
    fn lock_cache(&self) -> std::sync::MutexGuard<'_, LruCache<String, (T, Instant)>> {
        self.cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let mut cache = self.lock_cache();
        let entry = cache.get(key).cloned();
        match entry {
            Some((value, ts)) if ts.elapsed() < self.ttl => {
                metrics::inc_cache_hit();
                Some(value)
            }
            Some(_) => {
                cache.pop(key);
                metrics::inc_cache_miss();
                None
            }
            None => {
                metrics::inc_cache_miss();
                None
            }
        }
    }

    pub fn put(&self, key: &str, value: T) {
        let mut cache = self.lock_cache();
        cache.put(key.to_string(), (value, Instant::now()));
    }

    pub fn invalidate_all(&self) {
        let mut cache = self.lock_cache();
        cache.clear();
    }
}
