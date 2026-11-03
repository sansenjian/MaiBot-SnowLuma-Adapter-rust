//! 消息去重，逐语义对齐 platform_io/dedupe.py 的 MessageDeduplicator。
//!
//! Python 版用 dict + heapq 懒清理；这里用 HashMap + BinaryHeap(Reverse) 小顶堆，
//! 语义一一对应：先懒清理过期键，再判重，容量满时淘汰最早过期的有效键。
//!
//! mark_seen 返回 true = 该键在 TTL 窗口内首次出现（放行）；false = 重复（拦截）。

use std::collections::{BinaryHeap, HashMap};
use std::time::Duration;
use tokio::time::{Instant, Reverse};

/// 与 Python 版默认参数一致：TTL 300s，最大 10000 条。
pub const DEFAULT_TTL: Duration = Duration::from_secs(300);
pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

pub struct Deduper {
    ttl: Duration,
    max_entries: usize,
    seen: HashMap<String, Instant>,
    expire_heap: BinaryHeap<Reverse<(Instant, String)>>,
}

impl Deduper {
    pub fn new(ttl: Duration, max_entries: usize) -> anyhow::Result<Self> {
        if ttl.is_zero() {
            anyhow::bail!("ttl 必须大于 0");
        }
        if max_entries == 0 {
            anyhow::bail!("max_entries 必须大于 0");
        }
        Ok(Self {
            ttl,
            max_entries,
            seen: HashMap::new(),
            expire_heap: BinaryHeap::new(),
        })
    }

    /// 标记一条去重键已经出现过。
    /// 返回 true 表示当前 TTL 窗口内首次出现，false 表示重复。
    pub fn mark_seen(&mut self, dedupe_key: &str) -> bool {
        let now = Instant::now();
        self.purge_expired(now);

        if let Some(&expires_at) = self.seen.get(dedupe_key) {
            if expires_at > now {
                return false;
            }
        }

        if self.seen.len() >= self.max_entries {
            self.evict_earliest_live();
        }

        let expires_at = now + self.ttl;
        self.seen.insert(dedupe_key.to_owned(), expires_at);
        self.expire_heap
            .push(Reverse((expires_at, dedupe_key.to_owned())));
        true
    }

    pub fn clear(&mut self) {
        self.expire_heap.clear();
        self.seen.clear();
    }

    /// 懒清理：弹出所有已过期堆节点；跳过已被重新写入的旧版本节点。
    fn purge_expired(&mut self, now: Instant) {
        while let Some(&Reverse((expires_at, _))) = self.expire_heap.peek() {
            if expires_at > now {
                break;
            }
            let Reverse((expires_at, key)) = self.expire_heap.pop().unwrap();
            if self.seen.get(&key) == Some(&expires_at) {
                self.seen.remove(&key);
            }
        }
    }

    /// 容量满时淘汰一条最早过期的有效键（同样跳过失效节点）。
    fn evict_earliest_live(&mut self) {
        while let Some(Reverse((expires_at, key))) = self.expire_heap.pop() {
            if self.seen.get(&key) == Some(&expires_at) {
                self.seen.remove(&key);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_within_ttl_is_rejected() {
        let mut d = Deduper::new(DEFAULT_TTL, DEFAULT_MAX_ENTRIES).unwrap();
        assert!(d.mark_seen("k1"));
        assert!(!d.mark_seen("k1"));
    }

    #[test]
    fn capacity_evicts_earliest_live() {
        let mut d = Deduper::new(DEFAULT_TTL, 2).unwrap();
        assert!(d.mark_seen("a"));
        assert!(d.mark_seen("b"));
        assert!(!d.mark_seen("b"));
        assert!(d.mark_seen("c")); // 淘汰最早的 a
        assert!(d.mark_seen("a")); // a 已被淘汰，可再次放行
        assert!(!d.mark_seen("c"));
    }
}
