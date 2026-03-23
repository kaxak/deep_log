use deep_log::{LogZone, dlog};

fn main() {
    // Console : info normale sur RENDER
    deep_log::set(10, LogZone::RENDER);

    // Fichier : tout BASIC dans logs/
    deep_log::log_to_file(100, LogZone::BASIC, "logs/");

    // Console seulement (RENDER)
    dlog!(LogZone::RENDER, 10, "Surface : 1280x720");          // console ✓
    dlog!(LogZone::RENDER, 50, "Stats verbose");               // pas affiché (50 > 10)

    // Fichier seulement (BASIC)
    dlog!(LogZone::BASIC,  10, "GPU : RTX 2080 Ti");           // fichier ✓
    dlog!(LogZone::BASIC,  50, "Matrices validées");           // fichier ✓
    dlog!(LogZone::BASIC, 100, "Ultra verbeux");               // fichier ✓

    // Ni l'un ni l'autre (MATRIX pas configuré)
    dlog!(LogZone::MATRIX, 10, "invisible partout");

    println!("Vérifier logs/BASIC_*.log");
}
