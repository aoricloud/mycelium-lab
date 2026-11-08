//! Mesure du chimiotropisme sur le scénario de démonstration : quelle part de la colonie se trouve près de la tache.
//!
//! Lit `scenarios/chimiotropisme-sans.txt` (la spore en `(20, 60)`, une tache de nutriments en `(42, 40)`), rejoue
//! 40 graines (1 à 40) pour chaque force de la liste `FORCES`, pendant la durée du scénario, et écrit une ligne par
//! force : la biomasse moyenne, la part de la biomasse dont le segment finit à moins de 12 cases du centre de la tache,
//! et le nombre de colonies qui ont au moins un segment dans ce disque. La force lue dans le scénario n'est pas
//! utilisée : l'exemple pose la sienne.
//!
//! `cargo run --release --example mesure_chimiotropisme`. Le calcul n'emploie que `+`, `-`, `*`, `/` et `sqrt`.

use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use mycelium_lab::tropisme::Tropismes;

/// Les forces essayées, en degrés.
const FORCES: [f64; 6] = [0.0, 2.0, 5.0, 10.0, 20.0, 45.0];
/// Les graines rejouées : de 1 à `GRAINES`.
const GRAINES: u64 = 40;
/// Le rayon du disque autour du centre de la tache, en cases.
const RAYON_DE_MESURE: f64 = 12.0;

fn main() {
    let texte = std::fs::read_to_string("scenarios/chimiotropisme-sans.txt").expect("lecture du scénario");
    let (reglages, milieu, _, duree) = lire_scenario_tropismes(&texte).expect("scénario valide");
    let tache = &milieu.sources[0];
    let (cx, cy) = (tache.x as f64, tache.y as f64);

    for force in FORCES {
        let mut biomasse_totale = 0.0;
        let mut biomasse_proche = 0.0;
        let mut colonies_qui_touchent = 0;
        for graine in 1..=GRAINES {
            let tropismes = Tropismes { chimiotropisme: force, ..Tropismes::default() };
            let mut simulation =
                Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), tropismes, graine)
                    .expect("simulation valide");
            simulation.avancer_n(duree);
            let mut proche = 0.0;
            for segment in simulation.segments() {
                let (dx, dy) = (segment.x1 - segment.x0, segment.y1 - segment.y0);
                let longueur = (dx * dx + dy * dy).sqrt();
                biomasse_totale += longueur;
                let (ex, ey) = (segment.x1 - cx, segment.y1 - cy);
                if ex * ex + ey * ey <= RAYON_DE_MESURE * RAYON_DE_MESURE {
                    proche += longueur;
                }
            }
            biomasse_proche += proche;
            if proche > 0.0 {
                colonies_qui_touchent += 1;
            }
        }
        let graines = GRAINES as f64;
        println!(
            "force {force:>4} : biomasse moyenne {:>7.1}, près de la tache {:>5.1} % ({} colonies sur {GRAINES} la touchent)",
            biomasse_totale / graines,
            100.0 * biomasse_proche / biomasse_totale,
            colonies_qui_touchent
        );
    }
}
