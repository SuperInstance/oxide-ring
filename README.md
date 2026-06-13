# Oxide Ring

**Oxide Ring** is a GPU event ring buffer with ternary overflow state — `+1` (normal), `0` (near full at 80% capacity), `-1` (overflowed, dropping oldest) — providing bounded-memory event logging for GPU kernel diagnostics.

## Why It Matters

GPU kernels emit millions of events per second — profiling data, error markers, memory transfers. An unbounded log causes GPU OOM; a fixed-size circular buffer loses old data silently. Oxide Ring provides the middle ground: a bounded ring buffer that signals its state via ternary overflow flags. At 80% capacity, it enters `NearFull` (0), giving consumers a warning to drain. Once full, it enters `Overflowed` (-1) and drops the oldest events, tracking total dropped count for health monitoring.

## How It Works

### Ring Buffer Mechanics

The ring uses a `VecDeque<Event>` with fixed capacity:

```
write(event):
  if buffer.len() >= capacity:
    buffer.pop_front()    // drop oldest
    dropped += 1
  buffer.push_back(event)
  total_written += 1
```

Write cost: **O(1)** amortized (VecDeque push_back). Pop front: **O(1)**.

### Ternary State Computation

```
fill_ratio = buffer.len() / capacity

if fill_ratio < warn_threshold (0.8):  → Normal (+1)
if warn_threshold ≤ fill_ratio < 1.0:  → NearFull (0)
if dropped > 0 (overflow occurred):     → Overflowed (-1)
```

The `warn_threshold` is configurable (default 0.8). State computation: **O(1)**.

### Event Structure

Each event contains:
- `id: u64` — monotonically increasing sequence number
- `kind: String` — event type tag (e.g., "kernel_launch", "memcpy")
- `data: Vec<u8>` — opaque payload
- `timestamp_us: u64` — microsecond timestamp

### Sequential Read

Consumers read events sequentially:

```
read_all() → Vec<&Event>    // O(N) where N = current buffer size
read_since(last_id) → Vec<&Event>    // O(N) with binary search for last_id
```

The `id` field enables gap detection — if a consumer sees IDs 1,2,5,6, it knows events 3,4 were dropped due to overflow.

### Statistics

```
total_written: u64   // lifetime write count
dropped: u64         // lifetime drop count
fill_ratio: f64      // current buffer utilization
```

All **O(1)** to compute from tracked counters.

## Quick Start

```rust
use oxide_ring::{OxideRing, BufferState};

let mut ring = OxideRing::new(1024); // 1024-event capacity

for i in 0..2000 {
    ring.write("kernel_done", &[i as u8], i * 1000);
}

println!("State: {:?}", ring.state());       // NearFull or Overflowed
println!("Written: {}", ring.total_written()); // 2000
println!("Dropped: {}", ring.dropped());       // 976
```

## API

| Type | Description |
|------|-------------|
| `OxideRing` | Bounded ring buffer with ternary overflow tracking |
| `Event` | id, kind, data, timestamp_us |
| `BufferState` | `Normal (+1)`, `NearFull (0)`, `Overflowed (-1)` |

Key methods: `write(kind, data, ts)`, `state()`, `dropped()`, `total_written()`, `set_warn_threshold(ratio)`.

## Architecture Notes

Oxide Ring provides event logging for GPU operations in the oxide-* stack. In γ + η = C, the ring enables γ (growth — capturing diagnostic events for analysis) while the ternary overflow state provides η (avoidance — NearFull signals consumers to drain before data loss; Overflowed tracks what was lost). Works with `oxide-barrier` for synchronized buffer access and `oxide-epoch` for safe event reclamation.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for GPU diagnostics architecture.

## References

1. Goodfellow, M. (2004). "Circular Buffers in Real-Time Systems." *Embedded Systems Programming*.
2. Nichols, B. et al. (1996). *Pthreads Programming*. O'Reilly. (On bounded buffer synchronization)
3. NVIDIA (2024). "CUPTI: CUDA Profiling Tools Interface." *NVIDIA Developer Documentation*.

## License

MIT
