//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « l'exemple `mesure_moule` ».
//!
//! `cargo run --release --example mesure_moule -- <scénario> <graine> [<dossier de sortie>]` lit le scénario, le développe, joue sa
//! `duree`, imprime la taille du moule, la courbe de remplissage, les mesures de la pièce et, pour un labyrinthe, le temps de
//! sortie et le plus court chemin ; avec un dossier, il écrit aussi `densite.png`. Ces tests lancent l'exemple compilé (le
//! binaire est cherché à côté du binaire de test, sous `examples/` ; s'il manque, il est construit par `cargo build --example
//! mesure_moule`).
//!
//! - **Sa sortie, caractère pour caractère**, sur les cinq scénarios (et plusieurs graines, dont deux du labyrinthe : une qui
//!   sort et une qui ne sort pas) : `moule N`, une ligne `courbe pas occupées remplissage` tous les 10 pas (plus le dernier
//!   pas), les quatorze lignes de `mesurer`, puis `sortie` et `chemin` pour un labyrinthe seulement. Tout est recalculé ici, sans
//!   l'exemple ni `courbe_de_remplissage` : une simulation jouée pas à pas, `mesurer` à chaque pas, `sortie_de_reference`, le plus
//!   court chemin par relaxations ;
//! - **les durées qui ne sont pas des multiples de 10** (0, 7, 10, 20, 25) : le dernier pas ajouté une seule fois ;
//! - **un labyrinthe** reconnu à sa ligne `moule labyrinthe` (en retrait, avec des commentaires qui parlent de labyrinthe et de
//!   brique) ;
//! - **`densite.png`** : chaque pixel (case bloquée grise, fond, rampe), à 4 pixels par case, sur les cinq scénarios ;
//! - **ses refus** : sans argument, trop d'arguments, une graine illisible, un scénario introuvable, invalide, ou dont le moule
//!   est refusé (chaque erreur de `developper`), une spore dans un obstacle, un dossier impossible : le code 2, rien sur la
//!   sortie standard, rien d'écrit ;
//! - **il est déterministe**, et **sans dossier, il n'écrit aucun fichier**.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `floor`, `sqrt`, `min` et `max`.

#[path = "etape_10/outils.rs"]
mod outils;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use mycelium_lab::moule::{centre_de_la_chambre, developper};
use mycelium_lab::piece::{MesuresPiece, carte_de_densite, mesurer};
use mycelium_lab::simulation::Simulation;
use outils::{
    FOND, GRIS_BLOQUE, cases_libres_de, chambre_de_reference, construire, couleur_attendue, delai,
    plus_court_de_reference, relire, scenarios_de_moule, sortie_de_reference, texte_du_fichier, texte_du_scenario,
};

/// Les quatorze champs de `MesuresPiece`, dans l'ordre de la spécification.
const CHAMPS: [&str; 14] = [
    "cases_libres",
    "cases_occupees",
    "remplissage",
    "densite_moyenne",
    "densite_maximale",
    "biomasse",
    "nourriture_initiale",
    "nourriture_restante",
    "nourriture_consommee",
    "ecart_du_bilan",
    "plus_grand_morceau",
    "nombre_de_morceaux",
    "traversee_gauche_droite",
    "traversee_haut_bas",
];

/// Le binaire de l'exemple, construit au besoin (une seule fois par processus de tests).
fn exemple() -> &'static Path {
    static CHEMIN: OnceLock<PathBuf> = OnceLock::new();
    CHEMIN.get_or_init(|| {
        let test = std::env::current_exe().expect("le chemin du binaire de test");
        let profil = test.parent().and_then(Path::parent).expect("target/<profil>/deps/…").to_path_buf();
        let binaire = profil.join("examples").join(format!("mesure_moule{}", std::env::consts::EXE_SUFFIX));
        if !binaire.exists() {
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
            let mut commande = Command::new(cargo);
            commande.args(["build", "--example", "mesure_moule"]).current_dir(env!("CARGO_MANIFEST_DIR"));
            if profil.file_name().is_some_and(|n| n == "release") {
                commande.arg("--release");
            }
            let statut = commande.status().expect("cargo se lance");
            assert!(statut.success(), "cargo build --example mesure_moule a échoué");
        }
        assert!(binaire.exists(), "l'exemple mesure_moule est introuvable : {}", binaire.display());
        binaire
    })
}

/// Un dossier neuf (vidé s'il existait) sous le dossier temporaire des tests de Cargo.
fn dossier_neuf(nom: &str) -> PathBuf {
    let dossier = Path::new(env!("CARGO_TARGET_TMPDIR")).join("etape_10_exemple").join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).expect("le dossier de test se crée");
    dossier
}

/// Lance l'exemple depuis la racine du dépôt.
fn lancer(args: &[&str]) -> Output {
    Command::new(exemple()).args(args).current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("l'exemple se lance")
}

/// Range un texte de scénario dans un dossier neuf et rend son chemin.
fn ranger(dossier: &str, texte: &str) -> String {
    let dossier = dossier_neuf(dossier);
    let fichier = dossier.join("scenario.txt");
    std::fs::write(&fichier, texte).expect("le scénario se range");
    fichier.to_str().unwrap().to_string()
}

/// Les quatorze lignes de `mesurer`, écrites d'après la spécification de M0.
fn lignes_des_mesures(m: &MesuresPiece) -> String {
    let traversee = |t: Option<usize>| t.map_or_else(|| "aucune".to_string(), |n| n.to_string());
    let valeurs = [
        m.cases_libres.to_string(),
        m.cases_occupees.to_string(),
        m.remplissage.to_string(),
        m.densite_moyenne.to_string(),
        m.densite_maximale.to_string(),
        m.biomasse.to_string(),
        m.nourriture_initiale.to_string(),
        m.nourriture_restante.to_string(),
        m.nourriture_consommee.to_string(),
        m.ecart_du_bilan.to_string(),
        m.plus_grand_morceau.to_string(),
        m.nombre_de_morceaux.to_string(),
        traversee(m.traversee_gauche_droite),
        traversee(m.traversee_haut_bas),
    ];
    CHAMPS.iter().zip(valeurs).map(|(nom, valeur)| format!("{nom} {valeur}\n")).collect()
}

/// La sortie standard attendue de l'exemple pour ce texte de scénario et cette graine, et la simulation jouée jusqu'à la durée.
/// `labyrinthe` : `(colonnes, rangees, couloir)` si la ligne `moule` du scénario est un labyrinthe.
fn sortie_attendue(texte: &str, graine: u64, labyrinthe: Option<(usize, usize, usize)>) -> (String, Simulation) {
    let developpe = developper(texte).expect("le scénario se développe");
    let (mut simulation, _, duree) = construire(&developpe, graine);
    // La courbe : une simulation jouée pas à pas, `mesurer` aux pas imprimés (tous les 10 pas et le dernier).
    let mut mesures = mesurer(&simulation);
    let mut attendu = format!("moule {}\n", mesures.cases_libres);
    attendu.push_str(&format!("courbe 0 {} {}\n", mesures.cases_occupees, mesures.remplissage));
    for pas in 1..=duree {
        simulation.avancer();
        if pas % 10 == 0 || pas == duree {
            mesures = mesurer(&simulation);
            attendu.push_str(&format!("courbe {pas} {} {}\n", mesures.cases_occupees, mesures.remplissage));
        }
    }
    attendu.push_str(&lignes_des_mesures(&mesures));
    if let Some((colonnes, rangees, couloir)) = labyrinthe {
        // La sortie est la dernière chambre entière ; le plus court chemin va de centre à centre.
        let arrivee = centre_de_la_chambre(couloir, colonnes - 1, rangees - 1);
        let depart = centre_de_la_chambre(couloir, 0, 0);
        let sortie =
            sortie_de_reference(&mut simulation, chambre_de_reference(couloir, colonnes - 1, rangees - 1), duree);
        let libres = cases_libres_de(&simulation);
        let (largeur, hauteur) = (simulation.reglages().largeur, simulation.reglages().hauteur);
        let chemin = plus_court_de_reference(largeur, hauteur, &libres, depart, arrivee);
        attendu.push_str(&format!("sortie {}\n", sortie.map_or_else(|| "aucun".to_string(), |t| t.to_string())));
        attendu.push_str(&format!("chemin {}\n", chemin.map_or_else(|| "aucun".to_string(), |n| n.to_string())));
    }
    (attendu, simulation)
}

/// Vérifie `densite.png` contre la simulation jouée jusqu'à la durée : `k` pixels par case, chaque case un carré uniforme de la
/// couleur de la spécification de M0.
fn verifier_l_image(octets: &[u8], simulation: &Simulation, k: usize) {
    assert!(octets.len() < 500_000, "l'image pèse {} octets : 500 Ko au plus", octets.len());
    let png = relire(octets);
    let reglages = simulation.reglages();
    let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
    assert_eq!((png.largeur, png.hauteur), (k * largeur, k * hauteur), "{k} pixels par case");
    let carte = carte_de_densite(largeur, hauteur, simulation.segments());
    let dmax = mesurer(simulation).densite_maximale;
    let mut niveaux = Vec::new();
    for y in 0..hauteur {
        for x in 0..largeur {
            let d = carte.valeur(x, y);
            let attendue = if simulation.substrat().est_bloquee(x, y) {
                GRIS_BLOQUE
            } else if d > 0.0 {
                niveaux.push(((d / dmax).min(1.0).sqrt() * 255.0) as i32);
                couleur_attendue(d, dmax)
            } else {
                FOND
            };
            for dy in 0..k {
                for dx in 0..k {
                    assert_eq!(
                        png.pixel(x * k + dx, y * k + dy),
                        attendue,
                        "le pixel ({}, {}) de la case ({x}, {y}), densité {d}",
                        x * k + dx,
                        y * k + dy
                    );
                }
            }
        }
    }
    niveaux.sort_unstable();
    niveaux.dedup();
    assert!(niveaux.len() >= 8 && niveaux.last() == Some(&255), "garde : niveaux {niveaux:?}");
}

/// Les `(colonnes, rangees, couloir)` du labyrinthe du scénario de moules, ou `None`.
fn labyrinthe_de(nom: &str) -> Option<(usize, usize, usize)> {
    (nom == "labyrinthe").then_some((6, 6, 3))
}

fn texte_de_sortie(resultat: &Output) -> String {
    assert!(resultat.status.success(), "l'exemple échoue : {}", String::from_utf8_lossy(&resultat.stderr));
    assert!(resultat.stderr.is_empty(), "rien sur la sortie d'erreur : {}", String::from_utf8_lossy(&resultat.stderr));
    String::from_utf8(resultat.stdout.clone()).expect("la sortie est de l'UTF-8")
}

// ---------------------------------------------------------------------------------------------------------------------
// Les tests
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_exemple_imprime_exactement_la_sortie_attendue_et_l_image_sur_les_cinq_scenarios() {
    delai();
    for s in scenarios_de_moule() {
        let fichier = format!("scenarios/moules/{}.txt", s.nom);
        let dossier = dossier_neuf(&format!("cinq-{}", s.nom));
        let resultat = lancer(&[&fichier, "1", dossier.to_str().unwrap()]);
        let sortie = texte_de_sortie(&resultat);
        let (attendu, simulation) = sortie_attendue(&texte_du_scenario(s.nom), 1, labyrinthe_de(s.nom));
        assert_eq!(sortie, attendu, "{} : la sortie de l'exemple, caractère pour caractère", s.nom);
        // La forme de la sortie : `moule`, des `courbe` (tous les 10 pas, le dernier compris), quatorze mesures, le labyrinthe.
        let lignes: Vec<&str> = sortie.lines().collect();
        assert_eq!(lignes[0], format!("moule {}", s.cases_libres), "{}", s.nom);
        let courbes = (s.duree / 10 + 1) as usize + usize::from(s.duree % 10 != 0);
        assert_eq!(lignes.iter().filter(|l| l.starts_with("courbe ")).count(), courbes, "{} : lignes courbe", s.nom);
        assert_eq!(lignes.len(), 1 + courbes + 14 + if s.nom == "labyrinthe" { 2 } else { 0 }, "{}", s.nom);
        assert_eq!(s.nom == "labyrinthe", sortie.contains("\nsortie ") && sortie.contains("\nchemin "), "{}", s.nom);
        let image = std::fs::read(dossier.join("densite.png")).expect("densite.png est écrit");
        verifier_l_image(&image, &simulation, 4);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn dans_le_labyrinthe_l_exemple_dit_la_sortie_et_le_plus_court_chemin() {
    delai();
    // Graines 1, 3, 7 et 5 : sortie aux pas 69, 65, 61 et 65 ; le plus court chemin fait toujours 52 pas.
    for (graine, sortie) in [(1u64, "69"), (3, "65"), (7, "61"), (5, "65")] {
        let resultat = lancer(&["scenarios/moules/labyrinthe.txt", &graine.to_string()]);
        let texte = texte_de_sortie(&resultat);
        assert!(texte.ends_with(&format!("sortie {sortie}\nchemin 52\n")), "graine {graine} : {texte}");
        let (attendu, _) = sortie_attendue(&texte_du_scenario("labyrinthe"), graine, Some((6, 6, 3)));
        assert_eq!(texte, attendu, "graine {graine}");
    }
    // Le même labyrinthe sans thigmotropisme : la colonie ne sort pas dans la durée, `sortie aucun`, et le chemin est le même.
    let texte: String = texte_du_scenario("labyrinthe")
        .lines()
        .map(|l| if l.starts_with("thigmotropisme ") { "thigmotropisme 0\n".to_string() } else { format!("{l}\n") })
        .collect();
    let fichier = ranger("labyrinthe-sans-thigmotropisme", &texte);
    let sortie = texte_de_sortie(&lancer(&[&fichier, "1"]));
    assert!(sortie.ends_with("sortie aucun\nchemin 52\n"), "{sortie}");
    let (attendu, _) = sortie_attendue(&texte, 1, Some((6, 6, 3)));
    assert_eq!(sortie, attendu);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_autres_graines_des_cavites_sont_aussi_celles_du_recalcul() {
    delai();
    for (nom, graine) in [("brique", 4u64), ("l", 2), ("disque", 3), ("anneau", 2)] {
        let fichier = format!("scenarios/moules/{nom}.txt");
        let resultat = lancer(&[&fichier, &graine.to_string()]);
        let (attendu, _) = sortie_attendue(&texte_du_scenario(nom), graine, None);
        assert_eq!(texte_de_sortie(&resultat), attendu, "{nom}, graine {graine}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_courbe_s_imprime_tous_les_dix_pas_et_au_dernier_pas_une_seule_fois() {
    delai();
    let base =
        "boite 30 20\nspore 15 10\nsubstrat 1.0 0.2\ncroissance 1.0 8.0 35 20\ncouplage 0.1 0.002\nanastomose 0.5 8\n";
    for duree in [0u32, 1, 7, 9, 10, 11, 20, 25, 30] {
        let texte = format!("# duree {duree}\n{base}duree {duree}\nmoule brique 5 5 24 14\nthigmotropisme 35\n");
        let fichier = ranger(&format!("duree-{duree}"), &texte);
        let resultat = lancer(&[&fichier, "3"]);
        let sortie = texte_de_sortie(&resultat);
        let (attendu, _) = sortie_attendue(&texte, 3, None);
        assert_eq!(sortie, attendu, "durée {duree}");
        let pas: Vec<u32> = sortie
            .lines()
            .filter(|l| l.starts_with("courbe "))
            .map(|l| l.split(' ').nth(1).unwrap().parse().unwrap())
            .collect();
        let mut voulus: Vec<u32> = (0..=duree).filter(|p| p % 10 == 0).collect();
        if duree % 10 != 0 {
            voulus.push(duree);
        }
        assert_eq!(pas, voulus, "durée {duree} : les pas de la courbe");
        // Les quatorze lignes de `mesurer` à la fin, puis rien d'autre : pas de ligne `sortie` hors d'un labyrinthe.
        assert!(!sortie.contains("sortie") && !sortie.contains("chemin"), "durée {duree}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_labyrinthe_est_reconnu_a_sa_ligne_moule_meme_en_retrait_et_malgre_les_commentaires() {
    delai();
    let base = "boite 9 9\nspore 1 1\nsubstrat 1.0 0.2\ncroissance 1.0 5.0 35 20\ncouplage 0.1 0.002\nanastomose 0.5 8\nduree 40\n";
    // Un labyrinthe 4 × 4 de couloir 1 (9 × 9), la ligne `moule` en retrait, des commentaires qui parlent de brique.
    let texte =
        format!("# ce n'est pas un moule brique 1 1 2 2\n{base}thigmotropisme 35\n   moule   labyrinthe 4 4 1 12\n");
    let fichier = ranger("labyrinthe-en-retrait", &texte);
    let sortie = texte_de_sortie(&lancer(&[&fichier, "2"]));
    let (attendu, _) = sortie_attendue(&texte, 2, Some((4, 4, 1)));
    assert_eq!(sortie, attendu);
    assert!(sortie.contains("\nsortie ") && sortie.contains("\nchemin "));
    // Une brique avec un commentaire qui parle de labyrinthe : ni `sortie` ni `chemin`.
    let texte = format!("# moule labyrinthe 4 4 1 12\n{base}moule brique 0 0 8 8\n");
    let fichier = ranger("brique-et-commentaire", &texte);
    let sortie = texte_de_sortie(&lancer(&[&fichier, "2"]));
    let (attendu, _) = sortie_attendue(&texte, 2, None);
    assert_eq!(sortie, attendu);
    assert!(!sortie.contains("sortie") && !sortie.contains("chemin"));
    // Un labyrinthe 1 × 1 : la spore est déjà dans la sortie (`sortie 0`) et le plus court chemin fait 0 pas.
    let texte = "boite 3 3\nspore 1 1\nsubstrat 1.0 0.2\ncroissance 1.0 5.0 35 20\ncouplage 0.1 0.002\nduree 5\nmoule labyrinthe 1 1 1 0\n";
    let fichier = ranger("labyrinthe-un-sur-un", texte);
    let sortie = texte_de_sortie(&lancer(&[&fichier, "2"]));
    assert!(sortie.ends_with("sortie 0\nchemin 0\n"), "{sortie}");
    let (attendu, _) = sortie_attendue(texte, 2, Some((1, 1, 1)));
    assert_eq!(sortie, attendu);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_scenario_sans_ligne_moule_se_mesure_aussi() {
    delai();
    let resultat = lancer(&["scenarios/mur.txt", "31"]);
    let sortie = texte_de_sortie(&resultat);
    let (attendu, _) = sortie_attendue(&texte_du_fichier("mur.txt"), 31, None);
    assert_eq!(sortie, attendu);
    assert!(sortie.starts_with("moule 9756\n"), "100 × 100 moins un obstacle de 4 × 61 : {sortie}");
    assert!(!sortie.contains("sortie") && !sortie.contains("chemin"));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_exemple_est_deterministe_et_sans_dossier_il_n_ecrit_rien() {
    delai();
    let a = dossier_neuf("det-a");
    let b = dossier_neuf("det-b");
    let un = lancer(&["scenarios/moules/disque.txt", "2", a.to_str().unwrap()]);
    let deux = lancer(&["scenarios/moules/disque.txt", "2", b.to_str().unwrap()]);
    assert!(un.status.success() && deux.status.success());
    assert_eq!(un.stdout, deux.stdout, "les mêmes lignes");
    assert_eq!(
        std::fs::read(a.join("densite.png")).unwrap(),
        std::fs::read(b.join("densite.png")).unwrap(),
        "les mêmes octets de PNG"
    );
    let autre = lancer(&["scenarios/moules/disque.txt", "3"]);
    assert!(autre.status.success());
    assert_ne!(un.stdout, autre.stdout, "une autre graine donne une autre sortie");

    // Sans dossier : aucune écriture, ni dans le dossier courant ni ailleurs. Le chemin du scénario est absolu.
    let courant = dossier_neuf("sans-dossier");
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios/moules/anneau.txt");
    let sans = Command::new(exemple()).args([chemin.to_str().unwrap(), "2"]).current_dir(&courant).output().unwrap();
    let (attendu, _) = sortie_attendue(&texte_du_scenario("anneau"), 2, None);
    assert_eq!(texte_de_sortie(&sans), attendu);
    let ecrits: Vec<_> = std::fs::read_dir(&courant).unwrap().collect();
    assert!(ecrits.is_empty(), "sans dossier de sortie, l'exemple n'écrit rien : {ecrits:?}");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_exemple_refuse_une_commande_fausse_sans_rien_ecrire() {
    delai();
    let sortie = dossier_neuf("refus");
    let dossier = sortie.join("sortie");
    let dossier = dossier.to_str().unwrap();
    let ecrire = |nom: &str, texte: &str| {
        let fichier = sortie.join(nom);
        std::fs::write(&fichier, texte).unwrap();
        fichier.to_str().unwrap().to_string()
    };
    let base = "substrat 1.0 0.2\ncroissance 1.0 5.0 35 20\ncouplage 0.1 0.002\nduree 5\n";
    let invalide = ecrire("invalide.txt", "boite 10 10\nce n'est pas un scénario\n");
    let sans_boite = ecrire("sans-boite.txt", &format!("spore 1 1\n{base}moule brique 0 0 3 3\n"));
    let en_double =
        ecrire("double.txt", &format!("boite 10 10\nspore 1 1\n{base}moule brique 0 0 3 3\nmoule brique 0 0 4 4\n"));
    let moule_inconnu = ecrire("inconnu.txt", &format!("boite 10 10\nspore 1 1\n{base}moule cube 1 2 3\n"));
    let mot_faux = ecrire("mot-faux.txt", &format!("boite 10 10\nspore 1 1\n{base}moule brique 0 0 -3 3\n"));
    let forme_invalide = ecrire("forme-invalide.txt", &format!("boite 10 10\nspore 1 1\n{base}moule l 0 0 5 5 0\n"));
    let hors = ecrire("hors.txt", &format!("boite 10 10\nspore 1 1\n{base}moule disque 2 2 5\n"));
    let spore_dans_un_mur =
        ecrire("spore-dans-un-mur.txt", &format!("boite 10 10\nspore 0 0\n{base}moule brique 5 5 8 8\n"));
    let sans_spore = ecrire("sans-spore.txt", &format!("boite 10 10\n{base}moule brique 0 0 3 3\n"));
    let mur = "scenarios/moules/brique.txt";
    let cas: [(&[&str], &str); 18] = [
        (&[], "aucun argument"),
        (&[mur], "pas de graine"),
        (&[mur, "1", dossier, "de trop"], "quatre arguments"),
        (&["scenarios/moules/introuvable.txt", "1", dossier], "scénario introuvable"),
        (&["brique", "1", dossier], "un nom nu n'est pas un chemin"),
        (&[&invalide, "1", dossier], "scénario invalide"),
        (&[&sans_boite, "1", dossier], "moule sans boîte"),
        (&[&en_double, "1", dossier], "deux lignes moule"),
        (&[&moule_inconnu, "1", dossier], "forme inconnue"),
        (&[&mot_faux, "1", dossier], "mot qui n'est pas un entier"),
        (&[&forme_invalide, "1", dossier], "forme hors contrat"),
        (&[&hors, "1", dossier], "cavité hors de la boîte"),
        (&[&spore_dans_un_mur, "1", dossier], "spore dans un obstacle"),
        (&[&sans_spore, "1", dossier], "scénario sans spore"),
        (&[mur, "abc", dossier], "graine illisible"),
        (&[mur, "-1", dossier], "graine négative"),
        (&[mur, "", dossier], "graine vide"),
        (&[mur, "18446744073709551616", dossier], "graine plus grande que 2⁶⁴ − 1"),
    ];
    for (args, raison) in cas {
        let resultat = lancer(args);
        assert_eq!(resultat.status.code(), Some(2), "{raison} : l'exemple sort avec le code 2");
        assert!(resultat.stdout.is_empty(), "{raison} : rien sur la sortie standard");
        assert!(!resultat.stderr.is_empty(), "{raison} : un message d'erreur");
        assert!(!Path::new(dossier).exists(), "{raison} : rien n'est écrit");
    }
    // La plus grande graine permise passe.
    let resultat = lancer(&[mur, "18446744073709551615"]);
    assert!(resultat.status.success(), "{}", String::from_utf8_lossy(&resultat.stderr));
    // Un dossier de sortie qui ne peut pas se créer (c'est un fichier) : le code 2, et aucune mesure imprimée.
    let fichier = sortie.join("un-fichier");
    std::fs::write(&fichier, "pas un dossier").unwrap();
    let resultat = lancer(&[mur, "31", fichier.to_str().unwrap()]);
    assert_eq!(resultat.status.code(), Some(2), "un dossier de sortie impossible");
    assert!(resultat.stdout.is_empty(), "un dossier de sortie impossible : rien sur la sortie standard");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_dossier_de_sortie_est_cree_avec_ses_parents_et_l_image_suit_la_boite_non_carree() {
    delai();
    // Une boîte de 30 × 20 (non carrée : un échange de la largeur et de la hauteur de l'image se verrait), et un dossier de
    // sortie qui n'existe pas, deux niveaux sous un dossier neuf : l'exemple le crée.
    let texte = "boite 30 20\nspore 15 10\nsubstrat 1.0 0.2\ncroissance 1.0 8.0 35 20\ncouplage 0.1 0.002\nanastomose 0.5 8\n\
                 duree 60\nthigmotropisme 35\nmoule brique 5 5 24 14\n";
    let fichier = ranger("non-carree", texte);
    let racine = dossier_neuf("parents");
    let dossier = racine.join("a").join("b");
    assert!(!dossier.exists());
    let sortie = texte_de_sortie(&lancer(&[&fichier, "3", dossier.to_str().unwrap()]));
    let (attendu, simulation) = sortie_attendue(texte, 3, None);
    assert_eq!(sortie, attendu);
    let image = std::fs::read(dossier.join("densite.png")).expect("densite.png est écrit dans le dossier créé");
    verifier_l_image(&image, &simulation, 4);
    let png = relire(&image);
    assert_eq!((png.largeur, png.hauteur), (120, 80), "30 × 20 cases à 4 pixels : la largeur avant la hauteur");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_nombre_de_pixels_par_case_suit_la_regle_des_490_000_octets() {
    delai();
    // 101 × 101 cases : à 4 pixels par case, les octets bruts feraient 404 × (3 × 404 + 1) = 490 052 > 490 000, donc 3 pixels par
    // case (303 × 303 ; une boîte de 100 × 100 tient à 4 : 400 × 1 201 = 480 400).
    let texte = "boite 101 101\nspore 50 50\nsubstrat 1.0 0.2\ncroissance 1.0 8.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\n\
                 duree 40\nthigmotropisme 35\nmoule disque 50 50 20\n";
    let fichier = ranger("cent-un", texte);
    let dossier = dossier_neuf("cent-un-image");
    let sortie = texte_de_sortie(&lancer(&[&fichier, "1", dossier.to_str().unwrap()]));
    let (attendu, simulation) = sortie_attendue(texte, 1, None);
    assert_eq!(sortie, attendu);
    let image = std::fs::read(dossier.join("densite.png")).unwrap();
    verifier_l_image(&image, &simulation, 3);
    let png = relire(&image);
    assert_eq!((png.largeur, png.hauteur), (303, 303));
}
