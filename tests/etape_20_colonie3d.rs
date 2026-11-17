//! Tests d'acceptation de l'étape 20 : la colonie en volume.
//!
//! Ce qui est vérifié :
//! - les empreintes : au temps 0, après 1 et après 10 pas pour `colonie-cube` (valeurs écrites ici), puis à la fin
//!   de sa durée pour chacune des deux scènes, l'empreinte de son fichier de `references/` ;
//! - les comptes de la fin de chaque scène (segments, ramifications), l'égalité
//!   `apex().len() == pointes() == 1 + nb_ramifications()`, et les identifiants d'apex de 0 à n - 1 sans trou ;
//! - le déterminisme : même graine, mêmes octets canoniques, `avancer_n(n)` comme `n` appels à `avancer`, et deux
//!   graines différentes qui donnent deux empreintes différentes ;
//! - la nourriture : la masse du substrat au temps 0 moins `cout · longueur()` des segments vaut la masse finale ;
//! - la géométrie : extrémités et apex dans le cube, longueurs dans `]0, pas]`, directions de norme 1 ;
//! - l'ordre des segments par `(temps, apex)` strictement croissant, et le premier segment parti du centre de la
//!   spore, par l'apex 0, au temps 1 ;
//! - le temps 0 : un seul apex, d'identifiant 0, actif, au centre de la spore, aucun segment, aucune ramification ;
//! - un scénario sans ramification ni virage : un seul apex, tous les segments de même direction ;
//! - les erreurs de lecture, une par cas, avec leur numéro de ligne, et les blancs et commentaires acceptés ;
//! - la construction : un pas instable est refusé, `0.125` est accepté.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt` et `abs` seulement.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ErreurColonie3d, ScenarioColonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::direction3d::Direction3d;
use mycelium_lab::substrat3d::ErreurSubstrat3d;

/// Les deux scènes et leur fichier d'empreinte de référence.
const SCENARIOS: [(&str, &str); 2] =
    [("colonie-cube.txt", "colonie-cube.empreinte"), ("colonie-goutte.txt", "colonie-goutte.empreinte")];

/// Les comptes de la fin de chaque scène : nombre de segments, puis nombre de ramifications.
const COMPTES: [(&str, usize, u64); 2] = [("colonie-cube.txt", 3996, 694), ("colonie-goutte.txt", 3439, 603)];

/// Le petit scénario valide des erreurs de lecture, une directive par ligne, dans un cube de 8 cases.
const LIGNES: [&str; 7] = [
    "cube 8 8 8",
    "substrat 1.0 0.1",
    "nourriture 0 0 0 1.0",
    "spore 4 4 4",
    "croissance 0.5 6 30 20",
    "couplage 0.1 0.5",
    "duree 10",
];

/// Un scénario sans ramification ni virage : `intervalle` infini et `amplitude` nulle.
const SANS_VIRAGE: &str = "cube 40 40 40\nsubstrat 1.0 0.0\nspore 20 20 20\ncroissance 0.5 infini 30 0\n\
                           couplage 0.1 0.5\nduree 30\n";

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// Le scénario d'un fichier de `scenarios/`.
fn scenario(nom: &str) -> ScenarioColonie3d {
    lire_scenario_colonie3d(&texte_du_fichier(nom)).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"))
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

/// L'erreur exacte d'un texte que la lecture doit refuser.
fn erreur(texte: &str) -> ErreurColonie3d {
    match lire_scenario_colonie3d(texte) {
        Ok(scenario) => panic!("ce texte devait être refusé, il donne {scenario:?}"),
        Err(erreur) => erreur,
    }
}

/// Une colonie de la scène `nom`, à la graine `graine`, au temps 0.
fn colonie(nom: &str, graine: u64) -> Colonie3d {
    let scenario = scenario(nom);
    Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"))
}

/// Une colonie de la scène `nom`, graine 1, avancée jusqu'à sa durée.
fn colonie_finie(nom: &str) -> Colonie3d {
    let scenario = scenario(nom);
    let mut colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    colonie.avancer_n(scenario.duree);
    colonie
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

/// La direction d'un segment : ses différences de coordonnées, normalisées.
fn direction_du_segment(segment: &Segment3d) -> Direction3d {
    let dx = segment.x1 - segment.x0;
    let dy = segment.y1 - segment.y0;
    let dz = segment.z1 - segment.z0;
    Direction3d::nouvelle(dx, dy, dz).expect("un segment a une longueur strictement positive")
}

/// Le texte du petit scénario valide.
fn texte_valide() -> String {
    LIGNES.join("\n")
}

/// Le petit scénario valide, la ligne `numero` (à partir de 1) remplacée.
fn avec_ligne(numero: usize, ligne: &str) -> String {
    assert!((1..=LIGNES.len()).contains(&numero), "la ligne {numero} existe dans le petit scénario");
    let mut lignes = LIGNES.to_vec();
    lignes[numero - 1] = ligne;
    lignes.join("\n")
}

/// Le petit scénario valide, la ligne `numero` (à partir de 1) ôtée.
fn sans_ligne(numero: usize) -> String {
    assert!((1..=LIGNES.len()).contains(&numero), "la ligne {numero} existe dans le petit scénario");
    let mut lignes = LIGNES.to_vec();
    lignes.remove(numero - 1);
    lignes.join("\n")
}

/// Le petit scénario valide suivi d'une ligne de plus : cette ligne est la huitième.
fn avec_ligne_8(ligne: &str) -> String {
    format!("{}\n{}\n", texte_valide(), ligne)
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn chaque_scene_avance_de_sa_duree_a_l_empreinte_de_son_fichier() {
    for (nom, fichier) in SCENARIOS {
        let scenario = scenario(nom);
        let reference = empreinte_de_reference(fichier);
        let mut colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
        colonie.avancer_n(scenario.duree);
        assert_eq!(colonie.temps(), scenario.duree, "{nom} : le temps est la durée");
        assert_eq!(colonie.empreinte(), reference, "{nom} : l'empreinte de référence");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn colonie_cube_a_les_empreintes_ecrites_au_temps_zero_un_et_dix_pas() {
    let mut colonie = colonie("colonie-cube.txt", 1);
    assert_eq!(
        colonie.empreinte(),
        "529b086e315830ca4103af6d05d31329d496049529fd89ef6cdb7a5bff5e74ef",
        "colonie-cube : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-cube : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "e336a52fd7018fb86cb8849519f624fdbd57d560284afa72d4ebaf7a2508e4c3",
        "colonie-cube : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-cube : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "c933c578f9e43bf969cdd699ef4066cd472659f60fcff3a2d41f5e4e6b3ddb1f",
        "colonie-cube : l'empreinte après 10 pas"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_comptes_de_la_fin_sont_ceux_du_contrat_et_les_apex_sont_numerotes_sans_trou() {
    for (nom, nb_segments, nb_ramifications) in COMPTES {
        let colonie = colonie_finie(nom);
        assert_eq!(colonie.segments().len(), nb_segments, "{nom} : les segments");
        assert_eq!(colonie.nb_ramifications(), nb_ramifications, "{nom} : les ramifications");
        let n = colonie.apex().len();
        assert_eq!(n, 1 + nb_ramifications as usize, "{nom} : les apex, un de plus que les ramifications");
        assert_eq!(colonie.pointes(), n, "{nom} : les pointes, tous les apex dans cette étape");
        assert!(n > 0, "{nom} : des apex à numéroter");
        for (rang, apex) in colonie.apex().iter().enumerate() {
            assert_eq!(apex.id, rang as u32, "{nom} : l'identifiant du rang {rang}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn meme_graine_memes_octets_et_avancer_n_vaut_les_appels_un_a_un() {
    for (nom, _) in SCENARIOS {
        let scenario = scenario(nom);
        let mut d_un_coup = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
        let mut un_a_un = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
        d_un_coup.avancer_n(scenario.duree);
        for _ in 0..scenario.duree {
            un_a_un.avancer();
        }
        assert_eq!(un_a_un.temps(), d_un_coup.temps(), "{nom} : le même temps");
        assert_eq!(
            un_a_un.ecrire_canonique(),
            d_un_coup.ecrire_canonique(),
            "{nom} : avancer_n(n) et n appels à avancer, les mêmes octets"
        );
        assert_eq!(un_a_un.empreinte(), d_un_coup.empreinte(), "{nom} : la même empreinte");

        let mut autre_graine = Colonie3d::nouvelle(&scenario, 2).expect("le scénario se construit");
        autre_graine.avancer_n(scenario.duree);
        assert_ne!(
            autre_graine.empreinte(),
            d_un_coup.empreinte(),
            "{nom} : les graines 1 et 2 donnent des empreintes différentes"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn la_masse_du_substrat_perd_exactement_le_cout_des_segments() {
    for (nom, _) in SCENARIOS {
        let scenario = scenario(nom);
        let mut colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
        let masse_initiale = colonie.substrat().masse_totale();
        colonie.avancer_n(scenario.duree);
        let mut consomme = 0.0;
        for segment in colonie.segments() {
            consomme += scenario.cout * segment.longueur();
        }
        let masse_finale = colonie.substrat().masse_totale();
        assert!(!colonie.segments().is_empty(), "{nom} : des segments consommés");
        assert!(
            egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
            "{nom} : {masse_initiale} - {consomme} contre la masse finale {masse_finale}"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_segments_et_les_apex_restent_dans_le_cube_et_les_longueurs_dans_le_pas() {
    for (nom, _) in SCENARIOS {
        let colonie = colonie_finie(nom);
        let scenario = colonie.scenario();
        let largeur = scenario.largeur as f64;
        let hauteur = scenario.hauteur as f64;
        let profondeur = scenario.profondeur as f64;
        assert!(!colonie.segments().is_empty(), "{nom} : des segments à vérifier");
        for segment in colonie.segments() {
            let longueur = segment.longueur();
            assert!(longueur > 0.0, "{nom} : la longueur {longueur} est strictement positive");
            assert!(
                longueur <= scenario.pas + 1e-12,
                "{nom} : la longueur {longueur} ne dépasse pas le pas {}",
                scenario.pas
            );
            for (x, y, z) in [(segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)] {
                assert!((0.0..largeur).contains(&x), "{nom} : x = {x} dans [0, {largeur})");
                assert!((0.0..hauteur).contains(&y), "{nom} : y = {y} dans [0, {hauteur})");
                assert!((0.0..profondeur).contains(&z), "{nom} : z = {z} dans [0, {profondeur})");
            }
        }
        assert!(!colonie.apex().is_empty(), "{nom} : des apex à vérifier");
        for apex in colonie.apex() {
            assert!(
                (0.0..largeur).contains(&apex.x),
                "{nom} : x = {} de l'apex {} dans [0, {largeur})",
                apex.x,
                apex.id
            );
            assert!(
                (0.0..hauteur).contains(&apex.y),
                "{nom} : y = {} de l'apex {} dans [0, {hauteur})",
                apex.y,
                apex.id
            );
            assert!(
                (0.0..profondeur).contains(&apex.z),
                "{nom} : z = {} de l'apex {} dans [0, {profondeur})",
                apex.z,
                apex.id
            );
            let direction = &apex.direction;
            let norme =
                (direction.x() * direction.x() + direction.y() * direction.y() + direction.z() * direction.z()).sqrt();
            assert!((norme - 1.0).abs() <= 1e-15, "{nom} : la norme {norme} de la direction de l'apex {}", apex.id);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_segments_sont_ranges_par_temps_et_apex_et_le_premier_part_du_centre_de_la_spore() {
    for (nom, _) in SCENARIOS {
        let colonie = colonie_finie(nom);
        let segments = colonie.segments();
        assert!(!segments.is_empty(), "{nom} : des segments à ordonner");
        for paire in segments.windows(2) {
            let (avant, apres) = (paire[0], paire[1]);
            assert!(
                (avant.temps, avant.apex) < (apres.temps, apres.apex),
                "{nom} : ({}, {}) précède strictement ({}, {})",
                avant.temps,
                avant.apex,
                apres.temps,
                apres.apex
            );
        }
        let premier = segments[0];
        assert_eq!(premier.apex, 0, "{nom} : le premier segment est celui de l'apex 0");
        assert_eq!(premier.temps, 1, "{nom} : le premier segment est au temps 1");
        let (sx, sy, sz) = colonie.scenario().spore;
        assert_eq!(
            (premier.x0, premier.y0, premier.z0),
            (sx as f64 + 0.5, sy as f64 + 0.5, sz as f64 + 0.5),
            "{nom} : le premier segment part du centre de la spore"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn au_temps_zero_un_seul_apex_au_centre_de_la_spore() {
    for (nom, _) in SCENARIOS {
        let scenario = scenario(nom);
        let colonie = colonie(nom, 1);
        assert_eq!(colonie.temps(), 0, "{nom} : le temps part de zéro");
        assert!(colonie.segments().is_empty(), "{nom} : aucun segment au temps 0");
        assert_eq!(colonie.nb_ramifications(), 0, "{nom} : aucune ramification au temps 0");
        assert_eq!(colonie.apex().len(), 1, "{nom} : un seul apex au temps 0");
        assert_eq!(colonie.pointes(), 1, "{nom} : une seule pointe au temps 0");
        let apex = colonie.apex()[0];
        let (sx, sy, sz) = scenario.spore;
        assert_eq!(apex.id, 0, "{nom} : l'apex du temps 0 a l'identifiant 0");
        assert!(apex.actif, "{nom} : l'apex du temps 0 est actif");
        assert_eq!(
            (apex.x, apex.y, apex.z),
            (sx as f64 + 0.5, sy as f64 + 0.5, sz as f64 + 0.5),
            "{nom} : l'apex du temps 0 est au centre de la spore"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn un_scenario_sans_ramification_ni_virage_garde_un_apex_et_une_direction() {
    let scenario = lire_scenario_colonie3d(SANS_VIRAGE).expect("le scénario sans virage se lit");
    let mut colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario sans virage se construit");
    colonie.avancer_n(scenario.duree);
    assert_eq!(colonie.apex().len(), 1, "aucune ramification : un seul apex");
    assert_eq!(colonie.nb_ramifications(), 0, "aucune ramification comptée");
    assert_eq!(colonie.pointes(), 1, "une seule pointe");
    assert_eq!(colonie.segments().len(), scenario.duree as usize, "un segment par pas");
    assert!(colonie.segments().len() >= 2, "au moins deux segments pour comparer leurs directions");
    let mut paires = 0;
    for paire in colonie.segments().windows(2) {
        let scalaire = direction_du_segment(&paire[0]).scalaire(&direction_du_segment(&paire[1]));
        assert!((scalaire - 1.0).abs() <= 1e-12, "segments successifs : produit scalaire {scalaire}");
        paires += 1;
    }
    assert_eq!(paires, colonie.segments().len() - 1, "chaque paire successive a été comparée");
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_lignes_mal_formees_sont_refusees_avec_leur_numero() {
    assert_eq!(erreur(&avec_ligne(1, "inconnu 8 8 8")), ErreurColonie3d::LigneInvalide(1), "un mot inconnu, ligne 1");
    assert_eq!(erreur(&avec_ligne(6, "inconnue 1 2")), ErreurColonie3d::LigneInvalide(6), "un mot inconnu, ligne 6");
    assert_eq!(erreur(&avec_ligne(1, "cube 8 8")), ErreurColonie3d::LigneInvalide(1), "trop peu de mots, ligne 1");
    assert_eq!(erreur(&avec_ligne(2, "substrat 1.0")), ErreurColonie3d::LigneInvalide(2), "trop peu de mots, ligne 2");
    assert_eq!(
        erreur(&avec_ligne(3, "nourriture 0 0 0")),
        ErreurColonie3d::LigneInvalide(3),
        "trop peu de mots, ligne 3"
    );
    assert_eq!(
        erreur(&avec_ligne(1, "cube 8 huit 8")),
        ErreurColonie3d::LigneInvalide(1),
        "un nombre illisible, ligne 1"
    );
    assert_eq!(
        erreur(&avec_ligne(5, "croissance abc 6 30 20")),
        ErreurColonie3d::LigneInvalide(5),
        "un nombre illisible, ligne 5"
    );
    assert_eq!(erreur(&avec_ligne(7, "duree dix")), ErreurColonie3d::LigneInvalide(7), "un nombre illisible, ligne 7");
    assert_eq!(erreur(&avec_ligne(4, "spore -4 4 4")), ErreurColonie3d::LigneInvalide(4), "un entier signé, ligne 4");
    assert_eq!(
        erreur(&avec_ligne(7, "duree 10.0")),
        ErreurColonie3d::LigneInvalide(7),
        "un entier écrit avec un point, ligne 7"
    );
    assert_eq!(
        erreur(&avec_ligne(5, "croissance 0.5 6 181 20")),
        ErreurColonie3d::LigneInvalide(5),
        "angle 181, ligne 5"
    );
    assert_eq!(
        erreur(&avec_ligne(5, "croissance 0.5 6 30 181")),
        ErreurColonie3d::LigneInvalide(5),
        "amplitude 181, ligne 5"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_blocs_sont_refuses_et_les_blancs_et_commentaires_sont_acceptes() {
    assert_eq!(erreur(&avec_ligne(3, "bloc 0 0 0 1 1 1")), ErreurColonie3d::BlocRefuse(3), "une ligne bloc, ligne 3");
    assert_eq!(erreur(&avec_ligne_8("bloc 1 1 1 2 2 2")), ErreurColonie3d::BlocRefuse(8), "une ligne bloc, ligne 8");

    let base = lire_scenario_colonie3d(&texte_valide()).expect("le petit scénario valide se lit");
    assert_eq!((base.largeur, base.hauteur, base.profondeur), (8, 8, 8), "le cube du petit scénario");
    assert_eq!((base.angle, base.amplitude), (30, 20), "la croissance du petit scénario");
    assert_eq!(base.duree, 10, "la durée du petit scénario");
    let decore = "  # un commentaire en tête, avec des blancs  \n\n   cube   8  8   8   \n\
                 \tsubstrat\t1.0\t0.1\t\nnourriture 0 0 0 1.0\n   \nspore  4 4  4\n\
                 croissance 0.5 6 30 20\ncouplage 0.1 0.5\nduree   10   \n# et à la fin\n";
    assert_eq!(
        lire_scenario_colonie3d(decore),
        Ok(base),
        "les blancs de tête et de queue, les espaces multiples et les lignes # sont acceptés"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_directives_uniques_manquantes_ou_en_double_sont_refusees() {
    let uniques = [(1, "cube"), (2, "substrat"), (4, "spore"), (5, "croissance"), (6, "couplage"), (7, "duree")];
    for (numero, nom) in uniques {
        assert_eq!(erreur(&sans_ligne(numero)), ErreurColonie3d::Manquante(nom), "sans {nom}, dans l'ordre du contrat");
        assert_eq!(
            erreur(&avec_ligne_8(LIGNES[numero - 1])),
            ErreurColonie3d::EnDouble(nom),
            "{nom} répété à la ligne 8"
        );
    }
    let sans_cube_ni_substrat = {
        let mut lignes = LIGNES.to_vec();
        lignes.remove(1);
        lignes.remove(0);
        lignes.join("\n")
    };
    assert_eq!(
        erreur(&sans_cube_ni_substrat),
        ErreurColonie3d::Manquante("cube"),
        "cube et substrat absents : cube d'abord"
    );
    let sans_deux = {
        let mut lignes = LIGNES.to_vec();
        lignes.remove(6);
        lignes.remove(1);
        lignes.join("\n")
    };
    assert_eq!(
        erreur(&sans_deux),
        ErreurColonie3d::Manquante("substrat"),
        "substrat et durée absents : substrat d'abord"
    );
    let deux_nourritures =
        lire_scenario_colonie3d(&avec_ligne_8("nourriture 1 1 1 2.0")).expect("nourriture se répète sans faute");
    assert_eq!(deux_nourritures.depots.len(), 2, "les deux nourritures sont gardées");
    assert_eq!(deux_nourritures.depots[0].quantite, 1.0, "la première nourriture, dans l'ordre du texte");
    assert_eq!(deux_nourritures.depots[1].quantite, 2.0, "la seconde nourriture, dans l'ordre du texte");
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn les_spores_et_nourritures_hors_du_cube_sont_refusees_dans_l_ordre() {
    assert_eq!(
        erreur(&avec_ligne(4, "spore 8 4 4")),
        ErreurColonie3d::HorsDuCube(4),
        "une spore hors du cube en largeur, ligne 4"
    );
    assert_eq!(
        erreur(&avec_ligne(4, "spore 4 4 8")),
        ErreurColonie3d::HorsDuCube(4),
        "une spore hors du cube en profondeur, ligne 4"
    );
    assert_eq!(
        erreur(&avec_ligne(3, "nourriture 8 0 0 1.0")),
        ErreurColonie3d::HorsDuCube(3),
        "une nourriture hors du cube, ligne 3"
    );
    assert_eq!(
        erreur(&avec_ligne(3, "nourriture 0 0 8 1.0")),
        ErreurColonie3d::HorsDuCube(3),
        "une nourriture hors du cube en profondeur, ligne 3"
    );
    let mut lignes = LIGNES.to_vec();
    lignes[2] = "nourriture 8 0 0 1.0";
    lignes[3] = "spore 8 4 4";
    assert_eq!(
        erreur(&lignes.join("\n")),
        ErreurColonie3d::HorsDuCube(4),
        "la spore passe avant la nourriture, même si sa ligne vient après"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-20"), ignore)]
fn la_construction_refuse_un_pas_instable_et_accepte_la_limite() {
    let instable = lire_scenario_colonie3d(&avec_ligne(2, "substrat 1.0 0.2")).expect("le texte se lit");
    assert_eq!(
        Colonie3d::nouvelle(&instable, 1).err(),
        Some(ErreurColonie3d::Substrat(ErreurSubstrat3d::PasInstable)),
        "6 × 0,2 dépasse 1 : la construction refuse"
    );
    let stable = lire_scenario_colonie3d(&avec_ligne(2, "substrat 1.0 0.125")).expect("le texte se lit");
    assert!(Colonie3d::nouvelle(&stable, 1).is_ok(), "6 × 0,125 vaut 0,75 : la construction accepte");
}
