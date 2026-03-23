# Changelog

## [0.2.0] - 2026-03-23

### Breaking Change
- **Sémantique du niveau inversée** : un message s'affiche maintenant si `son_niveau <= log_level`
  - `set(0,   zones)` → aucun log (avant : tout afficher)
  - `set(10,  zones)` → logs de niveau <= 10 (info normale)
  - `set(100, zones)` → tout afficher
  - `set_all()` → équivalent à `set(100, LogZone::ALL)`
  - `set_none()` → équivalent à `set(0, LogZone::ALL)`
- Tous les tests mis à jour pour la nouvelle sémantique

## [0.1.0] - 2026-03-23

### Added
- Système de log dual-axe : niveau + zone bitflag
- Macro `dlog!(zone, level, ...)`
- 8 zones prédéfinies : BASIC, RENDER, MATRIX, SHADER, CHUNK, PHYSICS, AUDIO, NET
- Zones custom avec `LogZone::custom(bit)`
- Thread-safe, zéro allocation
