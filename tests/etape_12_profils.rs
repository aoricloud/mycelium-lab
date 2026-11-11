//! Étape 12 (M2), partie « les deux profils » : ce que la simulation donne, à côté des chiffres publiés.
//!
//! `scenarios/calage-rapide.txt` (ancre : Mucor hiemalis, 354 µm par pointe, Trinci 1974, fig. 4) et
//! `scenarios/calage-dense.txt` (ancre : Penicillium chrysogenum, 47 à 48 µm, de seconde main) sont joués pour les
//! graines 1 à 20, une seule fois par processus, par les fonctions de l'exemple `mesure_calage` (inclus comme un module :
//! ce sont toujours ses sources du moment qui sont testées, jamais un binaire périmé).
//!
//! - les fichiers ne diffèrent de `couplage.txt` que par l'intervalle de ramification, la boîte, la spore et la durée ;
//! - chaque graine a une fenêtre refermée, loin du bord, le substrat presque plein ;
//! - la longueur par pointe moyenne est dans l'ancre ± 15 % (l'incertitude de lecture des courbes de Trinci), chaque
//!   graine dans le garde-fou de seconde main [35, 682] µm, le rapide au-dessus du dense pour chaque graine ;
//! - l'écart des taux de la biomasse et des pointes (Trinci : au plus 0,118, tolérance 0,12) : le profil dense le tient
//!   en moyenne ; **le profil rapide ne le tient pas** : l'échec est figé tel quel, et ce test se retourne le jour où
//!   l'écart repasse sous 0,12 ;
//! - le tableau de `tests/etape_12/tableau.txt` est ce que l'exemple écrit pour ces quarante graines ;
//! - les mesures sont celles des oracles écrits à part ; le jeu s'arrête à la fermeture sans rien changer ;
//! - l'exemple refuse une commande fausse.

#[path = "etape_12/outils.rs"]
mod outils;

#[allow(dead_code)]
#[path = "../examples/mesure_calage.rs"]
mod exemple;

use std::sync::OnceLock;

use exemple::{Mesure, TOLERANCE, ecrire_le_profil, jouer, lancer, mesurer, moyenne};
use mycelium_lab::releves::{Releve, en_csv};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use outils::{fenetre_oracle, longueur_oracle, taux_oracle};

/// Un profil : son nom, son fichier, ce que le fichier doit dire, et l'ancre publiée de sa longueur par pointe (µm).
struct Profil {
    nom: &'static str,
    fichier: &'static str,
    intervalle: f64,
    cote: usize,
    duree: u32,
    ancre_um: (f64, f64),
}

const RAPIDE: Profil = Profil {
    nom: "rapide",
    fichier: "scenarios/calage-rapide.txt",
    intervalle: 32.0,
    cote: 601,
    duree: 380,
    ancre_um: (354.0, 354.0),
};
const DENSE: Profil = Profil {
    nom: "dense",
    fichier: "scenarios/calage-dense.txt",
    intervalle: 7.5,
    cote: 161,
    duree: 100,
    ancre_um: (47.0, 48.0),
};

fn texte(fichier: &str) -> String {
    std::fs::read_to_string(format!("{}/{fichier}", env!("CARGO_MANIFEST_DIR"))).expect(fichier)
}

/// Les relevés et la mesure de chaque graine, dans l'ordre des graines.
type Serie = Vec<(Vec<Releve>, Mesure)>;

/// Les relevés et la mesure des graines 1 à 20 d'un profil, joués une fois par processus, quatre fils à la fois.
fn serie(profil: &Profil) -> &'static [(Vec<Releve>, Mesure)] {
    static SERIES: [OnceLock<Serie>; 2] = [OnceLock::new(), OnceLock::new()];
    SERIES[usize::from(profil.nom == "dense")].get_or_init(|| {
        let scenario = &texte(profil.fichier);
        let graines: Vec<u64> = (1..=20).collect();
        let mut serie = Vec::new();
        for paquet in graines.chunks(4) {
            std::thread::scope(|portee| {
                let fils: Vec<_> = paquet.iter().map(|&g| portee.spawn(move || jouer(scenario, g).unwrap())).collect();
                for fil in fils {
                    let releves = fil.join().unwrap();
                    let mesure =
                        mesurer(&releves).unwrap_or_else(|| panic!("{} : une graine sans fenêtre", profil.nom));
                    serie.push((releves, mesure));
                }
            });
        }
        serie
    })
}

/// Une colonne de mesures du profil, dans l'ordre des graines.
fn colonne(profil: &Profil, f: fn(&Mesure) -> f64) -> Vec<f64> {
    serie(profil).iter().map(|(_, m)| f(m)).collect()
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn les_profils_ne_different_de_couplage_que_par_l_intervalle_la_boite_la_spore_et_la_duree() {
    let lire = |fichier: &str| lire_scenario_tropismes(&texte(fichier)).expect(fichier);
    let (base, milieu, tropismes, duree) = lire("scenarios/couplage.txt");
    assert_eq!((base.largeur, base.spore, base.croissance.intervalle, duree), (161, (80, 80), 20.0, 80));
    for profil in [&RAPIDE, &DENSE] {
        let (mut r, m, t, duree) = lire(profil.fichier);
        assert_eq!((m, t), (milieu.clone(), tropismes), "{} : ni milieu ni tropisme", profil.nom);
        let demi = profil.cote / 2;
        assert_eq!((r.largeur, r.hauteur, r.spore, duree), (profil.cote, profil.cote, (demi, demi), profil.duree));
        assert_eq!(r.croissance.intervalle, profil.intervalle, "{}", profil.nom);
        // Le reste est celui de `couplage.txt`, anastomose comprise.
        (r.largeur, r.hauteur, r.spore, r.croissance.intervalle) = (161, 161, (80, 80), 20.0);
        assert_eq!(r, base, "{} : le reste des réglages", profil.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn chaque_graine_a_une_fenetre_refermee_loin_du_bord() {
    for (profil, relevees_au_moins) in [(&RAPIDE, 60), (&DENSE, 8)] {
        let demi = (profil.cote - 1) as f64 / 2.0;
        for (graine, (releves, mesure)) in (1..).zip(serie(profil)) {
            let (premier, dernier) = mesure.fenetre;
            let nom = format!("{}, graine {graine}", profil.nom);
            assert!(dernier + 1 - premier >= relevees_au_moins, "{nom} : {premier}..={dernier}");
            // Le jeu s'est arrêté au relevé qui referme la fenêtre, avant la fin de la durée.
            assert_eq!(releves.len(), dernier + 2, "{nom}");
            let fermeture = &releves[dernier + 1];
            assert!(fermeture.pointes > 200 && fermeture.temps < profil.duree, "{nom}");
            assert!(fermeture.rayon < demi - 5.0, "{nom} : rayon {} pour une demi-boîte de {demi}", fermeture.rayon);
            assert!(fermeture.substrat > 0.99 * (profil.cote * profil.cote) as f64, "{nom} : substrat");
        }
    }
}

/// La bande d'un profil : l'ancre à ± 15 %, l'incertitude de lecture des courbes de Trinci.
fn bande(profil: &Profil) -> (f64, f64) {
    (0.85 * profil.ancre_um.0, 1.15 * profil.ancre_um.1)
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn la_longueur_par_pointe_est_dans_l_ancre_a_15_pour_cent_et_dans_le_garde_fou() {
    // Les ancres sont celles des sources (Trinci 1974, fig. 4 ; Thykaer 2009 et Moore) : une bande décalée se voit ici.
    assert_eq!((RAPIDE.ancre_um, DENSE.ancre_um), ((354.0, 354.0), (47.0, 48.0)));
    let (bas, haut) = bande(&RAPIDE);
    assert!((bas - 300.9).abs() < 1e-9 && (haut - 407.1).abs() < 1e-9, "jamais plus étroite ni plus large que ± 15 %");
    for profil in [&RAPIDE, &DENSE] {
        let um = colonne(profil, |m| m.longueur_um);
        let (bas, haut) = bande(profil);
        let m = moyenne(&um).unwrap();
        assert!(bas <= m && m <= haut, "{} : {m} µm hors de {bas} à {haut}", profil.nom);
        // Le garde-fou de seconde main, graine par graine ; vingt graines, vingt valeurs différentes.
        assert!(um.iter().all(|&v| (35.0..=682.0).contains(&v)), "{} : {um:?}", profil.nom);
        assert!(um.iter().enumerate().all(|(i, v)| !um[..i].contains(v)), "{} : des graines identiques", profil.nom);
    }
    let (rapide, dense) = (colonne(&RAPIDE, |m| m.longueur_um), colonne(&DENSE, |m| m.longueur_um));
    let plus_petit_rapide = rapide.iter().copied().reduce(f64::min).unwrap();
    assert!(dense.iter().all(|&d| 2.0 * d < plus_petit_rapide), "le rapide au-dessus du dense, largement");
}

/// L'écart relatif des taux moyens des vingt graines, et la moyenne des écarts de chaque graine.
fn ecarts(profil: &Profil) -> (f64, f64) {
    let biomasse = moyenne(&colonne(profil, |m| m.taux_biomasse)).unwrap();
    let pointes = moyenne(&colonne(profil, |m| m.taux_pointes)).unwrap();
    ((pointes - biomasse).abs() / biomasse, moyenne(&colonne(profil, |m| m.ecart)).unwrap())
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_profil_dense_tient_la_tolerance_de_trinci_en_moyenne() {
    let (des_moyennes, moyen) = ecarts(&DENSE);
    assert!(des_moyennes <= TOLERANCE && moyen <= TOLERANCE, "{des_moyennes} et {moyen}");
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_profil_rapide_ne_reproduit_pas_la_tolerance_de_trinci() {
    // L'ÉCHEC DIT TEL QUEL (décision d'André) : la longueur croît plus vite que le nombre de pointes vivantes, parce
    // qu'une pointe sur deux finit en anastomose. Ce test ne relâche pas la tolérance : il fige l'échec. Le jour où
    // l'écart repasse sous 0,12, il échoue, et il faut le retourner en `<= TOLERANCE`.
    let (des_moyennes, moyen) = ecarts(&RAPIDE);
    assert!(des_moyennes > TOLERANCE && moyen > TOLERANCE, "{des_moyennes} et {moyen}");
    let mesures = serie(&RAPIDE).iter().map(|(_, m)| m);
    assert!(mesures.clone().all(|m| m.taux_pointes < m.taux_biomasse), "les pointes croissent moins vite");
    for (releves, m) in serie(&RAPIDE) {
        let fin = &releves[m.fenetre.1];
        let part = fin.anastomoses as f64 / fin.ramifications as f64;
        assert!(part > 0.35, "la cause : {part} des ramifications finissent en anastomose");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_tableau_de_reference_est_celui_que_l_exemple_ecrit_pour_les_quarante_graines() {
    let mut obtenu = String::new();
    for (profil, fichier) in [(&RAPIDE, RAPIDE.fichier), (&DENSE, DENSE.fichier)] {
        let mesures: Vec<(u64, Option<Mesure>)> = (1..).zip(serie(profil).iter().map(|(r, _)| mesurer(r))).collect();
        if !obtenu.is_empty() {
            obtenu.push('\n');
        }
        ecrire_le_profil(&mut obtenu, profil.nom, fichier, &mesures);
    }
    let reference = texte("tests/etape_12/tableau.txt");
    assert_eq!(obtenu, reference, "tests/etape_12/tableau.txt");
    // Le chemin de l'exemple pour un profil nommé est le même texte.
    let arguments = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(lancer(&arguments(&["1", "20", "dense"])).unwrap(), reference.split_once("\n\n").unwrap().1);
    // Une graine seule n'a pas d'écart-type ; la graine 55 du rapide n'a pas de fenêtre en 380 pas.
    let une = lancer(&arguments(&["1", "1", "dense"])).unwrap();
    assert!(une.contains("\nlongueur_um_ecart_type aucun\n") && une.contains("\ngraines 1\n"), "{une}");
    let sans = lancer(&arguments(&["55", "55", "rapide"])).unwrap();
    assert!(sans.contains("\n55 aucune\n") && sans.contains("\nsans_fenetre 1\n"), "{sans}");
    assert!(sans.contains("\nlongueur_um_moyenne aucun\n") && sans.contains("\nau_dessus_de_0_12 0\n"), "{sans}");
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn les_mesures_sont_celles_des_oracles() {
    for profil in [&RAPIDE, &DENSE] {
        for (releves, m) in serie(profil) {
            assert_eq!(Some(m.fenetre), fenetre_oracle(releves), "{}", profil.nom);
            let (p, d) = m.fenetre;
            assert!((m.longueur_um - 7.0 * longueur_oracle(releves, p, d)).abs() < 1e-9 * m.longueur_um);
            let biomasse: Vec<f64> = releves.iter().map(|r| r.biomasse).collect();
            let pointes: Vec<f64> = releves.iter().map(|r| r.pointes as f64).collect();
            assert!((m.taux_biomasse - taux_oracle(&biomasse, p, d)).abs() < 1e-9);
            assert!((m.taux_pointes - taux_oracle(&pointes, p, d)).abs() < 1e-9);
            assert!((m.ecart - (m.taux_pointes - m.taux_biomasse).abs() / m.taux_biomasse).abs() < 1e-12);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn le_jeu_est_deterministe_et_s_arreter_a_la_fermeture_ne_change_rien() {
    let scenario = texte(RAPIDE.fichier);
    let (premiers, mesure) = &serie(&RAPIDE)[0];
    assert_eq!(en_csv(&jouer(&scenario, 1).unwrap()), en_csv(premiers), "même graine, mêmes relevés");
    assert_ne!(en_csv(&serie(&RAPIDE)[1].0), en_csv(premiers), "une autre graine, d'autres relevés");
    // Toute la durée jouée : les premiers relevés sont les mêmes, la mesure aussi.
    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(&scenario).unwrap();
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, 1).unwrap();
    simulation.avancer_n(duree);
    let toute = simulation.releves();
    assert_eq!(toute.len(), 381);
    assert_eq!(en_csv(&toute[..premiers.len()]), en_csv(premiers));
    assert_eq!(mesurer(toute).as_ref(), Some(mesure));
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn l_exemple_refuse_une_commande_fausse() {
    let essai = |a: &[&str]| lancer(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    for faux in [
        &["1"][..],
        &["1", "2", "dense", "rapide"],
        &["abc", "3"],
        &["1", "abc"],
        &["-1", "3"],
        &["", "3"],
        &["1.5", "3"],
        &["1", "18446744073709551616"],
        &["3", "1"],
        &["1", "2", "Rapide"],
        &["1", "2", ""],
    ] {
        assert!(essai(faux).is_err(), "{faux:?} est refusé");
    }
    assert_eq!(exemple::lire_les_arguments(&[]), Ok((1, 20, None)));
}

#[test]
#[cfg_attr(not(feature = "etape-12"), ignore)]
fn les_cas_limites_de_l_exemple() {
    assert_eq!(TOLERANCE, 0.12, "le plus grand écart du tableau I de Trinci, 0,118, arrondi");
    // Sans profil nommé : les deux, le rapide d'abord, séparés par une ligne vide.
    let deux = lancer(&["1".to_string(), "1".to_string()]).unwrap();
    let (rapide, dense) = deux.split_once("\n\n").expect("une ligne vide entre les profils");
    assert!(rapide.starts_with("profil rapide scenarios/calage-rapide.txt\n") && dense.starts_with("profil dense "));
    // Pas de pente sur une fenêtre d'un seul relevé, ni sur une biomasse qui ne croît pas.
    assert_eq!(mesurer(&outils::pointes(&[10, 50, 201])), None);
    assert_eq!(mesurer(&outils::releves(&[(5.0, 10), (5.0, 50), (5.0, 60), (5.0, 300)])), None);
    assert!(mesurer(&outils::releves(&[(5.0, 10), (100.0, 50), (200.0, 60), (400.0, 300)])).is_some());
}
