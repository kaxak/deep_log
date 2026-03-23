# Changelog

## [0.3.0] - 2026-03-23

### Added
- `log_to_file(level, zones, dir)` — log vers fichier, indépendant de la console
  - Un fichier par zone : `<dir>/<ZONE>_<YYYY-MM-DD_HH-MM-SS>.log`
  - Nouveau fichier à chaque appel (datetime dans le nom)
  - Flush à chaque ligne (safe pour le debug)
  - Dossier créé automatiquement (absolu ou relatif)
  - Drop automatique en fin de programme
- `should_log_file(zone, level)` — vérifie si un message doit aller en fichier
- `LogZone::iter_single()` — itère sur les zones simples d'un bitflag
- `LogZone::name()` — nom court d'une zone simple
- `current_datetime()` — datetime UTC sans dépendance externe (stdlib uniquement)
- Macro `dlog!` mise à jour : évalue console ET fichier en un seul appel

## [0.2.0] - 2026-03-23

### Breaking Change
- Sémantique du niveau inversée : `level <= log_level` (avant `level >= log_level`)
- `set(0, zones)` → aucun log
- `set(10, zones)` → logs niveau <= 10
- `set(100, zones)` → tout afficher

## [0.1.0] - 2026-03-23

### Added
- Système de log dual-axe : niveau + zone bitflag
- Macro `dlog!(zone, level, ...)`
- 8 zones prédéfinies : BASIC, RENDER, MATRIX, SHADER, CHUNK, PHYSICS, AUDIO, NET
- Thread-safe, zéro allocation
