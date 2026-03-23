use deep_log::{LogZone, dlog};

fn main() {
    // Tout voir
    deep_log::set(0, LogZone::ALL);
    dlog!(LogZone::BASIC,  10, "=== deep_log demo ===");

    // Niveau 10, toutes zones
    dlog!(LogZone::BASIC,   10, "GPU : RTX 2080 Ti");
    dlog!(LogZone::RENDER,  10, "Surface : 1280x720");
    dlog!(LogZone::MATRIX,  50, "NDC vertex 0 : [0.029, 0.071, 0.980]");
    dlog!(LogZone::CHUNK,  100, "Face 42 émise");

    println!("---");

    // Seulement MATRIX, niveau >= 50
    deep_log::set(50, LogZone::MATRIX);
    dlog!(LogZone::BASIC,  10, "invisible (zone inactive)");
    dlog!(LogZone::MATRIX, 10, "invisible (niveau trop bas)");
    dlog!(LogZone::MATRIX, 50, "visible  — NDC validés ✓");
    dlog!(LogZone::RENDER, 50, "invisible (zone inactive)");

    println!("---");

    // Silencieux
    deep_log::set_none();
    dlog!(LogZone::BASIC, 0, "invisible — logs désactivés");
    println!("(aucun log ci-dessus — normal)");
}
