//! A map that forgets its least recently used entries past a count or a size.

use std::collections::{BTreeMap, HashMap};

pub(crate) struct Lru<V> {
    map: HashMap<String, (V, usize, u64)>,
    order: BTreeMap<u64, String>,
    tick: u64,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}

impl<V: Clone> Lru<V> {
    pub(crate) fn new(max_entries: usize, max_bytes: usize) -> Lru<V> {
        Lru {
            map: HashMap::new(),
            order: BTreeMap::new(),
            tick: 0,
            bytes: 0,
            max_entries,
            max_bytes,
        }
    }

    pub(crate) fn get(&mut self, key: &str) -> Option<V> {
        let tick = self.next();
        let e = self.map.get_mut(key)?;
        self.order.remove(&e.2);
        e.2 = tick;
        self.order.insert(tick, key.to_string());
        Some(e.0.clone())
    }

    pub(crate) fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub(crate) fn set(&mut self, key: &str, value: V, bytes: usize) {
        self.delete(key);
        if bytes > self.max_bytes {
            return;
        }
        let tick = self.next();
        self.map.insert(key.to_string(), (value, bytes, tick));
        self.order.insert(tick, key.to_string());
        self.bytes += bytes;
        while self.map.len() > self.max_entries || self.bytes > self.max_bytes {
            let Some((_, k)) = self.order.pop_first() else {
                break;
            };
            if let Some((_, b, _)) = self.map.remove(&k) {
                self.bytes -= b;
            }
        }
    }

    pub(crate) fn delete(&mut self, key: &str) {
        if let Some((_, b, t)) = self.map.remove(key) {
            self.order.remove(&t);
            self.bytes -= b;
        }
    }

    fn next(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forgets_the_least_recently_used() {
        let mut l = Lru::new(2, 100);
        l.set("a", 1, 10);
        l.set("b", 2, 10);
        assert_eq!(l.get("a"), Some(1));
        l.set("c", 3, 10);
        assert!(!l.has("b"));
        assert!(l.has("a") && l.has("c"));
        l.set("d", 4, 95);
        assert_eq!(
            (l.has("a"), l.has("c"), l.get("d")),
            (false, false, Some(4))
        );
        l.set("e", 5, 101);
        assert!(!l.has("e"));
    }
}
