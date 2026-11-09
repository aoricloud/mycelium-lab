//! Tests d'acceptation de l'étape 9 (M0, mesurer la pièce), partie « les morceaux et la traversée ».
//!
//! Ils fixent `morceaux`, `plus_grand_morceau` et `traversee` sur des grilles de cases occupées :
//!
//! - **des grilles écrites à la main** (`#` occupée, `.` vide ; le haut de la figure est la ligne 0) : une case seule, une
//!   ligne, un L, un anneau (et un anneau avec un îlot au centre), deux îlots dont le plus grand est rencontré en second,
//!   deux cases en diagonale seulement (un seul morceau, dans les quatre diagonales, et les huit voisines d'une case),
//!   deux cases qui ne se touchent que par le « retour à la ligne » du rangement (deux morceaux), une grille vide, une
//!   grille pleine, un U profond où le plus court chemin fait 14 cases pour 10 colonnes, un trou qui coupe la traversée,
//!   une rangée de cases pleines sauf une qui se rejoint en diagonale, le haut-bas et le gauche-droite, une case seule
//!   sur les deux bords (boîte d'une case, d'une ligne, d'une colonne, case de coin pour deux bords voisins) ;
//! - **les paniques** : `de == vers` (pour chaque bord), une tranche de mauvaise longueur (trop courte, trop longue, pour
//!   les trois fonctions) ;
//! - **toutes les grilles** de moins de 13 cases (toutes les formes) et toutes les grilles de 4 × 4, comparées à deux calculs
//!   indépendants écrits dans `etape_9/outils.rs` (une union de classes, des relaxations successives) pour les douze
//!   traversées possibles ; puis des grilles au hasard, jusqu'à 14 × 14 ;
//! - **une grande boîte** (161 × 161, pleine, en serpentin, et au hasard contre l'union de classes) : pas de débordement de
//!   pile, des valeurs connues ;
//! - **une boîte sans case** (largeur ou hauteur nulle, tranche vide) : aucun morceau, aucune traversée, pas de panique.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_9/outils.rs"]
mod outils;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::piece::{Bord, morceaux, plus_grand_morceau, traversee};
use outils::{Hasard, bord, delai, grille, morceaux_de_reference, nom_du_bord, traversee_de_reference};

fn panique<F: FnOnce()>(f: F) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

/// Vrai si l'evaluation de l'expression panique (le resultat est jete).
macro_rules! appel {
    ($e:expr) => {
        panique(|| {
            let _ = $e;
        })
    };
}

/// Les trois résultats de morceaux d'une grille écrite à la main, vérifiés ensemble.
fn verifier_morceaux(texte: &str, attendu: &[usize]) {
    let (l, h, g) = grille(texte);
    assert_eq!(morceaux(l, h, &g), attendu, "morceaux de\n{texte}");
    assert_eq!(plus_grand_morceau(l, h, &g), attendu.first().copied().unwrap_or(0), "plus grand morceau de\n{texte}");
}

/// La traversée d'une grille écrite à la main, entre les bords d'indices `de` et `vers`.
fn t(texte: &str, de: usize, vers: usize) -> Option<usize> {
    let (l, h, g) = grille(texte);
    traversee(l, h, &g, bord(de), bord(vers))
}

const GAUCHE: usize = 0;
const DROITE: usize = 1;
const HAUT: usize = 2;
const BAS: usize = 3;

// ---------------------------------------------------------------------------------------------------------------------
// morceaux et plus_grand_morceau, à la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_case_seule_est_un_morceau_de_une_case() {
    delai();
    verifier_morceaux("#", &[1]);
    verifier_morceaux(".", &[]);
    verifier_morceaux("...\n.#.\n...", &[1]);
    verifier_morceaux("#..\n...\n...", &[1]);
    verifier_morceaux("...\n...\n..#", &[1]);
    verifier_morceaux("..#\n...\n...", &[1]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_grille_vide_n_a_aucun_morceau() {
    delai();
    verifier_morceaux("....\n....\n....", &[]);
    verifier_morceaux("...", &[]);
    verifier_morceaux(".\n.\n.", &[]);
    assert_eq!(plus_grand_morceau(4, 3, &[false; 12]), 0);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_grille_pleine_est_un_seul_morceau() {
    delai();
    verifier_morceaux("####\n####\n####", &[12]);
    verifier_morceaux("#", &[1]);
    verifier_morceaux("#####", &[5]);
    verifier_morceaux("#\n#\n#\n#", &[4]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_ligne_un_l_et_un_anneau_sont_un_seul_morceau() {
    delai();
    verifier_morceaux(".....\n#####\n.....", &[5]);
    verifier_morceaux(".#.\n.#.\n.#.\n.#.", &[4]);
    verifier_morceaux("#....\n#....\n#....\n#####", &[8]);
    verifier_morceaux("#####\n#...#\n#...#\n#...#\n#####", &[16]);
    // L'intérieur vide de l'anneau ne compte pas ; un îlot au centre en fait un second morceau.
    verifier_morceaux("#####\n#...#\n#.#.#\n#...#\n#####", &[16, 1]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn deux_ilots_sont_rendus_du_plus_grand_au_plus_petit() {
    delai();
    // Le petit îlot (2 cases) est rencontré en premier, le grand (5 cases) en second.
    verifier_morceaux("##....\n......\n....##\n...###", &[5, 2]);
    // Trois îlots de tailles 1, 3, 2 dans l'ordre de lecture.
    verifier_morceaux("#.....\n......\n.###..\n......\n....##", &[3, 2, 1]);
    // Deux îlots de même taille.
    verifier_morceaux("##...\n.....\n...##", &[2, 2]);
    // Des îlots séparés par une seule case vide (ligne, colonne) restent séparés.
    verifier_morceaux("#.#", &[1, 1]);
    verifier_morceaux("#\n.\n#", &[1, 1]);
    verifier_morceaux("#..#\n....\n....\n#..#", &[1, 1, 1, 1]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn deux_cases_en_diagonale_font_un_seul_morceau() {
    delai();
    verifier_morceaux("#.\n.#", &[2]);
    verifier_morceaux(".#\n#.", &[2]);
    verifier_morceaux("#...\n.#..\n..#.\n...#", &[4]);
    verifier_morceaux("...#\n..#.\n.#..\n#...", &[4]);
    // Un escalier en diagonale, plus loin une case seule.
    verifier_morceaux("#.....\n.#....\n..#...\n......\n.....#", &[3, 1]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_case_du_centre_a_huit_voisines() {
    delai();
    // Chacune des huit voisines, seule avec le centre, fait un morceau de 2.
    let voisines = [(0, 0), (1, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2), (2, 2)];
    for (vx, vy) in voisines {
        let mut texte = String::new();
        for y in 0..3 {
            for x in 0..3 {
                texte.push(if (x, y) == (1, 1) || (x, y) == (vx, vy) { '#' } else { '.' });
            }
            texte.push('\n');
        }
        verifier_morceaux(&texte, &[2]);
    }
    // Deux cases à deux de distance ne se touchent pas.
    verifier_morceaux("#..\n...\n..#", &[1, 1]);
    verifier_morceaux("#.\n..\n.#", &[1, 1]);
    verifier_morceaux("#...\n....\n.#..", &[1, 1]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_fin_d_une_ligne_ne_touche_pas_le_debut_de_la_suivante() {
    delai();
    // Dans le rangement, la case (2, 0) et la case (0, 1) sont consécutives ; dans la boîte, elles sont à deux colonnes.
    verifier_morceaux("..#\n#..", &[1, 1]);
    verifier_morceaux("#..\n..#", &[1, 1]);
    // Pareil entre la fin d'une ligne et le début de la suivante, sur plusieurs lignes.
    verifier_morceaux("...#\n#...\n...#\n#...", &[1, 1, 1, 1]);
    // Et la dernière colonne d'une ligne avec la première de la ligne d'après, en diagonale du rangement (x + 1 = largeur).
    verifier_morceaux(".#\n#.\n.#", &[3]);
}

// ---------------------------------------------------------------------------------------------------------------------
// traversee, à la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_traversee_compte_les_cases_du_chemin() {
    delai();
    // Une ligne pleine de 5 cases : 5 cases de gauche à droite (pas 4 pas), dans les deux sens.
    let ligne = ".....\n#####\n.....";
    assert_eq!(t(ligne, GAUCHE, DROITE), Some(5));
    assert_eq!(t(ligne, DROITE, GAUCHE), Some(5));
    // Rien du haut au bas : la ligne du haut est vide.
    assert_eq!(t(ligne, HAUT, BAS), None);
    assert_eq!(t(ligne, BAS, HAUT), None);
    // Une colonne pleine de 4 cases.
    let colonne = ".#.\n.#.\n.#.\n.#.";
    assert_eq!(t(colonne, HAUT, BAS), Some(4));
    assert_eq!(t(colonne, BAS, HAUT), Some(4));
    assert_eq!(t(colonne, GAUCHE, DROITE), None);
    assert_eq!(t(colonne, DROITE, GAUCHE), None);
    // Une grille pleine : une case par colonne ou par ligne.
    assert_eq!(t("####\n####\n####", GAUCHE, DROITE), Some(4));
    assert_eq!(t("####\n####\n####", HAUT, BAS), Some(3));
    assert_eq!(t("####\n####\n####", DROITE, GAUCHE), Some(4));
    assert_eq!(t("####\n####\n####", BAS, HAUT), Some(3));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_grille_vide_n_a_aucune_traversee() {
    delai();
    for de in 0..4 {
        for vers in 0..4 {
            if de != vers {
                assert_eq!(t("....\n....\n....", de, vers), None, "{} → {}", nom_du_bord(de), nom_du_bord(vers));
                assert_eq!(t(".", de, vers), None);
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_case_seule_sur_les_deux_bords_est_une_traversee_de_une_case() {
    delai();
    // Une boîte d'une case : elle est sur les quatre bords, donc sur les douze couples.
    for de in 0..4 {
        for vers in 0..4 {
            if de != vers {
                assert_eq!(t("#", de, vers), Some(1), "{} → {}", nom_du_bord(de), nom_du_bord(vers));
            }
        }
    }
    // Une boîte d'une ligne : n'importe quelle case occupée est en haut et en bas à la fois.
    assert_eq!(t("..#..", HAUT, BAS), Some(1));
    assert_eq!(t("..#..", BAS, HAUT), Some(1));
    assert_eq!(t("..#..", GAUCHE, DROITE), None);
    assert_eq!(t("#...#", GAUCHE, DROITE), None);
    assert_eq!(t("#####", GAUCHE, DROITE), Some(5));
    // Une boîte d'une colonne.
    assert_eq!(t(".\n.\n#\n.\n.", GAUCHE, DROITE), Some(1));
    assert_eq!(t(".\n.\n#\n.\n.", DROITE, GAUCHE), Some(1));
    assert_eq!(t(".\n.\n#\n.\n.", HAUT, BAS), None);
    assert_eq!(t("#\n.\n#", HAUT, BAS), None);
    assert_eq!(t("#\n#\n#", HAUT, BAS), Some(3));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_case_de_coin_est_sur_deux_bords_voisins_et_le_haut_est_la_ligne_zero() {
    delai();
    // Coin haut-gauche : (0, 0), sur le bord gauche et sur le bord haut.
    let coin = "#..\n...\n...";
    assert_eq!(t(coin, GAUCHE, HAUT), Some(1));
    assert_eq!(t(coin, HAUT, GAUCHE), Some(1));
    assert_eq!(t(coin, GAUCHE, BAS), None);
    assert_eq!(t(coin, DROITE, HAUT), None);
    assert_eq!(t(coin, GAUCHE, DROITE), None);
    assert_eq!(t(coin, HAUT, BAS), None);
    // Coin bas-gauche : (0, 2).
    let coin = "...\n...\n#..";
    assert_eq!(t(coin, GAUCHE, BAS), Some(1));
    assert_eq!(t(coin, GAUCHE, HAUT), None);
    // Coin haut-droit : (2, 0).
    let coin = "..#\n...\n...";
    assert_eq!(t(coin, DROITE, HAUT), Some(1));
    assert_eq!(t(coin, DROITE, BAS), None);
    // Coin bas-droit : (2, 2).
    let coin = "...\n...\n..#";
    assert_eq!(t(coin, DROITE, BAS), Some(1));
    assert_eq!(t(coin, DROITE, HAUT), None);
    assert_eq!(t(coin, GAUCHE, BAS), None);
    // Une case au milieu n'est sur aucun bord.
    for de in 0..4 {
        for vers in 0..4 {
            if de != vers {
                assert_eq!(t("...\n.#.\n...", de, vers), None);
            }
        }
    }
    // Une case de bord, pas de coin : sur un seul bord.
    assert_eq!(t(".#.\n...\n...", HAUT, BAS), None);
    assert_eq!(t(".#.\n...\n...", GAUCHE, HAUT), None);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_l_se_traverse_par_son_coude() {
    delai();
    let l = "#....\n#....\n#....\n#####";
    assert_eq!(t(l, GAUCHE, DROITE), Some(5));
    assert_eq!(t(l, DROITE, GAUCHE), Some(5));
    assert_eq!(t(l, HAUT, BAS), Some(4));
    assert_eq!(t(l, BAS, HAUT), Some(4));
    // Le coude en diagonale : le même L, mais l'angle est coupé par une diagonale.
    let l_en_diagonale = "#...\n#...\n.###";
    assert_eq!(t(l_en_diagonale, HAUT, BAS), Some(3));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_anneau_se_traverse_en_une_largeur_ou_une_hauteur() {
    delai();
    let anneau = "#####\n#...#\n#...#\n#...#\n#####";
    assert_eq!(t(anneau, GAUCHE, DROITE), Some(5));
    assert_eq!(t(anneau, HAUT, BAS), Some(5));
    let anneau_large = "#######\n#.....#\n#######";
    assert_eq!(t(anneau_large, GAUCHE, DROITE), Some(7));
    assert_eq!(t(anneau_large, HAUT, BAS), Some(3));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_escalier_en_diagonale_se_traverse_en_diagonale() {
    delai();
    let descendante = "#...\n.#..\n..#.\n...#";
    assert_eq!(t(descendante, GAUCHE, DROITE), Some(4));
    assert_eq!(t(descendante, HAUT, BAS), Some(4));
    assert_eq!(t(descendante, DROITE, GAUCHE), Some(4));
    let montante = "...#\n..#.\n.#..\n#...";
    assert_eq!(t(montante, GAUCHE, DROITE), Some(4));
    assert_eq!(t(montante, HAUT, BAS), Some(4));
    // Une diagonale qui n'atteint pas tout : pas de traversée.
    assert_eq!(t("#...\n.#..\n..#.\n....", HAUT, BAS), None);
    assert_eq!(t("#...\n.#..\n..#.\n....", GAUCHE, DROITE), None);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_u_profond_se_traverse_par_le_fond_et_non_en_ligne_droite() {
    delai();
    // Les deux queues touchent les bords gauche et droit, mais le seul chemin descend jusqu'au fond du U et remonte :
    // 14 cases pour 10 colonnes. Les diagonales ne raccourcissent que les coins.
    let u = "..........\n##......##\n.#......#.\n.#......#.\n.#......#.\n.########.\n..........";
    assert_eq!(t(u, GAUCHE, DROITE), Some(14));
    assert_eq!(t(u, DROITE, GAUCHE), Some(14));
    // Ni le haut ni le bas ne sont touchés.
    assert_eq!(t(u, HAUT, BAS), None);
    assert_eq!(t(u, BAS, HAUT), None);
    verifier_morceaux(u, &[18]);
    // Un second chemin, plus long mais encore plus court que le tour du U, ne change pas la réponse : la droite du U a
    // une deuxième queue, au fond du bord droit.
    let u_deux_queues = "..........\n##......##\n.#......#.\n.#......#.\n.#......##\n.########.\n..........";
    assert_eq!(t(u_deux_queues, GAUCHE, DROITE), Some(12));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_trou_dans_une_ligne_coupe_la_traversee() {
    delai();
    let trou = ".......\n###.###\n.......";
    assert_eq!(t(trou, GAUCHE, DROITE), None);
    assert_eq!(t(trou, DROITE, GAUCHE), None);
    verifier_morceaux(trou, &[3, 3]);
    // Un trou bouché en diagonale par une case de la ligne d'au-dessus rétablit la traversée.
    let pont = "...#...\n###.###\n.......";
    assert_eq!(t(pont, GAUCHE, DROITE), Some(7));
    verifier_morceaux(pont, &[7]);
    // Un trou dans une rangée pleine coupe le haut-bas : la rangée 2 est vide.
    let mur_troue = "#####\n#####\n.....\n#####\n#####";
    assert_eq!(t(mur_troue, HAUT, BAS), None);
    assert_eq!(t(mur_troue, GAUCHE, DROITE), Some(5));
    verifier_morceaux(mur_troue, &[10, 10]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_seule_case_en_diagonale_dans_une_rangee_rejoint_les_deux_cotes() {
    delai();
    // La rangée 2 ne contient que la case (2, 2) : on y entre et on en sort en diagonale.
    let g = "#####\n#####\n..#..\n#####\n#####";
    assert_eq!(t(g, HAUT, BAS), Some(5));
    assert_eq!(t(g, BAS, HAUT), Some(5));
    verifier_morceaux(g, &[21]);
    // Mêmes bords, mais les rangées 1 et 3 sont vides : la case (2, 2) est seule, et le haut-bas est coupé.
    let g = "#####\n.....\n..#..\n.....\n#####";
    assert_eq!(t(g, HAUT, BAS), None);
    assert_eq!(t(g, GAUCHE, DROITE), Some(5));
    verifier_morceaux(g, &[5, 5, 1]);
    // Un escalier (1, 1), (2, 2), (3, 3) relie les deux rangées pleines par des diagonales.
    let g = "#####\n##...\n..#..\n...##\n#####";
    assert_eq!(t(g, HAUT, BAS), Some(5));
    verifier_morceaux(g, &[15]);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_fin_d_une_ligne_ne_se_prolonge_pas_sur_la_suivante_pour_la_traversee() {
    delai();
    // Gauche : (0, 1). Droite : (2, 0). Dans le rangement, elles sont consécutives ; dans la boîte, elles sont séparées.
    assert_eq!(t("..#\n#..", GAUCHE, DROITE), None);
    assert_eq!(t("..#\n#..", DROITE, GAUCHE), None);
    assert_eq!(t("#..\n..#", GAUCHE, DROITE), None);
    // Haut / bas : la fin de la ligne du bas (2, 1) et le début de la ligne du haut ne se touchent pas non plus.
    assert_eq!(t("#..\n..#", HAUT, BAS), None);
    // Mais dans le même rangement, des cases voisines en diagonale se rejoignent bien : (1, 0) et (0, 1).
    assert_eq!(t(".#\n#.", GAUCHE, DROITE), Some(2));
    assert_eq!(t(".#\n#.", HAUT, BAS), Some(2));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn le_plus_court_chemin_est_choisi_parmi_plusieurs_departs() {
    delai();
    // Deux départs sur le bord gauche : (0, 0), dont le chemin fait un grand détour, et (0, 4), tout droit.
    let g = "#.......\n#######.\n......#.\n......#.\n########";
    // Départ (0,4) : la rangée du bas, 8 cases jusqu'à (7,4), sur le bord droit.
    assert_eq!(t(g, GAUCHE, DROITE), Some(8));
    // Du haut au bas : le seul départ est (0, 0), qui ne rejoint la rangée du bas que par le coude : (1, 1) … (5, 1), puis
    // (6, 2), (6, 3) et (6, 4) : 1 + 5 + 3 = 9 cases (le pas (5, 1) → (6, 2) est en diagonale).
    assert_eq!(t(g, HAUT, BAS), Some(9));
}

// ---------------------------------------------------------------------------------------------------------------------
// Les paniques
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_traversee_d_un_bord_vers_lui_meme_fait_paniquer() {
    delai();
    let pleine = [true; 12];
    let vide = [false; 12];
    for i in 0..4 {
        assert!(
            appel!(traversee(4, 3, &pleine, bord(i), bord(i))),
            "{} → {} (grille pleine)",
            nom_du_bord(i),
            nom_du_bord(i)
        );
        assert!(
            appel!(traversee(4, 3, &vide, bord(i), bord(i))),
            "{} → {} (grille vide)",
            nom_du_bord(i),
            nom_du_bord(i)
        );
        assert!(
            appel!(traversee(1, 1, &[true], bord(i), bord(i))),
            "{} → {} (une case)",
            nom_du_bord(i),
            nom_du_bord(i)
        );
    }
    // Deux bords différents ne font jamais paniquer.
    for de in 0..4 {
        for vers in 0..4 {
            if de != vers {
                assert!(!appel!(traversee(4, 3, &pleine, bord(de), bord(vers))));
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_tranche_de_mauvaise_longueur_fait_paniquer() {
    delai();
    // 3 × 2 = 6 cases : 5 et 7 sont faux, et aussi 0, 12 et 2 × 3 + 1 = 7.
    for longueur in [0usize, 1, 5, 7, 8, 12] {
        let tranche = vec![true; longueur];
        assert!(appel!(morceaux(3, 2, &tranche)), "morceaux, longueur {longueur}");
        assert!(appel!(plus_grand_morceau(3, 2, &tranche)), "plus_grand_morceau, longueur {longueur}");
        assert!(appel!(traversee(3, 2, &tranche, Bord::Gauche, Bord::Droite)), "traversée, longueur {longueur}");
        assert!(appel!(traversee(3, 2, &tranche, Bord::Haut, Bord::Bas)), "traversée verticale, longueur {longueur}");
    }
    // La bonne longueur ne panique pas.
    let bonne = [true; 6];
    assert!(!appel!(morceaux(3, 2, &bonne)));
    assert!(!appel!(plus_grand_morceau(3, 2, &bonne)));
    assert!(!appel!(traversee(3, 2, &bonne, Bord::Gauche, Bord::Droite)));
    // Une dimension fausse avec la bonne tranche : 2 × 4 = 8 ≠ 6.
    assert!(appel!(morceaux(2, 4, &bonne)));
    assert!(appel!(traversee(2, 4, &bonne, Bord::Haut, Bord::Bas)));
    // Même longueur, mais pas le même produit : 3 × 3 = 9 pour 6 cases.
    assert!(appel!(plus_grand_morceau(3, 3, &bonne)));
}

// ---------------------------------------------------------------------------------------------------------------------
// Les calculs indépendants
// ---------------------------------------------------------------------------------------------------------------------

/// Vérifie une grille contre les deux calculs de référence : les morceaux, le plus grand, et les douze traversées.
fn verifier_contre_la_reference(largeur: usize, hauteur: usize, occupees: &[bool], toutes_les_traversees: bool) {
    let attendu = morceaux_de_reference(largeur, hauteur, occupees);
    let obtenu = morceaux(largeur, hauteur, occupees);
    assert_eq!(obtenu, attendu, "morceaux de la grille {largeur} × {hauteur} : {}", dessin(largeur, occupees));
    assert_eq!(
        plus_grand_morceau(largeur, hauteur, occupees),
        attendu.first().copied().unwrap_or(0),
        "plus grand morceau de la grille {largeur} × {hauteur} : {}",
        dessin(largeur, occupees)
    );
    let couples: &[(usize, usize)] = if toutes_les_traversees {
        &[(0, 1), (1, 0), (2, 3), (3, 2), (0, 2), (2, 0), (0, 3), (3, 0), (1, 2), (2, 1), (1, 3), (3, 1)]
    } else {
        &[(0, 1), (2, 3)]
    };
    for &(de, vers) in couples {
        let attendue = traversee_de_reference(largeur, hauteur, occupees, de, vers);
        let obtenue = traversee(largeur, hauteur, occupees, bord(de), bord(vers));
        assert_eq!(
            obtenue,
            attendue,
            "traversée {} → {} de la grille {largeur} × {hauteur} : {}",
            nom_du_bord(de),
            nom_du_bord(vers),
            dessin(largeur, occupees)
        );
    }
}

/// La grille en une ligne de texte, lignes séparées par `/`.
fn dessin(largeur: usize, occupees: &[bool]) -> String {
    occupees
        .chunks(largeur)
        .map(|ligne| ligne.iter().map(|&o| if o { '#' } else { '.' }).collect::<String>())
        .collect::<Vec<_>>()
        .join("/")
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn toutes_les_petites_grilles_suivent_les_deux_calculs_independants() {
    delai();
    let mut nombre = 0usize;
    for largeur in 1..=12usize {
        for hauteur in 1..=12usize {
            let cases = largeur * hauteur;
            if cases > 12 {
                continue;
            }
            for masque in 0u32..(1 << cases) {
                let occupees: Vec<bool> = (0..cases).map(|i| ((masque >> i) & 1) == 1).collect();
                verifier_contre_la_reference(largeur, hauteur, &occupees, cases <= 9);
                nombre += 1;
            }
        }
    }
    // 12 formes de moins de 13 cases (de 1 × 1 à 12 × 1, 1 × 12, 6 × 2, 4 × 3, etc.) : on en a bien joué beaucoup.
    assert!(nombre > 20_000, "{nombre} grilles jouées");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn toutes_les_grilles_de_quatre_sur_quatre() {
    delai();
    for masque in 0u32..(1 << 16) {
        let occupees: Vec<bool> = (0..16).map(|i| ((masque >> i) & 1) == 1).collect();
        verifier_contre_la_reference(4, 4, &occupees, false);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn des_grilles_au_hasard_suivent_les_deux_calculs_independants() {
    delai();
    let mut h = Hasard::new(9001);
    for tour in 0..600 {
        let largeur = 1 + h.jusqu_a(14);
        let hauteur = 1 + h.jusqu_a(14);
        // La densité d'occupation varie de 15 % à 85 % : des îlots, et des grilles presque pleines.
        let proportion = 15 + h.jusqu_a(71);
        let occupees: Vec<bool> = (0..largeur * hauteur).map(|_| h.avec_proba(proportion, 100)).collect();
        verifier_contre_la_reference(largeur, hauteur, &occupees, tour % 2 == 0);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_traversee_est_la_meme_dans_les_deux_sens() {
    delai();
    let mut h = Hasard::new(77);
    let mut avec_une_traversee = 0;
    for _ in 0..400 {
        let largeur = 2 + h.jusqu_a(10);
        let hauteur = 2 + h.jusqu_a(10);
        let occupees: Vec<bool> = (0..largeur * hauteur).map(|_| h.avec_proba(60, 100)).collect();
        for (a, b) in [(0, 1), (2, 3), (0, 2), (0, 3), (1, 2), (1, 3)] {
            let aller = traversee(largeur, hauteur, &occupees, bord(a), bord(b));
            let retour = traversee(largeur, hauteur, &occupees, bord(b), bord(a));
            assert_eq!(aller, retour, "{} ↔ {} : {}", nom_du_bord(a), nom_du_bord(b), dessin(largeur, &occupees));
            // Et la bonne valeur : sans cela, une fonction qui rend toujours la même chose serait symétrique.
            assert_eq!(
                aller,
                traversee_de_reference(largeur, hauteur, &occupees, a, b),
                "{} → {}",
                nom_du_bord(a),
                nom_du_bord(b)
            );
            avec_une_traversee += usize::from(aller.is_some());
        }
    }
    assert!(avec_une_traversee > 100, "garde : trop peu de grilles traversées ({avec_une_traversee})");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_morceaux_partagent_les_cases_occupees_et_la_traversee_est_dans_un_morceau() {
    delai();
    let mut h = Hasard::new(5);
    for _ in 0..300 {
        let largeur = 1 + h.jusqu_a(15);
        let hauteur = 1 + h.jusqu_a(15);
        let occupees: Vec<bool> = (0..largeur * hauteur).map(|_| h.avec_proba(50, 100)).collect();
        let tailles = morceaux(largeur, hauteur, &occupees);
        let total = occupees.iter().filter(|&&o| o).count();
        assert_eq!(tailles.iter().sum::<usize>(), total, "chaque case occupée est dans exactement un morceau");
        assert!(tailles.windows(2).all(|p| p[0] >= p[1]), "du plus grand au plus petit : {tailles:?}");
        assert!(tailles.iter().all(|&t| t >= 1));
        assert_eq!(plus_grand_morceau(largeur, hauteur, &occupees), tailles.first().copied().unwrap_or(0));
        // Une traversée tient dans un morceau, et ne peut pas faire moins de cases que la largeur (ou la hauteur).
        if let Some(n) = traversee(largeur, hauteur, &occupees, Bord::Gauche, Bord::Droite) {
            assert!(n >= largeur && n <= tailles[0], "gauche-droite : {n} cases pour une largeur de {largeur}");
        }
        if let Some(n) = traversee(largeur, hauteur, &occupees, Bord::Haut, Bord::Bas) {
            assert!(n >= hauteur && n <= tailles[0], "haut-bas : {n} cases pour une hauteur de {hauteur}");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Une grande boîte
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_grande_boite_pleine_ne_fait_pas_deborder_la_pile() {
    delai();
    let (l, h) = (161usize, 161usize);
    let pleine = vec![true; l * h];
    assert_eq!(morceaux(l, h, &pleine), vec![l * h]);
    assert_eq!(plus_grand_morceau(l, h, &pleine), l * h);
    assert_eq!(traversee(l, h, &pleine, Bord::Gauche, Bord::Droite), Some(161));
    assert_eq!(traversee(l, h, &pleine, Bord::Bas, Bord::Haut), Some(161));
    let vide = vec![false; l * h];
    assert!(morceaux(l, h, &vide).is_empty());
    assert_eq!(traversee(l, h, &vide, Bord::Gauche, Bord::Droite), None);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_serpentin_de_treize_mille_cases_est_un_seul_morceau() {
    delai();
    // Les lignes paires sont pleines ; sur la ligne 2k + 1, une seule case relie les deux lignes pleines voisines, à
    // droite quand k est pair, à gauche quand k est impair.
    let (l, h) = (161usize, 161usize);
    let mut serpentin = vec![false; l * h];
    for y in 0..h {
        for x in 0..l {
            let plein = if y % 2 == 0 {
                true
            } else if (y / 2) % 2 == 0 {
                x == l - 1
            } else {
                x == 0
            };
            serpentin[y * l + x] = plein;
        }
    }
    let cases = 81 * 161 + 80;
    assert_eq!(serpentin.iter().filter(|&&o| o).count(), cases);
    assert_eq!(morceaux(l, h, &serpentin), vec![cases]);
    assert_eq!(plus_grand_morceau(l, h, &serpentin), cases);
    // De gauche à droite : la première ligne, tout droit.
    assert_eq!(traversee(l, h, &serpentin, Bord::Gauche, Bord::Droite), Some(161));
    // Du haut au bas : il faut suivre tout le serpentin. On entre par (159, 0) ou (160, 0) (1 case), 80 cases de liaison,
    // 79 lignes pleines intermédiaires de 159 cases (en diagonale d'une liaison à l'autre) et la dernière ligne (1 case).
    assert_eq!(traversee(l, h, &serpentin, Bord::Haut, Bord::Bas), Some(1 + 80 + 79 * 159 + 1));
    assert_eq!(traversee(l, h, &serpentin, Bord::Bas, Bord::Haut), Some(1 + 80 + 79 * 159 + 1));
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_grande_boite_au_hasard_suit_l_union_de_classes() {
    delai();
    let mut h = Hasard::new(161);
    for proportion in [45, 55, 65] {
        let (l, ht) = (161usize, 161usize);
        let occupees: Vec<bool> = (0..l * ht).map(|_| h.avec_proba(proportion, 100)).collect();
        assert_eq!(morceaux(l, ht, &occupees), morceaux_de_reference(l, ht, &occupees), "{proportion} %");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn une_boite_sans_case_n_a_ni_morceau_ni_traversee() {
    delai();
    for (l, ht) in [(0usize, 0usize), (0, 5), (5, 0)] {
        assert!(!appel!(morceaux(l, ht, &[])), "{l} × {ht} : pas de panique");
        assert_eq!(morceaux(l, ht, &[]), Vec::<usize>::new(), "{l} × {ht}");
        assert_eq!(plus_grand_morceau(l, ht, &[]), 0, "{l} × {ht}");
        for de in 0..4 {
            for vers in 0..4 {
                if de != vers {
                    assert_eq!(traversee(l, ht, &[], bord(de), bord(vers)), None, "{l} × {ht}");
                }
            }
        }
        // La longueur reste contrôlée : une case de trop panique.
        assert!(appel!(morceaux(l, ht, &[true])), "{l} × {ht} : une case de trop");
        assert!(appel!(traversee(l, ht, &[true], Bord::Gauche, Bord::Droite)), "{l} × {ht} : une case de trop");
    }
}
