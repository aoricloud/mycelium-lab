//! Tests d'acceptation de l'étape 7 (T2, les obstacles durs), partie « jamais un segment dans un obstacle ».
//!
//! Ils fixent : sur huit dispositifs (un mur, un mur avec une porte, un coin en L, des murs contigus et qui se chevauchent,
//! un escalier de blocs qui se touchent par les coins, une grille de piliers d'une case, une salle fermée à petite porte, une
//! boîte dont les quatre bords sont des murs), avec une colonie qui se ramifie et s'anastomose, **aucun segment ne traverse
//! un obstacle et aucune extrémité n'est dans une case bloquée**, pour `thigmotropisme 0` (8 graines), `35` (8 graines),
//! `10` et `90` (4 graines) : 192 colonies de 100 pas, plus 120 avec d'autres pas. La vérification géométrique est écrite dans `outils.rs`, sans
//! le code testé : recoupement exact du segment avec chaque rectangle (méthode des dalles, marge 1e-9) et échantillonnage
//! fin (18 points par segment, plancher de chaque point). Les mêmes dispositifs avec des pas de 0,5 et 2,5, aux angles 0, 35
//! et 90 (2 et 3 graines). Garde contre un test vide : à chaque réglage, la colonie longe les obstacles (assez d'extrémités de
//! segment à moins d'une case d'un obstacle, tous dispositifs confondus) et les compteurs ne sont pas tous nuls ; un second
//! contrôle sur les extrémités des pointes : toutes dans la boîte ; et le filet contre l'emballement des ramifications
//! (`outils::RAMIFICATIONS_MAXIMALES` par colonie, vérifié à chaque pas).
//!
//! Ils laissent ouverts : tout comportement de la colonie contre l'obstacle (voir `etape_7_contact.rs` et
//! `etape_7_comportement.rs`).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `floor`, `max` et `min`.

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::simulation::{Obstacle, Simulation};
use outils::{RAMIFICATIONS_MAXIMALES, construire, obstacle, premiere_faute};

/// Un dispositif : son nom, la spore, et ses rectangles `(x0, y0, x1, y1)`.
struct Dispositif {
    nom: &'static str,
    spore: (usize, usize),
    murs: Vec<(usize, usize, usize, usize)>,
}

fn dispositifs() -> Vec<Dispositif> {
    let mut piliers = Vec::new();
    for x in [15, 21, 27, 33, 39, 45] {
        for y in [15, 21, 27, 33, 39, 45] {
            piliers.push((x, y, x, y));
        }
    }
    vec![
        Dispositif { nom: "mur", spore: (10, 30), murs: vec![(30, 5, 33, 55)] },
        Dispositif { nom: "mur avec porte", spore: (10, 30), murs: vec![(30, 0, 33, 24), (30, 36, 33, 59)] },
        Dispositif { nom: "coin en L", spore: (45, 30), murs: vec![(30, 10, 33, 50), (30, 50, 55, 53)] },
        Dispositif {
            nom: "murs contigus",
            spore: (10, 30),
            murs: vec![(28, 5, 30, 55), (31, 5, 33, 55), (29, 20, 32, 40)],
        },
        Dispositif {
            nom: "escalier",
            spore: (10, 30),
            murs: vec![(20, 10, 24, 14), (25, 15, 28, 19), (29, 20, 32, 24), (33, 25, 36, 29), (37, 30, 40, 34)],
        },
        Dispositif { nom: "piliers", spore: (9, 29), murs: piliers },
        Dispositif {
            nom: "salle à petite porte",
            spore: (30, 30),
            murs: vec![(20, 20, 40, 21), (20, 38, 40, 39), (20, 20, 21, 39), (39, 20, 40, 27), (39, 32, 40, 39)],
        },
        Dispositif {
            nom: "boîte murée",
            spore: (30, 30),
            murs: vec![(0, 0, 59, 1), (0, 58, 59, 59), (0, 0, 1, 59), (58, 0, 59, 59)],
        },
    ]
}

/// Le texte du scénario d'un dispositif : une colonie qui se ramifie (intervalle 8, angle 40), marche avec une amplitude de
/// 25°, s'anastomose (rayon 0,8, délai 8), 100 pas, avec un pas de croissance `pas`.
fn texte(d: &Dispositif, thigmotropisme: f64, pas: f64) -> String {
    let mut t = format!(
        "boite 60 60\nspore {} {}\nsubstrat 1.0 0.2\ncroissance {pas} 8.0 40 25\ncouplage 0.1 0.002\nanastomose 0.8 8\nduree 100\n",
        d.spore.0, d.spore.1
    );
    for (x0, y0, x1, y1) in &d.murs {
        t.push_str(&format!("obstacle {x0} {y0} {x1} {y1}\n"));
    }
    t.push_str(&format!("thigmotropisme {thigmotropisme}\n"));
    t
}

/// Vrai si le point est à moins d'une case du rectangle plein de l'obstacle.
fn pres_de(o: &Obstacle, x: f64, y: f64) -> bool {
    let dx = (o.x0 as f64 - x).max(0.0).max(x - (o.x1 as f64 + 1.0));
    let dy = (o.y0 as f64 - y).max(0.0).max(y - (o.y1 as f64 + 1.0));
    dx * dx + dy * dy < 1.0
}

/// Joue un dispositif avec une graine et un angle, vérifie les segments et rend `(extrémités près d'un obstacle, segments)`.
fn controler(d: &Dispositif, thigmotropisme: f64, pas: f64, graine: u64) -> (usize, usize, Simulation) {
    let (mut simulation, duree) = construire(&texte(d, thigmotropisme, pas), graine);
    for pas in 1..=duree {
        simulation.avancer();
        assert!(
            simulation.ramifications() <= RAMIFICATIONS_MAXIMALES,
            "{}, thigmotropisme {thigmotropisme}, graine {graine} : {} ramifications au pas {pas} (emballement)",
            d.nom,
            simulation.ramifications()
        );
    }
    let obstacles: Vec<Obstacle> = d.murs.iter().map(|&(a, b, c, e)| obstacle(a, b, c, e)).collect();
    if let Some(faute) = premiere_faute(simulation.segments(), &obstacles) {
        panic!("{}, thigmotropisme {thigmotropisme}, graine {graine} : {faute}", d.nom);
    }
    let mut pres = 0;
    for s in simulation.segments() {
        assert!(
            s.x1 >= 0.0 && s.y1 >= 0.0 && s.x1 < 60.0 && s.y1 < 60.0,
            "{}, thigmotropisme {thigmotropisme}, graine {graine} : extrémité hors de la boîte {s:?}",
            d.nom
        );
        if obstacles.iter().any(|o| pres_de(o, s.x1, s.y1)) {
            pres += 1;
        }
    }
    let segments = simulation.segments().len();
    (pres, segments, simulation)
}

/// Joue tous les dispositifs avec `graines` graines (de 0) à l'angle donné et rend le total des extrémités près d'un obstacle
/// et celui des segments.
fn tout_controler(thigmotropisme: f64, pas: f64, graines: u64) -> (usize, usize, u64) {
    let (mut pres, mut segments, mut ramifications) = (0, 0, 0);
    for d in dispositifs() {
        for graine in 0..graines {
            let (p, s, simulation) = controler(&d, thigmotropisme, pas, graine);
            pres += p;
            segments += s;
            ramifications += simulation.ramifications();
        }
    }
    (pres, segments, ramifications)
}

/// À `thigmotropisme 0`, 8 dispositifs × 8 graines : jamais un segment dans un obstacle. (La pointe qui touche reste là et
/// retente : elle ne traverse pas.)
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn jamais_un_segment_dans_un_obstacle_sans_thigmotropisme() {
    outils::delai();
    let (pres, segments, _) = tout_controler(0.0, 1.0, 8);
    assert!(pres >= 300, "garde : seulement {pres} extrémités près d'un obstacle");
    assert!(segments >= 10_000, "garde : seulement {segments} segments");
}

/// À `thigmotropisme 35` (la valeur publiée), 8 dispositifs × 8 graines : jamais un segment dans un obstacle, que la pointe
/// longe l'obstacle ou s'arrête et se ramifie contre lui, y compris dans les coins concaves et convexes, entre les piliers,
/// dans la boîte murée.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn jamais_un_segment_dans_un_obstacle_a_35_degres() {
    outils::delai();
    let (pres, segments, ramifications) = tout_controler(35.0, 1.0, 8);
    assert!(pres >= 300, "garde : seulement {pres} extrémités près d'un obstacle");
    assert!(segments >= 10_000, "garde : seulement {segments} segments");
    assert!(ramifications > 0, "garde : aucune ramification");
}

/// Aux angles critiques 10 et 90 (à 90, toute rencontre non perpendiculaire est longée), 8 dispositifs × 4 graines :
/// jamais un segment dans un obstacle.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn jamais_un_segment_dans_un_obstacle_aux_angles_10_et_90() {
    outils::delai();
    for a in [10.0, 90.0] {
        let (pres, segments, _) = tout_controler(a, 1.0, 4);
        assert!(pres >= 100, "garde, angle {a} : seulement {pres} extrémités près d'un obstacle");
        assert!(segments >= 3_000, "garde, angle {a} : seulement {segments} segments");
    }
}

/// Avec des pas de croissance de 0,5 et de 2,5 (la pointe touche alors un obstacle jusqu'à plus d'une case avant lui, et la
/// première case bloquée peut être à plusieurs cases), aux angles 0, 35 et 90, 8 dispositifs × 2 ou 3 graines : jamais un
/// segment dans un obstacle.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn jamais_un_segment_dans_un_obstacle_avec_des_pas_de_0_5_et_2_5() {
    outils::delai();
    for (pas, graines) in [(0.5, 2), (2.5, 3)] {
        for a in [0.0, 35.0, 90.0] {
            let (pres, segments, _) = tout_controler(a, pas, graines);
            assert!(pres >= 50, "garde, pas {pas}, angle {a} : seulement {pres} extrémités près d'un obstacle");
            assert!(segments >= 500, "garde, pas {pas}, angle {a} : seulement {segments} segments");
        }
    }
}

/// Garde du dispositif de contrôle : le contrôle détecte un segment qui traverse (un faux segment posé à la main dans
/// chacun des cas : en travers, par un coin, dans la case, extrémité sur une face) et laisse passer ceux qui longent ou
/// touchent un coin.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_verification_geometrique_detecte_les_fautes_et_laisse_passer_les_contacts() {
    outils::delai();
    use mycelium_lab::croissance::Segment;
    let seg = |x0: f64, y0: f64, x1: f64, y1: f64| Segment { apex: 0, temps: 1, x0, y0, x1, y1 };
    let o = [obstacle(5, 5, 6, 6)]; // le rectangle plein [5, 7] × [5, 7]
    // Fautes.
    for (nom, s) in [
        ("en travers", seg(3.0, 5.5, 9.0, 5.5)),
        ("diagonale", seg(4.0, 4.0, 8.0, 8.0)),
        ("par un coin et dedans", seg(4.5, 4.5, 5.5, 5.5)),
        ("finit dedans", seg(3.0, 6.0, 5.2, 6.0)),
        ("finit exactement sur la face basse du rectangle", seg(5.5, 3.0, 5.5, 5.0)),
        ("part de dedans", seg(6.0, 6.0, 8.0, 3.0)),
    ] {
        assert!(premiere_faute(&[s], &o).is_some(), "la faute « {nom} » n'est pas détectée");
    }
    // Pas des fautes.
    for (nom, s) in [
        ("à côté", seg(3.0, 4.5, 9.0, 4.5)),
        ("longe la face droite", seg(7.0, 3.0, 7.0, 9.0)),
        ("coin exact seulement (hors des points échantillonnés)", seg(4.25, 5.75, 5.75, 4.25)),
        ("finit juste avant", seg(3.0, 5.5, 4.999, 5.5)),
        ("loin", seg(1.0, 1.0, 2.0, 2.0)),
    ] {
        assert_eq!(premiere_faute(&[s], &o), None, "« {nom} » est détecté à tort");
    }
}
