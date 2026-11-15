//! Étape 17 (M4) : la lecture d'une colonie et le nuage. Un point ou une raison (jamais un nombre à la place, jamais une
//! panique), la densité dans la cavité, et un petit nuage simulé : ses comptes exacts, ses points et sa droite figés comme
//! une non-régression, chaque graine indépendante des autres.

#[path = "../examples/nuage_raideur.rs"]
#[allow(dead_code)]
mod exemple;
#[path = "etape_17/outils.rs"]
mod outils;

use exemple::{ErreurNuage, MOULES, PROFILS, lire, nuage, scenario};
use mycelium_lab::moule::developper;
use mycelium_lab::nuage::{Cavite, Comptes, Ecarte, Nuage, Raison, ajuster};
use mycelium_lab::reseau::Hyphe;
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use outils::{HYPHE, cavite, egal, longueur, paralleles, segment};
use std::sync::OnceLock;

/// Les densités faites à la main et la raideur d'une hyphe droite : l'erreur relative mesurée est de 4e-16 au plus,
/// une marge de 200.
const TOLERANCE: f64 = 1e-13;

/// La simulation d'un moule et d'un profil de l'exemple, à son départ.
fn simulation(moule: usize, profil: usize, graine: u64) -> Simulation {
    let texte = developper(&scenario(&MOULES[moule], &PROFILS[profil])).unwrap();
    let (reglages, milieu, tropismes, _) = lire_scenario_tropismes(&texte).unwrap();
    Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine).unwrap()
}

/// Un zigzag de 1 100 segments (1 101 nœuds, plus que le plafond du solveur) de `x = 0,5` à `x = fin`.
fn zigzag(fin: f64) -> Vec<mycelium_lab::croissance::Segment> {
    let x = |i: usize| 0.5 + (fin - 0.5) * i as f64 / 1100.0;
    (0..1100).map(|i| segment(x(i), [0.5, 9.5][i % 2], x(i + 1), [9.5, 0.5][i % 2])).collect()
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn une_colonie_est_un_point_ou_une_raison_jamais_un_nombre_a_la_place() {
    let c = cavite(30, 10);
    let lire_ici = |s: &[_]| lire(&c, s, &[], (3, 60), HYPHE);
    // Une hyphe d'un mors à l'autre : un point, avec son étiquette, sa densité et sa raideur.
    let p = lire_ici(&paralleles(1, 30)).unwrap();
    assert_eq!((p.graine, p.pas, p.noeuds), (3, 60, 2));
    assert!(egal(p.densite, 29.0 / 300.0, TOLERANCE) && egal(p.raideur, 5.0 / 29.0, TOLERANCE), "{p:?}");
    // Aucun segment, ou une hyphe qui ne touche qu'un mors (même avec plus de 1 000 nœuds) : elle ne traverse pas.
    for segments in [vec![], vec![segment(0.5, 0.5, 28.5, 0.5)], vec![segment(1.5, 0.5, 29.5, 0.5)], zigzag(28.5)] {
        assert_eq!(lire_ici(&segments), Err(Raison::NeTraversePas), "{} segments", segments.len());
    }
    // Plus de 1 000 nœuds qui touchent les deux mors : trop grande, et non « ne traverse pas ».
    assert_eq!(lire_ici(&zigzag(29.5)), Err(Raison::TropGrande));
    // Un autre refus garde son nom : celui de l'extraction pour un segment de longueur nulle, celui de la poussée pour
    // deux hyphes qui touchent chacune un mors sans se rejoindre (la force est nulle : ni un point, ni « ne traverse pas »).
    for segments in
        [vec![segment(2.0, 2.0, 2.0, 2.0)], vec![segment(0.5, 0.5, 9.5, 0.5), segment(20.5, 0.5, 29.5, 0.5)]]
    {
        let lue = lire_ici(&segments);
        assert!(matches!(&lue, Err(Raison::Refus(texte)) if texte.starts_with("DonneeInvalide")), "{lue:?}");
    }
    // Une colonie qui traverse sans densité : la cavité n'a aucune case libre, l'hyphe est tout entière dans un mur, ou
    // elle sort de la boîte. C'est un refus, pas un point de densité nulle ou fausse, et pas une panique.
    let hyphe = paralleles(1, 4);
    let sans_case = Cavite { largeur: 4, hauteur: 2, libres: vec![false; 8] };
    let dans_le_mur = Cavite { largeur: 4, hauteur: 2, libres: [vec![false; 4], vec![true; 4]].concat() };
    let dehors = [segment(0.5, 0.5, 3.5, 2.5)];
    for (cavite, segments) in [(&sans_case, &hyphe[..]), (&dans_le_mur, &hyphe[..]), (&cavite(4, 2), &dehors[..])] {
        let lue = lire(cavite, segments, &[], (1, 1), HYPHE);
        assert!(matches!(&lue, Err(Raison::Refus(texte)) if texte.starts_with("densité")), "{lue:?}");
    }
    assert!(
        lire(&cavite(4, 2), &hyphe, &[], (1, 1), HYPHE).is_ok(),
        "la même hyphe, dans une cavité libre, est un point"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn la_densite_est_la_longueur_d_hyphe_dans_la_cavite_sur_le_nombre_de_cases_libres() {
    let c = cavite(30, 10);
    assert_eq!(c.densite(&[]), Some(0.0));
    // Une hyphe oblique (3-4-5) de longueur 5, et une autre de longueur 2 jusqu'au coin de la boîte : 7 sur 300 cases.
    let hyphes = [segment(1.5, 1.5, 4.5, 5.5), segment(28.0, 10.0, 30.0, 10.0)];
    assert!(egal(c.densite(&hyphes).unwrap(), 7.0 / 300.0, TOLERANCE), "{:?}", c.densite(&hyphes));
    // Une cavité de 6 × 2 dont trois cases sont un mur (la fin de la ligne du bas) : 9 cases libres. Une hyphe de
    // longueur 5 sur la ligne du bas n'a que 2,5 de sa longueur dans la cavité : 2,5/9, et non 5/9 ni 2,5/12.
    let mur = Cavite { largeur: 6, hauteur: 2, libres: [vec![true; 9], vec![false; 3]].concat() };
    assert_eq!(mur.cases_libres(), 9);
    let basse = [segment(0.5, 1.5, 5.5, 1.5)];
    assert!(egal(mur.densite(&basse).unwrap(), 2.5 / 9.0, TOLERANCE), "{:?}", mur.densite(&basse));
    // Pas de densité : aucune case libre, une liste de cases de la mauvaise longueur, un segment hors de la boîte.
    assert_eq!(Cavite { largeur: 6, hauteur: 2, libres: vec![false; 12] }.densite(&basse), None);
    assert_eq!(Cavite { largeur: 6, hauteur: 2, libres: vec![true; 11] }.densite(&basse), None);
    assert_eq!(Cavite { largeur: 6, hauteur: 2, libres: vec![true; 13] }.densite(&basse), None);
    assert_eq!(Cavite { largeur: usize::MAX, hauteur: 2, libres: vec![true; 12] }.densite(&basse), None);
    for dehors in [30.5, -0.5, f64::NAN, f64::INFINITY] {
        for s in [segment(dehors, 1.0, 2.0, 1.0), segment(2.0, 1.0, dehors, 1.0)] {
            assert_eq!(c.densite(&[s]), None, "x = {dehors}");
        }
    }
    for dehors in [10.5, -0.5, f64::NAN, f64::NEG_INFINITY] {
        for s in [segment(1.0, dehors, 1.0, 2.0), segment(1.0, 2.0, 1.0, dehors)] {
            assert_eq!(c.densite(&[basse[0], s]), None, "y = {dehors}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn la_cavite_d_une_simulation_est_celle_de_son_moule() {
    // La brique couvre sa boîte ; l'anneau, ce que dit le calcul en entiers (8² < d² ≤ 14²) ; le L de 30 × 30, sa barre
    // de gauche (10 colonnes) et sa barre du bas (10 lignes) : 500 cases.
    let de = |moule: usize| Cavite::de(&simulation(moule, 0, 1));
    assert_eq!(de(0), cavite(30, 20));
    let dans_l_anneau = |i: usize| {
        let d2 = (i % 29).abs_diff(14).pow(2) + (i / 29).abs_diff(14).pow(2);
        d2 > 64 && d2 <= 196
    };
    assert_eq!(de(1), Cavite { largeur: 29, hauteur: 29, libres: (0..29 * 29).map(dans_l_anneau).collect() });
    assert_eq!(
        de(2),
        Cavite { largeur: 30, hauteur: 30, libres: (0..900).map(|i| i % 30 < 10 || i / 30 >= 20).collect() }
    );
    assert_eq!((de(0).cases_libres(), de(1).cases_libres(), de(2).cases_libres()), (600, 416, 500));
}

/// Le petit nuage : l'anneau, profil rapide, les graines 3 à 11, aux pas 70 et 110 (2,3 s de calcul, une fois pour le fichier).
const GRAINES: std::ops::RangeInclusive<u64> = 3..=11;
const PAS: [u32; 2] = [70, 110];

fn petit() -> &'static Nuage {
    static PETIT: OnceLock<Nuage> = OnceLock::new();
    let graines: Vec<u64> = GRAINES.collect();
    PETIT.get_or_init(|| nuage(&scenario(&MOULES[1], &PROFILS[1]), &graines, &PAS, exemple::HYPHE).unwrap())
}

/// Les points du petit nuage, **mesurés** : (graine, pas, nœuds, `ρ`, `K`). Une non-régression, jamais une cible.
#[rustfmt::skip]
const POINTS: [(u64, u32, usize, f64, f64); 11] = [
    (3, 70, 209, 0.43482901658868484, 0.0018114324586093744),
    (3, 110, 355, 0.7254321872480767, 0.011967518975538886),
    (6, 70, 204, 0.42390963059085635, 0.0018289743532697855),
    (6, 110, 327, 0.6839295274139648, 0.001829034176779487),
    (7, 70, 430, 0.9089170645992891, 0.0030548682694266915),
    (7, 110, 473, 0.9875514849733398, 0.00464493027889786),
    (9, 70, 164, 0.3518002609043261, 0.00044516193354127154),
    (9, 110, 340, 0.7166919103370426, 0.0011775160634783788),
    (10, 110, 623, 1.3108604655500502, 0.0022033295280308052),
    (11, 70, 127, 0.2600270664834593, 0.0007628570220050614),
    (11, 110, 130, 0.26439709784345544, 0.0008876192438868266),
];
/// Sa droite, mesurée elle aussi : aucune pente de colonie n'est publiée, et celle-ci (sur onze points de six graines,
/// avec un `R²` de 0,42) n'est réglée vers aucun repère : d'autres graines en donnent une autre.
const PENTE: f64 = 1.0775677540258743;
const R2: f64 = 0.4199882187733769;
/// Une colonie simulée n'a que six à sept chiffres (étape 15). Les mêmes colonies, les nœuds numérotés à l'envers,
/// rendent `K` à 7e-8 près, la pente à 2,2e-8 près et le `R²` à 1,2e-8 près : une marge de 14, de 45 et de 80.
const TOLERANCE_RAIDEUR: f64 = 1e-6;
const TOLERANCE_DROITE: f64 = 1e-6;
/// La densité d'une colonie simulée, par la carte ou par ses segments : l'écart mesuré sur 7 650 colonies (les six
/// scénarios, les graines 201 à 350) est de 9e-15 au plus : une marge de 100.
const TOLERANCE_DENSITE: f64 = 1e-12;

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn le_petit_nuage_a_ses_comptes_exacts_et_ses_points_et_sa_droite_figes_comme_une_non_regression() {
    let n = petit();
    assert_eq!(n.comptes(), Comptes { retenus: 11, ne_traversent_pas: 7, trop_grandes: 0, refus: 0 });
    let non = |graine, pas| Ecarte { graine, pas, raison: Raison::NeTraversePas };
    assert_eq!(n.ecartes, [non(4, 70), non(4, 110), non(5, 70), non(5, 110), non(8, 70), non(8, 110), non(10, 70)]);
    assert_eq!(n.points.len(), POINTS.len());
    for (p, (graine, pas, noeuds, densite, raideur)) in n.points.iter().zip(POINTS) {
        assert_eq!((p.graine, p.pas, p.noeuds), (graine, pas, noeuds));
        assert!(egal(p.densite, densite, TOLERANCE_DENSITE), "graine {graine}, pas {pas} : ρ = {:e}", p.densite);
        assert!(egal(p.raideur, raideur, TOLERANCE_RAIDEUR), "graine {graine}, pas {pas} : K = {:e}", p.raideur);
    }
    let droite = ajuster(&n.paires()).unwrap();
    assert_eq!(droite.points, 11);
    assert!((droite.pente - PENTE).abs() < TOLERANCE_DROITE, "pente {}", droite.pente);
    assert!((droite.r2 - R2).abs() < TOLERANCE_DROITE, "R² {}", droite.r2);
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn les_points_du_petit_nuage_sont_ses_colonies_qui_touchent_les_deux_mors_avec_la_densite_de_leurs_segments() {
    // Les dix-huit colonies du petit nuage, rejouées à part, sans l'extraction ni la poussée : chacune touche un mors si
    // un bout de segment est dans sa colonne. Ici, aucune n'est trop grande et aucune poutre fondue ne change la colonne
    // d'un nœud : les points sont donc exactement les colonies qui touchent les deux mors (ce n'est pas vrai de tout
    // nuage). La densité d'un point est la longueur de ses segments sur les 416 cases de l'anneau : une colonie simulée
    // ne sort pas de sa cavité.
    let (n, mut lues) = (petit(), 0);
    for graine in GRAINES {
        let mut s = simulation(1, 1, graine);
        for pas in PAS {
            s.avancer_n(pas - s.temps());
            let dans = |colonne: f64| s.segments().iter().any(|t| t.x0.floor() == colonne || t.x1.floor() == colonne);
            let point = n.points.iter().find(|p| (p.graine, p.pas) == (graine, pas));
            assert_eq!(point.is_some(), dans(0.0) && dans(28.0), "graine {graine}, pas {pas}");
            assert_eq!(point.is_none(), n.ecartes.contains(&Ecarte { graine, pas, raison: Raison::NeTraversePas }));
            if let Some(p) = point {
                let densite = longueur(s.segments()) / 416.0;
                assert!(egal(p.densite, densite, TOLERANCE_DENSITE), "graine {graine}, pas {pas}");
                assert!(p.raideur > 0.0 && p.raideur.is_finite() && p.noeuds <= 1000);
                lues += 1;
            }
        }
    }
    assert_eq!(lues, 11, "chaque point a été rejoué");
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn chaque_graine_seule_rend_ce_qu_elle_rend_dans_le_nuage_au_bit_pres() {
    // Chaque graine calculée seule, de la dernière à la première, puis rangée à sa place : le nuage entier. Des nombres
    // finis et non nuls ne sont égaux que bit à bit : cette égalité dit aussi que le même calcul, refait, rend les mêmes bits.
    let texte = scenario(&MOULES[1], &PROFILS[1]);
    let seules: Vec<Nuage> =
        GRAINES.rev().map(|graine| nuage(&texte, &[graine], &PAS, exemple::HYPHE).unwrap()).collect();
    let mut assemble = Nuage::default();
    for seule in seules.iter().rev() {
        assert_eq!(seule.points.len() + seule.ecartes.len(), 2, "une colonie lue par pas");
        assemble.points.extend(seule.points.iter().copied());
        assemble.ecartes.extend(seule.ecartes.iter().cloned());
    }
    assert_eq!(&assemble, petit());
    assert_eq!(ajuster(&assemble.paires()), ajuster(&petit().paires()));
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn les_trois_raisons_se_comptent_et_un_nuage_impossible_est_une_erreur() {
    // La brique dense : la graine 1 ne traverse pas au pas 40 et déborde le solveur au pas 60 ; la graine 8 traverse aux
    // deux. Les pas sont joués dans l'ordre : le même nuage, pas par pas, rend les mêmes colonies.
    let texte = scenario(&MOULES[0], &PROFILS[0]);
    let n = nuage(&texte, &[1, 8], &[40, 60], exemple::HYPHE).unwrap();
    assert_eq!(n.comptes(), Comptes { retenus: 2, ne_traversent_pas: 1, trop_grandes: 1, refus: 0 });
    let ecarte = |pas, raison| Ecarte { graine: 1, pas, raison };
    assert_eq!(n.ecartes, [ecarte(40, Raison::NeTraversePas), ecarte(60, Raison::TropGrande)]);
    assert_eq!(n.points.iter().map(|p| (p.graine, p.pas, p.noeuds)).collect::<Vec<_>>(), [(8, 40, 174), (8, 60, 188)]);
    assert_eq!(n.paires(), [(n.points[0].densite, n.points[0].raideur), (n.points[1].densite, n.points[1].raideur)]);
    let au_pas = |pas: u32| nuage(&texte, &[1, 8], &[pas], exemple::HYPHE).unwrap();
    assert_eq!([au_pas(40).points, au_pas(60).points].concat(), [n.points[0], n.points[1]]);
    // Les raideurs d'hyphe sont celles qu'on donne : deux fois plus raides, la même colonie est deux fois plus raide.
    let double = Hyphe { ea: 2.0 * exemple::HYPHE.ea, ei: 2.0 * exemple::HYPHE.ei };
    let raide = nuage(&texte, &[8], &[40], double).unwrap().points[0];
    assert_eq!((raide.densite, raide.noeuds), (n.points[0].densite, 174));
    assert!(egal(raide.raideur, 2.0 * n.points[0].raideur, TOLERANCE_RAIDEUR), "K = {:e}", raide.raideur);
    // Un refus se compte à part des deux autres raisons.
    let mut avec_refus = n.clone();
    avec_refus.ecartes.push(ecarte(80, Raison::Refus("Mecanisme".to_string())));
    assert_eq!(avec_refus.comptes(), Comptes { retenus: 2, ne_traversent_pas: 1, trop_grandes: 1, refus: 1 });
    // Aucune graine, ou aucun pas : un nuage vide. Des pas qui ne croissent pas, ou un scénario refusé : une erreur.
    assert_eq!(nuage(&texte, &[], &[40], exemple::HYPHE), Ok(Nuage::default()));
    assert_eq!(nuage(&texte, &[1], &[], exemple::HYPHE), Ok(Nuage::default()));
    for pas in [[40, 40, 60], [60, 40, 80], [40, 60, 50]] {
        assert_eq!(nuage(&texte, &[1], &pas, exemple::HYPHE), Err(ErreurNuage::PasNonCroissants), "{pas:?}");
    }
    for refuse in ["boite 0 0\n".to_string(), texte.replace("brique 0 0 29 19", "brique 0 0 99 99")] {
        assert!(matches!(nuage(&refuse, &[1], &[10], exemple::HYPHE), Err(ErreurNuage::Scenario(_))), "{refuse}");
    }
    // Le pas 0 se lit : la spore seule, aucun segment, elle ne traverse pas.
    assert_eq!(
        nuage(&texte, &[8], &[0], exemple::HYPHE).unwrap().ecartes,
        [Ecarte { graine: 8, pas: 0, raison: Raison::NeTraversePas }]
    );
}
