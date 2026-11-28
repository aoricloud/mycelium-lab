//! Tests d'acceptation de l'étape 27 : la colonie dans un moule en anneau.
//!
//! La scène `scenarios/colonie-tore.txt` : un tore d'axe `z` (grand rayon 10, petit rayon 4) au centre d'un cube de
//! 32 ; la spore est dans l'anneau, du côté `x = 26`, et la colonie doit en faire le tour par les deux côtés. Le
//! scénario porte une ligne `moule`, développée en lignes `obstacle` par `developper_3d` avant sa lecture.
//!
//! Ce qui est vérifié :
//! - les empreintes de la graine 1 au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis à la fin de la
//!   durée celle de `references/colonie-tore.empreinte`, les comptes de la fin (segments, ramifications, pointes,
//!   apex) et l'empreinte de la fin des graines 2 et 3 ;
//! - le moule : 291 lignes `obstacle` dans le texte développé, aucune ligne `moule`, autant d'obstacles lus, et la
//!   spore comme chaque nourriture dans une case libre ;
//! - la cavité respectée, graines 1 à 3 : chaque segment est libre pour `traverser_3d` et ses deux bouts sont dans
//!   le cube ;
//! - le remplissage de la graine 1 : un point par pas de 0 à la durée, 2992 cases libres, des cases atteintes qui ne
//!   décroissent jamais, et au dernier point 2455 atteintes sur 2992 soit la part écrite ; le rejeu des arrivées des
//!   segments redonne ce nombre ; les cases atteintes des graines 2 et 3 ;
//! - la propriété de la forme : les cases atteintes par quart du plan, plus de 500 dans chaque quart aux trois
//!   graines, et la case (6, 16, 16), à l'opposé de la spore, atteinte aux graines 1 et 3 ;
//! - les invariants et le déterminisme, graines 1 à 3 : un apex pour la spore plus un par ramification, le bilan de
//!   la masse, la même graine donnant les mêmes octets, `avancer_n(n)` égalant `n` appels à `avancer`, et les
//!   graines 1 et 2 qui diffèrent.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, lire_scenario_colonie3d};
use mycelium_lab::contact3d::{Traversee3d, traverser_3d};
use mycelium_lab::moule3d::{cases_atteintes, courbe_de_remplissage_3d, developper_3d};

/// Le côté du cube de la scène, en cases.
const COTE: usize = 32;
/// Le nombre de cases libres du tore 10/4 dans le cube de 32³.
const LIBRES: u64 = 2992;
/// La durée du scénario.
const DUREE: u32 = 130;

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// Le scénario d'une scène, lu par `developper_3d` puis `lire_scenario_colonie3d`.
fn scenario(nom: &str) -> ScenarioColonie3d {
    let texte = texte_du_fichier(nom);
    let developpe = developper_3d(&texte).unwrap_or_else(|e| panic!("{nom} : moule refusé {e:?}"));
    lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"))
}

/// Une colonie de la scène `nom`, à la graine `graine`, au temps 0.
fn colonie(nom: &str, graine: u64) -> Colonie3d {
    let scenario = scenario(nom);
    Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("{nom} : construction refusée {e:?}"))
}

/// Une colonie de la scène `nom`, à la graine `graine`, avancée jusqu'à sa durée.
fn colonie_finie(nom: &str, graine: u64) -> Colonie3d {
    let mut colonie = colonie(nom, graine);
    colonie.avancer_n(colonie.duree());
    colonie
}

/// L'empreinte de référence d'un fichier de `references/` : 64 chiffres hexadécimaux minuscules, une ligne.
fn empreinte_de_reference(fichier: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join(fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
    assert_eq!(texte.lines().count(), 1, "{fichier} : une seule ligne");
    let empreinte = texte.trim();
    assert_eq!(empreinte.len(), 64, "{fichier} : 64 chiffres hexadécimaux");
    assert!(
        empreinte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "{fichier} : des chiffres hexadécimaux minuscules"
    );
    empreinte.to_string()
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

/// Le nombre de cases vraies d'un tableau.
fn compter(cases: &[bool]) -> u64 {
    cases.iter().filter(|&&libre| libre).count() as u64
}

/// Les cases atteintes, recalculées dans le test : l'arrivée de chaque segment, au plancher, bornée à la dernière
/// case, et seulement si cette case est libre.
fn rejeu_des_atteintes(colonie: &Colonie3d) -> Vec<bool> {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let mut atteintes = vec![false; largeur * hauteur * profondeur];
    for segment in colonie.segments() {
        let x = (segment.x1.floor() as usize).min(largeur - 1);
        let y = (segment.y1.floor() as usize).min(hauteur - 1);
        let z = (segment.z1.floor() as usize).min(profondeur - 1);
        if !colonie.substrat().est_bloquee(x, y, z) {
            atteintes[(z * hauteur + y) * largeur + x] = true;
        }
    }
    atteintes
}

/// Le nombre de cases atteintes d'un quart du plan de dessus : `x` du côté `x_haut` (`x ≥ 16`), `y` du côté
/// `y_haut` (`y ≥ 16`).
fn atteintes_du_quart(atteintes: &[bool], x_haut: bool, y_haut: bool) -> u64 {
    let mut compte = 0u64;
    for z in 0..COTE {
        for y in 0..COTE {
            for x in 0..COTE {
                if atteintes[(z * COTE + y) * COTE + x] && ((x >= 16) == x_haut) && ((y >= 16) == y_haut) {
                    compte += 1;
                }
            }
        }
    }
    compte
}

/// Vrai si la case `(x, y, z)` est atteinte.
fn est_atteinte(atteintes: &[bool], x: usize, y: usize, z: usize) -> bool {
    atteintes[(z * COTE + y) * COTE + x]
}

#[test]
fn colonie_tore_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut colonie = colonie("colonie-tore.txt", 1);
    assert_eq!(
        colonie.empreinte(),
        "876811151e0bbdd1d45d0227c1d6664c8bf255f71bb111e118b135bb14852ec5",
        "colonie-tore : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-tore : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "6f4246014f278e369852004ca988d56251b65b33896064ca8e6c2616fa5cef0c",
        "colonie-tore : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-tore : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "1da43ff7c44c6b3f7ebba088134e8c18748f70f570a2b9dda54d74c701e874b0",
        "colonie-tore : l'empreinte après 10 pas"
    );
    let duree = colonie.duree();
    assert_eq!(duree, DUREE, "colonie-tore : la durée du scénario");
    colonie.avancer_n(duree - 10);
    assert_eq!(colonie.temps(), duree, "colonie-tore : le temps est la durée");
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-tore.empreinte"),
        "colonie-tore : l'empreinte de son fichier de références"
    );
    assert_eq!(colonie.segments().len(), 22914, "colonie-tore : les segments de la fin");
    assert_eq!(colonie.nb_ramifications(), 3016, "colonie-tore : les ramifications de la fin");
    assert_eq!(colonie.pointes(), 2185, "colonie-tore : les pointes actives de la fin");
    assert_eq!(colonie.apex().len(), 3017, "colonie-tore : les apex de la fin");

    assert_eq!(
        colonie_finie("colonie-tore.txt", 2).empreinte(),
        "7df5395cf7e3a79dffd2200068914e4f7c2e62dab8e7a2a9368adf1e7dd24979",
        "colonie-tore, graine 2 : l'empreinte de la fin"
    );
    assert_eq!(
        colonie_finie("colonie-tore.txt", 3).empreinte(),
        "074e5600c4056760ed958c63693bb4ccac16831469727b182cf6b5a52d1720f8",
        "colonie-tore, graine 3 : l'empreinte de la fin"
    );
}

#[test]
fn le_moule_est_le_tore_et_la_spore_est_dans_une_case_libre() {
    let texte = texte_du_fichier("colonie-tore.txt");
    let developpe = developper_3d(&texte).unwrap_or_else(|e| panic!("colonie-tore : moule refusé {e:?}"));
    let lignes: Vec<&str> = developpe.lines().map(str::trim).collect();
    let obstacles = lignes.iter().filter(|ligne| ligne.starts_with("obstacle")).count();
    assert_eq!(obstacles, 291, "colonie-tore : 291 lignes `obstacle` dans le texte développé");
    assert!(!lignes.iter().any(|ligne| ligne.starts_with("moule")), "colonie-tore : aucune ligne `moule`");

    let scenario = lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("colonie-tore : refus {e:?}"));
    assert_eq!(scenario.obstacles.len(), 291, "colonie-tore : 291 obstacles lus");
    assert!(scenario.depots.is_empty(), "colonie-tore : la scène n'a pas de nourriture");
    let colonie = Colonie3d::nouvelle(&scenario, 1).unwrap_or_else(|e| panic!("construction refusée {e:?}"));

    let substrat = colonie.substrat();
    let (x, y, z) = (scenario.spore.0 as usize, scenario.spore.1 as usize, scenario.spore.2 as usize);
    assert!(!substrat.est_bloquee(x, y, z), "colonie-tore : la spore ({x}, {y}, {z}) est dans une case libre");
    let mut cases_vues = 1usize;
    for depot in &scenario.depots {
        let (x, y, z) = (depot.x as usize, depot.y as usize, depot.z as usize);
        assert!(!substrat.est_bloquee(x, y, z), "colonie-tore : la nourriture ({x}, {y}, {z}) est dans une case libre");
        cases_vues += 1;
    }
    assert_eq!(cases_vues, 1 + scenario.depots.len(), "colonie-tore : la spore et chaque nourriture sont vues");

    // La cavité est bien le tore : 2992 cases libres, et la spore est à 10 cases du centre, sur l'axe x.
    let mut libres = 0u64;
    for z in 0..COTE {
        for y in 0..COTE {
            for x in 0..COTE {
                if !substrat.est_bloquee(x, y, z) {
                    libres += 1;
                }
            }
        }
    }
    assert_eq!(libres, LIBRES, "colonie-tore : 2992 cases libres");
    assert_eq!((scenario.spore.0, scenario.spore.1, scenario.spore.2), (26, 16, 16), "colonie-tore : la spore");
}

#[test]
fn la_cavite_est_respectee_graines_1_a_3() {
    for graine in 1..=3u64 {
        let colonie = colonie_finie("colonie-tore.txt", graine);
        let scenario = colonie.scenario();
        let (largeur, hauteur, profondeur) =
            (scenario.largeur as f64, scenario.hauteur as f64, scenario.profondeur as f64);
        let substrat = colonie.substrat();
        let segments = colonie.segments();
        assert!(!segments.is_empty(), "graine {graine} : des segments à vérifier");

        let mut bouts_vus = 0usize;
        for (numero, segment) in segments.iter().enumerate() {
            assert_eq!(
                traverser_3d(substrat, (segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)),
                Traversee3d::Libre,
                "graine {graine}, segment {numero} : le segment ne touche aucune case bloquée"
            );
            for (nom, x, y, z) in
                [("départ", segment.x0, segment.y0, segment.z0), ("arrivée", segment.x1, segment.y1, segment.z1)]
            {
                assert!(
                    (0.0..largeur).contains(&x) && (0.0..hauteur).contains(&y) && (0.0..profondeur).contains(&z),
                    "graine {graine}, segment {numero} : le {nom} ({x}, {y}, {z}) est dans le cube"
                );
                bouts_vus += 1;
            }
        }
        assert_eq!(bouts_vus, 2 * segments.len(), "graine {graine} : les deux bouts de chaque segment sont vus");
    }
}

#[test]
fn le_remplissage_ne_decroit_jamais_et_finit_a_la_part_ecrite() {
    let mut colonie = colonie("colonie-tore.txt", 1);
    let courbe = courbe_de_remplissage_3d(&mut colonie, DUREE);
    assert_eq!(courbe.len(), DUREE as usize + 1, "un point par pas, de 0 à la durée");
    for (pas, point) in courbe.iter().enumerate() {
        assert_eq!(point.pas, pas as u32, "le pas du point {pas}");
        assert_eq!(point.libres, LIBRES, "colonie-tore : 2992 cases libres à chaque point");
        if pas > 0 {
            assert!(
                point.atteintes >= courbe[pas - 1].atteintes,
                "les cases atteintes ne décroissent jamais, au pas {pas}"
            );
        }
    }
    let dernier = courbe.last().expect("la courbe a un dernier point");
    assert_eq!(dernier.pas, DUREE, "le dernier point est à la durée");
    assert_eq!(dernier.atteintes, 2455, "colonie-tore : 2455 cases atteintes à la fin");
    assert_eq!(dernier.part, 0.8205213903743316, "colonie-tore : la part écrite");
    assert_eq!(dernier.part, 2455.0 / 2992.0, "colonie-tore : 2455 divisé par 2992");

    // Le rejeu des arrivées des segments retrouve les cases atteintes de la fin.
    assert_eq!(colonie.temps(), DUREE, "la courbe a mené la colonie à sa durée");
    let rejeu = rejeu_des_atteintes(&colonie);
    let atteintes = cases_atteintes(&colonie);
    assert_eq!(rejeu, atteintes, "le rejeu des arrivées redonne les cases atteintes");
    assert_eq!(compter(&rejeu), 2455, "le rejeu : 2455 cases atteintes à la fin");

    assert_eq!(
        compter(&cases_atteintes(&colonie_finie("colonie-tore.txt", 2))),
        2557,
        "colonie-tore, graine 2 : 2557 cases atteintes"
    );
    assert_eq!(
        compter(&cases_atteintes(&colonie_finie("colonie-tore.txt", 3))),
        2510,
        "colonie-tore, graine 3 : 2510 cases atteintes"
    );
}

#[test]
fn la_colonie_fait_le_tour_de_l_anneau() {
    for graine in 1..=3u64 {
        let colonie = colonie_finie("colonie-tore.txt", graine);
        let atteintes = cases_atteintes(&colonie);
        let quarts = [
            ("x ≥ 16, y ≥ 16", atteintes_du_quart(&atteintes, true, true)),
            ("x < 16, y ≥ 16", atteintes_du_quart(&atteintes, false, true)),
            ("x ≥ 16, y < 16", atteintes_du_quart(&atteintes, true, false)),
            ("x < 16, y < 16", atteintes_du_quart(&atteintes, false, false)),
        ];
        for (nom, compte) in quarts {
            assert!(compte > 500, "graine {graine}, quart {nom} : {compte} cases atteintes, plus de 500 attendues");
        }
        if graine == 1 {
            assert_eq!(quarts[0].1, 645, "graine 1, quart x ≥ 16, y ≥ 16");
            assert_eq!(quarts[1].1, 608, "graine 1, quart x < 16, y ≥ 16");
            assert_eq!(quarts[2].1, 621, "graine 1, quart x ≥ 16, y < 16");
            assert_eq!(quarts[3].1, 581, "graine 1, quart x < 16, y < 16");
        }
        if graine == 1 || graine == 3 {
            assert!(est_atteinte(&atteintes, 6, 16, 16), "graine {graine} : la case (6, 16, 16) est atteinte");
        }
    }
}

#[test]
fn les_invariants_et_le_determinisme_tiennent_graines_1_a_3() {
    let scenario = scenario("colonie-tore.txt");
    let duree = scenario.duree;
    let mut empreintes = Vec::new();

    for graine in 1..=3u64 {
        let mut d_un_coup =
            Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("graine {graine} : refus {e:?}"));
        let masse_initiale = d_un_coup.substrat().masse_totale();
        let mut encore =
            Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("graine {graine} : refus {e:?}"));
        let mut un_a_un =
            Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("graine {graine} : refus {e:?}"));

        d_un_coup.avancer_n(duree);
        encore.avancer_n(duree);
        for _ in 0..duree {
            un_a_un.avancer();
        }
        assert_eq!(un_a_un.temps(), d_un_coup.temps(), "graine {graine} : le même temps");
        assert_eq!(
            un_a_un.ecrire_canonique(),
            d_un_coup.ecrire_canonique(),
            "graine {graine} : avancer_n(n) et n appels à avancer, les mêmes octets"
        );
        assert_eq!(
            encore.ecrire_canonique(),
            d_un_coup.ecrire_canonique(),
            "graine {graine} : même graine, mêmes octets"
        );

        assert_eq!(
            d_un_coup.apex().len() as u64,
            1 + d_un_coup.nb_ramifications(),
            "graine {graine} : un apex pour la spore, plus un par ramification"
        );

        let mut consomme = 0.0;
        for segment in d_un_coup.segments() {
            consomme += scenario.cout * segment.longueur();
        }
        let masse_finale = d_un_coup.substrat().masse_totale();
        assert!(!d_un_coup.segments().is_empty(), "graine {graine} : des segments consommés");
        assert!(consomme > 0.0, "graine {graine} : de la nourriture consommée ({consomme})");
        assert!(
            egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
            "graine {graine} : {masse_initiale} - {consomme} contre la masse finale {masse_finale}"
        );

        empreintes.push(d_un_coup.empreinte());
    }

    assert_ne!(empreintes[0], empreintes[1], "les graines 1 et 2 donnent deux empreintes");
}
