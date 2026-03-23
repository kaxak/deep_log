//! # deep_log
//!
//! Système de log à deux axes orthogonaux : niveau + zone (bitflag).
//!
//! ## Deux sorties indépendantes
//!
//! - **Console** : `set(level, zones)` — affiche sur stderr
//! - **Fichier** : `log_to_file(level, zones, dir)` — un fichier par zone
//!
//! Les deux sorties ont leur propre niveau et zones — totalement indépendants.
//!
//! ```rust
//! use deep_log::LogZone;
//!
//! // Console : info normale sur RENDER seulement
//! deep_log::set(10, LogZone::RENDER);
//!
//! // Fichier : tout BASIC dans logs/
//! deep_log::log_to_file(100, LogZone::BASIC, "logs/");
//! // → génère : logs/BASIC_2026-03-23_14-30-00.log
//! ```
//!
//! ## Niveaux
//!
//! `set(niveau_max, zones)` — affiche les messages de niveau <= niveau_max.
//! - `set(0,   zones)` → aucun log
//! - `set(10,  zones)` → info normale
//! - `set(100, zones)` → tout

use std::collections::HashMap;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// LogZone — bitflags
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogZone(pub u32);

impl LogZone {
    pub const NONE:    Self = Self(0);
    pub const BASIC:   Self = Self(1 << 0);
    pub const RENDER:  Self = Self(1 << 1);
    pub const MATRIX:  Self = Self(1 << 2);
    pub const SHADER:  Self = Self(1 << 3);
    pub const CHUNK:   Self = Self(1 << 4);
    pub const PHYSICS: Self = Self(1 << 5);
    pub const AUDIO:   Self = Self(1 << 6);
    pub const NET:     Self = Self(1 << 7);
    pub const ALL:     Self = Self(u32::MAX);

    pub const fn custom(bit: u32) -> Self { Self(bit) }

    #[inline]
    pub fn contains(self, other: Self) -> bool { (self.0 & other.0) != 0 }

    /// Itère sur chaque zone simple (un bit) contenu dans ce bitflag
    pub fn iter_single(self) -> impl Iterator<Item = LogZone> {
        (0..32u8)
            .filter(move |&bit| self.0 & (1u32 << bit) != 0)
            .map(|bit| LogZone(1u32 << bit))
    }

    /// Nom d'une zone simple (un seul bit actif)
    pub fn name(self) -> &'static str {
        match self {
            Self::BASIC   => "BASIC",
            Self::RENDER  => "RENDER",
            Self::MATRIX  => "MATRIX",
            Self::SHADER  => "SHADER",
            Self::CHUNK   => "CHUNK",
            Self::PHYSICS => "PHYSICS",
            Self::AUDIO   => "AUDIO",
            Self::NET     => "NET",
            _             => "CUSTOM",
        }
    }
}

impl std::ops::BitOr for LogZone {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

impl std::ops::BitOrAssign for LogZone {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl std::fmt::Debug for LogZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0        { return write!(f, "NONE"); }
        if self.0 == u32::MAX { return write!(f, "ALL");  }
        let mut names: Vec<&str> = Vec::new();
        if self.contains(Self::BASIC)   { names.push("BASIC");   }
        if self.contains(Self::RENDER)  { names.push("RENDER");  }
        if self.contains(Self::MATRIX)  { names.push("MATRIX");  }
        if self.contains(Self::SHADER)  { names.push("SHADER");  }
        if self.contains(Self::CHUNK)   { names.push("CHUNK");   }
        if self.contains(Self::PHYSICS) { names.push("PHYSICS"); }
        if self.contains(Self::AUDIO)   { names.push("AUDIO");   }
        if self.contains(Self::NET)     { names.push("NET");      }
        if self.0 & !0xFF != 0          { names.push("CUSTOM");   }
        write!(f, "{}", names.join("|"))
    }
}

impl std::fmt::Display for LogZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}

// ---------------------------------------------------------------------------
// Config console
// ---------------------------------------------------------------------------

static LOG_LEVEL: AtomicU8  = AtomicU8::new(0);
static LOG_ZONES: AtomicU32 = AtomicU32::new(0);

/// Configure la sortie console.
/// `set(niveau_max, zones)` — affiche les messages de niveau <= niveau_max dans les zones.
/// - `set(0,   zones)` → aucun log
/// - `set(10,  zones)` → info normale (niveau <= 10)
/// - `set(100, zones)` → tout afficher
pub fn set(level: u8, zones: LogZone) {
    LOG_LEVEL.store(level, Ordering::Relaxed);
    LOG_ZONES.store(zones.0, Ordering::Relaxed);
}

pub fn set_all()  { set(100, LogZone::ALL); }
pub fn set_none() { set(0,   LogZone::ALL); }
pub fn level()    -> u8      { LOG_LEVEL.load(Ordering::Relaxed) }
pub fn zones()    -> LogZone { LogZone(LOG_ZONES.load(Ordering::Relaxed)) }

#[inline]
pub fn should_log(zone: LogZone, level: u8) -> bool {
    let max    = LOG_LEVEL.load(Ordering::Relaxed);
    let active = LogZone(LOG_ZONES.load(Ordering::Relaxed));
    max > 0 && active.contains(zone) && level <= max
}

// ---------------------------------------------------------------------------
// Config fichier
// ---------------------------------------------------------------------------

static FILE_LEVEL: AtomicU8  = AtomicU8::new(0);
static FILE_ZONES: AtomicU32 = AtomicU32::new(0);

#[inline]
pub fn should_log_file(zone: LogZone, level: u8) -> bool {
    let max    = FILE_LEVEL.load(Ordering::Relaxed);
    let active = LogZone(FILE_ZONES.load(Ordering::Relaxed));
    max > 0 && active.contains(zone) && level <= max
}

// ---------------------------------------------------------------------------
// FileLogger — un File par zone, flush à chaque ligne
// ---------------------------------------------------------------------------

struct FileLogger {
    // clé = LogZone.0 (un seul bit), valeur = fichier ouvert
    files: HashMap<u32, std::fs::File>,
}

impl FileLogger {
    fn new() -> Self { Self { files: HashMap::new() } }

    /// Ouvre un fichier par zone simple dans `dir`, avec `datetime` dans le nom.
    /// Format : `<dir>/<ZONE>_<datetime>.log`
    fn open_zones(&mut self, zones: LogZone, dir: &str, datetime: &str) {
        let dir = dir.trim_end_matches('/');
        for single in zones.iter_single() {
            if self.files.contains_key(&single.0) { continue; }
            let path = format!("{}{}{}_{}.log",
                dir,
                std::path::MAIN_SEPARATOR,
                single.name(),
                datetime);
            match OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
                Ok(f) => {
                    self.files.insert(single.0, f);
                    eprintln!("[deep_log] fichier log ouvert : {}", path);
                }
                Err(e) => {
                    eprintln!("[deep_log] erreur ouverture {} : {}", path, e);
                }
            }
        }
    }

    /// Écrit dans tous les fichiers correspondant à la zone, flush immédiat
    fn write(&mut self, zone: LogZone, msg: &str) {
        for single in zone.iter_single() {
            if let Some(file) = self.files.get_mut(&single.0) {
                let _ = writeln!(file, "{}", msg);
                let _ = file.flush();
            }
        }
    }
}

// OnceLock — initialisé au premier appel de log_to_file()
static FILE_LOGGER: OnceLock<Mutex<FileLogger>> = OnceLock::new();

fn file_logger() -> &'static Mutex<FileLogger> {
    FILE_LOGGER.get_or_init(|| Mutex::new(FileLogger::new()))
}

// ---------------------------------------------------------------------------
// Datetime — sans dépendance externe (stdlib uniquement)
// Utilise SystemTime pour obtenir un timestamp, converti en YYYY-MM-DD_HH-MM-SS
// ---------------------------------------------------------------------------

fn current_datetime() -> String {
    // Secondes depuis UNIX epoch
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Conversion manuelle en date/heure UTC
    let s   = secs % 60;
    let m   = (secs / 60) % 60;
    let h   = (secs / 3600) % 24;
    let days = secs / 86400; // jours depuis 1970-01-01

    // Algorithme de conversion jours → date (algorithme de Fliegel & Van Flandern)
    let z   = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let d   = doy - (153 * mp + 2) / 5 + 1;
    let mo  = if mp < 10 { mp + 3 } else { mp - 9 };
    let y   = if mo <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}", y, mo, d, h, m, s)
}

// ---------------------------------------------------------------------------
// API publique — log fichier
// ---------------------------------------------------------------------------

/// Active le log vers fichier pour les zones demandées.
///
/// - `level`  : niveau max (0 = désactivé, 100 = tout)
/// - `zones`  : zones à logger (indépendant de `set()`)
/// - `dir`    : dossier de destination (absolu ou relatif), créé si absent
///
/// Génère un fichier par zone : `<dir>/<ZONE>_<YYYY-MM-DD_HH-MM-SS>.log`
/// Nouveau fichier à chaque appel (datetime dans le nom → pas d'écrasement).
/// Flush à chaque ligne.
///
/// ```rust
/// use deep_log::LogZone;
/// // Console : RENDER en info normale
/// deep_log::set(10, LogZone::RENDER);
/// // Fichier : tout BASIC dans logs/
/// deep_log::log_to_file(100, LogZone::BASIC, "logs/");
/// ```
pub fn log_to_file(level: u8, zones: LogZone, dir: &str) {
    if let Err(e) = create_dir_all(dir) {
        eprintln!("[deep_log] impossible de créer {} : {}", dir, e);
        return;
    }

    let datetime = current_datetime();

    file_logger()
        .lock()
        .unwrap()
        .open_zones(zones, dir, &datetime);

    // Active les atomics fichier
    FILE_LEVEL.store(level, Ordering::Relaxed);
    FILE_ZONES.fetch_or(zones.0, Ordering::Relaxed); // | pour ne pas écraser les zones déjà actives
}

// ---------------------------------------------------------------------------
// print — console + fichier
// ---------------------------------------------------------------------------

#[inline]
pub fn print(zone: LogZone, level: u8, msg: &str) {
    // Console
    eprintln!("[{:?}|{}] {}", zone, level, msg);

    // Fichier — si la zone est active pour le fichier
    if should_log_file(zone, level) {
        if let Ok(mut logger) = file_logger().lock() {
            logger.write(zone, &format!("[{:?}|{}] {}", zone, level, msg));
        }
    }
}

// ---------------------------------------------------------------------------
// Macro principale
// ---------------------------------------------------------------------------

/// Log à deux axes : zone + niveau.
///
/// Sortie console si `zone ∈ zones console` ET `niveau <= log_level console`.
/// Sortie fichier  si `zone ∈ zones fichier` ET `niveau <= log_level fichier`.
///
/// ```rust
/// use deep_log::{LogZone, dlog};
/// deep_log::set(10, LogZone::ALL);
/// dlog!(LogZone::BASIC, 10, "affiché en console");
/// dlog!(LogZone::BASIC, 50, "pas affiché — 50 > 10");
/// ```
#[macro_export]
macro_rules! dlog {
    ($zone:expr, $level:expr, $($arg:tt)*) => {
        if $crate::should_log($zone, $level) || $crate::should_log_file($zone, $level) {
            $crate::print($zone, $level, &format!($($arg)*));
        }
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_none_affiche_rien() {
        set(0, LogZone::ALL);
        assert!(!should_log(LogZone::BASIC,  10));
        assert!(!should_log(LogZone::BASIC, 100));
    }

    #[test]
    fn test_set_10_affiche_inferieur_egal() {
        set(10, LogZone::ALL);
        assert!( should_log(LogZone::BASIC,  1));
        assert!( should_log(LogZone::BASIC, 10));
        assert!(!should_log(LogZone::BASIC, 11));
        assert!(!should_log(LogZone::BASIC, 50));
    }

    #[test]
    fn test_set_100_affiche_tout() {
        set(100, LogZone::ALL);
        assert!(should_log(LogZone::BASIC,   1));
        assert!(should_log(LogZone::BASIC,  10));
        assert!(should_log(LogZone::BASIC,  50));
        assert!(should_log(LogZone::BASIC, 100));
    }

    #[test]
    fn test_zones_filtrent() {
        set(100, LogZone::BASIC | LogZone::MATRIX);
        assert!( should_log(LogZone::BASIC,  10));
        assert!( should_log(LogZone::MATRIX, 50));
        assert!(!should_log(LogZone::RENDER, 10));
        assert!(!should_log(LogZone::CHUNK,  10));
    }

    #[test]
    fn test_custom_zone() {
        const MY_AI: LogZone = LogZone::custom(1 << 8);
        set(10, MY_AI);
        assert!( should_log(MY_AI,          10));
        assert!(!should_log(LogZone::BASIC,  10));
    }

    #[test]
    fn test_zone_debug() {
        let z = LogZone::BASIC | LogZone::MATRIX;
        assert_eq!(format!("{:?}", z), "BASIC|MATRIX");
        assert_eq!(format!("{:?}", LogZone::NONE), "NONE");
        assert_eq!(format!("{:?}", LogZone::ALL),  "ALL");
    }

    #[test]
    fn test_iter_single() {
        let z = LogZone::BASIC | LogZone::PHYSICS;
        let singles: Vec<u32> = z.iter_single().map(|z| z.0).collect();
        assert!(singles.contains(&LogZone::BASIC.0));
        assert!(singles.contains(&LogZone::PHYSICS.0));
        assert_eq!(singles.len(), 2);
    }

    #[test]
    fn test_zone_name() {
        assert_eq!(LogZone::BASIC.name(),   "BASIC");
        assert_eq!(LogZone::RENDER.name(),  "RENDER");
        assert_eq!(LogZone::PHYSICS.name(), "PHYSICS");
    }

    #[test]
    fn test_should_log_file() {
        FILE_LEVEL.store(50, Ordering::Relaxed);
        FILE_ZONES.store(LogZone::BASIC.0, Ordering::Relaxed);
        assert!( should_log_file(LogZone::BASIC,  10));
        assert!( should_log_file(LogZone::BASIC,  50));
        assert!(!should_log_file(LogZone::BASIC,  51));
        assert!(!should_log_file(LogZone::RENDER, 10));
    }

    #[test]
    fn test_datetime_format() {
        let dt = current_datetime();
        // Format attendu : YYYY-MM-DD_HH-MM-SS (19 chars)
        assert_eq!(dt.len(), 19, "datetime = '{}'", dt);
        assert_eq!(&dt[4..5],  "-");
        assert_eq!(&dt[7..8],  "-");
        assert_eq!(&dt[10..11], "_");
        assert_eq!(&dt[13..14], "-");
        assert_eq!(&dt[16..17], "-");
    }
}
