//! # oxide-ring
//!
//! Ring buffer for GPU event logging with ternary overflow state.

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState { Normal = 1, NearFull = 0, Overflowed = -1 }

#[derive(Debug, Clone)]
pub struct Event {
    pub id: u64,
    pub kind: String,
    pub data: Vec<u8>,
    pub timestamp_us: u64,
}

pub struct OxideRing {
    buffer: VecDeque<Event>,
    capacity: usize,
    next_id: u64,
    dropped: u64,
    total_written: u64,
    warn_threshold: f64, // 0.0-1.0, fraction for NearFull
}

impl OxideRing {
    pub fn new(capacity: usize) -> Self {
        Self { buffer: VecDeque::with_capacity(capacity), capacity, next_id: 1, dropped: 0, total_written: 0, warn_threshold: 0.8 }
    }

    pub fn write(&mut self, kind: &str, data: &[u8], timestamp_us: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.total_written += 1;

        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
            self.dropped += 1;
        }

        self.buffer.push_back(Event { id, kind: kind.into(), data: data.into(), timestamp_us });
        id
    }

    pub fn read(&mut self) -> Option<Event> { self.buffer.pop_front() }

    pub fn peek(&self) -> Option<&Event> { self.buffer.front() }

    pub fn state(&self) -> BufferState {
        let ratio = self.buffer.len() as f64 / self.capacity as f64;
        if self.dropped > 0 { BufferState::Overflowed }
        else if ratio >= self.warn_threshold { BufferState::NearFull }
        else { BufferState::Normal }
    }

    pub fn drain(&mut self) -> Vec<Event> {
        let events: Vec<Event> = self.buffer.drain(..).collect();
        events
    }

    pub fn query_by_kind(&self, kind: &str) -> Vec<&Event> {
        self.buffer.iter().filter(|e| e.kind == kind).collect()
    }

    pub fn len(&self) -> usize { self.buffer.len() }
    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }
    pub fn capacity(&self) -> usize { self.capacity }
    pub fn dropped(&self) -> u64 { self.dropped }
    pub fn total_written(&self) -> u64 { self.total_written }
    pub fn fill_ratio(&self) -> f64 { self.buffer.len() as f64 / self.capacity as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read() {
        let mut ring = OxideRing::new(10);
        ring.write("kernel_start", b"gpu0", 100);
        let event = ring.read().unwrap();
        assert_eq!(event.kind, "kernel_start");
    }

    #[test]
    fn test_normal_state() {
        let ring = OxideRing::new(10);
        assert_eq!(ring.state(), BufferState::Normal);
    }

    #[test]
    fn test_overflow() {
        let mut ring = OxideRing::new(3);
        ring.write("a", b"1", 1);
        ring.write("b", b"2", 2);
        ring.write("c", b"3", 3);
        ring.write("d", b"4", 4); // drops "a"
        assert_eq!(ring.state(), BufferState::Overflowed);
        assert_eq!(ring.dropped(), 1);
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn test_oldest_dropped() {
        let mut ring = OxideRing::new(2);
        ring.write("first", b"1", 1);
        ring.write("second", b"2", 2);
        ring.write("third", b"3", 3);
        let event = ring.read().unwrap();
        assert_eq!(event.kind, "second"); // first was dropped
    }

    #[test]
    fn test_query_by_kind() {
        let mut ring = OxideRing::new(10);
        ring.write("gpu", b"a", 1);
        ring.write("cpu", b"b", 2);
        ring.write("gpu", b"c", 3);
        let gpu_events = ring.query_by_kind("gpu");
        assert_eq!(gpu_events.len(), 2);
    }

    #[test]
    fn test_drain() {
        let mut ring = OxideRing::new(10);
        ring.write("a", b"1", 1);
        ring.write("b", b"2", 2);
        let events = ring.drain();
        assert_eq!(events.len(), 2);
        assert!(ring.is_empty());
    }

    #[test]
    fn test_total_written() {
        let mut ring = OxideRing::new(2);
        for i in 0..5 { ring.write("x", b"d", i); }
        assert_eq!(ring.total_written(), 5);
    }

    #[test]
    fn test_fill_ratio() {
        let mut ring = OxideRing::new(10);
        ring.write("a", b"1", 1);
        assert!((ring.fill_ratio() - 0.1).abs() < 0.01);
    }
}
