//! Tests d'acceptation de l'étape 7 (T2, le thigmotropisme), partie contact : ce que fait une pointe qui touche un obstacle,
//! à `thigmotropisme 0` et à `thigmotropisme a > 0`, et la mémoire du cap.
//!
//! Le dispositif : des colonies sans anastomose, sur un substrat qui ne s'épuise pas et ne diffuse pas (coût nul, diffusion
//! nulle) : chaque pas a la longueur exacte `(pas · s) / (affinite + s)` de la V0, `s` valant 1 ou la quantité d'une source.
//! Leur état se calcule ici, à l'octet près, par le **modèle de référence** de `outils.rs` (la spécification récrite à part, géométrie exacte de la règle
//! du plancher) : chaque test compare la simulation au modèle **après chaque pas** (segments, compteurs, apex, mémoires de
//! l'écriture canonique) et vérifie en plus, à la main, la propriété qu'il nomme. Les salles ont des murs sur leurs bords
//! gauche et haut (les pointes ne s'approchent jamais de `x = 0` ou `y = 0`), et la direction de départ est fixée par le choix
//! de la graine.
//!
//! Ils fixent : à `thigmotropisme 0`, la pointe qui touche un mur reste où elle est (4 murs, les directions jusqu'à 85° de
//! chaque côté de la normale) ; à `thigmotropisme 35`, sur les mêmes 684 cas : `α < 35` → elle longe (une seule pointe, sa
//! mémoire `(d, g)`, son pas refait dans la direction `g` au pas du contact) ; `α ≥ 35` → elle s'arrête (inactive, sans
//! mémoire) et deux pointes sans mémoire naissent à sa position, la plus petite direction parallèle recevant le plus petit
//! identifiant ; le seuil suit l'angle critique (7 angles, dont 34,5, 35, 35,5, 89,5 et 90) ; les pas autres que 1, avec ou
//! sans affinité (la sonde du retour a la longueur `pas`, le pas réel la longueur du substrat) ; avec la marche au hasard, sur
//! des murs finis, un coin en L, des piliers, une salle et un mur jusqu'au bord, 324 colonies suivent le modèle sur 120 pas
//! (tirages de la marche et de `δ` dans l'ordre de la spécification, retour au cap, naissances seulement si un pas plein est
//! libre) ; après le bout d'un mur, le cap est retrouvé à 20° près, au plus un pas après le bout ; au bord de la boîte, une
//! pointe qui longe s'arrête de longer et oublie ; dans un coin concave, seule la branche qui a la place naît.
//!
//! Ils laissent ouverts : rien de ce que la spécification fixe pour ces dispositifs ; le comportement pour des coordonnées
//! à l'ULP d'une ligne de la grille (aucun cas n'en dépend ici).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `floor`.

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::croissance::Segment;
use mycelium_lab::tropisme::Face;
use outils::{
    Salle, angle_attendu, apex_lus, direction_du_segment, ecart_circulaire, glissement_attendu, graines_par_direction,
    graines_pour, premiere_faute, suivre_le_modele,
};

/// Un mur d'une salle de contact, vu par la pointe : à sa droite, en dessous, à sa gauche, au-dessus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cote {
    Droite,
    Bas,
    Gauche,
    Haut,
}

const COTES: [Cote; 4] = [Cote::Droite, Cote::Bas, Cote::Gauche, Cote::Haut];

impl Cote {
    /// La direction de la normale au mur, vers le mur (les `y` croissent vers le bas).
    fn normale(self) -> usize {
        match self {
            Cote::Droite => 0,
            Cote::Bas => 90,
            Cote::Gauche => 180,
            Cote::Haut => 270,
        }
    }

    fn face(self) -> Face {
        if matches!(self, Cote::Droite | Cote::Gauche) { Face::Verticale } else { Face::Horizontale }
    }

    fn direction(self, ecart: i32) -> usize {
        (self.normale() as i32 + ecart).rem_euclid(360) as usize
    }

    /// La salle : une boîte longue de 80 cases, des murs de deux cases sur les bords gauche et haut, et pour `Droite` et
    /// `Bas` un mur de deux cases sur le bord opposé ; la spore est à 1,5 case de la face du mur.
    fn salle(self, pas: f64, affinite: f64, thigmotropisme: f64) -> Salle {
        let (largeur, hauteur, spore, mut murs) = match self {
            Cote::Droite => (24, 80, (20, 40), vec![(22, 0, 23, 79)]),
            Cote::Gauche => (24, 80, (3, 40), vec![]),
            Cote::Bas => (80, 24, (40, 20), vec![(0, 22, 79, 23)]),
            Cote::Haut => (80, 24, (40, 3), vec![]),
        };
        murs.push((0, 0, 1, hauteur - 1));
        murs.push((0, 0, largeur - 1, 1));
        Salle {
            largeur,
            hauteur,
            spore,
            murs,
            pas,
            affinite,
            amplitude: 0,
            intervalle: f64::INFINITY,
            thigmotropisme,
            sources: vec![],
            chimiotropisme: 0.0,
        }
    }
}

/// Le pas où la pointe 0 du modèle touche le mur pour la première fois (elle prend une mémoire ou s'arrête), et le modèle
/// à ce pas.
fn pas_du_contact(salle: &Salle, graine: u64) -> u32 {
    let mut modele = salle.modele(graine);
    for k in 1..=400 {
        let avant = modele.pointes[0];
        modele.avancer();
        let apres = modele.pointes[0];
        if apres.memoire.is_some() || !apres.actif || (apres.x, apres.y) == (avant.x, avant.y) {
            return k;
        }
    }
    panic!("pas de contact en 400 pas");
}

/// Joue une pointe de direction `d` dans la salle jusqu'au contact puis `apres` pas, en suivant le modèle à chaque pas.
/// Rend la simulation, le modèle et le pas du contact.
fn jouer_le_contact(
    salle: &Salle,
    d: usize,
    graines: &[u64],
    apres: u32,
    contexte: &str,
) -> (mycelium_lab::simulation::Simulation, outils::Modele, u32) {
    let k = pas_du_contact(salle, graines[d]);
    let (simulation, modele) = suivre_le_modele(salle, graines[d], k + apres, contexte);
    (simulation, modele, k)
}

/// Les segments d'une pointe.
fn segments_de(segments: &[Segment], apex: u32) -> Vec<Segment> {
    segments.iter().copied().filter(|s| s.apex == apex).collect()
}

/// À `thigmotropisme 0` (le défaut), une pointe qui touche un mur reste où elle est, avec sa direction : elle n'a laissé
/// que les segments de son trajet droit, aucune naissance, et elle est toujours active. 4 murs × 171 directions.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn sans_thigmotropisme_la_pointe_reste_au_contact_du_mur() {
    outils::delai();
    let graines = graines_par_direction();
    for cote in COTES {
        let salle = cote.salle(1.0, 0.0, 0.0);
        for ecart in -85..=85 {
            let d = cote.direction(ecart);
            let contexte = format!("{cote:?}, direction {d}, angle 0");
            let (simulation, _, k) = jouer_le_contact(&salle, d, &graines, 8, &contexte);
            assert_eq!(simulation.segments().len() as u32, k - 1, "{contexte} : seulement le trajet droit");
            assert_eq!(
                (simulation.pointes(), simulation.ramifications()),
                (1, 0),
                "{contexte} : une pointe, aucune naissance"
            );
            let apex = apex_lus(&simulation);
            assert_eq!(apex[0].direction as usize, d, "{contexte} : la direction est gardée");
        }
    }
}

/// À `thigmotropisme 35` (la valeur publiée), sur 4 murs × 171 directions : la pointe longe si l'angle de contact `α`
/// (l'écart à la parallèle la plus proche, calculé ici) est inférieur à 35 ; sinon elle s'arrête et deux pointes naissent.
/// La suite est vérifiée cinq pas après le contact, et le modèle est suivi à chaque pas.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn a_35_la_pointe_longe_sous_35_degres_et_s_arrete_au_dela() {
    outils::delai();
    let graines = graines_par_direction();
    let (mut longe, mut arrete) = (0, 0);
    for cote in COTES {
        let salle = cote.salle(1.0, 0.0, 35.0);
        for ecart in -85..=85 {
            let d = cote.direction(ecart);
            let contexte = format!("{cote:?}, direction {d}, angle 35");
            let (simulation, _, k) = jouer_le_contact(&salle, d, &graines, 5, &contexte);
            controler_longe_ou_arret(&simulation, cote, d, 35.0, k, &contexte);
            if (angle_attendu(d, cote.face()) as f64) < 35.0 {
                longe += 1;
            } else {
                arrete += 1;
            }
        }
    }
    assert_eq!((longe, arrete), (4 * 2 * 30, 4 * (171 - 60)), "répartition des cas (α de 5 à 34 de chaque côté)");
}

/// Le contrôle à la main d'un contact, cinq pas après le pas du contact `k`.
fn controler_longe_ou_arret(
    simulation: &mycelium_lab::simulation::Simulation,
    cote: Cote,
    d: usize,
    a: f64,
    k: u32,
    contexte: &str,
) {
    let alpha = angle_attendu(d, cote.face());
    let apex = apex_lus(simulation);
    let segments = simulation.segments();
    if (alpha as f64) < a {
        let g = glissement_attendu(d, cote.face());
        assert_eq!((simulation.pointes(), simulation.ramifications()), (1, 0), "{contexte} (longe, α = {alpha})");
        assert_eq!(apex[0].memoire, Some((1, d as u32, g as u32)), "{contexte} (longe) : mémoire (cap = d, g)");
        let apres: Vec<&Segment> = segments.iter().filter(|s| s.temps >= k).collect();
        assert_eq!(apres.len(), 6, "{contexte} (longe) : un segment au pas du contact et cinq ensuite");
        for s in apres {
            assert_eq!(direction_du_segment(s), g, "{contexte} (longe) : segment {s:?} dans la direction g");
        }
    } else {
        assert_eq!((simulation.pointes(), simulation.ramifications()), (2, 2), "{contexte} (s'arrête, α = {alpha})");
        assert!(!apex[0].actif, "{contexte} (s'arrête) : la pointe d'origine est inactive");
        assert_eq!(segments_de(segments, 0).len() as u32, k - 1, "{contexte} (s'arrête) : rien au pas du contact");
        let (p, q) = if cote.face() == Face::Verticale { (90, 270) } else { (0, 180) };
        for (id, attendue) in [(1u32, p), (2u32, q)] {
            assert_eq!(apex[id as usize].direction, attendue, "{contexte} : la pointe {id} (ordre des directions)");
            assert_eq!(apex[id as usize].memoire, Some((0, 0, 0)), "{contexte} : la pointe {id} naît sans mémoire");
            let siens = segments_de(segments, id);
            assert_eq!(siens.len(), 5, "{contexte} : la pointe {id} avance de cinq pas");
            assert_eq!((siens[0].x0, siens[0].y0), (apex[0].x, apex[0].y), "{contexte} : née à la position de l'arrêt");
            for s in siens {
                assert_eq!(direction_du_segment(&s), attendue as usize, "{contexte} : la pointe {id} va droit");
            }
        }
        assert_eq!(apex[0].memoire, Some((0, 0, 0)), "{contexte} : la pointe arrêtée n'a pas de mémoire");
    }
}

/// Le seuil suit l'angle critique : pour 7 angles (10, 34,5, 35, 35,5, 60, 89,5, 90), sur les murs droit et bas, les
/// directions dont l'angle de contact est à moins de 2° du seuil et une sur quinze : `α < a` longe, `α ≥ a` s'arrête
/// (donc `α = a` s'arrête, et à 89,5 ou 90 tout est longé sauf la perpendiculaire).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_seuil_suit_l_angle_critique() {
    outils::delai();
    let graines = graines_par_direction();
    for a in [10.0, 34.5, 35.0, 35.5, 60.0, 89.5, 90.0] {
        for cote in [Cote::Droite, Cote::Bas] {
            let salle = cote.salle(1.0, 0.0, a);
            for ecart in -85i32..=85 {
                let d = cote.direction(ecart);
                let alpha = angle_attendu(d, cote.face()) as f64;
                if (alpha - a).abs() > 2.0 && ecart % 15 != 0 {
                    continue;
                }
                let contexte = format!("{cote:?}, direction {d}, α = {alpha}, angle critique {a}");
                let (simulation, _, k) = jouer_le_contact(&salle, d, &graines, 5, &contexte);
                controler_longe_ou_arret(&simulation, cote, d, a, k, &contexte);
            }
        }
    }
}

/// Des pas autres que 1, avec ou sans affinité (`pas` 0,6 et 2,5, affinité 0 et 0,25 : longueur réelle `pas` ou
/// `0,8 · pas`), aux angles 35 et 90 : la simulation suit le modèle pendant 30 pas sur les 4 murs, une direction sur cinq.
/// La sonde du retour a la longueur `pas`, le pas réel la longueur que donne le substrat.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_pas_autres_que_1_suivent_le_modele() {
    outils::delai();
    let graines = graines_par_direction();
    for (pas, affinite) in [(0.6, 0.0), (0.6, 0.25), (2.5, 0.0), (2.5, 0.25)] {
        for a in [35.0, 90.0] {
            for cote in COTES {
                let salle = cote.salle(pas, affinite, a);
                for ecart in (-85..=85).step_by(5) {
                    let d = cote.direction(ecart);
                    let contexte = format!("{cote:?}, direction {d}, pas {pas}, affinité {affinite}, angle {a}");
                    suivre_le_modele(&salle, graines[d], 30, &contexte);
                }
            }
        }
    }
}

/// Les dispositifs à marche au hasard : une boîte 60 × 60 aux bords gauche et haut murés, la spore en (12, 30).
/// Un dispositif : son nom et ses rectangles `(x0, y0, x1, y1)`.
type Dispositif = (&'static str, Vec<(usize, usize, usize, usize)>);

fn dispositifs() -> Vec<Dispositif> {
    let mut piliers = Vec::new();
    for x in [20, 26, 32, 38, 44] {
        for y in [14, 20, 26, 32, 38, 44] {
            piliers.push((x, y, x, y));
        }
    }
    vec![
        ("mur fini", vec![(30, 15, 33, 45)]),
        ("coin en L", vec![(30, 8, 33, 40), (16, 40, 33, 43)]),
        ("piliers", piliers),
        ("salle à porte", vec![(6, 22, 24, 23), (6, 38, 24, 39), (24, 22, 25, 27), (24, 33, 25, 39)]),
        ("mur jusqu'au bord", vec![(30, 20, 33, 59)]),
        ("murs en damier", vec![(24, 24, 27, 27), (28, 28, 31, 31), (24, 32, 27, 35), (32, 24, 35, 27)]),
    ]
}

/// Avec la marche au hasard (amplitudes 15 et 40), des pas de 1 et 0,7, une affinité de 0 ou 0,25, aux angles 35 et 60, avec
/// ou sans ramification aléatoire (intervalle 40 : la pointe née d'une pointe qui longe n'a pas de mémoire),
/// sur six dispositifs (mur fini, coin en L, piliers, salle à porte, mur jusqu'au bord de la boîte, blocs qui se touchent
/// par les coins), dont trois réglages avec des sources (la longueur du pas change d'une case à l'autre), l'angle nul et le
/// chimiotropisme : 324 colonies (9 réglages, 6 graines chacun) suivent le modèle pendant 120 pas, à l'octet près, et ne
/// laissent aucun segment dans un obstacle. Garde contre un test vide : beaucoup de segments de glissement (axiaux) et de naissances.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn avec_la_marche_au_hasard_la_simulation_suit_le_modele() {
    outils::delai();
    let (mut memoires, mut naissances) = (0usize, 0u64);
    let graines = [0u64, 1, 2, 3, 4, 5];
    for (nom, murs_propres) in dispositifs() {
        let mut murs = murs_propres.clone();
        murs.push((0, 0, 1, 59));
        murs.push((0, 0, 59, 1));
        let obstacles: Vec<_> = murs.iter().map(|&(a, b, c, d)| outils::obstacle(a, b, c, d)).collect();
        // Une zone pauvre sur le haut des murs et une tache riche : la longueur du pas change le long des murs, et le
        // gradient fait tourner les pointes (chimiotropisme).
        let zones = vec![(31, 15, 9, 0.03), (20, 44, 6, 3.0)];
        for (amplitude, pas, affinite, a, intervalle, sources, chimiotropisme) in [
            (15, 1.0, 0.0, 35.0, f64::INFINITY, vec![], 0.0),
            (40, 1.0, 0.25, 35.0, 40.0, vec![], 0.0),
            (15, 0.7, 0.25, 60.0, f64::INFINITY, vec![], 0.0),
            (40, 0.7, 0.0, 60.0, 40.0, vec![], 0.0),
            (15, 1.0, 0.25, 60.0, 40.0, vec![], 0.0),
            (40, 0.7, 0.25, 35.0, f64::INFINITY, vec![], 0.0),
            (20, 1.0, 1.0, 0.0, f64::INFINITY, zones.clone(), 0.0),
            (20, 1.0, 1.0, 35.0, 40.0, zones.clone(), 3.0),
            (20, 1.5, 0.5, 60.0, f64::INFINITY, zones.clone(), 0.0),
        ] {
            let salle = Salle {
                largeur: 60,
                hauteur: 60,
                spore: (12, 30),
                murs: murs.clone(),
                pas,
                affinite,
                amplitude,
                intervalle,
                thigmotropisme: a,
                sources: sources.clone(),
                chimiotropisme,
            };
            for &graine in &graines {
                let contexte = format!(
                    "{nom}, amplitude {amplitude}, pas {pas}, affinité {affinite}, angle {a}, intervalle {intervalle}, sources {sources:?}, chimiotropisme {chimiotropisme}"
                );
                let (simulation, modele) = suivre_le_modele(&salle, graine, 120, &contexte);
                naissances += modele.ramifications;
                memoires += simulation.segments().iter().filter(|s| s.x0 == s.x1 || s.y0 == s.y1).count();
                if let Some(faute) = premiere_faute(simulation.segments(), &obstacles) {
                    panic!("{contexte}, graine {graine} : {faute}");
                }
            }
        }
    }
    assert!(memoires > 10_000, "garde : seulement {memoires} segments axiaux (glissements)");
    assert!(naissances > 1_000, "garde : {naissances} naissances");
}

/// Après la fin d'un mur fini, la pointe retrouve son cap à 20° près, au plus un pas après le bout du mur. Dispositif : un
/// mur (colonnes 30 à 33, lignes 20 à 60), une pointe de direction 60° sans marche au hasard (150 graines qui donnent cette
/// direction) : elle touche la face gauche sous 30°, longe vers le bas (90°), et au bout du mur (la face basse est la ligne
/// `y = 61`) repart dans la direction `60 + δ`. Vérifié ici à la main, sans le modèle : l'approche droite, le glissement
/// vertical, puis le premier segment non vertical, dont la direction est à au plus 20° de 60, qui part d'un point à
/// `y ≥ 60` (moins d'une case avant le bout) alors que le glissement précédent partait d'un point à `y < 61` ; ensuite, la
/// pointe va droit. Sur les 150 graines, `δ` prend au moins 30 valeurs différentes, atteint ±20, et sa moyenne est à moins de
/// 4 de zéro (écart-type de la moyenne : 1,0).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn apres_la_fin_du_mur_la_pointe_retrouve_son_cap_a_20_degres_pres() {
    outils::delai();
    let mut salle = Cote::Droite.salle(1.0, 0.0, 35.0);
    salle.largeur = 60;
    salle.spore = (20, 22);
    salle.murs = vec![(30, 20, 33, 60), (0, 0, 1, 79), (0, 0, 59, 1)];
    let mut ecarts = Vec::new();
    for graine in graines_pour(60, 150) {
        let (simulation, _) = suivre_le_modele(&salle, graine, 45, "mur fini, direction 60");
        let segments = segments_de(simulation.segments(), 0);
        let directions: Vec<usize> = segments.iter().map(direction_du_segment).collect();
        let premier_vertical = directions.iter().position(|&d| d == 90).expect("un glissement vertical");
        assert!(directions[..premier_vertical].iter().all(|&d| d == 60), "graine {graine} : approche droite à 60°");
        let retour = premier_vertical
            + directions[premier_vertical..].iter().position(|&d| d != 90).expect("un retour après le glissement");
        assert!(retour - premier_vertical >= 15, "graine {graine} : le glissement longe le mur");
        let cap = directions[retour];
        assert!(
            ecart_circulaire(cap, 60) <= 20,
            "graine {graine} : direction retrouvée {cap}, à plus de 20° du cap 60"
        );
        assert!(
            segments[retour].y0 >= 60.0,
            "graine {graine} : retour à y = {} avant le bout du mur",
            segments[retour].y0
        );
        assert!(segments[retour - 1].y0 < 61.0, "graine {graine} : le retour tarde après le bout du mur");
        assert!(
            directions[retour..].iter().all(|&d| d == cap),
            "graine {graine} : après le retour, la pointe va droit"
        );
        ecarts.push(cap as i64 - 60);
    }
    let mut distincts = ecarts.clone();
    distincts.sort_unstable();
    distincts.dedup();
    assert!(distincts.len() >= 30, "seulement {} écarts différents : {distincts:?}", distincts.len());
    assert!(distincts.contains(&20) && distincts.contains(&-20), "les écarts ±20 sont atteints : {distincts:?}");
    let moyenne = ecarts.iter().sum::<i64>() as f64 / ecarts.len() as f64;
    assert!(moyenne.abs() < 4.0, "écart moyen {moyenne}");
}

/// Au bord de la boîte : une pointe qui longe un mur jusqu'au bord y reste, encore active, sans naissance, et **oublie** sa
/// mémoire (drapeau nul) ; elle garde la direction de glissement. Mur colonnes 30 à 33, lignes 20 à 59 (jusqu'au bord bas
/// de la boîte, `y = 60`), pointe de direction 60°.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn au_bord_de_la_boite_la_pointe_qui_longe_s_arrete_et_oublie() {
    outils::delai();
    let mut salle = Cote::Droite.salle(1.0, 0.0, 35.0);
    salle.largeur = 60;
    salle.hauteur = 60;
    salle.spore = (20, 30);
    salle.murs = vec![(30, 20, 33, 59), (0, 0, 1, 59), (0, 0, 59, 1)];
    for graine in graines_pour(60, 10) {
        let (simulation, _) = suivre_le_modele(&salle, graine, 60, "mur jusqu'au bord");
        let apex = apex_lus(&simulation);
        assert_eq!(
            (simulation.pointes(), simulation.ramifications()),
            (1, 0),
            "graine {graine} : une pointe, rien de né"
        );
        assert!(apex[0].actif && apex[0].y >= 59.0, "graine {graine} : la pointe est au bord, active : {:?}", apex[0]);
        assert_eq!(apex[0].memoire, Some((0, 0, 0)), "graine {graine} : la mémoire est oubliée au bord");
        assert_eq!(apex[0].direction, 90, "graine {graine} : elle garde la direction de glissement");
    }
}

/// Dans un coin concave, seule la branche qui a la place naît. Un mur vertical (colonnes 30 à 33, lignes 10 à 50) et un sol
/// (lignes 50 à 53, colonnes 16 à 33) : la pointe de direction 60° longe le mur vers le bas, et son pas de glissement touche
/// le sol (face horizontale) : elle s'arrête ; des deux directions parallèles au sol, 0° (vers le mur) n'a pas un pas plein
/// libre, 180° l'a : **une** pointe naît, sans mémoire, qui s'éloigne le long du sol. Et une pointe de direction 0° (la
/// normale) qui frappe le mur juste au-dessus du sol : des deux directions 90° (vers le sol) et 270°, seule la seconde naît.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn dans_un_coin_concave_seule_la_branche_qui_a_la_place_nait() {
    outils::delai();
    let mut salle = Cote::Droite.salle(1.0, 0.0, 35.0);
    salle.largeur = 60;
    salle.hauteur = 60;
    salle.murs = vec![(30, 10, 33, 50), (16, 50, 33, 53), (0, 0, 1, 59), (0, 0, 59, 1)];
    // Glissement jusqu'au sol.
    salle.spore = (20, 30);
    for graine in graines_pour(60, 10) {
        let (simulation, _) = suivre_le_modele(&salle, graine, 35, "coin concave, glissement");
        let apex = apex_lus(&simulation);
        assert_eq!(simulation.ramifications(), 1, "graine {graine} : une seule naissance");
        assert!(!apex[0].actif, "graine {graine} : la pointe d'origine s'est arrêtée dans le coin");
        assert_eq!((apex[1].direction, apex[1].memoire), (180, Some((0, 0, 0))), "graine {graine} : la pointe née");
        assert!(!segments_de(simulation.segments(), 1).is_empty(), "graine {graine} : la pointe née avance");
    }
    // Frappe frontale juste au-dessus du sol : la spore en (20, 49), direction 0.
    salle.spore = (20, 49);
    for graine in graines_pour(0, 5) {
        let (simulation, _) = suivre_le_modele(&salle, graine, 20, "coin concave, frappe frontale");
        let apex = apex_lus(&simulation);
        assert_eq!(simulation.ramifications(), 1, "graine {graine} : une seule naissance");
        assert_eq!(apex[1].direction, 270, "graine {graine} : seule la branche vers le haut naît");
    }
}
