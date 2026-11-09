//! Tests d'acceptation de l'étape 9 (M0, mesurer la pièce), partie « la carte de densité ».
//!
//! Ils fixent `Carte` et `carte_de_densite` : pour chaque case, la longueur d'hyphe qu'elle contient.
//!
//! - **À la main**, avec des valeurs connues : un segment dans une seule case (sa longueur exacte), un segment horizontal ou
//!   vertical sur plusieurs cases avec des longueurs partielles, dans les deux sens ; des diagonales à 45° qui passent par
//!   des coins (trois cases en diagonale, `√2` chacune), depuis des coins et depuis des centres de case, dans les quatre
//!   directions ; une pente 1/2 qui passe par des coins ; un segment qui **longe** une ligne de grille (la case du côté du
//!   plancher), qui **finit** sur une ligne ou sur le bord haut, qui **commence** sur le bord haut, qui longe le bord haut ;
//!   un segment de longueur nulle ; les paniques (chaque coordonnée, chaque côté, NaN, infinis, une dimension nulle).
//! - **Contre des oracles indépendants** (écrits dans `etape_9/outils.rs`, sans couper les segments aux lignes de grille) :
//!   des fractions exactes pour des extrémités en huitièmes de case, la méthode des dalles en `f64`, et l'échantillonnage du
//!   segment en N points très fins pour des extrémités quelconques. Une case est **exactement nulle** là où l'oracle exact
//!   (ou les dalles) le dit : un coin ne laisse rien.
//! - **Des invariants** : la somme des cases est la somme des longueurs (relative 1e-12), aucune case négative, la carte
//!   inchangée quand on renverse le segment ou quand on échange `x` et `y` (carte transposée), la somme de `Carte::somme`
//!   faite dans l'ordre de `valeurs()` (au bit près), les apports ajoutés dans l'ordre des segments (au bit près).
//! - **Les cases occupées à un seuil** (`Carte::occupees`) : strictement au-dessus du seuil (le seuil posé sur la valeur
//!   exacte d'une case l'exclut, un ulp dessous la garde), la moindre longueur occupe sa case au seuil nul, un seuil qui
//!   coupe une ligne en deux morceaux et sa traversée, la monotonie quand le seuil monte, les seuils refusés.
//! - **Les traits** que la spécification donne aux types publics.
//!
//! Ils laissent ouvert : tout ce qui touche à la simulation (voir `etape_9_mesures.rs`).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `sqrt`, `floor`, `min`, `max` et
//! `clamp`.

#[path = "etape_9/outils.rs"]
mod outils;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::croissance::Segment;
use mycelium_lab::piece::{Bord, Carte, MesuresPiece, carte_de_densite, morceaux, traversee};
use outils::{
    Hasard, delai, densite_exacte, densite_par_dalles, densite_par_echantillons, faire_la_carte, juste_sous, longueur,
    seg, segments_en_huitiemes, somme_a_l_envers, somme_dans_l_ordre, somme_des_longueurs,
};

/// La racine de 2, écrite par `sqrt` (exacte à l'arrondi près, la même partout).
fn racine_de_2() -> f64 {
    2.0f64.sqrt()
}

/// Vrai si l'appel panique.
fn panique<F: FnOnce()>(f: F) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

/// Vérifie une carte entière : chaque case de `attendu` à `tolerance` près, **toutes les autres exactement à `0.0`**.
fn verifier(carte: &Carte, attendu: &[((usize, usize), f64)], tolerance: f64, contexte: &str) {
    for y in 0..carte.hauteur() {
        for x in 0..carte.largeur() {
            let obtenu = carte.valeur(x, y);
            match attendu.iter().find(|((i, j), _)| (*i, *j) == (x, y)) {
                Some((_, v)) => assert!(
                    (obtenu - v).abs() <= tolerance,
                    "{contexte} : la case ({x}, {y}) vaut {obtenu} au lieu de {v}"
                ),
                None => assert!(obtenu == 0.0, "{contexte} : la case ({x}, {y}) vaut {obtenu} au lieu de 0"),
            }
        }
    }
}

/// La carte d'un seul segment dans une boîte de `largeur × hauteur`.
fn carte_d_un_segment(largeur: usize, hauteur: usize, s: Segment) -> Carte {
    faire_la_carte(largeur, hauteur, &[s])
}

const PRECISION: f64 = 1e-12;

// ---------------------------------------------------------------------------------------------------------------------
// Carte
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_carte_neuve_est_a_zero_et_garde_sa_forme() {
    delai();
    for (largeur, hauteur) in [(1, 1), (5, 3), (3, 5), (1, 7), (7, 1)] {
        let carte = Carte::nouvelle(largeur, hauteur);
        assert_eq!((carte.largeur(), carte.hauteur()), (largeur, hauteur));
        assert_eq!(carte.valeurs().len(), largeur * hauteur);
        assert!(carte.valeurs().iter().all(|&v| v == 0.0), "toutes les cases d'une carte neuve valent 0");
        assert_eq!(carte.somme(), 0.0);
        for y in 0..hauteur {
            for x in 0..largeur {
                assert_eq!(carte.valeur(x, y), 0.0);
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_carte_sans_case_est_refusee() {
    delai();
    assert!(panique(|| drop(Carte::nouvelle(0, 5))), "largeur nulle");
    assert!(panique(|| drop(Carte::nouvelle(5, 0))), "hauteur nulle");
    assert!(panique(|| drop(Carte::nouvelle(0, 0))), "les deux nulles");
    assert!(panique(|| drop(carte_de_densite(0, 4, &[]))), "carte_de_densite : largeur nulle");
    assert!(panique(|| drop(carte_de_densite(4, 0, &[]))), "carte_de_densite : hauteur nulle");
    assert!(!panique(|| drop(Carte::nouvelle(1, 1))), "une case suffit");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn valeur_refuse_une_case_hors_de_la_boite() {
    delai();
    // 5 × 3 : `valeur(5, 0)` aurait un indice valide (c'est `(0, 1)`), seul un contrôle de la colonne l'arrête.
    let carte = carte_d_un_segment(5, 3, seg(0.0, 1.5, 1.0, 1.5));
    assert!(carte.valeur(0, 1) > 0.0);
    assert!(
        !panique(|| {
            carte.valeur(4, 2);
        }),
        "la dernière case existe"
    );
    assert!(
        panique(|| {
            carte.valeur(5, 0);
        }),
        "x = largeur ne glisse pas sur la ligne suivante"
    );
    assert!(
        panique(|| {
            carte.valeur(5, 1);
        }),
        "x = largeur, y du milieu"
    );
    assert!(
        panique(|| {
            carte.valeur(0, 3);
        }),
        "y = hauteur"
    );
    assert!(
        panique(|| {
            carte.valeur(4, 3);
        }),
        "y = hauteur, x au bout"
    );
    assert!(
        panique(|| {
            carte.valeur(5, 3);
        }),
        "les deux"
    );
    assert!(
        panique(|| {
            carte.valeur(100, 0);
        }),
        "très loin en x"
    );
    assert!(
        panique(|| {
            carte.valeur(0, 100);
        }),
        "très loin en y"
    );
    assert!(
        panique(|| {
            carte.valeur(usize::MAX, 0);
        }),
        "l'indice maximal"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_valeurs_sont_rangees_ligne_par_ligne() {
    delai();
    // 5 × 4 : le segment est dans la case (3, 1), à l'indice 1 × 5 + 3 = 8 de `valeurs()`.
    let carte = carte_d_un_segment(5, 4, seg(3.25, 1.5, 3.75, 1.5));
    assert_eq!(carte.largeur(), 5);
    assert_eq!(carte.hauteur(), 4);
    assert_eq!(carte.valeurs().len(), 20);
    assert_eq!(carte.valeurs()[8], 0.5);
    assert_eq!(carte.valeur(3, 1), 0.5);
    assert_eq!(carte.valeurs().iter().filter(|&&v| v != 0.0).count(), 1);
    // Une carte pleine de valeurs variées : `valeur(x, y)` est `valeurs()[y * largeur + x]` partout.
    let mut h = Hasard::new(5);
    let segments: Vec<Segment> =
        (0..40).map(|_| seg(h.unite() * 6.0, h.unite() * 4.0, h.unite() * 6.0, h.unite() * 4.0)).collect();
    let carte = faire_la_carte(6, 4, &segments);
    for y in 0..4 {
        for x in 0..6 {
            assert_eq!(carte.valeur(x, y).to_bits(), carte.valeurs()[y * 6 + x].to_bits());
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_somme_se_fait_dans_l_ordre_des_valeurs() {
    delai();
    // Une carte aux valeurs variées : la somme faite à l'envers n'a pas les mêmes bits (garde : sans cela, ce test ne
    // verrait pas un changement d'ordre).
    let mut h = Hasard::new(77);
    let segments: Vec<Segment> =
        (0..300).map(|_| seg(h.unite() * 9.0, h.unite() * 7.0, h.unite() * 9.0, h.unite() * 7.0)).collect();
    let carte = faire_la_carte(9, 7, &segments);
    let dans_l_ordre = somme_dans_l_ordre(carte.valeurs());
    assert_ne!(
        dans_l_ordre.to_bits(),
        somme_a_l_envers(carte.valeurs()).to_bits(),
        "la carte du test est trop uniforme pour distinguer deux ordres de somme"
    );
    assert_eq!(carte.somme().to_bits(), dans_l_ordre.to_bits(), "Carte::somme additionne dans l'ordre de valeurs()");
    // Et la somme n'est ni vide ni faussée.
    assert!(proche_de_la_somme(carte.somme(), somme_des_longueurs(&segments)));
}

fn proche_de_la_somme(a: f64, b: f64) -> bool {
    outils::proche_relatif(a, b, 1e-12)
}

// ---------------------------------------------------------------------------------------------------------------------
// À la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_dans_une_case_lui_apporte_sa_longueur_exacte() {
    delai();
    // Horizontal, vertical, diagonal, dans la case (1, 2) d'une boîte 4 × 4, puis dans le coin (0, 0) et dans la dernière case.
    let diagonale = (0.5f64 * 0.5 + 0.5 * 0.5).sqrt();
    let cas: [(Segment, (usize, usize), f64); 8] = [
        (seg(1.25, 2.5, 1.75, 2.5), (1, 2), 0.5),
        (seg(1.75, 2.5, 1.25, 2.5), (1, 2), 0.5),
        (seg(1.5, 2.25, 1.5, 2.75), (1, 2), 0.5),
        (seg(1.5, 2.75, 1.5, 2.25), (1, 2), 0.5),
        (seg(1.25, 2.25, 1.75, 2.75), (1, 2), diagonale),
        (seg(1.75, 2.25, 1.25, 2.75), (1, 2), diagonale),
        (seg(0.25, 0.25, 0.75, 0.25), (0, 0), 0.5),
        (seg(3.25, 3.25, 3.75, 3.75), (3, 3), diagonale),
    ];
    for (s, (x, y), valeur) in cas {
        let carte = carte_d_un_segment(4, 4, s);
        assert_eq!(carte.valeur(x, y).to_bits(), valeur.to_bits(), "{s:?} : la longueur entière dans la case");
        verifier(&carte, &[((x, y), valeur)], 0.0, &format!("{s:?}"));
    }
    // Sur une boîte qui n'est pas carrée, la colonne et la ligne ne se confondent pas.
    let carte = carte_d_un_segment(7, 2, seg(5.5, 1.25, 5.5, 1.75));
    verifier(&carte, &[((5, 1), 0.5)], 0.0, "boîte 7 × 2");
    let carte = carte_d_un_segment(2, 7, seg(1.25, 5.5, 1.75, 5.5));
    verifier(&carte, &[((1, 5), 0.5)], 0.0, "boîte 2 × 7");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn horizontal_sur_plusieurs_cases_dans_les_deux_sens() {
    delai();
    // De 0,5 à 3,25 sur la ligne 1 d'une boîte 5 × 3 : 0,5 dans la colonne 0, 1 dans les colonnes 1 et 2, 0,25 dans la 3.
    let attendu = [((0, 1), 0.5), ((1, 1), 1.0), ((2, 1), 1.0), ((3, 1), 0.25)];
    verifier(&carte_d_un_segment(5, 3, seg(0.5, 1.5, 3.25, 1.5)), &attendu, PRECISION, "aller");
    verifier(&carte_d_un_segment(5, 3, seg(3.25, 1.5, 0.5, 1.5)), &attendu, PRECISION, "retour");
    // Un segment qui part de la dernière colonne et qui finit dans la première.
    let attendu = [((0, 0), 0.75), ((1, 0), 1.0), ((2, 0), 1.0), ((3, 0), 1.0), ((4, 0), 0.75)];
    verifier(&carte_d_un_segment(5, 3, seg(4.75, 0.5, 0.25, 0.5)), &attendu, PRECISION, "de bout en bout");
    verifier(&carte_d_un_segment(5, 3, seg(0.25, 0.5, 4.75, 0.5)), &attendu, PRECISION, "de bout en bout, aller");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn vertical_sur_plusieurs_cases_dans_les_deux_sens() {
    delai();
    // De 0,25 à 3,5 dans la colonne 3 d'une boîte 5 × 4 : 0,75 en ligne 0, 1 en lignes 1 et 2, 0,5 en ligne 3.
    let attendu = [((3, 0), 0.75), ((3, 1), 1.0), ((3, 2), 1.0), ((3, 3), 0.5)];
    verifier(&carte_d_un_segment(5, 4, seg(3.5, 0.25, 3.5, 3.5)), &attendu, PRECISION, "descente");
    verifier(&carte_d_un_segment(5, 4, seg(3.5, 3.5, 3.5, 0.25)), &attendu, PRECISION, "montée");
    // Sur une boîte haute et étroite.
    let attendu = [((1, 4), 0.5), ((1, 5), 1.0), ((1, 6), 0.25)];
    verifier(&carte_d_un_segment(2, 8, seg(1.9, 4.5, 1.9, 6.25)), &attendu, PRECISION, "boîte 2 × 8");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn diagonale_a_45_degres_par_les_coins_trois_cases_de_racine_de_2() {
    delai();
    let r = racine_de_2();
    // De (0, 0) à (3, 3) : elle passe par les coins (1, 1) et (2, 2) ; trois cases en diagonale, √2 chacune, rien ailleurs.
    let attendu = [((0, 0), r), ((1, 1), r), ((2, 2), r)];
    verifier(&carte_d_un_segment(5, 4, seg(0.0, 0.0, 3.0, 3.0)), &attendu, PRECISION, "descente vers la droite");
    verifier(&carte_d_un_segment(5, 4, seg(3.0, 3.0, 0.0, 0.0)), &attendu, PRECISION, "remontée vers la gauche");
    // L'autre diagonale : de (3, 0) à (0, 3), cases (2, 0), (1, 1), (0, 2).
    let attendu = [((2, 0), r), ((1, 1), r), ((0, 2), r)];
    verifier(&carte_d_un_segment(5, 4, seg(3.0, 0.0, 0.0, 3.0)), &attendu, PRECISION, "descente vers la gauche");
    verifier(&carte_d_un_segment(5, 4, seg(0.0, 3.0, 3.0, 0.0)), &attendu, PRECISION, "remontée vers la droite");
    // Depuis le bord haut de la boîte (coordonnées égales à la largeur et à la hauteur) : de (5, 4) à (2, 1).
    let attendu = [((4, 3), r), ((3, 2), r), ((2, 1), r)];
    verifier(&carte_d_un_segment(5, 4, seg(5.0, 4.0, 2.0, 1.0)), &attendu, PRECISION, "depuis le coin haut");
    verifier(&carte_d_un_segment(5, 4, seg(2.0, 1.0, 5.0, 4.0)), &attendu, PRECISION, "vers le coin haut");
    // Une diagonale de coin à coin d'une boîte carrée.
    let attendu: Vec<((usize, usize), f64)> = (0..4).map(|i| ((i, i), r)).collect();
    verifier(&carte_d_un_segment(4, 4, seg(0.0, 0.0, 4.0, 4.0)), &attendu, PRECISION, "toute la diagonale");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn diagonale_a_45_degres_depuis_des_centres_de_case() {
    delai();
    let r = racine_de_2();
    // De (0,5; 0,5) à (3,5; 3,5) : la moitié d'une diagonale, trois diagonales entières, la moitié de la dernière.
    let attendu = [((0, 0), r / 2.0), ((1, 1), r), ((2, 2), r), ((3, 3), r / 2.0)];
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 0.5, 3.5, 3.5)), &attendu, PRECISION, "aller");
    verifier(&carte_d_un_segment(5, 4, seg(3.5, 3.5, 0.5, 0.5)), &attendu, PRECISION, "retour");
    // L'autre pente : de (3,5; 0,5) à (0,5; 3,5).
    let attendu = [((3, 0), r / 2.0), ((2, 1), r), ((1, 2), r), ((0, 3), r / 2.0)];
    verifier(&carte_d_un_segment(5, 4, seg(3.5, 0.5, 0.5, 3.5)), &attendu, PRECISION, "autre pente, aller");
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 3.5, 3.5, 0.5)), &attendu, PRECISION, "autre pente, retour");
    // Un pas de 1 à 45° depuis un centre de case (ce que fait la simulation) passe par le coin : (0,5; 0,5) → (1,2; 1,2).
    let fin = 0.5 + std::f64::consts::FRAC_1_SQRT_2;
    let s = seg(0.5, 0.5, fin, fin);
    let carte = carte_d_un_segment(3, 3, s);
    assert!(proche(carte.valeur(0, 0), r / 2.0) && proche(carte.valeur(1, 1), 1.0 - r / 2.0));
    assert!(proche_de_la_somme(carte.somme(), longueur(&s)));
}

fn proche(a: f64, b: f64) -> bool {
    outils::proche(a, b, 1e-12)
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_pente_une_demie_passe_par_des_coins_en_quatre_parts_egales() {
    delai();
    // De (0, 0) à (4, 2) : la droite y = x / 2 passe par les coins (2, 1) ; quatre morceaux égaux, de longueur √20 / 4.
    let part = 20.0f64.sqrt() / 4.0;
    let attendu = [((0, 0), part), ((1, 0), part), ((2, 1), part), ((3, 1), part)];
    verifier(&carte_d_un_segment(5, 3, seg(0.0, 0.0, 4.0, 2.0)), &attendu, PRECISION, "aller");
    verifier(&carte_d_un_segment(5, 3, seg(4.0, 2.0, 0.0, 0.0)), &attendu, PRECISION, "retour");
    // Même pente, sur l'axe des y : de (0, 0) à (2, 4), cases (0, 0), (0, 1), (1, 2), (1, 3).
    let attendu = [((0, 0), part), ((0, 1), part), ((1, 2), part), ((1, 3), part)];
    verifier(&carte_d_un_segment(3, 5, seg(0.0, 0.0, 2.0, 4.0)), &attendu, PRECISION, "pente 2");
    verifier(&carte_d_un_segment(3, 5, seg(2.0, 4.0, 0.0, 0.0)), &attendu, PRECISION, "pente 2, retour");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_qui_longe_une_ligne_de_grille_est_dans_la_case_du_plancher() {
    delai();
    // x = 3,0 de y = 0,5 à y = 2,5 : la colonne 3 (et rien dans la colonne 2).
    let attendu = [((3, 0), 0.5), ((3, 1), 1.0), ((3, 2), 0.5)];
    verifier(&carte_d_un_segment(5, 4, seg(3.0, 0.5, 3.0, 2.5)), &attendu, PRECISION, "x = 3, descente");
    verifier(&carte_d_un_segment(5, 4, seg(3.0, 2.5, 3.0, 0.5)), &attendu, PRECISION, "x = 3, montée");
    // y = 2,0 de x = 0,5 à x = 3,5 : la ligne 2 (et rien dans la ligne 1).
    let attendu = [((0, 2), 0.5), ((1, 2), 1.0), ((2, 2), 1.0), ((3, 2), 0.5)];
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 2.0, 3.5, 2.0)), &attendu, PRECISION, "y = 2, aller");
    verifier(&carte_d_un_segment(5, 4, seg(3.5, 2.0, 0.5, 2.0)), &attendu, PRECISION, "y = 2, retour");
    // Un segment qui part d'un coin entier et longe la ligne : (3, 1) → (3, 3), cases (3, 1) et (3, 2) ; (1, 2) → (4, 2).
    verifier(&carte_d_un_segment(5, 4, seg(3.0, 1.0, 3.0, 3.0)), &[((3, 1), 1.0), ((3, 2), 1.0)], PRECISION, "entier");
    let attendu = [((1, 2), 1.0), ((2, 2), 1.0), ((3, 2), 1.0)];
    verifier(&carte_d_un_segment(5, 4, seg(1.0, 2.0, 4.0, 2.0)), &attendu, PRECISION, "entier horizontal");
    // La première ligne de grille, x = 0 : la colonne 0.
    verifier(&carte_d_un_segment(5, 4, seg(0.0, 0.5, 0.0, 1.5)), &[((0, 0), 0.5), ((0, 1), 0.5)], PRECISION, "x = 0");
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 0.0, 1.5, 0.0)), &[((0, 0), 0.5), ((1, 0), 0.5)], PRECISION, "y = 0");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_qui_finit_sur_une_ligne_ne_deborde_pas_dans_la_case_suivante() {
    delai();
    // Fini sur x = 2 : la case de la colonne 2 ne reçoit rien.
    verifier(&carte_d_un_segment(5, 3, seg(0.5, 0.5, 2.0, 0.5)), &[((0, 0), 0.5), ((1, 0), 1.0)], PRECISION, "x = 2");
    verifier(
        &carte_d_un_segment(5, 3, seg(2.0, 0.5, 0.5, 0.5)),
        &[((0, 0), 0.5), ((1, 0), 1.0)],
        PRECISION,
        "x = 2, retour",
    );
    // Fini sur y = 2.
    verifier(&carte_d_un_segment(5, 4, seg(1.5, 0.25, 1.5, 2.0)), &[((1, 0), 0.75), ((1, 1), 1.0)], PRECISION, "y = 2");
    // Diagonale qui finit exactement sur le coin (2, 2) : le coin ne donne rien à la case (2, 2).
    let r = racine_de_2();
    let attendu = [((0, 0), r / 2.0), ((1, 1), r)];
    verifier(&carte_d_un_segment(4, 4, seg(0.5, 0.5, 2.0, 2.0)), &attendu, PRECISION, "coin (2, 2)");
    // Segment qui finit sur le bord haut de la boîte : ni panique ni case de trop.
    let attendu = [((0, 0), 0.5), ((0, 1), 1.0), ((0, 2), 1.0), ((0, 3), 1.0)];
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 0.5, 0.5, 4.0)), &attendu, PRECISION, "jusqu'à y = hauteur");
    let attendu = [((0, 0), 0.5), ((1, 0), 1.0), ((2, 0), 1.0), ((3, 0), 1.0), ((4, 0), 1.0)];
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 0.5, 5.0, 0.5)), &attendu, PRECISION, "jusqu'à x = largeur");
    // Fini sur le coin haut de la boîte : la diagonale entière, jusqu'à (5, 4).
    let attendu = [((1, 0), 1.0), ((2, 1), 1.0), ((3, 2), 1.0), ((4, 3), 1.0)];
    verifier(
        &carte_d_un_segment(5, 4, seg(1.0, 0.0, 5.0, 4.0)),
        &attendu.map(|(c, v)| (c, v * r)),
        PRECISION,
        "jusqu'au coin",
    );
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_qui_commence_sur_le_bord_haut_va_dans_la_derniere_case() {
    delai();
    verifier(
        &carte_d_un_segment(5, 4, seg(5.0, 0.5, 3.5, 0.5)),
        &[((4, 0), 1.0), ((3, 0), 0.5)],
        PRECISION,
        "x = largeur",
    );
    verifier(
        &carte_d_un_segment(5, 4, seg(0.5, 4.0, 0.5, 2.5)),
        &[((0, 3), 1.0), ((0, 2), 0.5)],
        PRECISION,
        "y = hauteur",
    );
    let r = racine_de_2();
    verifier(&carte_d_un_segment(3, 3, seg(3.0, 3.0, 2.0, 2.0)), &[((2, 2), r)], PRECISION, "depuis le coin haut");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_le_long_du_bord_haut_est_dans_la_derniere_colonne_ou_ligne() {
    delai();
    // Le contrat accepte `x = largeur` : la case du plancher serait la colonne `largeur`, qui n'existe pas ; le segment est au
    // bord de la dernière colonne, et c'est elle qui le reçoit (sinon la somme des cases perdrait des longueurs).
    let attendu = [((4, 0), 0.5), ((4, 1), 1.0), ((4, 2), 0.5)];
    verifier(&carte_d_un_segment(5, 4, seg(5.0, 0.5, 5.0, 2.5)), &attendu, PRECISION, "x = largeur");
    verifier(&carte_d_un_segment(5, 4, seg(5.0, 2.5, 5.0, 0.5)), &attendu, PRECISION, "x = largeur, retour");
    let attendu = [((0, 3), 0.5), ((1, 3), 1.0), ((2, 3), 0.5)];
    verifier(&carte_d_un_segment(5, 4, seg(0.5, 4.0, 2.5, 4.0)), &attendu, PRECISION, "y = hauteur");
    // Le long du bord, jusqu'au coin.
    verifier(&carte_d_un_segment(5, 4, seg(5.0, 3.0, 5.0, 4.0)), &[((4, 3), 1.0)], PRECISION, "jusqu'au coin");
    // Un segment d'un ulp de la borne, dont le milieu s'arrondit sur la borne, reste dans la boîte.
    let juste_avant = f64::from_bits(5.0f64.to_bits() - 1);
    let carte = carte_d_un_segment(5, 4, seg(juste_avant, 1.5, 5.0, 1.5));
    assert!(carte.valeur(4, 1) > 0.0 && proche_de_la_somme(carte.somme(), 5.0 - juste_avant));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_qui_frole_un_coin_laisse_sa_longueur_minuscule_dans_la_case_du_coin() {
    delai();
    // De (0 ; 0,5) à (2 ; 1,5 + 2e-13) : la droite passe y = 1 juste avant x = 1 (à t = 0,5 − 1e-13 environ), puis x = 1 à
    // t = 0,5 ; entre les deux, un morceau de 2,2e-13 dans la case (0, 1). Il compte, si petit soit-il.
    let s = seg(0.0, 0.5, 2.0, 1.5 + 2e-13);
    let carte = carte_d_un_segment(3, 3, s);
    let dalles = densite_par_dalles(3, 3, &[s]);
    assert!(dalles[3] > 0.0 && dalles[3] < 1e-12, "garde : les dalles voient le morceau ({})", dalles[3]);
    assert!(carte.valeur(0, 1) > 0.0 && carte.valeur(0, 1) < 1e-12, "la case (0, 1) vaut {}", carte.valeur(0, 1));
    for (k, (obtenu, attendu)) in carte.valeurs().iter().zip(&dalles).enumerate() {
        assert!((obtenu - attendu).abs() <= 1e-12, "case {k}");
        assert_eq!(*obtenu == 0.0, *attendu == 0.0, "case {k}");
    }
    assert_eq!(carte.occupees(0.0), vec![true, false, false, true, true, false, false, false, false]);
    // Le même, renversé et transposé.
    let carte = carte_d_un_segment(3, 3, seg(2.0, 1.5 + 2e-13, 0.0, 0.5));
    assert!(carte.valeur(0, 1) > 0.0 && carte.valeur(0, 1) < 1e-12);
    let carte = carte_d_un_segment(3, 3, seg(0.5, 0.0, 1.5 + 2e-13, 2.0));
    assert!(carte.valeur(1, 0) > 0.0 && carte.valeur(1, 0) < 1e-12);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_passage_par_un_coin_a_pente_quelconque_ne_laisse_rien_aux_cases_du_coin() {
    delai();
    // (0, 0) → (1,125 ; 5,625) passe exactement par le coin (1, 5) : `t = (1 − 0) / 1,125` et `t = (5 − 0) / 5,625` sont la
    // même fraction (8/9), donc le même `f64` ; écrit autrement (`(n − c0) × (1 / d)`), les deux instants diffèrent d'un ulp
    // et un morceau minuscule tombe dans une case que le segment ne traverse pas. Dans les quatre sens, et transposé.
    let en_huitiemes: [[i64; 4]; 4] = [[0, 0, 9, 45], [0, 0, 10, 70], [0, 0, 18, 45], [0, 0, 20, 70]];
    for [x0, y0, x1, y1] in en_huitiemes {
        let (largeur, hauteur) = (3usize, 9usize);
        let variantes = [
            ([x0, y0, x1, y1], largeur, hauteur),
            ([x1, y1, x0, y0], largeur, hauteur),
            ([8 * 3 - x0, y0, 8 * 3 - x1, y1], largeur, hauteur),
            ([x0, 8 * 9 - y0, x1, 8 * 9 - y1], largeur, hauteur),
            ([y0, x0, y1, x1], hauteur, largeur),
        ];
        for (s, l, h) in variantes {
            let exacte = densite_exacte(l, h, &[s]);
            let carte = faire_la_carte(l, h, &segments_en_huitiemes(&[s]));
            for (k, (obtenu, attendu)) in carte.valeurs().iter().zip(&exacte).enumerate() {
                assert!((obtenu - attendu).abs() <= 1e-12, "{s:?} : case {k}, {obtenu} au lieu de {attendu}");
                assert_eq!(*obtenu == 0.0, *attendu == 0.0, "{s:?} : case {k}, {obtenu} au lieu de {attendu}");
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_dans_une_case_apporte_sqrt_de_dx2_plus_dy2_au_bit_pres() {
    delai();
    // Des segments quelconques tenus dans une case : la case reçoit `sqrt(dx * dx + dy * dy)`, exactement.
    let mut h = Hasard::new(8080);
    for _ in 0..2000 {
        let (i, j) = (h.jusqu_a(4) as f64, h.jusqu_a(3) as f64);
        let s = seg(i + h.unite(), j + h.unite(), i + h.unite(), j + h.unite());
        let carte = carte_d_un_segment(4, 3, s);
        let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
        assert_eq!(carte.valeur(i as usize, j as usize).to_bits(), (dx * dx + dy * dy).sqrt().to_bits(), "{s:?}");
        assert_eq!(carte.valeurs().iter().filter(|&&v| v != 0.0).count(), usize::from(dx != 0.0 || dy != 0.0));
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_segment_de_longueur_nulle_ne_contribue_a_rien() {
    delai();
    for (x, y) in [(1.5, 1.5), (2.0, 2.0), (0.0, 0.0), (5.0, 4.0), (5.0, 1.5), (1.5, 4.0), (3.0, 0.25)] {
        let carte = carte_d_un_segment(5, 4, seg(x, y, x, y));
        assert!(carte.valeurs().iter().all(|&v| v == 0.0), "({x}, {y}) : un point ne laisse rien");
        assert_eq!(carte.somme(), 0.0);
    }
    // Parmi d'autres segments, il ne change rien (au bit près).
    let sans = faire_la_carte(5, 4, &[seg(0.5, 0.5, 3.5, 2.5)]);
    let avec = faire_la_carte(5, 4, &[seg(1.0, 1.0, 1.0, 1.0), seg(0.5, 0.5, 3.5, 2.5), seg(4.0, 3.0, 4.0, 3.0)]);
    for (a, b) in sans.valeurs().iter().zip(avec.valeurs()) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_longueurs_s_additionnent_case_par_case() {
    delai();
    // Deux segments dans la même case : 0,5 + 0,25 ; un troisième dans une autre case.
    let carte =
        faire_la_carte(4, 3, &[seg(1.0, 1.25, 1.5, 1.25), seg(1.5, 1.75, 1.75, 1.75), seg(3.25, 2.5, 3.75, 2.5)]);
    verifier(&carte, &[((1, 1), 0.75), ((3, 2), 0.5)], 0.0, "trois segments");
    // Le même segment deux fois : le double.
    let un = faire_la_carte(5, 3, &[seg(0.5, 1.5, 3.25, 1.5)]);
    let deux = faire_la_carte(5, 3, &[seg(0.5, 1.5, 3.25, 1.5), seg(0.5, 1.5, 3.25, 1.5)]);
    for (a, b) in un.valeurs().iter().zip(deux.valeurs()) {
        assert_eq!(2.0 * a, *b, "le double, exactement (l'addition de deux valeurs égales)");
    }
    // Deux segments qui se croisent dans une case : la case reçoit les deux longueurs.
    let carte = faire_la_carte(3, 3, &[seg(1.25, 1.5, 1.75, 1.5), seg(1.5, 1.25, 1.5, 1.75)]);
    verifier(&carte, &[((1, 1), 1.0)], 0.0, "croisement");
    // Une liste vide.
    let vide = faire_la_carte(6, 2, &[]);
    assert!(vide.valeurs().iter().all(|&v| v == 0.0) && vide.largeur() == 6 && vide.hauteur() == 2);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_coordonnee_hors_de_la_boite_fait_paniquer() {
    delai();
    let (l, h) = (5usize, 4usize);
    let au_dessus_de_5 = f64::from_bits(5.0f64.to_bits() + 1);
    let au_dessus_de_4 = f64::from_bits(4.0f64.to_bits() + 1);
    let sous_zero = -f64::MIN_POSITIVE;
    // Pour chacune des quatre coordonnées, chaque mauvaise valeur fait paniquer, les trois autres étant bonnes.
    for position in 0..4 {
        let mauvaises: Vec<f64> = if position % 2 == 0 {
            vec![au_dessus_de_5, 5.5, 6.0, 1e9, sous_zero, -0.5, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
        } else {
            vec![au_dessus_de_4, 4.5, 5.0, 1e9, sous_zero, -0.5, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
        };
        for mauvaise in mauvaises {
            let mut c = [1.5, 1.5, 2.5, 2.5];
            c[position] = mauvaise;
            assert!(
                panique(|| drop(carte_de_densite(l, h, &[seg(c[0], c[1], c[2], c[3])]))),
                "la coordonnée {position} = {mauvaise} est hors de [0, {l}] × [0, {h}]"
            );
        }
    }
    // Un seul segment fautif, au milieu d'autres bons ou au bout de la liste, suffit.
    let bon = seg(0.5, 0.5, 1.5, 1.5);
    assert!(panique(|| drop(carte_de_densite(l, h, &[bon, seg(1.0, 1.0, 6.0, 1.0), bon]))));
    assert!(panique(|| drop(carte_de_densite(l, h, &[bon, bon, seg(1.0, 1.0, 1.0, f64::NAN)]))));
    assert!(panique(|| drop(carte_de_densite(l, h, &[seg(-1.0, 1.0, 1.0, 1.0), bon]))));
    // La largeur ne protège pas la hauteur : un `y` de 4,5 est bon dans une boîte de 5 de large, pas de 4 de haut.
    assert!(panique(|| drop(carte_de_densite(5, 4, &[seg(1.0, 4.5, 1.0, 1.0)]))));
    assert!(!panique(|| drop(carte_de_densite(4, 5, &[seg(1.0, 4.5, 1.0, 1.0)]))));
    assert!(panique(|| drop(carte_de_densite(4, 5, &[seg(4.5, 1.0, 1.0, 1.0)]))));
    assert!(!panique(|| drop(carte_de_densite(5, 4, &[seg(4.5, 1.0, 1.0, 1.0)]))));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_bornes_de_la_boite_sont_permises_zero_negatif_compris() {
    delai();
    let juste_avant_5 = f64::from_bits(5.0f64.to_bits() - 1);
    let mut tous = Vec::new();
    for x in [0.0, -0.0, juste_avant_5, 5.0] {
        for y in [0.0, -0.0, 3.5, 4.0] {
            tous.push(seg(x, y, 5.0 - x, 4.0 - y));
            tous.push(seg(5.0 - x, 4.0 - y, x, y));
        }
    }
    for s in &tous {
        let carte = carte_d_un_segment(5, 4, *s);
        assert!(proche_de_la_somme(carte.somme(), longueur(s)), "{s:?} : la somme des cases est la longueur");
    }
    let carte = faire_la_carte(5, 4, &tous);
    assert!(proche_de_la_somme(carte.somme(), somme_des_longueurs(&tous)));
}

// ---------------------------------------------------------------------------------------------------------------------
// Les oracles indépendants
// ---------------------------------------------------------------------------------------------------------------------

/// Une coordonnée en huitièmes de case, dans `[0, 8 limite]` : souvent entière ou demi-entière, pour tomber sur les
/// lignes de grille.
fn coordonnee_en_huitiemes(h: &mut Hasard, limite: usize) -> i64 {
    match h.jusqu_a(4) {
        0 => 8 * h.jusqu_a(limite + 1) as i64,
        1 => 4 * h.jusqu_a(2 * limite + 1) as i64,
        _ => h.jusqu_a(8 * limite + 1) as i64,
    }
}

/// Un segment en huitièmes dans la boîte : tantôt quelconque, tantôt vertical, horizontal ou à 45° (qui passe par des coins).
fn segment_en_huitiemes(h: &mut Hasard, largeur: usize, hauteur: usize) -> [i64; 4] {
    let (max_x, max_y) = (8 * largeur as i64, 8 * hauteur as i64);
    let x0 = coordonnee_en_huitiemes(h, largeur);
    let y0 = coordonnee_en_huitiemes(h, hauteur);
    let mut x1 = coordonnee_en_huitiemes(h, largeur);
    let mut y1 = coordonnee_en_huitiemes(h, hauteur);
    match h.jusqu_a(6) {
        0 => x1 = x0,
        1 => y1 = y0,
        2 => {
            // 45° : on essaie un dx, avec les deux signes de dy, et on garde ce qui reste dans la boîte.
            let dx = x1 - x0;
            let y_haut = y0 + dx.abs();
            let y_bas = y0 - dx.abs();
            if h.jusqu_a(2) == 0 && y_haut <= max_y {
                y1 = y_haut;
            } else if y_bas >= 0 {
                y1 = y_bas;
            } else if y_haut <= max_y {
                y1 = y_haut;
            }
        }
        _ => {}
    }
    assert!((0..=max_x).contains(&x1) && (0..=max_y).contains(&y1));
    [x0, y0, x1, y1]
}

/// Les formes de boîte essayées : carrées, larges, hautes, d'une seule ligne ou colonne.
const BOITES: [(usize, usize); 9] = [(1, 1), (2, 1), (1, 3), (3, 2), (2, 5), (5, 4), (8, 8), (4, 9), (9, 3)];

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn chaque_segment_a_la_carte_exacte_en_huitiemes() {
    delai();
    let mut h = Hasard::new(2024);
    for (largeur, hauteur) in BOITES {
        for _ in 0..500 {
            let s = segment_en_huitiemes(&mut h, largeur, hauteur);
            let exacte = densite_exacte(largeur, hauteur, &[s]);
            let carte = faire_la_carte(largeur, hauteur, &segments_en_huitiemes(&[s]));
            let l = longueur(&segments_en_huitiemes(&[s])[0]);
            for (k, (obtenu, attendu)) in carte.valeurs().iter().zip(&exacte).enumerate() {
                assert!(
                    (obtenu - attendu).abs() <= 1e-12 * l.max(1.0),
                    "boîte {largeur} × {hauteur}, segment {s:?} (en huitièmes) : la case ({}, {}) vaut {obtenu} au lieu de {attendu}",
                    k % largeur,
                    k / largeur
                );
                // Exactement nulle là où le segment ne passe pas (un coin ne laisse rien) : c'est ce que compte une case
                // occupée au seuil nul.
                assert_eq!(
                    *obtenu == 0.0,
                    *attendu == 0.0,
                    "boîte {largeur} × {hauteur}, segment {s:?} (en huitièmes) : la case ({}, {}) vaut {obtenu}, exactement {attendu}",
                    k % largeur,
                    k / largeur
                );
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn plusieurs_segments_ont_la_carte_exacte_en_huitiemes() {
    delai();
    let mut h = Hasard::new(99);
    for (largeur, hauteur) in BOITES {
        for _ in 0..40 {
            let nombre = 1 + h.jusqu_a(25);
            let s: Vec<[i64; 4]> = (0..nombre).map(|_| segment_en_huitiemes(&mut h, largeur, hauteur)).collect();
            let exacte = densite_exacte(largeur, hauteur, &s);
            let segments = segments_en_huitiemes(&s);
            let carte = faire_la_carte(largeur, hauteur, &segments);
            let total = somme_des_longueurs(&segments);
            for (k, (obtenu, attendu)) in carte.valeurs().iter().zip(&exacte).enumerate() {
                assert!(
                    (obtenu - attendu).abs() <= 1e-12 * total.max(1.0),
                    "boîte {largeur} × {hauteur}, {nombre} segments : la case ({}, {}) vaut {obtenu} au lieu de {attendu}",
                    k % largeur,
                    k / largeur
                );
                assert_eq!(
                    *obtenu == 0.0,
                    *attendu == 0.0,
                    "boîte {largeur} × {hauteur}, {nombre} segments : la case ({}, {}) vaut {obtenu}, exactement {attendu}",
                    k % largeur,
                    k / largeur
                );
            }
        }
    }
}

/// Une coordonnée quelconque dans `[0, limite]`, souvent calée sur un entier ou sur un bord.
fn coordonnee_quelconque(h: &mut Hasard, limite: usize) -> f64 {
    match h.jusqu_a(6) {
        0 => h.jusqu_a(limite + 1) as f64,
        1 => limite as f64,
        _ => h.unite() * limite as f64,
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn des_segments_quelconques_suivent_l_echantillonnage_et_les_dalles() {
    delai();
    const POINTS: usize = 6000;
    let mut h = Hasard::new(31);
    for (largeur, hauteur) in BOITES {
        for _ in 0..60 {
            let s = seg(
                coordonnee_quelconque(&mut h, largeur),
                coordonnee_quelconque(&mut h, hauteur),
                coordonnee_quelconque(&mut h, largeur),
                coordonnee_quelconque(&mut h, hauteur),
            );
            let carte = carte_d_un_segment(largeur, hauteur, s);
            let echantillons = densite_par_echantillons(largeur, hauteur, &[s], POINTS);
            let dalles = densite_par_dalles(largeur, hauteur, &[s]);
            let l = longueur(&s);
            for k in 0..largeur * hauteur {
                let obtenu = carte.valeurs()[k];
                assert!(
                    (obtenu - dalles[k]).abs() <= 1e-9,
                    "{s:?} dans {largeur} × {hauteur} : la case ({}, {}) vaut {obtenu}, les dalles disent {}",
                    k % largeur,
                    k / largeur,
                    dalles[k]
                );
                assert_eq!(
                    obtenu == 0.0,
                    dalles[k] == 0.0,
                    "{s:?} dans {largeur} × {hauteur} : la case ({}, {}) vaut {obtenu}, les dalles disent {}",
                    k % largeur,
                    k / largeur,
                    dalles[k]
                );
                assert!(
                    (obtenu - echantillons[k]).abs() <= 3.0 * l / POINTS as f64 + 1e-9,
                    "{s:?} dans {largeur} × {hauteur} : la case ({}, {}) vaut {obtenu}, l'échantillonnage dit {}",
                    k % largeur,
                    k / largeur,
                    echantillons[k]
                );
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_somme_des_cases_est_la_somme_des_longueurs_et_aucune_case_n_est_negative() {
    delai();
    let mut h = Hasard::new(8);
    for (largeur, hauteur) in BOITES {
        for tour in 0..60 {
            let nombre = 1 + h.jusqu_a(40);
            let segments: Vec<Segment> = if tour % 2 == 0 {
                (0..nombre)
                    .map(|_| {
                        seg(
                            coordonnee_quelconque(&mut h, largeur),
                            coordonnee_quelconque(&mut h, hauteur),
                            coordonnee_quelconque(&mut h, largeur),
                            coordonnee_quelconque(&mut h, hauteur),
                        )
                    })
                    .collect()
            } else {
                let huitiemes: Vec<[i64; 4]> =
                    (0..nombre).map(|_| segment_en_huitiemes(&mut h, largeur, hauteur)).collect();
                segments_en_huitiemes(&huitiemes)
            };
            let carte = faire_la_carte(largeur, hauteur, &segments);
            let total = somme_des_longueurs(&segments);
            assert!(
                outils::proche_relatif(carte.somme(), total, 1e-12) || (carte.somme() == 0.0 && total == 0.0),
                "boîte {largeur} × {hauteur} : la somme des cases {} n'est pas la somme des longueurs {total}",
                carte.somme()
            );
            assert!(carte.valeurs().iter().all(|&v| v >= 0.0 && v.is_finite()), "aucune case négative ni infinie");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn renverser_un_segment_ou_echanger_x_et_y_ne_change_pas_la_carte() {
    delai();
    let mut h = Hasard::new(4);
    for (largeur, hauteur) in BOITES {
        for _ in 0..80 {
            let nombre = 1 + h.jusqu_a(8);
            let huitiemes: Vec<[i64; 4]> =
                (0..nombre).map(|_| segment_en_huitiemes(&mut h, largeur, hauteur)).collect();
            let segments = segments_en_huitiemes(&huitiemes);
            let carte = faire_la_carte(largeur, hauteur, &segments);
            let total = somme_des_longueurs(&segments);

            // Le segment à l'envers : la même carte.
            let renverses: Vec<Segment> = segments.iter().map(|s| seg(s.x1, s.y1, s.x0, s.y0)).collect();
            let a_l_envers = faire_la_carte(largeur, hauteur, &renverses);
            for (a, b) in carte.valeurs().iter().zip(a_l_envers.valeurs()) {
                assert!((a - b).abs() <= 1e-12 * total.max(1.0), "à l'envers : {a} contre {b}");
            }

            // Les axes échangés, dans la boîte transposée : la carte transposée.
            let transposes: Vec<Segment> = segments.iter().map(|s| seg(s.y0, s.x0, s.y1, s.x1)).collect();
            let transposee = faire_la_carte(hauteur, largeur, &transposes);
            for y in 0..hauteur {
                for x in 0..largeur {
                    let (a, b) = (carte.valeur(x, y), transposee.valeur(y, x));
                    assert!((a - b).abs() <= 1e-12 * total.max(1.0), "transposée en ({x}, {y}) : {a} contre {b}");
                }
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_carte_est_deterministe_au_bit_pres() {
    delai();
    let mut h = Hasard::new(12);
    let segments: Vec<Segment> =
        (0..200).map(|_| seg(h.unite() * 12.0, h.unite() * 7.0, h.unite() * 12.0, h.unite() * 7.0)).collect();
    let a = faire_la_carte(12, 7, &segments);
    let b = faire_la_carte(12, 7, &segments);
    for (x, y) in a.valeurs().iter().zip(b.valeurs()) {
        assert_eq!(x.to_bits(), y.to_bits());
    }
    assert_eq!(a.somme().to_bits(), b.somme().to_bits());
}

// ---------------------------------------------------------------------------------------------------------------------
// L'ordre des additions
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_apports_s_ajoutent_dans_l_ordre_des_segments() {
    delai();
    // Avec des extrémités en huitièmes, un segment n'apporte à une case qu'un seul morceau : la carte de plusieurs segments
    // est donc, case par case et au bit près, `((0 + a1) + a2) + a3 …`, où `ai` est la carte du segment `i` seul.
    let mut h = Hasard::new(606);
    let mut ordre_visible = 0;
    for (largeur, hauteur) in [(3usize, 2usize), (4, 4), (2, 5)] {
        for _ in 0..200 {
            let nombre = 3 + h.jusqu_a(10);
            let huitiemes: Vec<[i64; 4]> =
                (0..nombre).map(|_| segment_en_huitiemes(&mut h, largeur, hauteur)).collect();
            let segments = segments_en_huitiemes(&huitiemes);
            let seuls: Vec<Carte> = segments.iter().map(|s| faire_la_carte(largeur, hauteur, &[*s])).collect();
            let carte = faire_la_carte(largeur, hauteur, &segments);
            for k in 0..largeur * hauteur {
                let mut dans_l_ordre = 0.0;
                let mut a_l_envers = 0.0;
                for i in 0..nombre {
                    dans_l_ordre += seuls[i].valeurs()[k];
                    a_l_envers += seuls[nombre - 1 - i].valeurs()[k];
                }
                assert_eq!(
                    carte.valeurs()[k].to_bits(),
                    dans_l_ordre.to_bits(),
                    "boîte {largeur} × {hauteur}, segments {huitiemes:?} : la case {k} n'additionne pas dans l'ordre des segments"
                );
                ordre_visible += usize::from(dans_l_ordre.to_bits() != a_l_envers.to_bits());
            }
        }
    }
    // Garde : des cases où l'ordre change le résultat, sans quoi le test ne verrait pas un autre ordre.
    assert!(ordre_visible > 20, "garde : seulement {ordre_visible} cases sensibles à l'ordre des additions");
}

// ---------------------------------------------------------------------------------------------------------------------
// Les cases occupées à un seuil
// ---------------------------------------------------------------------------------------------------------------------

/// Une ligne de cinq cases (ligne 1 d'une boîte 5 × 3) de densités 1, 1, 0,25, 1, 1 : la case du milieu est faible.
fn ligne_au_milieu_faible() -> Carte {
    faire_la_carte(5, 3, &[seg(0.0, 1.5, 2.0, 1.5), seg(2.25, 1.5, 2.5, 1.5), seg(3.0, 1.5, 5.0, 1.5)])
}

/// Les indices des cases occupées.
fn indices(occupees: &[bool]) -> Vec<usize> {
    occupees.iter().enumerate().filter(|(_, o)| **o).map(|(i, _)| i).collect()
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_cases_occupees_sont_strictement_au_dessus_du_seuil() {
    delai();
    let carte = ligne_au_milieu_faible();
    verifier(&carte, &[((0, 1), 1.0), ((1, 1), 1.0), ((2, 1), 0.25), ((3, 1), 1.0), ((4, 1), 1.0)], 0.0, "la ligne");
    // Au seuil nul, les cinq cases ; au seuil 0,25 (la valeur exacte de la case du milieu), elle n'y est plus.
    assert_eq!(indices(&carte.occupees(0.0)), vec![5, 6, 7, 8, 9]);
    assert_eq!(indices(&carte.occupees(juste_sous(0.25))), vec![5, 6, 7, 8, 9], "un ulp sous 0,25");
    assert_eq!(indices(&carte.occupees(0.25)), vec![5, 6, 8, 9], "0,25 n'est pas strictement au-dessus de 0,25");
    assert_eq!(indices(&carte.occupees(0.5)), vec![5, 6, 8, 9]);
    assert_eq!(indices(&carte.occupees(juste_sous(1.0))), vec![5, 6, 8, 9], "un ulp sous 1");
    assert_eq!(indices(&carte.occupees(1.0)), Vec::<usize>::new(), "1 n'est pas strictement au-dessus de 1");
    assert_eq!(indices(&carte.occupees(1e300)), Vec::<usize>::new());
    // Une tranche par case, dans l'ordre de `valeurs()`.
    for seuil in [0.0, 0.1, 0.25, 0.9, 1.0, 7.0] {
        let occupees = carte.occupees(seuil);
        assert_eq!(occupees.len(), 15, "seuil {seuil} : une valeur par case");
        for (k, &o) in occupees.iter().enumerate() {
            assert_eq!(o, carte.valeurs()[k] > seuil, "seuil {seuil}, case {k}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_seuil_coupe_la_ligne_en_deux_morceaux_et_la_traversee() {
    delai();
    let carte = ligne_au_milieu_faible();
    // (seuil, morceaux, traversée gauche-droite)
    let cas: [(f64, &[usize], Option<usize>); 6] = [
        (0.0, &[5], Some(5)),
        (juste_sous(0.25), &[5], Some(5)),
        (0.25, &[2, 2], None),
        (0.75, &[2, 2], None),
        (juste_sous(1.0), &[2, 2], None),
        (1.0, &[], None),
    ];
    for (seuil, attendus, gauche_droite) in cas {
        let occupees = carte.occupees(seuil);
        assert_eq!(morceaux(5, 3, &occupees), attendus, "seuil {seuil}");
        assert_eq!(traversee(5, 3, &occupees, Bord::Gauche, Bord::Droite), gauche_droite, "seuil {seuil}");
        assert_eq!(traversee(5, 3, &occupees, Bord::Haut, Bord::Bas), None, "seuil {seuil}");
    }
    // Une diagonale dense et un segment faible à l'écart : au-dessus de son seuil, la diagonale reste d'un seul tenant.
    let r = racine_de_2();
    let carte = faire_la_carte(4, 4, &[seg(0.0, 0.0, 4.0, 4.0), seg(0.25, 3.5, 0.5, 3.5)]);
    verifier(&carte, &[((0, 0), r), ((1, 1), r), ((2, 2), r), ((3, 3), r), ((0, 3), 0.25)], PRECISION, "diagonale");
    assert_eq!(morceaux(4, 4, &carte.occupees(0.0)), vec![4, 1]);
    assert_eq!(morceaux(4, 4, &carte.occupees(0.25)), vec![4]);
    assert_eq!(traversee(4, 4, &carte.occupees(0.25), Bord::Gauche, Bord::Droite), Some(4));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn au_seuil_nul_la_moindre_longueur_occupe_la_case() {
    delai();
    // Un segment d'un ulp (2,2e-16) et un autre de 1e-12 : leurs cases sont occupées au seuil nul, pas au seuil 1e-9.
    let un_ulp = f64::from_bits(1.5f64.to_bits() + 1);
    let carte = faire_la_carte(4, 3, &[seg(1.5, 1.5, un_ulp, 1.5), seg(3.5, 2.5, 3.5, 2.5 + 1e-12)]);
    assert!(carte.valeur(1, 1) > 0.0 && carte.valeur(1, 1) < 1e-15, "{}", carte.valeur(1, 1));
    assert!(carte.valeur(3, 2) > 0.0 && carte.valeur(3, 2) < 1e-11, "{}", carte.valeur(3, 2));
    assert_eq!(indices(&carte.occupees(0.0)), vec![5, 11]);
    assert_eq!(indices(&carte.occupees(-0.0)), vec![5, 11], "-0 est le seuil nul");
    assert_eq!(indices(&carte.occupees(1e-15)), vec![11]);
    assert_eq!(indices(&carte.occupees(1e-9)), Vec::<usize>::new());
    assert_eq!(morceaux(4, 3, &carte.occupees(0.0)), vec![1, 1]);
    // Une densité de 1e-150 occupe encore sa case au seuil nul. (Un segment de 1e-170 n'a, lui, aucune longueur :
    // `dx²` s'arrondit à 0 dans `L = sqrt(dx² + dy²)`, il ne contribue à rien.)
    let carte = faire_la_carte(2, 1, &[seg(0.0, 0.5, 1e-150, 0.5), seg(1.5, 0.5, 1.5 + 1e-15, 0.5)]);
    assert!(carte.valeur(0, 0) > 0.0 && carte.valeur(0, 0) <= 1e-149);
    assert_eq!(carte.occupees(0.0), vec![true, true]);
    assert_eq!(carte.occupees(1e-140), vec![false, true]);
    let carte = faire_la_carte(2, 1, &[seg(0.0, 0.5, 1e-170, 0.5)]);
    assert_eq!(carte.valeurs(), &[0.0, 0.0]);
    assert_eq!(carte.occupees(0.0), vec![false, false]);
    // Une carte neuve n'a aucune case occupée, même au seuil nul.
    assert!(Carte::nouvelle(3, 2).occupees(0.0).iter().all(|&o| !o));
    assert_eq!(Carte::nouvelle(3, 2).occupees(0.0).len(), 6);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_seuil_negatif_ou_non_fini_fait_paniquer() {
    delai();
    let carte = ligne_au_milieu_faible();
    for mauvais in [-1.0, -0.25, -f64::MIN_POSITIVE, -f64::from_bits(1), f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(panique(|| drop(carte.occupees(mauvais))), "le seuil {mauvais} est refusé");
    }
    for bon in [0.0, -0.0, f64::from_bits(1), 0.25, 1.0, f64::MAX] {
        assert!(!panique(|| drop(carte.occupees(bon))), "le seuil {bon} est permis");
    }
    assert!(carte.occupees(f64::MAX).iter().all(|&o| !o));
    assert_eq!(carte.occupees(f64::MAX).len(), 15);
}

/// Un segment court au hasard dans la boîte (au plus une case de long sur chaque axe).
fn segment_court(h: &mut Hasard, largeur: usize, hauteur: usize) -> Segment {
    let (l, ht) = (largeur as f64, hauteur as f64);
    let (x, y) = (h.unite() * l, h.unite() * ht);
    let x1 = (x + h.unite() * 2.0 - 1.0).clamp(0.0, l);
    let y1 = (y + h.unite() * 2.0 - 1.0).clamp(0.0, ht);
    seg(x, y, x1, y1)
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn elever_le_seuil_ne_fait_que_retirer_des_cases() {
    delai();
    // Sur des cartes au hasard, des seuils croissants (dont les valeurs exactes des cases et un ulp dessous) : les cases
    // occupées d'un seuil plus haut sont parmi celles d'un seuil plus bas ; leur nombre et le plus grand morceau ne
    // croissent jamais ; une traversée ne naît jamais, et ne raccourcit jamais.
    let mut h = Hasard::new(1312);
    let mut retraits = 0;
    for (largeur, hauteur) in [(6usize, 5usize), (9, 3), (3, 8)] {
        for _ in 0..30 {
            let nombre = 5 + h.jusqu_a(30);
            let segments: Vec<Segment> = (0..nombre).map(|_| segment_court(&mut h, largeur, hauteur)).collect();
            let carte = faire_la_carte(largeur, hauteur, &segments);
            let mut seuils = vec![0.0];
            for &v in carte.valeurs() {
                if v > 0.0 {
                    seuils.push(v);
                    seuils.push(juste_sous(v));
                }
            }
            seuils.sort_by(f64::total_cmp);
            let mut avant: Option<(Vec<bool>, usize, [Option<usize>; 2])> = None;
            for &seuil in &seuils {
                let occupees = carte.occupees(seuil);
                let plus_grand = morceaux(largeur, hauteur, &occupees).first().copied().unwrap_or(0);
                let traversees = [
                    traversee(largeur, hauteur, &occupees, Bord::Gauche, Bord::Droite),
                    traversee(largeur, hauteur, &occupees, Bord::Haut, Bord::Bas),
                ];
                if let Some((precedentes, grand, traversees_avant)) = &avant {
                    for k in 0..occupees.len() {
                        assert!(
                            !occupees[k] || precedentes[k],
                            "seuil {seuil} : la case {k} apparaît en montant le seuil"
                        );
                    }
                    assert!(plus_grand <= *grand, "seuil {seuil} : le plus grand morceau grandit");
                    for (apres, avant) in traversees.iter().zip(traversees_avant) {
                        match (avant, apres) {
                            (None, Some(n)) => panic!("seuil {seuil} : une traversée de {n} cases naît"),
                            (Some(a), Some(b)) => assert!(b >= a, "seuil {seuil} : une traversée raccourcit"),
                            _ => {}
                        }
                    }
                    let compte = |o: &[bool]| o.iter().filter(|&&c| c).count();
                    retraits += usize::from(compte(&occupees) < compte(precedentes));
                }
                avant = Some((occupees, plus_grand, traversees));
            }
        }
    }
    assert!(retraits > 100, "garde : seulement {retraits} hausses de seuil ont retiré une case");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_types_publics_ont_leurs_traits() {
    delai();
    // La spécification donne à `Carte` et à `MesuresPiece` : Clone, Debug, PartialEq ; à `Bord` : Clone, Copy, Debug,
    // PartialEq, Eq.
    fn clone_debug_egal<T: Clone + std::fmt::Debug + PartialEq>(_: &T) {}
    fn copie_egale<T: Copy + Eq + std::fmt::Debug>(_: &T) {}
    let carte = ligne_au_milieu_faible();
    clone_debug_egal(&carte);
    assert_eq!(carte.clone(), carte);
    assert_ne!(Carte::nouvelle(5, 3), carte);
    assert_ne!(Carte::nouvelle(3, 5), Carte::nouvelle(5, 3), "la forme compte dans l'égalité");
    let bord = Bord::Haut;
    copie_egale(&bord);
    assert_eq!(bord, Bord::Haut);
    assert_ne!(Bord::Gauche, Bord::Droite);
    let mesures = MesuresPiece {
        cases_libres: 1,
        cases_occupees: 0,
        remplissage: 0.0,
        densite_moyenne: 0.0,
        densite_maximale: 0.0,
        biomasse: 0.0,
        nourriture_initiale: 1.0,
        nourriture_restante: 1.0,
        nourriture_consommee: 0.0,
        ecart_du_bilan: 0.0,
        plus_grand_morceau: 0,
        nombre_de_morceaux: 0,
        traversee_gauche_droite: None,
        traversee_haut_bas: Some(1),
    };
    clone_debug_egal(&mesures);
    assert_eq!(mesures.clone(), mesures);
}
