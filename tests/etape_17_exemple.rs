//! Étape 17 (M4) : l'exemple `nuage_raideur` (ses fonctions, jamais son binaire ni le nuage complet) : les six scénarios,
//! le fichier du nuage, l'image ; puis les règles du dépôt et la section de `MODELE.md`.

#[path = "../examples/nuage_raideur.rs"]
#[allow(dead_code)]
mod exemple;

use exemple::{
    CADRE, COULEURS, Calcul, FOND, HYPHE, MOULES, PROFILS, calculer, fichier, image, nuage, pas, pixels, scenario,
};
use mycelium_lab::image::ecrire_png;
use mycelium_lab::moule::developper;
use mycelium_lab::nuage::{Cavite, Ecarte, Nuage, Point, Raison, ajuster};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn les_six_scenarios_sont_ceux_de_la_specification_et_leur_cavite_touche_les_deux_mors() {
    assert_eq!(
        scenario(&MOULES[1], &PROFILS[1]),
        "boite 29 29\nspore 0 14\nsubstrat 1.0 0.2\ncroissance 1.0 32.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\n\
         thigmotropisme 35\nduree 150\nmoule anneau 14 14 14 8\n"
    );
    // Par moule : la boîte, la spore, la ligne `moule`, et le nombre de cases libres de la colonne de chaque mors.
    let moules = [
        ("brique", (30, 20), (2, 10), "brique 0 0 29 19", 20, 20),
        ("anneau", (29, 29), (0, 14), "anneau 14 14 14 8", 1, 1),
        ("l", (30, 30), (2, 24), "l 0 0 29 29 10", 30, 10),
    ];
    for (m, (nom, boite, spore, ligne, a_gauche, a_droite)) in MOULES.iter().zip(moules) {
        assert_eq!((m.nom, m.boite, m.spore, m.ligne), (nom, boite, spore, ligne));
        // Les deux profils ne diffèrent que par l'intervalle de ramification (7,5 ou 32) et la durée (le dernier pas).
        let (dense, rapide) = (scenario(m, &PROFILS[0]), scenario(m, &PROFILS[1]));
        assert!(dense.contains("\ncroissance 1.0 7.5 30 20\n") && dense.contains("\nduree 100\n"), "{dense}");
        assert_eq!(
            dense.replace("croissance 1.0 7.5 30", "croissance 1.0 32.0 30").replace("duree 100", "duree 150"),
            rapide
        );
        let (reglages, milieu, tropismes, _) = lire_scenario_tropismes(&developper(&rapide).unwrap()).unwrap();
        let cavite = Cavite::de(&Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, 1).unwrap());
        let libres = |x: usize| (0..boite.1).filter(|y| cavite.libres[y * boite.0 + x]).count();
        assert_eq!((libres(0), libres(boite.0 - 1)), (a_gauche, a_droite), "{nom} : les cases libres des mors");
        assert!(cavite.libres[spore.1 * boite.0 + spore.0], "{nom} : la spore est dans la cavité");
    }
    assert_eq!((PROFILS[0].nom, PROFILS[1].nom), ("dense", "rapide"));
    assert_eq!(pas(&PROFILS[0]), [40, 50, 60, 70, 80, 90, 100]);
    assert_eq!(pas(&PROFILS[1]), [60, 70, 80, 90, 100, 110, 120, 130, 140, 150]);
    assert_eq!(exemple::graines(), (1..=200).collect::<Vec<u64>>());
    assert_eq!((HYPHE.ea, HYPHE.ei, exemple::DELTA), (22.9, 18.3 / 49.0, 1.0));
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn calculer_rend_les_six_nuages_dans_l_ordre_des_moules_et_des_profils() {
    // Deux graines dont les six scénarios se calculent vite, 8 et 12 : pour chaque scénario, les comptes mesurés (une
    // non-régression) : retenus, ne traversent pas, trop grandes, autres refus.
    let six = calculer(&[8, 12]).unwrap();
    let attendus = [
        ("brique", "dense", [7, 7, 0, 0]),
        ("brique", "rapide", [4, 16, 0, 0]),
        ("anneau", "dense", [0, 8, 6, 0]),
        ("anneau", "rapide", [0, 20, 0, 0]),
        ("l", "dense", [0, 11, 3, 0]),
        ("l", "rapide", [0, 20, 0, 0]),
    ];
    assert_eq!(six.len(), 6);
    for (i, (c, (moule, profil, comptes))) in six.iter().zip(attendus).enumerate() {
        let n = c.nuage.comptes();
        assert_eq!((c.moule, c.profil), (moule, profil));
        assert_eq!([n.retenus, n.ne_traversent_pas, n.trop_grandes, n.refus], comptes, "{moule} {profil}");
        // C'est le nuage de son scénario, aux pas de son profil et avec les raideurs d'hyphe de l'étape 15.
        let (m, p) = (&MOULES[i / 2], &PROFILS[i % 2]);
        assert_eq!(
            comptes.iter().sum::<usize>(),
            2 * pas(p).len(),
            "{moule} {profil} : une colonie lue par graine et par pas"
        );
        assert_eq!(Ok(&c.nuage), nuage(&scenario(m, p), &[8, 12], &pas(p), HYPHE).as_ref(), "{moule} {profil}");
    }
}

/// Trois calculs faits à la main : sept colonies lues en brique dense (deux points, les trois raisons), un point en brique
/// rapide, aucun dans le L. Les trois points de la brique sont sur `K = 8 ρ²`.
fn a_la_main() -> Vec<Calcul> {
    let point = |graine, pas, densite, raideur, noeuds| Point { graine, pas, densite, raideur, noeuds };
    let ecarte = |graine, pas, raison| Ecarte { graine, pas, raison };
    let dense = Nuage {
        points: vec![point(3, 60, 0.25, 0.5, 12), point(3, 100, 0.5, 2.0, 40)],
        ecartes: vec![
            ecarte(1, 60, Raison::NeTraversePas),
            ecarte(1, 100, Raison::TropGrande),
            ecarte(2, 60, Raison::NeTraversePas),
            ecarte(2, 100, Raison::Refus("Mecanisme".to_string())),
            ecarte(4, 60, Raison::NeTraversePas),
        ],
    };
    let rapide = Nuage { points: vec![point(7, 150, 1.0, 8.0, 99)], ecartes: vec![] };
    let l = Nuage { points: vec![], ecartes: vec![ecarte(5, 40, Raison::NeTraversePas)] };
    vec![
        Calcul { moule: "brique", profil: "dense", nuage: dense },
        Calcul { moule: "brique", profil: "rapide", nuage: rapide },
        Calcul { moule: "l", profil: "dense", nuage: l },
    ]
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn le_fichier_du_nuage_a_son_en_tete_une_ligne_par_point_les_comptes_des_ecartes_et_les_droites() {
    // Une droite de pente 2, écrite comme le fichier l'écrit.
    let ecrite = |paires: &[(f64, f64)]| {
        let d = ajuster(paires).unwrap();
        assert!((d.pente - 2.0).abs() < 1e-12 && (d.r2 - 1.0).abs() < 1e-12, "{d:?}");
        format!("pente {:e} ordonnee {:e} r2 {:e} points {}", d.pente, d.ordonnee, d.r2, d.points)
    };
    let attendu = [
        "# Nuage de l'étape 17 (M4) : la raideur selon la densité. Un modèle à nous, pas une reproduction.",
        "# densité en cases^-1 (longueur d'hyphe par case libre), raideur en µN par case ; un point par colonie qui traverse.",
        "# colonnes : moule profil graine pas densite raideur noeuds",
        "brique dense 3 60 2.5e-1 5e-1 12",
        "brique dense 3 100 5e-1 2e0 40",
        "brique rapide 7 150 1e0 8e0 99",
        "# ecartes brique dense : ne_traversent_pas 3 trop_grandes 1 autres_refus 1",
        &format!("# droite brique dense : {}", ecrite(&[(0.25, 0.5), (0.5, 2.0)])),
        "# ecartes brique rapide : ne_traversent_pas 0 trop_grandes 0 autres_refus 0",
        "# droite brique rapide : indisponible DensitesIdentiques",
        "# ecartes l dense : ne_traversent_pas 1 trop_grandes 0 autres_refus 0",
        "# droite l dense : indisponible DensitesIdentiques",
        &format!("# droite brique reunis : {}", ecrite(&[(0.25, 0.5), (0.5, 2.0), (1.0, 8.0)])),
        "# droite anneau reunis : indisponible DensitesIdentiques",
        "# droite l reunis : indisponible DensitesIdentiques",
    ];
    assert_eq!(fichier(&a_la_main()), attendu.join("\n") + "\n");
    // Des nombres qui ne tombent pas juste se relisent au bit près, et chaque ligne de point a sept mots.
    let (densite, raideur) = (0.1 + 0.2, 1.0 / 3.0);
    let mut calculs = a_la_main();
    calculs[1].nuage.points[0] = Point { graine: u64::MAX, pas: u32::MAX, densite, raideur, noeuds: 1000 };
    let texte = fichier(&calculs);
    let points: Vec<Vec<&str>> =
        texte.lines().filter(|l| !l.starts_with('#')).map(|l| l.split(' ').collect()).collect();
    assert!(points.len() == 3 && points.iter().all(|mots| mots.len() == 7), "{points:?}");
    assert_eq!(points[2][..4], ["brique", "rapide", "18446744073709551615", "4294967295"]);
    assert_eq!((points[2][4].parse(), points[2][5].parse(), points[2][6]), (Ok(densite), Ok(raideur), "1000"));
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn l_image_place_les_points_extremes_sur_le_cadre_avec_la_couleur_de_leur_moule() {
    let de = |(l, h, p): (usize, usize, Vec<u8>), x: usize, y: usize| -> [u8; 3] {
        assert_eq!((l, h, p.len()), (640, 480, 640 * 480 * 3));
        [p[(y * 640 + x) * 3], p[(y * 640 + x) * 3 + 1], p[(y * 640 + x) * 3 + 2]]
    };
    // Sans point : le fond et le cadre seuls, un rectangle de 561 × 401 pixels à 40 pixels des bords.
    let (_, _, vide) = pixels(&[]);
    assert_eq!(vide.chunks(3).filter(|p| *p == CADRE).count(), 2 * 561 + 2 * 401 - 4);
    assert_eq!(vide.chunks(3).filter(|p| *p == FOND).count(), 640 * 480 - (2 * 561 + 2 * 401 - 4));
    let coins =
        [(40, 40, CADRE), (600, 440, CADRE), (320, 40, CADRE), (40, 240, CADRE), (39, 40, FOND), (41, 41, FOND)];
    for (x, y, couleur) in coins {
        assert_eq!(de(pixels(&[]), x, y), couleur, "({x}, {y})");
    }
    // Les trois points de la brique (`ρ` de 0,25 à 1, `K` de 0,5 à 8) : le plus petit en bas à gauche du cadre, le plus
    // grand en haut à droite (la raideur monte vers le haut), dans la couleur de la brique, et la droite entre les deux.
    let calculs = a_la_main();
    assert_eq!(de(pixels(&calculs), 40, 440), COULEURS[0]);
    // Le carré est centré sur son point : son coin (39, 439) déborde du cadre.
    assert_eq!(de(pixels(&calculs), 39, 439), COULEURS[0]);
    assert_eq!(de(pixels(&calculs), 600, 40), COULEURS[0]);
    assert_eq!(de(pixels(&calculs), 600, 440), CADRE);
    let brique = pixels(&calculs).2.chunks(3).filter(|p| *p == COULEURS[0]).count();
    assert!((3 * 9 + 150..=3 * 9 + 201).contains(&brique), "trois carrés de 9 pixels et une droite : {brique} pixels");
    // Un point de plus, dans l'anneau, plus dense et moins raide que tous les autres : il prend le coin en bas à droite.
    let mut avec_anneau = a_la_main();
    let point = Point { graine: 1, pas: 60, densite: 4.0, raideur: 0.125, noeuds: 9 };
    avec_anneau.push(Calcul {
        moule: "anneau",
        profil: "rapide",
        nuage: Nuage { points: vec![point], ecartes: vec![] },
    });
    assert_eq!(de(pixels(&avec_anneau), 600, 440), COULEURS[1]);
    assert_eq!(de(pixels(&avec_anneau), 600, 40), CADRE, "la brique n'est plus à l'extrême droite");
    // La droite de la brique s'arrête à ses propres densités extrêmes : elle n'est pas coupée.
    let brique = pixels(&avec_anneau).2.chunks(3).filter(|p| *p == COULEURS[0]).count();
    assert!((3 * 9 + 150..=3 * 9 + 201).contains(&brique), "{brique} pixels");
    // Un seul point : pas d'étendue, il se pose au coin en bas à gauche du cadre, sans droite.
    let seul = [Calcul { moule: "l", profil: "dense", nuage: Nuage { points: vec![point], ecartes: vec![] } }];
    assert_eq!(de(pixels(&seul), 40, 440), COULEURS[2]);
    assert_eq!(pixels(&seul).2.chunks(3).filter(|p| *p == COULEURS[2]).count(), 9);
    // Le PNG est celui de ces pixels.
    let (largeur, hauteur, attendus) = pixels(&avec_anneau);
    assert_eq!(image(&avec_anneau), ecrire_png(largeur as u32, hauteur as u32, &attendus));
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn ecrire_pose_le_fichier_et_l_image_dans_le_dossier_et_le_binaire_refuse_deux_arguments() {
    let dossier = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("etape-17-nuage");
    let calculs = a_la_main();
    // Un passage précédent a pu laisser ces fichiers : on part d'un dossier vide.
    let _ = std::fs::remove_dir_all(&dossier);
    assert_eq!(exemple::ecrire(&dossier, &calculs), Ok(()));
    assert_eq!(std::fs::read(dossier.join("nuage.txt")).unwrap(), fichier(&calculs).into_bytes());
    assert_eq!(std::fs::read(dossier.join("nuage.png")).unwrap(), image(&calculs));
    // Un dossier impossible (sous un fichier) : une erreur qui le nomme, pas une panique.
    let sous_un_fichier = dossier.join("nuage.txt").join("dessous");
    let erreur = exemple::ecrire(&sous_un_fichier, &calculs).unwrap_err();
    assert!(erreur.contains("dessous"), "{erreur}");
    // Deux arguments : refusés avant tout calcul (le nuage complet prendrait dix minutes).
    let deux = ["a".to_string(), "b".to_string()];
    assert_eq!(exemple::lancer(&deux), Err("zéro ou un argument attendu".to_string()));
}

fn lire(chemin: &str) -> String {
    let complet = format!("{}/{chemin}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&complet).unwrap_or_else(|e| panic!("{complet} : {e}"))
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn le_noyau_ne_connait_pas_le_nuage_et_etape_17_est_la() {
    // Le noyau et les couches de calcul d'avant ne connaissent pas le nuage. La liste est fermée, et sans `lib.rs` (la
    // déclaration), `main.rs` ni `vitrine.rs` : une étape future peut employer le nuage sans toucher à ce test.
    for nom in [
        "alea",
        "anastomose",
        "calage",
        "croissance",
        "empreinte",
        "grille",
        "image",
        "mesures",
        "moule",
        "piece",
        "poutres",
        "raideur",
        "releves",
        "reseau",
        "scenario",
        "simulation",
        "substrat",
        "systeme",
        "tropisme",
        "voisinage",
    ] {
        assert!(!lire(&format!("src/{nom}.rs")).contains("nuage"), "src/{nom}.rs parle du nuage");
    }
    assert!(lire("src/lib.rs").contains("\npub mod nuage;\n"));
    // Aucune dépendance ; la fonctionnalité est déclarée après `etape-16`, et elle est la dernière de `default`.
    let cargo = lire("Cargo.toml");
    assert!(cargo.contains("\n[dependencies]\n\n[features]\n") && cargo.matches("dependencies]").count() == 1);
    assert!(cargo.contains("\netape-16 = []\netape-17 = []\n"), "etape-17 se déclare après etape-16");
    assert!(cargo.contains("\"etape-16\", \"etape-17\"]\n"), "etape-17 est la dernière de `default`");
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn la_section_de_modele_md_dit_le_modele_les_statuts_les_nombres_et_les_limites() {
    let modele = lire("MODELE.md");
    let debut = modele.find("\n# La raideur selon la densité (étape 17, M4)\n").expect("la section de l'étape 17");
    let section = &modele[debut + 1..];
    let section = &section[..section.find("\n# ").unwrap_or(section.len())];
    for mot in [
        "à nous",
        "repère",
        "pas une reproduction",
        "ne prédit pas",
        "aucune pente de colonie n'est publiée",
        "lu à la source",
        "de seconde main",
        "calculé",
        "choix",
        "MIT 3.054",
        "Roberts et Garboczi",
        "Gibson et Ashby",
        "155920/601511",
        "plafond de 1 000 nœuds",
        "mors d'une colonne",
        "plus d'hyphes",
        "ne traverse pas",
        "TropGrande",
        "corrélés",
        "nuage_raideur",
        "Ce que les tests ne vérifient pas",
    ] {
        assert!(section.contains(mot), "la section de l'étape 17 ne dit pas « {mot} »");
    }
}
