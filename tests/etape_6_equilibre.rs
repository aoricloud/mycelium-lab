//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie équilibre : l'orientation moyenne d'une pointe
//! seule dans un gradient constant (preuve 5, celle qui ne dépend d'aucun réglage).
//!
//! Le noyau d'un pas de la pointe est écrit ici comme la spécification le décrit : un tirage de `delta` uniforme sur
//! `-amplitude..=amplitude` (`alea.suivant() % (2·amplitude + 1)`), la direction tourne de `delta`, puis
//! `tourner_chimiotropique` fait le virage vers le gradient avec son arrondi au sort. La pointe ne se déplace pas : le
//! gradient reste le même. La direction forme donc une chaîne de Markov sur 360 états, dont la loi d'équilibre se
//! calcule exactement : les constantes de `EXACTES` sont `E[cos(θ − θ_g)]` à l'équilibre, calculées **hors du code testé**
//! par un script Python en nombres flottants (`chaine-t1-exact.py` : il lit les tables `COS` et `SIN` du dépôt, résout
//! `π (T − I) = 0` par élimination de Gauss avec pivot, et vérifie le résidu `π T − π`, inférieur à 1e-16 ; pour
//! `amplitude = 20`, force 1 et `s = 0,5`, elle donne 0,2016, comme la spécification). Elles ne dépendent que de
//! l'amplitude et du produit `p = force · s`.
//!
//! Ils fixent : `E[cos(θ − θ_g)]` mesuré (20 chaînes de 100 000 pas, après 1 000 pas de mise en route) vaut la valeur
//! exacte à `TOLERANCE_EQUILIBRE` près, pour plusieurs amplitudes, forces, normes de gradient (donc `s`) et directions
//! de gradient (les quatre axes) ; il est positif quand la force l'est (la pointe se tourne **vers** le gradient, dans
//! le bon sens, pour les quatre axes) ; il est croissant avec la force ; il est nul à force 0 ; il ne dépend de la
//! force et de la norme que par `force · s`.
//!
//! Ils laissent ouverts : le détail de la trajectoire (seule la loi d'équilibre est fixée) ; la durée de la mise en
//! route.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `abs`.

use mycelium_lab::alea::Alea;
use mycelium_lab::croissance::{COS, NB_DIRECTIONS, SIN};
use mycelium_lab::tropisme::tourner_chimiotropique;

/// Nombre de chaînes indépendantes (une graine chacune).
const CHAINES: u64 = 20;
/// Nombre de pas mesurés par chaîne.
const PAS_PAR_CHAINE: u32 = 100_000;
/// Pas de mise en route, non mesurés (le temps de relaxation de la chaîne est d'au plus 120 pas).
const MISE_EN_ROUTE: u32 = 1_000;

/// Tolérance (absolue) sur `E[cos(θ − θ_g)]`. L'écart-type de la moyenne de `CHAINES × PAS_PAR_CHAINE = 2·10⁶` pas est
/// `sqrt(σ² / 2·10⁶)`, avec la variance asymptotique `σ²` de la chaîne, que le script Python calcule aussi : 46,7 à
/// `p = 0`, 42,6 à `p = 0,5`, 32,7 à `p = 1`, 13,3 à `p = 2`, 1,4 à `p = 4` ; donc au plus 0,0048. Mesuré sur le
/// prototype, sur 30 blocs de graines différents par cas : les écarts-types observés sont ceux de la théorie, à 15 % près
/// (au plus 0,0055) et l'écart maximal observé à la valeur exacte est 0,0126 (tous les cas, 30 blocs). La tolérance, 0,03,
/// est 5,4 fois le plus grand écart-type observé (6,2 fois celui de la théorie). Les fautes de code testées séparément
/// (voir le rapport) font sortir au moins un cas de la table de plus de 0,1 : arrondi vers le bas (0,16 à `p = 1`), arrondi
/// au plus proche (0,10 à `p = 0,25`, 0,20 à `p = 0,5`), arrondi vers zéro (0,2 à `p = 0,5`), signe inversé (le double
/// de la valeur), `s` oublié (0,18 à `p = 0,5`), virage divisé par deux (0,10 à `p = 0,25`).
///
/// Rejoué par le juge sur 20 autres jeux de graines (graines décalées de `1 000 003 · k`, k = 31 à 50) : écart-type
/// observé de `moyenne − exacte` au plus 0,0051 (sur les 14 cas de la table et les 4 axes), écart maximal 0,011 ; la
/// tolérance est donc à 5,9 écarts-types. Le pas de croissance exigé (0,05) est à 6,4 écarts-types du plus petit gain
/// moyen (force 0 → 0,5 : 0,102, écart-type 0,0081). Les valeurs exactes de `EXACTES` ont été recalculées par le juge
/// par une autre méthode (itération du noyau jusqu'à un résidu < 2e-17) : identiques à 8 chiffres.
const TOLERANCE_EQUILIBRE: f64 = 0.03;

/// `GRADIENT_DE_REFERENCE`, réécrit ici : la norme `G0` donne `s = 0,5`.
const G0: f64 = 0.01;
/// Une norme si grande que `s = n / (n + 0,01)` vaut 1 à 1e-11 près.
const NORME_FORTE: f64 = 1e9;

/// `(amplitude, p = force · s, E[cos(θ − θ_g)] exact)` : sortie de `chaine-t1-exact.py`, huit chiffres.
const EXACTES: [(u64, f64, f64); 14] = [
    (20, 0.0, 0.0),
    (20, 0.25, 0.10235700),
    (20, 0.5, 0.20160672),
    (20, 1.0, 0.38138201),
    (20, 1.5, 0.52639870),
    (20, 2.0, 0.63571556),
    (20, 3.0, 0.77101386),
    (20, 4.0, 0.84103468),
    (20, 8.0, 0.93435759),
    (10, 1.0, 0.82122305),
    (10, 2.0, 0.91944777),
    (40, 1.0, 0.10691924),
    (40, 2.0, 0.21082415),
    (5, 1.0, 0.95569436),
];

/// Le gradient de norme `norme` dont l'angle est `angle` degrés (un multiple de 90).
fn gradient_vers(angle: usize, norme: f64) -> (f64, f64) {
    (norme * COS[angle], norme * SIN[angle])
}

/// La moyenne de `cos(θ − θ_g)` sur `CHAINES` chaînes de `PAS_PAR_CHAINE` pas, `θ_g = angle` (en degrés entiers), pour
/// une amplitude, une force et un gradient de norme `norme`. Les graines sont `graine0 + 1 ..= graine0 + CHAINES`.
fn orientation_moyenne(amplitude: u64, force: f64, norme: f64, angle: usize, graine0: u64) -> f64 {
    let gradient = gradient_vers(angle, norme);
    let mut somme = 0.0;
    for graine in graine0 + 1..=graine0 + CHAINES {
        let mut alea = Alea::new(graine);
        // Comme `Simulation::nouvelle` : la première direction est tirée dans la table.
        let mut direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
        for pas in 0..MISE_EN_ROUTE + PAS_PAR_CHAINE {
            // Le pas de la pointe : une marche au hasard, puis le virage vers le gradient.
            let delta = (alea.suivant() % (2 * amplitude + 1)) as i64 - amplitude as i64;
            direction = (direction as i64 + delta).rem_euclid(NB_DIRECTIONS as i64) as usize;
            direction = tourner_chimiotropique(direction, gradient, force, &mut alea);
            if pas >= MISE_EN_ROUTE {
                // cos(θ − θ_g), lu dans la table à l'indice de l'écart.
                somme += COS[(direction + NB_DIRECTIONS - angle) % NB_DIRECTIONS];
            }
        }
    }
    somme / (CHAINES as f64 * f64::from(PAS_PAR_CHAINE))
}

/// 5. L'orientation moyenne à l'équilibre vaut la valeur exacte de la chaîne (14 cas : amplitudes 5, 10, 20 et 40, `p`
///    de 0 à 8). Les cas pairs de la table sont joués avec un gradient très fort (`s = 1`, force `p`), les cas impairs avec
///    un gradient de norme `G0` (`s = 0,5`, force `2p`) : la loi ne dépend que de `force · s`, donc les deux familles
///    doivent tomber sur la même table.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn l_orientation_moyenne_a_l_equilibre_est_la_valeur_exacte() {
    for (k, (amplitude, p, exacte)) in EXACTES.into_iter().enumerate() {
        let (force, norme, nom) =
            if k % 2 == 0 { (p, NORME_FORTE, "gradient très fort") } else { (2.0 * p, G0, "gradient G0") };
        let moyenne = orientation_moyenne(amplitude, force, norme, 0, 1_000 * k as u64);
        assert!(
            (moyenne - exacte).abs() <= TOLERANCE_EQUILIBRE,
            "amplitude {amplitude}, force {force}, {nom} : E[cos(θ − θ_g)] = {moyenne}, exact {exacte}"
        );
    }
}

/// 5. La pointe se tourne vers le gradient, dans le bon sens, pour les quatre axes : pour un gradient d'angle 0°, 90°,
///    180° et 270° (norme `G0`, force 2, amplitude 20), `E[cos(θ − θ_g)]` vaut la valeur exacte de `p = 1` (0,3814). Une
///    faute de signe (la pointe fuirait le gradient) ou d'axe (`x` et `y` échangés) donnerait − 0,38 ou une valeur
///    nulle pour les axes concernés.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_pointe_se_tourne_vers_le_gradient_pour_les_quatre_axes() {
    for angle in [0usize, 90, 180, 270] {
        let moyenne = orientation_moyenne(20, 2.0, G0, angle, 5_000 + angle as u64);
        assert!(
            (moyenne - 0.38138201).abs() <= TOLERANCE_EQUILIBRE,
            "gradient d'angle {angle}° : E[cos(θ − θ_g)] = {moyenne}, exact 0,3814"
        );
    }
}

/// 5. L'orientation moyenne est strictement croissante avec la force (0, 0,5, 1, 2, 4, 8, 16 ; amplitude 20, gradient de
///    norme `G0`), nulle à force 0 (à `TOLERANCE_EQUILIBRE`), et ne dépend de la force et de `s` que par leur produit :
///    force 1 à `s = 1` et force 2 à `s = 0,5` donnent la même moyenne (à la tolérance, sur des graines différentes).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn l_orientation_moyenne_croit_avec_la_force_et_est_nulle_a_force_0() {
    let forces = [0.0, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];
    let moyennes: Vec<f64> =
        forces.iter().enumerate().map(|(k, &f)| orientation_moyenne(20, f, G0, 0, 10_000 + 100 * k as u64)).collect();
    assert!(moyennes[0].abs() <= TOLERANCE_EQUILIBRE, "force 0 : E[cos(θ − θ_g)] = {}", moyennes[0]);
    for k in 1..forces.len() {
        assert!(
            moyennes[k] > moyennes[k - 1] + 0.05,
            "force {} : {} ne dépasse pas force {} : {} de 0,05",
            forces[k],
            moyennes[k],
            forces[k - 1],
            moyennes[k - 1]
        );
    }
    // Même produit force · s : (force 2, s = 0,5) et (force 1, s = 1).
    let a = moyennes[3];
    let b = orientation_moyenne(20, 1.0, NORME_FORTE, 0, 20_000);
    assert!((a - b).abs() <= 2.0 * TOLERANCE_EQUILIBRE, "force 2 à s = 0,5 : {a} ; force 1 à s = 1 : {b}");
}
