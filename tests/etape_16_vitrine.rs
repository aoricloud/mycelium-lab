//! Tests d'acceptation de l'étape 16 (b) : les trois lectures de la vitrine, côté natif.
//!
//! `mycelium_segment_temps(i)`, `mycelium_segment_apex(i)` et `mycelium_pointe(i, k)` de `vitrine` sont ce que la page lit,
//! à travers le module WebAssembly, pour dessiner l'âge, le flux et les pointes. Ici on les appelle comme des fonctions Rust
//! et on les compare, bit à bit, à une `Simulation` construite **sans passer par `vitrine.rs`** (le texte du scénario est lu
//! sur le disque, jamais par `include_str!`) : `segments()` pour les deux premières, `etat_des_pointes()` filtré sur les
//! pointes vivantes pour la troisième. Le module WebAssembly lui-même est éprouvé dans `etape_16_artefact.rs`.
//!
//! L'état de la vitrine est global au processus et les tests d'un fichier tournent en parallèle : chaque test prend d'abord
//! le verrou `VERROU`, propre à ce fichier, et le garde jusqu'à la fin. **Le verrou remet le choix à 0 en se relâchant**,
//! même quand le test a paniqué.
//!
//! Ce que valent les trois fonctions avant tout `mycelium_demarrer` (`NaN`) n'est pas testé ici : l'état global est déjà posé
//! par les autres tests du processus. Le module fraîchement instancié, lui, n'a rien démarré : `etape_16_artefact.rs` le
//! vérifie sur lui.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_bits`, `is_nan` et `from` (la conversion exacte d'un `u32`).

use std::sync::{Mutex, MutexGuard};

use mycelium_lab::simulation::{EtatPointe, Simulation, lire_scenario_tropismes};
use mycelium_lab::vitrine::{
    mycelium_avancer, mycelium_choisir_scenario, mycelium_demarrer, mycelium_empreinte_mot, mycelium_nb_segments,
    mycelium_pointe, mycelium_pointes, mycelium_segment_apex, mycelium_segment_temps, mycelium_temps,
};

/// Un des six scénarios de la vitrine, tel que la spécification de l'étape 8 le fixe.
struct Scenario {
    /// Le numéro que prend `mycelium_choisir_scenario`.
    numero: u32,
    /// Le nom dans l'adresse de la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// Le pas de référence (`duree` du fichier).
    duree: u32,
}

/// Les six scénarios, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas de `vitrine.rs`.
const SCENARIOS: [Scenario; 6] = [
    Scenario { numero: 0, nom: "libre", fichier: "couplage.txt", duree: 80 },
    Scenario { numero: 1, nom: "terrain", fichier: "terrain.txt", duree: 80 },
    Scenario { numero: 2, nom: "tache", fichier: "chimiotropisme.txt", duree: 100 },
    Scenario { numero: 3, nom: "tache-sans", fichier: "chimiotropisme-sans.txt", duree: 100 },
    Scenario { numero: 4, nom: "mur", fichier: "mur.txt", duree: 120 },
    Scenario { numero: 5, nom: "mur-sans", fichier: "mur-sans.txt", duree: 120 },
];

/// Les graines de la comparaison : la plus simple et une qui ne tient ni sur 16 bits ni sur 31.
const GRAINES: [u32; 2] = [1, 0xDEAD_BEEF];

/// Les moments de la comparaison, en nombre de pas depuis le démarrage : le départ, un pas, un pas moyen,
/// le pas de référence et 40 pas de plus (la page ne s'arrête pas à `duree`).
fn moments(scenario: &Scenario) -> [u32; 5] {
    [0, 1, 37, scenario.duree, scenario.duree + 40]
}

/// Sérialise les tests de ce fichier : l'état de la vitrine est unique.
static VERROU: Mutex<()> = Mutex::new(());

/// Le verrou, tenu pendant tout un test. En se relâchant, même si le test a paniqué, il remet le choix à 0.
struct Garde {
    /// Le verrou lui-même : tenu pour sérialiser les tests, jamais lu.
    _verrou: MutexGuard<'static, ()>,
}

impl Drop for Garde {
    fn drop(&mut self) {
        mycelium_choisir_scenario(0);
    }
}

/// Prend le verrou ; un test qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> Garde {
    Garde { _verrou: VERROU.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner()) }
}

/// La simulation native en face : le scénario lu sur le disque, la graine, au temps 0.
fn simulation_native(scenario: &Scenario, graine: u32) -> Simulation {
    let chemin = format!("{}/scenarios/{}", env!("CARGO_MANIFEST_DIR"), scenario.fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{chemin} se lit : {erreur}"));
    let (reglages, milieu, tropismes, _duree) =
        lire_scenario_tropismes(&texte).expect("le scénario du disque est valide");
    Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages sont valides")
}

/// Choisit ce scénario, démarre la vitrine avec cette graine et rend la simulation native jumelle, au temps 0.
fn demarrer(scenario: &Scenario, graine: u32) -> Simulation {
    assert_eq!(mycelium_choisir_scenario(scenario.numero), 1, "le scénario {} est refusé", scenario.nom);
    mycelium_demarrer(graine);
    simulation_native(scenario, graine)
}

/// Avance la vitrine et sa jumelle de `n` pas.
fn avancer(native: &mut Simulation, n: u32) {
    mycelium_avancer(n);
    native.avancer_n(n);
}

/// Les pointes vivantes de la simulation native, dans l'ordre des identifiants.
fn vivantes(native: &Simulation) -> Vec<EtatPointe> {
    native.etat_des_pointes().filter(|pointe| pointe.actif).collect()
}

/// Les huit mots de l'empreinte de la vitrine, mis bout à bout.
fn empreinte_de_la_vitrine() -> String {
    (0..8).map(|i| format!("{:08x}", mycelium_empreinte_mot(i))).collect()
}

/// Vérifie que les trois lectures de la vitrine sont, dans son état courant, celles de `native` : chaque segment (pas de
/// naissance, hyphe), chaque pointe vivante (`x` puis `y`), bit à bit, et leurs nombres.
fn verifier_les_valeurs(native: &Simulation, contexte: &str) {
    let segments = native.segments();
    assert_eq!(mycelium_nb_segments() as usize, segments.len(), "{contexte} : nombre de segments");
    for (i, segment) in segments.iter().enumerate() {
        let temps = mycelium_segment_temps(i as u32);
        let apex = mycelium_segment_apex(i as u32);
        assert_eq!(temps.to_bits(), f64::from(segment.temps).to_bits(), "{contexte} : segment {i}, pas de naissance");
        assert_eq!(apex.to_bits(), f64::from(segment.apex).to_bits(), "{contexte} : segment {i}, hyphe");
    }
    let pointes = vivantes(native);
    assert_eq!(mycelium_pointes() as usize, pointes.len(), "{contexte} : nombre de pointes");
    for (j, pointe) in pointes.iter().enumerate() {
        let (x, y) = (mycelium_pointe(j as u32, 0), mycelium_pointe(j as u32, 1));
        assert_eq!(x.to_bits(), pointe.x.to_bits(), "{contexte} : pointe {j}, x");
        assert_eq!(y.to_bits(), pointe.y.to_bits(), "{contexte} : pointe {j}, y");
    }
}

/// Vérifie les bords : `NaN` pour un segment ou une pointe hors de portée (le premier indice trop grand, un très grand,
/// `u32::MAX`), pour `k` trop grand (2, un très grand, `u32::MAX`) et pour les deux à la fois ; mais pas pour le dernier
/// indice valide.
fn verifier_les_bords(contexte: &str) {
    let n = mycelium_nb_segments();
    let m = mycelium_pointes();
    assert!(m >= 1, "{contexte} : une colonie a toujours une pointe");
    for hors in [n, n + 1, 1 << 31, u32::MAX] {
        assert!(mycelium_segment_temps(hors).is_nan(), "{contexte} : segment_temps({hors}) avec {n} segments");
        assert!(mycelium_segment_apex(hors).is_nan(), "{contexte} : segment_apex({hors}) avec {n} segments");
    }
    if n > 0 {
        assert!(!mycelium_segment_temps(n - 1).is_nan(), "{contexte} : segment_temps du dernier segment");
        assert!(!mycelium_segment_apex(n - 1).is_nan(), "{contexte} : segment_apex du dernier segment");
    }
    for hors in [m, m + 1, 1 << 31, u32::MAX] {
        for k in [0, 1] {
            assert!(mycelium_pointe(hors, k).is_nan(), "{contexte} : pointe({hors}, {k}) avec {m} pointes");
        }
    }
    for i in [0, m - 1] {
        for k in [2, 3, 1 << 31, u32::MAX] {
            assert!(mycelium_pointe(i, k).is_nan(), "{contexte} : pointe({i}, {k})");
        }
        for k in [0, 1] {
            assert!(!mycelium_pointe(i, k).is_nan(), "{contexte} : pointe({i}, {k}) est valide");
        }
    }
    assert!(mycelium_pointe(u32::MAX, u32::MAX).is_nan(), "{contexte} : pointe(u32::MAX, u32::MAX)");
}

/// 1. Pour chacun des **six scénarios**, deux graines et cinq moments (0, 1, un pas moyen, le pas de référence, 40 de plus),
///    les trois lectures sont celles de `Simulation` : le pas de naissance et l'hyphe de **chaque** segment, et `x` puis `y` de
///    **chaque** pointe vivante, dans l'ordre des identifiants. Une pointe arrêtée (anastomosée ou contre un mur) n'est pas
///    comptée et ne décale pas les suivantes : le test en a vu, avant une pointe vivante, dans ces parcours.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn les_trois_lectures_sont_celles_de_la_simulation_pour_les_six_scenarios() {
    let _garde = verrou();
    let mut arretees_avant_une_vivante = 0usize;
    for scenario in &SCENARIOS {
        for graine in GRAINES {
            let mut native = demarrer(scenario, graine);
            let mut fait = 0;
            for moment in moments(scenario) {
                avancer(&mut native, moment - fait);
                fait = moment;
                let contexte = format!("{} graine {graine} pas {moment}", scenario.nom);
                assert_eq!(mycelium_temps(), moment, "{contexte}");
                verifier_les_valeurs(&native, &contexte);
                let etats: Vec<EtatPointe> = native.etat_des_pointes().collect();
                let derniere_vivante = etats.iter().rposition(|etat| etat.actif).expect("une pointe vivante");
                arretees_avant_une_vivante += etats[..derniere_vivante].iter().filter(|etat| !etat.actif).count();
            }
        }
    }
    assert!(arretees_avant_une_vivante > 0, "aucune pointe arrêtée avant une vivante : le test serait creux");
}

/// 2. Les bords, à chaque moment de chaque scénario : `NaN` hors de portée sur chaque indice et sur `k` (au premier indice
///    hors de portée, bien au-delà, `u32::MAX`), jamais pour le dernier indice valide. Au temps 0 il n'y a aucun segment : même
///    `i = 0` rend `NaN` pour les deux premières, alors que la pointe 0 existe (sur la spore).
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn hors_de_portee_et_aux_bords_les_lectures_rendent_nan() {
    let _garde = verrou();
    for scenario in &SCENARIOS {
        let mut native = demarrer(scenario, 1);
        let mut fait = 0;
        for moment in moments(scenario) {
            avancer(&mut native, moment - fait);
            fait = moment;
            verifier_les_bords(&format!("{} pas {moment}", scenario.nom));
        }
    }
    let _ = demarrer(&SCENARIOS[0], 1);
    assert_eq!(mycelium_nb_segments(), 0);
    assert!(mycelium_segment_temps(0).is_nan() && mycelium_segment_apex(0).is_nan());
    assert_eq!(mycelium_pointes(), 1);
    assert!(!mycelium_pointe(0, 0).is_nan() && !mycelium_pointe(0, 1).is_nan());
}

/// 3. Lire ne change rien : la vitrine lue à chaque pas (les trois lectures, tous les indices, les bords) a, tous les
///    10 pas, l'empreinte de sa jumelle native et, à la fin, celle d'une simulation native jamais lue.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn lire_les_pointes_et_les_segments_ne_change_pas_l_empreinte() {
    let _garde = verrou();
    for scenario in [&SCENARIOS[0], &SCENARIOS[4]] {
        let mut jumelle = demarrer(scenario, 0xDEAD_BEEF);
        for pas in 1..=scenario.duree {
            avancer(&mut jumelle, 1);
            let contexte = format!("{} pas {pas}", scenario.nom);
            verifier_les_valeurs(&jumelle, &contexte);
            verifier_les_bords(&contexte);
            if pas % 10 == 0 {
                assert_eq!(
                    empreinte_de_la_vitrine(),
                    jumelle.empreinte(),
                    "{contexte} : empreinte lue après les lectures"
                );
            }
        }
        // La jumelle de la jumelle : jamais lue, mais avancée comme la vitrine.
        let mut jamais_lue = simulation_native(scenario, 0xDEAD_BEEF);
        jamais_lue.avancer_n(scenario.duree);
        assert_eq!(empreinte_de_la_vitrine(), jamais_lue.empreinte(), "{} : l'empreinte finale", scenario.nom);
    }
}
