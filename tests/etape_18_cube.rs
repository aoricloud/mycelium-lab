//! Tests d'acceptation de l'étape 18 : le scénario du cube et sa simulation.
//!
//! Ils lisent les deux scénarios de `scenarios/` (la goutte et le mur), refusent mot pour mot les textes mal formés,
//! vérifient la construction du substrat en volume, puis rejouent chaque scénario jusqu'à son empreinte de référence
//! (`references/cube-goutte.empreinte`, `references/cube-mur.empreinte`) et regardent ce qu'on doit voir : la masse
//! conservée, les cases du mur à zéro, et la nourriture qui passe la fenêtre, freinée.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/` et `abs` seulement.

use std::path::Path;

use mycelium_lab::cube::{Bloc, Cube, Depot, ErreurCube, ScenarioCube, lire_scenario_cube};
use mycelium_lab::substrat3d::ErreurSubstrat3d;

/// Les deux scénarios du cube et leur fichier d'empreinte de référence.
const SCENARIOS: [(&str, &str); 2] =
    [("cube-goutte.txt", "cube-goutte.empreinte"), ("cube-mur.txt", "cube-mur.empreinte")];

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// Le scénario d'un fichier de `scenarios/`.
fn scenario(nom: &str) -> ScenarioCube {
    lire_scenario_cube(&texte_du_fichier(nom)).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"))
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
fn erreur(texte: &str) -> ErreurCube {
    match lire_scenario_cube(texte) {
        Ok(scenario) => panic!("ce texte devait être refusé, il donne {scenario:?}"),
        Err(erreur) => erreur,
    }
}

/// Un cube du scénario, avancé de sa durée d'un seul coup.
fn cube_fini(nom: &str) -> Cube {
    let scenario = scenario(nom);
    let mut cube = Cube::nouveau(&scenario).expect("le scénario se construit");
    cube.avancer(scenario.duree);
    cube
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn les_deux_scenarios_de_cube_se_lisent_avec_leurs_valeurs() {
    let goutte = scenario("cube-goutte.txt");
    assert_eq!((goutte.largeur, goutte.hauteur, goutte.profondeur), (32, 32, 32), "cube-goutte : les dimensions");
    assert_eq!(goutte.substrat_initial, 0.0, "cube-goutte : le substrat initial");
    assert_eq!(goutte.diffusion, 0.15, "cube-goutte : la diffusion");
    assert_eq!(goutte.depots, vec![Depot { x: 16, y: 16, z: 16, quantite: 100.0 }], "cube-goutte : le dépôt");
    assert!(goutte.blocs.is_empty(), "cube-goutte : aucun bloc");
    assert_eq!(goutte.duree, 300, "cube-goutte : la durée");

    let mur = scenario("cube-mur.txt");
    assert_eq!((mur.largeur, mur.hauteur, mur.profondeur), (32, 32, 32), "cube-mur : les dimensions");
    assert_eq!(mur.substrat_initial, 0.0, "cube-mur : le substrat initial");
    assert_eq!(mur.diffusion, 0.15, "cube-mur : la diffusion");
    assert_eq!(mur.depots, vec![Depot { x: 16, y: 16, z: 8, quantite: 100.0 }], "cube-mur : le dépôt");
    assert_eq!(
        mur.blocs,
        vec![
            Bloc { x0: 0, y0: 0, z0: 20, x1: 31, y1: 13, z1: 21 },
            Bloc { x0: 0, y0: 18, z0: 20, x1: 31, y1: 31, z1: 21 },
            Bloc { x0: 0, y0: 14, z0: 20, x1: 13, y1: 17, z1: 21 },
            Bloc { x0: 18, y0: 14, z0: 20, x1: 31, y1: 17, z1: 21 },
        ],
        "cube-mur : les quatre blocs, dans l'ordre du texte"
    );
    assert_eq!(mur.duree, 400, "cube-mur : la durée");
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn un_texte_avec_commentaires_et_lignes_vides_se_lit_pareil() {
    let goutte = "# la goutte\n\ncube 32 32 32\n# le substrat\nsubstrat 0.0 0.15\n\nnourriture 16 16 16 100.0\n\nduree 300\n# fin\n";
    assert_eq!(lire_scenario_cube(goutte), Ok(scenario("cube-goutte.txt")), "cube-goutte annoté");

    let mur = "# le mur\ncube 32 32 32\n\nsubstrat 0.0 0.15\nbloc 0 0 20 31 13 21\n# la fenêtre\n\
               bloc 0 18 20 31 31 21\nbloc 0 14 20 13 17 21\n\nbloc 18 14 20 31 17 21\n\
               nourriture 16 16 8 100.0\n# la fin\nduree 400\n";
    assert_eq!(lire_scenario_cube(mur), Ok(scenario("cube-mur.txt")), "cube-mur annoté");
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn les_directives_uniques_manquantes_ou_en_double_sont_refusees() {
    assert_eq!(erreur("substrat 0.0 0.1\nduree 1\n"), ErreurCube::Manquante("cube"), "sans cube");
    assert_eq!(erreur("cube 4 4 4\nduree 1\n"), ErreurCube::Manquante("substrat"), "sans substrat");
    assert_eq!(erreur("cube 4 4 4\nsubstrat 0.0 0.1\n"), ErreurCube::Manquante("duree"), "sans durée");
    assert_eq!(
        erreur("# commentaire\ncube 4 4 4\ncube 4 4 4\nsubstrat 0.0 0.1\nduree 1\n"),
        ErreurCube::EnDouble("cube"),
        "cube en double"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn les_lignes_mal_formees_sont_refusees_avec_leur_numero() {
    assert_eq!(erreur("cube 4 0 4\nsubstrat 0.0 0.1\nduree 1\n"), ErreurCube::LigneInvalide(1), "cube 4 0 4");
    assert_eq!(erreur("cube 4 4\nsubstrat 0.0 0.1\nduree 1\n"), ErreurCube::LigneInvalide(1), "cube 4 4");
    assert_eq!(
        erreur("inconnu 1 2 3\ncube 4 4 4\nsubstrat 0.0 0.1\nduree 1\n"),
        ErreurCube::LigneInvalide(1),
        "un mot inconnu"
    );
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat  0.0 0.1\nduree 1\n"),
        ErreurCube::LigneInvalide(2),
        "deux espaces de suite"
    );
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat 0.0 0.1\n# une ligne vide suit\n\nnourriture 0 0 0 -1.0\nduree 1\n"),
        ErreurCube::LigneInvalide(5),
        "une quantité négative"
    );
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat 0.0 0.1\n# une ligne vide suit\n\nnourriture 0 0 0 NaN\nduree 1\n"),
        ErreurCube::LigneInvalide(5),
        "une quantité NaN"
    );
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat 0.0 0.1\nbloc 3 0 0 2 1 1\nduree 1\n"),
        ErreurCube::LigneInvalide(3),
        "un bloc à l'envers"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn les_depots_et_blocs_hors_du_cube_ou_dans_un_bloc_sont_refuses() {
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat 0.0 0.1\nnourriture 4 0 0 1.0\nduree 1\n"),
        ErreurCube::HorsDuCube(3),
        "une nourriture hors du cube"
    );
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat 0.0 0.1\nbloc 0 0 0 4 0 0\nduree 1\n"),
        ErreurCube::HorsDuCube(3),
        "un bloc qui sort"
    );
    assert_eq!(
        erreur("cube 4 4 4\nsubstrat 0.0 0.1\nbloc 0 0 0 1 1 1\nnourriture 0 0 0 1.0\nduree 1\n"),
        ErreurCube::DepotBloque(4),
        "une nourriture dans un bloc"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn une_diffusion_de_zero_deux_se_lit_mais_la_construction_la_refuse() {
    let scenario = lire_scenario_cube("cube 4 4 4\nsubstrat 0.5 0.2\nduree 1\n").expect("le texte se lit");
    assert_eq!(scenario.diffusion, 0.2, "la lecture ne juge pas la stabilité");
    assert_eq!(
        Cube::nouveau(&scenario),
        Err(ErreurCube::Substrat(ErreurSubstrat3d::PasInstable)),
        "6 × 0.2 dépasse 1 : la construction refuse"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn la_construction_remplit_les_cases_libres_les_blocs_et_les_depots() {
    let texte = "cube 4 4 4\nsubstrat 0.5 0.1\nbloc 1 1 1 2 2 2\nnourriture 0 0 0 2.0\nduree 10\n";
    let lu = lire_scenario_cube(texte).expect("le scénario se lit");
    let cube = Cube::nouveau(&lu).expect("le substrat est stable");
    assert_eq!(cube.temps(), 0, "le temps part de zéro");
    assert_eq!(cube.duree(), 10, "la durée est celle du scénario");
    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                let dans_le_bloc = (1..=2).contains(&x) && (1..=2).contains(&y) && (1..=2).contains(&z);
                if dans_le_bloc {
                    assert_eq!(cube.substrat().valeur(x, y, z), 0.0, "la case bloquée ({x}, {y}, {z})");
                    assert!(cube.substrat().est_bloquee(x, y, z), "la case ({x}, {y}, {z}) est bloquée");
                } else if (x, y, z) == (0, 0, 0) {
                    assert_eq!(cube.substrat().valeur(x, y, z), 2.5, "l'initial 0,5 plus le dépôt 2,0");
                } else {
                    assert_eq!(cube.substrat().valeur(x, y, z), 0.5, "la case libre ({x}, {y}, {z})");
                }
            }
        }
    }
    let depart = Cube::nouveau(&scenario("cube-goutte.txt")).expect("cube-goutte se construit");
    assert_eq!(depart.substrat().masse_totale(), 100.0, "la masse de départ de cube-goutte, exactement");
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn chaque_scenario_avance_de_sa_duree_a_l_empreinte_de_son_fichier() {
    for (nom, fichier) in SCENARIOS {
        let scenario = scenario(nom);
        let reference = empreinte_de_reference(fichier);
        let mut d_un_coup = Cube::nouveau(&scenario).expect("le scénario se construit");
        d_un_coup.avancer(scenario.duree);
        assert_eq!(d_un_coup.temps(), scenario.duree, "{nom} : le temps est la durée");
        assert_eq!(d_un_coup.empreinte(), reference, "{nom} : l'empreinte d'un seul coup");
        assert_eq!(d_un_coup.ecrire_canonique().len(), 4 + 24 + 8 * 32 * 32 * 32, "{nom} : la longueur canonique");

        let mut par_morceaux = Cube::nouveau(&scenario).expect("le scénario se construit");
        for pas in [1, 0, 49] {
            par_morceaux.avancer(pas);
        }
        par_morceaux.avancer(scenario.duree - 50);
        assert_eq!(par_morceaux.temps(), scenario.duree, "{nom} : le temps après les morceaux");
        assert_eq!(par_morceaux.empreinte(), reference, "{nom} : l'empreinte par morceaux");
        assert_eq!(
            par_morceaux.ecrire_canonique(),
            d_un_coup.ecrire_canonique(),
            "{nom} : deux cubes du même scénario, les mêmes octets"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn l_empreinte_au_temps_zero_differe_de_celle_de_la_fin() {
    for (nom, _) in SCENARIOS {
        let scenario = scenario(nom);
        let depart = Cube::nouveau(&scenario).expect("le scénario se construit");
        assert_eq!(depart.temps(), 0, "{nom} : le temps part de zéro");
        let fin = cube_fini(nom);
        assert_ne!(depart.empreinte(), fin.empreinte(), "{nom} : l'empreinte du départ et celle de la fin");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn a_la_fin_de_cube_goutte_la_masse_vaut_cent_et_le_coin_est_positif() {
    let goutte = cube_fini("cube-goutte.txt");
    let masse = goutte.substrat().masse_totale();
    assert!(egal_relatif(masse, 100.0, 1e-9), "la masse vaut {masse}");
    let coin = goutte.substrat().valeur(0, 0, 0);
    assert!(coin > 0.0, "le coin (0, 0, 0) vaut {coin}");
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn a_la_fin_de_cube_mur_le_mur_est_nul_et_la_fenetre_freine_la_nourriture() {
    let scenario = scenario("cube-mur.txt");
    let mur = cube_fini("cube-mur.txt");
    for bloc in &scenario.blocs {
        for z in bloc.z0..=bloc.z1 {
            for y in bloc.y0..=bloc.y1 {
                for x in bloc.x0..=bloc.x1 {
                    let (x, y, z) = (x as usize, y as usize, z as usize);
                    assert_eq!(mur.substrat().valeur(x, y, z), 0.0, "la case du mur ({x}, {y}, {z})");
                }
            }
        }
    }
    let masse = mur.substrat().masse_totale();
    assert!(egal_relatif(masse, 100.0, 1e-9), "la masse vaut {masse}");
    let goutte = cube_fini("cube-goutte.txt");
    let derriere = mur.substrat().valeur(16, 16, 28);
    let libre = goutte.substrat().valeur(16, 16, 28);
    assert!(derriere > 0.0, "la nourriture passe la fenêtre : (16, 16, 28) vaut {derriere}");
    assert!(derriere < libre, "freinée derrière la fenêtre : {derriere} < {libre}");
}
