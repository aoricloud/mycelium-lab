//! Tests d'acceptation de l'étape 7 (T2, le thigmotropisme), partie fonctions pures : `Face`, `angle_de_contact`,
//! `direction_de_glissement`, `branches_de_contact`, `ANGLE_CRITIQUE_MAXIMAL`, `MEMOIRE_DE_DIRECTION` et le champ
//! `Tropismes::thigmotropisme`.
//!
//! Ils fixent, sur **toutes les 360 directions** et les deux faces, des valeurs calculées ici par des méthodes
//! indépendantes des formules du code : l'angle de contact vaut l'écart angulaire entre la direction et la plus proche des
//! deux directions parallèles à la face (et son sinus, lu dans la table `SIN`, vaut la composante de la direction selon
//! la normale de la face, lue dans `COS` ou `SIN`) ; il va de 0 (parallèle) à 90 (perpendiculaire), ses valeurs aux
//! sommets sont fixées à la main (la direction 0 contre une face verticale : 90 ; la 90 : 0 ; etc.) ; la somme des angles
//! de contact aux deux faces vaut 90 ; la direction de glissement est la plus proche des deux parallèles, la plus
//! petite à égalité (la pointe perpendiculaire à la face) ; les branches de contact sont les deux directions parallèles à
//! la face, opposées, dans l'ordre croissant (90 puis 270 pour la face verticale, 0 puis 180 pour l'horizontale) ; `angle_de_contact` et `direction_de_glissement` paniquent pour
//! une direction de 360 ou plus ; `ANGLE_CRITIQUE_MAXIMAL` vaut 90 et `MEMOIRE_DE_DIRECTION` 20 ; `Tropismes::default()`
//! éteint le thigmotropisme (valeur `+0.0`, bits nuls), la mise à jour de structure
//! `Tropismes { chimiotropisme: f, ..Tropismes::default() }` des tests de T1 compile et garde l'angle nul. Les tables `COS` et `SIN` sont
//! exactes sur les axes (le glissement est ainsi exactement horizontal ou vertical).
//!
//! Ils laissent ouverts : le texte des messages de panique (la face d'un contact est fixée par `etape_7_substrat.rs`).
//!
//! Ils n'utilisent aucune fonction mathématique de `f64`.

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::croissance::{COS, NB_DIRECTIONS, SIN};
use mycelium_lab::tropisme::{
    ANGLE_CRITIQUE_MAXIMAL, Face, MEMOIRE_DE_DIRECTION, Tropismes, angle_de_contact, branches_de_contact,
    direction_de_glissement,
};

/// Tolérance (absolue) sur le sinus de l'angle de contact lu dans la table `SIN` : la table est arrondie au `f64` le plus
/// proche. Mesurée sur le prototype : l'écart maximal sur les 720 couples (direction, face) est 5,3e-16 ; un angle de
/// contact faux d'un degré donne un écart d'au moins 0,0087.
const TOLERANCE_SINUS: f64 = 1e-12;

/// Les deux directions parallèles à chaque face, écrites ici à la main.
fn paralleles(face: Face) -> (usize, usize) {
    match face {
        Face::Verticale => (90, 270),
        Face::Horizontale => (0, 180),
    }
}

/// L'écart angulaire entre deux directions, de 0 à 180.
fn ecart(a: usize, b: usize) -> usize {
    let e = a.abs_diff(b);
    e.min(360 - e)
}

const FACES: [Face; 2] = [Face::Verticale, Face::Horizontale];

/// Exige que `f` panique.
fn exiger_la_panique(nom: &str, f: impl FnOnce() -> u64) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok() {
        panic!("{nom} : l'appel aurait dû paniquer");
    }
}

/// Les tables sont exactes sur les axes : les tests de glissement comparent des segments « exactement verticaux » ou
/// « exactement horizontaux ». (Propriété de la table de la V0, gardée ici parce que T2 en dépend.)
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_tables_sont_exactes_sur_les_axes() {
    outils::delai();
    assert_eq!((COS[0], SIN[0]), (1.0, 0.0));
    assert_eq!((COS[90], SIN[90]), (0.0, 1.0));
    assert_eq!((COS[180], SIN[180]), (-1.0, 0.0));
    assert_eq!((COS[270], SIN[270]), (0.0, -1.0));
}

/// Les deux constantes publiées : l'angle critique maximal vaut 90 (la perpendiculaire) et la mémoire de direction 20
/// (« à environ 20° près », PNAS 116(27), 2019).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_constantes_valent_90_et_20() {
    outils::delai();
    assert_eq!(ANGLE_CRITIQUE_MAXIMAL, 90.0, "ANGLE_CRITIQUE_MAXIMAL : attendu 90");
    assert_eq!(MEMOIRE_DE_DIRECTION, 20, "MEMOIRE_DE_DIRECTION : attendu 20");
}

/// L'angle de contact pour les 360 directions et les deux faces est l'écart angulaire à la plus proche des deux directions
/// parallèles à la face.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn l_angle_de_contact_est_l_ecart_a_la_parallele_la_plus_proche_pour_les_360_directions() {
    outils::delai();
    for face in FACES {
        let (a, b) = paralleles(face);
        for d in 0..NB_DIRECTIONS {
            let attendu = ecart(d, a).min(ecart(d, b)) as u32;
            assert_eq!(angle_de_contact(d, face), attendu, "face {face:?}, direction {d}");
        }
    }
}

/// Le sinus de l'angle de contact (lu dans la table) vaut la composante de la direction selon la normale de la face : `|COS|`
/// pour une face verticale (la normale est l'axe x), `|SIN|` pour une face horizontale. Seconde méthode, par les tables, qui ne
/// partage rien avec la formule en entiers du code.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_sinus_de_l_angle_de_contact_est_la_composante_selon_la_normale() {
    outils::delai();
    let mut ecart_max: f64 = 0.0;
    for face in FACES {
        for d in 0..NB_DIRECTIONS {
            let alpha = angle_de_contact(d, face) as usize;
            assert!(alpha <= 90, "face {face:?}, direction {d} : angle {alpha} au-delà de 90");
            let normale = match face {
                Face::Verticale => COS[d].abs(),
                Face::Horizontale => SIN[d].abs(),
            };
            let e = (SIN[alpha] - normale).abs();
            ecart_max = ecart_max.max(e);
            assert!(
                e <= TOLERANCE_SINUS,
                "face {face:?}, direction {d} : sin({alpha}°) = {} au lieu de {normale}",
                SIN[alpha]
            );
        }
    }
    assert!(ecart_max <= TOLERANCE_SINUS);
}

/// Valeurs fixées à la main : aux axes et aux diagonales, contre les deux faces.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_angles_de_contact_aux_sommets() {
    outils::delai();
    // (direction, angle contre une face verticale, angle contre une face horizontale)
    let cas: [(usize, u32, u32); 13] = [
        (0, 90, 0),
        (90, 0, 90),
        (180, 90, 0),
        (270, 0, 90),
        (45, 45, 45),
        (135, 45, 45),
        (225, 45, 45),
        (315, 45, 45),
        (10, 80, 10),
        (80, 10, 80),
        (170, 80, 10),
        (350, 80, 10),
        (359, 89, 1),
    ];
    for (d, vertical, horizontal) in cas {
        assert_eq!(angle_de_contact(d, Face::Verticale), vertical, "direction {d}, face verticale");
        assert_eq!(angle_de_contact(d, Face::Horizontale), horizontal, "direction {d}, face horizontale");
    }
}

/// Les angles contre les deux faces s'ajoutent à 90 (une direction est d'autant plus parallèle à l'une qu'elle est moins
/// parallèle à l'autre) ; l'angle est le même pour `d` et `d + 180` (une droite n'a pas de sens) ; et la symétrie par rapport
/// à l'axe x (`d` devient `360 − d`) garde l'angle.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_angles_aux_deux_faces_s_ajoutent_a_90_et_ne_dependent_pas_du_sens() {
    outils::delai();
    for d in 0..NB_DIRECTIONS {
        let v = angle_de_contact(d, Face::Verticale);
        let h = angle_de_contact(d, Face::Horizontale);
        assert_eq!(v + h, 90, "direction {d}");
        let oppose = (d + 180) % 360;
        assert_eq!(angle_de_contact(oppose, Face::Verticale), v, "direction {d} et son opposée, face verticale");
        assert_eq!(angle_de_contact(oppose, Face::Horizontale), h, "direction {d} et son opposée, face horizontale");
        let miroir = (360 - d) % 360;
        assert_eq!(angle_de_contact(miroir, Face::Verticale), v, "direction {d} et son miroir, face verticale");
        assert_eq!(angle_de_contact(miroir, Face::Horizontale), h, "direction {d} et son miroir, face horizontale");
    }
}

/// La direction de glissement est, pour les 360 directions et les deux faces, la plus proche des deux directions parallèles ;
/// à égalité (la pointe perpendiculaire à la face), la plus petite. Elle est toujours l'une des deux branches, et son
/// écart à la direction de la pointe est l'angle de contact.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_direction_de_glissement_est_la_parallele_la_plus_proche_pour_les_360_directions() {
    outils::delai();
    for face in FACES {
        let (a, b) = paralleles(face);
        for d in 0..NB_DIRECTIONS {
            let attendu = if ecart(d, b) < ecart(d, a) { b } else { a };
            let obtenu = direction_de_glissement(d, face);
            assert_eq!(obtenu, attendu, "face {face:?}, direction {d}");
            assert!(obtenu == a || obtenu == b);
            assert_eq!(
                ecart(d, obtenu) as u32,
                angle_de_contact(d, face),
                "face {face:?}, direction {d} : écart au glissement"
            );
        }
    }
    // À la main : les égalités, les deux côtés d'une direction, la plus petite à égalité.
    assert_eq!(direction_de_glissement(0, Face::Verticale), 90);
    assert_eq!(direction_de_glissement(180, Face::Verticale), 90);
    assert_eq!(direction_de_glissement(45, Face::Verticale), 90);
    assert_eq!(direction_de_glissement(135, Face::Verticale), 90);
    assert_eq!(direction_de_glissement(225, Face::Verticale), 270);
    assert_eq!(direction_de_glissement(315, Face::Verticale), 270);
    assert_eq!(direction_de_glissement(90, Face::Horizontale), 0);
    assert_eq!(direction_de_glissement(270, Face::Horizontale), 0);
    assert_eq!(direction_de_glissement(100, Face::Horizontale), 180);
    assert_eq!(direction_de_glissement(260, Face::Horizontale), 180);
    assert_eq!(direction_de_glissement(359, Face::Horizontale), 0);
}

/// Les branches de contact sont les deux directions parallèles à la face, opposées, exactement horizontales ou verticales
/// dans les tables, **dans l'ordre croissant** (l'ordre des naissances d'un arrêt) : `(90, 270)` pour la face verticale,
/// `(0, 180)` pour l'horizontale.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_branches_de_contact_sont_les_deux_directions_paralleles_a_la_face() {
    outils::delai();
    for face in FACES {
        let (a, b) = branches_de_contact(face);
        assert_eq!((a, b), paralleles(face), "face {face:?}");
        assert_eq!((a + 180) % 360, b, "face {face:?} : les deux branches sont opposées ({a}, {b})");
        // Parallèle à la face : la composante selon la normale est exactement nulle.
        for d in [a, b] {
            let normale = match face {
                Face::Verticale => COS[d],
                Face::Horizontale => SIN[d],
            };
            assert_eq!(normale, 0.0, "face {face:?}, branche {d} : composante normale");
        }
    }
}

/// Les deux fonctions de direction paniquent pour une direction de 360 ou plus, pour les deux faces (comme `virage_chimiotropique`).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn une_direction_hors_de_la_table_est_une_panique() {
    outils::delai();
    for face in FACES {
        for d in [360usize, 361, 720, usize::MAX] {
            exiger_la_panique(&format!("angle_de_contact({d}, {face:?})"), || u64::from(angle_de_contact(d, face)));
            exiger_la_panique(&format!("direction_de_glissement({d}, {face:?})"), || {
                direction_de_glissement(d, face) as u64
            });
        }
        // Garde : 359 ne panique pas.
        let _ = angle_de_contact(359, face);
        let _ = direction_de_glissement(359, face);
    }
}

/// `Tropismes::default()` éteint le thigmotropisme : l'angle est `+0.0` (bits nuls) et la force du chimiotropisme aussi ; la
/// mise à jour de structure des tests de T1 compile, garde l'angle nul et change seulement la force ; poser l'angle seul
/// laisse la force nulle ; `Copy`, `Clone`, `PartialEq` et `Debug` sont là.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_champ_thigmotropisme_vaut_zero_par_defaut_et_la_mise_a_jour_de_structure_marche() {
    outils::delai();
    let defaut = Tropismes::default();
    assert_eq!(defaut.thigmotropisme.to_bits(), 0, "thigmotropisme par défaut : attendu +0.0");
    assert_eq!(defaut.chimiotropisme.to_bits(), 0);
    let avec_force = Tropismes { chimiotropisme: 2.5, ..Tropismes::default() };
    assert_eq!(avec_force.chimiotropisme, 2.5);
    assert_eq!(avec_force.thigmotropisme.to_bits(), 0);
    let avec_angle = Tropismes { thigmotropisme: 35.0, ..Tropismes::default() };
    assert_eq!(avec_angle.thigmotropisme, 35.0);
    assert_eq!(avec_angle.chimiotropisme.to_bits(), 0);
    assert_ne!(avec_angle, defaut);
    assert_ne!(avec_angle, avec_force);
    let copie = avec_angle;
    #[allow(clippy::clone_on_copy)]
    let clone = avec_angle.clone();
    assert_eq!(copie, clone);
    assert!(format!("{avec_angle:?}").contains("thigmotropisme"), "le Debug nomme le champ");
}
