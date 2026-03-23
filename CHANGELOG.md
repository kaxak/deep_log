# Changelog

All notable changes to `deep_log` will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-03-23

### Added
- `LogZone` — bitflag type with 8 predefined zones : `BASIC`, `RENDER`, `MATRIX`, `SHADER`, `CHUNK`, `PHYSICS`, `AUDIO`, `NET`
- `LogZone::custom(bit)` — define your own zones using bits 8→31
- `LogZone::ALL` and `LogZone::NONE` constants
- `dlog!(zone, level, ...)` macro — displays a message if `level >= log_level` AND `zone ∈ active_zones`
- `deep_log::set(level, zones)` — configure level and active zones
- `deep_log::set_all()` — enable everything
- `deep_log::set_none()` — disable everything
- `deep_log::level()` and `deep_log::zones()` — query current configuration
- `deep_log::should_log(zone, level)` — manual check for expensive computations
- Thread-safe implementation using `AtomicU8` and `AtomicU32`
- Zero allocation — all filtering is done with integer comparisons
- 7 unit tests + 6 doc-tests
