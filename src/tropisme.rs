//! Tropismes : ce qui fait pencher une pointe de la colonie. L'étape T1 pose le chimiotropisme.
//!
//! # Le chimiotropisme
//!
//! Une pointe se penche vers le côté où il y a plus de nutriments. À chaque pas, juste après sa marche au hasard,
//! elle lit le gradient du substrat de la case où elle se trouve ([`crate::substrat::Substrat::gradient`]) et tourne
//! d'un petit angle vers lui ([`tourner_chimiotropique`]). Le virage réel ([`virage_chimiotropique`]) vaut
//!
//! `force · s · sin(angle du gradient − direction de la pointe)`, avec `s = n / (n + G₀)`,
//!
//! où `n` est la norme du gradient et `G₀` ([`GRADIENT_DE_REFERENCE`]) la constante de saturation de la réponse :
//! un gradient très faible tourne peu, un gradient très fort tourne au plus de `force` degrés. Le virage est nul
//! quand le gradient est aligné sur la pointe ou opposé à elle, et maximal à angle droit.
//!
//! La direction est un entier (un degré par indice de table) : le virage, qui est un réel, est arrondi **au
//! sort**, pour que sa partie fractionnaire ne soit pas perdue. Un virage de 0,3 degré fait tourner d'un degré
//! dans 30 % des pas, de zéro degré dans les autres : en moyenne, le virage appliqué est exactement le virage réel.
//!
//! Avec une force nulle, ou un gradient nul, la pointe ne tire aucun nombre au hasard de plus : l'état est celui
//! d'avant, octet pour octet. G₀ est un choix de modélisation, non calibré.

use crate::alea::Alea;
use crate::croissance::{COS, NB_DIRECTIONS, SIN};

/// La constante de saturation de la réponse, dans la même unité que le gradient du substrat : un gradient de cette
/// norme tourne la pointe de la moitié du virage maximal. Un choix de modélisation, non calibré.
pub const GRADIENT_DE_REFERENCE: f64 = 0.01;

/// La plus grande force permise, en degrés : le virage d'un pas ne dépasse jamais 45°. Au-delà d'environ 57° (où
/// `force · sin 1° = 1` : la pointe à un degré du gradient le dépasse), l'orientation moyenne cesserait de croître avec
/// la force ; la borne 45 la garde croissante pour toute amplitude de la marche.
pub const FORCE_MAXIMALE: f64 = 45.0;

/// Les tropismes d'une simulation. `Tropismes::default()` les éteint tous.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Tropismes {
    /// Force du chimiotropisme, en degrés : le virage maximal d'une pointe par pas, dans un gradient très fort.
    /// `0.0` l'éteint. Dans `0.0..=FORCE_MAXIMALE` (`0.0..=45.0`, [`crate::simulation::Simulation::nouvelle_avec_tropismes`] refuse le
    /// reste).
    pub chimiotropisme: f64,
    /// L'angle critique du thigmotropisme, en degrés : une pointe qui touche un obstacle sous un angle (par rapport
    /// à sa surface) inférieur à celui-ci le longe, sinon elle s'arrête et se ramifie contre lui. `0.0` l'éteint
    /// (une pointe qui touche reste où elle est). Dans `0.0..=ANGLE_CRITIQUE_MAXIMAL` (`0.0..=90.0`,
    /// [`crate::simulation::Simulation::nouvelle_avec_tropismes`] refuse le reste).
    pub thigmotropisme: f64,
}

/// Le plus grand angle critique permis, en degrés : à 90, seule la perpendiculaire s'arrête. Une borne de contrat,
/// sans sens biologique.
pub const ANGLE_CRITIQUE_MAXIMAL: f64 = 90.0;

/// L'écart maximal, en degrés, du retour au cap après un obstacle : la direction retrouvée est celle d'avant, à
/// plus ou moins cette valeur (Held et al. 2019, chiffre cité d'après les travaux antérieurs du groupe).
pub const MEMOIRE_DE_DIRECTION: u32 = 20;

/// La face d'un obstacle touchée par une pointe : une ligne de la grille.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// Une ligne `x = entier` (la normale est selon x : la face est parallèle à l'axe des y).
    Verticale,
    /// Une ligne `y = entier` (la normale est selon y : la face est parallèle à l'axe des x).
    Horizontale,
}

/// Le virage **réel**, en degrés, d'une pointe de direction `direction` dans le gradient `gradient`.
///
/// Positif quand le gradient est dans le sens trigonométrique de la pointe (la direction croît). Nul quand le
/// gradient est nul, aligné ou opposé à la pointe ; maximal, `force · s`, à angle droit.
///
/// # Panics
///
/// Si `direction >= 360` (même pour un gradient nul), ou si `force` n'est pas dans `0.0..=45.0` (`FORCE_MAXIMALE`).
pub fn virage_chimiotropique(direction: usize, gradient: (f64, f64), force: f64) -> f64 {
    assert!(direction < NB_DIRECTIONS, "direction doit être inférieure à {NB_DIRECTIONS} : {direction}");
    assert!((0.0..=FORCE_MAXIMALE).contains(&force), "force doit être dans 0.0..={FORCE_MAXIMALE} : {force}");
    let (gx, gy) = gradient;
    let n = (gx * gx + gy * gy).sqrt();
    if n > 0.0 {
        let sinus = (COS[direction] * gy - SIN[direction] * gx) / n;
        let s = n / (n + GRADIENT_DE_REFERENCE);
        force * s * sinus
    } else {
        0.0
    }
}

/// La nouvelle direction d'une pointe qui tourne vers le gradient : le virage réel, arrondi au sort.
///
/// Sans tirage quand `force <= 0.0` ou quand le gradient est nul ; sinon, un seul tirage de `alea`.
///
/// # Panics
///
/// Comme [`virage_chimiotropique`], quand `force > 0.0` : `direction >= 360`, ou `force` supérieure à 45 ou non finie.
pub fn tourner_chimiotropique(direction: usize, gradient: (f64, f64), force: f64, alea: &mut Alea) -> usize {
    if force <= 0.0 {
        return direction;
    }
    let v = virage_chimiotropique(direction, gradient, force);
    let (gx, gy) = gradient;
    let n = (gx * gx + gy * gy).sqrt();
    if n == 0.0 {
        return direction;
    }
    let bas = v.floor();
    let frac = v - bas;
    let mut d = direction as i64 + bas as i64;
    if alea.suivant_f64() < frac {
        d += 1;
    }
    d.rem_euclid(NB_DIRECTIONS as i64) as usize
}

/// L'angle de contact, en degrés entiers, d'une pointe de direction `direction` avec la face `face` : l'écart à la
/// parallèle à la face, de 0 (parallèle) à 90 (perpendiculaire). Face verticale : `|90 − (d mod 180)|` ; face
/// horizontale : `min(d mod 180, 180 − d mod 180)`.
///
/// # Panics
///
/// Si `direction >= 360`.
pub fn angle_de_contact(direction: usize, face: Face) -> u32 {
    todo!("étape 7 : angle_de_contact {direction} {face:?}")
}

/// La direction de glissement : celle des deux directions parallèles à la face (90 ou 270 pour une face verticale,
/// 0 ou 180 pour une horizontale) dont l'écart angulaire à `direction` est le plus petit ; à égalité, la plus petite.
///
/// # Panics
///
/// Si `direction >= 360`.
pub fn direction_de_glissement(direction: usize, face: Face) -> usize {
    todo!("étape 7 : direction_de_glissement {direction} {face:?}")
}

/// Les deux directions parallèles à la face, dans l'ordre croissant : `(90, 270)` pour une face verticale, `(0, 180)`
/// pour une horizontale. C'est l'ordre des naissances à l'arrêt d'une pointe.
pub fn branches_de_contact(face: Face) -> (usize, usize) {
    todo!("étape 7 : branches_de_contact {face:?}")
}
