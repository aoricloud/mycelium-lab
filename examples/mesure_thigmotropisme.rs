//! Mesure du thigmotropisme sur le scénario `scenarios/mur.txt` : ce que l'angle critique change à la colonie.
//!
//! Lit le scénario (une spore à gauche, un mur devant elle), rejoue 40 graines (1 à 40) pour chaque angle critique de la
//! liste `ANGLES`, pendant la durée du scénario, et écrit une ligne par angle : la biomasse moyenne ; la biomasse des segments
//! qui finissent au-delà du mur (à droite de sa dernière colonne), par colonie et en part de la biomasse ; la part de la
//! biomasse collée au mur (segments qui finissent à moins de 1,5 case de son rectangle) ; le nombre de colonies qui ont au
//! moins un segment au-delà ; le nombre moyen de ramifications. L'angle lu dans le scénario n'est pas utilisé : l'exemple
//! pose le sien.
//!
//! `cargo run --release --example mesure_thigmotropisme`. Le calcul n'emploie que `+`, `-`, `*`, `/`, `max` et `sqrt`.

use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use mycelium_lab::tropisme::Tropismes;

/// Les angles critiques essayés, en degrés.
const ANGLES: [f64; 5] = [0.0, 15.0, 35.0, 60.0, 90.0];
/// Les graines rejouées : de 1 à `GRAINES`.
const GRAINES: u64 = 40;

fn main() {
    let chemin = std::env::args().nth(1).unwrap_or_else(|| "scenarios/mur.txt".to_string());
    let texte = std::fs::read_to_string(&chemin).expect("lecture du scénario");
    let (reglages, milieu, _, duree) = lire_scenario_tropismes(&texte).expect("scénario valide");
    let mur = milieu.obstacles[0].clone();
    let au_dela = (mur.x1 + 1) as f64;

    for angle in ANGLES {
        let (mut biomasse, mut biomasse_au_dela, mut colle) = (0.0, 0.0, 0.0);
        let mut colonies_au_dela = 0;
        let mut ramifications = 0u64;
        for graine in 1..=GRAINES {
            let tropismes = Tropismes { thigmotropisme: angle, ..Tropismes::default() };
            let mut simulation =
                Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), tropismes, graine)
                    .expect("simulation valide");
            simulation.avancer_n(duree);
            ramifications += simulation.ramifications();
            let mut au_dela_ici = 0.0;
            for segment in simulation.segments() {
                let longueur = segment.longueur();
                biomasse += longueur;
                if segment.x1 >= au_dela {
                    au_dela_ici += longueur;
                }
                let dx = (mur.x0 as f64 - segment.x1).max(0.0).max(segment.x1 - (mur.x1 as f64 + 1.0));
                let dy = (mur.y0 as f64 - segment.y1).max(0.0).max(segment.y1 - (mur.y1 as f64 + 1.0));
                if (dx * dx + dy * dy).sqrt() <= 1.5 {
                    colle += longueur;
                }
            }
            biomasse_au_dela += au_dela_ici;
            if au_dela_ici > 0.0 {
                colonies_au_dela += 1;
            }
        }
        let graines = GRAINES as f64;
        println!(
            "angle {angle:>4} : biomasse {:>7.1}, au-delà du mur {:>6.1} par colonie ({:>4.1} %), collée {:>4.1} %, \
             {colonies_au_dela} colonies sur {GRAINES} au-delà, ramifications {:>6.1}",
            biomasse / graines,
            biomasse_au_dela / graines,
            100.0 * biomasse_au_dela / biomasse,
            100.0 * colle / biomasse,
            ramifications as f64 / graines
        );
    }
}
