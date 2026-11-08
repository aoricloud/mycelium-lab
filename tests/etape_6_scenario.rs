//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie lecture : `lire_scenario_tropismes` et la directive
//! `chimiotropisme <force>`.
//!
//! Ils fixent : la directive se lit (un nombre, `f64`), optionnelle, au plus une fois, à n'importe quelle place et dans
//! n'importe quel ordre avec `source` et `obstacle` ; une ligne mal formée (mot en trop ou en moins, nombre illisible), une
//! seconde ligne `chimiotropisme` (même identique), rendent `LigneInvalide(numéro)`, les lignes vides et les commentaires
//! comptant dans la numérotation ; la valeur n'est pas vérifiée à la lecture (une force négative ou supérieure à 45 se
//! lit, la construction la refuse) ; `lire_scenario` et `lire_scenario_complet` refusent toujours la ligne ; un texte sans
//! la ligne donne par `lire_scenario_tropismes` les mêmes réglages, le même milieu et la même durée que
//! `lire_scenario_complet`, et `Tropismes::default()` ; les directives obligatoires restent obligatoires ; espaces,
//! tabulations, fins de ligne `\r\n` et commentaires se traitent comme pour les autres directives.
//!
//! Le mot de la force se lit par `str::parse::<f64>` de Rust, sans rien de plus : ce qu'elle accepte se lit (`nan`, `inf`,
//! `-inf`, `infinity`, `+2`, `1e1`, `.5`), la construction refuse ensuite ce qui est hors de `0..=45` ; ce qu'elle refuse
//! est une ligne invalide. Sans la ligne, les tropismes sont exactement `Tropismes::default()` (force `0.0`, bits nuls).
//!
//! Ils laissent ouverts : le texte des erreurs ; l'ordre dans lequel une ligne fautive et une directive manquante sont
//! signalées quand les deux existent (comme avant T1).
//!
//! Ils n'utilisent aucune fonction mathématique de `f64`.

use mycelium_lab::simulation::{
    ErreurLecture, ErreurSimulation, Milieu, Obstacle, Reglages, Simulation, Source, lire_scenario,
    lire_scenario_complet, lire_scenario_tropismes,
};
use mycelium_lab::tropisme::Tropismes;

/// Un scénario couplé sans milieu ni tropisme : six lignes, donc la ligne suivante est la 7.
const BASE: &str =
    "boite 40 30\nspore 20 15\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nduree 30\n";

/// Le numéro de la première ligne ajoutée à `BASE`.
const PREMIERE_LIGNE_AJOUTEE: usize = 7;

fn avec(supplement: &str) -> String {
    format!("{BASE}{supplement}")
}

/// Des tropismes de force `f`, sans littéral complet : T2 ajoutera un champ à `Tropismes`, et `..Tropismes::default()` garde
/// les tests valables. Tant qu'il n'y a qu'un champ, clippy trouve la mise à jour inutile (`needless_update`) : l'`allow`
/// tombera avec T2.
#[allow(clippy::needless_update)]
fn tropismes(f: f64) -> Tropismes {
    Tropismes { chimiotropisme: f, ..Tropismes::default() }
}

fn source(x: usize, y: usize, rayon: usize, quantite: f64) -> Source {
    Source { x, y, rayon, quantite }
}

fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

/// Les réglages et la durée de `BASE`, lus par `lire_scenario`.
fn reglages_de_base() -> (Reglages, u32) {
    lire_scenario(BASE).expect("le texte de base se lit")
}

/// 8. `chimiotropisme <force>` se lit : la force exacte, un nombre entier sans point, décimal, 360 (lu ici, refusé à la construction), très
///    petit ; les réglages, le milieu et la durée sont ceux du texte sans la ligne.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn chimiotropisme_se_lit() {
    let (reglages, duree) = reglages_de_base();
    for (texte, force) in
        [("2.5", 2.5), ("0", 0.0), ("3", 3.0), ("0.125", 0.125), ("360", 360.0), ("0.001", 0.001), ("100.0", 100.0)]
    {
        let lu = lire_scenario_tropismes(&avec(&format!("chimiotropisme {texte}\n"))).expect("le texte se lit");
        assert_eq!(lu, (reglages.clone(), Milieu::default(), tropismes(force), duree), "force {texte}");
    }
    // Avec un milieu : tout se lit ensemble.
    let lu =
        lire_scenario_tropismes(&avec("source 10 12 3 2.5\nchimiotropisme 2.5\nobstacle 1 2 3 4\n")).expect("lecture");
    let milieu = Milieu { sources: vec![source(10, 12, 3, 2.5)], obstacles: vec![obstacle(1, 2, 3, 4)] };
    assert_eq!(lu, (reglages, milieu, tropismes(2.5), duree));
}

/// 8. La valeur n'est pas vérifiée à la lecture mais à la construction : une force négative, supérieure à 45 (360
///    compris) ou très grande se lit telle quelle, puis `nouvelle_avec_tropismes` la refuse ; 0 et 45 sont lues et
///    acceptées.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_valeur_se_verifie_a_la_construction_pas_a_la_lecture() {
    for (texte, force, valide) in [
        ("-1", -1.0, false),
        ("-0.5", -0.5, false),
        ("361", 361.0, false),
        ("1000000", 1_000_000.0, false),
        ("1e300", 1e300, false),
        ("0", 0.0, true),
        ("-0", -0.0, true),
        ("360", 360.0, false),
        ("45.5", 45.5, false),
        ("46", 46.0, false),
        ("45", 45.0, true),
        ("44.75", 44.75, true),
        ("2.5", 2.5, true),
    ] {
        let (reglages, milieu, t, _) = lire_scenario_tropismes(&avec(&format!("chimiotropisme {texte}\n")))
            .unwrap_or_else(|e| {
                panic!("force {texte} : devrait se lire (la valeur se vérifie à la construction) : {e:?}")
            });
        assert_eq!(t, tropismes(force), "force {texte} : valeur lue");
        let construction = Simulation::nouvelle_avec_tropismes(reglages, milieu, t, 1);
        if valide {
            let simulation = construction.unwrap_or_else(|e| panic!("force {texte} : refusée à tort : {e:?}"));
            assert_eq!(simulation.tropismes(), &tropismes(force));
        } else {
            assert_eq!(construction.err(), Some(ErreurSimulation::TropismeInvalide), "force {texte}");
        }
    }
}

/// 8. L'ordre des directives est indifférent, avec `source` et `obstacle` : `chimiotropisme` en tête, entre les autres lignes,
///    à la fin, après des commentaires et des lignes vides ; le résultat est le même.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn l_ordre_des_directives_est_indifferent() {
    let reference =
        lire_scenario_tropismes(&avec("source 10 12 3 2.5\nobstacle 1 2 3 4\nsource 30 5 1 4.0\nchimiotropisme 2.5\n"))
            .expect("lecture de référence");
    assert_eq!(reference.2, tropismes(2.5));
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
    let extras = ["source 10 12 3 2.5", "obstacle 1 2 3 4", "source 30 5 1 4.0"];
    let mut textes = vec![
        // La force d'abord.
        format!("chimiotropisme 2.5\n{}\n{}\n", extras.join("\n"), lignes.join("\n")),
        // Le milieu et la force entremêlés aux autres lignes.
        format!(
            "{}\nchimiotropisme 2.5\n{}\n{}\n{}\n{}\n{}\n",
            lignes[0], extras[0], lignes[1], lignes[2], lignes[3], lignes[4]
        ) + &format!("{}\n{}\n{}\n", extras[1], lignes[5], extras[2]),
        // La force en dernier, après des commentaires et des lignes vides.
        format!("# un commentaire\n\n{}\n\n# encore\n{}\n\nchimiotropisme 2.5\n", lignes.join("\n"), extras.join("\n")),
        // La force au milieu des sources.
        format!("{}\n{}\nchimiotropisme 2.5\n{}\n{}\n", lignes.join("\n"), extras[0], extras[1], extras[2]),
    ];
    // Les autres ordres de trois lignes `source`/`obstacle` gardent l'ordre relatif des sources.
    textes.push(format!(
        "{}\nsource 10 12 3 2.5\nchimiotropisme 2.5\n{}\nobstacle 1 2 3 4\nsource 30 5 1 4.0\n",
        lignes[..3].join("\n"),
        lignes[3..].join("\n")
    ));
    for (k, texte) in textes.iter().enumerate() {
        let lu = lire_scenario_tropismes(texte).unwrap_or_else(|e| panic!("texte {k} : {e:?}"));
        assert_eq!(lu, reference, "texte {k} : le résultat dépend de l'ordre des lignes");
    }
}

/// 8. Une ligne `chimiotropisme` mal formée rend `LigneInvalide` avec son numéro : mot en trop ou en moins, nombre
///    illisible (lettres, virgule décimale, suffixe, hexadécimal, deux points, double signe), nom de directive à la
///    mauvaise casse ou suivi de deux points.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn une_ligne_mal_formee_rend_son_numero() {
    let fautives = [
        "chimiotropisme",
        "chimiotropisme 1 2",
        "chimiotropisme 1 mot",
        "chimiotropisme abc",
        "chimiotropisme un",
        "chimiotropisme 1,5",
        "chimiotropisme 2.5x",
        "chimiotropisme 0x10",
        "chimiotropisme 1.5.2",
        "chimiotropisme --1",
        "chimiotropisme ''",
        "Chimiotropisme 2",
        "chimiotropisme: 2",
        "chimiotropismes 2",
        "chimio 2",
    ];
    for ligne in fautives {
        assert_eq!(
            lire_scenario_tropismes(&avec(&format!("{ligne}\n"))),
            Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)),
            "la ligne {ligne:?} aurait dû être refusée à la ligne {PREMIERE_LIGNE_AJOUTEE}"
        );
    }
    // Les lignes vides et les commentaires comptent dans la numérotation, et la première faute l'emporte.
    let texte = format!("# commentaire\n\n{BASE}chimiotropisme abc\n\nbogus\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(9)));
    let texte = format!("chimiotropisme 1 2\n{BASE}source 1 2 3\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(1)));
    let texte = format!("source 1 1 1 1.0\n{BASE}bogus\nchimiotropisme abc\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(8)));
    // Une ligne fautive d'une autre directive reste fautive avec une force valide, à son numéro.
    let texte = format!("chimiotropisme 2\n{BASE}source 1 2 3\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(8)));
}

/// 8. Une seconde ligne `chimiotropisme` rend `LigneInvalide` avec le numéro de la seconde, qu'elle ait la même valeur
///    ou non, qu'elle soit voisine ou séparée par d'autres lignes, et même quand les deux valent zéro.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn une_seconde_ligne_chimiotropisme_est_refusee() {
    for (supplement, numero) in [
        ("chimiotropisme 1\nchimiotropisme 2\n", 8),
        ("chimiotropisme 1\nchimiotropisme 1\n", 8),
        ("chimiotropisme 0\nchimiotropisme 0\n", 8),
        ("chimiotropisme 0\nchimiotropisme 2.5\n", 8),
        ("chimiotropisme 2.5\n\n# commentaire\nsource 1 1 1 1.0\nchimiotropisme 2.5\n", 11),
        ("chimiotropisme 1\nobstacle 1 2 3 4\nchimiotropisme 3\nsource 2 2 1 1.0\nchimiotropisme 4\n", 9),
    ] {
        assert_eq!(
            lire_scenario_tropismes(&avec(supplement)),
            Err(ErreurLecture::LigneInvalide(numero)),
            "texte {supplement:?} : la seconde ligne devrait être refusée à la ligne {numero}"
        );
    }
    // Une seule ligne, même répétée sous forme de commentaire, est permise.
    assert!(lire_scenario_tropismes(&avec("chimiotropisme 1\n# chimiotropisme 2\n")).is_ok());
}

/// 8. `lire_scenario` et `lire_scenario_complet` ne changent pas : elles refusent toujours `chimiotropisme` (ligne
///    invalide, au bon numéro, y compris au milieu de lignes de milieu), là où `lire_scenario_tropismes` la lit.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn lire_scenario_et_lire_scenario_complet_refusent_toujours_la_ligne() {
    for ligne in ["chimiotropisme 2.5", "chimiotropisme 0", "chimiotropisme 360"] {
        let texte = avec(&format!("{ligne}\n"));
        assert_eq!(
            lire_scenario(&texte),
            Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)),
            "lire_scenario devrait refuser {ligne:?}"
        );
        assert_eq!(
            lire_scenario_complet(&texte),
            Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)),
            "lire_scenario_complet devrait refuser {ligne:?}"
        );
        assert!(lire_scenario_tropismes(&texte).is_ok(), "lire_scenario_tropismes devrait lire {ligne:?}");
    }
    // Au milieu de lignes de milieu : `lire_scenario_complet` lit le milieu et refuse la force, à son numéro.
    let texte = avec("source 10 12 3 2.5\nchimiotropisme 2.5\nobstacle 1 2 3 4\n");
    assert_eq!(lire_scenario_complet(&texte), Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE + 1)));
    assert_eq!(lire_scenario(&texte), Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)));
    assert!(lire_scenario_tropismes(&texte).is_ok());
}

/// 8. Un texte sans la ligne donne par `lire_scenario_tropismes` les mêmes réglages, le même milieu et la même durée que
///    `lire_scenario_complet`, et `Tropismes::default()` : le scénario du dépôt, avec et sans anastomose, avec `infini`,
///    avec des commentaires, avec un milieu.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_texte_sans_la_ligne_se_lit_comme_lire_scenario_complet() {
    let textes = [
        include_str!("../scenarios/couplage.txt").to_string(),
        BASE.to_string(),
        avec("anastomose 0.5 10\n"),
        "boite 161 161\nspore 80 80\nsubstrat 1.0 0.2\ncroissance 1.0 infini 30 20\ncouplage 0.1 0.002\nduree 80\n"
            .to_string(),
        format!("# commentaire\n\n{BASE}\n"),
        avec("source 10 12 3 2.5\nobstacle 1 2 3 4\nanastomose 0.5 10\n"),
        avec("# chimiotropisme 2.5\nsource 5 5 0 1.0\n"),
    ];
    for (k, texte) in textes.iter().enumerate() {
        let (reglages, milieu, duree) =
            lire_scenario_complet(texte).unwrap_or_else(|e| panic!("texte {k} : lire_scenario_complet : {e:?}"));
        let (r, m, t, d) =
            lire_scenario_tropismes(texte).unwrap_or_else(|e| panic!("texte {k} : lire_scenario_tropismes : {e:?}"));
        assert_eq!(r, reglages, "texte {k} : réglages");
        assert_eq!(m, milieu, "texte {k} : milieu");
        assert_eq!(d, duree, "texte {k} : durée");
        assert_eq!(t, Tropismes::default(), "texte {k} : tropismes");
        assert_eq!(
            t.chimiotropisme.to_bits(),
            0,
            "texte {k} : force sans la ligne : attendu +0.0, obtenu {:?}",
            t.chimiotropisme
        );
    }
}

/// 8. Les directives obligatoires manquantes rendent toujours `Manquante(nom)` par `lire_scenario_tropismes`, même quand
///    le texte porte un milieu et un tropisme.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
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
        texte.push_str("source 10 12 3 2.5\nchimiotropisme 2.5\nobstacle 1 2 3 4\n");
        assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::Manquante(manquante)), "sans {manquante}");
    }
    // La seule ligne `chimiotropisme` ne remplace aucune directive obligatoire.
    let seule = lire_scenario_tropismes("chimiotropisme 2\n");
    assert!(matches!(seule, Err(ErreurLecture::Manquante(_))), "{seule:?}");
}

/// 8. Les espaces multiples, les tabulations, les espaces autour de la ligne et les fins de ligne `\r\n` ne changent
///    rien (comme pour les autres directives) ; une directive mise en commentaire (`#` en première colonne) est ignorée :
///    les tropismes restent ceux par défaut.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn espaces_tabulations_fins_de_ligne_et_directive_commentee() {
    let reference = lire_scenario_tropismes(&avec("chimiotropisme 2.5\nsource 10 12 3 2.5\n")).expect("référence");
    assert_eq!(reference.2, tropismes(2.5));
    let texte = format!(
        "# chimiotropisme 9\n{BASE}  chimiotropisme\t  2.5  \r\n\tsource   10 12\t3 2.5\r\n\r\n# chimiotropisme 7\r\n"
    );
    assert_eq!(lire_scenario_tropismes(&texte), Ok(reference));
    // Une directive en commentaire seule : tropismes par défaut.
    let (_, _, t, _) = lire_scenario_tropismes(&format!("{BASE}# chimiotropisme 2.5\n")).expect("le texte se lit");
    assert_eq!(t, Tropismes::default());
}

/// 8. De bout en bout : le texte se lit, la simulation se construit avec les tropismes lus, `tropismes()` et `milieu()` les
///    rendent, et la colonie avance (la force lue agit : un autre état qu'à force nulle).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_scenario_lu_se_retrouve_dans_la_simulation() {
    let texte = avec("obstacle 30 2 31 20\nchimiotropisme 2.5\nsource 25 12 3 3.0\n");
    let (reglages, milieu, t, duree) = lire_scenario_tropismes(&texte).expect("le texte se lit");
    assert_eq!(duree, 30);
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), t, 1).expect("valide");
    assert_eq!(simulation.tropismes(), &tropismes(2.5));
    assert_eq!(simulation.milieu(), &milieu);
    simulation.avancer_n(duree);
    let sans_la_ligne = texte.replace("chimiotropisme 2.5\n", "");
    let (r0, m0, t0, d0) = lire_scenario_tropismes(&sans_la_ligne).expect("le texte sans la ligne se lit");
    assert_eq!(t0, Tropismes::default());
    let mut a_zero = Simulation::nouvelle_avec_tropismes(r0, m0, t0, 1).expect("valide");
    a_zero.avancer_n(d0);
    assert_ne!(simulation.empreinte(), a_zero.empreinte(), "la force lue n'a rien changé à l'état");
}

/// 8. Le mot de la force se lit par `str::parse::<f64>` de Rust, tel quel : `nan`, `NaN`, `inf`, `-inf`, `infinity`, `+2`,
///    `1e1`, `.5` et `5.` se lisent (la construction refuse ensuite `NaN` et les infinis par `TropismeInvalide`, accepte
///    2, 10, 0,5 et 5) ; un mot que `parse::<f64>` refuse est une ligne invalide (voir `une_ligne_mal_formee_rend_son_numero`).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_mot_de_la_force_se_lit_comme_parse_f64() {
    for mot in ["nan", "NaN", "inf", "-inf", "infinity", "+2", "1e1", ".5", "5.", "2.5E-1"] {
        let attendu: f64 = mot.parse().expect("garde : parse::<f64> accepte ce mot");
        let (reglages, milieu, t, _) = lire_scenario_tropismes(&avec(&format!("chimiotropisme {mot}\n")))
            .unwrap_or_else(|e| panic!("chimiotropisme {mot} : attendu lu (parse::<f64> l'accepte), obtenu {e:?}"));
        assert_eq!(
            t.chimiotropisme.to_bits(),
            attendu.to_bits(),
            "chimiotropisme {mot} : force lue {:?}, attendue {attendu:?}",
            t.chimiotropisme
        );
        let construction = Simulation::nouvelle_avec_tropismes(reglages, milieu, t, 1);
        if attendu.is_finite() {
            assert!(construction.is_ok(), "chimiotropisme {mot} : force {attendu} valide, refusée");
        } else {
            assert_eq!(construction.err(), Some(ErreurSimulation::TropismeInvalide), "chimiotropisme {mot}");
        }
    }
}
