//! Tests d'acceptation de l'étape 27 : la colonie dans un moule en boule de rayon 14.
//!
//! Ce qui est vérifié :
//! - les empreintes de `colonie-boule`, graine 1 : au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis à la
//!   fin de sa durée (l'empreinte de son fichier de `references/`), les comptes de la fin (segments, ramifications,
//!   pointes, apex), et les empreintes de la fin des graines 2 et 3 ;
//! - le moule : le texte développé a les 570 lignes `obstacle` de la tâche et aucune ligne `moule`, le scénario lu a
//!   autant d'obstacles, et la spore et chaque nourriture sont dans une case libre ;
//! - la cavité respectée, graines 1 à 3 : chaque segment va de son départ à son arrivée sans toucher de case bloquée
//!   (`traverser_3d` rend `Traversee3d::Libre`) et ses deux bouts sont dans le cube ;
//! - le remplissage : la courbe de la graine 1 jusqu'à la durée (un point par pas, 11513 cases libres, des cases
//!   atteintes qui ne décroissent jamais, 8705 atteintes et la part écrite au dernier point), le rejeu des arrivées
//!   écrit ici, et les cases atteintes des graines 2 et 3 ;
//! - la propriété de la forme : toute arrivée de segment reste à moins de 15,5 cases du centre de la case de la spore ;
//! - les invariants et le déterminisme, graines 1 à 3 : un apex pour la spore plus un par ramification, le bilan de la
//!   masse, `avancer_n(n)` égal à `n` appels à `avancer`, même graine mêmes octets, et les graines 1 et 2 différentes.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, lire_scenario_colonie3d};
use mycelium_lab::contact3d::{Traversee3d, traverser_3d};
use mycelium_lab::moule3d::{cases_atteintes, courbe_de_remplissage_3d, developper_3d};

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// Le scénario d'un fichier de `scenarios/` : son texte développé par le moule, puis lu.
fn scenario(nom: &str) -> ScenarioColonie3d {
    let texte = texte_du_fichier(nom);
    let developpe = developper_3d(&texte).unwrap_or_else(|e| panic!("{nom} : moule refusé {e:?}"));
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
    colonie.avancer_n(colonie.duree());
    colonie
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

/// Le nombre de cases vraies d'un tableau.
fn compter(cases: &[bool]) -> u64 {
    cases.iter().filter(|&&vue| vue).count() as u64
}

/// Le rejeu écrit ici des cases atteintes : une boucle sur les arrivées des segments, au plancher, bornées à la
/// dernière case, et seules les cases non bloquées comptent. Rend le nombre de cases distinctes vues.
fn rejeu_des_atteintes(colonie: &Colonie3d) -> u64 {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let substrat = colonie.substrat();
    let mut atteintes = vec![false; largeur * hauteur * profondeur];
    for segment in colonie.segments() {
        let x = (segment.x1.floor() as usize).min(largeur - 1);
        let y = (segment.y1.floor() as usize).min(hauteur - 1);
        let z = (segment.z1.floor() as usize).min(profondeur - 1);
        if !substrat.est_bloquee(x, y, z) {
            atteintes[(z * hauteur + y) * largeur + x] = true;
        }
    }
    compter(&atteintes)
}

#[test]
fn colonie_boule_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut colonie = colonie("colonie-boule.txt", 1);
    assert_eq!(
        colonie.empreinte(),
        "8a7d35b6af1b89b07c58b25f6f6db2b3adcff9ad1f220fc36219fbf79c842893",
        "colonie-boule : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-boule : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "d1077c3378365be6afd0ea4250e5142e3c1fdc407048427b234cc54dd117ca78",
        "colonie-boule : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-boule : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "a083e4987bbb5a1405642cb6b3a21b18f8904f3615bc926721c07102b3a4bfee",
        "colonie-boule : l'empreinte après 10 pas"
    );
    let duree = colonie.duree();
    colonie.avancer_n(duree - 10);
    assert_eq!(colonie.temps(), duree, "colonie-boule : le temps est la durée");
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-boule.empreinte"),
        "colonie-boule : l'empreinte de son fichier de références"
    );
    assert_eq!(colonie.segments().len(), 29560, "colonie-boule : les segments de la fin");
    assert_eq!(colonie.nb_ramifications(), 4241, "colonie-boule : les ramifications de la fin");
    assert_eq!(colonie.pointes(), 2874, "colonie-boule : les pointes actives de la fin");
    assert_eq!(colonie.apex().len(), 4242, "colonie-boule : les apex de la fin, actifs ou non");

    for (graine, attendue) in [
        (2u64, "19112d2309d88344f54fb972ed0e9f19c037b8559115b5c9d677086fbd9a2817"),
        (3, "59e0a14b4fbd7dcd414b6933dfc926026731e8d63cce9a5525ac2ab29ac6ed05"),
    ] {
        let colonie = colonie_finie("colonie-boule.txt", graine);
        assert_eq!(colonie.temps(), colonie.duree(), "graine {graine} : le temps est la durée");
        assert_eq!(colonie.empreinte(), attendue, "graine {graine} : l'empreinte de la fin");
    }
}

#[test]
fn le_moule_de_la_boule_se_developpe_en_les_obstacles_ecrits() {
    let texte = texte_du_fichier("colonie-boule.txt");
    let developpe = developper_3d(&texte).expect("le moule de la boule se développe");

    let lignes_obstacle = developpe.lines().filter(|ligne| ligne.split_whitespace().next() == Some("obstacle")).count();
    let lignes_moule = developpe.lines().filter(|ligne| ligne.split_whitespace().next() == Some("moule")).count();
    assert_eq!(lignes_obstacle, 570, "colonie-boule : 570 lignes obstacle");
    assert_eq!(lignes_moule, 0, "colonie-boule : aucune ligne moule après le développement");

    let scenario = lire_scenario_colonie3d(&developpe).expect("le texte développé se lit");
    assert_eq!(scenario.obstacles.len(), lignes_obstacle, "colonie-boule : un obstacle par ligne obstacle");
    assert_eq!(scenario.obstacles.len(), 570, "colonie-boule : 570 pavés bloqués");

    let colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    let substrat = colonie.substrat();
    let (sx, sy, sz) = (scenario.spore.0 as usize, scenario.spore.1 as usize, scenario.spore.2 as usize);
    assert!(!substrat.est_bloquee(sx, sy, sz), "colonie-boule : la case de la spore est libre");

    let mut nourritures_vues = 0usize;
    for depot in &scenario.depots {
        let (x, y, z) = (depot.x as usize, depot.y as usize, depot.z as usize);
        assert!(
            !substrat.est_bloquee(x, y, z),
            "colonie-boule : la nourriture ({x}, {y}, {z}) est dans une case libre"
        );
        nourritures_vues += 1;
    }
    assert_eq!(nourritures_vues, scenario.depots.len(), "colonie-boule : chaque nourriture est vue");
    assert!(scenario.depots.is_empty(), "colonie-boule : aucune ligne nourriture, le substrat est uniforme");
}

#[test]
fn la_cavite_de_la_boule_est_respectee_graines_1_a_3() {
    let scenario = scenario("colonie-boule.txt");
    let (largeur, hauteur, profondeur) = (scenario.largeur as f64, scenario.hauteur as f64, scenario.profondeur as f64);

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        colonie.avancer_n(colonie.duree());
        assert_eq!(colonie.temps(), colonie.duree(), "graine {graine} : le temps est la durée");

        let substrat = colonie.substrat();
        let segments = colonie.segments();
        assert!(!segments.is_empty(), "graine {graine} : des segments à vérifier");

        let mut bouts_vus = 0usize;
        for (numero, segment) in segments.iter().enumerate() {
            assert_eq!(
                traverser_3d(substrat, (segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)),
                Traversee3d::Libre,
                "graine {graine}, segment {numero} : un segment libre dans la cavité"
            );
            for (x, y, z) in [(segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)] {
                assert!(
                    (0.0..largeur).contains(&x) && (0.0..hauteur).contains(&y) && (0.0..profondeur).contains(&z),
                    "graine {graine}, segment {numero} : le bout ({x}, {y}, {z}) est dans le cube"
                );
                bouts_vus += 1;
            }
        }
        assert_eq!(bouts_vus, 2 * segments.len(), "graine {graine} : les deux bouts de chaque segment sont vus");
    }
}

#[test]
fn la_courbe_de_remplissage_de_la_boule_est_ecrite() {
    let scenario = scenario("colonie-boule.txt");
    let duree = scenario.duree;

    let mut colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    let courbe = courbe_de_remplissage_3d(&mut colonie, duree);
    assert_eq!(courbe.len(), duree as usize + 1, "un point par pas, de 0 à la durée");
    for (pas, point) in courbe.iter().enumerate() {
        assert_eq!(point.pas, pas as u32, "le pas du point {pas}");
        assert_eq!(point.libres, 11513, "la boule : 11513 cases libres au pas {pas}");
        if pas > 0 {
            assert!(
                point.atteintes >= courbe[pas - 1].atteintes,
                "les cases atteintes ne décroissent jamais, au pas {pas}"
            );
        }
    }
    let dernier = courbe.last().expect("la courbe a un dernier point");
    assert_eq!(dernier.pas, duree, "le dernier point est à la durée");
    assert_eq!(dernier.atteintes, 8705, "la boule : 8705 cases atteintes à la fin");
    assert_eq!(dernier.part, 0.756101797967515, "la boule : la part écrite");
    assert_eq!(dernier.part, dernier.atteintes as f64 / dernier.libres as f64, "la part est le quotient des deux");

    // Le rejeu écrit ici : les arrivées des segments, au plancher, dans les cases non bloquées.
    let rejeu = rejeu_des_atteintes(&colonie);
    assert_eq!(rejeu, 8705, "la boule : le rejeu trouve les 8705 cases atteintes");
    assert_eq!(rejeu, dernier.atteintes, "la boule : le rejeu et la courbe donnent le même nombre");
    assert_eq!(
        compter(&cases_atteintes(&colonie)),
        dernier.atteintes,
        "la boule : les cases atteintes de la bibliothèque redonnent le même nombre"
    );

    for (graine, atteintes) in [(2u64, 8612u64), (3, 9042)] {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let courbe = courbe_de_remplissage_3d(&mut colonie, duree);
        let dernier = courbe.last().expect("la courbe a un dernier point");
        assert_eq!(dernier.pas, duree, "graine {graine} : le dernier point est à la durée");
        assert_eq!(dernier.atteintes, atteintes, "graine {graine} : les cases atteintes de la fin");
        assert_eq!(rejeu_des_atteintes(&colonie), atteintes, "graine {graine} : le rejeu redonne ce nombre");
    }
}

#[test]
fn toute_arrivee_reste_pres_du_centre_de_la_case_de_la_spore() {
    let scenario = scenario("colonie-boule.txt");
    let centre = (scenario.spore.0 as f64 + 0.5, scenario.spore.1 as f64 + 0.5, scenario.spore.2 as f64 + 0.5);
    assert_eq!((scenario.spore.0, scenario.spore.1, scenario.spore.2), (16, 16, 16), "la spore est au centre du cube");
    assert_eq!(centre, (16.5, 16.5, 16.5), "le centre de la case de la spore");

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        colonie.avancer_n(colonie.duree());
        let segments = colonie.segments();
        assert!(!segments.is_empty(), "graine {graine} : des arrivées à mesurer");

        let mut arrivees_vues = 0usize;
        for segment in segments {
            let dx = segment.x1 - centre.0;
            let dy = segment.y1 - centre.1;
            let dz = segment.z1 - centre.2;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            assert!(
                distance < 15.5,
                "graine {graine} : l'arrivée ({}, {}, {}) est à {distance} du centre, 15,5 au plus attendu",
                segment.x1,
                segment.y1,
                segment.z1
            );
            arrivees_vues += 1;
        }
        assert_eq!(arrivees_vues, segments.len(), "graine {graine} : chaque arrivée est mesurée");
    }
}

#[test]
fn les_invariants_et_le_determinisme_tiennent_pour_les_graines_1_a_3() {
    let scenario = scenario("colonie-boule.txt");
    let duree = scenario.duree;
    let cout = scenario.cout;
    assert!(cout > 0.0, "colonie-boule : un coût de segment positif");

    let mut empreinte_1 = None;
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

        assert_eq!(d_un_coup.temps(), duree, "graine {graine} : le temps est la durée");
        assert_eq!(
            d_un_coup.apex().len() as u64,
            1 + d_un_coup.nb_ramifications(),
            "graine {graine} : un apex pour la spore, plus un par ramification"
        );

        // La nourriture : la masse du départ moins le coût des segments est la masse de la fin.
        let mut consomme = 0.0;
        for segment in d_un_coup.segments() {
            consomme += cout * segment.longueur();
        }
        assert!(!d_un_coup.segments().is_empty(), "graine {graine} : des segments consommés");
        assert!(consomme > 0.0, "graine {graine} : de la nourriture consommée : {consomme}");
        let masse_finale = d_un_coup.substrat().masse_totale();
        assert!(
            egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
            "graine {graine} : {masse_initiale} - {consomme} contre la masse finale {masse_finale}"
        );

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

        if graine == 1 {
            empreinte_1 = Some(d_un_coup.empreinte());
        } else if graine == 2 {
            let premiere = empreinte_1.clone().expect("la graine 1 est passée avant la graine 2");
            assert_ne!(d_un_coup.empreinte(), premiere, "les graines 1 et 2 donnent deux empreintes");
        }
    }
}
