//! Tests d'acceptation de l'étape 1 : biomasse et dimension fractale sur des images synthétiques.

use mycelium_lab::mesures::{ImageBinaire, biomasse, dimension_fractale, logarithme};

/// Carré plein de `cote` × `cote`.
fn carre_plein(cote: usize) -> ImageBinaire {
    let mut image = ImageBinaire::new(cote, cote);
    for y in 0..cote {
        for x in 0..cote {
            image.ecrire(x, y, true);
        }
    }
    image
}

/// Tapis de Sierpinski de niveau `niveau` (côté 3^niveau) : une case est vide si, à un rang de son
/// écriture en base 3, ses deux coordonnées valent 1.
fn tapis_sierpinski(niveau: u32) -> ImageBinaire {
    let cote = 3usize.pow(niveau);
    let mut image = ImageBinaire::new(cote, cote);
    for y in 0..cote {
        for x in 0..cote {
            let (mut a, mut b) = (x, y);
            let mut pleine = true;
            while a > 0 || b > 0 {
                if a % 3 == 1 && b % 3 == 1 {
                    pleine = false;
                    break;
                }
                a /= 3;
                b /= 3;
            }
            image.ecrire(x, y, pleine);
        }
    }
    image
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn carre_plein_a_pour_dimension_2() {
    let image = carre_plein(256);
    let d = dimension_fractale(&image, &[1, 2, 4, 8, 16, 32, 64]);
    assert!((d - 2.0).abs() < 0.02, "dimension du carré : {d}");
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn segment_a_pour_dimension_1() {
    let mut image = ImageBinaire::new(256, 1);
    for x in 0..256 {
        image.ecrire(x, 0, true);
    }
    let d = dimension_fractale(&image, &[1, 2, 4, 8, 16, 32, 64]);
    assert!((d - 1.0).abs() < 0.02, "dimension du segment : {d}");
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn tapis_de_sierpinski_niveau_5() {
    let image = tapis_sierpinski(5);
    assert_eq!((image.largeur(), image.hauteur()), (243, 243));
    // ln 8 / ln 3, écrit en littéral : le test n'appelle pas `ln`.
    let attendu = 1.8927892607143721;
    let d = dimension_fractale(&image, &[1, 3, 9, 27, 81]);
    assert!((d - attendu).abs() < 0.01, "dimension du tapis : {d}");
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn biomasse_exacte() {
    let mut image = ImageBinaire::new(7, 5);
    assert_eq!(biomasse(&image), 0);
    for &(x, y) in &[(0, 0), (6, 4), (3, 2), (0, 4), (6, 0), (3, 3)] {
        image.ecrire(x, y, true);
    }
    // Réécrire une case pleine ne change rien ; vider une case la retire.
    image.ecrire(3, 2, true);
    image.ecrire(3, 3, false);
    assert!(image.lire(6, 4));
    assert!(!image.lire(3, 3));
    assert_eq!(biomasse(&image), 5);
    assert_eq!(biomasse(&carre_plein(16)), 256);
    assert_eq!(biomasse(&tapis_sierpinski(5)), 8usize.pow(5));
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn logarithme_valeurs_connues() {
    assert!(logarithme(1.0).abs() < 1e-9);
    assert!((logarithme(2.0) - std::f64::consts::LN_2).abs() < 1e-9);
    assert!((logarithme(3.0) - 1.0986122886681098).abs() < 1e-9);
    assert!((logarithme(1000.0) - 6.907755278982137).abs() < 1e-9);
    assert!(logarithme(0.0).is_nan());
    assert!(logarithme(-1.0).is_nan());
}
