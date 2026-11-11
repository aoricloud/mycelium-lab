//! Étape 13 (M3) : le solveur linéaire `systeme` : Cholesky contre un oracle, la mise à l'échelle, le refus d'un
//! système singulier, le plus petit mode et son seuil, les degrés de liberté imposés.
//!
//! Les tolérances viennent de la mesure (voir `MODELE.md`) : un système bien conditionné est résolu à quelques 1e-14 près ;
//! l'erreur d'un système mal conditionné vaut au plus `2,5e-16 / mode` sur tous les cas mesurés ; les tests permettent
//! `2,2e-15 / mode`.

#[path = "etape_13/outils.rs"]
mod outils;

use mycelium_lab::systeme::{Mecanisme, SEUIL_SINGULIER, plus_petit_mode, resoudre, resoudre_avec_imposes};
use outils::{Hasard, ecart, fois, gauss};

/// Une matrice symétrique définie positive au hasard, `n × n` : la somme de `rang` matrices `g gᵀ` (`g` au hasard), plus
/// `plus` fois l'identité. Avec `plus = 0` et `rang < n`, elle est singulière.
fn matrice(h: &mut Hasard, n: usize, rang: usize, plus: f64) -> Vec<f64> {
    let mut a = vec![0.0; n * n];
    for _ in 0..rang {
        let g: Vec<f64> = (0..n).map(|_| h.pm()).collect();
        for i in 0..n {
            for j in 0..n {
                a[i * n + j] += g[i] * g[j];
            }
        }
    }
    for i in 0..n {
        a[i * n + i] += plus;
    }
    a
}

/// La matrice de deux ressorts en série, de raideurs 1 (attaché au sol) puis `r` : `u₁ = 1` et `u₂ = 1 + 1/r` sous
/// une force 1 au bout. Son plus petit mode vaut `1 − √(r/(1 + r))`, à peu près `1/(2r)`.
fn serie(r: f64) -> [f64; 4] {
    [1.0 + r, -r, -r, r]
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn cholesky_donne_la_reponse_de_gauss_quelles_que_soient_les_unites() {
    assert!(ecart(&resoudre(&[4.0, 2.0, 2.0, 3.0], &[10.0, 8.0]).unwrap(), &[1.75, 1.5]) < 1e-15);
    assert_eq!(resoudre(&[4.0], &[2.0]), Ok(vec![0.5]));
    assert_eq!(resoudre(&[], &[]), Ok(vec![]));
    let mut h = Hasard(13);
    for essai in 0..200 {
        let n = 1 + essai % 30;
        let (a, b) = (matrice(&mut h, n, n + 2, 0.1), (0..n).map(|_| h.pm()).collect::<Vec<f64>>());
        let x = resoudre(&a, &b).expect("définie positive");
        assert!(ecart(&x, &gauss(&a, &b)) < 1e-11, "essai {essai} : Gauss");
        assert!(ecart(&fois(&a, &x), &b) < 1e-12, "essai {essai} : résidu");
        // D'autres unités pour chaque inconnue : `(D A D) (D⁻¹ x) = D b`, avec des échelles de 1e-6 à 1e6.
        let d: Vec<f64> = (0..n).map(|_| [1e-6, 1e-3, 1.0, 1e3, 1e6][h.jusqu_a(5)]).collect();
        let ada: Vec<f64> = (0..n * n).map(|q| d[q / n] * a[q] * d[q % n]).collect();
        let db: Vec<f64> = (0..n).map(|i| d[i] * b[i]).collect();
        let dx: Vec<f64> =
            resoudre(&ada, &db).expect("la même, en d'autres unités").iter().zip(&d).map(|(y, d)| y * d).collect();
        assert!(ecart(&dx, &x) < 1e-11, "essai {essai} : unités");
        assert!((plus_petit_mode(&ada, n) / plus_petit_mode(&a, n) - 1.0).abs() < 1e-9, "essai {essai} : mode");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn un_systeme_singulier_ne_rend_jamais_un_nombre() {
    let (nan, inf) = (f64::NAN, f64::INFINITY);
    let singulieres: [&[f64]; 10] = [
        &[0.0],
        &[-1.0],
        &[1.0, 0.0, 0.0, 0.0],  // un degré de liberté sans raideur
        &[1.0, 1.0, 1.0, 1.0],  // de rang un
        &[1.0, 2.0, 2.0, 1.0],  // indéfinie
        &[1.0, 0.0, 0.0, -1.0], // une raideur négative
        &[1.0, nan, nan, 1.0],
        &[inf, 0.0, 0.0, 1.0],
        &[1.0, inf, inf, 1.0],
        &[2.0, -2.0, 0.0, -2.0, 4.0, -2.0, 0.0, -2.0, 2.0], // trois ressorts libres : une translation d'ensemble
    ];
    for a in singulieres {
        let n = if a.len() == 9 { 3 } else { a.len().min(2) };
        assert_eq!(resoudre(a, &vec![1.0; n]), Err(Mecanisme), "{a:?}");
        assert_eq!(resoudre(a, &vec![0.0; n]), Err(Mecanisme), "{a:?} sans charge");
        assert!(plus_petit_mode(a, n) <= 1e-15, "{a:?}");
    }
    // Des matrices singulières au hasard (de rang n − 1, n − 2 ou n − 3) : le plus petit mode reste un bruit d'arrondi.
    let mut h = Hasard(14);
    for essai in 0..100 {
        let n = 4 + essai % 20;
        let a = matrice(&mut h, n, n - 1 - essai % 3, 0.0);
        let mode = plus_petit_mode(&a, n);
        assert!((0.0..=1e-14).contains(&mode), "essai {essai} : mode {mode:e}");
        assert_eq!(resoudre(&a, &vec![1.0; n]), Err(Mecanisme), "essai {essai}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn le_plus_petit_mode_mesure_la_singularite_et_le_seuil_tient_a_un_facteur_quatre() {
    assert_eq!(SEUIL_SINGULIER, 1e-12);
    assert_eq!(plus_petit_mode(&[], 0), f64::INFINITY);
    assert_eq!(plus_petit_mode(&[5.0], 1), 1.0);
    // Un contraste entre des inconnues sans lien n'est pas une singularité : la mise à l'échelle l'efface.
    assert_eq!(plus_petit_mode(&[1.0, 0.0, 0.0, 1e-20], 2), 1.0);
    // [[1, c], [c, 1]] a les valeurs propres 1 − c et 1 + c : le mode est un majorant de la plus petite, exact quand
    // elle est petite devant l'autre.
    let mode = plus_petit_mode(&[1.0, 0.5, 0.5, 1.0], 2);
    assert!((0.5..=1.5).contains(&mode), "{mode}");
    // (Un seul pas d'itération inverse laisserait une erreur de petit/2 : le second la ramène sous 1e-9 tant que l'arrondi ne
    // domine pas, c'est-à-dire pour petit ≥ 1e-6 ; l'assertion permet 1e-6.)
    for petit in [1e-3, 1e-6, 1e-9, 1e-12] {
        let mode = plus_petit_mode(&[1.0, 1.0 - petit, 1.0 - petit, 1.0], 2);
        assert!((mode / petit - 1.0).abs() < 1e-6_f64.max(1e-15 / petit), "{petit:e} : {mode:e}");
    }
    // Deux ressorts en série de raideurs 1 et r : acceptés tant que le mode (1/(2r)) dépasse le seuil, avec une
    // erreur d'au plus 2,2e-15 / mode (environ dix fois la plus grande mesurée, 2,5e-16 / mode) ; refusés au-delà.
    for r in [1.0, 1e3, 1e6, 1e9, 1e11, 2.5e11] {
        let mode = plus_petit_mode(&serie(r), 2);
        assert!((mode * 2.0 * r).max(1.0 / (mode * 2.0 * r)) < 1.8, "r = {r:e} : mode {mode:e}");
        let u = resoudre(&serie(r), &[0.0, 1.0]).unwrap_or_else(|_| panic!("r = {r:e} est valide"));
        let erreur = (u[0] - 1.0).abs().max(((u[1] - u[0]) * r - 1.0).abs());
        assert!(erreur <= 2.2e-15 / mode, "r = {r:e} : erreur {erreur:e}, mode {mode:e}");
    }
    for r in [1e12, 1e13, 1e15, 1e18] {
        assert_eq!(resoudre(&serie(r), &[0.0, 1.0]), Err(Mecanisme), "r = {r:e}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn les_degres_de_liberte_imposes_les_reactions_et_l_energie() {
    // Trois nœuds sur une ligne, deux ressorts de raideurs 2 et 3.
    let k = [2.0, -2.0, 0.0, -2.0, 5.0, -3.0, 0.0, -3.0, 3.0];
    let proche = |a: &[f64], b: &[f64]| ecart(a, b) < 1e-14;
    // Le nœud 0 tenu, le nœud 2 tiré de 6 : les deux ressorts portent 6.
    let r = resoudre_avec_imposes(&k, &[0.0, 0.0, 6.0], &[(0, 0.0)]).unwrap();
    assert!(proche(&r.deplacements, &[0.0, 3.0, 5.0]) && proche(&r.reactions, &[-6.0, 0.0, 0.0]), "{r:?}");
    assert!((r.energie - 15.0).abs() < 1e-13 && r.reactions[1] == 0.0 && r.reactions[2] == 0.0, "{r:?}");
    // Les deux bouts poussés (1 et 4), sans force : le milieu se place à 2,8 ; les ressorts portent 3,6.
    let r = resoudre_avec_imposes(&k, &[0.0; 3], &[(2, 4.0), (0, 1.0)]).unwrap();
    assert!(proche(&r.deplacements, &[1.0, 2.8, 4.0]) && proche(&r.reactions, &[-3.6, 0.0, 3.6]), "{r:?}");
    assert!((r.energie - 5.4).abs() < 1e-14, "{r:?}");
    // Une force sur un degré de liberté imposé part dans sa réaction ; une force sur le milieu le déplace.
    let r = resoudre_avec_imposes(&k, &[7.0, 5.0, 0.0], &[(2, 4.0), (0, 1.0)]).unwrap();
    assert!(proche(&r.deplacements, &[1.0, 3.8, 4.0]) && proche(&r.reactions, &[-12.6, 0.0, 0.6]), "{r:?}");
    // Tout imposé : rien à résoudre. Rien d'imposé : les ressorts libres sont un mécanisme.
    let r = resoudre_avec_imposes(&k, &[0.0; 3], &[(0, 1.0), (1, 1.0), (2, 2.0)]).unwrap();
    assert!(proche(&r.deplacements, &[1.0, 1.0, 2.0]) && proche(&r.reactions, &[0.0, -3.0, 3.0]), "{r:?}");
    assert!((r.energie - 1.5).abs() < 1e-14, "{r:?}");
    assert_eq!(resoudre_avec_imposes(&k, &[0.0, 0.0, 1.0], &[]), Err(Mecanisme));
    assert_eq!(resoudre_avec_imposes(&[], &[], &[]).unwrap().energie, 0.0);
    // Les erreurs de programme paniquent.
    for (taille, imposes) in [(8, vec![(0, 0.0)]), (9, vec![(3, 0.0)]), (9, vec![(1, 0.0), (1, 0.0)])] {
        let essai = std::panic::catch_unwind(|| resoudre_avec_imposes(&k[..taille], &[0.0; 3], &imposes));
        assert!(essai.is_err(), "{taille} nombres, imposés {imposes:?}");
    }
    assert!(std::panic::catch_unwind(|| resoudre(&k, &[0.0; 2])).is_err());
    assert!(std::panic::catch_unwind(|| plus_petit_mode(&k, 2)).is_err());
}
