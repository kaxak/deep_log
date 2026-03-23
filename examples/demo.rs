use deep_log::{LogZone, dlog};

fn main() {
    // set(10, ...) → affiche les logs de niveau <= 10
    deep_log::set(10, LogZone::ALL);
    dlog!(LogZone::BASIC,  10, "=== deep_log demo ===");
    dlog!(LogZone::BASIC,  10, "GPU : RTX 2080 Ti");
    dlog!(LogZone::RENDER, 10, "Surface : 1280x720");
    dlog!(LogZone::MATRIX, 50, "NDC vertex — pas affiché (50 > 10)");
    dlog!(LogZone::CHUNK, 100, "Face émise — pas affiché (100 > 10)");

    println!("---");

    // set(100, ...) → tout afficher
    deep_log::set(100, LogZone::ALL);
    dlog!(LogZone::BASIC,   10, "info normale");
    dlog!(LogZone::MATRIX,  50, "matrices");
    dlog!(LogZone::CHUNK,  100, "ultra verbeux");

    println!("---");

    // set(100, zone partielle) → tout de la zone, rien des autres
    deep_log::set(100, LogZone::MATRIX);
    dlog!(LogZone::BASIC,  10, "invisible — zone inactive");
    dlog!(LogZone::MATRIX, 50, "visible   — MATRIX active");

    println!("---");

    // set(0, ...) → aucun log
    deep_log::set(0, LogZone::ALL);
    dlog!(LogZone::BASIC, 10, "invisible — level 0 = aucun log");
    println!("(aucun log ci-dessus — normal)");
}
