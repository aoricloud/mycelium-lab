//! Tests d'acceptation de l'étape 7 (T2, le thigmotropisme), partie comportement : sur `scenarios/mur.txt` (une spore à
//! gauche, un mur de 4 colonnes sur 61 lignes devant elle, 120 pas), ce que le thigmotropisme change à la colonie, **en
//! moyenne sur 120 graines** (1 à 120).
//!
//! Ils fixent, comme pour le chimiotropisme de T1, des propriétés en moyenne (une graine seule ne prouve rien : certaines
//! colonies s'éteignent tôt), avec des marges mesurées par le juge sur le prototype corrigé, sur des jeux disjoints de 120
//! graines (le détail et la marge en écarts-types sont dans son rapport) :
//!
//! - **la colonie épouse le mur** : la part de la biomasse (somme des longueurs des segments) dont le segment finit à moins
//!   de 1,5 case du rectangle du mur est plus grande, avec `thigmotropisme 35`, d'au moins 3 points qu'à angle nul (mesuré
//!   sur 20 autres jeux de 120 graines : 8,8 ± 0,6 points, au moins 7,2 ; à angle nul, une pointe qui touche le mur reste
//!   où elle est et ne laisse aucun segment contre lui) ;
//! - **elle se ramifie contre lui** : les ramifications par colonie sont au moins 1,3 fois celles de l'angle nul (mesuré :
//!   2,30 ± 0,18 fois, au moins 2,04 ; chaque arrêt fait naître jusqu'à deux pointes) ;
//! - **longer le contourne** : à 90° (toute rencontre non perpendiculaire est longée, puis le cap est retrouvé au bout du
//!   mur), la **part** de la biomasse au-delà du mur (segments qui finissent à `x ≥ 54`) est plus grande d'au moins 2 points
//!   qu'à angle nul (mesuré : 13,3 ± 2,3 points, au moins 9,8) ;
//! - toujours : aucun segment dans le mur (contrôle géométrique de `outils.rs`), et le filet contre l'emballement.
//!
//! Chaque seuil est à au moins 5 écarts-types de la moyenne mesurée sur les 20 autres jeux.
//!
//! Ils laissent ouverts : la **part** de la biomasse au-delà du mur à 35° (à peine plus grande qu'à angle nul : 3,4 ± 2,2
//! points, parfois négative) et la biomasse absolue au-delà du mur à 35° (186 ± 40 de plus par colonie : l'écart est réel
//! mais trop variable d'un jeu de graines à l'autre pour un seuil à 5 écarts-types) ; les valeurs exactes de ces mesures.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `max` et `sqrt`.

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::simulation::{Obstacle, lire_scenario_tropismes};
use outils::{jouer, premiere_faute};

/// Les graines rejouées : de 1 à `GRAINES`.
const GRAINES: u64 = 120;

/// Ce que l'on mesure sur les 120 colonies d'un angle critique (sommes sur les colonies).
struct Mesure {
    biomasse: f64,
    au_dela: f64,
    colle: f64,
    ramifications: f64,
}

/// La distance de `(x, y)` au rectangle plein du mur.
fn distance_au_mur(mur: &Obstacle, x: f64, y: f64) -> f64 {
    let dx = (mur.x0 as f64 - x).max(0.0).max(x - (mur.x1 as f64 + 1.0));
    let dy = (mur.y0 as f64 - y).max(0.0).max(y - (mur.y1 as f64 + 1.0));
    (dx * dx + dy * dy).sqrt()
}

/// Joue `scenarios/mur.txt` pour les graines 1 à 120 avec l'angle `angle` (celui du fichier est remplacé).
fn mesurer(angle: f64) -> Mesure {
    let texte = include_str!("../scenarios/mur.txt").replace("thigmotropisme 35", &format!("thigmotropisme {angle}"));
    let (_, milieu, _, _) = lire_scenario_tropismes(&texte).expect("le scénario se lit");
    let mur = milieu.obstacles[0].clone();
    let bord_droit = (mur.x1 + 1) as f64; // 54
    let mut m = Mesure { biomasse: 0.0, au_dela: 0.0, colle: 0.0, ramifications: 0.0 };
    for graine in 1..=GRAINES {
        let simulation = jouer(&texte, graine);
        if let Some(faute) = premiere_faute(simulation.segments(), &milieu.obstacles) {
            panic!("angle {angle}, graine {graine} : {faute}");
        }
        m.ramifications += simulation.ramifications() as f64;
        for s in simulation.segments() {
            let l = s.longueur();
            m.biomasse += l;
            if s.x1 >= bord_droit {
                m.au_dela += l;
            }
            if distance_au_mur(&mur, s.x1, s.y1) <= 1.5 {
                m.colle += l;
            }
        }
    }
    m
}

/// À 35° contre l'angle nul : la colonie épouse le mur (part collée : au moins 3 points de plus) et s'y ramifie (au moins
/// 1,3 fois les ramifications).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn avec_le_thigmotropisme_la_colonie_epouse_le_mur_et_s_y_ramifie() {
    outils::delai();
    let (zero, trente_cinq) = (mesurer(0.0), mesurer(35.0));
    let n = GRAINES as f64;
    let (colle_0, colle_35) = (100.0 * zero.colle / zero.biomasse, 100.0 * trente_cinq.colle / trente_cinq.biomasse);
    assert!(
        colle_35 >= colle_0 + 3.0,
        "part collée au mur : {colle_35:.1} % à 35° contre {colle_0:.1} % à l'angle nul (au moins 3 points d'écart attendus)"
    );
    assert!(
        trente_cinq.ramifications >= 1.3 * zero.ramifications,
        "ramifications par colonie : {:.1} à 35° contre {:.1} à l'angle nul (au moins 1,3 fois)",
        trente_cinq.ramifications / n,
        zero.ramifications / n
    );
}

/// À 90° contre l'angle nul : quand toute rencontre non perpendiculaire est longée, la part de la biomasse au-delà du mur
/// est plus grande d'au moins 2 points (le glissement et le retour au cap font contourner le mur).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn longer_le_mur_le_fait_contourner() {
    outils::delai();
    let (zero, quatre_vingt_dix) = (mesurer(0.0), mesurer(90.0));
    let (part_0, part_90) =
        (100.0 * zero.au_dela / zero.biomasse, 100.0 * quatre_vingt_dix.au_dela / quatre_vingt_dix.biomasse);
    assert!(
        part_90 >= part_0 + 2.0,
        "part au-delà du mur : {part_90:.1} % à 90° contre {part_0:.1} % à l'angle nul (au moins 2 points d'écart attendus)"
    );
}
