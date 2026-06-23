use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub struct QueryCache<T> {
    cache: Mutex<LruCache<String, T>>,
}

impl<T: Clone> QueryCache<T> {
    pub fn new(cap: usize) -> Self {
        QueryCache {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap())),
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    pub fn put(&self, key: &str, value: T) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(key.to_string(), value);
    }

    pub fn invalidate_all(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}
