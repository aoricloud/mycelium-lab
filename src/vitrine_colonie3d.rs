//! La colonie en volume pilotée par des nombres seulement, pour une page web : la deuxième et la troisième marche de
//! la 3D.
//!
//! Ce module suit [`crate::vitrine_cube`] : il est compilé en natif (les tests appellent ses fonctions comme
//! n'importe quelle fonction Rust) et en WebAssembly (`wasm32-unknown-unknown`, module sans importation). Chaque
//! fonction `mycelium_colonie_*` est exportée sous son nom (`#[unsafe(no_mangle)] pub extern "C"`) et ne prend et ne
//! rend que des nombres : `u32` devient `i32` côté WebAssembly, `f64` reste `f64`. Il n'y a ni pointeur ni mémoire à
//! manipuler côté JavaScript.
//!
//! # Le module WebAssembly
//!
//! C'est le même `docs/mycelium.wasm` que celui des pages 2D et du cube ; il se construit comme le dit
//! [`crate::vitrine_cube`].
//!
//! # L'état global
//!
//! La colonie en cours est un `static` protégé par un [`std::sync::Mutex`] : la cible WebAssembly n'a pas de fil
//! d'exécution, et en natif le verrou sérialise les appels des tests. [`mycelium_colonie_demarrer`] la construit sur
//! la scène choisie, au temps 0, avec la graine donnée. Le verrou garde avec elle les soudures relevées depuis le
//! démarrage, et le numéro de la scène que construira le prochain démarrage.
//!
//! # Les neuf scènes
//!
//! Les neuf fichiers de `scenarios/` que montre la page sont embarqués dans le module par `include_str!`, et
//! [`mycelium_colonie_choisir`] en désigne un par son numéro :
//!
//! | numéro | fichier                            | l'étape qui l'a apportée               |
//! |--------|------------------------------------|----------------------------------------|
//! | 0      | `scenarios/colonie-cube.txt`       | 20, la colonie                         |
//! | 1      | `scenarios/colonie-goutte.txt`     | 20, la colonie                         |
//! | 2      | `scenarios/colonie-chimio.txt`     | 22, le chimiotropisme                  |
//! | 3      | `scenarios/colonie-mur.txt`        | 23, les obstacles et le thigmotropisme |
//! | 4      | `scenarios/colonie-anastomose.txt` | 24, l'anastomose                       |
//! | 5      | `scenarios/colonie-boule.txt`      | 27, les formes : une boule             |
//! | 6      | `scenarios/colonie-tore.txt`       | 27, les formes : un anneau             |
//! | 7      | `scenarios/colonie-coquille.txt`   | 27, les formes : une brique creuse     |
//! | 8      | `scenarios/colonie-labyrinthe.txt` | 27, les formes : un labyrinthe         |
//!
//! Ces numéros sont fixes : la page et les tests les emploient. Avant tout choix, la scène est la 0. Choisir n'agit
//! pas sur la colonie en cours : le choix ne vaut que pour le **prochain** [`mycelium_colonie_demarrer`], et il tient
//! pour tous les démarrages suivants jusqu'à un autre choix.
//!
//! Chaque scène est développée par [`developper_3d`] (sa ligne `moule`, s'il y en a une, devient des lignes
//! `obstacle` ; un texte sans `moule` reste tel quel), lue par [`lire_scenario_colonie3d`] et construite par
//! [`Colonie3d::nouvelle`] avec la graine donnée (en `u64`) : c'est la même colonie, octet pour octet, que celle des
//! tests natifs de même scène et de même graine. La graine 1 est celle des empreintes de référence.
//!
//! # Les lectures de la page
//!
//! [`mycelium_colonie_temps`] et [`mycelium_colonie_duree`] donnent l'horloge et la durée de la colonie en cours ;
//! [`mycelium_colonie_dimension`] ses trois dimensions ; [`mycelium_colonie_nb_segments`],
//! [`mycelium_colonie_segment`] et [`mycelium_colonie_segment_temps`] ses filaments ; [`mycelium_colonie_pointes`],
//! [`mycelium_colonie_ramifications`] et [`mycelium_colonie_anastomoses`] ses comptes ;
//! [`mycelium_colonie_nourriture`], [`mycelium_colonie_nourriture_maximum`] et [`mycelium_colonie_nourriture_masse`]
//! son substrat ; [`mycelium_colonie_nb_depots`] et [`mycelium_colonie_depot`] les nourritures de son scénario ;
//! [`mycelium_colonie_nb_obstacles`] et [`mycelium_colonie_obstacle`] ses obstacles ; [`mycelium_colonie_soudure`]
//! le lieu de chaque anastomose ; [`mycelium_colonie_part_remplie`] la part de la cavité que la colonie a atteinte
//! ([`part_remplie`]) ; [`mycelium_colonie_empreinte_mot`] l'empreinte de l'état, mot par mot. Elles ne font que
//! lire : aucune ne change la colonie.
//!
//! # Les soudures
//!
//! Une **soudure** est le point où une pointe s'est anastomosée : l'arrivée du segment qu'elle a laissé pendant le pas
//! de son anastomose (étapes 9 et 10 de [`Colonie3d::avancer`]). La colonie ne garde que leur nombre ; la vitrine en
//! relève le lieu en avançant pas par pas ([`Colonie3d::avancer_n`] est la même chose que `n` appels à
//! [`Colonie3d::avancer`]). Après chaque pas, chaque segment laissé pendant ce pas, dans l'ordre des segments, dont
//! l'apex est devenu inactif, donne une soudure à son arrivée `(x1, y1, z1)`. La règle est exacte : un apex laisse au
//! plus un segment par pas ; il n'en laisse aucun quand il s'arrête contre un obstacle (étape 8) ; après son segment,
//! seule l'anastomose (étape 10) peut le rendre inactif, et une pointe née pendant le pas n'agit qu'au suivant. Il y a
//! donc toujours autant de soudures que [`Colonie3d::nb_anastomoses`], dans l'ordre où elles sont arrivées (par pas,
//! puis par identifiant d'apex).
//!
//! # Avant tout démarrage
//!
//! Tant que [`mycelium_colonie_demarrer`] n'a pas été appelé, les fonctions qui rendent un entier rendent 0 et celles
//! qui rendent un flottant rendent `NaN` ; [`mycelium_colonie_avancer`] ne fait rien.
//!
//! # Hors de portée
//!
//! [`mycelium_colonie_segment`] rend `NaN` pour un segment qui n'existe pas ou un `k > 5`,
//! [`mycelium_colonie_segment_temps`] rend 0 pour un segment qui n'existe pas (un segment réel naît au pas 1 au
//! plus tôt), [`mycelium_colonie_nourriture`] rend `NaN` hors du volume, [`mycelium_colonie_dimension`] rend 0 pour
//! un `k` autre que 0, 1 ou 2, [`mycelium_colonie_depot`] rend `NaN` pour un dépôt qui n'existe pas ou un `k > 3`,
//! [`mycelium_colonie_obstacle`] rend `NaN` pour un obstacle qui n'existe pas ou un `k > 5`,
//! [`mycelium_colonie_soudure`] rend `NaN` pour une soudure qui n'existe pas ou un `k > 2`, et
//! [`mycelium_colonie_empreinte_mot`] rend 0 pour un mot d'indice 8 ou plus. [`mycelium_colonie_choisir`] rend 0,
//! sans rien changer, pour un numéro qui n'est pas de 0 à 8.
//!
//! # Aucune panique
//!
//! Une panique dans une fonction `extern "C"` fait avorter le processus (le binaire de test, ou le module). Aucune
//! fonction `mycelium_colonie_*` ne doit donc en provoquer : chaque coordonnée est contrôlée avant l'appel à
//! [`Substrat3d::valeur`](crate::substrat3d::Substrat3d::valeur), qui panique hors du volume, chaque indice passe par
//! `get`, et le développement et la lecture d'une scène embarquée ne rendent jamais d'erreur (ces textes sont fixes et
//! valides). S'ils en rendaient une, [`mycelium_colonie_demarrer`] ne changerait rien.

use std::sync::{Mutex, MutexGuard};

use crate::colonie3d::{Colonie3d, lire_scenario_colonie3d};
use crate::moule3d::{developper_3d, part_remplie};

/// Les neuf scènes de la page, dans l'ordre de leurs numéros (0 à 8), embarquées dans le module.
///
/// L'ordre est celui du tableau de la doc du module : le numéro `n` désigne `SCENARIOS[n]`.
const SCENARIOS: [&str; 9] = [
    include_str!("../scenarios/colonie-cube.txt"),
    include_str!("../scenarios/colonie-goutte.txt"),
    include_str!("../scenarios/colonie-chimio.txt"),
    include_str!("../scenarios/colonie-mur.txt"),
    include_str!("../scenarios/colonie-anastomose.txt"),
    include_str!("../scenarios/colonie-boule.txt"),
    include_str!("../scenarios/colonie-tore.txt"),
    include_str!("../scenarios/colonie-coquille.txt"),
    include_str!("../scenarios/colonie-labyrinthe.txt"),
];

/// Ce que la page retient entre deux appels : la colonie en cours, ses soudures et la scène du prochain démarrage.
struct VitrineColonie3d {
    /// La colonie en cours, ou `None` tant que [`mycelium_colonie_demarrer`] n'a pas été appelé.
    colonie: Option<Colonie3d>,
    /// Le lieu `(x, y, z)` de chaque anastomose de la colonie en cours, dans l'ordre où elles sont arrivées (voir
    /// « Les soudures » dans la doc du module) ; vide au démarrage.
    soudures: Vec<(f64, f64, f64)>,
    /// L'indice, dans [`SCENARIOS`], de la scène que construira le prochain [`mycelium_colonie_demarrer`] ; 0 au
    /// départ.
    choix: usize,
}

/// L'état global de la vitrine de la colonie en volume.
static VITRINE_COLONIE3D: Mutex<VitrineColonie3d> =
    Mutex::new(VitrineColonie3d { colonie: None, soudures: Vec::new(), choix: 0 });

/// Prend le verrou de l'état global ; un appel qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, VitrineColonie3d> {
    VITRINE_COLONIE3D.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// Fixe la scène que construira le **prochain** [`mycelium_colonie_demarrer`] : de 0 à 8, voir la doc du module.
///
/// Rend 1 si `numero` vaut de 0 à 8. Sinon rend 0 et ne change rien : le choix d'avant reste. N'agit pas sur la
/// colonie en cours, qui n'est pas redémarrée ni ne change de scène.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_choisir(numero: u32) -> u32 {
    if (numero as usize) < SCENARIOS.len() {
        verrou().choix = numero as usize;
        1
    } else {
        0
    }
}

/// (Re)construit la colonie de la scène choisie avec la graine `graine` (prise en `u64`) ; le temps vaut 0.
///
/// Remplace entièrement l'état précédent, soudures comprises : rappelée après des pas, elle repart de zéro. La scène
/// est celle que [`mycelium_colonie_choisir`] a fixée en dernier (la 0 si rien n'a été choisi) ; elle reste choisie
/// pour les démarrages suivants. Son texte est développé par [`developper_3d`], puis lu. Une scène embarquée invalide
/// ne peut pas arriver : si elle arrivait, rien ne changerait.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_demarrer(graine: u32) {
    let mut vitrine = verrou();
    let Ok(texte) = developper_3d(SCENARIOS[vitrine.choix]) else {
        return;
    };
    let Ok(scenario) = lire_scenario_colonie3d(&texte) else {
        return;
    };
    let Ok(colonie) = Colonie3d::nouvelle(&scenario, u64::from(graine)) else {
        return;
    };
    vitrine.colonie = Some(colonie);
    vitrine.soudures.clear();
}

/// Avance la colonie de `n` pas de temps, un pas à la fois, en relevant les soudures de chaque pas ; sans effet si
/// `n` vaut 0 ou si rien n'est démarré.
///
/// La colonie obtenue est celle de [`Colonie3d::avancer_n`] avec le même `n` : relever les soudures ne fait que lire.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_avancer(n: u32) {
    let mut vitrine = verrou();
    let VitrineColonie3d { colonie, soudures, .. } = &mut *vitrine;
    let Some(colonie) = colonie.as_mut() else {
        return;
    };
    for _ in 0..n {
        let avant = colonie.segments().len();
        colonie.avancer();
        relever_soudures(colonie, avant, soudures);
    }
}

/// Ajoute à `soudures` celles du pas qui vient d'être joué : chaque segment laissé depuis le segment `avant` dont
/// l'apex est devenu inactif, à son arrivée, dans l'ordre des segments (voir « Les soudures » dans la doc du module).
fn relever_soudures(colonie: &Colonie3d, avant: usize, soudures: &mut Vec<(f64, f64, f64)>) {
    let apex = colonie.apex();
    for segment in colonie.segments().get(avant..).unwrap_or(&[]) {
        if apex.get(segment.apex as usize).is_some_and(|a| !a.actif) {
            soudures.push((segment.x1, segment.y1, segment.z1));
        }
    }
}

/// Le nombre de pas joués depuis le démarrage ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_temps() -> u32 {
    verrou().colonie.as_ref().map_or(0, Colonie3d::temps)
}

/// La durée de la scène de la colonie **en cours** (et non de celle choisie pour le suivant) ; 0 avant tout
/// démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_duree() -> u32 {
    verrou().colonie.as_ref().map_or(0, Colonie3d::duree)
}

/// Une dimension du cube, en cases : `k` vaut 0, 1 ou 2 pour la largeur, la hauteur ou la profondeur.
///
/// Rend 0 si `k > 2` et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_dimension(k: u32) -> u32 {
    let verrou = verrou();
    let Some(colonie) = verrou.colonie.as_ref() else {
        return 0;
    };
    let scenario = colonie.scenario();
    match k {
        0 => scenario.largeur,
        1 => scenario.hauteur,
        2 => scenario.profondeur,
        _ => 0,
    }
}

/// Le nombre de segments (de filaments) laissés par la colonie ; 0 avant tout démarrage.
///
/// Les segments ne font que s'ajouter, dans l'ordre où ils sont laissés : la page lit seulement les nouveaux.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_nb_segments() -> u32 {
    verrou().colonie.as_ref().map_or(0, |colonie| colonie.segments().len() as u32)
}

/// Une coordonnée du segment `i` : `k` vaut 0 à 5 pour `x0`, `y0`, `z0`, `x1`, `y1` ou `z1`, en cases.
///
/// Rend `NaN` si `i >= mycelium_colonie_nb_segments()` ou si `k > 5`, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_segment(i: u32, k: u32) -> f64 {
    let verrou = verrou();
    let Some(segment) = verrou.colonie.as_ref().and_then(|colonie| colonie.segments().get(i as usize)) else {
        return f64::NAN;
    };
    match k {
        0 => segment.x0,
        1 => segment.y0,
        2 => segment.z0,
        3 => segment.x1,
        4 => segment.y1,
        5 => segment.z1,
        _ => f64::NAN,
    }
}

/// Le pas de naissance du segment `i` : le numéro (à partir de 1) du pas pendant lequel il a été laissé.
///
/// Rend 0 si `i >= mycelium_colonie_nb_segments()` et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_segment_temps(i: u32) -> u32 {
    let verrou = verrou();
    verrou.colonie.as_ref().and_then(|colonie| colonie.segments().get(i as usize)).map_or(0, |segment| segment.temps)
}

/// Le nombre de pointes (d'apex actifs) de la colonie ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_pointes() -> u32 {
    verrou().colonie.as_ref().map_or(0, |colonie| colonie.pointes() as u32)
}

/// Le nombre cumulé de ramifications, ramené à `u32::MAX` s'il le dépasse ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_ramifications() -> u32 {
    verrou().colonie.as_ref().map_or(0, |colonie| u32::try_from(colonie.nb_ramifications()).unwrap_or(u32::MAX))
}

/// Le nombre cumulé d'anastomoses ([`Colonie3d::nb_anastomoses`]), ramené à `u32::MAX` s'il le dépasse ; 0 avant
/// tout démarrage, et toujours 0 pour une scène sans ligne `anastomose`.
///
/// C'est aussi le nombre de soudures que lit [`mycelium_colonie_soudure`].
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_anastomoses() -> u32 {
    verrou().colonie.as_ref().map_or(0, |colonie| u32::try_from(colonie.nb_anastomoses()).unwrap_or(u32::MAX))
}

/// Une coordonnée de la soudure `i`, la `i`-ième anastomose depuis le démarrage (de 0) : `k` vaut 0, 1 ou 2 pour
/// `x`, `y` ou `z`, en cases. C'est l'arrivée du segment que la pointe a laissé pendant le pas de son anastomose.
///
/// Rend `NaN` si `i >= mycelium_colonie_anastomoses()` ou si `k > 2`, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_soudure(i: u32, k: u32) -> f64 {
    let verrou = verrou();
    if verrou.colonie.is_none() {
        return f64::NAN;
    }
    let Some(&(x, y, z)) = verrou.soudures.get(i as usize) else {
        return f64::NAN;
    };
    match k {
        0 => x,
        1 => y,
        2 => z,
        _ => f64::NAN,
    }
}

/// La quantité de substrat (de nourriture) dans la case `(x, y, z)`.
///
/// Rend `NaN` si la case est hors du volume, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_nourriture(x: u32, y: u32, z: u32) -> f64 {
    let verrou = verrou();
    let Some(colonie) = verrou.colonie.as_ref() else {
        return f64::NAN;
    };
    let substrat = colonie.substrat();
    let (x, y, z) = (x as usize, y as usize, z as usize);
    // Le contrôle porte sur chaque coordonnée : un `x` trop grand ne glisse pas sur la profondeur.
    if x >= substrat.largeur() || y >= substrat.hauteur() || z >= substrat.profondeur() {
        return f64::NAN;
    }
    substrat.valeur(x, y, z)
}

/// La plus grande quantité de substrat de toutes les cases du volume, ou `0.0` pour un volume vide.
///
/// Les cases sont parcourues `z` lent, puis `y`, puis `x` rapide, le maximum partant de `0.0` et grandissant par
/// [`f64::max`] : une valeur négative ne le fait pas descendre. Rend `NaN` avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_nourriture_maximum() -> f64 {
    let verrou = verrou();
    let Some(colonie) = verrou.colonie.as_ref() else {
        return f64::NAN;
    };
    let substrat = colonie.substrat();
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

/// La masse totale du substrat, somme des valeurs de toutes ses cases ; `NaN` avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_nourriture_masse() -> f64 {
    verrou().colonie.as_ref().map_or(f64::NAN, |colonie| colonie.substrat().masse_totale())
}

/// Le nombre de lignes `nourriture` du scénario de la colonie en cours ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_nb_depots() -> u32 {
    verrou().colonie.as_ref().map_or(0, |colonie| colonie.scenario().depots.len() as u32)
}

/// Un champ du dépôt `i`, dans l'ordre du texte du scénario : `k` vaut 0 à 3 pour `x`, `y`, `z` (la case) ou la
/// quantité déposée au départ.
///
/// Rend `NaN` si `i >= mycelium_colonie_nb_depots()` ou si `k > 3`, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_depot(i: u32, k: u32) -> f64 {
    let verrou = verrou();
    let Some(depot) = verrou.colonie.as_ref().and_then(|colonie| colonie.scenario().depots.get(i as usize)) else {
        return f64::NAN;
    };
    match k {
        0 => f64::from(depot.x),
        1 => f64::from(depot.y),
        2 => f64::from(depot.z),
        3 => depot.quantite,
        _ => f64::NAN,
    }
}

/// Le nombre de lignes `obstacle` du scénario de la colonie en cours ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_nb_obstacles() -> u32 {
    verrou().colonie.as_ref().map_or(0, |colonie| colonie.scenario().obstacles.len() as u32)
}

/// Un champ de l'obstacle `i`, dans l'ordre du texte du scénario : `k` vaut 0 à 5 pour `x0`, `y0`, `z0`, `x1`, `y1`
/// ou `z1`, les cases de ses deux coins, bornes comprises (le pavé bloqué va de la case `x0` à la case `x1`, et de
/// même en `y` et `z`).
///
/// Rend `NaN` si `i >= mycelium_colonie_nb_obstacles()` ou si `k > 5`, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_obstacle(i: u32, k: u32) -> f64 {
    let verrou = verrou();
    let Some(obstacle) = verrou.colonie.as_ref().and_then(|colonie| colonie.scenario().obstacles.get(i as usize))
    else {
        return f64::NAN;
    };
    match k {
        0 => f64::from(obstacle.x0),
        1 => f64::from(obstacle.y0),
        2 => f64::from(obstacle.z0),
        3 => f64::from(obstacle.x1),
        4 => f64::from(obstacle.y1),
        5 => f64::from(obstacle.z1),
        _ => f64::NAN,
    }
}

/// La part de la cavité que la colonie en cours a atteinte : [`part_remplie`] (les cases non bloquées où finit au
/// moins un segment, sur les cases non bloquées), de 0 à 1 ; sans moule, la cavité est tout le cube. `NaN` avant tout
/// démarrage.
///
/// Chaque appel recompte les cases : la page la lit une fois par pas.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_part_remplie() -> f64 {
    verrou().colonie.as_ref().map_or(f64::NAN, part_remplie)
}

/// Le `i`-ième mot (0 à 7) de 32 bits, en gros-boutiste, du SHA-256 de l'écriture canonique de la colonie.
///
/// Les huit mots, chacun écrit sur 8 chiffres hexadécimaux minuscules et mis bout à bout dans l'ordre de 0 à 7,
/// redonnent [`Colonie3d::empreinte`] (64 caractères). Rend 0 si `i >= 8` et avant tout démarrage.
///
/// Chaque appel recalcule l'empreinte de l'état : la page ne la lit qu'à la fin de la scène.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_colonie_empreinte_mot(i: u32) -> u32 {
    if i >= 8 {
        return 0;
    }
    let verrou = verrou();
    let Some(colonie) = verrou.colonie.as_ref() else {
        return 0;
    };
    let empreinte = colonie.empreinte();
    let debut = i as usize * 8;
    empreinte.get(debut..debut + 8).and_then(|mot| u32::from_str_radix(mot, 16).ok()).unwrap_or(0)
}
