# oxide-ring

*Ring buffer for GPU event logging with ternary overflow state. Events flow in, old events flow out — but you always know whether what you lost mattered.*

## Why This Exists

GPU event logging has a unique constraint: you can't pause the pipeline to flush. Kernels run asynchronously, events arrive faster than you can read them, and the only sane data structure is a ring buffer — old events get overwritten by new ones. But "overwritten" isn't the end of the story. Sometimes the lost events were noise. Sometimes they were critical. The ternary overflow state distinguishes between three outcomes:

- **+1 (Aggregated):** Overflow was productive — events were merged into summaries
- **0 (Clean):** No overflow — every event was preserved
- **-1 (Lost):** Events were dropped without processing

## Architecture

```
Write Pointer ──→ [E₁][E₂][E₃][E₄][E₅][E₆][E₇][E₈] ──→ Read Pointer
                   ↑                                      ↑
               Oldest (overwritten first)            Newest

Overflow State Machine:
  Clean (0) ──buffer full──→ Lost (-1) ──enable aggregation──→ Aggregated (+1)
  Aggregated (+1) ──reader catches up──→ Clean (0)
```

### Key Types

- **`RingBuffer<T>`** — Bounded ring buffer with ternary overflow tracking. Generic over event type. Push overwrites oldest when full.
- **`RingReader<T>`** — Cursor-based reader that tracks how far behind it is. Reports lag in events.
- **`OverflowState`** — Enum: Lost / Clean / Aggregated. Updated automatically on push.
- **`RingStats`** — Total pushes, total reads, overflow count, current lag.

## Usage

```rust
use oxide_ring::*;

// Create a 1024-event ring buffer
let mut ring: RingBuffer<u64> = RingBuffer::new(1024);

// Push events (from GPU kernel callbacks)
for i in 0..2000 {
    ring.push(i); // Overwrites oldest after 1024
}

// Check overflow state
match ring.overflow_state() {
    OverflowState::Lost => println!("Events were dropped!"),
    OverflowState::Clean => println!("All events preserved"),
    OverflowState::Aggregated => println!("Events merged into summaries"),
}

// Read with a cursor
let mut reader = ring.reader();
while let Some(event) = reader.next() {
    process(event);
}
println!("Reader lag: {} events", reader.lag());
```

## The Deeper Idea

Ring buffers are the simplest possible streaming data structure. On the GPU, they're also the most practical — a kernel can write events to a ring buffer with a single atomic increment of the write pointer, no synchronization needed. The ternary overflow state adds metadata without adding latency.

This pattern appears everywhere in the SuperInstance ecosystem: `ternary-walsh` (streaming Walsh transforms), `oxide-journal` (WAL with similar overflow semantics), and `agent-transcription` (streaming event capture).

## Related Crates

- `oxide-journal` — Write-ahead log (durable version of ring semantics)
- `oxide-chunk` — Memory chunk management for ring buffer backing storage
- `oxide-sandbox` — Safe execution environment that uses rings for event capture
- `agent-transcription` — Agent event streaming using the same overflow model
