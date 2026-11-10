//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « le temps de sortie » (T3 : la colonie dans le labyrinthe).
//!
//! Ils fixent `temps_de_sortie` (la cible est un **rectangle de cases** `(x0, y0, x1, y1)`, bornes incluses) et `chambre`, sur
//! les cinq scénarios de `scenarios/moules/` :
//!
//! - **un recalcul indépendant** (`outils::sortie_de_reference` : avancer à la main, regarder tous les segments à chaque pas, la
//!   règle exacte de la spécification) : même résultat, **même état** (même temps, même empreinte) sur des suites d'appels sur
//!   la même simulation, avec des cibles au hasard : une case, un rectangle, la spore, un mur, le dehors ;
//! - **la règle exacte** : la spore dans la cible vaut `Some(0)` **toujours**, même quand un segment déjà posé finit dans sa
//!   case ; un segment déjà posé compte, et le résultat est le plus petit `temps` parmi ceux qui finissent dans la cible ;
//!   `pas_max` borne aussi la lecture des segments déjà posés ; donc **le résultat ne dépend pas du nombre de pas déjà
//!   joués** ;
//! - **le rectangle** : il est atteint dès que l'une de ses cases l'est (le minimum des temps de ses cases, chacune sur une
//!   simulation neuve), chaque bord compte (une case de plus ou de moins change le résultat) ; un rectangle à l'envers fait
//!   paniquer ; une cible qui sort de la boîte, même d'une seule case, rend `None` sans avancer ;
//! - **`None` quand `pas_max` est trop petit**, la simulation s'arrêtant alors exactement à `pas_max` ; un mur n'est jamais
//!   atteint ;
//! - **minoré** : `temps × pas ≥` la distance de la spore (le centre de sa case) au plus proche point du rectangle cible (un
//!   apex avance d'au plus `pas` par pas de temps ; on compare les carrés) ;
//! - **le labyrinthe** (le scénario, graine 5 ; la sortie est la chambre (5, 5) entière, `chambre(3, 5, 5)`) : les temps de
//!   sortie des graines 1 à 10, tous entre 57 (52 cases à 0,909 par pas) et la durée ; `None` avec un `pas_max` trop court ;
//!   sans thigmotropisme, la colonie ne sort pas dans la durée ; la chambre entière est atteinte au plus tard quand sa case
//!   centre l'est, et avant elle pour certaines graines.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `floor`, `min` et `max`.

#[path = "etape_10/outils.rs"]
mod outils;

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::moule::{centre_de_la_chambre, chambre, temps_de_sortie};
use outils::{
    Cible, Hasard, carre_de_la_distance_minimale, case, cases_bloquees, chambre_de_reference, construire,
    construire_moule, dans_la_cible, delai, premier_segment_dans, scenarios_de_moule, sortie_de_reference,
    texte_du_scenario,
};

/// La sortie du labyrinthe du scénario : la chambre (5, 5) entière, les cases 21 à 23 en x et en y.
const SORTIE: Cible = (21, 21, 23, 23);

fn panique<F: FnOnce()>(f: F) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

/// Une case libre au hasard de la simulation.
fn case_libre(hasard: &mut Hasard, libres: &[(usize, usize)]) -> (usize, usize) {
    libres[hasard.jusqu_a(libres.len())]
}

/// Un rectangle au hasard d'au plus `cote` cases de côté, dans la boîte, autour de la case `(x, y)`.
fn rectangle_autour(
    hasard: &mut Hasard,
    (x, y): (usize, usize),
    cote: usize,
    (largeur, hauteur): (usize, usize),
) -> Cible {
    let x0 = x.saturating_sub(hasard.jusqu_a(cote));
    let y0 = y.saturating_sub(hasard.jusqu_a(cote));
    let x1 = (x + hasard.jusqu_a(cote)).min(largeur - 1);
    let y1 = (y + hasard.jusqu_a(cote)).min(hauteur - 1);
    (x0, y0, x1, y1)
}

// ---------------------------------------------------------------------------------------------------------------------
// Le recalcul indépendant
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_temps_de_sortie_est_le_recalcul_independant_sur_des_suites_d_appels() {
    delai();
    let mut hasard = Hasard::new(808);
    let (mut avec_resultat, mut sans_resultat, mut sans_avancer, mut rectangles) = (0, 0, 0, 0);
    for s in scenarios_de_moule() {
        for graine in [1u64, 2, 3] {
            let (mut a, reglages, _) = construire_moule(s.nom, graine);
            let (mut b, _, _) = construire_moule(s.nom, graine);
            let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
            let bloquees = cases_bloquees(&a);
            let libres: Vec<(usize, usize)> =
                (0..largeur * hauteur).filter(|&i| !bloquees[i]).map(|i| (i % largeur, i / largeur)).collect();
            let murs: Vec<(usize, usize)> =
                (0..largeur * hauteur).filter(|&i| bloquees[i]).map(|i| (i % largeur, i / largeur)).collect();
            for appel in 0..12 {
                // Une cible : la fin d'un segment déjà posé, une case de la cavité, la spore, un mur, le dehors, ou un rectangle.
                let fin_de_segment = if a.segments().is_empty() {
                    reglages.spore
                } else {
                    let segment = a.segments()[hasard.jusqu_a(a.segments().len())];
                    (segment.x1.floor() as usize, segment.y1.floor() as usize)
                };
                let cible = match hasard.jusqu_a(12) {
                    0 => case(reglages.spore.0, reglages.spore.1),
                    1 => {
                        let m = murs[hasard.jusqu_a(murs.len())];
                        case(m.0, m.1)
                    }
                    2 => (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur), largeur + hasard.jusqu_a(3), hauteur - 1),
                    3 => (0, hasard.jusqu_a(hauteur), hasard.jusqu_a(largeur), hauteur + hasard.jusqu_a(3)),
                    4..=5 => case(fin_de_segment.0, fin_de_segment.1),
                    6..=7 => {
                        rectangles += 1;
                        rectangle_autour(&mut hasard, fin_de_segment, 3, (largeur, hauteur))
                    }
                    8 => {
                        rectangles += 1;
                        {
                            let c = case_libre(&mut hasard, &libres);
                            rectangle_autour(&mut hasard, c, 6, (largeur, hauteur))
                        }
                    }
                    _ => {
                        let c = case_libre(&mut hasard, &libres);
                        case(c.0, c.1)
                    }
                };
                // `pas_max` : de 20 pas avant à 60 pas après le temps courant.
                let pas_max = (a.temps() + hasard.jusqu_a(81) as u32).saturating_sub(20);
                let temps_avant = a.temps();
                let attendu = sortie_de_reference(&mut b, cible, pas_max);
                let obtenu = temps_de_sortie(&mut a, cible, pas_max);
                let contexte = format!("{}, graine {graine}, appel {appel}, cible {cible:?}, pas_max {pas_max}", s.nom);
                assert_eq!(obtenu, attendu, "{contexte}");
                assert_eq!(a.temps(), b.temps(), "{contexte} : la simulation s'arrête au même pas");
                assert_eq!(a.empreinte(), b.empreinte(), "{contexte} : le même état");
                match obtenu {
                    Some(t) => {
                        avec_resultat += 1;
                        assert!(t <= pas_max, "{contexte} : un temps au plus pas_max");
                    }
                    None => sans_resultat += 1,
                }
                if a.temps() == temps_avant {
                    sans_avancer += 1;
                }
                // Si la colonie a avancé, c'est pour trouver la cible ou pour atteindre `pas_max`.
                if a.temps() > temps_avant {
                    match obtenu {
                        Some(m) => assert_eq!(a.temps(), m, "{contexte} : arrêtée au pas où la cible est atteinte"),
                        None => assert_eq!(a.temps(), pas_max, "{contexte} : arrêtée à pas_max"),
                    }
                }
            }
        }
    }
    assert!(
        avec_resultat >= 70 && sans_resultat >= 25,
        "garde : des cibles atteintes et des cibles manquées : {avec_resultat} / {sans_resultat}"
    );
    assert!(sans_avancer >= 20, "garde : des appels qui n'avancent pas : {sans_avancer}");
    assert!(rectangles >= 30, "garde : des rectangles de plusieurs cases : {rectangles}");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_temps_de_sortie_de_chaque_case_d_un_labyrinthe_est_le_recalcul_independant() {
    delai();
    // Une case libre sur dix du labyrinthe, et la sortie, chacune sur une simulation neuve (le labyrinthe est petit), jusqu'au
    // pas 90.
    let (modele, reglages, _) = construire_moule("labyrinthe", 1);
    let bloquees = cases_bloquees(&modele);
    let largeur = reglages.largeur;
    let mut libres = 0;
    let mut essayees = 0;
    let mut cibles: Vec<Cible> = vec![SORTIE];
    for (i, &bloquee) in bloquees.iter().enumerate() {
        if bloquee {
            continue;
        }
        libres += 1;
        if libres % 10 == 1 {
            cibles.push(case(i % largeur, i / largeur));
        }
    }
    for cible in cibles {
        essayees += 1;
        let (mut a, _, _) = construire_moule("labyrinthe", 1);
        let (mut b, _, _) = construire_moule("labyrinthe", 1);
        assert_eq!(temps_de_sortie(&mut a, cible, 90), sortie_de_reference(&mut b, cible, 90), "cible {cible:?}");
        assert_eq!(a.temps(), b.temps(), "cible {cible:?}");
        assert_eq!(a.empreinte(), b.empreinte(), "cible {cible:?}");
    }
    assert_eq!(libres, 429);
    assert!(essayees >= 40, "garde : {essayees} cibles essayées");
}

// ---------------------------------------------------------------------------------------------------------------------
// La règle exacte : la spore, les segments déjà posés, pas_max
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_spore_dans_la_cible_vaut_some_zero_sans_avancer() {
    delai();
    for s in scenarios_de_moule() {
        let (sx, sy) = s.spore;
        let (largeur, hauteur) = s.boite;
        // La case seule, un rectangle qui la contient (dont la spore sur un bord), toute la boîte.
        for cible in [
            case(sx, sy),
            (sx - 1, sy - 1, sx + 1, sy + 1),
            (sx, sy, sx + 3, sy + 2),
            (sx.saturating_sub(2), sy.saturating_sub(5), sx, sy),
            (0, 0, largeur - 1, hauteur - 1),
        ] {
            for pas_max in [0u32, 1, 17, 1000, u32::MAX] {
                let (mut simulation, _, _) = construire_moule(s.nom, 4);
                let empreinte = simulation.empreinte();
                assert_eq!(
                    temps_de_sortie(&mut simulation, cible, pas_max),
                    Some(0),
                    "{}, {cible:?}, {pas_max}",
                    s.nom
                );
                assert_eq!(simulation.temps(), 0, "{} : la simulation n'a pas avancé", s.nom);
                assert_eq!(simulation.empreinte(), empreinte, "{} : l'état n'a pas changé", s.nom);
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_spore_dans_la_cible_vaut_some_zero_meme_quand_un_segment_finit_dans_sa_case() {
    delai();
    // La brique, graine 1 : des segments finissent dans la case de la spore (le premier au pas 68). La règle (1) passe avant
    // la lecture des segments : `Some(0)`, quel que soit `pas_max`, sans avancer.
    let (mut simulation, _, _) = construire_moule("brique", 1);
    simulation.avancer_n(100);
    let spore = case(50, 50);
    let garde = premier_segment_dans(&simulation, spore, 100);
    assert!(garde.is_some_and(|t| t > 0 && t <= 100), "garde : un segment finit dans la case de la spore : {garde:?}");
    for pas_max in [0u32, 10, 100, 500] {
        assert_eq!(temps_de_sortie(&mut simulation, spore, pas_max), Some(0), "pas_max {pas_max}");
        assert_eq!(temps_de_sortie(&mut simulation, (49, 49, 51, 51), pas_max), Some(0), "pas_max {pas_max}");
    }
    assert_eq!(simulation.temps(), 100, "la simulation n'a pas avancé");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_segment_deja_pose_compte_et_c_est_le_plus_petit_temps() {
    delai();
    for s in scenarios_de_moule() {
        let (mut simulation, _, _) = construire_moule(s.nom, 2);
        simulation.avancer_n(80);
        // Les cases où finit au moins un segment, avec leurs temps (la case de la spore exclue : elle vaut toujours 0).
        let mut fins: BTreeMap<(usize, usize), Vec<u32>> = BTreeMap::new();
        for segment in simulation.segments() {
            let c = (segment.x1.floor() as usize, segment.y1.floor() as usize);
            if c != s.spore {
                fins.entry(c).or_default().push(segment.temps);
            }
        }
        // Une case où plusieurs segments finissent à des temps différents, la plus écartée.
        let (c, temps) = fins
            .iter()
            .filter(|(_, t)| t.len() >= 2)
            .max_by_key(|(_, t)| t.iter().max().unwrap() - t.iter().min().unwrap())
            .map(|(c, t)| (*c, t.clone()))
            .expect("garde : au moins une case reçoit plusieurs fins de segment");
        let (plus_petit, plus_grand) = (*temps.iter().min().unwrap(), *temps.iter().max().unwrap());
        assert!(plus_petit < plus_grand, "{} : garde, des temps différents dans la case {c:?} : {temps:?}", s.nom);
        let empreinte = simulation.empreinte();
        assert_eq!(
            temps_de_sortie(&mut simulation, case(c.0, c.1), 80),
            Some(plus_petit),
            "{} : {c:?} {temps:?}",
            s.nom
        );
        assert_eq!(simulation.temps(), 80, "{} : la simulation n'a pas avancé", s.nom);
        assert_eq!(simulation.empreinte(), empreinte, "{} : l'état n'a pas changé", s.nom);
        // Pour toute case où finit un segment, le résultat est son plus petit temps, sans avancer, et se répète.
        for (c, temps) in fins.iter().step_by(37) {
            let m = *temps.iter().min().unwrap();
            assert_eq!(temps_de_sortie(&mut simulation, case(c.0, c.1), 80), Some(m), "{} : case {c:?}", s.nom);
            assert_eq!(
                temps_de_sortie(&mut simulation, case(c.0, c.1), 80),
                Some(m),
                "{} : au second appel aussi",
                s.nom
            );
            assert_eq!(premier_segment_dans(&simulation, case(c.0, c.1), 80), Some(m));
        }
        assert_eq!(simulation.temps(), 80);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn pas_max_borne_aussi_les_segments_deja_poses() {
    delai();
    for s in scenarios_de_moule() {
        let (mut simulation, _, _) = construire_moule(s.nom, 3);
        simulation.avancer_n(100);
        // Une case atteinte pour la première fois entre les pas 40 et 90, et pas par la spore.
        let mut premiers: BTreeMap<(usize, usize), u32> = BTreeMap::new();
        for segment in simulation.segments() {
            let c = (segment.x1.floor() as usize, segment.y1.floor() as usize);
            let t = premiers.entry(c).or_insert(segment.temps);
            *t = (*t).min(segment.temps);
        }
        let (c, t) = premiers
            .iter()
            .find(|(c, t)| **c != s.spore && (40..=90).contains(*t))
            .map(|(c, t)| (*c, *t))
            .expect("garde : une case atteinte entre les pas 40 et 90");
        let cible = case(c.0, c.1);
        let empreinte = simulation.empreinte();
        // Un `pas_max` sous le premier temps : `None` ; à partir de lui : `Some(t)` ; sans jamais avancer (la simulation est au
        // pas 100, au-delà de tous ces `pas_max`).
        assert_eq!(temps_de_sortie(&mut simulation, cible, 0), None, "{} : pas_max 0", s.nom);
        assert_eq!(temps_de_sortie(&mut simulation, cible, t - 1), None, "{} : pas_max {} < {t}", s.nom, t - 1);
        assert_eq!(temps_de_sortie(&mut simulation, cible, t), Some(t), "{} : pas_max = {t}", s.nom);
        assert_eq!(temps_de_sortie(&mut simulation, cible, t + 1), Some(t), "{} : pas_max {} > {t}", s.nom, t + 1);
        assert_eq!(temps_de_sortie(&mut simulation, cible, 100), Some(t), "{}", s.nom);
        assert_eq!(temps_de_sortie(&mut simulation, cible, 5000), Some(t), "{} : pas_max au-delà du temps joué", s.nom);
        assert_eq!(simulation.temps(), 100, "{} : la simulation n'a pas avancé", s.nom);
        assert_eq!(simulation.empreinte(), empreinte, "{} : l'état n'a pas changé", s.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_resultat_ne_depend_pas_du_nombre_de_pas_deja_joues() {
    delai();
    let mut hasard = Hasard::new(1515);
    let mut differents = 0;
    for s in scenarios_de_moule() {
        let (mut modele, reglages, _) = construire_moule(s.nom, 2);
        modele.avancer_n(120);
        let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
        // Des cibles : des fins de segment (atteintes) et des cases au hasard (parfois jamais atteintes), des rectangles.
        let mut cibles = Vec::new();
        for _ in 0..2 {
            let segment = modele.segments()[hasard.jusqu_a(modele.segments().len())];
            let fin = (segment.x1.floor() as usize, segment.y1.floor() as usize);
            cibles.push(case(fin.0, fin.1));
            cibles.push(rectangle_autour(&mut hasard, fin, 3, (largeur, hauteur)));
            cibles.push(case(hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur)));
        }
        for cible in cibles {
            for pas_max in [40u32, 120] {
                // Depuis une simulation neuve, et depuis des simulations déjà avancées à 10, 60 et 120 pas.
                let (mut neuve, _, _) = construire_moule(s.nom, 2);
                let attendu = temps_de_sortie(&mut neuve, cible, pas_max);
                for deja in [10u32, 60, 120] {
                    let (mut avancee, _, _) = construire_moule(s.nom, 2);
                    avancee.avancer_n(deja);
                    let obtenu = temps_de_sortie(&mut avancee, cible, pas_max);
                    assert_eq!(obtenu, attendu, "{}, cible {cible:?}, pas_max {pas_max}, déjà {deja} pas joués", s.nom);
                    // Elle n'avance que s'il le faut : jusqu'au temps trouvé, ou jusqu'à `pas_max`.
                    let arret = match attendu {
                        Some(t) => deja.max(t),
                        None => deja.max(pas_max),
                    };
                    assert_eq!(avancee.temps(), arret, "{}, cible {cible:?}, pas_max {pas_max}, déjà {deja}", s.nom);
                }
                if attendu.is_some_and(|t| t > 0) {
                    differents += 1;
                }
            }
        }
    }
    assert!(differents >= 12, "garde : des cibles atteintes à un temps strictement positif : {differents}");
}

// ---------------------------------------------------------------------------------------------------------------------
// Le rectangle
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_rectangle_est_atteint_des_que_l_une_de_ses_cases_l_est() {
    delai();
    let mut hasard = Hasard::new(99);
    let mut plus_tot = 0;
    for s in scenarios_de_moule() {
        let (mut modele, reglages, _) = construire_moule(s.nom, 1);
        modele.avancer_n(100);
        let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
        for _ in 0..5 {
            let segment = modele.segments()[hasard.jusqu_a(modele.segments().len())];
            let fin = (segment.x1.floor() as usize, segment.y1.floor() as usize);
            let cible = rectangle_autour(&mut hasard, fin, 2, (largeur, hauteur));
            let (x0, y0, x1, y1) = cible;
            // Le minimum des temps de chaque case du rectangle, chacune sur une simulation neuve (la spore comprise : 0).
            let mut minimum: Option<u32> = None;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (mut une, _, _) = construire_moule(s.nom, 1);
                    let t = sortie_de_reference(&mut une, case(x, y), 100);
                    minimum = match (minimum, t) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                }
            }
            let (mut simulation, _, _) = construire_moule(s.nom, 1);
            let obtenu = temps_de_sortie(&mut simulation, cible, 100);
            assert_eq!(obtenu, minimum, "{} : rectangle {cible:?}", s.nom);
            assert!(obtenu.is_some(), "{} : garde, la fin d'un segment est dedans", s.nom);
            let (mut seule, _, _) = construire_moule(s.nom, 1);
            if temps_de_sortie(&mut seule, case(fin.0, fin.1), 100) > obtenu {
                plus_tot += 1;
            }
        }
    }
    assert!(plus_tot >= 5, "garde : le rectangle est souvent atteint avant la case qui l'a fait choisir : {plus_tot}");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn chaque_bord_du_rectangle_compte() {
    delai();
    // La brique, graine 1, au pas 60 : une case `c` atteinte, dont aucune voisine par les côtés n'a été atteinte plus tôt.
    let (mut modele, _, _) = construire_moule("brique", 1);
    modele.avancer_n(60);
    let mut premiers: BTreeMap<(usize, usize), u32> = BTreeMap::new();
    for segment in modele.segments() {
        let c = (segment.x1.floor() as usize, segment.y1.floor() as usize);
        let t = premiers.entry(c).or_insert(segment.temps);
        *t = (*t).min(segment.temps);
    }
    let premier = |x: usize, y: usize| premiers.get(&(x, y)).copied();
    let (c, t) = premiers
        .iter()
        .map(|(c, t)| (*c, *t))
        .find(|&((x, y), t)| {
            (x, y) != (50, 50)
                && t >= 20
                && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                    .iter()
                    .all(|&(vx, vy)| premier(vx, vy).is_some_and(|tv| tv > t + 1))
        })
        .expect("garde : une case atteinte avant toutes ses voisines");
    let (x, y) = c;
    let seule = case(x, y);
    assert_eq!(temps_de_sortie(&mut construire_moule("brique", 1).0, seule, 60), Some(t));
    // Chaque bord élargi d'une case inclut une voisine atteinte plus tard : le temps ne change pas ; chaque bord rétréci d'une
    // case exclut `c` : le temps change (il devient celui de la voisine, plus grand, ou `None`).
    for (elargi, retreci) in [
        ((x - 1, y, x, y), (x + 1, y, x + 1, y)),
        ((x, y, x + 1, y), (x - 1, y, x - 1, y)),
        ((x, y - 1, x, y), (x, y + 1, x, y + 1)),
        ((x, y, x, y + 1), (x, y - 1, x, y - 1)),
    ] {
        assert_eq!(temps_de_sortie(&mut construire_moule("brique", 1).0, elargi, 60), Some(t), "élargi {elargi:?}");
        let autre = temps_de_sortie(&mut construire_moule("brique", 1).0, retreci, 60);
        assert_ne!(autre, Some(t), "rétréci {retreci:?} : la case {c:?} n'y est plus");
        assert_eq!(autre, sortie_de_reference(&mut construire_moule("brique", 1).0, retreci, 60));
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_rectangle_a_l_envers_fait_paniquer() {
    delai();
    let (mut simulation, _, _) = construire_moule("brique", 1);
    for cible in [(5, 5, 4, 5), (5, 5, 5, 4), (60, 60, 40, 40), (0, 1, 0, 0), (1, 0, 0, 0), (99, 99, 0, 0)] {
        assert!(panique(|| _ = temps_de_sortie(&mut simulation, cible, 10)), "la cible {cible:?} fait paniquer");
    }
    assert_eq!(simulation.temps(), 0, "rien n'a avancé");
    // Un rectangle à l'envers et hors de la boîte : il panique quand même (le contrat passe avant la boîte).
    assert!(panique(|| _ = temps_de_sortie(&mut simulation, (100, 0, 99, 0), 10)));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_cible_hors_de_la_boite_est_none_sans_avancer() {
    delai();
    for s in scenarios_de_moule() {
        let (mut simulation, reglages, _) = construire_moule(s.nom, 1);
        simulation.avancer_n(40);
        let empreinte = simulation.empreinte();
        let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
        let (sx, sy) = reglages.spore;
        for cible in [
            case(largeur, 0),
            case(largeur, sy),
            case(0, hauteur),
            case(sx, hauteur),
            case(largeur, hauteur),
            case(largeur + 5, 1),
            case(usize::MAX, 0),
            case(0, usize::MAX),
            case(usize::MAX, usize::MAX),
            // Un rectangle qui contient la spore mais dépasse d'une case : `None` quand même.
            (sx, sy, largeur, sy),
            (sx, sy, sx, hauteur),
            (0, 0, largeur, hauteur - 1),
            (0, 0, largeur - 1, hauteur),
            (0, 0, usize::MAX, usize::MAX),
        ] {
            assert_eq!(temps_de_sortie(&mut simulation, cible, 500), None, "{} : cible {cible:?}", s.nom);
            assert_eq!(simulation.temps(), 40, "{} : cible {cible:?}, la simulation n'avance pas", s.nom);
            assert_eq!(simulation.empreinte(), empreinte, "{} : cible {cible:?}, l'état est intact", s.nom);
        }
        // La dernière case de la boîte, elle, est une cible permise (un mur ici : `None` après avoir avancé à pas_max).
        assert_eq!(temps_de_sortie(&mut simulation, case(largeur - 1, hauteur - 1), 41), None, "{}", s.nom);
        assert_eq!(simulation.temps(), 41, "{}", s.nom);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// None, les murs, le minorant
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn none_quand_pas_max_est_trop_petit_et_la_simulation_s_arrete_a_pas_max() {
    delai();
    for s in scenarios_de_moule() {
        // Une cible loin de la spore : la colonie n'y arrive pas en quelques pas.
        let cible = match s.nom {
            "brique" => case(64, 50),
            "l" => case(45, 70),
            "disque" => case(70, 50),
            "anneau" => case(50, 84),
            _ => SORTIE,
        };
        // pas_max = 0 : rien ne se passe.
        let (mut simulation, _, _) = construire_moule(s.nom, 1);
        assert_eq!(temps_de_sortie(&mut simulation, cible, 0), None, "{}", s.nom);
        assert_eq!(simulation.temps(), 0, "{} : pas_max = 0, la simulation n'avance pas", s.nom);
        // pas_max = 12 : douze pas, pas un de plus, pas un de moins.
        let (mut simulation, _, _) = construire_moule(s.nom, 1);
        let (mut jumelle, _, _) = construire_moule(s.nom, 1);
        assert_eq!(temps_de_sortie(&mut simulation, cible, 12), None, "{}", s.nom);
        assert_eq!(simulation.temps(), 12, "{} : la simulation s'arrête à pas_max", s.nom);
        jumelle.avancer_n(12);
        assert_eq!(simulation.empreinte(), jumelle.empreinte(), "{} : douze pas, ni plus ni moins", s.nom);
        // Un pas_max au temps courant, ou en deçà : rien n'avance, rien n'est trouvé.
        assert_eq!(temps_de_sortie(&mut simulation, cible, 12), None, "{}", s.nom);
        assert_eq!(temps_de_sortie(&mut simulation, cible, 5), None, "{}", s.nom);
        assert_eq!(simulation.temps(), 12, "{} : la simulation n'a pas avancé", s.nom);
        assert_eq!(simulation.empreinte(), jumelle.empreinte());
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_mur_n_est_jamais_atteint() {
    delai();
    for s in scenarios_de_moule() {
        let (mut simulation, reglages, _) = construire_moule(s.nom, 1);
        let bloquees = cases_bloquees(&simulation);
        let largeur = reglages.largeur;
        let mur = (0..bloquees.len()).find(|&i| bloquees[i]).map(|i| (i % largeur, i / largeur)).unwrap();
        assert_eq!(
            temps_de_sortie(&mut simulation, case(mur.0, mur.1), 60),
            None,
            "{} : la case bloquée {mur:?}",
            s.nom
        );
        assert_eq!(simulation.temps(), 60, "{} : jusqu'à pas_max", s.nom);
        // Un rectangle entièrement dans les murs (deux cases de la ligne du haut de la boîte, hors de toute cavité).
        let coin = (0, 0, 1.min(largeur - 1), 0);
        assert!((coin.1..=coin.3).all(|y| (coin.0..=coin.2).all(|x| bloquees[y * largeur + x])), "{} : garde", s.nom);
        assert_eq!(temps_de_sortie(&mut simulation, coin, 70), None, "{} : le coin {coin:?}", s.nom);
        assert_eq!(simulation.temps(), 70, "{}", s.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_temps_de_sortie_est_au_moins_la_distance_divisee_par_le_pas() {
    delai();
    let mut hasard = Hasard::new(2025);
    let mut verifies = 0;
    let mut serres = 0.0f64;
    for s in scenarios_de_moule() {
        for graine in [1u64, 2, 3, 4] {
            let (mut simulation, reglages, duree) = construire_moule(s.nom, graine);
            simulation.avancer_n(duree.min(150));
            let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
            let pas = reglages.croissance.pas;
            // Des cases et des rectangles au hasard : leur temps de sortie est borné par le bas.
            for essai in 0..40 {
                let c = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let cible = if essai % 3 == 0 {
                    rectangle_autour(&mut hasard, c, 4, (largeur, hauteur))
                } else {
                    case(c.0, c.1)
                };
                let maintenant = simulation.temps();
                let Some(t) = temps_de_sortie(&mut simulation, cible, maintenant) else { continue };
                let atteint = t as f64 * pas;
                let distance_carree = carre_de_la_distance_minimale(reglages.spore, cible);
                assert!(
                    atteint * atteint >= distance_carree,
                    "{}, graine {graine}, cible {cible:?} : temps {t} × pas {pas} = {atteint}, distance² {distance_carree}",
                    s.nom
                );
                verifies += 1;
                if distance_carree > 0.0 && t > 0 {
                    serres = serres.max(distance_carree / (atteint * atteint));
                }
            }
        }
    }
    assert!(verifies >= 150, "garde : assez de cibles vérifiées : {verifies}");
    // Garde contre une borne molle (par exemple un temps qui serait toujours énorme) : une cible est atteinte à moins de 20 %
    // de la borne, c'est-à-dire presque en ligne droite.
    assert!(serres >= 0.64, "garde : la borne est approchée (rapport max {serres})");
}

// ---------------------------------------------------------------------------------------------------------------------
// Le labyrinthe
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_chambre_est_le_rectangle_de_la_specification() {
    delai();
    assert_eq!(chambre(3, 5, 5), SORTIE);
    assert_eq!(chambre(3, 0, 0), (1, 1, 3, 3));
    assert_eq!(chambre(1, 0, 0), (1, 1, 1, 1));
    assert_eq!(chambre(1, 2, 3), (5, 7, 5, 7));
    assert_eq!(chambre(2, 1, 0), (4, 1, 5, 2), "colonne et rangée ne s'échangent pas");
    assert_eq!(chambre(2, 0, 1), (1, 4, 2, 5));
    for couloir in 1..=6 {
        for colonne in 0..5 {
            for rangee in 0..5 {
                let c = chambre(couloir, colonne, rangee);
                assert_eq!(
                    c,
                    chambre_de_reference(couloir, colonne, rangee),
                    "couloir {couloir}, ({colonne}, {rangee})"
                );
                let (x, y) = centre_de_la_chambre(couloir, colonne, rangee);
                assert!(dans_la_cible(c, x, y), "le centre est dans sa chambre");
                assert_eq!((c.2 - c.0 + 1, c.3 - c.1 + 1), (couloir, couloir), "couloir × couloir cases");
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn dans_le_labyrinthe_la_colonie_sort_entre_le_plus_court_chemin_et_la_duree() {
    delai();
    // Graines 1 à 10 : les temps de sortie, tous au moins 57 (52 cases à au plus 0,909 par pas) et au plus la durée (300).
    let mut sorties = Vec::new();
    for graine in 1..=10u64 {
        let (mut simulation, _, duree) = construire_moule("labyrinthe", graine);
        assert_eq!(duree, 300);
        let t = temps_de_sortie(&mut simulation, SORTIE, duree).unwrap_or_else(|| panic!("la graine {graine} sort"));
        assert_eq!(simulation.temps(), t, "graine {graine} : la simulation s'arrête à la sortie");
        assert!((57..=duree).contains(&t), "graine {graine} : {t}");
        sorties.push(t);
    }
    assert_eq!(sorties, SORTIES_DES_GRAINES_1_A_10, "les temps de sortie des graines 1 à 10");
    // Graine 1 : avec `pas_max` en dessous de son temps de sortie : `None`, la simulation s'arrête à `pas_max`.
    let t1 = sorties[0];
    for (pas_max, attendu) in
        [(0u32, None), (40, None), (t1 - 1, None), (t1, Some(t1)), (t1 + 1, Some(t1)), (300, Some(t1))]
    {
        let (mut simulation, _, _) = construire_moule("labyrinthe", 1);
        assert_eq!(temps_de_sortie(&mut simulation, SORTIE, pas_max), attendu, "graine 1, pas_max {pas_max}");
        assert_eq!(
            simulation.temps(),
            attendu.unwrap_or(pas_max),
            "graine 1, pas_max {pas_max} : la simulation s'arrête là"
        );
    }
}

/// Les temps de sortie des graines 1 à 10 du scénario du labyrinthe (la chambre (5, 5) entière).
const SORTIES_DES_GRAINES_1_A_10: [u32; 10] = [69, 63, 65, 66, 65, 63, 61, 61, 70, 71];

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_chambre_entiere_est_atteinte_au_plus_tard_quand_sa_case_centre_l_est() {
    delai();
    let centre = centre_de_la_chambre(3, 5, 5);
    let mut avant = 0;
    for graine in 1..=10u64 {
        let (mut par_la_chambre, _, duree) = construire_moule("labyrinthe", graine);
        let (mut par_le_centre, _, _) = construire_moule("labyrinthe", graine);
        let chambre_entiere = temps_de_sortie(&mut par_la_chambre, SORTIE, duree).expect("la colonie sort");
        let case_centre = temps_de_sortie(&mut par_le_centre, case(centre.0, centre.1), duree);
        match case_centre {
            Some(t) => {
                assert!(chambre_entiere <= t, "graine {graine} : chambre {chambre_entiere}, centre {t}");
                if chambre_entiere < t {
                    avant += 1;
                }
            }
            // La case centre seule n'est jamais touchée dans la durée : l'instrument bruité de la spécification d'origine.
            None => avant += 1,
        }
    }
    assert!(
        avant >= 5,
        "garde : la chambre est atteinte avant sa case centre pour au moins la moitié des graines : {avant}"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn sans_thigmotropisme_la_colonie_ne_sort_pas_du_labyrinthe_dans_la_duree() {
    delai();
    // Le scénario du labyrinthe avec `thigmotropisme 0` : une pointe contre un mur reste immobile (T2), et la colonie n'atteint
    // pas la dernière chambre en 300 pas, pour les graines 1 à 3.
    let texte: String = texte_du_scenario("labyrinthe")
        .lines()
        .map(|l| if l.starts_with("thigmotropisme ") { "thigmotropisme 0".to_string() } else { l.to_string() })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(texte.contains("\nthigmotropisme 0"), "garde : la ligne est remplacée");
    let developpe = mycelium_lab::moule::developper(&texte).unwrap();
    for graine in [1u64, 2, 3] {
        let (mut simulation, _, duree) = construire(&developpe, graine);
        assert_eq!(temps_de_sortie(&mut simulation, SORTIE, duree), None, "graine {graine}");
        assert_eq!(simulation.temps(), duree, "graine {graine}");
    }
}
