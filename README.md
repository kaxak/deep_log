# deep_log

A dual-axis logging system for Rust — **level** (verbosity) + **zone** (bitflag per system).

Two orthogonal axes mean you can be verbose on `MATRIX` and silent on `CHUNK` at the same time.

---

## Concept

Most logging systems give you a single verbosity axis (`debug`, `info`, `warn`, `error`).  
`deep_log` adds a second axis — **zones** — so you can target exactly which part of your code you want to hear from.

```
A message is displayed if:
  its_level >= log_level   (verbosity threshold)
  AND
  its_zone ∈ active_zones  (system filter)
```

| `log_level` | Effect |
|-------------|--------|
| `0`         | Everything is displayed |
| `50`        | Only messages with level ≥ 50 |
| `100`       | Only the most critical messages |

---

## Installation

```toml
[dependencies]
deep_log = "0.1.0"
```

---

## Quick start

```rust
use deep_log::{LogZone, dlog};

fn main() {
    // Show everything, all zones
    deep_log::set(0, LogZone::ALL);

    dlog!(LogZone::BASIC,  10,  "GPU : {}", "RTX 2080 Ti");
    dlog!(LogZone::RENDER, 20,  "Surface : 1280x720");
    dlog!(LogZone::MATRIX, 50,  "NDC vertex 0 : [{:.3}, {:.3}, {:.3}]", 0.029, 0.071, 0.980);
    dlog!(LogZone::CHUNK,  100, "Face {} emitted", 42);
}
```

Output:
```
[BASIC|10]  GPU : RTX 2080 Ti
[RENDER|20] Surface : 1280x720
[MATRIX|50] NDC vertex 0 : [0.029, 0.071, 0.980]
[CHUNK|100] Face 42 emitted
```

---

## Filtering

```rust
use deep_log::LogZone;

// Only MATRIX zone, level >= 50
deep_log::set(50, LogZone::MATRIX);

// Multiple zones, level >= 10
deep_log::set(10, LogZone::BASIC | LogZone::RENDER | LogZone::MATRIX);

// Everything
deep_log::set_all();

// Nothing
deep_log::set_none();
```

---

## Predefined zones

| Zone      | Bit     | Intended use                        |
|-----------|---------|-------------------------------------|
| `BASIC`   | `1 << 0`| Init, startup, configuration        |
| `RENDER`  | `1 << 1`| Pipeline, draw calls, frames        |
| `MATRIX`  | `1 << 2`| Matrices, NDC, camera               |
| `SHADER`  | `1 << 3`| Uniforms, bindings, shaders         |
| `CHUNK`   | `1 << 4`| Meshing, voxels, world generation   |
| `PHYSICS` | `1 << 5`| Collisions, raycasts                |
| `AUDIO`   | `1 << 6`| Audio, sounds                       |
| `NET`     | `1 << 7`| Network, serialization              |
| `ALL`     | `u32::MAX` | All zones                        |
| `NONE`    | `0`     | No zone                             |

---

## Custom zones

Use bits 8→31 to define your own zones without colliding with predefined ones:

```rust
use deep_log::LogZone;

const AI:       LogZone = LogZone::custom(1 << 8);
const PATHFIND: LogZone = LogZone::custom(1 << 9);
const INVENTORY:LogZone = LogZone::custom(1 << 10);

deep_log::set(0, AI | PATHFIND);

dlog!(AI,       20, "Agent {} thinking...", agent_id);
dlog!(PATHFIND, 50, "Path found : {} nodes", path.len());
dlog!(INVENTORY, 10, "invisible — zone not active");
```

---

## Suggested levels

| Level | Usage                              |
|-------|------------------------------------|
| `1`   | Fatal errors                       |
| `10`  | Normal info (startup, GPU name...) |
| `20`  | Render stats                       |
| `50`  | Matrices, NDC validation           |
| `70`  | Meshing, chunk data                |
| `100` | Frame-by-frame verbose             |

---

## Thread safety

`deep_log` uses `AtomicU8` and `AtomicU32` internally — configuration and logging are fully thread-safe with zero allocation.

---

## Runtime check

You can query the current configuration:

```rust
let current_level = deep_log::level();
let current_zones = deep_log::zones();

// Manual check (same condition as dlog!)
if deep_log::should_log(LogZone::MATRIX, 50) {
    // expensive computation only when needed
    let ndc = compute_ndc();
    dlog!(LogZone::MATRIX, 50, "NDC : {:?}", ndc);
}
```

---

## Why not `log` / `tracing`?

`log` and `tracing` are excellent general-purpose crates.  
`deep_log` is purpose-built for **real-time systems** (games, simulations, renderers) where:

- You need to silence an entire subsystem (e.g. mute `CHUNK` while debugging `MATRIX`) with a single call
- You want zero-overhead filtering — two integer comparisons, no string parsing
- You don't want to pull in a macro ecosystem just to print a matrix

---

## License

MIT OR Apache-2.0
