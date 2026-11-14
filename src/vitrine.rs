//! La vitrine : la simulation couplée pilotée par des nombres seulement, pour une page web.
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
//! scénario choisi, avec la graine demandée. Le verrou garde avec elle la `duree` de son scénario et le numéro du
//! scénario que construira le prochain démarrage.
//!
//! # Les six scénarios
//!
//! Les six fichiers de `scenarios/` que montre la page sont embarqués dans le module par `include_str!`, et
//! [`mycelium_choisir_scenario`] en désigne un par son numéro :
//!
//! | numéro | fichier                             | nom dans l'adresse de la page |
//! |--------|-------------------------------------|-------------------------------|
//! | 0      | `scenarios/couplage.txt`            | `libre`                       |
//! | 1      | `scenarios/terrain.txt`             | `terrain`                     |
//! | 2      | `scenarios/chimiotropisme.txt`      | `tache`                       |
//! | 3      | `scenarios/chimiotropisme-sans.txt` | `tache-sans`                  |
//! | 4      | `scenarios/mur.txt`                 | `mur`                         |
//! | 5      | `scenarios/mur-sans.txt`            | `mur-sans`                    |
//!
//! Ces numéros sont fixes : la page et les tests les emploient. Avant tout choix, le scénario est le 0, celui que la
//! vitrine a toujours fait tourner : tant qu'on ne choisit rien, tout se comporte comme avant les six scénarios.
//! Choisir n'agit pas sur la simulation en cours : le choix ne vaut que pour le **prochain**
//! [`mycelium_demarrer`], et il tient pour tous les démarrages suivants jusqu'à un autre choix.
//!
//! Chaque scénario est lu par [`lire_scenario_tropismes`] et construit par
//! [`Simulation::nouvelle_avec_tropismes`]. Pour le scénario 0, qui n'a ni source, ni obstacle, ni tropisme, l'état
//! est le même, octet pour octet, que celui de l'ancienne lecture (`lire_scenario` puis `Simulation::nouvelle`) :
//! c'est ce qui garde vraie l'empreinte de référence `references/couplage-graine-1.empreinte`.
//!
//! # La densité
//!
//! [`mycelium_densite`] et [`mycelium_densite_max`] donnent la carte de densité d'hyphe de l'étape M0
//! ([`carte_de_densite`], sur les segments de la simulation en cours) : la case `(x, y)` vaut la longueur d'hyphe qui
//! y passe, et le maximum de la carte met à l'échelle les couleurs de la page. La carte se calcule **une seule fois
//! après chaque changement** de la simulation (un [`mycelium_demarrer`], un [`mycelium_avancer`] de 1 pas ou plus), à la
//! première lecture qui le suit, puis elle est gardée jusqu'au changement suivant : lire les `largeur × hauteur`
//! cases d'une image ne la recalcule pas, et une page qui ne lit jamais la densité n'en paie pas le calcul. Un
//! [`mycelium_avancer`] de 0 pas ne change rien et garde la carte. Une carte vide (le temps 0, aucun segment) vaut 0.0
//! partout et son maximum vaut `0.0` : ce n'est pas `NaN`, qui est réservé à « pas de simulation » et à « hors de la
//! boîte ».
//!
//! # Les lectures de la page
//!
//! Trois exportations donnent à la page de quoi dessiner l'âge, le flux et les pointes : [`mycelium_segment_temps`] (le
//! pas de naissance d'un segment), [`mycelium_segment_apex`] (l'hyphe qui l'a laissé) et [`mycelium_pointe`] (la position
//! d'une pointe vivante). Elles ne font que lire : aucune ne change la simulation ni l'écriture canonique.
//!
//! Les deux premières rendent des entiers (le pas à partir de 1, l'identifiant d'apex à partir de 0) **en `f64`** : 0 est
//! un identifiant valide, aucun entier ne reste libre pour « hors de portée », et la règle du module est déjà « hors de
//! portée : `NaN` » ; un entier de moins de 2^53 est exact en `f64`. Hors de portée (un segment ou une pointe qui
//! n'existe pas, `k` trop grand) et avant tout démarrage, les trois rendent `NaN`, comme [`mycelium_segment`].
//!
//! [`mycelium_pointe`] parcourt les apex à chaque appel et ne garde rien en mémoire : lire toutes les pointes d'une image
//! coûte le produit du nombre de pointes par celui des apex. Mesuré à l'étape 16 sur les scénarios de la page, c'est au
//! plus de l'ordre de 0,3 ms par image sous Node, moins que la lecture des segments : pas de cache (voir `MODELE.md`).
//!
//! # Avant tout démarrage
//!
//! Tant que [`mycelium_demarrer`] n'a pas été appelé, les fonctions qui rendent un entier rendent 0 et celles qui
//! rendent un flottant rendent `NaN` ; [`mycelium_avancer`] ne fait rien. Cela vaut aussi pour [`mycelium_duree`],
//! [`mycelium_nb_obstacles`], [`mycelium_obstacle`], [`mycelium_densite`] et [`mycelium_densite_max`]. Les tests natifs
//! ne le vérifient pas : l'état global est déjà posé par les autres tests du processus. Un module WebAssembly
//! fraîchement instancié, lui, n'a rien démarré : les tests d'artefact des étapes 8 et 11 le vérifient sur lui.
//!
//! # Hors de portée
//!
//! [`mycelium_segment`] rend `NaN` pour un segment ou une coordonnée hors de portée, [`mycelium_obstacle`] pour un
//! obstacle ou une coordonnée hors de portée, [`mycelium_substrat`] et [`mycelium_densite`] pour une case hors de la
//! boîte, et [`mycelium_empreinte_mot`] rend 0 pour un mot d'indice 8 ou plus. [`mycelium_choisir_scenario`] rend 0,
//! sans rien changer, pour un numéro qui n'est pas de 0 à 5.
//!
//! # Aucune panique
//!
//! Une panique dans une fonction `extern "C"` fait avorter le processus (le binaire de test, ou le module). Aucune
//! fonction `mycelium_*` ne doit donc en provoquer : chaque indice est contrôlé avant d'être employé (la carte de
//! densité se lit par `get`), et [`carte_de_densite`] ne panique pas : elle exige des segments dans
//! `[0, largeur] × [0, hauteur]`, et la simulation n'en laisse que dans `[0, largeur) × [0, hauteur)`.

use std::sync::{Mutex, MutexGuard};

use crate::piece::{Carte, carte_de_densite};
use crate::simulation::{Simulation, lire_scenario_tropismes};

/// Les six scénarios de la vitrine, dans l'ordre de leurs numéros (0 à 5), embarqués dans le module.
///
/// L'ordre est celui du tableau de la doc du module : le numéro `n` désigne `SCENARIOS[n]`.
const SCENARIOS: [&str; 6] = [
    include_str!("../scenarios/couplage.txt"),
    include_str!("../scenarios/terrain.txt"),
    include_str!("../scenarios/chimiotropisme.txt"),
    include_str!("../scenarios/chimiotropisme-sans.txt"),
    include_str!("../scenarios/mur.txt"),
    include_str!("../scenarios/mur-sans.txt"),
];

/// La carte de densité d'hyphe de la simulation en cours, et son maximum.
struct Densite {
    carte: Carte,
    /// La plus grande case de la carte ; `0.0` pour une carte vide (aucun segment).
    maximum: f64,
}

/// Ce que la vitrine retient entre deux appels : la simulation en cours, sa durée, le scénario du prochain démarrage,
/// et la carte de densité de la simulation en cours.
struct Vitrine {
    /// La simulation en cours, ou `None` tant que [`mycelium_demarrer`] n'a pas été appelé.
    simulation: Option<Simulation>,
    /// La `duree` du scénario de la simulation en cours (et non du scénario choisi pour la suivante) ; 0 avant tout
    /// démarrage.
    duree: u32,
    /// L'indice, dans [`SCENARIOS`], du scénario que construira le prochain [`mycelium_demarrer`] ; 0 au départ.
    choix: usize,
    /// La carte de densité de la simulation en cours, ou `None` tant qu'elle n'est pas calculée depuis le dernier
    /// changement de la simulation : [`mycelium_demarrer`] et un [`mycelium_avancer`] de 1 pas ou plus la remettent à
    /// `None`, [`densite_a_jour`] la calcule.
    densite: Option<Densite>,
}

/// L'état global de la vitrine.
static VITRINE: Mutex<Vitrine> = Mutex::new(Vitrine { simulation: None, duree: 0, choix: 0, densite: None });

/// Prend le verrou de l'état global ; un appel qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, Vitrine> {
    VITRINE.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// Fixe le scénario que construira le **prochain** [`mycelium_demarrer`] : 0 à 5, voir la doc du module.
///
/// Rend 1 si `numero` est de 0 à 5. Sinon rend 0 et ne change rien : le choix d'avant reste. N'agit pas sur la
/// simulation en cours, qui n'est pas redémarrée ni ne change de scénario.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_choisir_scenario(numero: u32) -> u32 {
    if (numero as usize) < SCENARIOS.len() {
        verrou().choix = numero as usize;
        1
    } else {
        0
    }
}

/// (Re)construit la simulation du scénario choisi avec cette graine ; le temps vaut 0.
///
/// Remplace entièrement l'état précédent : rappelée après des pas, elle repart de zéro. Le scénario est celui que
/// [`mycelium_choisir_scenario`] a fixé en dernier (le 0 si rien n'a été choisi) ; il reste choisi pour les démarrages
/// suivants.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_demarrer(graine: u32) {
    let mut vitrine = verrou();
    let (reglages, milieu, tropismes, duree) =
        lire_scenario_tropismes(SCENARIOS[vitrine.choix]).expect("le scénario embarqué est valide");
    let simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages, le milieu et les tropismes sont valides");
    vitrine.simulation = Some(simulation);
    vitrine.duree = duree;
    vitrine.densite = None;
}

/// Avance la simulation de `n` pas de temps ; sans effet si `n` vaut 0 ou si rien n'est démarré.
///
/// Un `n` de 1 ou plus rend périmée la carte de densité gardée : la prochaine lecture la recalcule.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_avancer(n: u32) {
    let mut vitrine = verrou();
    let Vitrine { simulation, densite, .. } = &mut *vitrine;
    if let Some(simulation) = simulation.as_mut() {
        simulation.avancer_n(n);
        if n > 0 {
            *densite = None;
        }
    }
}

/// La `duree` du scénario de la simulation en cours, c'est-à-dire son pas de référence ; 0 avant tout démarrage.
///
/// C'est le scénario de la simulation **en cours** et non celui qu'a choisi [`mycelium_choisir_scenario`] pour la
/// suivante : la page affiche le pas de la colonie qu'elle montre.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_duree() -> u32 {
    verrou().duree
}

/// Le nombre de pas joués depuis le démarrage ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_temps() -> u32 {
    verrou().simulation.as_ref().map_or(0, Simulation::temps)
}

/// La largeur de la boîte, en cases ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_largeur() -> u32 {
    verrou().simulation.as_ref().map_or(0, |simulation| simulation.substrat().largeur() as u32)
}

/// La hauteur de la boîte, en cases ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_hauteur() -> u32 {
    verrou().simulation.as_ref().map_or(0, |simulation| simulation.substrat().hauteur() as u32)
}

/// Le nombre de segments laissés par la colonie ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_nb_segments() -> u32 {
    verrou().simulation.as_ref().map_or(0, |simulation| simulation.segments().len() as u32)
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
    let Some(segment) = verrou.simulation.as_ref().and_then(|simulation| simulation.segments().get(i as usize)) else {
        return f64::NAN;
    };
    [segment.x0, segment.y0, segment.x1, segment.y1][k as usize]
}

/// Le pas de naissance du segment `i` : le numéro (à partir de 1) du pas de temps pendant lequel il a été laissé.
///
/// Rend `NaN` si `i >= mycelium_nb_segments()` et avant tout démarrage. Un entier, rendu en `f64` : voir « Les lectures de
/// la page » dans la doc du module.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_segment_temps(i: u32) -> f64 {
    let verrou = verrou();
    let Some(segment) = verrou.simulation.as_ref().and_then(|simulation| simulation.segments().get(i as usize)) else {
        return f64::NAN;
    };
    f64::from(segment.temps)
}

/// L'identifiant de l'apex, c'est-à-dire de l'hyphe, qui a laissé le segment `i` (0 pour la première).
///
/// Rend `NaN` si `i >= mycelium_nb_segments()` et avant tout démarrage. Un entier, rendu en `f64` : voir « Les lectures de
/// la page » dans la doc du module.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_segment_apex(i: u32) -> f64 {
    let verrou = verrou();
    let Some(segment) = verrou.simulation.as_ref().and_then(|simulation| simulation.segments().get(i as usize)) else {
        return f64::NAN;
    };
    f64::from(segment.apex)
}

/// La quantité de substrat dans la case `(x, y)`.
///
/// Rend `NaN` si la case est hors de la boîte, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_substrat(x: u32, y: u32) -> f64 {
    let verrou = verrou();
    let Some(simulation) = verrou.simulation.as_ref() else {
        return f64::NAN;
    };
    let substrat = simulation.substrat();
    let (x, y) = (x as usize, y as usize);
    if x >= substrat.largeur() || y >= substrat.hauteur() {
        return f64::NAN;
    }
    substrat.valeur(x, y)
}

/// La carte de densité de la simulation en cours : celle qui est gardée, ou, à la première lecture qui suit un
/// changement de la simulation, celle qu'on calcule maintenant puis garde. `None` avant tout démarrage.
fn densite_a_jour(vitrine: &mut Vitrine) -> Option<&Densite> {
    if vitrine.densite.is_none() {
        let simulation = vitrine.simulation.as_ref()?;
        let substrat = simulation.substrat();
        let carte = carte_de_densite(substrat.largeur(), substrat.hauteur(), simulation.segments());
        let mut maximum = 0.0f64;
        for &valeur in carte.valeurs() {
            maximum = maximum.max(valeur);
        }
        vitrine.densite = Some(Densite { carte, maximum });
    }
    vitrine.densite.as_ref()
}

/// La densité d'hyphe de la case `(x, y)` : la somme des longueurs des morceaux de segment qui y passent, comme la
/// carte de densité de l'étape M0 ([`carte_de_densite`]). Une case sans hyphe, ou bloquée par un obstacle, vaut `0.0`.
///
/// Rend `NaN` si la case est hors de la boîte, et avant tout démarrage. Voir « La densité » dans la doc du module.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_densite(x: u32, y: u32) -> f64 {
    let mut vitrine = verrou();
    let Some(densite) = densite_a_jour(&mut vitrine) else {
        return f64::NAN;
    };
    let (largeur, hauteur) = (densite.carte.largeur(), densite.carte.hauteur());
    let (x, y) = (x as usize, y as usize);
    // Le contrôle porte sur chaque coordonnée : un `x` trop grand ne glisse pas sur la ligne suivante.
    if x >= largeur || y >= hauteur {
        return f64::NAN;
    }
    densite.carte.valeurs().get(y * largeur + x).copied().unwrap_or(f64::NAN)
}

/// La plus grande densité de la carte, celle qui met à l'échelle les couleurs de la page.
///
/// Rend `0.0` pour une carte vide (aucun segment : le temps 0) et `NaN` avant tout démarrage. Voir « La densité » dans
/// la doc du module.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_densite_max() -> f64 {
    let mut vitrine = verrou();
    densite_a_jour(&mut vitrine).map_or(f64::NAN, |densite| densite.maximum)
}

/// Le nombre d'obstacles du milieu de la simulation en cours ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_nb_obstacles() -> u32 {
    verrou().simulation.as_ref().map_or(0, |simulation| simulation.milieu().obstacles.len() as u32)
}

/// Une coordonnée de l'obstacle `i` du milieu, dans l'ordre du scénario : `k` vaut 0, 1, 2 ou 3 pour `x0`, `y0`,
/// `x1` ou `y1`. Un obstacle est un rectangle de **cases** et ses deux bords sont inclus : l'obstacle `50 20 53 80`
/// occupe les colonnes 50 à 53 et les lignes 20 à 80.
///
/// Rend `NaN` si `i >= mycelium_nb_obstacles()` ou si `k > 3`, et avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_obstacle(i: u32, k: u32) -> f64 {
    if k > 3 {
        return f64::NAN;
    }
    let verrou = verrou();
    let Some(obstacle) =
        verrou.simulation.as_ref().and_then(|simulation| simulation.milieu().obstacles.get(i as usize))
    else {
        return f64::NAN;
    };
    [obstacle.x0, obstacle.y0, obstacle.x1, obstacle.y1][k as usize] as f64
}

/// La biomasse de la colonie (la somme des longueurs de ses segments) ; `NaN` avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_biomasse() -> f64 {
    verrou().simulation.as_ref().map_or(f64::NAN, Simulation::biomasse)
}

/// Le nombre de pointes (apex vivants) ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_pointes() -> u32 {
    verrou().simulation.as_ref().map_or(0, |simulation| simulation.pointes() as u32)
}

/// Une coordonnée de la `i`-ième pointe **vivante**, dans l'ordre des identifiants d'apex : `k` vaut 0 pour `x`, 1 pour `y`.
///
/// `i` va de 0 à `mycelium_pointes() - 1` ; un apex anastomosé ou arrêté par un obstacle n'est pas compté. Rend `NaN` si
/// `i >= mycelium_pointes()` ou si `k > 1`, et avant tout démarrage. Chaque appel parcourt les apex : voir « Les lectures de
/// la page » dans la doc du module.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_pointe(i: u32, k: u32) -> f64 {
    if k > 1 {
        return f64::NAN;
    }
    let verrou = verrou();
    let Some(pointe) = verrou
        .simulation
        .as_ref()
        .and_then(|simulation| simulation.etat_des_pointes().filter(|pointe| pointe.actif).nth(i as usize))
    else {
        return f64::NAN;
    };
    [pointe.x, pointe.y][k as usize]
}

/// Le nombre cumulé d'anastomoses ; 0 avant tout démarrage.
#[unsafe(no_mangle)]
pub extern "C" fn mycelium_anastomoses() -> u32 {
    verrou().simulation.as_ref().map_or(0, |simulation| simulation.anastomoses() as u32)
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
    let Some(simulation) = verrou.simulation.as_ref() else {
        return 0;
    };
    let empreinte = simulation.empreinte();
    let debut = i as usize * 8;
    u32::from_str_radix(&empreinte[debut..debut + 8], 16).expect("un mot d'empreinte est de l'hexadécimal")
}
