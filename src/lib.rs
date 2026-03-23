//! # deep_log
//!
//! Système de log à deux axes orthogonaux :
//!
//! - **Niveau** : verbosité de 1 à 100.
//!   Un message s'affiche si `son_niveau <= log_level`.
//!   `log_level = 0`   → aucun log.
//!   `log_level = 10`  → logs de niveau <= 10 (info normale).
//!   `log_level = 100` → tout afficher.
//!
//! - **Zone** : bitflag par système (BASIC, RENDER, MATRIX, SHADER, CHUNK, PHYSICS...).
//!   Un message s'affiche seulement si sa zone est dans le set actif.
//!
//! Les deux axes sont **orthogonaux** : on peut être verbeux sur MATRIX
//! et silencieux sur CHUNK simultanément.
//!
//! ## Usage rapide
//!
//! ```rust
//! use deep_log::{LogZone, dlog};
//!
//! fn main() {
//!     // Afficher les logs info (niveau <= 10), zones BASIC et RENDER
//!     deep_log::set(10, LogZone::BASIC | LogZone::RENDER);
//!
//!     dlog!(LogZone::BASIC,  10, "GPU : {}", "RTX 4090");   // affiché
//!     dlog!(LogZone::RENDER, 20, "Stats de rendu");          // pas affiché (20 > 10)
//!     dlog!(LogZone::MATRIX, 10, "NDC");                     // pas affiché (zone inactive)
//!
//!     // Tout afficher
//!     deep_log::set(100, LogZone::ALL);
//!
//!     // Aucun log
//!     deep_log::set(0, LogZone::ALL);
//! }
//! ```
//!
//! ## Niveaux conseillés
//!
//! | Niveau | Usage                              |
//! |--------|------------------------------------|
//! | 10     | Info normale (démarrage, GPU...)   |
//! | 20     | Stats de rendu                     |
//! | 50     | Matrices, NDC, caméra              |
//! | 70     | Meshing, chunks                    |
//! | 100    | Tout (frame-by-frame verbose)      |
//!
//! ## Zones prédéfinies
//!
//! ```rust
//! use deep_log::LogZone;
//! deep_log::set(10, LogZone::BASIC | LogZone::MATRIX);
//! ```

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

// ---------------------------------------------------------------------------
// LogZone — bitflags
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
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

    /// Crée une zone personnalisée (bits 8→31).
    /// ```rust
    /// use deep_log::LogZone;
    /// const MY_SYSTEM: LogZone = LogZone::custom(1 << 8);
    /// ```
    pub const fn custom(bit: u32) -> Self { Self(bit) }

    #[inline]
    pub fn contains(self, other: Self) -> bool { (self.0 & other.0) != 0 }
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
// Config globale
// ---------------------------------------------------------------------------

/// Niveau maximum d'affichage.
/// 0   = aucun log.
/// 10  = info normale (niveau <= 10).
/// 100 = tout afficher.
static LOG_LEVEL: AtomicU8  = AtomicU8::new(0);
static LOG_ZONES: AtomicU32 = AtomicU32::new(0);

/// Configure le niveau et les zones.
///
/// ```rust
/// use deep_log::LogZone;
///
/// deep_log::set(10,  LogZone::BASIC | LogZone::RENDER); // info normale
/// deep_log::set(100, LogZone::ALL);                     // tout voir
/// deep_log::set(0,   LogZone::ALL);                     // aucun log
/// ```
pub fn set(level: u8, zones: LogZone) {
    LOG_LEVEL.store(level, Ordering::Relaxed);
    LOG_ZONES.store(zones.0, Ordering::Relaxed);
}

/// Tout afficher. Équivalent à `set(100, LogZone::ALL)`.
pub fn set_all() { set(100, LogZone::ALL); }

/// Aucun log. Équivalent à `set(0, LogZone::ALL)`.
pub fn set_none() { set(0, LogZone::ALL); }

pub fn level() -> u8      { LOG_LEVEL.load(Ordering::Relaxed) }
pub fn zones() -> LogZone { LogZone(LOG_ZONES.load(Ordering::Relaxed)) }

/// Retourne `true` si ce message doit être affiché.
/// Condition : `zone ∈ active_zones` ET `level <= log_level`
#[inline]
pub fn should_log(zone: LogZone, level: u8) -> bool {
    let active = LogZone(LOG_ZONES.load(Ordering::Relaxed));
    let max    = LOG_LEVEL.load(Ordering::Relaxed);
    max > 0 && active.contains(zone) && level <= max
}

#[inline]
pub fn print(zone: LogZone, level: u8, msg: &str) {
    eprintln!("[{:?}|{}] {}", zone, level, msg);
}

// ---------------------------------------------------------------------------
// Macro
// ---------------------------------------------------------------------------

/// Log à deux axes : zone + niveau.
/// S'affiche si `zone ∈ zones actives` ET `niveau <= log_level`.
///
/// ```rust
/// use deep_log::{LogZone, dlog};
/// deep_log::set(10, LogZone::ALL);
/// dlog!(LogZone::BASIC, 10, "affiché");
/// dlog!(LogZone::BASIC, 50, "pas affiché — 50 > 10");
/// ```
#[macro_export]
macro_rules! dlog {
    ($zone:expr, $level:expr, $($arg:tt)*) => {
        if $crate::should_log($zone, $level) {
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
        assert!( should_log(LogZone::BASIC,  1));   // 1  <= 10 ✓
        assert!( should_log(LogZone::BASIC, 10));   // 10 <= 10 ✓
        assert!(!should_log(LogZone::BASIC, 11));   // 11 > 10  ✗
        assert!(!should_log(LogZone::BASIC, 50));   // 50 > 10  ✗
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
        assert!(!should_log(LogZone::RENDER, 10)); // zone inactive
        assert!(!should_log(LogZone::CHUNK,  10)); // zone inactive
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
}
