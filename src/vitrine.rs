//! La vitrine : la simulation de l'étape 2 pilotée par des nombres seulement, pour une page web.
//!
//! Ce module est compilé deux fois : en natif (les tests appellent ses fonctions comme n'importe quelle fonction
//! Rust) et en WebAssembly (`wasm32-unknown-unknown`, module sans importation). Chaque fonction `mycelium_*` est
//! exportée sous son nom (`#[unsafe(no_mangle)] pub extern "C"`) et ne prend et ne rend que des nombres : `u32`
//! devient `i32` côté WebAssembly, `f64` reste `f64`. Il n'y a ni pointeur ni mémoire à manipuler côté JavaScript.
//!
//! # Le module WebAssembly
//!
//! `docs/mycelium.wasm` est commité. Il se construit, depuis la racine du dépôt, par :
//!
//! ```text
//! cargo rustc --release --lib --target wasm32-unknown-unknown --crate-type cdylib
//! ```
//!
//! puis `target/wasm32-unknown-unknown/release/mycelium_lab.wasm` se copie en `docs/mycelium.wasm`. Le test
//! `tests/etape_4_artefact.rs` fait tourner ce fichier avec Node et compare à ceux du code natif son empreinte et ce
//! que lisent ses accesseurs : si le code change sans que le module soit reconstruit, il échoue.
//!
//! # L'état global
//!
//! La simulation en cours est un `static` protégé par un [`std::sync::Mutex`] : la cible WebAssembly n'a pas de fil
//! d'exécution, et en natif le verrou sérialise les appels des tests. [`mycelium_demarrer`] la construit sur le
//! scénario `scenarios/couplage.txt`, embarqué dans le module par `include_str!`, avec la graine demandée.
//!
//! # Avant tout démarrage
//!
//! Tant que [`mycelium_demarrer`] n'a pas été appelé, les fonctions qui rendent un entier rendent 0 et celles qui
//! rendent un flottant rendent `NaN` ; [`mycelium_avancer`] ne fait rien. Ce contrat n'est pas testé : l'état
//! global est déjà posé par les autres tests du processus.
//!
//! # Hors de portée
//!
//! [`mycelium_segment`] rend `NaN` pour un segment ou une coordonnée hors de portée, [`mycelium_substrat`] pour
//! une case hors de la boîte, et [`mycelium_empreinte_mot`] rend 0 pour un mot d'indice 8 ou plus.

use std::sync::{Mutex, MutexGuard};

use crate::simulation::{Simulation, lire_scenario};

/// Le scénario que fait tourner la vitrine, embarqué dans le module.
const SCENARIO: &str = include_str!("../scenarios/couplage.txt");

/// La simulation en cours, ou `None` tant que [`mycelium_demarrer`] n'a pas été appelé.
static SIMULATION: Mutex<Option<Simulation>> = Mutex::new(None);

/// Prend le verrou de l'état global ; un appel qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, Option<Simulation>> {
    SIMULATION.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// (Re)construit la simulation du scénario embarqué avec cette graine ; le temps vaut 0.
///
/// Remplace entièrement l'état précédent : rappelée après des pas, elle repart de zéro.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_demarrer(graine: u32) {
    let (reglages, _duree) = lire_scenario(SCENARIO).expect("le scénario embarqué est valide");
    let simulation = Simulation::nouvelle(reglages, u64::from(graine)).expect("les réglages sont valides");
    *verrou() = Some(simulation);
}

/// Avance la simulation de `n` pas de temps ; sans effet si `n` vaut 0 ou si rien n'est démarré.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_avancer(n: u32) {
    if let Some(simulation) = verrou().as_mut() {
        simulation.avancer_n(n);
    }
}

/// Le nombre de pas joués depuis le démarrage ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_temps() -> u32 {
    verrou().as_ref().map_or(0, Simulation::temps)
}

/// La largeur de la boîte, en cases ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_largeur() -> u32 {
    verrou().as_ref().map_or(0, |simulation| simulation.substrat().largeur() as u32)
}

/// La hauteur de la boîte, en cases ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_hauteur() -> u32 {
    verrou().as_ref().map_or(0, |simulation| simulation.substrat().hauteur() as u32)
}

/// Le nombre de segments laissés par la colonie ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_nb_segments() -> u32 {
    verrou().as_ref().map_or(0, |simulation| simulation.segments().len() as u32)
}

/// Une coordonnée du segment `i` : `k` vaut 0, 1, 2 ou 3 pour `x0`, `y0`, `x1` ou `y1`.
///
/// Rend `NaN` si `i >= mycelium_nb_segments()` ou si `k > 3`, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_segment(i: u32, k: u32) -> f64 {
    if k > 3 {
        return f64::NAN;
    }
    let verrou = verrou();
    let Some(segment) = verrou.as_ref().and_then(|simulation| simulation.segments().get(i as usize)) else {
        return f64::NAN;
    };
    [segment.x0, segment.y0, segment.x1, segment.y1][k as usize]
}

/// La quantité de substrat dans la case `(x, y)`.
///
/// Rend `NaN` si la case est hors de la boîte, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_substrat(x: u32, y: u32) -> f64 {
    let verrou = verrou();
    let Some(simulation) = verrou.as_ref() else {
        return f64::NAN;
    };
    let substrat = simulation.substrat();
    let (x, y) = (x as usize, y as usize);
    if x >= substrat.largeur() || y >= substrat.hauteur() {
        return f64::NAN;
    }
    substrat.valeur(x, y)
}

/// La biomasse de la colonie (la somme des longueurs de ses segments) ; `NaN` avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_biomasse() -> f64 {
    verrou().as_ref().map_or(f64::NAN, Simulation::biomasse)
}

/// Le nombre de pointes (apex vivants) ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_pointes() -> u32 {
    verrou().as_ref().map_or(0, |simulation| simulation.pointes() as u32)
}

/// Le nombre cumulé d'anastomoses ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_anastomoses() -> u32 {
    verrou().as_ref().map_or(0, |simulation| simulation.anastomoses() as u32)
}

/// Le `i`-ième mot (0 à 7) de 32 bits, en gros-boutiste, du SHA-256 de l'écriture canonique de l'état.
///
/// Les huit mots, chacun écrit sur 8 chiffres hexadécimaux minuscules et mis bout à bout dans l'ordre de 0 à 7,
/// redonnent [`Simulation::empreinte`](crate::simulation::Simulation::empreinte) (64 caractères). Rend 0 si
/// `i >= 8` et avant tout démarrage.
///
/// Chaque appel recalcule l'empreinte de l'état : la page ne la lit pas à chaque image.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_empreinte_mot(i: u32) -> u32 {
    if i >= 8 {
        return 0;
    }
    let verrou = verrou();
    let Some(simulation) = verrou.as_ref() else {
        return 0;
    };
    let empreinte = simulation.empreinte();
    let debut = i as usize * 8;
    u32::from_str_radix(&empreinte[debut..debut + 8], 16).expect("un mot d'empreinte est de l'hexadécimal")
}

// ---------------------------------------------------------------------------------------------------------------
// Les quatre fonctions de l'étape 8, en place mais vides : sans `no_mangle`, donc sans être exportées par le module.
// Elles ne servent qu'à ce que les tests de l'étape 8 compilent avant la vague de code ; ils y échouent.
// ---------------------------------------------------------------------------------------------------------------

/// Vide : ne choisit rien et rend 0.
pub fn mycelium_choisir_scenario(_numero: u32) -> u32 {
    0
}

/// Vide : rend 0.
pub fn mycelium_duree() -> u32 {
    0
}

/// Vide : rend 0.
pub fn mycelium_nb_obstacles() -> u32 {
    0
}

/// Vide : rend `NaN`.
pub fn mycelium_obstacle(_i: u32, _k: u32) -> f64 {
    f64::NAN
}
