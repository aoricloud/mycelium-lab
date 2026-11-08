//! Tests d'acceptation de l'étape 5 (T0, le terrain), partie lecture : `lire_scenario_complet` et les directives
//! `source` et `obstacle`.
//!
//! Ils fixent : les deux directives se lisent, optionnelles, répétables et dans n'importe quel ordre ; une ligne mal
//! formée rend `LigneInvalide(numéro)` ; les contrats « dans la boîte » ne se vérifient pas à la lecture mais à la
//! construction ; `lire_scenario` refuse toujours les deux directives ; un texte sans milieu donne par
//! `lire_scenario_complet` les mêmes réglages et la même durée que `lire_scenario`, et un milieu vide.
//!
//! La spécification dit « un nombre » pour la quantité d'une source, sans dire si une quantité négative ou non finie
//! est refusée à la lecture ou à la construction : ces tests acceptent l'un ou l'autre.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use mycelium_lab::simulation::{
    ErreurLecture, Milieu, Obstacle, Simulation, Source, lire_scenario, lire_scenario_complet,
};

/// Un scénario couplé sans milieu : six lignes, donc la ligne suivante est la 7.
const BASE: &str =
    "boite 40 30\nspore 20 15\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nduree 30\n";

/// Le numéro de la première ligne ajoutée à `BASE`.
const PREMIERE_LIGNE_AJOUTEE: usize = 7;

fn avec(supplement: &str) -> String {
    format!("{BASE}{supplement}")
}

fn source(x: usize, y: usize, rayon: usize, quantite: f64) -> Source {
    Source { x, y, rayon, quantite }
}

fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

/// 5. `source` et `obstacle` se lisent : valeurs exactes, dans l'ordre du texte pour chaque sorte, répétables,
///    mêlées ; les réglages et la durée sont ceux du texte sans milieu.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn source_et_obstacle_se_lisent() {
    let (reglages_de_base, duree_de_base) = lire_scenario(BASE).expect("le texte de base se lit");
    let texte = avec(
        "source 10 12 3 2.5\nobstacle 1 2 3 4\nsource 30 5 0 0\nobstacle 7 7 7 7\nsource 0 0 5 0.125\nsource 10 12 3 2.5\n",
    );
    let (reglages, milieu, duree) = lire_scenario_complet(&texte).expect("le texte se lit");
    assert_eq!(reglages, reglages_de_base);
    assert_eq!(duree, duree_de_base);
    let attendu = Milieu {
        sources: vec![source(10, 12, 3, 2.5), source(30, 5, 0, 0.0), source(0, 0, 5, 0.125), source(10, 12, 3, 2.5)],
        obstacles: vec![obstacle(1, 2, 3, 4), obstacle(7, 7, 7, 7)],
    };
    assert_eq!(milieu, attendu);

    // Une quantité entière s'écrit sans point ; une seule source et aucun obstacle, ou l'inverse.
    let (_, milieu, _) = lire_scenario_complet(&avec("source 5 5 2 3\n")).expect("source seule");
    assert_eq!(milieu, Milieu { sources: vec![source(5, 5, 2, 3.0)], obstacles: vec![] });
    let (_, milieu, _) = lire_scenario_complet(&avec("obstacle 5 6 8 6\n")).expect("obstacle seul");
    assert_eq!(milieu, Milieu { sources: vec![], obstacles: vec![obstacle(5, 6, 8, 6)] });
}

/// 5 (suite). L'ordre des directives est indifférent : le milieu avant `boite`, au milieu des autres directives, à la
///    fin ; les lignes vides et les commentaires sont ignorés ; le résultat est le même.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn l_ordre_des_directives_est_indifferent() {
    let reference = lire_scenario_complet(&avec("source 10 12 3 2.5\nobstacle 1 2 3 4\nsource 30 5 1 4.0\n"))
        .expect("lecture de référence");
    assert_eq!(reference.1.sources.len(), 2);
    assert_eq!(reference.1.obstacles.len(), 1);

    let lignes = [
        "boite 40 30",
        "spore 20 15",
        "substrat 1.0 0.2",
        "croissance 1.0 20.0 30 20",
        "couplage 0.1 0.002",
        "duree 30",
    ];
    let milieu = ["source 10 12 3 2.5", "obstacle 1 2 3 4", "source 30 5 1 4.0"];
    // Le milieu d'abord ; le milieu entremêlé aux autres lignes ; le milieu à la fin après des commentaires.
    let mut textes = vec![format!("{}\n{}\n", milieu.join("\n"), lignes.join("\n"))];
    let mut entremele = String::new();
    for (k, ligne) in lignes.iter().enumerate() {
        entremele.push_str(ligne);
        entremele.push('\n');
        if k < milieu.len() {
            entremele.push_str(milieu[k]);
            entremele.push('\n');
        }
    }
    textes.push(entremele);
    textes.push(format!("# un commentaire\n\n{}\n\n# encore\n{}\n", lignes.join("\n"), milieu.join("\n")));
    // Les deux sources de part et d'autre de l'obstacle : l'ordre relatif des sources reste celui du texte.
    textes.push(format!(
        "{}\nsource 10 12 3 2.5\n{}\nobstacle 1 2 3 4\nsource 30 5 1 4.0\n",
        lignes[..3].join("\n"),
        lignes[3..].join("\n")
    ));
    for (k, texte) in textes.iter().enumerate() {
        let lu = lire_scenario_complet(texte).unwrap_or_else(|e| panic!("texte {k} : {e:?}"));
        assert_eq!(lu, reference, "texte {k} : le résultat dépend de l'ordre des lignes");
    }
}

/// 5 (suite). Une ligne `source` ou `obstacle` mal formée rend `LigneInvalide` avec son numéro (à partir de 1, les
///    lignes vides et les commentaires comptant) : un mot en trop ou en moins, un nombre non entier, négatif ou qui
///    n'est pas un nombre, `x0 > x1`, `y0 > y1`. C'est la première ligne fautive qui est rendue.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_ligne_mal_formee_rend_son_numero() {
    let fautives = [
        "source",
        "source 1 2 3",
        "source 1 2 3 4.0 5",
        "source 1 2 3 1.0 mot",
        "source 1.5 2 3 1.0",
        "source 1 2.5 3 1.0",
        "source 1 2 3.0 1.0",
        "source -1 2 3 1.0",
        "source 1 -2 3 1.0",
        "source 1 2 -3 1.0",
        "source a 2 3 1.0",
        "source 1 2 3 abc",
        "source 1 2 3 1,5",
        "obstacle",
        "obstacle 1 2 3",
        "obstacle 1 2 3 4 5",
        "obstacle 1.5 2 3 4",
        "obstacle 1 2 3 4.0",
        "obstacle -1 2 3 4",
        "obstacle 1 -2 3 4",
        "obstacle 1 2 3 x",
        "obstacle 4 1 3 2",
        "obstacle 1 4 3 2",
        "obstacle 9 9 8 8",
    ];
    for ligne in fautives {
        let texte = avec(&format!("{ligne}\n"));
        assert_eq!(
            lire_scenario_complet(&texte),
            Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)),
            "la ligne {ligne:?} aurait dû être refusée à la ligne {PREMIERE_LIGNE_AJOUTEE}"
        );
    }

    // Les lignes vides et les commentaires comptent dans la numérotation, et la première faute l'emporte.
    let texte = format!("# commentaire\n\n{BASE}obstacle 1 1 2 2\n\nsource 1 2 3\nobstacle 5 1 2 2\n");
    assert_eq!(lire_scenario_complet(&texte), Err(ErreurLecture::LigneInvalide(11)));
    let texte = format!("obstacle 3 3 2 2\n{BASE}source 1 2 3\n");
    assert_eq!(lire_scenario_complet(&texte), Err(ErreurLecture::LigneInvalide(1)));

    // Les bornes égales sont permises (un rectangle d'une case, d'une ligne, d'une colonne).
    for ligne in ["obstacle 4 4 4 4", "obstacle 2 5 9 5", "obstacle 6 1 6 8", "source 0 0 0 0", "source 5 5 0 0.0"] {
        assert!(lire_scenario_complet(&avec(&format!("{ligne}\n"))).is_ok(), "la ligne {ligne:?} est valide");
    }
}

/// 5 (suite). Une quantité négative ou qui n'est pas finie est refusée, à la lecture (`LigneInvalide`) ou à la
///    construction : la spécification ne tranche pas, et ces tests acceptent les deux.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_quantite_negative_ou_non_finie_est_refusee_a_la_lecture_ou_a_la_construction() {
    for quantite in ["-1.0", "-0.5", "-1", "nan", "inf", "-inf"] {
        let texte = avec(&format!("source 10 10 2 {quantite}\n"));
        match lire_scenario_complet(&texte) {
            Err(ErreurLecture::LigneInvalide(n)) => {
                assert_eq!(n, PREMIERE_LIGNE_AJOUTEE, "quantité {quantite} : mauvais numéro de ligne");
            }
            Err(autre) => panic!("quantité {quantite} : erreur inattendue {autre:?}"),
            Ok((reglages, milieu, _)) => {
                assert!(
                    Simulation::nouvelle_avec_milieu(reglages, milieu, 1).is_err(),
                    "quantité {quantite} : acceptée à la lecture et à la construction"
                );
            }
        }
    }
}

/// 5 (suite). `lire_scenario` ne change pas : elle refuse `source` et `obstacle` (ligne invalide), là où
///    `lire_scenario_complet` les lit.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn lire_scenario_refuse_toujours_source_et_obstacle() {
    for ligne in ["source 10 12 3 2.5", "obstacle 1 2 3 4", "source 5 5 0 0", "obstacle 4 4 4 4"] {
        let texte = avec(&format!("{ligne}\n"));
        assert_eq!(
            lire_scenario(&texte),
            Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)),
            "lire_scenario devrait refuser {ligne:?}"
        );
        assert!(lire_scenario_complet(&texte).is_ok(), "lire_scenario_complet devrait lire {ligne:?}");
    }
    // Les autres lignes inconnues restent invalides pour les deux lectures, avec le même numéro.
    for lecture in
        [lire_scenario_complet(&avec("inconnue 1 2\n")).map(|_| ()), lire_scenario(&avec("inconnue 1 2\n")).map(|_| ())]
    {
        assert_eq!(lecture, Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)));
    }
}

/// 5 (suite). Un texte sans milieu donne par `lire_scenario_complet` les mêmes réglages et la même durée que
///    `lire_scenario`, et un milieu vide : le scénario du dépôt, avec et sans anastomose, avec `infini`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_texte_sans_milieu_se_lit_comme_avant() {
    let textes = [
        include_str!("../scenarios/couplage.txt").to_string(),
        BASE.to_string(),
        avec("anastomose 0.5 10\n"),
        "boite 161 161\nspore 80 80\nsubstrat 1.0 0.2\ncroissance 1.0 infini 30 20\ncouplage 0.1 0.002\nduree 80\n"
            .to_string(),
        format!("# commentaire\n\n{BASE}\n"),
    ];
    for (k, texte) in textes.iter().enumerate() {
        let (reglages, duree) = lire_scenario(texte).unwrap_or_else(|e| panic!("texte {k} : lire_scenario : {e:?}"));
        let (reglages_complets, milieu, duree_complete) =
            lire_scenario_complet(texte).unwrap_or_else(|e| panic!("texte {k} : lire_scenario_complet : {e:?}"));
        assert_eq!(reglages_complets, reglages, "texte {k} : réglages");
        assert_eq!(duree_complete, duree, "texte {k} : durée");
        assert_eq!(milieu, Milieu::default(), "texte {k} : le milieu devrait être vide");
        assert!(milieu.sources.is_empty() && milieu.obstacles.is_empty());
    }
}

/// 5 (suite). Les directives obligatoires manquantes rendent toujours `Manquante(nom)` par `lire_scenario_complet`,
///    même quand le texte porte un milieu ; une durée, une boîte… ne sont pas confondues avec lui.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn les_directives_obligatoires_restent_obligatoires() {
    let lignes = [
        ("boite", "boite 40 30"),
        ("spore", "spore 20 15"),
        ("substrat", "substrat 1.0 0.2"),
        ("croissance", "croissance 1.0 20.0 30 20"),
        ("couplage", "couplage 0.1 0.002"),
        ("duree", "duree 30"),
    ];
    for (manquante, _) in lignes {
        let mut texte = String::new();
        for (nom, ligne) in lignes {
            if nom != manquante {
                texte.push_str(ligne);
                texte.push('\n');
            }
        }
        texte.push_str("source 10 12 3 2.5\nobstacle 1 2 3 4\n");
        assert_eq!(lire_scenario_complet(&texte), Err(ErreurLecture::Manquante(manquante)), "sans {manquante}");
    }
}

/// 5 et 6 (suite). Les contrats « dans la boîte » se vérifient à la construction, pas à la lecture : une source ou un
///    obstacle hors de la boîte, ou un obstacle sur la spore, se lit, puis la construction refuse.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn le_contrat_de_la_boite_se_verifie_a_la_construction_pas_a_la_lecture() {
    for ligne in [
        "source 999 5 2 1.0",
        "source 5 30 2 1.0",
        "source 40 0 0 1.0",
        "obstacle 30 3 50 4",
        "obstacle 3 3 5 30",
        "obstacle 20 15 20 15",
        "obstacle 18 14 22 16",
    ] {
        let (reglages, milieu, _) = lire_scenario_complet(&avec(&format!("{ligne}\n"))).unwrap_or_else(|e| {
            panic!("la ligne {ligne:?} devrait se lire (la boîte se vérifie à la construction) : {e:?}")
        });
        assert!(
            Simulation::nouvelle_avec_milieu(reglages, milieu, 1).is_err(),
            "la ligne {ligne:?} devrait être refusée à la construction"
        );
    }
}

/// 5 (suite). De bout en bout : le texte se lit, la simulation se construit avec le milieu lu, `milieu()` le rend, et le
///    substrat porte la tache et l'obstacle.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn le_milieu_lu_se_retrouve_dans_la_simulation() {
    let texte = avec("obstacle 30 2 31 20\nsource 10 12 2 3.0\n");
    let (reglages, milieu, duree) = lire_scenario_complet(&texte).expect("le texte se lit");
    assert_eq!(duree, 30);
    let simulation = Simulation::nouvelle_avec_milieu(reglages, milieu.clone(), 1).expect("le milieu est valide");
    assert_eq!(simulation.milieu(), &milieu);
    assert_eq!(simulation.substrat().valeur(10, 12), 3.0);
    assert_eq!(simulation.substrat().valeur(12, 12), 3.0);
    assert_eq!(simulation.substrat().valeur(13, 12), 1.0);
    assert!(simulation.substrat().est_bloquee(30, 2) && simulation.substrat().est_bloquee(31, 20));
    assert!(!simulation.substrat().est_bloquee(29, 2) && !simulation.substrat().est_bloquee(32, 20));
    assert_eq!(simulation.substrat().valeur(30, 10), 0.0);
}

/// 5 (suite). Les numéros de ligne comptent toutes les lignes du texte, celles du milieu comprises : une ligne
///    classique fautive qui suit des lignes `source` ou `obstacle` (et des commentaires, des lignes vides) rend son
///    vrai numéro, pas son rang parmi les autres directives.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn les_numeros_de_ligne_comptent_les_lignes_du_milieu() {
    let cas = [
        ("source 1 1 1 1.0\nobstacle 1 1 2 2\ninconnue 1 2\n", 3),
        ("source 2 2 1 1.0\n# un commentaire\n\nboite 40\n", 4),
        ("obstacle 3 3 4 4\nsource 5 5 0 2.0\nobstacle 1 1 1 1\nduree abc\n", 4),
        ("# début\nsource 2 2 1 1.0\n\nsource 3 3 1 1.0\nspore 1\n", 5),
    ];
    for (prefixe, numero) in cas {
        // Le préfixe fautif, puis un texte complet et valide : la première ligne fautive est dans le préfixe.
        let texte = format!("{prefixe}{BASE}");
        assert_eq!(lire_scenario_complet(&texte), Err(ErreurLecture::LigneInvalide(numero)), "préfixe {prefixe:?}");
    }
    // Une ligne classique fautive au milieu de lignes valides, après des lignes de milieu entremêlées.
    let texte = "boite 40 30\nsource 1 1 1 1.0\nspore 20 15\nobstacle 1 1 2 2\nsubstrat 1.0 0.2\nbogus\n\
                 croissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nduree 30\n";
    assert_eq!(lire_scenario_complet(texte), Err(ErreurLecture::LigneInvalide(6)));
}

/// 5 (suite). Les espaces multiples, les tabulations, les espaces autour de la ligne et les fins de ligne `\r\n` ne
///    changent rien (c'est le comportement de `lire_scenario` pour les autres directives) ; une directive de milieu
///    mise en commentaire (`# source ...`, `#` en première colonne) est ignorée comme tout commentaire.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn espaces_tabulations_fins_de_ligne_et_directives_commentees() {
    let reference = lire_scenario_complet(&avec("source 10 12 3 2.5\nobstacle 1 2 3 4\n")).expect("référence");
    assert_eq!(reference.1.sources.len(), 1);
    assert_eq!(reference.1.obstacles.len(), 1);
    let texte = format!(
        "# source 5 5 2 1.0\n# obstacle 0 0 1 1\n{BASE}  source\t10   12 3\t2.5  \r\n\tobstacle   1 2\t3 4\r\n\r\n# obstacle 9 9 9 9\r\n"
    );
    assert_eq!(lire_scenario_complet(&texte), Ok(reference));
    // Un texte sans milieu, mis en commentaire : le milieu est vide.
    let texte = format!("{BASE}# source 5 5 2 1.0\n# obstacle 0 0 1 1\n");
    let (_, milieu, _) = lire_scenario_complet(&texte).expect("le texte se lit");
    assert_eq!(milieu, Milieu::default());
}
