//! Tests d'acceptation de l'étape 7 (T2, les obstacles durs), partie substrat : `Substrat::segment_libre` et
//! `Substrat::face_de_contact`.
//!
//! Ils fixent **la règle du plancher** : chaque point `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)` ; un segment fermé est libre si
//! aucun de ses points n'est dans une case bloquée. Donc : un champ sans case bloquée est libre pour tout segment ; un
//! segment qui traverse une case bloquée, y part ou y finit n'est pas libre ; une arrivée posée exactement sur la face
//! gauche ou basse d'une case bloquée n'est pas libre, sur la face droite ou haute elle l'est ; longer exactement la face
//! gauche ou basse n'est pas libre, la face droite ou haute l'est ; passer pile par le coin bas-gauche (indices les plus
//! petits) d'une case bloquée la touche, par un autre coin non ; le bord droit et bas de la boîte fermée est admis ; hors de
//! la boîte fermée ou non fini, c'est une panique (contrat de `Index::ajouter`), et `face_de_contact` panique aussi pour un
//! départ dans une case bloquée. Ils fixent **la face d'un contact** : celle de la ligne de la grille franchie pour entrer
//! dans la première case bloquée touchée ; par un coin, la règle des deux voisines. Le tout est comparé, sur 40 000 segments
//! aux extrémités multiples de 1/16 (dans les deux sens), à l'oracle exact de `outils.rs` (instants d'entrée en fractions
//! d'entiers, case par case, sans parcours de grille).
//!
//! Ils laissent ouverts : le texte des paniques ; le résultat pour des coordonnées à l'ULP près d'une ligne de la grille
//! quand ce ne sont pas des flottants exacts (l'arrondi des instants calculés ; aucun test n'en dépend).
//!
//! Ils n'utilisent aucune fonction mathématique de `f64` sauf `floor` et `min`/`max` (dans `outils.rs`).

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::alea::Alea;
use mycelium_lab::substrat::Substrat;
use mycelium_lab::tropisme::Face;
use outils::{Contact, Grille};

/// Un champ de `l × h` cases, sans diffusion, avec les rectangles bloqués donnés `(x0, y0, x1, y1)`.
fn champ(l: usize, h: usize, rectangles: &[(usize, usize, usize, usize)]) -> Substrat {
    let mut s = Substrat::nouveau(l, h, 0.0, 1.0, 1.0).expect("champ valide");
    for &(x0, y0, x1, y1) in rectangles {
        s.bloquer(x0, y0, x1, y1);
    }
    s
}

/// Exige que `f` panique.
fn exiger_la_panique(nom: &str, f: impl FnOnce() -> bool) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok() {
        panic!("{nom} : l'appel aurait dû paniquer");
    }
}

/// 1. Sans case bloquée, tout segment de la boîte est libre : 2 000 segments tirés au hasard, y compris longs, verticaux,
///    horizontaux, réduits à un point et sur les bords de la boîte fermée.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn sans_case_bloquee_tout_segment_est_libre() {
    outils::delai();
    let s = champ(12, 9, &[]);
    let mut alea = Alea::new(1);
    for _ in 0..2000 {
        let mut pris = [0.0; 4];
        for (k, v) in pris.iter_mut().enumerate() {
            let borne = if k % 2 == 0 { 12.0 } else { 9.0 };
            *v = alea.suivant_f64() * borne;
        }
        assert!(s.segment_libre(pris[0], pris[1], pris[2], pris[3]), "segment {pris:?}");
    }
    assert!(s.segment_libre(0.0, 0.0, 12.0, 9.0), "la diagonale de la boîte fermée");
    assert!(s.segment_libre(3.0, 0.0, 3.0, 9.0), "le long d'une ligne de la grille");
    assert!(s.segment_libre(4.5, 4.5, 4.5, 4.5), "un point");
}

/// 2. Un segment qui rencontre l'intérieur d'une case bloquée n'est pas libre, quelle que soit sa direction ; un segment qui
///    passe à une case de là l'est. Case bloquée `(5, 4)` dans une boîte 10 × 10.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn traverser_l_interieur_d_une_case_bloquee_n_est_pas_libre() {
    outils::delai();
    let s = champ(10, 10, &[(5, 4, 5, 4)]);
    // (départ, arrivée, libre ?)
    type Cas = ((f64, f64), (f64, f64), bool);
    let cas: [Cas; 14] = [
        ((5.5, 0.5), (5.5, 9.5), false),    // verticale, en travers
        ((0.5, 4.5), (9.5, 4.5), false),    // horizontale, en travers
        ((3.5, 2.5), (7.5, 6.5), false),    // diagonale par le centre
        ((7.5, 6.5), (3.5, 2.5), false),    // la même, à l'envers
        ((5.2, 3.0), (5.2, 3.9), true),     // finit juste avant la case
        ((5.2, 3.5), (5.2, 4.1), false),    // finit juste dedans
        ((5.2, 5.0), (5.2, 6.0), true),     // part pile du bord bas de la case, vers le bas
        ((4.5, 0.5), (4.5, 9.5), true),     // verticale à côté
        ((6.5, 0.5), (6.5, 9.5), true),     // verticale de l'autre côté
        ((0.5, 3.5), (9.5, 3.5), true),     // horizontale au-dessus
        ((0.5, 5.5), (9.5, 5.5), true),     // horizontale en dessous
        ((4.1, 3.1), (5.9, 4.9), false),    // petite diagonale qui coupe la case de coin à coin
        ((4.1, 3.9), (4.9, 3.1), true),     // petite diagonale d'une case voisine
        ((5.99, 3.0), (5.99, 4.01), false), // finit tout juste dedans, près du bord droit
    ];
    for (k, (a, b, libre)) in cas.into_iter().enumerate() {
        assert_eq!(s.segment_libre(a.0, a.1, b.0, b.1), libre, "cas {k} : {a:?} -> {b:?}");
        assert_eq!(s.segment_libre(b.0, b.1, a.0, a.1), libre, "cas {k} à l'envers : {b:?} -> {a:?}");
    }
}

/// 3. Une extrémité dans une case bloquée n'est pas libre : départ dedans, arrivée dedans, segment réduit à un point dedans
///    (libre pour un point dans une case libre).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn une_extremite_dans_une_case_bloquee_n_est_pas_libre() {
    outils::delai();
    let s = champ(10, 10, &[(5, 4, 6, 5)]);
    assert!(!s.segment_libre(5.5, 4.5, 2.5, 2.5), "départ dans la case bloquée");
    assert!(!s.segment_libre(2.5, 2.5, 6.5, 5.5), "arrivée dans la case bloquée");
    assert!(!s.segment_libre(6.5, 5.5, 6.5, 5.5), "un point dans la case bloquée");
    assert!(s.segment_libre(2.5, 2.5, 2.5, 2.5), "un point dans une case libre");
    assert!(!s.segment_libre(5.5, 4.5, 6.5, 5.5), "tout le segment dans la paroi");
}

/// 4. Une arrivée posée exactement sur la face basse (ou gauche) d'une case bloquée est dans cette case au sens du plancher :
///    pas libre ; un cheveu avant, libre ; posée exactement sur la face haute (ou droite), libre (l'extrémité est dans la
///    case libre, et l'intérieur de la case bloquée n'est pas rencontré). Case bloquée `(5, 5)` dans une boîte 10 × 10.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn une_arrivee_sur_la_face_basse_ou_gauche_d_une_case_bloquee_n_est_pas_libre() {
    outils::delai();
    let s = champ(10, 10, &[(5, 5, 5, 5)]);
    // Par le haut : arrivée sur y = 5.0 (face haute de la case, dans l'indexation des lignes qui croissent vers le bas) ;
    // cette face est la ligne d'indice 5, donc celle de la case bloquée.
    assert!(!s.segment_libre(5.5, 3.5, 5.5, 5.0), "arrivée exactement sur y = 5");
    assert!(s.segment_libre(5.5, 3.5, 5.5, 4.999999999999999), "arrivée un ULP avant");
    // Par la gauche.
    assert!(!s.segment_libre(3.5, 5.5, 5.0, 5.5), "arrivée exactement sur x = 5");
    assert!(s.segment_libre(3.5, 5.5, 4.999999999999999, 5.5), "arrivée un ULP avant");
    // Par le bas et par la droite, la face est celle de la case libre voisine : l'arrivée est libre.
    assert!(s.segment_libre(5.5, 7.5, 5.5, 6.0), "arrivée exactement sur y = 6, par le bas");
    assert!(s.segment_libre(7.5, 5.5, 6.0, 5.5), "arrivée exactement sur x = 6, par la droite");
    assert!(!s.segment_libre(5.5, 7.5, 5.5, 5.999999999999999), "un ULP dans la case");
}

/// 5. Le passage exact par un coin suit la règle du plancher : le point du coin `(X, Y)` est dans la case `(X, Y)`. Passer
///    pile par le coin bas-gauche (au sens des indices : `x` et `y` les plus petits) d'une case bloquée la touche donc ;
///    passer pile par un de ses trois autres coins ne la touche pas. Les coordonnées sont des multiples de 1/2 : tout est
///    exact.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_passage_par_un_coin_suit_la_regle_du_plancher() {
    outils::delai();
    let s = champ(10, 10, &[(5, 5, 5, 5)]);
    // Coin (5, 5) : le point du coin est dans la case bloquée.
    assert!(!s.segment_libre(4.5, 5.5, 5.5, 4.5), "anti-diagonale par le coin (5, 5)");
    assert!(!s.segment_libre(5.5, 4.5, 4.5, 5.5), "la même, à l'envers");
    // Les trois autres coins : (6, 6), (6, 5), (5, 6) sont dans des cases libres.
    assert!(s.segment_libre(5.5, 6.5, 6.5, 5.5), "anti-diagonale par le coin (6, 6)");
    assert!(s.segment_libre(6.5, 5.5, 5.5, 6.5), "la même, à l'envers");
    assert!(s.segment_libre(5.5, 4.5, 6.5, 5.5), "diagonale par le coin (6, 5)");
    assert!(s.segment_libre(4.5, 5.5, 5.5, 6.5), "diagonale par le coin (5, 6)");
    // La diagonale qui entre par le coin (5, 5) dans la case la traverse.
    assert!(!s.segment_libre(4.5, 4.5, 6.5, 6.5), "diagonale qui entre par le coin");
    assert!(!s.segment_libre(6.5, 6.5, 4.5, 4.5), "la même, à l'envers");
    // Damier (4, 5) et (5, 4) bloquées : la diagonale par le coin commun (5, 5) passe entre elles (le point du coin est
    // dans la case (5, 5), libre).
    let damier = champ(10, 10, &[(4, 5, 4, 5), (5, 4, 5, 4)]);
    assert!(damier.segment_libre(4.5, 4.5, 5.5, 5.5), "entre deux cases bloquées en damier");
    assert!(damier.segment_libre(3.5, 3.5, 6.5, 6.5), "long passage par le coin du damier");
    // L'autre damier, (4, 4) et (5, 5) : l'anti-diagonale par le coin commun touche (5, 5).
    let autre = champ(10, 10, &[(4, 4, 4, 4), (5, 5, 5, 5)]);
    assert!(!autre.segment_libre(4.5, 5.5, 5.5, 4.5), "le coin commun est dans la case (5, 5)");
}

/// 6. Longer exactement une face : le long de la face droite ou haute d'une paroi (côté des indices croissants), les points
///    sont dans les cases libres voisines : libre ; le long de la face gauche ou basse, ils sont dans la paroi : pas libre,
///    même si les deux extrémités sont dans des cases libres. Paroi : colonnes 5, lignes 2 à 7.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn longer_une_face_suit_la_regle_du_plancher() {
    outils::delai();
    let s = champ(10, 10, &[(5, 2, 5, 7)]);
    assert!(s.segment_libre(6.0, 1.5, 6.0, 8.5), "le long de la face droite, d'un bout à l'autre");
    assert!(s.segment_libre(6.0, 3.5, 6.0, 6.5), "le long de la face droite, au milieu");
    assert!(!s.segment_libre(5.0, 1.5, 5.0, 8.5), "le long de la face gauche, extrémités libres");
    assert!(!s.segment_libre(5.0, 3.5, 5.0, 6.5), "sur la ligne x = 5, dans la colonne 5");
    assert!(!s.segment_libre(5.5, 1.5, 5.5, 8.5), "dans la colonne 5, en travers de la paroi");
    let plafond = champ(10, 10, &[(2, 5, 7, 5)]);
    assert!(plafond.segment_libre(1.5, 6.0, 8.5, 6.0), "le long de la face haute (y = 6)");
    assert!(!plafond.segment_libre(1.5, 5.0, 8.5, 5.0), "le long de la face basse (y = 5)");
}

/// 7. Le bord droit et le bord bas de la boîte fermée sont admis (c'est la ligne ou la colonne d'indice `largeur` ou
///    `hauteur`, hors grille, jamais bloquée) ; un segment sur le bord ne rencontre que les cases du bord.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_bords_de_la_boite_fermee_sont_admis() {
    outils::delai();
    let s = champ(10, 10, &[(9, 3, 9, 6)]);
    assert!(!s.segment_libre(5.5, 5.5, 10.0, 5.5), "traverse la colonne 9 bloquée jusqu'au bord");
    assert!(s.segment_libre(5.5, 8.5, 10.0, 8.5), "jusqu'au bord, en dessous de la paroi");
    assert!(s.segment_libre(10.0, 0.0, 10.0, 10.0), "le long du bord droit");
    assert!(s.segment_libre(0.0, 10.0, 10.0, 10.0), "le long du bord bas");
    assert!(s.segment_libre(10.0, 10.0, 10.0, 10.0), "le coin bas droit, réduit à un point");
}

/// 8. Hors de la boîte fermée `[0, largeur] × [0, hauteur]`, ou avec une coordonnée non finie, `segment_libre` panique (le
///    contrat de l'`Index` d'anastomose, repris).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn hors_de_la_boite_ou_non_fini_c_est_une_panique() {
    outils::delai();
    let s = champ(10, 8, &[(4, 4, 4, 4)]);
    assert!(s.segment_libre(0.0, 0.0, 10.0, 8.0), "garde : la boîte fermée est admise");
    for (nom, a, b, c, d) in [
        ("x0 négatif", -0.001, 1.0, 2.0, 2.0),
        ("y0 négatif", 1.0, -0.001, 2.0, 2.0),
        ("x1 trop grand", 1.0, 1.0, 10.001, 2.0),
        ("y1 trop grand", 1.0, 1.0, 2.0, 8.001),
        ("x0 NaN", f64::NAN, 1.0, 2.0, 2.0),
        ("y1 infini", 1.0, 1.0, 2.0, f64::INFINITY),
        ("x1 -infini", 1.0, 1.0, f64::NEG_INFINITY, 2.0),
    ] {
        exiger_la_panique(nom, || s.segment_libre(a, b, c, d));
    }
}

/// 9. `face_de_contact` panique dans les mêmes cas que `segment_libre`, et en plus quand la case de départ est bloquée
///    (le départ d'un contact est toujours la position d'une pointe, dans une case libre).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn face_de_contact_panique_hors_contrat() {
    outils::delai();
    let s = champ(10, 8, &[(4, 4, 4, 4)]);
    assert_eq!(s.face_de_contact(0.0, 0.0, 10.0, 8.0), None, "garde : la boîte fermée est admise");
    for (nom, a, b, c, d) in [
        ("x0 négatif", -0.001, 1.0, 2.0, 2.0),
        ("y1 trop grand", 1.0, 1.0, 2.0, 8.001),
        ("x0 NaN", f64::NAN, 1.0, 2.0, 2.0),
        ("y1 infini", 1.0, 1.0, 2.0, f64::INFINITY),
        ("départ dans la case bloquée", 4.5, 4.5, 1.5, 1.5),
        ("départ sur la face gauche de la case bloquée", 4.0, 4.5, 1.5, 1.5),
    ] {
        exiger_la_panique(nom, || s.face_de_contact(a, b, c, d).is_some());
    }
}

/// 10. La face d'un contact, à la main : par la gauche ou la droite d'un mur, verticale ; par le haut ou le bas,
///     horizontale ; un segment vertical touche toujours une face horizontale, un horizontal une face verticale ; une
///     arrivée posée sur la face gauche ou basse donne cette face ; la première case touchée décide (deux murs sur le
///     trajet) ; un départ posé sur la face droite d'une case bloquée, qui repart vers elle, la touche par cette face.
///     `None` quand le segment est libre.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_face_d_un_contact_a_la_main() {
    outils::delai();
    let v = Some(Face::Verticale);
    let h = Some(Face::Horizontale);
    let s = champ(20, 20, &[(10, 5, 11, 14)]); // un mur : colonnes 10 et 11, lignes 5 à 14
    assert_eq!(s.face_de_contact(8.5, 9.5, 10.5, 10.25), v, "par la gauche");
    assert_eq!(s.face_de_contact(13.5, 9.5, 11.5, 8.75), v, "par la droite");
    assert_eq!(s.face_de_contact(10.75, 3.5, 11.25, 5.5), h, "par le haut (y croissant vers la ligne 5)");
    assert_eq!(s.face_de_contact(10.25, 16.5, 10.75, 14.5), h, "par le bas");
    assert_eq!(s.face_de_contact(10.5, 2.5, 10.5, 18.5), h, "segment vertical");
    assert_eq!(s.face_de_contact(2.5, 9.5, 18.5, 9.5), v, "segment horizontal");
    assert_eq!(s.face_de_contact(8.5, 9.5, 10.0, 9.5), v, "arrivée posée sur la face gauche");
    assert_eq!(s.face_de_contact(10.5, 3.5, 10.5, 5.0), h, "arrivée posée sur la face basse (y = 5)");
    assert_eq!(s.face_de_contact(12.0, 9.5, 11.5, 9.0), v, "départ sur la face droite, vers le mur");
    assert_eq!(s.face_de_contact(12.0, 9.5, 13.5, 9.0), None, "départ sur la face droite, en s'éloignant");
    assert_eq!(s.face_de_contact(8.5, 3.5, 9.5, 4.5), None, "libre");
    // Une diagonale raide qui touche le coin haut du mur par sa face haute, puis une plate qui le touche par la gauche.
    assert_eq!(
        s.face_de_contact(9.75, 3.0, 10.25, 5.5),
        h,
        "diagonale raide : la ligne y = 5 est franchie après x = 10"
    );
    assert_eq!(s.face_de_contact(8.0, 4.75, 10.5, 5.25), v, "diagonale plate : x = 10 est franchie après y = 5");
    // La première case touchée décide : sans pilier, ce segment touche le mur par la gauche ; avec un pilier (8, 6) sur
    // son chemin, il touche d'abord le pilier, par le haut.
    assert_eq!(s.face_de_contact(8.25, 4.5, 10.25, 10.5), v, "sans pilier : le mur, par la gauche");
    let deux = champ(20, 20, &[(8, 6, 8, 6), (10, 5, 11, 14)]);
    assert_eq!(deux.face_de_contact(8.25, 4.5, 10.25, 10.5), h, "le pilier d'abord, par le haut");
}

/// 11. L'entrée exacte par un coin (les deux lignes de la grille franchies au même instant) : on regarde les deux voisines
///     de la case touchée du côté d'où vient le segment ; seule la voisine de la même ligne bloquée → horizontale ; sinon
///     (seule celle de la même colonne, aucune, ou les deux) → verticale. Les huit cas, dans les quatre sens.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn l_entree_par_un_coin_regarde_les_voisines() {
    outils::delai();
    let v = Some(Face::Verticale);
    let h = Some(Face::Horizontale);
    // La case touchée (5, 5), le segment vient de la case diagonale (4, 4) par le coin (5, 5) ; voisines (5, 4) (même
    // colonne) et (4, 5) (même ligne).
    for (murs, attendu, nom) in [
        (vec![(5, 5, 5, 5)], v, "aucune voisine"),
        (vec![(5, 5, 5, 5), (5, 4, 5, 4)], v, "voisine de la même colonne"),
        (vec![(5, 5, 5, 5), (4, 5, 4, 5)], h, "voisine de la même ligne"),
        (vec![(5, 5, 5, 5), (5, 4, 5, 4), (4, 5, 4, 5)], v, "les deux voisines"),
    ] {
        let s = champ(10, 10, &murs);
        assert_eq!(s.face_de_contact(4.5, 4.5, 5.5, 5.5), attendu, "vers les x et y croissants : {nom}");
    }
    // Vers les x et y décroissants : la case touchée (4, 4) depuis (5, 5) par le coin (5, 5) ; voisines (4, 5) (même
    // colonne) et (5, 4) (même ligne).
    for (murs, attendu, nom) in [
        (vec![(4, 4, 4, 4)], v, "aucune voisine"),
        (vec![(4, 4, 4, 4), (4, 5, 4, 5)], v, "voisine de la même colonne"),
        (vec![(4, 4, 4, 4), (5, 4, 5, 4)], h, "voisine de la même ligne"),
    ] {
        let s = champ(10, 10, &murs);
        assert_eq!(s.face_de_contact(5.5, 5.5, 4.5, 4.5), attendu, "vers les x et y décroissants : {nom}");
    }
    // Vers les x croissants et y décroissants, par le coin (5, 5) : le point du coin est dans la case (5, 5), atteinte en
    // franchissant x = 5 alors que y est déjà dans la ligne 5 : verticale. La case (5, 4), entrée ensuite par le coin :
    // voisines (5, 5) (même colonne) et (4, 4) (même ligne).
    let s = champ(10, 10, &[(5, 5, 5, 5)]);
    assert_eq!(s.face_de_contact(4.5, 5.5, 5.5, 4.5), v, "la case du coin, en venant d'en dessous à gauche");
    let s = champ(10, 10, &[(5, 4, 5, 4), (4, 4, 4, 4)]);
    assert_eq!(s.face_de_contact(4.5, 5.5, 5.5, 4.5), h, "la case après le coin, voisine de la même ligne bloquée");
    let s = champ(10, 10, &[(5, 4, 5, 4)]);
    assert_eq!(s.face_de_contact(4.5, 5.5, 5.5, 4.5), v, "la case après le coin, aucune voisine");
    // Un départ posé exactement sur le coin (5, 5), vers les x et y décroissants : il quitte aussitôt sa case (5, 5) par le
    // coin pour entrer dans (4, 4) ; voisines (4, 5) et (5, 4). Une arrivée posée exactement sur le coin (5, 5) en venant
    // de (4, 4) : le point d'arrivée est dans la case (5, 5) ; voisines (5, 4) et (4, 5).
    let s = champ(10, 10, &[(4, 4, 4, 4), (5, 4, 5, 4)]);
    assert_eq!(s.face_de_contact(5.0, 5.0, 4.5, 4.5), h, "départ sur le coin, voisine de la même ligne bloquée");
    let s = champ(10, 10, &[(4, 4, 4, 4), (4, 5, 4, 5)]);
    assert_eq!(s.face_de_contact(5.0, 5.0, 4.5, 4.5), v, "départ sur le coin, voisine de la même colonne bloquée");
    let s = champ(10, 10, &[(5, 5, 5, 5), (4, 5, 4, 5)]);
    assert_eq!(s.face_de_contact(4.5, 4.5, 5.0, 5.0), h, "arrivée sur le coin, voisine de la même ligne bloquée");
    let s = champ(10, 10, &[(5, 5, 5, 5), (5, 4, 5, 4)]);
    assert_eq!(s.face_de_contact(4.5, 4.5, 5.0, 5.0), v, "arrivée sur le coin, voisine de la même colonne bloquée");
    // Vers les x décroissants et y croissants, par le coin (5, 5) : la case du coin (5, 5) est atteinte en franchissant
    // y = 5 alors que x est déjà dans la colonne 5 : horizontale.
    let s = champ(10, 10, &[(5, 5, 5, 5)]);
    assert_eq!(s.face_de_contact(5.5, 4.5, 4.5, 5.5), h, "la case du coin, en venant d'au-dessus à droite");
}

/// Un entier tiré dans `0..=max`.
fn tirer(alea: &mut Alea, max: i64) -> i64 {
    (alea.suivant() % (max as u64 + 1)) as i64
}

/// 12. Sur 40 000 segments aux extrémités multiples de 1/16 dans une boîte 6 × 6, avec 1 à 3 rectangles bloqués tirés au
///     hasard, `segment_libre` et `face_de_contact` sont égaux à l'oracle exact de `outils.rs` (instants d'entrée dans
///     chaque case comparés en fractions d'entiers, sans parcours de grille), dans les deux sens du segment. Les multiples de
///     1/16 sont des flottants exacts et tombent souvent pile sur une ligne ou un coin de la grille. Gardes contre un test
///     vide (mesurées sur le prototype corrigé, voir le rapport du juge) : les deux issues sont fréquentes ; les deux faces
///     sont rendues ; des centaines de segments sont couchés sur une ligne de la grille, et des dizaines entrent par un coin.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn segment_libre_et_face_de_contact_sont_egaux_a_l_oracle_sur_40_000_segments() {
    outils::delai();
    const ECHELLE: i64 = 16;
    let mut alea = Alea::new(20_261_108);
    let (mut libres, mut total, mut verticales, mut horizontales) = (0usize, 0usize, 0usize, 0usize);
    let (mut sur_une_ligne, mut par_un_coin) = (0usize, 0usize);
    for _ in 0..2_000 {
        let n = 1 + tirer(&mut alea, 2);
        let mut murs = Vec::new();
        for _ in 0..n {
            let x0 = tirer(&mut alea, 5);
            let y0 = tirer(&mut alea, 5);
            let x1 = (x0 + tirer(&mut alea, 2)).min(5);
            let y1 = (y0 + tirer(&mut alea, 2)).min(5);
            murs.push((x0 as usize, y0 as usize, x1 as usize, y1 as usize));
        }
        let s = champ(6, 6, &murs);
        let grille = Grille::nouvelle(6, 6, &murs);
        for k in 0..20 {
            let mut p = [0; 4].map(|_| tirer(&mut alea, 6 * ECHELLE) as f64 / ECHELLE as f64);
            // Un segment sur quatre est vertical, un sur quatre horizontal.
            match k % 4 {
                0 => p[2] = p[0],
                1 => p[3] = p[1],
                _ => {}
            }
            for (a, b, c, d) in [(p[0], p[1], p[2], p[3]), (p[2], p[3], p[0], p[1])] {
                let attendu = grille.libre(a, b, c, d);
                assert_eq!(s.segment_libre(a, b, c, d), attendu, "murs {murs:?}, segment ({a}, {b}) -> ({c}, {d})");
                if !grille.bloquee(a.floor() as i64, b.floor() as i64) {
                    let face = match grille.contact(a, b, c, d) {
                        Contact::Libre => None,
                        Contact::Touche { face, .. } => Some(face),
                    };
                    assert_eq!(
                        s.face_de_contact(a, b, c, d),
                        face,
                        "murs {murs:?}, face du segment ({a}, {b}) -> ({c}, {d})"
                    );
                    match face {
                        Some(Face::Verticale) => verticales += 1,
                        Some(Face::Horizontale) => horizontales += 1,
                        None => {}
                    }
                }
                total += 1;
                libres += usize::from(attendu);
            }
            if p[0] == p[2] && p[0] == p[0].floor() || p[1] == p[3] && p[1] == p[1].floor() {
                sur_une_ligne += 1;
            }
            // Un coin de la grille sur le segment : un point (X, Y) entier sur la droite, entre les extrémités.
            let (dx, dy) = (p[2] - p[0], p[3] - p[1]);
            if dx != 0.0 && dy != 0.0 {
                let (lo, hi) = (p[0].min(p[2]).floor() as i64, p[0].max(p[2]).floor() as i64);
                for x in lo..=hi + 1 {
                    let t = (x as f64 - p[0]) / dx;
                    let y = p[1] + t * dy;
                    if (0.0..=1.0).contains(&t) && y == y.floor() && (x as f64 - p[0]) * dy == (y - p[1]) * dx {
                        par_un_coin += 1;
                        break;
                    }
                }
            }
        }
    }
    assert_eq!(total, 80_000);
    assert!(libres > total / 5 && libres < total * 4 / 5, "garde : {libres} segments libres sur {total}");
    assert!(
        verticales > 1_000 && horizontales > 1_000,
        "garde : {verticales} faces verticales, {horizontales} horizontales"
    );
    assert!(sur_une_ligne > 100, "garde : {sur_une_ligne} segments couchés sur une ligne de la grille");
    assert!(par_un_coin > 50, "garde : {par_un_coin} segments passent par un coin de la grille");
}
