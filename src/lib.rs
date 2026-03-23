//! # deep_log
//!
//! Système de log à deux axes orthogonaux :
//!
//! - **Niveau** : verbosité de 0 à 100.
//!   Un message s'affiche si `son_niveau >= log_level`.
//!   `log_level = 0` → tout s'affiche. `log_level = 100` → seulement le plus critique.
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
//!     // Tout afficher, toutes les zones
//!     deep_log::set(0, LogZone::ALL);
//!
//!     // Info normale sur la zone BASIC
//!     dlog!(LogZone::BASIC, 10, "Démarrage — version {}", env!("CARGO_PKG_VERSION"));
//!
//!     // Matrices (niveau 50, zone MATRIX)
//!     dlog!(LogZone::MATRIX, 50, "NDC vertex 0 : {:?}", [0.1, 0.2, 0.9]);
//!
//!     // Log ultra-verbeux (niveau 100, zone CHUNK)
//!     dlog!(LogZone::CHUNK, 100, "Face émise : idx={}", 42);
//! }
//! ```
//!
//! ## Niveaux conseillés
//!
//! | Niveau | Usage                              |
//! |--------|------------------------------------|
//! | 1      | Erreurs fatales                    |
//! | 10     | Info normale (démarrage, GPU...)   |
//! | 20     | Stats de rendu                     |
//! | 50     | Matrices, NDC, caméra              |
//! | 70     | Meshing, chunks                    |
//! | 100    | Tout (frame-by-frame verbose)      |
//!
//! ## Zones prédéfinies
//!
//! Les zones sont des bitflags — combinez-les avec `|` :
//!
//! ```rust
//! use deep_log::LogZone;
//! deep_log::set(10, LogZone::BASIC | LogZone::MATRIX);
//! ```
//!
//! Vous pouvez aussi définir vos propres zones avec `LogZone::custom(1 << 16)`.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

// ---------------------------------------------------------------------------
// LogZone — bitflags
// ---------------------------------------------------------------------------

/// Bitflag identifiant le système émetteur d'un message de log.
///
/// Combinez les zones avec `|` :
/// ```rust
/// use deep_log::LogZone;
/// let zones = LogZone::BASIC | LogZone::MATRIX | LogZone::SHADER;
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LogZone(pub u32);

impl LogZone {
    // Zones prédéfinies — les 8 premiers bits
    pub const NONE:    Self = Self(0);
    pub const BASIC:   Self = Self(1 << 0);  // init, démarrage, configuration
    pub const RENDER:  Self = Self(1 << 1);  // pipeline, draw calls, frames
    pub const MATRIX:  Self = Self(1 << 2);  // matrices, NDC, caméra
    pub const SHADER:  Self = Self(1 << 3);  // uniforms, bindings, shaders
    pub const CHUNK:   Self = Self(1 << 4);  // meshing, voxels, génération
    pub const PHYSICS: Self = Self(1 << 5);  // collisions, raycasts
    pub const AUDIO:   Self = Self(1 << 6);  // audio, sons
    pub const NET:     Self = Self(1 << 7);  // réseau, sérialisation
    /// Toutes les zones actives.
    pub const ALL:     Self = Self(u32::MAX);

    /// Crée une zone personnalisée.
    /// Utilisez les bits 8→31 pour éviter les collisions avec les zones prédéfinies.
    ///
    /// ```rust
    /// use deep_log::LogZone;
    /// const MY_SYSTEM: LogZone = LogZone::custom(1 << 8);
    /// ```
    pub const fn custom(bit: u32) -> Self {
        Self(bit)
    }

    /// Retourne true si cette zone contient au moins un bit de `other`.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
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
        if self.0 == 0 { return write!(f, "NONE"); }
        if self.0 == u32::MAX { return write!(f, "ALL"); }

        let mut names: Vec<&str> = Vec::new();
        if self.contains(Self::BASIC)   { names.push("BASIC");   }
        if self.contains(Self::RENDER)  { names.push("RENDER");  }
        if self.contains(Self::MATRIX)  { names.push("MATRIX");  }
        if self.contains(Self::SHADER)  { names.push("SHADER");  }
        if self.contains(Self::CHUNK)   { names.push("CHUNK");   }
        if self.contains(Self::PHYSICS) { names.push("PHYSICS"); }
        if self.contains(Self::AUDIO)   { names.push("AUDIO");   }
        if self.contains(Self::NET)     { names.push("NET");      }

        // Bits custom (8→31)
        let custom = self.0 & !0xFF;
        if custom != 0 { names.push("CUSTOM"); }

        write!(f, "{}", names.join("|"))
    }
}

impl std::fmt::Display for LogZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// Config globale — thread-safe, zéro allocation
// ---------------------------------------------------------------------------

/// Niveau minimum d'affichage (0 = tout, 100 = critique seulement).
static LOG_LEVEL: AtomicU8  = AtomicU8::new(0);

/// Bitset des zones actives.
static LOG_ZONES: AtomicU32 = AtomicU32::new(u32::MAX);

/// Configure le niveau et les zones actifs.
///
/// ```rust
/// use deep_log::LogZone;
///
/// // Tout voir
/// deep_log::set(0, LogZone::ALL);
///
/// // Seulement MATRIX et RENDER, niveau >= 10
/// deep_log::set(10, LogZone::MATRIX | LogZone::RENDER);
///
/// // Silencieux
/// deep_log::set(0, LogZone::NONE);
/// ```
pub fn set(level: u8, zones: LogZone) {
    LOG_LEVEL.store(level, Ordering::Relaxed);
    LOG_ZONES.store(zones.0, Ordering::Relaxed);
}

/// Tout afficher, toutes les zones. Équivalent à `set(0, LogZone::ALL)`.
pub fn set_all() {
    set(0, LogZone::ALL);
}

/// Désactive tous les logs. Équivalent à `set(0, LogZone::NONE)`.
pub fn set_none() {
    set(0, LogZone::NONE);
}

/// Retourne le niveau actif.
pub fn level() -> u8 {
    LOG_LEVEL.load(Ordering::Relaxed)
}

/// Retourne les zones actives.
pub fn zones() -> LogZone {
    LogZone(LOG_ZONES.load(Ordering::Relaxed))
}

/// Retourne `true` si ce message doit être affiché.
///
/// Condition : `zone ∈ active_zones` ET `level >= log_level`
#[inline]
pub fn should_log(zone: LogZone, level: u8) -> bool {
    let active = LogZone(LOG_ZONES.load(Ordering::Relaxed));
    let min    = LOG_LEVEL.load(Ordering::Relaxed);
    active.contains(zone) && level >= min
}

/// Affiche un message — appelé par la macro `dlog!`.
/// Format : `[ZONE|niveau] message`
#[inline]
pub fn print(zone: LogZone, level: u8, msg: &str) {
    eprintln!("[{:?}|{}] {}", zone, level, msg);
}

// ---------------------------------------------------------------------------
// Macro principale
// ---------------------------------------------------------------------------

/// Log à deux axes : zone + niveau.
///
/// Un message s'affiche si :
/// - `sa_zone ∈ zones actives` (configuré via `deep_log::set`)
/// - `son_niveau >= log_level` (configuré via `deep_log::set`)
///
/// # Exemples
///
/// ```rust
/// use deep_log::{LogZone, dlog};
///
/// deep_log::set(0, LogZone::ALL);
///
/// dlog!(LogZone::BASIC,  10, "GPU : {}", "RTX 4090");
/// dlog!(LogZone::MATRIX, 50, "NDC : [{:.3}, {:.3}, {:.3}]", 0.1, 0.2, 0.9);
/// dlog!(LogZone::CHUNK, 100, "Face {} émise", 42);
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
    fn test_zone_contains() {
        let zones = LogZone::BASIC | LogZone::MATRIX;
        assert!(zones.contains(LogZone::BASIC));
        assert!(zones.contains(LogZone::MATRIX));
        assert!(!zones.contains(LogZone::RENDER));
        assert!(!zones.contains(LogZone::CHUNK));
    }

    #[test]
    fn test_should_log_level() {
        set(50, LogZone::ALL);
        assert!(!should_log(LogZone::BASIC, 10));  // 10 < 50 → non
        assert!(should_log(LogZone::BASIC,  50));  // 50 >= 50 → oui
        assert!(should_log(LogZone::BASIC, 100));  // 100 >= 50 → oui
    }

    #[test]
    fn test_should_log_zone() {
        set(0, LogZone::BASIC | LogZone::MATRIX);
        assert!( should_log(LogZone::BASIC,  10)); // zone active
        assert!( should_log(LogZone::MATRIX, 10)); // zone active
        assert!(!should_log(LogZone::RENDER, 10)); // zone inactive
        assert!(!should_log(LogZone::CHUNK,  10)); // zone inactive
    }

    #[test]
    fn test_set_none() {
        set_none();
        assert!(!should_log(LogZone::BASIC, 0));
        assert!(!should_log(LogZone::ALL,   0));
    }

    #[test]
    fn test_set_all() {
        set_all();
        assert!(should_log(LogZone::BASIC,   0));
        assert!(should_log(LogZone::RENDER,  0));
        assert!(should_log(LogZone::MATRIX,  0));
        assert!(should_log(LogZone::PHYSICS, 0));
    }

    #[test]
    fn test_custom_zone() {
        const MY_AI: LogZone = LogZone::custom(1 << 8);
        set(0, MY_AI);
        assert!( should_log(MY_AI,          0));
        assert!(!should_log(LogZone::BASIC,  0));
    }

    #[test]
    fn test_zone_debug_display() {
        let z = LogZone::BASIC | LogZone::MATRIX;
        assert_eq!(format!("{:?}", z), "BASIC|MATRIX");
        assert_eq!(format!("{:?}", LogZone::NONE), "NONE");
        assert_eq!(format!("{:?}", LogZone::ALL),  "ALL");
    }
}
