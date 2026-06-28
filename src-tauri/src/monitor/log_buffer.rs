use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub ts: i64,
    pub source: String,
    pub level: String,
    pub line: String,
}

pub struct LogBuffer {
    server: VecDeque<LogEntry>,
    bridge: VecDeque<LogEntry>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            server: VecDeque::with_capacity(capacity),
            bridge: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        let buf = if entry.source == "server" { &mut self.server } else { &mut self.bridge };
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    pub fn recent(&self, source: &str, limit: usize) -> Vec<LogEntry> {
        let buf = if source == "server" { &self.server } else { &self.bridge };
        let skip = buf.len().saturating_sub(limit);
        buf.iter().skip(skip).cloned().collect()
    }

    pub fn recent_all(&self, limit: usize) -> Vec<LogEntry> {
        let mut all: Vec<LogEntry> = self.server.iter().chain(self.bridge.iter()).cloned().collect();
        all.sort_by_key(|e| e.ts);
        if all.len() > limit {
            all.drain(0..all.len() - limit);
        }
        all
    }

    pub fn clear(&mut self, source: &str) {
        if source == "server" { self.server.clear(); }
        else { self.bridge.clear(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(src: &str, n: i64) -> LogEntry {
        LogEntry { ts: n, source: src.to_string(), level: "info".to_string(), line: format!("line {}", n) }
    }

    #[test]
    fn evicts_oldest_when_over_capacity() {
        let mut buf = LogBuffer::new(3);
        for i in 0..5 { buf.push(entry("server", i)); }
        let recent = buf.recent("server", 10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].line, "line 2");
    }

    #[test]
    fn recent_all_merges_and_sorts() {
        let mut buf = LogBuffer::new(100);
        buf.push(entry("server", 2));
        buf.push(entry("bridge", 1));
        buf.push(entry("server", 3));
        let all = buf.recent_all(10);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].ts, 1);
        assert_eq!(all[1].ts, 2);
        assert_eq!(all[2].ts, 3);
    }

    #[test]
    fn clear_removes_entries() {
        let mut buf = LogBuffer::new(100);
        buf.push(entry("server", 1));
        buf.clear("server");
        assert_eq!(buf.recent("server", 10).len(), 0);
    }
}
