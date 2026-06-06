# oxide-ring

Ring buffer for GPU event logging with ternary overflow state. {+1=normal, 0=near_full, -1=overflowed}. Lossless until overflow, then oldest dropped.

## Why This Matters

# oxide-ring
Ring buffer for GPU event logging with ternary overflow state.

## The Five-Layer Stack

This crate is part of the **Oxide Stack** — a distributed GPU runtime built on five layers:

```
┌─────────────────┐
│  cudaclaw        │  Persistent GPU kernels, warp consensus, SmartCRDT
├─────────────────┤
│  cuda-oxide      │  Flux → MIR → Pliron → NVVM → PTX compiler
├─────────────────┤
│  flux-core       │  Bytecode VM + A2A agent protocol
├─────────────────┤
│  pincher         │  "Vector DB as runtime, LLM as compiler"
├─────────────────┤
│  open-parallel   │  Async runtime (tokio fork)
└─────────────────┘
```

The key insight: **ternary values {-1, 0, +1} map directly to GPU compute**. They pack 16× denser than FP32, enable XNOR+popcount matmul, and conservation laws become compile-time checks.

## Design

Every value in this crate follows **ternary algebra** (Z₃):

| Value | Meaning | GPU Analog |
|-------|---------|------------|
| +1 | Positive / Active / Healthy | Warp vote yes |
| 0 | Neutral / Pending / Balanced | Warp vote abstain |
| -1 | Negative / Failed / Overloaded | Warp vote no |

This isn't arbitrary — ternary is the natural encoding for:
1. **BitNet b1.58** (Microsoft) — ternary LLMs at 60% less power
2. **GPU warp voting** — hardware ballot returns ternary consensus
3. **Conservation laws** — {-1, 0, +1} preserves quantity

## Key Types

```rust
pub enum BufferState
pub struct Event
pub struct OxideRing
pub fn new
pub fn write
pub fn read
pub fn peek
pub fn state
pub fn drain
pub fn query_by_kind
pub fn len
pub fn is_empty
```

## Usage

```toml
[dependencies]
oxide-ring = "0.1.0"
```

```rust
use oxide_ring::*;
// See src/lib.rs tests for complete working examples
```

## Testing

```bash
git clone https://github.com/SuperInstance/oxide-ring.git
cd oxide-ring
cargo test    # 8 tests
```

## Stats

| Metric | Value |
|--------|-------|
| Tests | 8 |
| Lines of Rust | 148 |
| Public API | 16 items |

## License

Apache-2.0
