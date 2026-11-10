//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « les obstacles ».
//!
//! Ils fixent `obstacles`, la décomposition du complément de la cavité en rectangles :
//!
//! - **des cas écrits à la main**, avec la liste exacte des rectangles : une brique (qui prolonge verticalement les deux
//!   bandes latérales), un disque de rayon 1 (aucune suite ne prolonge la précédente : huit rectangles), deux rectangles de
//!   même largeur que sépare une ligne libre (qui ne fusionnent pas), un L dont le tri `(y0, x0)` n'est pas l'ordre de
//!   fermeture, un anneau, une boîte d'une case ;
//! - **aucun obstacle** quand la cavité est toute la boîte ;
//! - **les propriétés** : les rectangles sont à l'endroit, dans la boîte, **disjoints**, triés par `(y0, x0)`, et leur union est
//!   exactement le complément de la cavité (un oracle naïf : les cases bloquées une à une) ;
//! - **la décomposition de la spécification, comparée à celle de `outils.rs`** (une table de rectangles ouverts, écrite
//!   autrement), pour des formes au hasard : briques, L, disques, anneaux **et labyrinthes**, dans des boîtes de toute
//!   forme ;
//! - **les erreurs** de `libres` sont celles de `obstacles`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_10/outils.rs"]
mod outils;

use mycelium_lab::moule::{ErreurMoule, Forme, libres, obstacles};
use mycelium_lab::simulation::Obstacle;
use outils::{Hasard, compter, couverture, decomposition_de_reference, delai, libres_de_reference, obstacle};

fn brique(x0: usize, y0: usize, x1: usize, y1: usize) -> Forme {
    Forme::Brique { x0, y0, x1, y1 }
}

/// Les obstacles d'une forme dans une boîte, après avoir vérifié les propriétés de la spécification.
fn obstacles_verifies(largeur: usize, hauteur: usize, forme: &Forme) -> Vec<Obstacle> {
    let rectangles = obstacles(largeur, hauteur, forme).expect("la forme est valide et dans la boîte");
    let cavite = libres(largeur, hauteur, forme).expect("la forme est valide et dans la boîte");
    // À l'endroit, dans la boîte (panique sinon), et la couverture de chaque case : 1 si bloquée, 0 si libre. Disjoints.
    let couvertes = couverture(largeur, hauteur, &rectangles);
    for (i, &n) in couvertes.iter().enumerate() {
        assert!(n <= 1, "la case ({}, {}) est couverte {n} fois : {rectangles:?}", i % largeur, i / largeur);
        assert_eq!(
            n == 1,
            !cavite[i],
            "la case ({}, {}) : bloquée ssi couverte par un obstacle",
            i % largeur,
            i / largeur
        );
    }
    // Triés par (y0, x0) croissants.
    for paire in rectangles.windows(2) {
        assert!(
            (paire[0].y0, paire[0].x0) < (paire[1].y0, paire[1].x0),
            "trié par (y0, x0) : {:?} avant {:?}",
            paire[0],
            paire[1]
        );
    }
    rectangles
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_brique_prolonge_ses_bandes_laterales_verticalement() {
    delai();
    // 5 × 4, brique x 1..=3, y 1..=2 :
    //   #####
    //   #...#
    //   #...#
    //   #####
    // Les cases bloquées de la ligne 1 (x = 0 et x = 4) prolongent celles de la ligne 2 : un rectangle chacune.
    assert_eq!(
        obstacles_verifies(5, 4, &brique(1, 1, 3, 2)),
        vec![obstacle(0, 0, 4, 0), obstacle(0, 1, 0, 2), obstacle(4, 1, 4, 2), obstacle(0, 3, 4, 3)]
    );
    // 6 × 5, brique x 2..=3, y 0..=4 : deux bandes tout en hauteur, sans rien au-dessus ni au-dessous.
    assert_eq!(obstacles_verifies(6, 5, &brique(2, 0, 3, 4)), vec![obstacle(0, 0, 1, 4), obstacle(4, 0, 5, 4)]);
    // Une brique contre le haut et la gauche : un L de deux rectangles (la bande de droite d'abord : y0 = 0).
    assert_eq!(obstacles_verifies(4, 3, &brique(0, 0, 1, 1)), vec![obstacle(2, 0, 3, 1), obstacle(0, 2, 3, 2)]);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_suite_de_largeur_differente_ne_prolonge_pas_le_rectangle_precedent() {
    delai();
    // Un disque de rayon 1 centré en (2, 2) :
    //   #####
    //   ##.##
    //   #...#
    //   ##.##
    //   #####
    // Aucune suite n'a le même (xa, xb) que celle de la ligne d'avant : huit rectangles.
    assert_eq!(
        obstacles_verifies(5, 5, &Forme::Disque { cx: 2, cy: 2, rayon: 1 }),
        vec![
            obstacle(0, 0, 4, 0),
            obstacle(0, 1, 1, 1),
            obstacle(3, 1, 4, 1),
            obstacle(0, 2, 0, 2),
            obstacle(4, 2, 4, 2),
            obstacle(0, 3, 1, 3),
            obstacle(3, 3, 4, 3),
            obstacle(0, 4, 4, 4),
        ]
    );
    // Deux suites de même (xa, xb) que sépare une ligne libre ne fusionnent pas : lignes 0 et 2 d'une boîte 5 × 3.
    assert_eq!(obstacles_verifies(5, 3, &brique(0, 1, 4, 1)), vec![obstacle(0, 0, 4, 0), obstacle(0, 2, 4, 2)]);
    // Même chose en colonnes : deux colonnes bloquées de même hauteur, séparées par une colonne libre.
    assert_eq!(obstacles_verifies(3, 4, &brique(1, 0, 1, 3)), vec![obstacle(0, 0, 0, 3), obstacle(2, 0, 2, 3)]);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_rectangles_sont_rendus_par_y0_puis_x0_et_non_dans_l_ordre_de_fermeture() {
    delai();
    // Un L dans 4 × 4 (rectangle x 1..=3, y 0..=3, épaisseur 1) : la colonne 1 et la ligne 3 sont libres.
    //   #...   hmm : voir le dessin ci-dessous
    //   ##..   (les colonnes 2 et 3 des lignes 0 à 2 sont bloquées, la colonne 0 aussi)
    // Le rectangle de droite (2..=3, 0..=2) se ferme à la ligne 3, avant la colonne 0 (0..=0, 0..=3) qui se ferme à la fin :
    // l'ordre de fermeture est donc (2, 0) puis (0, 0) ; l'ordre demandé est par x0 croissant à y0 égal.
    let l = Forme::L { x0: 1, y0: 0, x1: 3, y1: 3, epaisseur: 1 };
    assert_eq!(obstacles_verifies(4, 4, &l), vec![obstacle(0, 0, 0, 3), obstacle(2, 0, 3, 2)]);
    // Même idée avec un L décalé d'une ligne : #### / #.## / #.## / #...  (colonne 0 puis bloc de droite, tous deux à y0 = 1).
    let l = Forme::L { x0: 1, y0: 1, x1: 3, y1: 3, epaisseur: 1 };
    assert_eq!(obstacles_verifies(4, 4, &l), vec![obstacle(0, 0, 3, 0), obstacle(0, 1, 0, 3), obstacle(2, 1, 3, 2)]);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_anneau_et_un_labyrinthe_a_la_main() {
    delai();
    // Un anneau en losange (centre (2, 2), rayons 2 et 1) dans 5 × 5 :
    //   ##.##
    //   #.#.#
    //   .###.
    //   #.#.#
    //   ##.##
    assert_eq!(
        obstacles_verifies(5, 5, &Forme::Anneau { cx: 2, cy: 2, rayon_exterieur: 2, rayon_interieur: 1 }),
        vec![
            obstacle(0, 0, 1, 0),
            obstacle(3, 0, 4, 0),
            obstacle(0, 1, 0, 1),
            obstacle(2, 1, 2, 1),
            obstacle(4, 1, 4, 1),
            obstacle(1, 2, 3, 2),
            obstacle(0, 3, 0, 3),
            obstacle(2, 3, 2, 3),
            obstacle(4, 3, 4, 3),
            obstacle(0, 4, 1, 4),
            obstacle(3, 4, 4, 4),
        ]
    );
    // Le plus petit labyrinthe (1 × 1, couloir 1) : une chambre au centre d'une boîte 3 × 3 entourée de mur.
    assert_eq!(
        obstacles_verifies(3, 3, &Forme::Labyrinthe { colonnes: 1, rangees: 1, couloir: 1, graine: 0 }),
        vec![obstacle(0, 0, 2, 0), obstacle(0, 1, 0, 1), obstacle(2, 1, 2, 1), obstacle(0, 2, 2, 2)]
    );
    // Le même, dans une boîte plus grande et non carrée : les cases au-delà de W et de H sont bloquées. Le mur du bas
    // (ligne 2) se prolonge sur la ligne 3, et les bandes au-delà de la colonne 2 sont des rectangles à part.
    assert_eq!(
        obstacles_verifies(5, 4, &Forme::Labyrinthe { colonnes: 1, rangees: 1, couloir: 1, graine: 0 }),
        vec![obstacle(0, 0, 4, 0), obstacle(0, 1, 0, 1), obstacle(2, 1, 4, 1), obstacle(0, 2, 4, 3)]
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn aucun_obstacle_quand_la_cavite_est_toute_la_boite() {
    delai();
    assert_eq!(obstacles_verifies(7, 4, &brique(0, 0, 6, 3)), Vec::<Obstacle>::new());
    assert_eq!(obstacles_verifies(1, 1, &brique(0, 0, 0, 0)), Vec::<Obstacle>::new());
    assert_eq!(obstacles_verifies(1, 1, &Forme::Disque { cx: 0, cy: 0, rayon: 0 }), Vec::<Obstacle>::new());
    assert_eq!(
        obstacles_verifies(4, 4, &Forme::L { x0: 0, y0: 0, x1: 3, y1: 3, epaisseur: 4 }),
        Vec::<Obstacle>::new()
    );
    assert_eq!(
        obstacles_verifies(5, 2, &Forme::L { x0: 0, y0: 0, x1: 4, y1: 1, epaisseur: 2 }),
        Vec::<Obstacle>::new()
    );
    // Un seul obstacle quand la cavité est une bande qui touche trois bords.
    assert_eq!(obstacles_verifies(5, 4, &brique(0, 0, 4, 1)), vec![obstacle(0, 2, 4, 3)]);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_erreurs_des_formes_sont_celles_de_libres() {
    delai();
    assert_eq!(obstacles(5, 5, &brique(3, 0, 2, 0)), Err(ErreurMoule::FormeInvalide));
    assert_eq!(obstacles(5, 5, &brique(0, 0, 5, 0)), Err(ErreurMoule::HorsDeLaBoite));
    assert_eq!(obstacles(0, 5, &brique(0, 0, 0, 0)), Err(ErreurMoule::HorsDeLaBoite));
    assert_eq!(obstacles(5, 0, &brique(0, 0, 0, 0)), Err(ErreurMoule::HorsDeLaBoite));
}

/// Une forme au hasard (valide, qui tient dans une boîte au hasard) : rend `(largeur, hauteur, forme)`.
fn forme_au_hasard(hasard: &mut Hasard) -> (usize, usize, Forme) {
    loop {
        let (largeur, hauteur) = (hasard.entre(1, 26), hasard.entre(1, 26));
        let forme = match hasard.jusqu_a(5) {
            0 => {
                let (x0, y0) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                brique(x0, y0, hasard.entre(x0, largeur - 1), hasard.entre(y0, hauteur - 1))
            }
            1 => {
                let (x0, y0) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let (x1, y1) = (hasard.entre(x0, largeur - 1), hasard.entre(y0, hauteur - 1));
                Forme::L { x0, y0, x1, y1, epaisseur: hasard.entre(1, (x1 - x0 + 1).min(y1 - y0 + 1)) }
            }
            2 => {
                let (cx, cy) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let maximum = cx.min(cy).min(largeur - 1 - cx).min(hauteur - 1 - cy);
                Forme::Disque { cx, cy, rayon: hasard.entre(0, maximum) }
            }
            3 => {
                let (cx, cy) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let maximum = cx.min(cy).min(largeur - 1 - cx).min(hauteur - 1 - cy);
                if maximum == 0 {
                    continue;
                }
                let exterieur = hasard.entre(1, maximum);
                Forme::Anneau { cx, cy, rayon_exterieur: exterieur, rayon_interieur: hasard.entre(0, exterieur - 1) }
            }
            _ => {
                let couloir = hasard.entre(1, 3);
                let pas = couloir + 1;
                let (colonnes_max, rangees_max) = ((largeur - 1) / pas, (hauteur - 1) / pas);
                if colonnes_max == 0 || rangees_max == 0 {
                    continue;
                }
                Forme::Labyrinthe {
                    colonnes: hasard.entre(1, colonnes_max),
                    rangees: hasard.entre(1, rangees_max),
                    couloir,
                    graine: hasard.suivant(),
                }
            }
        };
        return (largeur, hauteur, forme);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn des_formes_au_hasard_ont_la_decomposition_de_reference() {
    delai();
    let mut hasard = Hasard::new(77);
    let mut par_forme = [0usize; 5];
    let mut avec_obstacles = 0;
    let mut avec_prolongement = 0;
    for _ in 0..1500 {
        let (largeur, hauteur, forme) = forme_au_hasard(&mut hasard);
        let rectangles = obstacles_verifies(largeur, hauteur, &forme);
        // La décomposition exacte de la spécification, par une table de rectangles ouverts (outils.rs).
        let libres_de_reference = libres_de_reference(largeur, hauteur, &forme);
        assert_eq!(
            rectangles,
            decomposition_de_reference(largeur, hauteur, &libres_de_reference),
            "{forme:?} dans {largeur} × {hauteur}"
        );
        // L'oracle naïf : les cases bloquées une à une, par rectangle.
        let mut bloquees = vec![false; largeur * hauteur];
        for o in &rectangles {
            for y in o.y0..=o.y1 {
                for x in o.x0..=o.x1 {
                    bloquees[y * largeur + x] = true;
                }
            }
        }
        let nombre_de_bloquees = bloquees.iter().filter(|&&b| b).count();
        assert_eq!(nombre_de_bloquees + compter(&libres_de_reference), largeur * hauteur, "{forme:?}");
        par_forme[match forme {
            Forme::Brique { .. } => 0,
            Forme::L { .. } => 1,
            Forme::Disque { .. } => 2,
            Forme::Anneau { .. } => 3,
            Forme::Labyrinthe { .. } => 4,
        }] += 1;
        if !rectangles.is_empty() {
            avec_obstacles += 1;
        }
        if rectangles.iter().any(|o| o.y1 > o.y0) {
            avec_prolongement += 1;
        }
    }
    assert!(par_forme.iter().all(|&n| n >= 100), "garde : chaque forme est tirée assez souvent : {par_forme:?}");
    assert!(avec_obstacles >= 1000, "garde : la plupart des formes laissent des obstacles : {avec_obstacles}");
    assert!(avec_prolongement >= 600, "garde : des rectangles prolongés verticalement : {avec_prolongement}");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_decomposition_ne_depend_que_de_la_forme_et_de_la_boite() {
    delai();
    // Deux appels donnent les mêmes rectangles, dans le même ordre, et ce sont ceux de la référence.
    let mut hasard = Hasard::new(5);
    for _ in 0..100 {
        let (largeur, hauteur, forme) = forme_au_hasard(&mut hasard);
        let premier = obstacles(largeur, hauteur, &forme).expect("la forme est valide et dans la boîte");
        assert_eq!(obstacles(largeur, hauteur, &forme).unwrap(), premier);
        let reference = decomposition_de_reference(largeur, hauteur, &libres_de_reference(largeur, hauteur, &forme));
        assert_eq!(premier, reference, "{forme:?} dans {largeur} × {hauteur}");
    }
}
