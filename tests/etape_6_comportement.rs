//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie comportement : ce que le chimiotropisme fait à la
//! colonie dans la simulation.
//!
//! Ils fixent : (a) une pointe seule (`amplitude_marche = 0`) lit le gradient de la case où elle est, après la diffusion du
//! pas, et sa première direction vaut `tourner_chimiotropique` appliqué à ce gradient avec les mêmes tirages que la
//! simulation (direction initiale, `delta`, arrondi), pour trois taches, avec et sans diffusion, et le premier segment part
//! dans cette direction ; la ramification du même tour part de la direction après le virage ; une pointe immobile (case
//! sans substrat) suit pas à pas, sur 3 000 pas, exactement le noyau « marche au hasard puis virage », au centre, au bord et
//! au coin de la boîte ; (b) plus la force augmente, plus la part de la biomasse tournée vers la tache augmente (forces 0, 1,
//! 2, 4, moyenne sur 100 graines, marge `MARGE` mesurée) ; sans gradient (champ uniforme sans consommation), la force ne
//! change rien à l'état, hors la queue canonique ; (c) la masse se conserve avec des obstacles et des taches quand la
//! force est positive (les trois contrôles de T0, rejoués avec une force 2 et une force 30), et aucun segment ne part d'une
//! case bloquée ; (d) la biomasse et le rayon restent finis, positifs et croissants, les segments restent dans la boîte,
//! jusqu'à la force 45 (la borne).
//!
//! Ils laissent ouverts : ce qu'une pointe fait en arrivant dans un obstacle (aucun test ne le fixe, comme à T0) ; la
//! forme exacte de la colonie à force positive (seules des lois en moyenne et des égalités exactes sont fixées) ; la
//! mesure « part de la biomasse tournée vers la tache » n'est pas une valeur publiée, c'est une comparaison entre forces.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs` et `sqrt`.

use mycelium_lab::alea::Alea;
use mycelium_lab::anastomose;
use mycelium_lab::croissance::{self, COS, NB_DIRECTIONS, SIN};
use mycelium_lab::simulation::{Milieu, Obstacle, Reglages, Simulation, Source};
use mycelium_lab::substrat::Substrat;
use mycelium_lab::tropisme::{Tropismes, tourner_chimiotropique};

/// Tolérance (relative) sur « substrat + coût × biomasse = constante » : celle de T0 (1e-10). Mesurée sur le prototype de T1,
/// avec les forces 0, 2 et 30 : l'écart relatif maximal est 5,9e-14 pour la masse totale (61 × 61, 60 pas, deux graines) et
/// 1,1e-15 pour la masse de chaque compartiment des murs (300 pas) ; une fuite à travers un obstacle donne plus de 1e-3.
const TOLERANCE_MASSE: f64 = 1e-10;

/// Marge (sur la part de biomasse tournée vers la tache) que chaque force doit gagner sur la précédente. Mesurée sur le
/// prototype : sur 30 blocs de 100 graines différentes, les gains moyens sont 0,127 (force 0 → 1, écart-type d'un bloc
/// à l'autre 0,019), 0,102 (1 → 2, 0,008) et 0,088 (2 → 4, 0,012) ; le plus petit gain observé sur un bloc est 0,065.
/// La marge 0,02 est à 5,7 écarts-types du plus faible gain moyen (0,088 − 0,02 = 5,8 × 0,012). Une force sans effet, ou
/// un virage dans le mauvais sens, donne un gain nul ou négatif. Rejoué par le juge sur 20 autres blocs de 100 graines
/// (1 000 · k + 1 ..= 1 000 · k + 100, k = 31 à 50) : gains moyens 0,131 (écart-type 0,013), 0,107 (0,008), 0,090 (0,009),
/// plus petit gain d'un bloc 0,074 ; force 4 contre 0 : 0,327 (0,012, au moins 0,306) ; part à force 0 : 0,439 (0,012).
/// La marge est donc à 7,8 écarts-types du plus faible gain moyen, le seuil 0,2 à 10.
const MARGE: f64 = 0.02;

fn source(x: usize, y: usize, rayon: usize, quantite: f64) -> Source {
    Source { x, y, rayon, quantite }
}

fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

/// Des tropismes de force `f`, sans littéral complet : T2 ajoutera un champ à `Tropismes`, et `..Tropismes::default()` garde
/// les tests valables. Tant qu'il n'y a qu'un champ, clippy trouve la mise à jour inutile (`needless_update`) : l'`allow`
/// tombera avec T2.
#[allow(clippy::needless_update)]
fn tropismes(f: f64) -> Tropismes {
    Tropismes { chimiotropisme: f, ..Tropismes::default() }
}

fn construire(r: Reglages, m: Milieu, f: f64, graine: u64) -> Simulation {
    Simulation::nouvelle_avec_tropismes(r, m, tropismes(f), graine).expect("réglages, milieu et tropismes valides")
}

/// Les réglages de base : substrat initial 1, diffusion 0,2, une ramification tous les 20 pas, sans anastomose.
fn reglages(largeur: usize, hauteur: usize, spore: (usize, usize)) -> Reglages {
    Reglages {
        largeur,
        hauteur,
        spore,
        substrat_initial: 1.0,
        diffusion: 0.2,
        croissance: croissance::Parametres { pas: 1.0, intervalle: 20.0, angle_ramification: 30, amplitude_marche: 20 },
        affinite: 0.1,
        cout: 0.002,
        anastomose: None,
    }
}

/// Une colonie qui ne se ramifie pas : une seule pointe.
fn reglages_sans_ramification(largeur: usize, hauteur: usize, spore: (usize, usize)) -> Reglages {
    let mut r = reglages(largeur, hauteur, spore);
    r.croissance.intervalle = f64::INFINITY;
    r
}

/// Un apex lu dans l'écriture canonique : `(id, x, y, direction, actif)`. Après le temps (4 octets), le nombre de
/// segments (8) et les segments (40 chacun), chaque apex tient 25 octets : `id` (`u32`), `x` et `y` (`f64`), `direction`
/// (`u32`), `actif` (`u8`).
fn apex_canonique(octets: &[u8], nb_segments: usize, k: usize) -> (u32, f64, f64, u32, bool) {
    let debut = 12 + 40 * nb_segments + 25 * k;
    let entier = |i: usize| u32::from_le_bytes(octets[debut + i..debut + i + 4].try_into().unwrap());
    let reel = |i: usize| f64::from_bits(u64::from_le_bytes(octets[debut + i..debut + i + 8].try_into().unwrap()));
    (entier(0), reel(4), reel(12), entier(20), octets[debut + 24] == 1)
}

/// La direction du premier apex (le seul, si la colonie ne se ramifie pas).
fn direction_du_premier_apex(simulation: &Simulation) -> usize {
    apex_canonique(&simulation.ecrire_canonique(), simulation.segments().len(), 0).3 as usize
}

/// Le gradient de la case `(x, y)` d'un substrat sans obstacle, écrit ici d'après la spécification (différences
/// centrées, voisin hors de la grille remplacé par la case), sans passer par `Substrat::gradient`.
fn gradient_de_reference(s: &Substrat, x: usize, y: usize) -> (f64, f64) {
    let c = s.valeur(x, y);
    let lire = |i: i64, j: i64| {
        if i < 0 || j < 0 || i >= s.largeur() as i64 || j >= s.hauteur() as i64 {
            c
        } else {
            s.valeur(i as usize, j as usize)
        }
    };
    let (x, y) = (x as i64, y as i64);
    ((lire(x + 1, y) - lire(x - 1, y)) * 0.5, (lire(x, y + 1) - lire(x, y - 1)) * 0.5)
}

// ---------------------------------------------------------------------------------------------------------------
// 6 (a). La pointe lit le gradient de sa case et tourne comme `tourner_chimiotropique`.
// ---------------------------------------------------------------------------------------------------------------

/// Les réglages d'une pointe seule qui va tout droit (`amplitude_marche = 0`), dans une boîte de 21 × 21, spore en
/// `(10, 10)`, substrat initial 1.
fn reglages_pointe_seule(diffusion: f64, intervalle: f64) -> Reglages {
    let mut r = reglages(21, 21, (10, 10));
    r.diffusion = diffusion;
    r.croissance.intervalle = intervalle;
    r.croissance.amplitude_marche = 0;
    r
}

/// Trois milieux qui donnent à la case de la spore un gradient non nul : une tache riche au sud-est (gradient `(1, 1)`
/// sans diffusion), une tache pauvre à l'ouest (le gradient pointe en sens inverse d'elle : `(0,35, 0,35)`), un seul
/// voisin un peu plus riche (gradient `(0,01, 0)`, `s = 0,5`).
fn milieux_a_gradient() -> [(&'static str, Milieu); 3] {
    [
        ("tache riche au sud-est", Milieu { sources: vec![source(12, 12, 3, 3.0)], obstacles: vec![] }),
        ("tache pauvre à l'ouest", Milieu { sources: vec![source(7, 9, 3, 0.3)], obstacles: vec![] }),
        ("un voisin un peu plus riche", Milieu { sources: vec![source(11, 10, 0, 1.02)], obstacles: vec![] }),
    ]
}

/// Le gradient de la case de la spore au premier pas, et le substrat qu'elle y lira, calculés hors de la simulation : on
/// refait le champ du temps 0 dans un substrat à part, on lui fait faire le pas de diffusion que la simulation fait en
/// premier, et on lit la case `(10, 10)`.
fn gradient_et_substrat_au_premier_pas(simulation: &Simulation, diffusion: f64) -> ((f64, f64), f64) {
    let mut copie = Substrat::nouveau(21, 21, diffusion, 1.0, 1.0).expect("substrat valide");
    for y in 0..21 {
        for x in 0..21 {
            copie.deposer(x, y, simulation.substrat().valeur(x, y));
        }
    }
    copie.pas();
    (gradient_de_reference(&copie, 10, 10), copie.valeur(10, 10))
}

/// 6 (a). Sa première direction vaut `tourner_chimiotropique` appliqué au gradient de sa case, avec les tirages de la
///    simulation : le test rejoue l'ordre des tirages (direction initiale, `delta` — un tirage même à amplitude 0 —,
///    arrondi). Le premier segment part dans cette direction : `(x, y) + longueur · (COS, SIN)`. Trois taches, avec
///    diffusion (le gradient est lu **après** le pas de diffusion) et sans, cinq forces, trente graines. Sans
///    chimiotropisme, la direction reste la direction initiale.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_premiere_direction_est_celle_de_tourner_chimiotropique_dans_le_gradient_de_la_case() {
    let (mut cas, mut changes) = (0, 0);
    for diffusion in [0.0, 0.2] {
        for (nom, m) in milieux_a_gradient() {
            for force in [0.5, 1.0, 3.0, 10.0, 45.0] {
                for graine in 0..30u64 {
                    let r = reglages_pointe_seule(diffusion, f64::INFINITY);
                    let mut simulation = construire(r.clone(), m.clone(), force, graine);
                    let (gradient, s) = gradient_et_substrat_au_premier_pas(&simulation, diffusion);
                    assert!(gradient != (0.0, 0.0), "{nom} : garde, le gradient de la case de la spore est nul");

                    // Le rejeu : direction initiale, `delta` (amplitude 0 : toujours 0, mais un tirage), puis le virage.
                    let mut alea = Alea::new(graine);
                    let initiale = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
                    let _delta = alea.suivant();
                    let attendue = tourner_chimiotropique(initiale, gradient, force, &mut alea);

                    simulation.avancer();
                    let obtenue = direction_du_premier_apex(&simulation);
                    assert_eq!(
                        obtenue, attendue,
                        "diffusion {diffusion}, {nom}, force {force}, graine {graine} : direction {obtenue}, attendue {attendue} (initiale {initiale}, gradient {gradient:?})"
                    );
                    // Le premier segment part dans la direction du virage : longueur pas · s / (affinité + s).
                    assert_eq!(simulation.segments().len(), 1, "un segment au premier pas");
                    let longueur = (1.0 * s) / (0.1 + s);
                    let segment = simulation.segments()[0];
                    assert!(
                        (segment.x1 - (10.5 + longueur * COS[attendue])).abs() <= 1e-12
                            && (segment.y1 - (10.5 + longueur * SIN[attendue])).abs() <= 1e-12,
                        "diffusion {diffusion}, {nom}, force {force}, graine {graine} : le segment ne part pas dans la direction {attendue}"
                    );

                    // Sans chimiotropisme, la direction est l'initiale (amplitude 0).
                    let mut sans = construire(r, m.clone(), 0.0, graine);
                    sans.avancer();
                    assert_eq!(direction_du_premier_apex(&sans), initiale, "force 0 : la direction a changé");
                    cas += 1;
                    changes += usize::from(attendue != initiale);
                }
            }
        }
    }
    // Garde contre un test vide : le virage change la direction dans une bonne part des cas.
    assert_eq!(cas, 2 * 3 * 5 * 30);
    assert!(changes * 2 >= cas, "le virage n'a changé la direction que {changes} fois sur {cas}");
}

/// 6 (a, suite). La nouvelle direction sert aussi à la ramification du tour : le fils naît de `direction ± angle`, où
///    `direction` est celle d'**après** le virage. Avec `intervalle = 1` la pointe se ramifie dès le premier pas (tirage
///    de ramification, puis de côté) ; le test rejoue les trois premiers tirages (direction initiale, `delta`, arrondi),
///    lit l'apex 0 (le père : sa direction est celle d'après le virage) et l'apex 1 (le fils : `direction ± 30`, l'un ou
///    l'autre côté) dans l'écriture canonique.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_ramification_part_de_la_direction_apres_le_virage() {
    let (mut cas, mut differents, mut cotes_vus) = (0, 0, [0usize; 2]);
    for (nom, m) in milieux_a_gradient() {
        for force in [0.0, 1.0, 4.0, 45.0] {
            for graine in 0..40u64 {
                let r = reglages_pointe_seule(0.0, 1.0);
                let mut simulation = construire(r, m.clone(), force, graine);
                let gradient = gradient_de_reference(simulation.substrat(), 10, 10);

                let mut alea = Alea::new(graine);
                let initiale = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
                let _delta = alea.suivant();
                let apres_virage = tourner_chimiotropique(initiale, gradient, force, &mut alea);
                // Le fils naît à `direction ± 30` (angle de ramification) : les deux côtés sont permis.
                let fils_possibles = [(apres_virage + 30) % 360, (apres_virage + 330) % 360];

                simulation.avancer();
                assert_eq!(simulation.ramifications(), 1, "{nom}, force {force}, graine {graine} : une ramification");
                let octets = simulation.ecrire_canonique();
                let nb_segments = simulation.segments().len();
                let pere = apex_canonique(&octets, nb_segments, 0);
                let enfant = apex_canonique(&octets, nb_segments, 1);
                assert_eq!(pere.3 as usize, apres_virage, "{nom}, force {force}, graine {graine} : direction du père");
                assert_eq!(enfant.0, 1);
                assert!(
                    fils_possibles.contains(&(enfant.3 as usize)),
                    "{nom}, force {force}, graine {graine} : direction du fils {}, attendue {fils_possibles:?} (père {apres_virage}, direction d'avant le virage {initiale})",
                    enfant.3
                );
                cotes_vus[usize::from(enfant.3 as usize == fils_possibles[1])] += 1;
                cas += 1;
                differents += usize::from(apres_virage != initiale);
            }
        }
    }
    // Garde contre un test vide : à force positive, le virage a changé la direction dans une bonne part des cas.
    assert_eq!(cas, 3 * 4 * 40);
    assert!(differents >= cas / 3, "le virage n'a changé la direction que {differents} fois sur {cas}");
    assert!(cotes_vus[0] >= 100 && cotes_vus[1] >= 100, "les deux côtés de la ramification : {cotes_vus:?}");
}

/// Réglages et milieu d'une pointe **immobile** : le substrat est nul partout (diffusion nulle) sauf sur les voisins de la
/// spore, que le milieu fait valoir `0,004` (gauche), `0,02` (droite), `0,015` (haut) et `0,002` (bas) quand ils existent ;
/// la case de la spore vaut `0` (une pointe sans substrat ne s'allonge pas : elle garde sa position et tourne). Le
/// gradient de la case est donc constant, par exemple `(0,008, −0,0065)` au centre (`s ≈ 0,51`).
fn pointe_immobile(spore: (usize, usize)) -> (Reglages, Milieu) {
    let mut r = reglages_sans_ramification(9, 9, spore);
    r.substrat_initial = 0.0;
    r.diffusion = 0.0;
    r.cout = 0.0;
    let mut sources = vec![source(spore.0, spore.1, 0, 0.0)];
    for (dx, dy, quantite) in [(-1i64, 0i64, 0.004), (1, 0, 0.02), (0, -1, 0.015), (0, 1, 0.002)] {
        let (x, y) = (spore.0 as i64 + dx, spore.1 as i64 + dy);
        if (0..9).contains(&x) && (0..9).contains(&y) {
            sources.push(source(x as usize, y as usize, 0, quantite));
        }
    }
    (r, Milieu { sources, obstacles: vec![] })
}

/// 6 (a, suite). Une pointe immobile suit, pas après pas, exactement le noyau « marche au hasard puis virage » : le test
///    rejoue la chaîne avec le même générateur et compare la direction après chacun des 3 000 pas, pour quatre forces,
///    trois graines et quatre places de spore (centre, bord gauche, bord haut, coin bas-droit) : le gradient est lu dans
///    la case de la pointe, bords compris (le voisin absent est la case elle-même).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn une_pointe_immobile_suit_exactement_le_noyau_marche_puis_virage() {
    for spore in [(4, 4), (0, 4), (4, 0), (8, 8)] {
        let (r, m) = pointe_immobile(spore);
        let amplitude = u64::from(r.croissance.amplitude_marche);
        for force in [1.0, 2.0, 4.0, 45.0] {
            for graine in [1u64, 2, 3] {
                let mut simulation = construire(r.clone(), m.clone(), force, graine);
                let gradient = gradient_de_reference(simulation.substrat(), spore.0, spore.1);
                assert!(gradient != (0.0, 0.0), "spore {spore:?} : garde, gradient nul");
                let mut alea = Alea::new(graine);
                let mut direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
                for pas in 1..=3000 {
                    let delta = (alea.suivant() % (2 * amplitude + 1)) as i64 - amplitude as i64;
                    direction = (direction as i64 + delta).rem_euclid(NB_DIRECTIONS as i64) as usize;
                    direction = tourner_chimiotropique(direction, gradient, force, &mut alea);
                    simulation.avancer();
                    assert_eq!(
                        direction_du_premier_apex(&simulation),
                        direction,
                        "spore {spore:?}, force {force}, graine {graine}, pas {pas}"
                    );
                }
                // Garde : la pointe n'a pas bougé, aucun segment n'est né, donc le gradient n'a pas changé.
                assert!(simulation.segments().is_empty(), "spore {spore:?} : la pointe a avancé");
            }
        }
    }
}

/// 6 (a, suite). Une pointe qui avance lit, à chaque pas, le gradient de la case **où elle est** (pas celle de la spore,
///    pas une case voisine), dans le substrat de ce pas après sa diffusion : le test rejoue 60 pas d'une pointe seule
///    (`amplitude_marche = 0`, sans ramification) près d'une tache, en recalculant à chaque pas le gradient de sa case sur
///    une copie du substrat qui fait le pas de diffusion, puis `delta` (un tirage) et `tourner_chimiotropique`. Ajouté par
///    le juge : sans lui, une main qui lit le gradient à la case de la spore n'était attrapée que par hasard.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn une_pointe_qui_avance_lit_le_gradient_de_sa_case_a_chaque_pas() {
    let milieu = Milieu { sources: vec![source(15, 13, 3, 3.0)], obstacles: vec![] };
    let mut cases_vues = Vec::new();
    for force in [3.0, 20.0] {
        for graine in 0..10u64 {
            let r = reglages_pointe_seule(0.2, f64::INFINITY);
            let mut simulation = construire(r, milieu.clone(), force, graine);
            let mut alea = Alea::new(graine);
            let mut direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
            for pas in 1..=60 {
                let octets = simulation.ecrire_canonique();
                let (_, x, y, _, _) = apex_canonique(&octets, simulation.segments().len(), 0);
                let (i, j) = (x as usize, y as usize);
                if !cases_vues.contains(&(i, j)) {
                    cases_vues.push((i, j));
                }
                let mut copie = simulation.substrat().clone();
                copie.pas();
                let gradient = gradient_de_reference(&copie, i, j);
                let _delta = alea.suivant();
                direction = tourner_chimiotropique(direction, gradient, force, &mut alea);
                simulation.avancer();
                let obtenue = direction_du_premier_apex(&simulation);
                assert_eq!(
                    obtenue, direction,
                    "force {force}, graine {graine}, pas {pas} : direction {obtenue}, attendue {direction} (pointe dans la case ({i}, {j}), gradient {gradient:?})"
                );
            }
        }
    }
    // Garde contre un test vide : les pointes ont traversé beaucoup de cases.
    assert!(cases_vues.len() >= 30, "les pointes n'ont visité que {} cases", cases_vues.len());
}

// ---------------------------------------------------------------------------------------------------------------
// 6 (b). La colonie va vers la tache.
// ---------------------------------------------------------------------------------------------------------------

/// Les réglages de la colonie du test de tendance : une boîte de 25 × 17, la spore en `(6, 8)`, un pas court (0,1) pour
/// que la colonie reste dans la zone du gradient pendant les 150 pas, une ramification tous les 30 pas, et une tache riche
/// (4, contre 0,5 de fond) de rayon 7 centrée en `(17, 8)`, à l'est de la spore. Ce choix vient de mesures sur le prototype :
/// avec un pas de 1, une boîte large et trente à soixante pas, la force 1 ne gagne que 0,005 à 0,02 de part sur la force 0
/// (écart-type de la moyenne de 300 graines : 0,015), donc rien de fiable ; le virage ne vaut que `force · s` degrés par pas
/// contre une marche de ±20°, il faut donc des pas courts et un gradient présent sur toute la durée.
fn colonie_et_tache() -> (Reglages, Milieu) {
    let mut r = reglages(25, 17, (6, 8));
    r.substrat_initial = 0.5;
    r.croissance = croissance::Parametres { pas: 0.1, intervalle: 30.0, angle_ramification: 30, amplitude_marche: 20 };
    (r, Milieu { sources: vec![source(17, 8, 7, 4.0)], obstacles: vec![] })
}

/// La part de la biomasse tournée vers `centre` : la somme des longueurs des segments dont la direction fait un produit
/// scalaire positif avec le vecteur qui va du milieu du segment au centre, sur la biomasse totale.
fn part_vers_la_tache(simulation: &Simulation, centre: (f64, f64)) -> f64 {
    let (mut total, mut vers) = (0.0, 0.0);
    for segment in simulation.segments() {
        let longueur = segment.longueur();
        let (dx, dy) = (segment.x1 - segment.x0, segment.y1 - segment.y0);
        let (mx, my) = ((segment.x0 + segment.x1) * 0.5, (segment.y0 + segment.y1) * 0.5);
        total += longueur;
        if dx * (centre.0 - mx) + dy * (centre.1 - my) > 0.0 {
            vers += longueur;
        }
    }
    assert!(total > 0.0, "la colonie n'a pas poussé");
    vers / total
}

/// 6 (b). Plus la force augmente, plus la part de la biomasse tournée vers la tache augmente : forces 0, 1, 2 et 4, 150
///    pas, moyenne sur les graines 1 à 100 ; chaque force gagne au moins `MARGE` sur la précédente, et la force 4 gagne
///    au moins 0,2 sur la force 0 (mesuré : 0,32, écart-type 0,016). À force 0, la colonie est celle de T0 : environ 0,44.
///    La colonie change bien avec la force (garde).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn plus_la_force_est_grande_plus_la_biomasse_va_vers_la_tache() {
    let (r, m) = colonie_et_tache();
    let centre = (17.5, 8.5);
    let forces = [0.0, 1.0, 2.0, 4.0];
    let mut moyennes = Vec::new();
    let mut premieres_colonies = Vec::new();
    for force in forces {
        let mut somme = 0.0;
        for graine in 1..=100u64 {
            let mut simulation = construire(r.clone(), m.clone(), force, graine);
            simulation.avancer_n(150);
            somme += part_vers_la_tache(&simulation, centre);
            if graine == 1 {
                premieres_colonies.push(simulation.segments().to_vec());
            }
        }
        moyennes.push(somme / 100.0);
    }
    for k in 1..forces.len() {
        assert!(
            moyennes[k] >= moyennes[k - 1] + MARGE,
            "force {} : part {} ; force {} : part {} ; gain inférieur à {MARGE}",
            forces[k],
            moyennes[k],
            forces[k - 1],
            moyennes[k - 1]
        );
    }
    assert!(moyennes[3] >= moyennes[0] + 0.2, "force 4 contre force 0 : {} contre {}", moyennes[3], moyennes[0]);
    assert!(
        moyennes[0] > 0.3 && moyennes[0] < 0.55,
        "part à force 0 : {} (la colonie de T0 devrait être près de 0,44)",
        moyennes[0]
    );
    // Garde : avec un gradient, la colonie de la graine 1 n'est pas la même avec et sans force.
    for k in 1..forces.len() {
        assert_ne!(premieres_colonies[k], premieres_colonies[0], "force {} : la colonie n'a pas changé", forces[k]);
    }
}

/// 6 (b, suite). Sans gradient, la force ne change rien : dans un champ exactement uniforme (substrat initial 1, sans
///    consommation : `cout = 0`), le gradient de toute case est `(0.0, 0.0)`, la pointe ne tire aucun nombre de plus, et la
///    colonie est la même qu'à force nulle, à l'octet près, hors la queue canonique (24 octets, milieu vide). Des
///    ramifications et une anastomose sont en jeu ; avec une tache, la même force change bien la colonie (garde).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn sans_gradient_la_force_ne_change_rien_que_la_queue() {
    let mut r = reglages(41, 41, (20, 20));
    r.cout = 0.0;
    r.croissance.intervalle = 8.0;
    r.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 5 });
    let mut segments_en_tout = 0;
    for graine in [1u64, 2, 3, 4, 5] {
        let mut a_zero = construire(r.clone(), Milieu::default(), 0.0, graine);
        a_zero.avancer_n(80);
        let octets_0 = a_zero.ecrire_canonique();
        for force in [0.5, 3.0, 45.0] {
            let mut simulation = construire(r.clone(), Milieu::default(), force, graine);
            simulation.avancer_n(80);
            let octets = simulation.ecrire_canonique();
            assert_eq!(octets.len(), octets_0.len() + 24, "graine {graine}, force {force} : longueur");
            assert!(
                octets[..octets_0.len()] == octets_0[..],
                "graine {graine}, force {force} : l'état diffère de celui de la force nulle dans un champ uniforme"
            );
            assert_eq!(simulation.segments(), a_zero.segments());
        }
        // Garde : le champ est resté uniforme (aucune consommation) et la colonie a poussé.
        assert_eq!(a_zero.substrat().masse_totale(), 41.0 * 41.0);
        segments_en_tout += a_zero.segments().len();
        assert!(a_zero.ramifications() >= 3, "graine {graine} : ramifications {}", a_zero.ramifications());
    }
    assert!(segments_en_tout >= 500, "les colonies n'ont presque pas poussé : {segments_en_tout}");

    // Contre-épreuve : avec une tache (donc un gradient), la même force change la colonie.
    let milieu = Milieu { sources: vec![source(30, 20, 4, 3.0)], obstacles: vec![] };
    let mut sans = construire(r.clone(), milieu.clone(), 0.0, 1);
    let mut avec = construire(r, milieu, 3.0, 1);
    sans.avancer_n(80);
    avec.avancer_n(80);
    assert_ne!(sans.segments(), avec.segments(), "avec une tache, la force n'a rien changé");
}

// ---------------------------------------------------------------------------------------------------------------
// 6 (c). Les obstacles retiennent la masse, avec chimiotropisme.
// ---------------------------------------------------------------------------------------------------------------

fn dans_le_rectangle(i: usize, j: usize, o: &Obstacle) -> bool {
    o.x0 <= i && i <= o.x1 && o.y0 <= j && j <= o.y1
}

/// 6 (c). La masse se conserve avec des obstacles et des taches, une colonie qui se ramifie et s'anastomose, à force 2 et
///    à force 30 : à chaque pas, substrat + coût × biomasse reste égal à la masse initiale (`TOLERANCE_MASSE`), aucune case
///    n'est négative, les cases d'obstacle valent `0.0`. C'est le contrôle de T0, rejoué.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_masse_se_conserve_avec_des_obstacles_et_des_taches_a_force_positive() {
    let mut r = reglages(61, 61, (30, 30));
    r.substrat_initial = 0.05;
    r.cout = 0.01;
    r.croissance.intervalle = 10.0;
    r.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 10 });
    let m = Milieu {
        sources: vec![source(30, 30, 3, 0.4), source(45, 20, 4, 0.6)],
        obstacles: vec![obstacle(36, 10, 38, 40), obstacle(10, 45, 40, 46), obstacle(0, 0, 5, 5)],
    };
    for force in [2.0, 30.0] {
        for graine in [2u64, 9] {
            let mut simulation = construire(r.clone(), m.clone(), force, graine);
            let initiale = simulation.substrat().masse_totale();
            for pas in 1..=60 {
                simulation.avancer();
                let totale = simulation.substrat().masse_totale() + r.cout * simulation.biomasse();
                assert!(
                    ((totale - initiale) / initiale).abs() <= TOLERANCE_MASSE,
                    "force {force}, graine {graine}, pas {pas} : substrat + coût × biomasse = {totale} au lieu de {initiale}"
                );
            }
            for j in 0..61 {
                for i in 0..61 {
                    let v = simulation.substrat().valeur(i, j);
                    assert!(v >= 0.0, "force {force}, graine {graine} : case ({i}, {j}) négative : {v}");
                    if m.obstacles.iter().any(|o| dans_le_rectangle(i, j, o)) {
                        assert_eq!(v, 0.0, "force {force}, graine {graine} : la case d'obstacle ({i}, {j}) vaut {v}");
                    }
                }
            }
            // Garde contre un test vide. Le juge a mesuré la biomasse après 60 pas sur 40 graines : de 5,8 à 1 163 (force
            // 2) et de 5,8 à 718 (force 30) ; l'ancienne garde (50) était fausse pour une part des graines, d'où 2.
            assert!(
                simulation.biomasse() >= 2.0,
                "force {force}, graine {graine} : biomasse {}",
                simulation.biomasse()
            );
        }
    }
}

/// Les réglages des tests de mur : une seule pointe qui marche, du substrat seulement dans les taches.
fn reglages_de_mur() -> Reglages {
    let mut r = reglages_sans_ramification(41, 25, (12, 12));
    r.substrat_initial = 0.0;
    r.cout = 0.01;
    r
}

/// La somme du substrat des colonnes `x_min..=x_max`.
fn masse_des_colonnes(simulation: &Simulation, x_min: usize, x_max: usize) -> f64 {
    let mut somme = 0.0;
    for j in 0..25 {
        for i in x_min..=x_max {
            somme += simulation.substrat().valeur(i, j);
        }
    }
    somme
}

/// 6 (c, suite). Un mur qui traverse toute la boîte sépare deux compartiments à force positive : la masse de chacun se
///    conserve séparément (à gauche, avec le coût de la biomasse), et un compartiment initialement vide reste exactement à
///    `0.0` après 300 pas, le mur aussi. Un mur percé d'une porte laisse passer la diffusion, la masse totale restant
///    conservée.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_mur_separe_deux_compartiments_a_force_positive() {
    let r = reglages_de_mur();
    let mur = obstacle(20, 0, 21, 24);
    for force in [2.0, 30.0] {
        for droite_vide in [true, false] {
            let mut sources = vec![source(12, 12, 5, 1.0)];
            if !droite_vide {
                sources.push(source(32, 12, 5, 2.0));
            }
            let m = Milieu { sources, obstacles: vec![mur.clone()] };
            for graine in [3u64, 4] {
                let mut simulation = construire(r.clone(), m.clone(), force, graine);
                let (gauche0, droite0) =
                    (masse_des_colonnes(&simulation, 0, 19), masse_des_colonnes(&simulation, 22, 40));
                assert!(gauche0 > 50.0);
                simulation.avancer_n(300);
                let gauche = masse_des_colonnes(&simulation, 0, 19) + r.cout * simulation.biomasse();
                let droite = masse_des_colonnes(&simulation, 22, 40);
                let nom = format!("force {force}, graine {graine}, droite vide au départ : {droite_vide}");
                assert!(
                    ((gauche - gauche0) / gauche0).abs() <= TOLERANCE_MASSE,
                    "{nom} : masse à gauche {gauche} au lieu de {gauche0}"
                );
                if droite_vide {
                    assert_eq!(droite0, 0.0);
                    for j in 0..25 {
                        for i in 20..41 {
                            assert_eq!(
                                simulation.substrat().valeur(i, j),
                                0.0,
                                "{nom} : la case ({i}, {j}) derrière le mur"
                            );
                        }
                    }
                } else {
                    assert!(
                        ((droite - droite0) / droite0).abs() <= TOLERANCE_MASSE,
                        "{nom} : masse à droite {droite} au lieu de {droite0}"
                    );
                }
                // Garde contre un test vide : la colonie a poussé et la diffusion a atteint le mur.
                assert!(simulation.biomasse() >= 3.0, "{nom} : biomasse {}", simulation.biomasse());
                assert!(simulation.substrat().valeur(19, 12) > 1e-6, "{nom} : rien n'a diffusé jusqu'au mur");
            }
        }
        // Un mur percé d'une porte.
        let m = Milieu {
            sources: vec![source(12, 12, 5, 1.0)],
            obstacles: vec![obstacle(20, 0, 21, 9), obstacle(20, 12, 21, 24)],
        };
        let mut simulation = construire(r.clone(), m, force, 5);
        let initiale = simulation.substrat().masse_totale();
        assert_eq!(masse_des_colonnes(&simulation, 22, 40), 0.0);
        simulation.avancer_n(300);
        let passee = masse_des_colonnes(&simulation, 22, 40);
        assert!(passee > 1e-3 * initiale, "force {force} : la masse passée par la porte est {passee} sur {initiale}");
        let totale = simulation.substrat().masse_totale() + r.cout * simulation.biomasse();
        assert!(
            ((totale - initiale) / initiale).abs() <= TOLERANCE_MASSE,
            "force {force} : masse totale {totale} au lieu de {initiale}"
        );
    }
}

/// 6 (c, suite). Aucun segment ne part jamais d'une case bloquée, et la masse reste conservée, à force 4 et à force 45 :
///    un anneau d'obstacles enferme la spore dans un carré de 3 × 3 cases, soixante graines, cinquante pas. Une pointe qui
///    entre dans un obstacle y est arrêtée (case sans substrat, gradient nul), sans panique.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn aucun_segment_ne_part_d_une_case_bloquee_a_force_positive() {
    let r = reglages_sans_ramification(15, 15, (7, 7));
    let m = Milieu {
        sources: vec![],
        obstacles: vec![
            obstacle(5, 5, 9, 5),
            obstacle(5, 9, 9, 9),
            obstacle(5, 6, 5, 8),
            obstacle(9, 6, 9, 8),
            obstacle(0, 0, 14, 0),
        ],
    };
    for force in [4.0, 45.0] {
        let mut segments_en_tout = 0;
        for graine in 1..=60u64 {
            let mut simulation = construire(r.clone(), m.clone(), force, graine);
            let initiale = simulation.substrat().masse_totale();
            simulation.avancer_n(50);
            segments_en_tout += simulation.segments().len();
            for segment in simulation.segments() {
                let (i, j) = (segment.x0 as usize, segment.y0 as usize);
                assert!(
                    !simulation.substrat().est_bloquee(i, j),
                    "force {force}, graine {graine} : un segment part de la case bloquée ({i}, {j})"
                );
            }
            let totale = simulation.substrat().masse_totale() + r.cout * simulation.biomasse();
            assert!(
                ((totale - initiale) / initiale).abs() <= TOLERANCE_MASSE,
                "force {force}, graine {graine} : masse {totale} au lieu de {initiale}"
            );
        }
        assert!(
            segments_en_tout >= 60,
            "force {force} : les colonies n'ont presque pas poussé : {segments_en_tout} segments"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 6 (d). Les nombres restent finis et positifs.
// ---------------------------------------------------------------------------------------------------------------

/// 6 (d). Les rayons et la biomasse restent finis et positifs, la biomasse et le rayon ne décroissent jamais, les segments
///    restent finis et dans la boîte, le substrat reste fini et non négatif, pour des forces de 1 à 45 (la borne
///    supérieure : un virage de 45° au plus à chaque pas), avec une tache et un mur.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn les_rayons_et_la_biomasse_restent_finis_et_positifs() {
    let (largeur, hauteur) = (41usize, 31usize);
    let mut r = reglages(largeur, hauteur, (10, 15));
    r.croissance.intervalle = 10.0;
    r.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 8 });
    let m = Milieu { sources: vec![source(30, 15, 6, 3.0)], obstacles: vec![obstacle(20, 5, 21, 25)] };
    for force in [1.0, 4.0, 30.0, 45.0] {
        for graine in [1u64, 2, 3] {
            let mut simulation = construire(r.clone(), m.clone(), force, graine);
            let (mut biomasse_avant, mut rayon_avant) = (0.0, 0.0);
            for pas in 1..=100 {
                simulation.avancer();
                let (biomasse, rayon) = (simulation.biomasse(), simulation.rayon());
                assert!(
                    biomasse.is_finite() && rayon.is_finite(),
                    "force {force}, graine {graine}, pas {pas} : {biomasse}, {rayon}"
                );
                assert!(
                    biomasse >= biomasse_avant && rayon >= rayon_avant,
                    "force {force}, graine {graine}, pas {pas} : décroissance"
                );
                (biomasse_avant, rayon_avant) = (biomasse, rayon);
            }
            assert!(
                biomasse_avant > 0.0 && rayon_avant > 0.0,
                "force {force}, graine {graine} : biomasse {biomasse_avant}, rayon {rayon_avant}"
            );
            let diagonale = ((largeur * largeur + hauteur * hauteur) as f64).sqrt();
            assert!(rayon_avant < diagonale, "force {force}, graine {graine} : rayon {rayon_avant}");
            for releve in simulation.releves() {
                assert!(
                    releve.biomasse.is_finite()
                        && releve.biomasse >= 0.0
                        && releve.rayon.is_finite()
                        && releve.rayon >= 0.0
                        && releve.substrat.is_finite()
                        && releve.substrat >= 0.0,
                    "force {force}, graine {graine} : relevé {releve:?}"
                );
            }
            for segment in simulation.segments() {
                for (nom, v, borne) in [
                    ("x0", segment.x0, largeur),
                    ("x1", segment.x1, largeur),
                    ("y0", segment.y0, hauteur),
                    ("y1", segment.y1, hauteur),
                ] {
                    assert!(
                        v.is_finite() && v >= 0.0 && v < borne as f64,
                        "force {force}, graine {graine} : segment {nom} = {v}"
                    );
                }
            }
            for j in 0..hauteur {
                for i in 0..largeur {
                    let v = simulation.substrat().valeur(i, j);
                    assert!(v.is_finite() && v >= 0.0, "force {force}, graine {graine} : case ({i}, {j}) = {v}");
                }
            }
        }
    }
}
