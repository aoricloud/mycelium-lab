//! Étape 17 (M4) : l'ajustement décide sur les logarithmes eux-mêmes. Deux nombres voisins d'un bit peuvent avoir le
//! même `logarithme` : en échelle logarithmique, ils sont égaux. Ce test a été écrit par le pilote de M4 après la
//! relecture de la PR 86, pas par le juge de l'étape : les autres tests de l'étape restent tels que le juge les a gelés.

use mycelium_lab::mesures::logarithme;
use mycelium_lab::nuage::{ErreurAjustement, ajuster};

/// Le nombre `f64` juste au-dessus de `x` (positif et fini).
fn voisin(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn deux_densites_distinctes_de_meme_logarithme_sont_refusees() {
    // 0,06 et son voisin sont deux densités distinctes, de même logarithme ; la moyenne des trois logarithmes ne tombe
    // pas juste, et `Sxx` vaudrait 2,7e-32 au lieu de 0 : sans la décision sur les logarithmes, la pente rendue serait
    // du bruit d'arrondi (1/6).
    let (a, b) = (0.06, voisin(0.06));
    assert!(a < b && logarithme(a) == logarithme(b));
    assert_eq!(ajuster(&[(a, 1.0), (b, 2.0), (a, 3.0)]), Err(ErreurAjustement::DensitesIdentiques));
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn deux_raideurs_distinctes_de_meme_logarithme_font_une_droite_plate_de_r2_egal_a_1() {
    // 0,02 et son voisin : deux raideurs distinctes, de même logarithme ; la droite est plate et passe par tous les
    // points, donc `R²` vaut 1, exactement (sans la décision sur les logarithmes : 2,7e-32).
    let (k, j) = (0.02, voisin(0.02));
    assert!(k < j && logarithme(k) == logarithme(j));
    let droite = ajuster(&[(0.1, k), (0.2, j), (0.1 * 3.0, k)]).unwrap();
    assert_eq!(droite.r2, 1.0, "{droite:?}");
}
