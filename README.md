# deep_log

A dual-axis logging system for Rust — **level** (verbosity) + **zone** (bitflag per system).  
Two orthogonal axes, two independent outputs: **console** and **file**.

---

## Concept

```
A message is displayed/written if:
  its_level <= log_level   (verbosity threshold)
  AND
  its_zone ∈ active_zones  (system filter)
```

| `log_level` | Effect |
|-------------|--------|
| `0`         | Nothing is logged |
| `10`        | Messages with level <= 10 (normal info) |
| `50`        | Messages with level <= 50 (details) |
| `100`       | Everything |

---

## Installation

```toml
[dependencies]
deep_log = "0.3.0"
```

---

## Quick start

```rust
use deep_log::{LogZone, dlog};

fn main() {
    // Console : normal info, RENDER zone only
    deep_log::set(10, LogZone::RENDER);

    // File : everything from BASIC, written to logs/
    deep_log::log_to_file(100, LogZone::BASIC, "logs/");
    // → creates : logs/BASIC_2026-03-23_14-30-00.log

    dlog!(LogZone::RENDER, 10, "Surface : 1280x720");   // console only
    dlog!(LogZone::BASIC,  10, "GPU : RTX 2080 Ti");    // file only
    dlog!(LogZone::BASIC, 100, "Verbose detail");        // file only
    dlog!(LogZone::MATRIX, 10, "invisible");             // neither (zone not active)
}
```

---

## Console output — `set()`

```rust
use deep_log::LogZone;

deep_log::set(10,  LogZone::BASIC | LogZone::RENDER); // normal info
deep_log::set(100, LogZone::ALL);                     // everything
deep_log::set(0,   LogZone::ALL);                     // nothing
deep_log::set_all();   // equivalent to set(100, LogZone::ALL)
deep_log::set_none();  // equivalent to set(0, LogZone::ALL)
```

---

## File output — `log_to_file()`

```rust
use deep_log::LogZone;

// One file per zone, in the given directory (absolute or relative)
deep_log::log_to_file(100, LogZone::BASIC | LogZone::PHYSICS, "logs/");
// → logs/BASIC_2026-03-23_14-30-00.log
// → logs/PHYSICS_2026-03-23_14-30-00.log
```

**Behaviour:**
- New file on every call (datetime in filename — no overwriting)
- Flush on every line (safe for crash debugging)
- One file per zone
- Directory created automatically if it doesn't exist
- Files closed automatically on program exit (drop)

---

## Console and file are independent

```rust
// Show only normal info on console
deep_log::set(10, LogZone::RENDER);

// Log everything to file, different zone
deep_log::log_to_file(100, LogZone::PHYSICS, "logs/");

// This goes to file only (PHYSICS not in console zones)
dlog!(LogZone::PHYSICS, 70, "collision at y=14.3");

// This goes to console only (RENDER not in file zones)
dlog!(LogZone::RENDER, 10, "frame rendered");
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

Use bits 8→31 to avoid collisions with predefined zones:

```rust
use deep_log::LogZone;

const AI:       LogZone = LogZone::custom(1 << 8);
const PATHFIND: LogZone = LogZone::custom(1 << 9);

deep_log::set(100, AI | PATHFIND);
deep_log::log_to_file(50, AI, "logs/");

dlog!(AI, 10, "Agent {} thinking", agent_id);
```

---

## Suggested levels

| Level | Usage                              |
|-------|------------------------------------|
| `10`  | Normal info (startup, GPU name...) |
| `20`  | Render stats                       |
| `50`  | Matrices, NDC validation           |
| `70`  | Meshing, chunk data                |
| `100` | Frame-by-frame verbose             |

---

## Thread safety

All configuration uses `AtomicU8` and `AtomicU32`. File writes are protected by a `Mutex`.  
Zero allocation on the hot path (console only). File writes allocate a formatted string per message.

---

## No external dependencies

`current_datetime()` is implemented using only `std::time::SystemTime` — no `chrono`, no `time`.

---

## License

MIT OR Apache-2.0
