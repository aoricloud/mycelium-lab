//! Étape 17 (M4) : l'ajustement en échelle logarithmique. Des points posés exactement sur `K = c ρⁿ` rendent `n` et `c` ;
//! la pente ne dépend ni de l'échelle, ni de l'ordre, ni du doublement des points ; un `R²` connu ; et ce qui ne s'ajuste
//! pas est une erreur, pas un nombre.

use mycelium_lab::mesures::logarithme;
use mycelium_lab::nuage::{Droite, ErreurAjustement, ajuster};

/// L'erreur mesurée de la pente, de l'ordonnée et de `R²` sur ces cas est de 1,3e-15 au plus : une marge de 750. Un
/// ajustement faux (la pente et l'ordonnée échangées, un logarithme oublié, une moyenne décalée) fait plus de 1e-3.
const TOLERANCE: f64 = 1e-12;
const DENSITES: [f64; 6] = [0.1, 0.2, 0.5, 1.0, 2.0, 4.0];

/// `ρⁿ` pour `n` = 1, 2, 3 et 2,5, avec `*` et `sqrt` seulement.
fn puissance(rho: f64, n: f64) -> f64 {
    [(1.0, rho), (2.0, rho * rho), (3.0, rho * rho * rho), (2.5, rho * rho * rho.sqrt())]
        .into_iter()
        .find(|&(exposant, _)| exposant == n)
        .expect("n vaut 1, 2, 3 ou 2,5")
        .1
}

/// Six points posés sur `K = c ρⁿ`.
fn loi(n: f64, c: f64) -> Vec<(f64, f64)> {
    DENSITES.iter().map(|&rho| (rho, c * puissance(rho, n))).collect()
}

/// Vrai si la droite a cette pente, cette ordonnée et ce `R²`, à `TOLERANCE` près, et si son `R²` ne dépasse pas 1.
fn est(droite: Droite, pente: f64, ordonnee: f64, r2: f64) -> bool {
    let proche = |a: f64, b: f64| (a - b).abs() < TOLERANCE;
    proche(droite.pente, pente) && proche(droite.ordonnee, ordonnee) && proche(droite.r2, r2) && droite.r2 <= 1.0
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn des_points_sur_k_egal_c_rho_puissance_n_rendent_n_et_c() {
    for (n, c) in [(1.0, 0.37), (2.0, 0.37), (3.0, 0.37), (2.5, 0.37), (3.0, 1e-3), (2.0, 250.0)] {
        let droite = ajuster(&loi(n, c)).unwrap();
        assert!(est(droite, n, logarithme(c), 1.0) && droite.points == 6, "n = {n}, c = {c} : {droite:?}");
    }
    // Trois points sur `K = 0,37 ρ` pour lesquels l'arrondi donnerait un `R²` de 1,0000000000000002 : il est borné à 1.
    let trois: Vec<_> = [0.1, 0.2, 0.3].iter().map(|&rho| (rho, 0.37 * rho)).collect();
    assert_eq!(ajuster(&trois).unwrap().r2, 1.0);
    // Une raideur qui baisse quand la densité monte, `K = 2/ρ` : la pente est négative, `R²` vaut encore 1.
    let droite = ajuster(&[(0.5, 4.0), (1.0, 2.0), (4.0, 0.5)]).unwrap();
    assert!(est(droite, -1.0, logarithme(2.0), 1.0) && droite.points == 3, "{droite:?}");
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn un_nuage_qui_n_est_pas_sur_une_droite_a_la_pente_l_ordonnee_et_le_r2_calcules_a_la_main() {
    // En logarithmes de base 2 : (0, 0), (1, 2), (2, 1). Moyennes 1 et 1 ; Sxx = 2, Sxy = 1, Syy = 2 : la pente vaut 1/2,
    // `R²` = Sxy²/(Sxx Syy) = 1/4, et l'ordonnée, 1 − 1/2 en base 2, vaut `logarithme(2)/2`.
    let droite = ajuster(&[(1.0, 1.0), (2.0, 4.0), (4.0, 2.0)]).unwrap();
    assert!(est(droite, 0.5, logarithme(2.0) / 2.0, 0.25) && droite.points == 3, "{droite:?}");
    // Deux points : la droite qui les joint, `R²` = 1.
    let (a, b) = ((0.3, 0.05), (1.7, 0.9));
    let pente = (logarithme(b.1) - logarithme(a.1)) / (logarithme(b.0) - logarithme(a.0));
    let droite = ajuster(&[a, b]).unwrap();
    assert!(est(droite, pente, logarithme(a.1) - pente * logarithme(a.0), 1.0) && droite.points == 2, "{droite:?}");
    // Des raideurs toutes égales : la droite est plate, elle passe par tous les points et `R²` vaut 1, exactement.
    let plate = ajuster(&[(0.5, 2.0), (1.0, 2.0), (3.0, 2.0)]).unwrap();
    assert!(est(plate, 0.0, logarithme(2.0), 1.0) && plate.r2 == 1.0, "{plate:?}");
    // Des raideurs égales dont la moyenne des logarithmes ne tombe pas juste (`Syy` vaudrait 1e-32) : `R²` vaut 1.
    assert_eq!(ajuster(&[(0.1, 0.2), (0.2, 0.2), (0.1 * 3.0, 0.2)]).unwrap().r2, 1.0);
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn la_pente_ne_depend_ni_de_l_echelle_ni_de_l_ordre_ni_du_doublement() {
    let base = loi(2.5, 0.37);
    let (c, sept) = (logarithme(0.37), logarithme(7.0));
    // Tous les `K` fois 7 : l'ordonnée monte de `log 7`. Tous les `ρ` fois 7 : elle baisse de 2,5 `log 7`.
    let fois_k: Vec<_> = base.iter().map(|&(rho, k)| (rho, 7.0 * k)).collect();
    let fois_rho: Vec<_> = base.iter().map(|&(rho, k)| (7.0 * rho, k)).collect();
    assert!(est(ajuster(&fois_k).unwrap(), 2.5, c + sept, 1.0));
    assert!(est(ajuster(&fois_rho).unwrap(), 2.5, c - 2.5 * sept, 1.0));
    // Chaque point deux fois : la même droite, douze points.
    let double: Vec<_> = base.iter().chain(&base).copied().collect();
    let droite = ajuster(&double).unwrap();
    assert!(est(droite, 2.5, c, 1.0) && droite.points == 12, "{droite:?}");
    // L'ordre, sur un nuage qui n'est pas une droite : les paires sont triées d'abord, le résultat est le même au bit près.
    let nuage = [(0.2, 0.011), (0.3, 0.05), (0.3, 0.02), (0.9, 0.4), (1.7, 0.9), (2.5, 0.7)];
    let attendue = ajuster(&nuage).unwrap();
    assert!(attendue.r2 > 0.5 && attendue.r2 < 0.99, "un nuage dispersé : {attendue:?}");
    let inverse: Vec<_> = nuage.iter().rev().copied().collect();
    let melange = [nuage[3], nuage[0], nuage[5], nuage[2], nuage[4], nuage[1]];
    assert_eq!(ajuster(&inverse), Ok(attendue));
    assert_eq!(ajuster(&melange), Ok(attendue));
    // Deux paires de même densité : leur ordre non plus ne change rien (le tri regarde aussi `K` ; sans cela, ces deux
    // ordres donnent deux droites qui diffèrent d'un bit).
    let memes = [(0.2, 0.1), (0.5, 0.1), (0.5, 0.2), (0.9, 0.6)];
    assert_eq!(ajuster(&[memes[0], memes[2], memes[1], memes[3]]), ajuster(&memes));
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn ce_qui_ne_s_ajuste_pas_est_une_erreur() {
    let bonne = (0.5, 1.0);
    for mauvaise in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for paires in
            [[bonne, (mauvaise, 2.0)], [bonne, (1.0, mauvaise)], [(mauvaise, 2.0), bonne], [(1.0, mauvaise), bonne]]
        {
            assert_eq!(ajuster(&paires), Err(ErreurAjustement::ValeurInvalide), "{paires:?}");
        }
        // Une seule paire, impossible : c'est la valeur qui est refusée d'abord.
        assert_eq!(ajuster(&[(mauvaise, 1.0)]), Err(ErreurAjustement::ValeurInvalide), "{mauvaise}");
    }
    // Aucune paire, une seule, ou une seule densité : pas de pente.
    assert_eq!(ajuster(&[]), Err(ErreurAjustement::DensitesIdentiques));
    assert_eq!(ajuster(&[bonne]), Err(ErreurAjustement::DensitesIdentiques));
    assert_eq!(ajuster(&[(0.5, 1.0), (0.5, 2.0), (0.5, 3.0)]), Err(ErreurAjustement::DensitesIdentiques));
    // Des densités identiques dont la moyenne des logarithmes ne tombe pas juste (0,4 ; 0,2) : toujours un refus. Le
    // même point répété, c'est une colonie qui a cessé de croître.
    assert_eq!(ajuster(&[(0.4, 1.0), (0.4, 2.0), (0.4, 3.0)]), Err(ErreurAjustement::DensitesIdentiques));
    assert_eq!(ajuster(&[(0.2, 0.003); 7]), Err(ErreurAjustement::DensitesIdentiques));
    // Les plus petits et les plus grands nombres de `f64` s'ajustent : ils ont un logarithme.
    let extremes = ajuster(&[(f64::MIN_POSITIVE, f64::MAX), (1.0, 1.0)]).unwrap();
    assert!(extremes.pente < -1.0 && extremes.pente > -1.01 && extremes.ordonnee.abs() < 1e-9, "{extremes:?}");
}
