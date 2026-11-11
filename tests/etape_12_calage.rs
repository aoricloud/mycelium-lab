//! Étape 12 (M2), partie « les fonctions de `calage` et la documentation » : ce qui se calcule exactement, sans jouer
//! de simulation.
//!
//! - l'échelle (7 µm la case, un choix écrit) et la longueur par pointe (`biomasse / pointes`, `None` sans pointe) ;
//! - le taux de croissance : exact à 1e-9 sur des suites géométriques (taux 0,05 et 0,6), les deux bouts de la plage
//!   inclus et rien de plus, les moindres carrés (pas la pente des deux bouts), la formule écrite à part, ses refus ;
//! - le temps de doublement (`ln 2 / taux`, les cinq temps du tableau I de Trinci) et l'écart relatif des taux (le taux
//!   de la biomasse au dénominateur : 0,04 / 0,34 = 0,118, le plus grand écart du tableau I) ;
//! - la fenêtre de croissance : des cas écrits à la main et 4 000 suites au hasard contre une machine à états ;
//! - la longueur par pointe moyenne d'une fenêtre ;
//! - l'hygiène des sources (ni fonction de `f64` interdite ni `HashMap`, le noyau ne connaît pas `calage`) ;
//! - la section de `MODELE.md` : les sources et leur statut, les chiffres publiés, les nombres mesurés, l'échec dit
//!   tel quel.

#[path = "etape_12/outils.rs"]
mod outils;

use std::f64::consts::{E, LN_2};
use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::calage::*;
use outils::{Hasard, fenetre_oracle, pointes, releves, taux_oracle};

/// Vrai si `f` panique.
fn panique<R>(f: impl FnOnce() -> R) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

/// `depart × rapport^t` pour t = 0..n, par multiplications successives.
fn geometrique(depart: f64, rapport: f64, n: usize) -> Vec<f64> {
    (0..n).scan(depart, |v, _| Some(std::mem::replace(v, *v * rapport))).collect()
}

/// `e^0,05` et `e^0,6`, écrits en toutes lettres (aucune exponentielle dans le dépôt).
const E_0_05: f64 = 1.0512710963760241;
const E_0_6: f64 = 1.8221188003905089;

fn lire(chemin: &str) -> String {
    std::fs::read_to_string(format!("{}/{chemin}", env!("CARGO_MANIFEST_DIR"))).expect(chemin)
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn l_echelle_les_constantes_et_la_longueur_par_pointe() {
    assert_eq!((MICROMETRES_PAR_CASE, POINTES_MINIMALES, POINTES_MAXIMALES), (7.0, 40, 200));
    assert_eq!((FOURCHETTE_MINIMALE_UM, FOURCHETTE_MAXIMALE_UM), (35.0, 682.0));
    for (cases, um) in [(0.0, 0.0), (1.0, 7.0), (2.5, 17.5), (-3.0, -21.0), (50.6, 50.6 * 7.0)] {
        assert_eq!(en_micrometres(cases), um);
    }
    assert_eq!(longueur_par_pointe_cases(100.0, 4), Some(25.0));
    assert_eq!(longueur_par_pointe_cases(1.0, 8), Some(0.125), "biomasse / pointes, pas l'inverse");
    assert_eq!(longueur_par_pointe_cases(0.0, 5), Some(0.0));
    assert_eq!(longueur_par_pointe_cases(100.0, 0), None);
    assert_eq!(longueur_par_pointe_cases(0.0, 0), None);
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_taux_est_la_pente_des_moindres_carres_du_logarithme_sur_la_plage() {
    for (rapport, taux) in [(E_0_05, 0.05), (E_0_6, 0.6), (2.0, LN_2), (0.25, -2.0 * LN_2), (1.0, 0.0)] {
        for depart in [1.0, 37.0, 1e-3, 1e6] {
            let t = taux_de_croissance(&geometrique(depart, rapport, 41), 0, 40);
            assert!((t - taux).abs() < 1e-9, "rapport {rapport}, départ {depart} : {t}");
        }
    }
    // Taux 0,05 jusqu'au pas 30 inclus, puis 0,6 : chaque plage ne voit que sa part, les deux bouts compris.
    let mut v = geometrique(1.0, E_0_05, 31);
    v.extend(geometrique(v[30] * E_0_6, E_0_6, 29));
    for (debut, fin, taux) in [(0, 30, 0.05), (29, 30, 0.05), (10, 20, 0.05), (30, 31, 0.6), (31, 59, 0.6)] {
        assert!((taux_de_croissance(&v, debut, fin) - taux).abs() < 1e-9, "{debut}..={fin}");
    }
    // ln v = 0, 1, 0, 1 : moindres carrés 0,2 (les deux bouts donneraient 1/3), où que soit la plage.
    assert!((taux_de_croissance(&[1.0, E, 1.0, E], 0, 3) - 0.2).abs() < 1e-12);
    assert!((taux_de_croissance(&[9.0, 1.0, E, 1.0, E, 5.0], 1, 4) - 0.2).abs() < 1e-12);
    assert!((taux_de_croissance(&[1.0, E, E], 0, 2) - 0.5).abs() < 1e-12);
    // Sur des suites quelconques : la formule `n Σxy − Σx Σy` écrite à part.
    let mut hasard = Hasard(12);
    for _ in 0..200 {
        let n = 2 + hasard.jusqu_a(400);
        let valeurs: Vec<f64> = (0..n).map(|_| 1.0 + hasard.jusqu_a(100_000) as f64 / 100.0).collect();
        let debut = hasard.jusqu_a(n - 1);
        let fin = debut + 1 + hasard.jusqu_a(n - debut - 1);
        let (t, oracle) = (taux_de_croissance(&valeurs, debut, fin), taux_oracle(&valeurs, debut, fin));
        assert!((t - oracle).abs() < 1e-9, "{debut}..={fin} : {t} contre {oracle}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_taux_refuse_une_plage_ou_une_valeur_impossible() {
    let bonne = geometrique(1.0, E_0_05, 10);
    assert!(!panique(|| taux_de_croissance(&bonne, 0, 9)));
    for (debut, fin) in [(3, 3), (5, 3), (0, 10)] {
        assert!(panique(|| taux_de_croissance(&bonne, debut, fin)), "{debut}..={fin}");
    }
    assert!(panique(|| taux_de_croissance(&[], 0, 1)));
    for mauvaise in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        for place in [0, 4, 9] {
            let mut v = bonne.clone();
            v[place] = mauvaise;
            assert!(panique(|| taux_de_croissance(&v, 0, 9)), "{mauvaise} à la place {place}");
        }
        // Hors de la plage, une valeur impossible ne compte pas.
        let mut v = bonne.clone();
        (v[0], v[9]) = (mauvaise, mauvaise);
        assert!((taux_de_croissance(&v, 1, 8) - 0.05).abs() < 1e-9);
    }
}

/// Le tableau I de Trinci (1974), lu sur l'image : taux spécifiques (h⁻¹) de la longueur d'hyphe et des pointes.
const TABLEAU_I: [(&str, f64, f64); 6] = [
    ("Aspergillus nidulans, 25 °C", 0.25, 0.23),
    ("Geotrichum candidum, 25 °C", 0.43, 0.40),
    ("Mucor hiemalis, 25 °C", 0.60, 0.56),
    ("Penicillium chrysogenum, 25 °C", 0.27, 0.28),
    ("Neurospora crassa spco 1, 25 °C", 0.34, 0.38),
    ("Neurospora crassa spco 1, 37 °C", 0.55, 0.52),
];

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_temps_de_doublement_et_l_ecart_relatif_des_taux() {
    assert!((temps_de_doublement(LN_2) - 1.0).abs() < 1e-12);
    assert!((temps_de_doublement(0.05) - 13.862943611198906).abs() < 1e-12);
    // Les temps de doublement du tableau I, en heures, au dixième : 2,8 ; 1,6 ; 1,2 ; 2,6 ; 2,0.
    for (taux, heures) in [(0.25, 2.8), (0.43, 1.6), (0.60, 1.2), (0.27, 2.6), (0.34, 2.0)] {
        assert!((temps_de_doublement(taux) - heures).abs() < 0.05, "taux {taux}");
    }
    for mauvais in [0.0, -0.1, f64::NAN, f64::INFINITY] {
        assert!(panique(|| temps_de_doublement(mauvais)), "taux {mauvais}");
    }
    // Le taux de la biomasse au dénominateur, en valeur absolue.
    assert!((ecart_relatif(0.34, 0.38) - 0.04 / 0.34).abs() < 1e-12, "pas 0,04 / 0,38");
    assert!((ecart_relatif(0.5, 0.4) - 0.2).abs() < 1e-12 && (ecart_relatif(0.5, 0.6) - 0.2).abs() < 1e-12);
    assert!((ecart_relatif(0.5, -0.5) - 2.0).abs() < 1e-12, "des pointes qui décroissent ont un écart");
    for (biomasse, pointes) in [(0.0, 0.1), (-0.2, 0.1), (f64::NAN, 0.1), (0.1, f64::NAN), (0.1, f64::INFINITY)] {
        assert!(panique(|| ecart_relatif(biomasse, pointes)), "{biomasse} {pointes}");
    }
    // Le plus grand écart du tableau I : N. crassa à 25 °C, 0,1176, sous la tolérance de 0,12.
    let (plus_grand, nom) = TABLEAU_I
        .iter()
        .map(|&(nom, h, t)| (ecart_relatif(h, t), nom))
        .fold((0.0, ""), |a, b| if b.0 > a.0 { b } else { a });
    assert_eq!(nom, "Neurospora crassa spco 1, 25 °C");
    assert!(plus_grand > 0.1176 && plus_grand < 0.1177);
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn la_fenetre_a_la_main() {
    let f = |nombres: &[usize]| fenetre_de_croissance(&pointes(nombres));
    // Pas de fenêtre : aucun relevé, jamais 40 pointes, jamais refermée, éteinte, ou déjà au-dessus de 200 à 40.
    let sans_fenetre: [&[usize]; 5] =
        [&[], &[1, 5, 39, 39, 30], &[39, 40, 100, 200], &[40, 120, 60, 0, 0], &[10, 300, 100]];
    for sans in sans_fenetre {
        assert_eq!(f(sans), None, "{sans:?}");
    }
    // Les bords 40 et 200 sont dedans, 39 avant et 201 après dehors.
    assert_eq!(f(&[39, 40, 100, 200, 201]), Some((1, 3)));
    assert_eq!(f(&[10, 39, 40, 201, 300, 150]), Some((2, 2)), "un seul relevé");
    // Seule la première montée compte.
    assert_eq!(f(&[5, 50, 120, 250, 300, 180, 60, 250]), Some((1, 2)));
    // Le nombre de pointes peut retomber sous 40 sans couper la fenêtre.
    assert_eq!(f(&[10, 39, 40, 41, 39, 25, 42, 100, 201, 50]), Some((2, 7)));
    // Les indices sont ceux de la liste.
    assert_eq!(f(&[0, 0, 0, 40, 100, 201]), Some((3, 4)));
    assert_eq!(f(&[0, 0, 40, 100, 201]), Some((2, 3)));
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn la_fenetre_est_celle_de_la_machine_a_etats_sur_quatre_mille_suites() {
    let mut hasard = Hasard(2024);
    let mut issues = [0; 3]; // une fenêtre ; aucune malgré 40 pointes ; jamais 40 pointes
    for cas in 0..4000 {
        let mut courant = 1 + hasard.jusqu_a(30);
        let mut nombres = Vec::new();
        for _ in 0..hasard.jusqu_a(40) {
            nombres.push(courant);
            courant = match hasard.jusqu_a(6) {
                0 => courant.saturating_sub(1 + hasard.jusqu_a(30)),
                1 | 2 => courant + hasard.jusqu_a(40),
                3 => hasard.jusqu_a(260),
                _ => courant + hasard.jusqu_a(3),
            };
        }
        let obtenue = fenetre_de_croissance(&pointes(&nombres));
        assert_eq!(obtenue, fenetre_oracle(&pointes(&nombres)), "cas {cas} : {nombres:?}");
        issues[match obtenue {
            Some(_) => 0,
            None if nombres.iter().any(|&p| p >= 40) => 1,
            None => 2,
        }] += 1;
    }
    assert!(issues.iter().all(|&n| n > 300), "les trois issues sont tirées : {issues:?}");
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn la_longueur_par_pointe_moyenne_d_une_fenetre() {
    // 90 / 30 = 3 ; 120 / 40 = 3 ; 250 / 50 = 5 ; 400 / 80 = 5.
    let r = releves(&[(90.0, 30), (120.0, 40), (250.0, 50), (400.0, 80)]);
    assert_eq!(longueur_par_pointe_moyenne(&r, (1, 3)), 13.0 / 3.0);
    assert_eq!(longueur_par_pointe_moyenne(&r, (0, 3)), 4.0);
    assert_eq!(longueur_par_pointe_moyenne(&r, (3, 3)), 5.0);
    assert_eq!(longueur_par_pointe_moyenne(&r, (1, 2)), 4.0, "la moyenne des rapports, pas 370 / 90");
    let r = releves(&[(10.0, 5), (30.0, 0), (40.0, 5)]);
    assert!(!panique(|| longueur_par_pointe_moyenne(&r, (2, 2))));
    for fenetre in [(0, 1), (1, 1), (2, 0), (0, 3)] {
        assert!(panique(|| longueur_par_pointe_moyenne(&r, fenetre)), "{fenetre:?}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn ni_fonction_interdite_ni_hashmap_et_le_noyau_ne_connait_pas_calage() {
    // Les noms sont assemblés pour que ce fichier ne se dénonce pas lui-même.
    let mut interdits: Vec<String> =
        "sin cos tan atan2 exp ln log2 log10 powf powi hypot mul_add".split(' ').map(|n| format!(".{n}(")).collect();
    interdits.extend(["Map", "Set"].map(|m| format!("Hash{m}")));
    for fichier in [
        "src/calage.rs",
        "examples/mesure_calage.rs",
        "tests/etape_12/outils.rs",
        "tests/etape_12_calage.rs",
        "tests/etape_12_profils.rs",
    ] {
        let texte = lire(fichier);
        assert!(texte.lines().count() > 50, "{fichier} est lu");
        for ligne in texte.lines().filter(|l| !l.trim_start().starts_with("//")) {
            assert!(!interdits.iter().any(|i| ligne.contains(i.as_str())), "{fichier} : {ligne}");
        }
    }
    for fichier in ["croissance", "simulation", "substrat", "voisinage", "releves", "scenario", "anastomose", "main"] {
        let texte = lire(&format!("src/{fichier}.rs"));
        assert!(!texte.contains("calage::") && !texte.contains("mod calage"), "src/{fichier}.rs");
    }
    assert!(lire("src/lib.rs").contains("\npub mod calage;\n"));
}

/// La section de l'étape 12 de `MODELE.md`, du titre au prochain titre de premier niveau.
fn section() -> String {
    let modele = lire("MODELE.md");
    let titre = "\n# Le calage sur des chiffres publiés (étape 12, M2)\n";
    let debut = modele.find(titre).expect("la section de l'étape 12");
    assert!(debut > modele.find("\n# La carte de densité sur la page (étape 11)\n").expect("l'étape 11"));
    let reste = &modele[debut + titre.len()..];
    reste[..reste.find("\n# ").unwrap_or(reste.len())].to_string()
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn la_section_de_modele_md_dit_les_sources_les_chiffres_et_l_echec() {
    let section = section();
    // Une ligne de tableau par source, avec le mot de son statut ; jamais « Bull et Trinci 1977 donnent ».
    for (source, statut) in [
        ("Held", "choix"),
        ("Trinci 1974", "lu à la source"),
        ("Bull et Trinci 1977", "de seconde main"),
        ("Thykaer", "de seconde main"),
        ("Schoder", "lu à la source"),
    ] {
        let ligne = section.lines().find(|l| l.starts_with('|') && l.contains(source) && l.contains(statut));
        assert!(ligne.is_some(), "pas de ligne de tableau « {source} » … « {statut} »");
    }
    assert!(!section.contains("Bull et Trinci 1977 donne") && !section.contains("Bull et Trinci (1977) donne"));
    // Les chiffres publiés et les réglages.
    for mot in [
        "7 µm",
        "0,25",
        "0,23",
        "0,43",
        "0,40",
        "0,60",
        "0,56",
        "0,27",
        "0,28",
        "0,34",
        "0,38",
        "0,55",
        "0,52",
        "0,118",
        "354",
        "47",
        "48",
        "35",
        "682",
        "calage-rapide.txt",
        "calage-dense.txt",
        "32",
        "7,5",
        "601",
        "161",
    ] {
        assert!(section.contains(mot), "« {mot} » manque");
    }
    // Les nombres du résumé du tableau de référence (ce qu'écrit l'exemple), à la française.
    let mut comptes = 0;
    for ligne in lire("tests/etape_12/tableau.txt").lines() {
        let Some((nom, valeur)) = ligne.split_once(' ') else { continue };
        if nom.starts_with("longueur_um_") || nom.starts_with("taux_") || nom.starts_with("ecart_") {
            assert!(section.contains(&valeur.replace('.', ",")), "« {nom} {valeur} » manque");
            comptes += 1;
        }
    }
    assert_eq!(comptes, 2 * 9, "neuf nombres du résumé par profil");
    // L'échec dit tel quel, sa cause, et ce que M2 ne fait pas.
    for mot in ["ne reproduit pas", "0,12", "anastomose", "qualitatif", "aucune densité chiffrée", "échelle de temps"]
    {
        assert!(section.contains(mot), "« {mot} » manque");
    }
}
