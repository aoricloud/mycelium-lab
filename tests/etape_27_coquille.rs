//! Tests d'acceptation de l'étape 27 : la colonie dans une brique creuse.
//!
//! Ce qui est vérifié :
//! - les empreintes de `colonie-coquille`, graine 1 : au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis à
//!   la fin de sa durée (l'empreinte de son fichier de `references/`), et les comptes de la fin (segments,
//!   ramifications, pointes, apex) ; les empreintes de la fin des graines 2 et 3 ;
//! - le moule : le texte développé a sept lignes `obstacle` et aucune ligne `moule`, le scénario lu a sept obstacles,
//!   et la spore comme chaque nourriture sont dans une case libre ;
//! - la cavité respectée, graines 1 à 3, à la fin : chaque segment va de son départ à son arrivée sans toucher de case
//!   bloquée (`traverser_3d` rend `Traversee3d::Libre`) et ses deux bouts sont dans le cube ;
//! - le remplissage : la courbe de la graine 1 jusqu'à la durée a un point par pas, les cases libres de la coquille à
//!   chaque point, des cases atteintes qui ne décroissent jamais, et au dernier point les cases atteintes et la part
//!   écrites ; un rejeu des arrivées de segments redonne les mêmes cases atteintes ; les cases atteintes des graines 2
//!   et 3 à la fin ;
//! - la propriété de la forme : les six parois de la coquille ont chacune plus de 1000 cases atteintes, graines 1 à 3,
//!   et les six valeurs de la graine 1 ;
//! - les invariants et le déterminisme, graines 1 à 3 : un apex pour la spore plus un par ramification, la masse de la
//!   nourriture consommée, même graine mêmes octets, `avancer_n(n)` égal à n `avancer`, et les graines 1 et 2
//!   différentes.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::contact3d::{Traversee3d, traverser_3d};
use mycelium_lab::moule3d::{cases_atteintes, courbe_de_remplissage_3d, developper_3d};

/// La scène de la brique creuse.
const SCENE: &str = "colonie-coquille.txt";

/// Le nombre de lignes `obstacle` que le moule développe.
const OBSTACLES: usize = 7;

/// Les cases non bloquées de la coquille d'épaisseur 3 : 28³ − 22³.
const CASES_LIBRES: u64 = 11304;

/// Le nombre de cases atteintes à la fin, graine 1.
const ATTEINTES_GRAINE_1: u64 = 7662;

/// La part de la cavité atteinte à la fin, graine 1 : le quotient écrit.
const PART_GRAINE_1: f64 = 0.6778131634819533;

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
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

/// Le scénario de la scène, développé par le moule puis lu.
fn scenario(nom: &str) -> ScenarioColonie3d {
    let texte = texte_du_fichier(nom);
    let developpe = developper_3d(&texte).unwrap_or_else(|e| panic!("{nom} : refus inattendu du moule {e:?}"));
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

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

/// Le nombre de cases vraies d'un tableau.
fn compter(cases: &[bool]) -> u64 {
    cases.iter().filter(|&&vraie| vraie).count() as u64
}

/// Les cases atteintes, recomptées dans le test par une boucle sur les arrivées des segments : au plancher, bornées à
/// la dernière case, et seulement si la case n'est pas bloquée.
fn rejeu_des_atteintes(colonie: &Colonie3d) -> u64 {
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
    compter(&atteintes)
}

/// Vrai si `segment` a ses deux bouts dans `[0, largeur) × [0, hauteur) × [0, profondeur)`.
fn les_deux_bouts_dans_le_cube(segment: &Segment3d, largeur: f64, hauteur: f64, profondeur: f64) -> bool {
    let dans = |x: f64, y: f64, z: f64| {
        (0.0..largeur).contains(&x) && (0.0..hauteur).contains(&y) && (0.0..profondeur).contains(&z)
    };
    dans(segment.x0, segment.y0, segment.z0) && dans(segment.x1, segment.y1, segment.z1)
}

/// Le nombre de cases atteintes de chacune des six parois de la coquille, dans l'ordre : `x ≤ 4`, `x ≥ 27`,
/// `y ≤ 4`, `y ≥ 27`, `z ≤ 4`, `z ≥ 27`. Une case d'arête compte dans ses deux parois.
fn cases_des_six_parois(colonie: &Colonie3d) -> [u64; 6] {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let atteintes = cases_atteintes(colonie);
    let mut parois = [0u64; 6];
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                if !atteintes[(z * hauteur + y) * largeur + x] {
                    continue;
                }
                if x <= 4 {
                    parois[0] += 1;
                }
                if x >= 27 {
                    parois[1] += 1;
                }
                if y <= 4 {
                    parois[2] += 1;
                }
                if y >= 27 {
                    parois[3] += 1;
                }
                if z <= 4 {
                    parois[4] += 1;
                }
                if z >= 27 {
                    parois[5] += 1;
                }
            }
        }
    }
    parois
}

#[test]
fn colonie_coquille_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut colonie = colonie(SCENE, 1);
    assert_eq!(
        colonie.empreinte(),
        "152b5e51a665776f9f60981499252669d20d5de7d24afa980378aacf3e1e95bb",
        "colonie-coquille : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-coquille : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "68a2cf618d33f9de00c61d004af2334da12cb92df638c5b81649ed11cca0a930",
        "colonie-coquille : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-coquille : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "458d44511373bd5bf28d45ab4f1bd08208398c1a4c6e414392d634805d61b032",
        "colonie-coquille : l'empreinte après 10 pas"
    );
    let duree = colonie.duree();
    assert_eq!(duree, 95, "colonie-coquille : la durée du scénario");
    colonie.avancer_n(duree - 10);
    assert_eq!(colonie.temps(), duree, "colonie-coquille : le temps est la durée");
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-coquille.empreinte"),
        "colonie-coquille : l'empreinte de son fichier de références"
    );
    assert_eq!(colonie.segments().len(), 35835, "colonie-coquille : les segments de la fin");
    assert_eq!(colonie.nb_ramifications(), 4730, "colonie-coquille : les ramifications de la fin");
    assert_eq!(colonie.pointes(), 3785, "colonie-coquille : les pointes actives de la fin");
    assert_eq!(colonie.apex().len(), 4731, "colonie-coquille : les apex de la fin");

    assert_eq!(
        colonie_finie(SCENE, 2).empreinte(),
        "95a9984930feb2e1e5ada2264d5021a8cd91309a8916eb48bc72b0f57bb628d7",
        "colonie-coquille, graine 2 : l'empreinte de la fin"
    );
    assert_eq!(
        colonie_finie(SCENE, 3).empreinte(),
        "1f36dc87f6b5b474c74acd669bb13ca8520faba4b8cf9e0acb2e7f6fb948218b",
        "colonie-coquille, graine 3 : l'empreinte de la fin"
    );
}

#[test]
fn le_moule_de_la_coquille_se_developpe_en_sept_obstacles() {
    let texte = texte_du_fichier(SCENE);
    let developpe = developper_3d(&texte).unwrap_or_else(|e| panic!("{SCENE} : refus inattendu du moule {e:?}"));

    let mut lignes_obstacle = 0usize;
    let mut lignes_moule = 0usize;
    for ligne in developpe.lines() {
        match ligne.split_whitespace().next() {
            Some("obstacle") => lignes_obstacle += 1,
            Some("moule") => lignes_moule += 1,
            _ => {}
        }
    }
    assert_eq!(lignes_obstacle, OBSTACLES, "le texte développé : sept lignes obstacle");
    assert_eq!(lignes_moule, 0, "le texte développé : aucune ligne moule");

    let scenario = lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("{SCENE} : refus inattendu {e:?}"));
    assert_eq!(scenario.obstacles.len(), OBSTACLES, "le scénario lu : sept obstacles");
    assert_eq!(scenario.spore, (3, 16, 16), "la spore est dans la paroi du côté x = 2");

    let colonie = Colonie3d::nouvelle(&scenario, 1).expect("la colonie de la coquille se construit");
    let substrat = colonie.substrat();
    let (sx, sy, sz) = scenario.spore;
    assert!(
        !substrat.est_bloquee(sx as usize, sy as usize, sz as usize),
        "la spore ({sx}, {sy}, {sz}) est dans une case libre"
    );
    let mut cases_vues = 1usize;
    for depot in &scenario.depots {
        let (x, y, z) = (depot.x as usize, depot.y as usize, depot.z as usize);
        assert!(!substrat.est_bloquee(x, y, z), "la nourriture ({x}, {y}, {z}) est dans une case libre");
        cases_vues += 1;
    }
    assert_eq!(cases_vues, 1 + scenario.depots.len(), "la spore et chaque nourriture ont été vues");
}

#[test]
fn la_cavite_est_respectee_par_les_segments() {
    let mut graines_vues = 0u64;
    for graine in 1..=3u64 {
        let colonie = colonie_finie(SCENE, graine);
        let substrat = colonie.substrat();
        let (largeur, hauteur, profondeur) =
            (substrat.largeur() as f64, substrat.hauteur() as f64, substrat.profondeur() as f64);
        let segments = colonie.segments();
        assert!(!segments.is_empty(), "graine {graine} : des segments à vérifier");

        let mut segments_vus = 0usize;
        for (numero, segment) in segments.iter().enumerate() {
            assert!(
                les_deux_bouts_dans_le_cube(segment, largeur, hauteur, profondeur),
                "graine {graine}, segment {numero} : les deux bouts ({}, {}, {}) et ({}, {}, {}) sont dans le cube",
                segment.x0,
                segment.y0,
                segment.z0,
                segment.x1,
                segment.y1,
                segment.z1
            );
            assert_eq!(
                traverser_3d(substrat, (segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1),),
                Traversee3d::Libre,
                "graine {graine}, segment {numero} : le segment ne touche aucune case bloquée"
            );
            segments_vus += 1;
        }
        assert_eq!(segments_vus, segments.len(), "graine {graine} : tous les segments sont vus");
        graines_vues += 1;
    }
    assert_eq!(graines_vues, 3, "les trois graines sont vérifiées");
}

#[test]
fn le_remplissage_de_la_coquille_est_ecrit() {
    let mut colonie = colonie(SCENE, 1);
    let duree = colonie.duree();
    let courbe = courbe_de_remplissage_3d(&mut colonie, duree);
    assert_eq!(courbe.len(), duree as usize + 1, "un point par pas, de 0 à la durée");
    assert_eq!(colonie.temps(), duree, "la courbe avance la colonie jusqu'à la durée");

    let mut points_vus = 0usize;
    for (pas, point) in courbe.iter().enumerate() {
        assert_eq!(point.pas, pas as u32, "le pas du point {pas}");
        assert_eq!(point.libres, CASES_LIBRES, "la coquille : les cases libres à chaque point");
        if pas > 0 {
            assert!(
                point.atteintes >= courbe[pas - 1].atteintes,
                "les cases atteintes ne décroissent jamais, au pas {pas}"
            );
        }
        points_vus += 1;
    }
    assert_eq!(points_vus, courbe.len(), "tous les points de la courbe sont vus");

    let dernier = courbe.last().expect("la courbe a un dernier point");
    assert_eq!(dernier.pas, duree, "le dernier point est à la durée");
    assert_eq!(dernier.atteintes, ATTEINTES_GRAINE_1, "colonie-coquille, graine 1 : les cases atteintes de la fin");
    assert_eq!(dernier.part, ATTEINTES_GRAINE_1 as f64 / CASES_LIBRES as f64, "la part est le quotient des deux");
    assert_eq!(dernier.part, PART_GRAINE_1, "colonie-coquille, graine 1 : la part écrite");

    let rejeu = rejeu_des_atteintes(&colonie);
    assert_eq!(rejeu, ATTEINTES_GRAINE_1, "le rejeu des arrivées redonne les cases atteintes de la graine 1");

    for (graine, attendues) in [(2u64, 7809u64), (3, 6906)] {
        let colonie = colonie_finie(SCENE, graine);
        assert_eq!(colonie.temps(), duree, "graine {graine} : le temps est la durée");
        assert_eq!(
            compter(&cases_atteintes(&colonie)),
            attendues,
            "colonie-coquille, graine {graine} : les cases atteintes de la fin"
        );
        assert_eq!(
            rejeu_des_atteintes(&colonie),
            attendues,
            "graine {graine} : le rejeu des arrivées redonne les cases atteintes"
        );
    }
}

#[test]
fn la_colonie_gagne_les_six_parois() {
    let mut graines_vues = 0u64;
    for graine in 1..=3u64 {
        let colonie = colonie_finie(SCENE, graine);
        assert_eq!(colonie.temps(), colonie.duree(), "graine {graine} : le temps est la durée");
        let parois = cases_des_six_parois(&colonie);
        for (numero, &cases) in parois.iter().enumerate() {
            assert!(cases > 1000, "graine {graine}, paroi {numero} : {cases} cases atteintes, plus de 1000 attendues");
        }
        if graine == 1 {
            assert_eq!(
                parois,
                [1593, 1704, 1413, 1858, 1414, 1860],
                "graine 1 : les six parois, dans l'ordre x ≤ 4, x ≥ 27, y ≤ 4, y ≥ 27, z ≤ 4, z ≥ 27"
            );
        }
        graines_vues += 1;
    }
    assert_eq!(graines_vues, 3, "les trois graines sont vérifiées");
}

#[test]
fn les_invariants_et_le_determinisme_tiennent() {
    let scenario = scenario(SCENE);
    let duree = scenario.duree;

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let masse_initiale = colonie.substrat().masse_totale();
        let mut temoin = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let mut un_a_un = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");

        colonie.avancer_n(duree);
        temoin.avancer_n(duree);
        for _ in 0..duree {
            un_a_un.avancer();
        }
        assert_eq!(colonie.temps(), duree, "graine {graine} : le temps est la durée");
        assert_eq!(un_a_un.temps(), colonie.temps(), "graine {graine} : le même temps");
        assert_eq!(
            un_a_un.ecrire_canonique(),
            colonie.ecrire_canonique(),
            "graine {graine} : avancer_n(n) et n appels à avancer, les mêmes octets"
        );
        assert_eq!(
            temoin.ecrire_canonique(),
            colonie.ecrire_canonique(),
            "graine {graine} : même graine, mêmes octets"
        );

        assert_eq!(
            colonie.apex().len() as u64,
            1 + colonie.nb_ramifications(),
            "graine {graine} : un apex pour la spore, plus un par ramification"
        );

        let mut consomme = 0.0;
        for segment in colonie.segments() {
            consomme += scenario.cout * segment.longueur();
        }
        let masse_finale = colonie.substrat().masse_totale();
        assert!(!colonie.segments().is_empty(), "graine {graine} : des segments consommés");
        assert!(consomme > 0.0, "graine {graine} : de la nourriture consommée : {consomme}");
        assert!(
            egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
            "graine {graine} : {masse_initiale} - {consomme} contre la masse finale {masse_finale}"
        );
    }

    let un = colonie_finie(SCENE, 1);
    let deux = colonie_finie(SCENE, 2);
    assert_ne!(un.empreinte(), deux.empreinte(), "les graines 1 et 2 donnent deux empreintes");
}
