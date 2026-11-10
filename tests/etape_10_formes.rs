//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « les formes ».
//!
//! Ils fixent `libres` (et ses erreurs) pour la brique, le L, le disque et l'anneau :
//!
//! - **des dessins écrits à la main** (`.` une case libre, `#` une case bloquée ; le haut du dessin est la ligne 0) sur de
//!   petites boîtes, dont des boîtes qui ne sont pas carrées (un échange de la largeur et de la hauteur, ou de `x` et `y`, ne
//!   se voit pas sur une boîte carrée) : une brique, un L (épaisseur 1, 2, égale au plus petit côté), un disque de rayon 0, 1,
//!   2 et 3, un anneau, une case seule dans chaque coin ;
//! - **les aires par formule** : la brique `w × h` ; le L `W H − (W − e)(H − e)` ; le disque, les comptages de Gauss pour
//!   `r = 0, 1, 2, 3, 4, 5, 10` (1, 5, 13, 29, 49, 81, 317) ; l'anneau, le disque extérieur moins le disque intérieur ;
//! - **les contrats** : `FormeInvalide` (chaque condition, à sa borne), `HorsDeLaBoite` (chaque bord, à sa borne : la dernière
//!   case dedans, la suivante dehors), `FormeInvalide` avant `HorsDeLaBoite`, les boîtes de dimension nulle, des paramètres
//!   énormes (`usize::MAX`) sans panique ni débordement ;
//! - **la symétrie** du disque et de l'anneau (par les axes et par la diagonale), et le L plein quand `epaisseur` égale le
//!   plus petit côté ;
//! - **des formes au hasard**, comparées case par case au prédicat écrit dans `outils.rs`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_10/outils.rs"]
mod outils;

use mycelium_lab::moule::{ErreurMoule, Forme, libres, obstacles};
use outils::{Hasard, compter, dans_la_forme, delai, dessin, gauss, libres_de_reference};

/// Le dessin de la cavité d'une forme dans une boîte, après avoir vérifié la forme du résultat.
fn dessiner(largeur: usize, hauteur: usize, forme: &Forme) -> String {
    let cases = libres(largeur, hauteur, forme).expect("la forme est valide et dans la boîte");
    assert_eq!(cases.len(), largeur * hauteur, "un booléen par case de la boîte");
    dessin(largeur, &cases)
}

fn brique(x0: usize, y0: usize, x1: usize, y1: usize) -> Forme {
    Forme::Brique { x0, y0, x1, y1 }
}

fn l(x0: usize, y0: usize, x1: usize, y1: usize, epaisseur: usize) -> Forme {
    Forme::L { x0, y0, x1, y1, epaisseur }
}

fn disque(cx: usize, cy: usize, rayon: usize) -> Forme {
    Forme::Disque { cx, cy, rayon }
}

fn anneau(cx: usize, cy: usize, rayon_exterieur: usize, rayon_interieur: usize) -> Forme {
    Forme::Anneau { cx, cy, rayon_exterieur, rayon_interieur }
}

fn labyrinthe(colonnes: usize, rangees: usize, couloir: usize, graine: u64) -> Forme {
    Forme::Labyrinthe { colonnes, rangees, couloir, graine }
}

// ---------------------------------------------------------------------------------------------------------------------
// La brique, à la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_brique_est_son_rectangle_bornes_incluses() {
    delai();
    // Une boîte large (6 × 4) : la brique x 1..=3, y 1..=2.
    assert_eq!(dessiner(6, 4, &brique(1, 1, 3, 2)), "######\n#...##\n#...##\n######");
    // Une boîte haute (4 × 6) : la même brique, la largeur et la hauteur ne s'échangent pas.
    assert_eq!(dessiner(4, 6, &brique(1, 1, 2, 3)), "####\n#..#\n#..#\n#..#\n####\n####");
    // Une case seule, dans chaque coin d'une boîte de 5 × 3.
    assert_eq!(dessiner(5, 3, &brique(0, 0, 0, 0)), ".####\n#####\n#####");
    assert_eq!(dessiner(5, 3, &brique(4, 0, 4, 0)), "####.\n#####\n#####");
    assert_eq!(dessiner(5, 3, &brique(0, 2, 0, 2)), "#####\n#####\n.####");
    assert_eq!(dessiner(5, 3, &brique(4, 2, 4, 2)), "#####\n#####\n####.");
    // Toute la boîte.
    assert_eq!(dessiner(3, 2, &brique(0, 0, 2, 1)), "...\n...");
    // Une ligne, une colonne.
    assert_eq!(dessiner(5, 3, &brique(1, 1, 3, 1)), "#####\n#...#\n#####");
    assert_eq!(dessiner(5, 3, &brique(2, 0, 2, 2)), "##.##\n##.##\n##.##");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_aire_d_une_brique_est_largeur_fois_hauteur() {
    delai();
    for (largeur, hauteur, x0, y0, x1, y1) in
        [(10, 7, 0, 0, 9, 6), (10, 7, 2, 1, 8, 5), (3, 11, 1, 3, 1, 9), (1, 1, 0, 0, 0, 0)]
    {
        let cases = libres(largeur, hauteur, &brique(x0, y0, x1, y1)).unwrap();
        assert_eq!(
            compter(&cases),
            (x1 - x0 + 1) * (y1 - y0 + 1),
            "brique {x0} {y0} {x1} {y1} dans {largeur} × {hauteur}"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le L, à la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_l_est_la_barre_de_gauche_et_la_barre_du_bas() {
    delai();
    // Le rectangle x 1..=5, y 1..=5 (5 × 5), épaisseur 2 : colonnes 1 et 2 sur toute la hauteur, lignes 4 et 5 sur toute la
    // largeur.
    assert_eq!(dessiner(6, 6, &l(1, 1, 5, 5, 2)), "######\n#..###\n#..###\n#..###\n#.....\n#.....");
    // Épaisseur 1 : une colonne et une ligne.
    assert_eq!(dessiner(6, 6, &l(1, 1, 5, 5, 1)), "######\n#.####\n#.####\n#.####\n#.####\n#.....");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_l_decale_ne_prend_pas_ses_barres_au_bord_de_la_boite() {
    delai();
    // x 2..=7, y 1..=4 dans 9 × 6, épaisseur 2 : barre de gauche = colonnes 2 et 3 (lignes 1 à 4), barre du bas = lignes 3 et 4
    // (colonnes 2 à 7).
    let texte = dessiner(9, 6, &l(2, 1, 7, 4, 2));
    let lignes: Vec<&str> = texte.lines().collect();
    assert_eq!(lignes.len(), 6);
    assert_eq!(lignes[0], "#########");
    assert_eq!(lignes[1], "##..#####");
    assert_eq!(lignes[2], "##..#####");
    assert_eq!(lignes[3], "##......#");
    assert_eq!(lignes[4], "##......#");
    assert_eq!(lignes[5], "#########");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_l_d_epaisseur_egale_au_plus_petit_cote_est_le_rectangle_plein() {
    delai();
    // 5 de large, 3 de haut, épaisseur 3 : le rectangle plein.
    assert_eq!(dessiner(7, 5, &l(1, 1, 5, 3, 3)), "#######\n#.....#\n#.....#\n#.....#\n#######");
    // 3 de large, 5 de haut, épaisseur 3.
    assert_eq!(dessiner(5, 7, &l(1, 1, 3, 5, 3)), "#####\n#...#\n#...#\n#...#\n#...#\n#...#\n#####");
    // Un carré d'épaisseur égale au côté.
    assert_eq!(dessiner(4, 4, &l(0, 0, 3, 3, 4)), "....\n....\n....\n....");
    // Pour toute épaisseur égale au plus petit côté, la même cavité que la brique.
    for (largeur, hauteur) in [(2usize, 2usize), (2, 5), (5, 2), (4, 4), (1, 6), (6, 1)] {
        let epaisseur = largeur.min(hauteur);
        let plein = libres(largeur + 2, hauteur + 2, &l(1, 1, largeur, hauteur, epaisseur)).unwrap();
        let rectangle = libres(largeur + 2, hauteur + 2, &brique(1, 1, largeur, hauteur)).unwrap();
        assert_eq!(plein, rectangle, "{largeur} × {hauteur}, épaisseur {epaisseur}");
    }
    // Une épaisseur d'une unité de moins n'est plus le rectangle plein (4 × 4, épaisseur 3 : la case en haut à droite manque).
    assert_eq!(dessiner(4, 4, &l(0, 0, 3, 3, 3)), "...#\n....\n....\n....");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_aire_d_un_l_est_la_formule() {
    delai();
    // L'aire d'un L de W × H et d'épaisseur e : W H − (W − e)(H − e).
    for (w, h) in [(5usize, 5usize), (6, 3), (3, 6), (9, 4), (2, 2), (1, 8), (8, 1), (10, 10)] {
        for e in 1..=w.min(h) {
            let cases = libres(w + 3, h + 2, &l(2, 1, 1 + w, h, e)).unwrap();
            assert_eq!(compter(&cases), w * h - (w - e) * (h - e), "L de {w} × {h}, épaisseur {e}");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le disque et l'anneau, à la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_disque_est_les_cases_a_une_distance_entiere_au_plus_egale_au_rayon() {
    delai();
    // Rayon 0 : la case seule (au centre, et dans un coin).
    assert_eq!(dessiner(3, 3, &disque(1, 1, 0)), "###\n#.#\n###");
    assert_eq!(dessiner(3, 2, &disque(0, 0, 0)), ".##\n###");
    // Rayon 1 : une croix de 5 cases.
    assert_eq!(dessiner(3, 3, &disque(1, 1, 1)), "#.#\n...\n#.#");
    // Rayon 2 : 13 cases.
    assert_eq!(dessiner(5, 5, &disque(2, 2, 2)), "##.##\n#...#\n.....\n#...#\n##.##");
    // Rayon 3 : 29 cases.
    assert_eq!(dessiner(7, 7, &disque(3, 3, 3)), "###.###\n#.....#\n#.....#\n.......\n#.....#\n#.....#\n###.###");
    // Un disque décalé dans une boîte plus grande et non carrée : centre (4, 3), rayon 2, dans 9 × 6.
    assert_eq!(dessiner(9, 6, &disque(4, 3, 2)), "#########\n####.####\n###...###\n##.....##\n###...###\n####.####");
    // La distance exacte compte : (3, 4) est à 5 du centre, donc dans le disque de rayon 5 (≤), et (4, 4) est à
    // sqrt(32) > 5, hors du disque.
    let cases = libres(11, 11, &disque(5, 5, 5)).unwrap();
    assert!(cases[(5 + 4) * 11 + (5 + 3)], "(x − cx, y − cy) = (3, 4) est sur le cercle : dedans");
    assert!(cases[(5 + 3) * 11 + (5 + 4)], "(4, 3) aussi");
    assert!(!cases[(5 + 4) * 11 + (5 + 4)], "(4, 4) est à sqrt(32) > 5 : dehors");
    assert!(cases[5 * 11 + 10] && cases[10 * 11 + 5] && cases[5] && cases[5 * 11], "les quatre pointes du disque");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_comptages_de_gauss() {
    delai();
    for (rayon, attendu) in [(0usize, 1usize), (1, 5), (2, 13), (3, 29), (4, 49), (5, 81), (10, 317)] {
        let cote = 2 * rayon + 1;
        let cases = libres(cote, cote, &disque(rayon, rayon, rayon)).unwrap();
        assert_eq!(compter(&cases), attendu, "disque de rayon {rayon}");
        assert_eq!(gauss(rayon), attendu, "le comptage de référence de outils.rs, rayon {rayon}");
    }
    // Le même disque dans une boîte plus grande et non carrée : il a la même aire.
    assert_eq!(compter(&libres(40, 25, &disque(14, 12, 10)).unwrap()), 317);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_anneau_est_les_cases_entre_les_deux_cercles_le_cercle_interieur_exclu() {
    delai();
    // Rayons 3 et 1 : le cercle intérieur (distance 1) est exclu, le cercle extérieur inclus.
    assert_eq!(dessiner(7, 7, &anneau(3, 3, 3, 1)), "###.###\n#.....#\n#..#..#\n..###..\n#..#..#\n#.....#\n###.###");
    // Rayon intérieur 0 : seul le centre manque.
    assert_eq!(dessiner(5, 5, &anneau(2, 2, 2, 0)), "##.##\n#...#\n..#..\n#...#\n##.##");
    // Rayons 5 et 3 : un anneau creux de 11 × 11 (81 − 29 = 52 cases).
    let cases = libres(11, 11, &anneau(5, 5, 5, 3)).unwrap();
    assert_eq!(compter(&cases), 81 - 29);
    // La case à distance exactement 3 est exclue, celle à distance exactement 5 est incluse.
    assert!(!cases[5 * 11 + 8], "(3, 0) est sur le cercle intérieur de rayon 3 : exclu");
    assert!(cases[5 * 11 + 10], "(5, 0) est sur le cercle extérieur : inclus");
    assert!(cases[9 * 11 + 8], "(3, 4) est à 5 : inclus");
    // Un anneau décalé dans une boîte non carrée : centre (5, 3), rayons 3 et 1, dans 11 × 8 (le haut de la boîte coupe à y = 0).
    assert_eq!(
        dessiner(11, 8, &anneau(5, 3, 3, 1)),
        "#####.#####\n###.....###\n###..#..###\n##..###..##\n###..#..###\n###.....###\n#####.#####\n###########"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// Les aires, la symétrie, les formes au hasard
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_aire_d_un_anneau_est_le_disque_exterieur_moins_le_disque_interieur() {
    delai();
    for (re, ri) in [(1usize, 0usize), (2, 1), (5, 2), (10, 3), (12, 11), (7, 0), (9, 8), (20, 10)] {
        let cote = 2 * re + 1;
        let cases = libres(cote, cote, &anneau(re, re, re, ri)).unwrap();
        assert_eq!(compter(&cases), gauss(re) - gauss(ri), "anneau de rayons {re} et {ri}");
        // Et par les deux disques de la bibliothèque.
        let exterieur = compter(&libres(cote, cote, &disque(re, re, re)).unwrap());
        let interieur = compter(&libres(cote, cote, &disque(re, re, ri)).unwrap());
        assert_eq!(compter(&cases), exterieur - interieur, "anneau de rayons {re} et {ri}, par les disques");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_disque_et_l_anneau_sont_symetriques() {
    delai();
    for forme_de in [0usize, 1] {
        for (rayon, interieur) in [(1usize, 0usize), (4, 1), (7, 3), (12, 5), (16, 15), (9, 8)] {
            let cote = 2 * rayon + 1;
            let forme =
                if forme_de == 0 { disque(rayon, rayon, rayon) } else { anneau(rayon, rayon, rayon, interieur) };
            let cases = libres(cote, cote, &forme).unwrap();
            let case = |x: usize, y: usize| cases[y * cote + x];
            for y in 0..cote {
                for x in 0..cote {
                    let v = case(x, y);
                    assert_eq!(v, case(cote - 1 - x, y), "symétrie gauche-droite en ({x}, {y}), rayon {rayon}");
                    assert_eq!(v, case(x, cote - 1 - y), "symétrie haut-bas en ({x}, {y}), rayon {rayon}");
                    assert_eq!(v, case(y, x), "symétrie par la diagonale en ({x}, {y}), rayon {rayon}");
                }
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn des_formes_au_hasard_sont_celles_du_predicat() {
    delai();
    let mut hasard = Hasard::new(2024);
    let mut vues = [0usize; 4];
    for _ in 0..600 {
        let largeur = hasard.entre(1, 24);
        let hauteur = hasard.entre(1, 24);
        let forme = match hasard.jusqu_a(4) {
            0 => {
                let (x0, y0) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                brique(x0, y0, hasard.entre(x0, largeur - 1), hasard.entre(y0, hauteur - 1))
            }
            1 => {
                let (x0, y0) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let (x1, y1) = (hasard.entre(x0, largeur - 1), hasard.entre(y0, hauteur - 1));
                let petit = (x1 - x0 + 1).min(y1 - y0 + 1);
                l(x0, y0, x1, y1, hasard.entre(1, petit))
            }
            2 => {
                let (cx, cy) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let maximum = cx.min(cy).min(largeur - 1 - cx).min(hauteur - 1 - cy);
                disque(cx, cy, hasard.entre(0, maximum))
            }
            _ => {
                let (cx, cy) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let maximum = cx.min(cy).min(largeur - 1 - cx).min(hauteur - 1 - cy);
                if maximum == 0 {
                    continue;
                }
                let exterieur = hasard.entre(1, maximum);
                anneau(cx, cy, exterieur, hasard.entre(0, exterieur - 1))
            }
        };
        let attendu: Vec<bool> =
            (0..largeur * hauteur).map(|i| dans_la_forme(&forme, (i % largeur) as i64, (i / largeur) as i64)).collect();
        let cases = libres(largeur, hauteur, &forme).unwrap();
        assert_eq!(cases, attendu, "{forme:?} dans {largeur} × {hauteur}\n{}", dessin(largeur, &cases));
        assert_eq!(cases, libres_de_reference(largeur, hauteur, &forme));
        assert!(compter(&cases) >= 1, "une forme valide a au moins une case : {forme:?}");
        vues[match forme {
            Forme::Brique { .. } => 0,
            Forme::L { .. } => 1,
            Forme::Disque { .. } => 2,
            _ => 3,
        }] += 1;
    }
    assert!(vues.iter().all(|&n| n >= 50), "garde : chaque forme est tirée assez souvent : {vues:?}");
}

// ---------------------------------------------------------------------------------------------------------------------
// Les contrats
// ---------------------------------------------------------------------------------------------------------------------

/// Le résultat de `libres` et celui de `obstacles` doivent être la même erreur.
fn erreur(largeur: usize, hauteur: usize, forme: &Forme) -> ErreurMoule {
    let a = libres(largeur, hauteur, forme).expect_err("une erreur était attendue");
    let b = obstacles(largeur, hauteur, forme).expect_err("obstacles : la même erreur était attendue");
    assert_eq!(a, b, "libres et obstacles rendent la même erreur pour {forme:?}");
    a
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_formes_hors_contrat_sont_refusees_sans_paniquer() {
    delai();
    let invalide = ErreurMoule::FormeInvalide;
    // Brique : x0 ≤ x1 et y0 ≤ y1, chacun à sa borne.
    assert_eq!(erreur(10, 10, &brique(3, 0, 2, 5)), invalide);
    assert_eq!(erreur(10, 10, &brique(0, 3, 5, 2)), invalide);
    assert!(libres(10, 10, &brique(3, 3, 3, 3)).is_ok(), "x0 = x1 et y0 = y1 sont permis");
    // L : épaisseur 0, épaisseur au-delà du plus petit côté (largeur, hauteur), rectangle à l'envers.
    assert_eq!(erreur(10, 10, &l(0, 0, 5, 5, 0)), invalide);
    assert_eq!(erreur(10, 10, &l(0, 0, 5, 3, 5)), invalide, "épaisseur 5 > hauteur 4");
    assert_eq!(erreur(10, 10, &l(0, 0, 3, 5, 5)), invalide, "épaisseur 5 > largeur 4");
    assert_eq!(erreur(10, 10, &l(0, 0, 5, 3, 7)), invalide);
    assert_eq!(erreur(10, 10, &l(4, 0, 3, 5, 1)), invalide);
    assert_eq!(erreur(10, 10, &l(0, 4, 5, 3, 1)), invalide);
    assert!(libres(10, 10, &l(0, 0, 5, 3, 4)).is_ok(), "épaisseur égale à la hauteur 4 : permis");
    assert!(libres(10, 10, &l(0, 0, 3, 5, 4)).is_ok(), "épaisseur égale à la largeur 4 : permis");
    assert!(libres(10, 10, &l(2, 2, 2, 2, 1)).is_ok(), "un L d'une case");
    // Anneau : rayon_interieur < rayon_exterieur.
    assert_eq!(erreur(10, 10, &anneau(5, 5, 3, 3)), invalide);
    assert_eq!(erreur(10, 10, &anneau(5, 5, 3, 4)), invalide);
    assert_eq!(erreur(10, 10, &anneau(5, 5, 0, 0)), invalide);
    assert!(libres(10, 10, &anneau(5, 5, 3, 2)).is_ok());
    // Labyrinthe : colonnes, rangees, couloir ≥ 1.
    assert_eq!(erreur(30, 30, &labyrinthe(0, 3, 2, 1)), invalide);
    assert_eq!(erreur(30, 30, &labyrinthe(3, 0, 2, 1)), invalide);
    assert_eq!(erreur(30, 30, &labyrinthe(3, 3, 0, 1)), invalide);
    assert!(libres(30, 30, &labyrinthe(1, 1, 1, 0)).is_ok());
    // Le disque accepte tout rayon (0 compris).
    assert!(libres(10, 10, &disque(5, 5, 0)).is_ok());
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_cavite_qui_sort_de_la_boite_est_refusee_a_chaque_bord() {
    delai();
    let hors = ErreurMoule::HorsDeLaBoite;
    // Une boîte de 12 × 9 (non carrée) : la dernière case dedans, la suivante dehors, bord par bord.
    assert!(libres(12, 9, &brique(0, 0, 11, 8)).is_ok());
    assert_eq!(erreur(12, 9, &brique(0, 0, 12, 8)), hors, "x1 = largeur");
    assert_eq!(erreur(12, 9, &brique(0, 0, 11, 9)), hors, "y1 = hauteur");
    assert_eq!(
        erreur(9, 12, &brique(0, 0, 11, 8)),
        hors,
        "x1 = 11 ≥ largeur 9 : pas d'échange de la largeur et de la hauteur"
    );
    assert_eq!(erreur(12, 9, &brique(20, 20, 30, 30)), hors);
    // Le L : le rectangle englobant compte.
    assert!(libres(12, 9, &l(5, 3, 11, 8, 2)).is_ok());
    assert_eq!(erreur(12, 9, &l(5, 3, 12, 8, 2)), hors);
    assert_eq!(erreur(12, 9, &l(5, 3, 11, 9, 2)), hors);
    // Le disque : cx + r ≤ largeur − 1, cy + r ≤ hauteur − 1, cx ≥ r, cy ≥ r.
    assert!(libres(12, 9, &disque(8, 4, 3)).is_ok(), "cx + r = 11 = largeur − 1");
    assert_eq!(erreur(12, 9, &disque(9, 4, 3)), hors, "cx + r = largeur");
    assert!(libres(12, 9, &disque(4, 5, 3)).is_ok(), "cy + r = 8 = hauteur − 1");
    assert_eq!(erreur(12, 9, &disque(4, 6, 3)), hors, "cy + r = hauteur");
    assert!(libres(12, 9, &disque(3, 3, 3)).is_ok(), "cx − r = 0");
    assert_eq!(erreur(12, 9, &disque(2, 4, 3)), hors, "cx − r < 0");
    assert_eq!(erreur(12, 9, &disque(4, 2, 3)), hors, "cy − r < 0");
    // Un disque dont seule la pointe sort : (cx − r, cy) est une case du disque, hors de la boîte.
    assert_eq!(erreur(12, 9, &disque(0, 4, 1)), hors);
    // L'anneau : le rayon extérieur décide.
    assert!(libres(12, 9, &anneau(4, 4, 4, 1)).is_ok());
    assert_eq!(erreur(12, 9, &anneau(4, 4, 5, 1)), hors, "cx − re < 0");
    assert_eq!(erreur(12, 9, &anneau(7, 4, 5, 1)), hors, "cx + re = largeur");
    assert_eq!(erreur(12, 9, &anneau(4, 5, 4, 1)), hors, "cy + re = hauteur");
    // Le labyrinthe : W = colonnes (couloir + 1) + 1 et H = rangees (couloir + 1) + 1.
    assert!(libres(25, 25, &labyrinthe(6, 6, 3, 5)).is_ok(), "W = H = 25");
    assert_eq!(erreur(24, 25, &labyrinthe(6, 6, 3, 5)), hors, "W = 25 > largeur 24");
    assert_eq!(erreur(25, 24, &labyrinthe(6, 6, 3, 5)), hors, "H = 25 > hauteur 24");
    assert!(libres(13, 9, &labyrinthe(4, 2, 2, 5)).is_ok(), "W = 13, H = 7");
    assert_eq!(erreur(12, 9, &labyrinthe(4, 2, 2, 5)), hors, "W = 13 > 12");
    assert_eq!(erreur(13, 6, &labyrinthe(4, 2, 2, 5)), hors, "H = 7 > 6");
    assert_eq!(erreur(7, 13, &labyrinthe(4, 2, 2, 5)), hors, "largeur et hauteur ne s'échangent pas");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_forme_invalide_l_emporte_sur_hors_de_la_boite() {
    delai();
    // Deux fautes à la fois : la forme est invalide ET elle sort de la boîte.
    assert_eq!(erreur(5, 5, &brique(40, 40, 30, 30)), ErreurMoule::FormeInvalide);
    assert_eq!(erreur(5, 5, &l(0, 0, 40, 40, 0)), ErreurMoule::FormeInvalide);
    assert_eq!(erreur(5, 5, &l(0, 0, 40, 40, 42)), ErreurMoule::FormeInvalide);
    assert_eq!(erreur(5, 5, &anneau(40, 40, 3, 5)), ErreurMoule::FormeInvalide);
    assert_eq!(erreur(5, 5, &labyrinthe(10, 0, 3, 0)), ErreurMoule::FormeInvalide);
    assert_eq!(erreur(0, 0, &brique(3, 0, 2, 0)), ErreurMoule::FormeInvalide, "même dans une boîte de dimension nulle");
    // Et quand aucune boîte ne pourrait contenir la forme (une case à gauche de la colonne 0, une coordonnée qui déborde) : le
    // contrat passe avant tout calcul d'encombrement.
    assert_eq!(erreur(5, 5, &anneau(2, 2, 5, 7)), ErreurMoule::FormeInvalide, "cx − rayon < 0 et rayons à l'envers");
    assert_eq!(erreur(5, 5, &disque(0, 0, 3)), ErreurMoule::HorsDeLaBoite, "le disque n'a pas de contrat : hors");
    assert_eq!(erreur(5, 5, &l(0, 0, usize::MAX, 5, 0)), ErreurMoule::FormeInvalide, "x1 = usize::MAX et épaisseur 0");
    assert_eq!(
        erreur(5, 5, &labyrinthe(usize::MAX, 0, 1, 0)),
        ErreurMoule::FormeInvalide,
        "colonnes énormes et rangées 0"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_boite_de_dimension_nulle_ne_contient_aucune_forme_valide() {
    delai();
    let formes = [
        brique(0, 0, 0, 0),
        l(0, 0, 0, 0, 1),
        disque(0, 0, 0),
        anneau(1, 1, 1, 0),
        labyrinthe(1, 1, 1, 7),
        brique(0, 0, 3, 3),
        disque(5, 5, 2),
    ];
    for forme in &formes {
        for (largeur, hauteur) in [(0usize, 0usize), (0, 7), (7, 0), (0, 1), (1, 0)] {
            assert_eq!(
                erreur(largeur, hauteur, forme),
                ErreurMoule::HorsDeLaBoite,
                "{forme:?} dans {largeur} × {hauteur}"
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn des_parametres_enormes_donnent_une_erreur_et_non_une_panique() {
    delai();
    let max = usize::MAX;
    let hors = ErreurMoule::HorsDeLaBoite;
    assert_eq!(erreur(10, 10, &brique(0, 0, max, 3)), hors);
    assert_eq!(erreur(10, 10, &brique(max, max, max, max)), hors);
    assert_eq!(
        erreur(10, 10, &l(0, 0, max, max, max)),
        hors,
        "épaisseur égale au plus petit côté (max + 1 en vrai) : valide"
    );
    assert_eq!(erreur(10, 10, &l(0, 0, max, 5, 3)), hors);
    assert_eq!(erreur(10, 10, &l(0, 0, 5, 5, max)), ErreurMoule::FormeInvalide);
    assert_eq!(erreur(10, 10, &disque(max, max, max)), hors);
    assert_eq!(erreur(10, 10, &disque(5, 5, max)), hors);
    assert_eq!(erreur(10, 10, &disque(max, 5, 1)), hors);
    assert_eq!(erreur(10, 10, &disque(max - 1, 5, 1)), hors, "cx + rayon + 1 déborde usize");
    assert_eq!(erreur(10, 10, &anneau(max, max, max, max - 1)), hors);
    assert_eq!(erreur(10, 10, &anneau(5, 5, max, 0)), hors);
    assert_eq!(erreur(10, 10, &labyrinthe(max, 1, 1, 0)), hors);
    assert_eq!(erreur(10, 10, &labyrinthe(1, max, 1, 0)), hors);
    assert_eq!(erreur(10, 10, &labyrinthe(1, 1, max, 0)), hors);
    assert_eq!(erreur(10, 10, &labyrinthe(max / 2, max / 2, max / 2, 0)), hors);
}
