//! Tests d'acceptation de l'étape 27 : la colonie dans un labyrinthe en volume.
//!
//! Le scénario `scenarios/colonie-labyrinthe.txt` est un labyrinthe parfait de 3 × 3 × 3 chambres de 4 cases (graine 4)
//! qui remplit un cube de 16 ; la spore est au centre de la chambre (0, 0, 0), une goutte au centre de la chambre
//! (2, 2, 2), la sortie. Ce qui est vérifié :
//!
//! - les empreintes de la graine 1 au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis à la fin de la
//!   durée celle de `references/colonie-labyrinthe.empreinte`, les comptes de la fin (segments, ramifications, pointes,
//!   apex) et les empreintes de la fin des graines 2 et 3 ;
//! - le moule : le texte développé a 37 lignes `obstacle` et aucune ligne `moule`, le scénario lu a 37 obstacles, et la
//!   spore comme la goutte sont dans une case libre ;
//! - la cavité respectée, graines 1 à 3 : chaque segment va de son départ à son arrivée par `traverser_3d` (qui rend
//!   `Libre`), et ses deux bouts sont dans le cube ;
//! - le remplissage : la courbe de la graine 1 jusqu'à la durée a un point par pas, les 2144 cases libres à chaque
//!   point, des cases atteintes qui ne décroissent jamais, et 1587 cases atteintes à la fin, soit la part
//!   `0.7402052238805971` (le quotient en `f64`) ; les cases atteintes des graines 2 et 3 (1236 et 1240) ; un rejeu
//!   écrit ici recompte les arrivées, cases non bloquées seulement, et trouve le même nombre ;
//! - la propriété de la forme : la colonie trouve la sortie, au pas 70 pour la graine 1, 94 pour la graine 2 et 92 pour
//!   la graine 3, et aucune arrivée ne tombe dans la chambre de sortie avant ;
//! - les invariants et le déterminisme, graines 1 à 3 : un apex pour la spore plus un par ramification, le bilan de la
//!   masse, `avancer_n(n)` égal à n appels à `avancer`, mêmes octets pour une même graine, et deux empreintes
//!   différentes pour les graines 1 et 2.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::contact3d::{Traversee3d, traverser_3d};
use mycelium_lab::moule3d::{cases_atteintes, chambre_3d, courbe_de_remplissage_3d, developper_3d, part_remplie};

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// Le scénario de la scène `nom`, développé par le moule puis lu.
fn scenario(nom: &str) -> ScenarioColonie3d {
    let developpe =
        developper_3d(&texte_du_fichier(nom)).unwrap_or_else(|e| panic!("{nom} : le moule refuse le texte : {e:?}"));
    lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"))
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

/// Une colonie de la scène `nom`, à la graine `graine`, au temps 0.
fn colonie(nom: &str, graine: u64) -> Colonie3d {
    Colonie3d::nouvelle(&scenario(nom), graine).unwrap_or_else(|e| panic!("{nom} : construction refusée {e:?}"))
}

/// Une colonie de la scène `nom`, à la graine `graine`, avancée jusqu'à sa durée.
fn colonie_finie(nom: &str, graine: u64) -> Colonie3d {
    let mut colonie = colonie(nom, graine);
    let duree = colonie.duree();
    colonie.avancer_n(duree);
    colonie
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

/// Le nombre de cases vraies d'un tableau.
fn compter(cases: &[bool]) -> u64 {
    cases.iter().filter(|&&libre| libre).count() as u64
}

/// Les cases atteintes, recomptées dans le test : l'arrivée de chaque segment, au plancher, bornée à la dernière case,
/// et seulement si la case n'est pas bloquée.
fn rejeu_des_atteintes_libres(colonie: &Colonie3d) -> Vec<bool> {
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

/// Vrai si l'arrivée du segment tombe dans la chambre `(x0, y0, z0, x1, y1, z1)`, règle du plancher.
fn arrivee_dans_la_chambre(segment: &Segment3d, chambre: (usize, usize, usize, usize, usize, usize)) -> bool {
    let (x0, y0, z0, x1, y1, z1) = chambre;
    let x = segment.x1.floor() as usize;
    let y = segment.y1.floor() as usize;
    let z = segment.z1.floor() as usize;
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (z0..=z1).contains(&z)
}

#[test]
fn colonie_labyrinthe_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut premiere = colonie("colonie-labyrinthe.txt", 1);
    assert_eq!(
        premiere.empreinte(),
        "82bedf5d68f364d591b1f6ea926c4cd25bd12fce217d6c279d11b418c6ba6a57",
        "colonie-labyrinthe : l'empreinte au temps 0"
    );
    premiere.avancer_n(1);
    assert_eq!(premiere.temps(), 1, "colonie-labyrinthe : un pas joué");
    assert_eq!(
        premiere.empreinte(),
        "f1b66be366985882e3a9705c673c2bc54b1d0e20e7e13cc7371d3d7c623270c9",
        "colonie-labyrinthe : l'empreinte après 1 pas"
    );
    premiere.avancer_n(9);
    assert_eq!(premiere.temps(), 10, "colonie-labyrinthe : dix pas joués");
    assert_eq!(
        premiere.empreinte(),
        "5431fa1c8c2e0d77a3db1324766d0e8addfecf138c35bdc6b1e07011afa0b309",
        "colonie-labyrinthe : l'empreinte après 10 pas"
    );
    let duree = premiere.duree();
    premiere.avancer_n(duree - 10);
    assert_eq!(premiere.temps(), duree, "colonie-labyrinthe : le temps est la durée");
    assert_eq!(
        premiere.empreinte(),
        empreinte_de_reference("colonie-labyrinthe.empreinte"),
        "colonie-labyrinthe : l'empreinte de son fichier de références"
    );
    assert_eq!(premiere.segments().len(), 31080, "colonie-labyrinthe : les segments de la fin");
    assert_eq!(premiere.nb_ramifications(), 4591, "colonie-labyrinthe : les ramifications de la fin");
    assert_eq!(premiere.pointes(), 3349, "colonie-labyrinthe : les pointes actives de la fin");
    assert_eq!(premiere.apex().len(), 4592, "colonie-labyrinthe : les apex de la fin");

    let mut graine2 = colonie("colonie-labyrinthe.txt", 2);
    let duree2 = graine2.duree();
    graine2.avancer_n(duree2);
    assert_eq!(graine2.temps(), duree2, "graine 2 : le temps est la durée");
    assert_eq!(
        graine2.empreinte(),
        "803b3e837768b849c9784e0852f9346649d1478ae72ceec890e0ca60f065ab24",
        "colonie-labyrinthe, graine 2 : l'empreinte de la fin"
    );

    let mut graine3 = colonie("colonie-labyrinthe.txt", 3);
    let duree3 = graine3.duree();
    graine3.avancer_n(duree3);
    assert_eq!(graine3.temps(), duree3, "graine 3 : le temps est la durée");
    assert_eq!(
        graine3.empreinte(),
        "c15fba24e07c74411f8a32851c27cab88c2ff6d54a07368a34d62f167ee23e3f",
        "colonie-labyrinthe, graine 3 : l'empreinte de la fin"
    );
}

#[test]
fn le_moule_se_developpe_en_trente_sept_obstacles_et_les_depots_sont_libres() {
    let texte = texte_du_fichier("colonie-labyrinthe.txt");
    let developpe = developper_3d(&texte).unwrap_or_else(|e| panic!("le moule du labyrinthe : {e:?}"));

    let mut lignes_obstacle = 0usize;
    let mut lignes_moule = 0usize;
    for brute in developpe.lines() {
        let ligne = brute.trim();
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        match ligne.split_whitespace().next() {
            Some("obstacle") => lignes_obstacle += 1,
            Some("moule") => lignes_moule += 1,
            _ => {}
        }
    }
    assert_eq!(lignes_obstacle, 37, "colonie-labyrinthe : 37 lignes `obstacle` dans le texte développé");
    assert_eq!(lignes_moule, 0, "colonie-labyrinthe : plus aucune ligne `moule` après le développement");

    let scenario = lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("texte développé : {e:?}"));
    assert_eq!(scenario.obstacles.len(), 37, "colonie-labyrinthe : 37 obstacles lus");

    let colonie = Colonie3d::nouvelle(&scenario, 1).unwrap_or_else(|e| panic!("construction : {e:?}"));
    let (sx, sy, sz) = (scenario.spore.0 as usize, scenario.spore.1 as usize, scenario.spore.2 as usize);
    assert!(!colonie.substrat().est_bloquee(sx, sy, sz), "la spore ({sx}, {sy}, {sz}) est dans une case libre");

    assert!(!scenario.depots.is_empty(), "colonie-labyrinthe : au moins une goutte de nourriture");
    let mut depots_vus = 0usize;
    for depot in &scenario.depots {
        let (x, y, z) = (depot.x as usize, depot.y as usize, depot.z as usize);
        assert!(!colonie.substrat().est_bloquee(x, y, z), "la nourriture ({x}, {y}, {z}) est dans une case libre");
        depots_vus += 1;
    }
    assert_eq!(depots_vus, scenario.depots.len(), "chaque nourriture a été vue");
}

#[test]
fn la_cavite_est_respectee_graines_1_a_3() {
    let scenario = scenario("colonie-labyrinthe.txt");
    let (largeur, hauteur, profondeur) = (scenario.largeur as f64, scenario.hauteur as f64, scenario.profondeur as f64);

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        colonie.avancer_n(colonie.duree());
        assert_eq!(colonie.temps(), colonie.duree(), "graine {graine} : le temps est la durée");
        assert!(!colonie.segments().is_empty(), "graine {graine} : des segments à vérifier");

        let substrat = colonie.substrat();
        let mut segments_vus = 0usize;
        for (numero, segment) in colonie.segments().iter().enumerate() {
            assert_eq!(
                traverser_3d(substrat, (segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)),
                Traversee3d::Libre,
                "graine {graine}, segment {numero} : un segment de la colonie est libre dans la cavité"
            );
            for (x, y, z) in [(segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)] {
                assert!(
                    (0.0..=largeur).contains(&x) && (0.0..=hauteur).contains(&y) && (0.0..=profondeur).contains(&z),
                    "graine {graine}, segment {numero} : le bout ({x}, {y}, {z}) est hors du cube"
                );
            }
            segments_vus += 1;
        }
        assert_eq!(segments_vus, colonie.segments().len(), "graine {graine} : tous les segments sont vus");
    }
}

#[test]
fn le_remplissage_du_labyrinthe_est_ecrit() {
    let mut colonie = colonie("colonie-labyrinthe.txt", 1);
    let duree = colonie.duree();
    let courbe = courbe_de_remplissage_3d(&mut colonie, duree);
    assert_eq!(courbe.len(), duree as usize + 1, "un point par pas, de 0 à la durée");
    let mut points_vus = 0usize;
    for (index, point) in courbe.iter().enumerate() {
        assert_eq!(point.pas, index as u32, "le pas du point {index}");
        assert_eq!(point.libres, 2144, "les cases libres de la tâche, au pas {index}");
        assert_eq!(
            point.part,
            point.atteintes as f64 / point.libres as f64,
            "la part est le quotient des cases atteintes par les cases libres, au pas {index}"
        );
        if index > 0 {
            assert!(
                point.atteintes >= courbe[index - 1].atteintes,
                "les cases atteintes ne décroissent jamais, au pas {index}"
            );
        }
        points_vus += 1;
    }
    assert_eq!(points_vus, duree as usize + 1, "tous les points de la courbe sont vus");

    let dernier = courbe.last().expect("la courbe a un dernier point");
    assert_eq!(dernier.pas, duree, "le dernier point est à la durée");
    assert_eq!(dernier.atteintes, 1587, "colonie-labyrinthe, graine 1 : 1587 cases atteintes à la fin");
    assert_eq!(dernier.part, 0.7402052238805971, "colonie-labyrinthe, graine 1 : la part de la fin");
    assert_eq!(dernier.part, 1587.0 / 2144.0, "la part est 1587 divisé par 2144");
    assert_eq!(part_remplie(&colonie), dernier.part, "la part de la colonie est celle du dernier point");

    // Le rejeu : une boucle sur les arrivées des segments, cases non bloquées seulement.
    let rejeu = rejeu_des_atteintes_libres(&colonie);
    assert!(!colonie.segments().is_empty(), "colonie-labyrinthe : des segments pour le rejeu");
    assert_eq!(compter(&rejeu), dernier.atteintes, "le rejeu des arrivées redonne les cases atteintes de la fin");

    for (graine, attendues) in [(2u64, 1236u64), (3, 1240)] {
        let colonie = colonie_finie("colonie-labyrinthe.txt", graine);
        assert_eq!(colonie.temps(), colonie.duree(), "graine {graine} : le temps est la durée");
        assert_eq!(compter(&cases_atteintes(&colonie)), attendues, "graine {graine} : les cases atteintes de la fin");
    }
}

#[test]
fn la_colonie_trouve_la_sortie() {
    assert_eq!(chambre_3d(4, 2, 2, 2), (11, 11, 11, 14, 14, 14), "la chambre de sortie, cases 11 à 14");
    let chambre = chambre_3d(4, 2, 2, 2);
    let scenario = scenario("colonie-labyrinthe.txt");

    for (graine, attendu) in [(1u64, 70u32), (2, 94), (3, 92)] {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let duree = colonie.duree();
        let mut sortie = None;
        let mut segments_vus = 0usize;
        for _ in 0..duree {
            colonie.avancer();
            let pas = colonie.temps();
            for segment in colonie.segments() {
                if segment.temps == pas {
                    segments_vus += 1;
                    if sortie.is_none() && arrivee_dans_la_chambre(segment, chambre) {
                        sortie = Some(pas);
                    }
                }
            }
        }
        assert!(segments_vus > 0, "graine {graine} : des segments vus");
        assert_eq!(
            sortie,
            Some(attendu),
            "graine {graine} : le premier pas où une arrivée tombe dans la chambre de sortie"
        );

        // Aucune arrivée ne tombe dans la chambre avant ce pas, et l'arrivée du pas est bien là.
        let avant = colonie.segments().iter().filter(|segment| segment.temps < attendu).count();
        assert!(avant > 0, "graine {graine} : des segments avant la sortie");
        assert!(
            colonie
                .segments()
                .iter()
                .filter(|segment| segment.temps < attendu)
                .all(|segment| !arrivee_dans_la_chambre(segment, chambre)),
            "graine {graine} : aucune arrivée dans la chambre de sortie avant le pas {attendu}"
        );
        assert!(
            colonie
                .segments()
                .iter()
                .any(|segment| segment.temps == attendu && arrivee_dans_la_chambre(segment, chambre)),
            "graine {graine} : une arrivée dans la chambre de sortie au pas {attendu}"
        );
    }
}

#[test]
fn les_invariants_et_le_determinisme_du_labyrinthe() {
    let scenario = scenario("colonie-labyrinthe.txt");
    let duree = scenario.duree;
    let mut empreintes = Vec::new();

    for graine in 1..=3u64 {
        let mut d_un_coup = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let masse_initiale = d_un_coup.substrat().masse_totale();
        let mut encore = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let mut un_a_un = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");

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
        assert!(consomme > 0.0, "graine {graine} : de la nourriture consommée : {consomme}");
        assert!(
            egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
            "graine {graine} : {masse_initiale} - {consomme} contre la masse finale {masse_finale}"
        );

        empreintes.push(d_un_coup.empreinte());
    }

    assert_eq!(empreintes.len(), 3, "les trois graines ont été jouées");
    assert_ne!(empreintes[0], empreintes[1], "les graines 1 et 2 donnent deux empreintes");
}
