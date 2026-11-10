//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « dans la simulation ».
//!
//! Sur les cinq scénarios de `scenarios/moules/` (brique, L, disque, anneau, labyrinthe), avec plusieurs graines, ils fixent ce
//! que M1 ajoute au-dessus du noyau sans le changer :
//!
//! - **les invariants de M0 tiennent dans tout moule** : la somme de la carte de densité est la biomasse (relative 1e-9) ; la
//!   densité est nulle dans toute case bloquée ; le bilan de nourriture tient **à chaque pas** ; aucun segment ne finit (ni ne
//!   commence) dans une case bloquée ; les cases bloquées de la simulation sont exactement le complément de la cavité de
//!   référence ;
//! - **les scénarios sont ceux de la spécification** : la boîte (100 × 100, le labyrinthe 25 × 25), la spore dans la cavité, la
//!   taille de la cavité (formule), la durée ;
//! - **la courbe de remplissage** : `duree + 1` points, un par pas, `cases_libres` égal à la taille de la cavité,
//!   `remplissage = cases_occupees / cases_libres` au bit près et au plus 1, **croissante au sens large** (aucune case ne
//!   se vide : l'ensemble des cases occupées ne fait que grandir), son premier point est le temps 0 (rien d'occupé), son
//!   dernier point est celui de `mesurer`, et chaque point est celui de `mesurer` sur une jumelle jouée pas à pas ; le cas
//!   `jusqu_au_pas ≤ temps()` (un point, sans avancer), la reprise d'une simulation déjà avancée ; la courbe ne change pas
//!   l'état (même empreinte qu'une jumelle jouée sans elle) ;
//! - **`pas_pour_remplir`** : des courbes écrites à la main (dont `part = 0` → le premier point, `None`, la comparaison sans
//!   tolérance, la panique hors de `[0, 1]`), puis les vraies courbes contre une recherche faite ici ;
//! - **de quoi voir quelque chose** : la colonie remplit une bonne part de chaque cavité.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `floor`, `sqrt`, `min` et `max`.

#[path = "etape_10/outils.rs"]
mod outils;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::moule::{PointDeCourbe, courbe_de_remplissage, pas_pour_remplir};
use mycelium_lab::piece::{carte_de_densite, mesurer};
use outils::{
    cases_bloquees, construire_moule, delai, libres_de_reference, proche_relatif, scenarios_de_moule,
    somme_dans_l_ordre,
};

/// Les graines jouées sur chaque scénario.
const GRAINES: [u64; 3] = [1, 2, 3];

fn panique<F: FnOnce()>(f: F) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

// ---------------------------------------------------------------------------------------------------------------------
// Les scénarios et les invariants de M0
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_cinq_scenarios_sont_ceux_de_la_specification() {
    delai();
    for s in scenarios_de_moule() {
        let (simulation, reglages, duree) = construire_moule(s.nom, 1);
        assert_eq!((reglages.largeur, reglages.hauteur), s.boite, "{} : la boîte", s.nom);
        assert_eq!(reglages.spore, s.spore, "{} : la spore", s.nom);
        assert_eq!(duree, s.duree, "{} : la durée", s.nom);
        // La spore est dans la cavité, et la cavité a la taille que dit la formule.
        let libres = libres_de_reference(s.boite.0, s.boite.1, &s.forme);
        assert!(libres[s.spore.1 * s.boite.0 + s.spore.0], "{} : la spore est dans la cavité", s.nom);
        assert_eq!(outils::compter(&libres), s.cases_libres, "{} : la taille de la cavité", s.nom);
        // Les cases bloquées de la simulation sont exactement le complément de la cavité.
        let bloquees = cases_bloquees(&simulation);
        let complement: Vec<bool> = libres.iter().map(|&l| !l).collect();
        assert_eq!(bloquees, complement, "{} : les cases bloquées sont le complément de la cavité", s.nom);
        // Les réglages des scénarios existants.
        assert_eq!((reglages.substrat_initial, reglages.diffusion), (1.0, 0.2), "{}", s.nom);
        assert_eq!((reglages.affinite, reglages.cout), (0.1, 0.002), "{}", s.nom);
        assert_eq!(reglages.croissance.pas, 1.0, "{}", s.nom);
        assert_eq!(
            (reglages.croissance.angle_ramification, reglages.croissance.amplitude_marche),
            (30, 20),
            "{}",
            s.nom
        );
        let anastomose = reglages.anastomose.expect("anastomose 0.5 10");
        assert_eq!((anastomose.rayon, anastomose.delai), (0.5, 10), "{}", s.nom);
        assert_eq!(simulation.temps(), 0);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_carte_la_biomasse_et_le_bilan_de_nourriture_tiennent_dans_chaque_moule() {
    delai();
    for s in scenarios_de_moule() {
        for graine in GRAINES {
            let (mut simulation, reglages, duree) = construire_moule(s.nom, graine);
            let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
            let bloquees = cases_bloquees(&simulation);
            let initiale = simulation.releves()[0].substrat;
            let mut deja_vus = 0;
            for pas in 1..=duree {
                simulation.avancer();
                // Le bilan de nourriture, à chaque pas.
                let ecart =
                    (simulation.substrat().masse_totale() + reglages.cout * simulation.biomasse() - initiale).abs();
                assert!(
                    ecart <= 1e-9 * initiale,
                    "{}, graine {graine}, pas {pas} : le bilan s'écarte de {ecart}",
                    s.nom
                );
                // Aucun segment nouveau ne commence ni ne finit dans une case bloquée.
                for segment in &simulation.segments()[deja_vus..] {
                    for (x, y) in [(segment.x0, segment.y0), (segment.x1, segment.y1)] {
                        let case = (x.floor() as usize, y.floor() as usize);
                        assert!(
                            !bloquees[case.1 * largeur + case.0],
                            "{}, graine {graine}, pas {pas} : un segment touche la case bloquée {case:?}",
                            s.nom
                        );
                    }
                }
                deja_vus = simulation.segments().len();
                // La carte de densité, tous les 25 pas et à la fin.
                if pas % 25 == 0 || pas == duree {
                    let carte = carte_de_densite(largeur, hauteur, simulation.segments());
                    assert!(
                        proche_relatif(carte.somme(), simulation.biomasse(), 1e-9),
                        "{}, graine {graine}, pas {pas} : somme de la carte {} contre biomasse {}",
                        s.nom,
                        carte.somme(),
                        simulation.biomasse()
                    );
                    assert!(proche_relatif(somme_dans_l_ordre(carte.valeurs()), carte.somme(), 1e-12));
                    for (i, &densite) in carte.valeurs().iter().enumerate() {
                        assert!(densite >= 0.0, "{} : densité négative en {i}", s.nom);
                        assert!(
                            !bloquees[i] || densite == 0.0,
                            "{}, graine {graine}, pas {pas} : densité {densite} dans la case bloquée ({}, {})",
                            s.nom,
                            i % largeur,
                            i / largeur
                        );
                    }
                }
            }
            assert_eq!(simulation.temps(), duree);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// La courbe de remplissage
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_courbe_de_remplissage_est_croissante_et_se_termine_sur_mesurer() {
    delai();
    for s in scenarios_de_moule() {
        for graine in GRAINES {
            // La durée entière pour la graine 1 ; 100 pas pour les autres (la courbe entière est longue à mesurer).
            let (mut simulation, _, duree) = construire_moule(s.nom, graine);
            let (mut jumelle, _, _) = construire_moule(s.nom, graine);
            let duree = if graine == 1 { duree } else { 100 };
            let courbe = courbe_de_remplissage(&mut simulation, duree);
            let contexte = format!("{}, graine {graine}", s.nom);
            assert_eq!(courbe.len(), duree as usize + 1, "{contexte} : un point par pas, le pas 0 compris");
            assert_eq!(simulation.temps(), duree, "{contexte} : la simulation a avancé jusqu'à la durée");
            for (i, point) in courbe.iter().enumerate() {
                assert_eq!(point.pas, i as u32, "{contexte} : les pas se suivent");
                assert_eq!(point.cases_libres, s.cases_libres, "{contexte}, pas {i} : la taille de la cavité");
                assert!(
                    point.cases_occupees <= point.cases_libres,
                    "{contexte}, pas {i} : plus d'occupées que de libres"
                );
                assert!(point.remplissage >= 0.0 && point.remplissage <= 1.0, "{contexte}, pas {i} : remplissage");
                assert_eq!(
                    point.remplissage.to_bits(),
                    (point.cases_occupees as f64 / point.cases_libres as f64).to_bits(),
                    "{contexte}, pas {i} : remplissage = occupées / libres"
                );
            }
            // Le premier point : le temps 0, rien d'occupé.
            assert_eq!(
                courbe[0],
                PointDeCourbe { pas: 0, cases_occupees: 0, cases_libres: s.cases_libres, remplissage: 0.0 }
            );
            // Croissante au sens large.
            for paire in courbe.windows(2) {
                assert!(
                    paire[1].cases_occupees >= paire[0].cases_occupees,
                    "{contexte}, pas {} : une case s'est vidée ({} → {})",
                    paire[1].pas,
                    paire[0].cases_occupees,
                    paire[1].cases_occupees
                );
                assert!(paire[1].remplissage >= paire[0].remplissage);
            }
            // Le dernier point est celui de `mesurer`, sur la jumelle jouée sans la courbe.
            jumelle.avancer_n(duree);
            let m = mesurer(&jumelle);
            let dernier = courbe.last().unwrap();
            assert_eq!(dernier.cases_occupees, m.cases_occupees, "{contexte}");
            assert_eq!(dernier.cases_libres, m.cases_libres, "{contexte}");
            assert_eq!(dernier.remplissage.to_bits(), m.remplissage.to_bits(), "{contexte}");
            assert_eq!(dernier.pas, jumelle.temps(), "{contexte}");
            // La courbe ne change rien à l'état : la même empreinte qu'une jumelle jamais mesurée.
            assert_eq!(
                simulation.empreinte(),
                jumelle.empreinte(),
                "{contexte} : la courbe ne change pas la simulation"
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn chaque_point_de_la_courbe_est_celui_de_mesurer_a_ce_pas_et_aucune_case_ne_se_vide() {
    delai();
    for s in scenarios_de_moule() {
        let (mut simulation, reglages, _) = construire_moule(s.nom, 2);
        let (mut jumelle, _, _) = construire_moule(s.nom, 2);
        let courbe = courbe_de_remplissage(&mut simulation, 70);
        assert_eq!(courbe.len(), 71);
        let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
        let mut avant = vec![false; largeur * hauteur];
        for (pas, point) in courbe.iter().enumerate() {
            if pas > 0 {
                jumelle.avancer();
            }
            let m = mesurer(&jumelle);
            assert_eq!(point.cases_occupees, m.cases_occupees, "{}, pas {pas}", s.nom);
            assert_eq!(point.cases_libres, m.cases_libres, "{}, pas {pas}", s.nom);
            assert_eq!(point.remplissage.to_bits(), m.remplissage.to_bits(), "{}, pas {pas}", s.nom);
            // Aucune case ne se vide : l'ensemble des cases occupées ne fait que grandir.
            let carte = carte_de_densite(largeur, hauteur, jumelle.segments());
            let occupees = carte.occupees(0.0);
            for (i, (&avant_i, &maintenant)) in avant.iter().zip(&occupees).enumerate() {
                assert!(
                    !avant_i || maintenant,
                    "{}, pas {pas} : la case ({}, {}) s'est vidée",
                    s.nom,
                    i % largeur,
                    i / largeur
                );
            }
            avant = occupees;
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_courbe_jusqu_a_un_pas_deja_atteint_est_un_seul_point_sans_avancer() {
    delai();
    let (mut simulation, _, _) = construire_moule("disque", 1);
    // Au temps 0 : jusqu'au pas 0, un seul point, rien n'avance.
    let courbe = courbe_de_remplissage(&mut simulation, 0);
    assert_eq!(courbe, vec![PointDeCourbe { pas: 0, cases_occupees: 0, cases_libres: 1961, remplissage: 0.0 }]);
    assert_eq!(simulation.temps(), 0);
    simulation.avancer_n(30);
    let empreinte = simulation.empreinte();
    // Après 30 pas : jusqu'au pas 30, jusqu'à 29, jusqu'à 0 : un seul point, celui du temps courant, sans avancer.
    let m = mesurer(&simulation);
    for jusqu_au_pas in [30u32, 29, 1, 0] {
        let courbe = courbe_de_remplissage(&mut simulation, jusqu_au_pas);
        assert_eq!(courbe.len(), 1, "jusqu'au pas {jusqu_au_pas}");
        assert_eq!(courbe[0].pas, 30);
        assert_eq!(courbe[0].cases_occupees, m.cases_occupees);
        assert_eq!(courbe[0].remplissage.to_bits(), m.remplissage.to_bits());
        assert_eq!(simulation.temps(), 30, "la simulation n'a pas avancé");
        assert_eq!(simulation.empreinte(), empreinte, "l'état n'a pas changé");
    }
    // Reprise : de 30 à 45, seize points (pas 30, 31, …, 45), et la simulation est au pas 45.
    let suite = courbe_de_remplissage(&mut simulation, 45);
    assert_eq!(suite.len(), 16);
    assert_eq!(suite.iter().map(|p| p.pas).collect::<Vec<_>>(), (30..=45).collect::<Vec<u32>>());
    assert_eq!(suite[0], courbe_de_remplissage(&mut construire_moule("disque", 1).0, 30)[30]);
    assert_eq!(simulation.temps(), 45);
    // Un pas de plus : deux points.
    let deux = courbe_de_remplissage(&mut simulation, 46);
    assert_eq!(deux.len(), 2);
    assert_eq!((deux[0].pas, deux[1].pas), (45, 46));
    assert_eq!(suite[15], deux[0]);
}

// ---------------------------------------------------------------------------------------------------------------------
// pas_pour_remplir
// ---------------------------------------------------------------------------------------------------------------------

/// Une courbe à la main : `(pas, remplissage)`, sur 100 cases libres (les occupées sont le remplissage × 100).
fn courbe(points: &[(u32, f64)]) -> Vec<PointDeCourbe> {
    points
        .iter()
        .map(|&(pas, remplissage)| PointDeCourbe {
            pas,
            cases_occupees: (remplissage * 100.0) as usize,
            cases_libres: 100,
            remplissage,
        })
        .collect()
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn pas_pour_remplir_rend_le_premier_pas_qui_atteint_la_part() {
    delai();
    let c = courbe(&[(0, 0.0), (1, 0.1), (2, 0.5), (3, 0.5), (4, 0.9), (5, 1.0)]);
    // part = 0 : le premier point, même vide.
    assert_eq!(pas_pour_remplir(&c, 0.0), Some(0));
    assert_eq!(pas_pour_remplir(&c, -0.0), Some(0));
    // Atteindre : ≥, sans tolérance, au premier pas qui y arrive.
    assert_eq!(pas_pour_remplir(&c, 0.1), Some(1), "exactement 0,1 : atteint au pas 1");
    assert_eq!(pas_pour_remplir(&c, 0.05), Some(1));
    assert_eq!(pas_pour_remplir(&c, 0.5), Some(2), "le premier des deux points à 0,5");
    assert_eq!(pas_pour_remplir(&c, 0.5000001), Some(4));
    assert_eq!(pas_pour_remplir(&c, 0.9), Some(4));
    assert_eq!(pas_pour_remplir(&c, 0.95), Some(5));
    assert_eq!(pas_pour_remplir(&c, 1.0), Some(5));
    // Une courbe qui n'y arrive pas.
    let partielle = courbe(&[(0, 0.0), (1, 0.3), (2, 0.3), (3, 0.7)]);
    assert_eq!(pas_pour_remplir(&partielle, 0.7), Some(3));
    assert_eq!(pas_pour_remplir(&partielle, 0.71), None);
    assert_eq!(pas_pour_remplir(&partielle, 1.0), None);
    // Une courbe vide.
    assert_eq!(pas_pour_remplir(&[], 0.0), None);
    assert_eq!(pas_pour_remplir(&[], 0.5), None);
    // Une courbe qui ne commence pas au pas 0 (une simulation déjà avancée) : le pas du point, pas son rang.
    let reprise = courbe(&[(7, 0.2), (8, 0.4), (9, 0.6)]);
    assert_eq!(pas_pour_remplir(&reprise, 0.0), Some(7));
    assert_eq!(pas_pour_remplir(&reprise, 0.4), Some(8));
    assert_eq!(pas_pour_remplir(&reprise, 0.6), Some(9));
    // Pas de tolérance : 0,30000000000000004 atteint 0,3 ; 0,29999999999999993 ne l'atteint pas.
    let fin = courbe(&[(0, 0.0), (1, 0.29999999999999993), (2, 0.30000000000000004)]);
    assert_eq!(pas_pour_remplir(&fin, 0.3), Some(2));
    // Un remplissage de 1 sur une seule case.
    assert_eq!(pas_pour_remplir(&courbe(&[(4, 1.0)]), 1.0), Some(4));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn pas_pour_remplir_refuse_une_part_hors_de_zero_un() {
    delai();
    let c = courbe(&[(0, 0.0), (1, 0.5)]);
    for part in [-0.0000001, -1.0, 1.0000001, 2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, f64::MIN] {
        assert!(panique(|| _ = pas_pour_remplir(&c, part)), "la part {part} fait paniquer");
        assert!(panique(|| _ = pas_pour_remplir(&[], part)), "la part {part} fait paniquer même sur une courbe vide");
    }
    for part in [0.0, 1.0, 0.5, f64::MIN_POSITIVE, 0.9999999999999999] {
        assert!(!panique(|| _ = pas_pour_remplir(&c, part)), "la part {part} est permise");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn pas_pour_remplir_sur_les_vraies_courbes_est_le_premier_pas_qui_atteint() {
    delai();
    let mut atteints = 0;
    let mut manques = 0;
    for s in scenarios_de_moule() {
        // La graine 1 sur la durée entière, et la graine 2 sur 120 pas.
        for (graine, jusqu_au_pas) in [(1u64, None), (2, Some(120))] {
            let (mut simulation, _, duree) = construire_moule(s.nom, graine);
            let c = courbe_de_remplissage(&mut simulation, jusqu_au_pas.unwrap_or(duree));
            for part in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99, 1.0] {
                // Recherche faite ici : le plus petit indice où occupées / libres ≥ part.
                let attendu = (0..c.len()).find(|&i| c[i].cases_occupees as f64 / c[i].cases_libres as f64 >= part);
                let obtenu = pas_pour_remplir(&c, part);
                assert_eq!(obtenu, attendu.map(|i| i as u32), "{}, graine {graine}, part {part}", s.nom);
                if let Some(pas) = obtenu {
                    atteints += 1;
                    assert!(c[pas as usize].remplissage >= part);
                    assert!(pas == 0 || c[pas as usize - 1].remplissage < part, "le pas {pas} est le premier");
                } else {
                    manques += 1;
                    assert!(c.iter().all(|p| p.remplissage < part));
                }
            }
        }
    }
    assert!(
        atteints >= 40 && manques >= 3,
        "garde : des parts atteintes et des parts manquées : {atteints} / {manques}"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// De quoi voir quelque chose
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_colonie_remplit_une_bonne_part_de_chaque_cavite() {
    delai();
    // Les réglages des scénarios (`croissance 1.0 8.0 30 20`, `thigmotropisme 35`) ont été choisis sur 30 graines pour que la
    // colonie remplisse la cavité avant de s'éteindre ; ici, sur les graines 1 à 4, un plancher sous le plus mauvais des 30
    // (brique 68 %, L 84 %, disque 69 %, anneau 88 %, labyrinthe 96 %) et la moitié de la cavité remplie avant le pas 150.
    for s in scenarios_de_moule() {
        let minimum = match s.nom {
            "brique" | "disque" => 0.6,
            "l" => 0.75,
            "anneau" => 0.8,
            _ => 0.9,
        };
        for graine in [1u64, 2, 3, 4] {
            // La courbe jusqu'au pas 150 (la moitié doit être atteinte avant), puis la mesure à 50 pas de la fin et à la fin.
            let (mut simulation, _, duree) = construire_moule(s.nom, graine);
            let c = courbe_de_remplissage(&mut simulation, 150);
            let moitie = pas_pour_remplir(&c, 0.5).unwrap_or_else(|| panic!("{}, graine {graine} : la moitié", s.nom));
            assert!(moitie <= 150, "{}, graine {graine} : la moitié de la cavité au pas {moitie}", s.nom);
            simulation.avancer_n(duree - 50 - 150);
            let avant_la_fin = mesurer(&simulation);
            simulation.avancer_n(50);
            let fin = mesurer(&simulation);
            assert_eq!(simulation.temps(), duree);
            let remplissage = fin.remplissage;
            assert!(remplissage >= minimum, "{}, graine {graine} : remplissage final {remplissage} < {minimum}", s.nom);
            // La colonie s'est éteinte avant la fin : plus aucune pointe, et plus rien d'occupé sur les 50 derniers pas.
            assert_eq!(simulation.pointes(), 0, "{}, graine {graine} : plus aucune pointe à la fin", s.nom);
            assert_eq!(avant_la_fin.cases_occupees, fin.cases_occupees, "{}, graine {graine} : courbe plate", s.nom);
            // Et la colonie ne sort pas du moule : aucune case occupée en dehors de la cavité (ce qui vaut `cases_libres`).
            assert!(c.iter().all(|p| p.cases_occupees <= s.cases_libres));
            assert!(fin.cases_occupees <= s.cases_libres && fin.cases_libres == s.cases_libres);
        }
    }
}
