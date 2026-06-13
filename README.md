# Oxide Ring

**Oxide Ring** is a ring buffer for GPU event logging with ternary overflow state — `+1 (Normal)`, `0 (NearFull)`, `-1 (Overflowed)` — providing lossless event recording until capacity is exceeded, then graceful degradation with oldest-event dropping and overflow tracking.

## Why It Matters

GPU kernels generate thousands of events per millisecond: kernel launches, memory transfers, synchronization points, and errors. Logging all of them requires a buffer that can absorb burst writes without blocking the GPU. Oxide Ring provides this: a bounded ring buffer that accepts events at **O(1)** write cost, never blocks (overwrites oldest on overflow), and exposes a ternary state — Normal, NearFull, or Overflowed — so consumers know whether they're reading complete or potentially-gapped history. The NearFull state (configurable threshold, default 80%) provides early warning before overflow occurs.

## How It Works

### Ring Buffer Structure

```
OxideRing {
    buffer: VecDeque<Event>,
    capacity: usize,
    next_id: u64,
    dropped: u64,
    total_written: u64,
    warn_threshold: f64,  // default 0.8 (80%)
}
```

### Write Operation

```
write(kind, data, timestamp) → event_id:
    id = next_id++
    total_written++
    if buffer.len() >= capacity:
        buffer.pop_front()   // drop oldest
        dropped++
    buffer.push_back(Event { id, kind, data, timestamp })
    return id
```

Write: **O(1)** amortized (VecDeque push_back). When capacity is full, pop_front is also **O(1)** amortized.

### Read Operation

```
read() → Option<Event>:
    buffer.pop_front()
```

Read: **O(1)** amortized. `peek()` (read without removing): **O(1)**.

### Ternary State Computation

```
state() → BufferState:
    ratio = buffer.len() / capacity
    if dropped > 0: Overflowed (-1)
    else if ratio >= warn_threshold: NearFull (0)
    else: Normal (+1)
```

State check: **O(1)** (length comparison). Note: once Overflowed, the state stays Overflowed even after reading — the `dropped` counter is cumulative.

### Query Operations

```
query_by_kind(kind) → Vec<&Event>:
    buffer.iter().filter(|e| e.kind == kind).collect()
```

Filter: **O(N)** where N = current buffer length. `drain()` (remove all): **O(N)**.

### Statistics

```
fill_ratio() = buffer.len() / capacity
dropped_count() = dropped (cumulative)
total_written() = total_written (cumulative)
```

All **O(1)** to read.

### Memory Layout

Each `Event` stores: `id (8B) + kind (24B String) + data (Vec<u8>) + timestamp (8B)`. For 1000 events with 32-byte payloads: ~64KB total. A 10K-event buffer: ~640KB, fitting in L2 cache.

## Quick Start

```rust
use oxide_ring::{OxideRing, BufferState};

let mut ring = OxideRing::new(1000);

// Write events
for i in 0..100 {
    ring.write("kernel_launch", &i.to_le_bytes(), i * 1000);
}

// Check state
assert_eq!(ring.state(), BufferState::Normal);
println!("Fill ratio: {:.1}%", ring.fill_ratio() * 100);

// Read events
while let Some(event) = ring.read() {
    println!("Event {}: {} at {}μs", event.id, event.kind, event.timestamp_us);
}

// Overflow behavior
let mut small = OxideRing::new(3);
small.write("a", b"1", 1);
small.write("b", b"2", 2);
small.write("c", b"3", 3);
small.write("d", b"4", 4); // drops "a"
assert_eq!(small.state(), BufferState::Overflowed);
assert_eq!(small.dropped(), 1);
```

## API

| Type | Methods | Complexity |
|------|---------|------------|
| `OxideRing` | `new(capacity)`, `write(kind, data, ts) → u64`, `read() → Option<Event>`, `peek() → Option<&Event>` | O(1) write/read |
| `OxideRing` | `state() → BufferState`, `fill_ratio()`, `dropped()`, `total_written()`, `len()`, `is_empty()`, `capacity()` | O(1) |
| `OxideRing` | `query_by_kind(kind) → Vec<&Event>`, `drain() → Vec<Event>` | O(N) |
| `BufferState` | `Normal (+1)`, `NearFull (0)`, `Overflowed (-1)` | — |
| `Event` | `id: u64`, `kind: String`, `data: Vec<u8>`, `timestamp_us: u64` | — |

## Architecture Notes

Oxide Ring provides event logging for GPU kernel monitoring in SuperInstance. In γ + η = C, the Normal (+1) state indicates γ (growth — the system is capturing complete event history), the Overflowed (-1) state indicates η (avoidance — oldest events sacrificed to prevent writer blocking), and the NearFull (0) state is the early warning enabling proactive drain before data loss. The `total_written` counter tracks γ (total growth), `dropped` tracks η (total avoidance), and their ratio approximates C (data conservation). Integrates with `oxide-barrier` for synchronization event logging and `opentelemetry-trace` for distributed tracing export.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for GPU event logging architecture.

## References

1. Goodrich, M. T. & Tamassia, R. (2014). *Data Structures and Algorithms in Java*, 6th ed. Wiley. Chapter 6: Queue and Deque.
2. Evans, R. (2019). "Lock-Free Ring Buffers for GPU-GPU Communication." *GPU Technology Conference*.
3. Drepper, U. (2007). "What Every Programmer Should Know About Memory." *Linux Weekly News*.

## License

Apache-2.0
