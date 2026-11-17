//! Le cube de l'étape 18 piloté par des nombres seulement, pour une page web.
//!
//! Ce module est compilé deux fois : en natif (les tests appellent ses fonctions comme n'importe quelle fonction
//! Rust) et en WebAssembly (`wasm32-unknown-unknown`, module sans importation). Chaque fonction `mycelium_cube_*`
//! est exportée sous son nom (`#[unsafe(no_mangle)] pub extern "C"`) et ne prend et ne rend que des nombres : `u32`
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
//! `tests/etape_18_artefact.rs` fait tourner ce fichier avec Node et compare au code natif, scène par scène, son
//! temps, sa durée, ses dimensions, sa masse, son maximum, ses valeurs, ses cases bloquées et son empreinte : si le
//! code change sans que le module soit reconstruit, il échoue.
//!
//! # L'état global
//!
//! Le cube en cours est un `static` protégé par un [`std::sync::Mutex`] : la cible WebAssembly n'a pas de fil
//! d'exécution, et en natif le verrou sérialise les appels des tests. [`mycelium_cube_demarrer`] le construit sur la
//! scène choisie, au temps 0. Le verrou garde avec lui le numéro de la scène que construira le prochain démarrage.
//!
//! # Les deux scènes
//!
//! Les deux fichiers de `scenarios/` que montre la page sont embarqués dans le module par `include_str!`, et
//! [`mycelium_cube_choisir`] en désigne un par son numéro :
//!
//! | numéro | fichier                    |
//! |--------|----------------------------|
//! | 0      | `scenarios/cube-goutte.txt` |
//! | 1      | `scenarios/cube-mur.txt`    |
//!
//! Ces numéros sont fixes : la page et les tests les emploient. Avant tout choix, la scène est la 0. Choisir n'agit
//! pas sur le cube en cours : le choix ne vaut que pour le **prochain** [`mycelium_cube_demarrer`], et il tient pour
//! tous les démarrages suivants jusqu'à un autre choix.
//!
//! Chaque scène est lue par [`lire_scenario_cube`] et construite par [`Cube::nouveau`]. C'est le même cube, octet
//! pour octet, que celui des tests de l'étape 18 : l'empreinte de la fin de chaque scène est celle de
//! `references/cube-goutte.empreinte` et `references/cube-mur.empreinte`.
//!
//! # Les lectures de la page
//!
//! [`mycelium_cube_temps`] et [`mycelium_cube_duree`] donnent l'horloge et le pas de référence du cube en cours ;
//! [`mycelium_cube_dimension`] ses trois dimensions ; [`mycelium_cube_valeur`] la quantité de substrat d'une case,
//! [`mycelium_cube_bloquee`] si elle est bloquée, [`mycelium_cube_maximum`] la plus grande valeur du volume,
//! [`mycelium_cube_masse`] la masse totale et [`mycelium_cube_empreinte_mot`] l'empreinte de l'état, mot par mot.
//! Elles ne font que lire : aucune ne change le cube.
//!
//! # Avant tout démarrage
//!
//! Tant que [`mycelium_cube_demarrer`] n'a pas été appelé, les fonctions qui rendent un entier rendent 0 et celles
//! qui rendent un flottant rendent `NaN` ; [`mycelium_cube_avancer`] ne fait rien. Un module WebAssembly
//! fraîchement instancié n'a rien démarré : `tests/etape_18_artefact.rs` le vérifie sur lui. Les tests natifs d'un
//! même processus partagent l'état global ; le test qui le pose le fait sous son propre verrou.
//!
//! # Hors du volume
//!
//! [`mycelium_cube_valeur`] rend `NaN` et [`mycelium_cube_bloquee`] rend 0 pour une case hors du volume,
//! [`mycelium_cube_dimension`] rend 0 pour un `k` autre que 0, 1 ou 2, et [`mycelium_cube_empreinte_mot`] rend 0
//! pour un mot d'indice 8 ou plus. [`mycelium_cube_choisir`] rend 0, sans rien changer, pour un numéro qui n'est ni
//! 0 ni 1.
//!
//! # Aucune panique
//!
//! Une panique dans une fonction `extern "C"` fait avorter le processus (le binaire de test, ou le module). Aucune
//! fonction `mycelium_cube_*` ne doit donc en provoquer : chaque coordonnée est contrôlée avant l'appel à
//! [`Substrat3d::valeur`](crate::substrat3d::Substrat3d::valeur) ou à
//! [`Substrat3d::est_bloquee`](crate::substrat3d::Substrat3d::est_bloquee), qui paniquent hors du volume, et la
//! lecture d'une scène embarquée ne rend jamais d'erreur (ces textes sont fixes et valides).

use std::sync::{Mutex, MutexGuard};

use crate::cube::{Cube, lire_scenario_cube};

/// Les deux scènes de la page, dans l'ordre de leurs numéros (0 et 1), embarquées dans le module.
///
/// L'ordre est celui du tableau de la doc du module : le numéro `n` désigne `SCENARIOS[n]`.
const SCENARIOS: [&str; 2] = [include_str!("../scenarios/cube-goutte.txt"), include_str!("../scenarios/cube-mur.txt")];

/// Ce que la page retient entre deux appels : le cube en cours et la scène du prochain démarrage.
struct VitrineCube {
    /// Le cube en cours, ou `None` tant que [`mycelium_cube_demarrer`] n'a pas été appelé.
    cube: Option<Cube>,
    /// L'indice, dans [`SCENARIOS`], de la scène que construira le prochain [`mycelium_cube_demarrer`] ; 0 au
    /// départ.
    choix: usize,
}

/// L'état global de la vitrine du cube.
static VITRINE_CUBE: Mutex<VitrineCube> = Mutex::new(VitrineCube { cube: None, choix: 0 });

/// Prend le verrou de l'état global ; un appel qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, VitrineCube> {
    VITRINE_CUBE.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// Fixe la scène que construira le **prochain** [`mycelium_cube_demarrer`] : 0 ou 1, voir la doc du module.
///
/// Rend 1 si `numero` vaut 0 ou 1. Sinon rend 0 et ne change rien : le choix d'avant reste. N'agit pas sur le cube
/// en cours, qui n'est pas redémarré ni ne change de scène.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_choisir(numero: u32) -> u32 {
    if (numero as usize) < SCENARIOS.len() {
        verrou().choix = numero as usize;
        1
    } else {
        0
    }
}

/// (Re)construit le cube de la scène choisie ; le temps vaut 0.
///
/// Remplace entièrement l'état précédent : rappelée après des pas, elle repart de zéro. La scène est celle que
/// [`mycelium_cube_choisir`] a fixée en dernier (la 0 si rien n'a été choisi) ; elle reste choisie pour les
/// démarrages suivants. Une scène embarquée invalide ne peut pas arriver : les deux textes sont fixes et valides.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_demarrer() {
    let mut vitrine = verrou();
    let Ok(scenario) = lire_scenario_cube(SCENARIOS[vitrine.choix]) else {
        return;
    };
    let Ok(cube) = Cube::nouveau(&scenario) else {
        return;
    };
    vitrine.cube = Some(cube);
}

/// Avance le cube de `n` pas de temps ; sans effet si `n` vaut 0 ou si rien n'est démarré.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_avancer(n: u32) {
    if let Some(cube) = verrou().cube.as_mut() {
        cube.avancer(n);
    }
}

/// Le nombre de pas joués depuis le démarrage ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_temps() -> u32 {
    verrou().cube.as_ref().map_or(0, Cube::temps)
}

/// La durée de la scène du cube en cours, c'est-à-dire son pas de référence ; 0 avant tout démarrage.
///
/// C'est la scène du cube **en cours** et non celle qu'a choisie [`mycelium_cube_choisir`] pour le suivant : la page
/// affiche le pas du cube qu'elle montre.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_duree() -> u32 {
    verrou().cube.as_ref().map_or(0, Cube::duree)
}

/// Une dimension du cube, en cases : `k` vaut 0, 1 ou 2 pour la largeur, la hauteur ou la profondeur.
///
/// Rend 0 si `k > 2` et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_dimension(k: u32) -> u32 {
    let verrou = verrou();
    let Some(cube) = verrou.cube.as_ref() else {
        return 0;
    };
    let substrat = cube.substrat();
    match k {
        0 => substrat.largeur() as u32,
        1 => substrat.hauteur() as u32,
        2 => substrat.profondeur() as u32,
        _ => 0,
    }
}

/// La quantité de substrat dans la case `(x, y, z)`.
///
/// Rend `NaN` si la case est hors du volume, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_valeur(x: u32, y: u32, z: u32) -> f64 {
    let verrou = verrou();
    let Some(cube) = verrou.cube.as_ref() else {
        return f64::NAN;
    };
    let substrat = cube.substrat();
    let (x, y, z) = (x as usize, y as usize, z as usize);
    // Le contrôle porte sur chaque coordonnée : un `x` trop grand ne glisse pas sur la profondeur.
    if x >= substrat.largeur() || y >= substrat.hauteur() || z >= substrat.profondeur() {
        return f64::NAN;
    }
    substrat.valeur(x, y, z)
}

/// 1 si la case `(x, y, z)` est bloquée, sinon 0.
///
/// Rend 0 si la case est hors du volume, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_bloquee(x: u32, y: u32, z: u32) -> u32 {
    let verrou = verrou();
    let Some(cube) = verrou.cube.as_ref() else {
        return 0;
    };
    let substrat = cube.substrat();
    let (x, y, z) = (x as usize, y as usize, z as usize);
    if x >= substrat.largeur() || y >= substrat.hauteur() || z >= substrat.profondeur() {
        return 0;
    }
    u32::from(substrat.est_bloquee(x, y, z))
}

/// La plus grande valeur de toutes les cases du volume, ou `0.0` pour un volume vide.
///
/// Les cases sont parcourues `z` lent, puis `y`, puis `x` rapide, le maximum partant de `0.0` et grandissant par
/// [`f64::max`] : une valeur négative ne le fait pas descendre. Rend `NaN` avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_maximum() -> f64 {
    let verrou = verrou();
    let Some(cube) = verrou.cube.as_ref() else {
        return f64::NAN;
    };
    let substrat = cube.substrat();
    let mut maximum = 0.0f64;
    for z in 0..substrat.profondeur() {
        for y in 0..substrat.hauteur() {
            for x in 0..substrat.largeur() {
                maximum = maximum.max(substrat.valeur(x, y, z));
            }
        }
    }
    maximum
}

/// La masse totale du cube, somme des valeurs de toutes ses cases ; `NaN` avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_masse() -> f64 {
    verrou().cube.as_ref().map_or(f64::NAN, |cube| cube.substrat().masse_totale())
}

/// Le `i`-ième mot (0 à 7) de 32 bits, en gros-boutiste, du SHA-256 de l'écriture canonique du cube.
///
/// Les huit mots, chacun écrit sur 8 chiffres hexadécimaux minuscules et mis bout à bout dans l'ordre de 0 à 7,
/// redonnent [`Cube::empreinte`] (64 caractères). Rend 0 si `i >= 8` et avant tout démarrage.
///
/// Chaque appel recalcule l'empreinte de l'état : la page ne la lit pas à chaque image.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_cube_empreinte_mot(i: u32) -> u32 {
    if i >= 8 {
        return 0;
    }
    let verrou = verrou();
    let Some(cube) = verrou.cube.as_ref() else {
        return 0;
    };
    let empreinte = cube.empreinte();
    let debut = i as usize * 8;
    u32::from_str_radix(&empreinte[debut..debut + 8], 16).expect("un mot d'empreinte est de l'hexadécimal")
}
