//! Tests d'acceptation de l'étape 7 (T2, le thigmotropisme), partie lecture : `lire_scenario_tropismes` et la directive
//! `thigmotropisme <angle>`.
//!
//! Ils fixent : la directive se lit (un nombre, `f64`), optionnelle, au plus une fois, à n'importe quelle place et dans
//! n'importe quel ordre avec `source`, `obstacle` et `chimiotropisme` ; elle ne remplace ni n'altère `chimiotropisme`
//! (les deux ensemble se lisent) ; une ligne mal formée (mot en trop ou en moins, nombre illisible), une seconde ligne
//! `thigmotropisme` (même identique, même à zéro), rendent `LigneInvalide(numéro)`, les lignes vides et les commentaires
//! comptant dans la numérotation ; la valeur n'est pas vérifiée à la lecture (un angle négatif ou supérieur à 90 se lit,
//! la construction le refuse par `TropismeInvalide`) ; `lire_scenario` et `lire_scenario_complet` refusent toujours la
//! ligne ; un texte sans la ligne donne par `lire_scenario_tropismes` le même résultat qu'avant (angle `+0.0`, bits nuls) ;
//! les directives obligatoires restent obligatoires ; espaces, tabulations, fins de ligne `\r\n` et commentaires se
//! traitent comme pour les autres directives ; le scénario du dépôt `scenarios/mur.txt` se lit, porte `thigmotropisme 35`
//! et un obstacle, et se construit ; de bout en bout, l'angle lu agit (un autre état qu'à angle nul, sur un scénario où
//! la colonie touche le mur).
//!
//! Le mot de l'angle se lit par `str::parse::<f64>` de Rust, sans rien de plus (comme pour `chimiotropisme`).
//!
//! Ils laissent ouverts : le texte des erreurs ; l'ordre dans lequel une ligne fautive et une directive manquante sont
//! signalées quand les deux existent.
//!
//! Ils n'utilisent aucune fonction mathématique de `f64`.

#[path = "etape_7/outils.rs"]
mod outils;

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

/// Les deux tropismes, sans littéral complet (une étape à venir pourra ajouter un champ).
#[allow(clippy::needless_update)]
fn deux(chimiotropisme: f64, thigmotropisme: f64) -> Tropismes {
    Tropismes { chimiotropisme, thigmotropisme, ..Tropismes::default() }
}

/// Des tropismes d'angle critique `a`, sans littéral complet.
fn angle(a: f64) -> Tropismes {
    Tropismes { thigmotropisme: a, ..Tropismes::default() }
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

/// `thigmotropisme <angle>` se lit : l'angle exact, entier sans point, décimal, 90, très petit, 360 (lu ici, refusé à la
/// construction) ; les réglages, le milieu, la durée et le chimiotropisme (nul) sont ceux du texte sans la ligne.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn thigmotropisme_se_lit() {
    outils::delai();
    let (reglages, duree) = reglages_de_base();
    for (texte, a) in
        [("35", 35.0), ("0", 0.0), ("35.0", 35.0), ("12.5", 12.5), ("90", 90.0), ("360", 360.0), ("0.001", 0.001)]
    {
        let lu = lire_scenario_tropismes(&avec(&format!("thigmotropisme {texte}\n"))).expect("le texte se lit");
        assert_eq!(lu, (reglages.clone(), Milieu::default(), angle(a), duree), "angle {texte}");
        assert_eq!(lu.2.chimiotropisme.to_bits(), 0, "angle {texte} : le chimiotropisme reste nul");
    }
    // Avec un milieu et le chimiotropisme : tout se lit ensemble.
    let lu =
        lire_scenario_tropismes(&avec("source 10 12 3 2.5\nthigmotropisme 35\nchimiotropisme 2.5\nobstacle 1 2 3 4\n"))
            .expect("lecture");
    let milieu = Milieu { sources: vec![source(10, 12, 3, 2.5)], obstacles: vec![obstacle(1, 2, 3, 4)] };
    assert_eq!(lu, (reglages, milieu, deux(2.5, 35.0), duree));
}

/// La valeur n'est pas vérifiée à la lecture mais à la construction : un angle négatif, supérieur à 90 ou très grand se
/// lit tel quel, puis `nouvelle_avec_tropismes` le refuse ; 0, `-0`, 35, 90 et 89,5 sont lus et acceptés.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_valeur_se_verifie_a_la_construction_pas_a_la_lecture() {
    outils::delai();
    for (texte, a, valide) in [
        ("-1", -1.0, false),
        ("-0.5", -0.5, false),
        ("90.5", 90.5, false),
        ("91", 91.0, false),
        ("360", 360.0, false),
        ("1e300", 1e300, false),
        ("0", 0.0, true),
        ("-0", -0.0, true),
        ("35", 35.0, true),
        ("89.5", 89.5, true),
        ("90", 90.0, true),
        ("90.0", 90.0, true),
    ] {
        let (reglages, milieu, t, _) = lire_scenario_tropismes(&avec(&format!("thigmotropisme {texte}\n")))
            .unwrap_or_else(|e| {
                panic!("angle {texte} : devrait se lire (la valeur se vérifie à la construction) : {e:?}")
            });
        let a: f64 = a;
        assert_eq!(t.thigmotropisme.to_bits(), a.to_bits(), "angle {texte} : valeur lue");
        let construction = Simulation::nouvelle_avec_tropismes(reglages, milieu, t, 1);
        if valide {
            let simulation = construction.unwrap_or_else(|e| panic!("angle {texte} : refusé à tort : {e:?}"));
            assert_eq!(simulation.tropismes(), &angle(a));
        } else {
            assert_eq!(construction.err(), Some(ErreurSimulation::TropismeInvalide), "angle {texte}");
        }
    }
}

/// L'ordre des directives est indifférent, avec `source`, `obstacle` et `chimiotropisme` : `thigmotropisme` en tête, entre
/// les autres lignes, à la fin, après des commentaires et des lignes vides ; le résultat est le même.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn l_ordre_des_directives_est_indifferent() {
    outils::delai();
    let reference =
        lire_scenario_tropismes(&avec("source 10 12 3 2.5\nobstacle 1 2 3 4\nchimiotropisme 2.5\nthigmotropisme 35\n"))
            .expect("lecture de référence");
    assert_eq!(reference.2, deux(2.5, 35.0));
    let lignes = [
        "boite 40 30",
        "spore 20 15",
        "substrat 1.0 0.2",
        "croissance 1.0 20.0 30 20",
        "couplage 0.1 0.002",
        "duree 30",
    ];
    let extras = ["source 10 12 3 2.5", "obstacle 1 2 3 4", "chimiotropisme 2.5"];
    let textes = [
        // L'angle d'abord.
        format!("thigmotropisme 35\n{}\n{}\n", extras.join("\n"), lignes.join("\n")),
        // L'angle entremêlé aux autres lignes.
        format!(
            "{}\n{}\nthigmotropisme 35\n{}\n{}\n{}\n{}\n{}\n{}\n",
            lignes[0], extras[2], lignes[1], lignes[2], extras[0], lignes[3], lignes[4], lignes[5]
        ) + &format!("{}\n", extras[1]),
        // L'angle en dernier, après des commentaires et des lignes vides.
        format!("# un commentaire\n\n{}\n\n# encore\n{}\n\nthigmotropisme 35\n", lignes.join("\n"), extras.join("\n")),
        // L'angle avant le chimiotropisme, au milieu des extras.
        format!("{}\n{}\nthigmotropisme 35\n{}\n{}\n", lignes.join("\n"), extras[0], extras[2], extras[1]),
    ];
    for (k, texte) in textes.iter().enumerate() {
        let lu = lire_scenario_tropismes(texte).unwrap_or_else(|e| panic!("texte {k} : {e:?}"));
        assert_eq!(lu, reference, "texte {k} : le résultat dépend de l'ordre des lignes");
    }
}

/// Une ligne `thigmotropisme` mal formée rend `LigneInvalide` avec son numéro : mot en trop ou en moins, nombre illisible
/// (lettres, virgule décimale, suffixe, hexadécimal, deux points, double signe), nom de directive à la mauvaise casse ou
/// suivi de deux points ou mal écrit (`thigmo`, `thigmotropismes`, `thygmotropisme`).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn une_ligne_mal_formee_rend_son_numero() {
    outils::delai();
    let fautives = [
        "thigmotropisme",
        "thigmotropisme 35 2",
        "thigmotropisme 35 mot",
        "thigmotropisme abc",
        "thigmotropisme trente",
        "thigmotropisme 35,5",
        "thigmotropisme 35deg",
        "thigmotropisme 0x23",
        "thigmotropisme 1.5.2",
        "thigmotropisme --1",
        "thigmotropisme ''",
        "Thigmotropisme 35",
        "thigmotropisme: 35",
        "thigmotropismes 35",
        "thigmo 35",
        "thygmotropisme 35",
    ];
    for ligne in fautives {
        assert_eq!(
            lire_scenario_tropismes(&avec(&format!("{ligne}\n"))),
            Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)),
            "la ligne {ligne:?} aurait dû être refusée à la ligne {PREMIERE_LIGNE_AJOUTEE}"
        );
    }
    // Les lignes vides et les commentaires comptent dans la numérotation, et la première faute l'emporte.
    let texte = format!("# commentaire\n\n{BASE}thigmotropisme abc\n\nbogus\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(9)));
    let texte = format!("thigmotropisme 1 2\n{BASE}source 1 2 3\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(1)));
    let texte = format!("source 1 1 1 1.0\n{BASE}bogus\nthigmotropisme abc\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(8)));
    // Une ligne fautive d'une autre directive reste fautive avec un angle valide, à son numéro.
    let texte = format!("thigmotropisme 35\n{BASE}source 1 2 3\n");
    assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::LigneInvalide(8)));
}

/// Une seconde ligne `thigmotropisme` rend `LigneInvalide` avec le numéro de la seconde, qu'elle ait la même valeur ou
/// non, voisine ou séparée par d'autres lignes, même quand les deux valent zéro. Une seule ligne par directive : un
/// `chimiotropisme` et un `thigmotropisme` ne se comptent pas ensemble.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn une_seconde_ligne_thigmotropisme_est_refusee() {
    outils::delai();
    for (supplement, numero) in [
        ("thigmotropisme 35\nthigmotropisme 20\n", 8),
        ("thigmotropisme 35\nthigmotropisme 35\n", 8),
        ("thigmotropisme 0\nthigmotropisme 0\n", 8),
        ("thigmotropisme 0\nthigmotropisme 35\n", 8),
        ("thigmotropisme 35\n\n# commentaire\nsource 1 1 1 1.0\nthigmotropisme 35\n", 11),
        ("thigmotropisme 1\nobstacle 1 2 3 4\nthigmotropisme 3\nsource 2 2 1 1.0\nthigmotropisme 4\n", 9),
    ] {
        assert_eq!(
            lire_scenario_tropismes(&avec(supplement)),
            Err(ErreurLecture::LigneInvalide(numero)),
            "texte {supplement:?} : la seconde ligne devrait être refusée à la ligne {numero}"
        );
    }
    // Une seule ligne, même répétée sous forme de commentaire, est permise ; et chaque directive a sa propre ligne.
    assert!(lire_scenario_tropismes(&avec("thigmotropisme 35\n# thigmotropisme 20\n")).is_ok());
    assert!(lire_scenario_tropismes(&avec("thigmotropisme 35\nchimiotropisme 5\n")).is_ok());
    assert_eq!(
        lire_scenario_tropismes(&avec("thigmotropisme 35\nchimiotropisme 5\nchimiotropisme 5\n")),
        Err(ErreurLecture::LigneInvalide(9))
    );
}

/// `lire_scenario` et `lire_scenario_complet` ne changent pas : elles refusent toujours `thigmotropisme` (ligne invalide, au
/// bon numéro, y compris au milieu de lignes de milieu), là où `lire_scenario_tropismes` la lit.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn lire_scenario_et_lire_scenario_complet_refusent_toujours_la_ligne() {
    outils::delai();
    for ligne in ["thigmotropisme 35", "thigmotropisme 0", "thigmotropisme 360"] {
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
    let texte = avec("source 10 12 3 2.5\nthigmotropisme 35\nobstacle 1 2 3 4\n");
    assert_eq!(lire_scenario_complet(&texte), Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE + 1)));
    assert_eq!(lire_scenario(&texte), Err(ErreurLecture::LigneInvalide(PREMIERE_LIGNE_AJOUTEE)));
    assert!(lire_scenario_tropismes(&texte).is_ok());
}

/// Un texte sans la ligne donne par `lire_scenario_tropismes` les mêmes réglages, le même milieu et la même durée que
/// `lire_scenario_complet`, et `Tropismes::default()` (angle `+0.0`, bits nuls) : le scénario du dépôt, avec et sans
/// anastomose, avec `infini`, avec des commentaires, avec un milieu.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn un_texte_sans_la_ligne_se_lit_comme_avant() {
    outils::delai();
    let textes = [
        include_str!("../scenarios/couplage.txt").to_string(),
        include_str!("../scenarios/terrain.txt").to_string(),
        BASE.to_string(),
        avec("anastomose 0.5 10\n"),
        "boite 161 161\nspore 80 80\nsubstrat 1.0 0.2\ncroissance 1.0 infini 30 20\ncouplage 0.1 0.002\nduree 80\n"
            .to_string(),
        format!("# commentaire\n\n{BASE}\n"),
        avec("source 10 12 3 2.5\nobstacle 1 2 3 4\nanastomose 0.5 10\n"),
        avec("# thigmotropisme 35\nsource 5 5 0 1.0\n"),
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
            t.thigmotropisme.to_bits(),
            0,
            "texte {k} : angle sans la ligne : attendu +0.0, obtenu {:?}",
            t.thigmotropisme
        );
    }
}

/// Les directives obligatoires manquantes rendent toujours `Manquante(nom)` par `lire_scenario_tropismes`, même quand le
/// texte porte un milieu et des tropismes ; la seule ligne `thigmotropisme` ne remplace aucune directive obligatoire.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_directives_obligatoires_restent_obligatoires() {
    outils::delai();
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
        texte.push_str("source 10 12 3 2.5\nthigmotropisme 35\nchimiotropisme 2\nobstacle 1 2 3 4\n");
        assert_eq!(lire_scenario_tropismes(&texte), Err(ErreurLecture::Manquante(manquante)), "sans {manquante}");
    }
    let seule = lire_scenario_tropismes("thigmotropisme 35\n");
    assert!(matches!(seule, Err(ErreurLecture::Manquante(_))), "{seule:?}");
}

/// Les espaces multiples, les tabulations, les espaces autour de la ligne et les fins de ligne `\r\n` ne changent rien (comme
/// pour les autres directives) ; une directive mise en commentaire (`#` en première colonne) est ignorée : l'angle reste nul.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn espaces_tabulations_fins_de_ligne_et_directive_commentee() {
    outils::delai();
    let reference = lire_scenario_tropismes(&avec("thigmotropisme 35\nsource 10 12 3 2.5\n")).expect("référence");
    assert_eq!(reference.2, angle(35.0));
    let texte = format!(
        "# thigmotropisme 9\n{BASE}  thigmotropisme\t  35  \r\n\tsource   10 12\t3 2.5\r\n\r\n# thigmotropisme 7\r\n"
    );
    assert_eq!(lire_scenario_tropismes(&texte), Ok(reference));
    let (_, _, t, _) = lire_scenario_tropismes(&format!("{BASE}# thigmotropisme 35\n")).expect("le texte se lit");
    assert_eq!(t, Tropismes::default());
}

/// Le mot de l'angle se lit par `str::parse::<f64>` de Rust, tel quel : `nan`, `inf`, `-inf`, `infinity`, `+35`, `3.5e1`,
/// `.5` et `5.` se lisent (la construction refuse ensuite `NaN` et les infinis par `TropismeInvalide`, accepte 35, 35, 0,5 et
/// 5) ; un mot que `parse::<f64>` refuse est une ligne invalide.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_mot_de_l_angle_se_lit_comme_parse_f64() {
    outils::delai();
    for mot in ["nan", "NaN", "inf", "-inf", "infinity", "+35", "3.5e1", ".5", "5.", "2.5E-1"] {
        let attendu: f64 = mot.parse().expect("garde : parse::<f64> accepte ce mot");
        let (reglages, milieu, t, _) = lire_scenario_tropismes(&avec(&format!("thigmotropisme {mot}\n")))
            .unwrap_or_else(|e| panic!("thigmotropisme {mot} : attendu lu (parse::<f64> l'accepte), obtenu {e:?}"));
        assert_eq!(
            t.thigmotropisme.to_bits(),
            attendu.to_bits(),
            "thigmotropisme {mot} : angle lu {:?}, attendu {attendu:?}",
            t.thigmotropisme
        );
        let construction = Simulation::nouvelle_avec_tropismes(reglages, milieu, t, 1);
        if attendu.is_finite() {
            assert!(construction.is_ok(), "thigmotropisme {mot} : angle {attendu} valide, refusé");
        } else {
            assert_eq!(construction.err(), Some(ErreurSimulation::TropismeInvalide), "thigmotropisme {mot}");
        }
    }
}

/// Le scénario du dépôt `scenarios/mur.txt` (étape T2) : il se lit avec `lire_scenario_tropismes`, porte `thigmotropisme 35` et
/// exactement un obstacle, et la spore est à gauche de cet obstacle ; il dure entre 100 et 150 pas ; il se construit. Il est
/// refusé par `lire_scenario_complet` à cause de la ligne `thigmotropisme` (un scénario couplé de T2).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_scenario_mur_du_depot_se_lit_et_se_construit() {
    outils::delai();
    let texte = include_str!("../scenarios/mur.txt");
    let (reglages, milieu, t, duree) = lire_scenario_tropismes(texte).expect("scenarios/mur.txt se lit");
    assert_eq!(t, angle(35.0), "scenarios/mur.txt : thigmotropisme 35");
    assert_eq!(milieu.obstacles.len(), 1, "scenarios/mur.txt : un mur");
    assert!((100..=150).contains(&duree), "scenarios/mur.txt : durée {duree} entre 100 et 150");
    let mur = &milieu.obstacles[0];
    assert!(reglages.spore.0 < mur.x0, "la spore doit être à gauche du mur");
    assert!(mur.y0 <= reglages.spore.1 && reglages.spore.1 <= mur.y1, "le mur doit être devant la spore");
    assert!(mur.y0 > 0 && mur.y1 + 1 < reglages.hauteur, "le mur doit pouvoir être contourné");
    assert!(Simulation::nouvelle_avec_tropismes(reglages, milieu, t, 1).is_ok());
    assert_eq!(
        lire_scenario_complet(texte),
        Err(ErreurLecture::LigneInvalide(texte.lines().position(|l| l.starts_with("thigmotropisme")).unwrap() + 1))
    );
}

/// De bout en bout : le texte se lit, la simulation se construit avec les tropismes lus, `tropismes()` et `milieu()` les
/// rendent, et la colonie avance ; sur `scenarios/mur.txt`, l'angle lu agit : pour 4 graines au moins sur les 10 premières, les
/// segments de la colonie ne sont pas ceux du même texte sans la ligne (angle nul), parce que la colonie touche le mur. (On
/// compare les segments, pas l'empreinte : l'écriture canonique change dès que l'angle est non nul, même sans contact.)
/// Mesuré sur le prototype : 6 graines sur 10 (les autres colonies s'éteignent ou n'atteignent pas le mur en 120 pas).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_scenario_lu_se_retrouve_dans_la_simulation() {
    outils::delai();
    let texte = include_str!("../scenarios/mur.txt");
    let (reglages, milieu, t, duree) = lire_scenario_tropismes(texte).expect("le texte se lit");
    let simulation = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), t, 1).expect("valide");
    assert_eq!(simulation.tropismes(), &angle(35.0));
    assert_eq!(simulation.milieu(), &milieu);
    let sans_la_ligne = texte.replace("thigmotropisme 35\n", "");
    let (r0, m0, t0, d0) = lire_scenario_tropismes(&sans_la_ligne).expect("le texte sans la ligne se lit");
    assert_eq!(t0, Tropismes::default());
    assert_eq!(d0, duree);
    let mut qui_different = 0;
    for graine in 1..=10 {
        let mut avec_angle =
            Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), t, graine).expect("valide");
        avec_angle.avancer_n(duree);
        let mut a_zero = Simulation::nouvelle_avec_tropismes(r0.clone(), m0.clone(), t0, graine).expect("valide");
        a_zero.avancer_n(d0);
        if avec_angle.segments() != a_zero.segments() {
            qui_different += 1;
        }
    }
    assert!(qui_different >= 4, "l'angle lu n'a changé la colonie que pour {qui_different} graines sur 10");
}
