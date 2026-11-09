//! Tests d'acceptation de l'étape 9 (M0, mesurer la pièce), partie « l'exemple `mesure_piece` ».
//!
//! `cargo run --release --example mesure_piece -- <scénario> <graine> [<dossier de sortie>]` lit le scénario, joue sa `duree`,
//! imprime les mesures et, avec un dossier, écrit `densite.png`. Ces tests lancent l'exemple compilé (le binaire est cherché à
//! côté du binaire de test, sous `examples/` ; s'il manque, il est construit par `cargo build --example mesure_piece`).
//!
//! - **Ce qu'il imprime** : exactement quatorze lignes `nom valeur`, dans l'ordre des champs de `MesuresPiece` (le nom du
//!   champ, une espace, la valeur : les entiers en décimal, les `f64` par `{}`, relus au bit près, `aucune` pour une
//!   traversée absente), avec les valeurs que donne `mesurer` sur la même simulation (jouée ici sans l'exemple).
//! - **Ce qu'il écrit** : `densite.png`, qui pèse moins de 500 Ko, qui se relit (signature, blocs, sommes de contrôle, flux
//!   zlib en blocs stockés, 8 bits RGB), à `k` pixels par case (`k = 4` pour les boîtes de 100 × 100 et les petites
//!   boîtes, `k = 2` pour la boîte de 161 × 161), où chaque case est un carré uniforme de **la couleur exacte que fixe la
//!   spécification** : gris (48, 48, 48) pour une case bloquée, fond (16, 16, 24) pour une case libre sans hyphe, la rampe
//!   entière du violet (70, 20, 110) au jaune (255, 240, 90) pour une case occupée. La position de chaque case est vérifiée
//!   (`x` et `y` ne sont pas échangés, la ligne 0 est en haut) sur le mur (obstacle plus haut que large) et sur des boîtes
//!   qui ne sont pas carrées.
//! - **Ses refus** : sans argument, avec trop d'arguments, une graine illisible, un scénario introuvable ou invalide, il
//!   sort avec le code 2, n'imprime rien sur la sortie standard et n'écrit rien.
//! - **Il est déterministe** : deux lancers donnent les mêmes lignes et les mêmes octets de PNG.
//! - **Sans dossier de sortie, il n'écrit aucun fichier.**
//! - **Le binaire `mycelium` n'est pas modifié** : son code ne parle pas de la pièce (vérifié dans le test du déterminisme).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `sqrt`, `min` et `max`.

#[path = "etape_9/outils.rs"]
mod outils;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use mycelium_lab::image::crc32;
use mycelium_lab::piece::{MesuresPiece, carte_de_densite, mesurer};
use mycelium_lab::simulation::Simulation;
use outils::{AUTRES_SCENARIOS, construire, delai, texte_du_fichier};

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
        let binaire = profil.join("examples").join(format!("mesure_piece{}", std::env::consts::EXE_SUFFIX));
        if !binaire.exists() {
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
            let mut commande = Command::new(cargo);
            commande.args(["build", "--example", "mesure_piece"]).current_dir(env!("CARGO_MANIFEST_DIR"));
            if profil.file_name().is_some_and(|n| n == "release") {
                commande.arg("--release");
            }
            let statut = commande.status().expect("cargo se lance");
            assert!(statut.success(), "cargo build --example mesure_piece a échoué");
        }
        assert!(binaire.exists(), "l'exemple mesure_piece est introuvable : {}", binaire.display());
        binaire
    })
}

/// Un dossier neuf (vidé s'il existait) sous le dossier temporaire des tests de Cargo.
fn dossier_neuf(nom: &str) -> PathBuf {
    let dossier = Path::new(env!("CARGO_TARGET_TMPDIR")).join("etape_9_exemple").join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).expect("le dossier de test se crée");
    dossier
}

/// Lance l'exemple depuis la racine du dépôt.
fn lancer(args: &[&str]) -> Output {
    Command::new(exemple()).args(args).current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("l'exemple se lance")
}

/// Le chemin (relatif à la racine du dépôt) d'un scénario de `scenarios/`.
fn scenario(fichier: &str) -> String {
    format!("scenarios/{fichier}")
}

/// Ce que l'exemple doit imprimer pour ces mesures, écrit ici d'après la spécification.
fn sortie_attendue(m: &MesuresPiece) -> String {
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

/// Compare ce que l'exemple a imprimé à ce que `mesurer` donne sur la même simulation, jouée ici : quatorze lignes
/// `nom valeur`, dans l'ordre, chaque nombre relu au bit près.
fn comparer_aux_mesures(sortie: &str, simulation: &Simulation) {
    let m = mesurer(simulation);
    let lignes: Vec<&str> = sortie.lines().collect();
    assert_eq!(lignes.len(), CHAMPS.len(), "quatorze lignes, et rien d'autre :\n{sortie}");
    for (ligne, champ) in lignes.iter().zip(CHAMPS) {
        let (nom, valeur) = ligne.split_once(' ').unwrap_or_else(|| panic!("`nom valeur` : {ligne}"));
        assert_eq!(nom, champ, "les champs dans l'ordre de la structure :\n{sortie}");
        assert!(!valeur.is_empty() && !valeur.contains(' '), "une seule valeur : {ligne}");
    }
    // Les nombres relus : un `f64` écrit par `{}` se relit au bit près.
    let lire = |i: usize| lignes[i].split_once(' ').expect("nom valeur").1;
    for (i, attendu) in [m.remplissage, m.densite_moyenne, m.densite_maximale, m.biomasse, m.nourriture_initiale]
        .into_iter()
        .enumerate()
    {
        let lu: f64 = lire(2 + i).parse().expect("un nombre");
        assert_eq!(lu.to_bits(), attendu.to_bits(), "{} : {lu} au lieu de {attendu}", CHAMPS[2 + i]);
    }
    let traversee = |t: Option<usize>| t.map_or_else(|| "aucune".to_string(), |n| n.to_string());
    assert_eq!(lire(12), traversee(m.traversee_gauche_droite), "traversee_gauche_droite");
    assert_eq!(lire(13), traversee(m.traversee_haut_bas), "traversee_haut_bas");
    // Et toute la sortie, telle que la spécification l'écrit.
    assert_eq!(sortie, sortie_attendue(&m), "la sortie de l'exemple");
}

// ---------------------------------------------------------------------------------------------------------------------
// Lire un PNG
// ---------------------------------------------------------------------------------------------------------------------

/// Une image relue : largeur, hauteur et pixels RGB, ligne par ligne.
struct Png {
    largeur: usize,
    hauteur: usize,
    pixels: Vec<u8>,
}

impl Png {
    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.largeur + x) * 3;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]]
    }
}

/// Relit un PNG écrit par `ecrire_png` : la signature, les blocs et leurs sommes de contrôle, l'en-tête (8 bits, RGB), le
/// flux zlib en blocs stockés, le filtre 0 de chaque ligne.
fn relire(octets: &[u8]) -> Png {
    assert_eq!(&octets[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A], "la signature PNG");
    let (mut i, mut idat) = (8usize, Vec::new());
    let (mut largeur, mut hauteur) = (0usize, 0usize);
    loop {
        let longueur = u32::from_be_bytes(octets[i..i + 4].try_into().unwrap()) as usize;
        let type_ = &octets[i + 4..i + 8];
        let donnees = &octets[i + 8..i + 8 + longueur];
        let somme = u32::from_be_bytes(octets[i + 8 + longueur..i + 12 + longueur].try_into().unwrap());
        assert_eq!(crc32(&octets[i + 4..i + 8 + longueur]), somme, "la somme de contrôle du bloc {type_:?}");
        match type_ {
            b"IHDR" => {
                largeur = u32::from_be_bytes(donnees[0..4].try_into().unwrap()) as usize;
                hauteur = u32::from_be_bytes(donnees[4..8].try_into().unwrap()) as usize;
                assert_eq!(&donnees[8..13], &[8, 2, 0, 0, 0], "8 bits, RGB, sans entrelacement");
            }
            b"IDAT" => idat.extend_from_slice(donnees),
            b"IEND" => break,
            _ => {}
        }
        i += 12 + longueur;
    }
    let mut brut = Vec::new();
    let mut j = 2;
    loop {
        let drapeau = idat[j];
        assert_eq!(drapeau & 0b110, 0, "le flux zlib est en blocs stockés");
        let n = u16::from_le_bytes([idat[j + 1], idat[j + 2]]) as usize;
        j += 5;
        brut.extend_from_slice(&idat[j..j + n]);
        j += n;
        if drapeau & 1 == 1 {
            break;
        }
    }
    let ligne = 1 + 3 * largeur;
    assert_eq!(brut.len(), ligne * hauteur, "les octets bruts : une ligne de filtre plus des pixels, par ligne");
    let mut pixels = Vec::new();
    for rang in brut.chunks(ligne) {
        assert_eq!(rang[0], 0, "filtre 0");
        pixels.extend_from_slice(&rang[1..]);
    }
    Png { largeur, hauteur, pixels }
}

const GRIS_BLOQUE: [u8; 3] = [48, 48, 48];
const FOND: [u8; 3] = [16, 16, 24];
const SOMBRE: [i32; 3] = [70, 20, 110];
const CLAIR: [i32; 3] = [255, 240, 90];

/// La couleur d'une case occupée de densité `d` (`d > 0`), d'après la rampe de la spécification : `niveau =
/// (sqrt(min(d / dmax, 1)) × 255) as i32`, puis `sombre + (clair − sombre) × niveau / 255` en entiers, canal par canal.
fn couleur_attendue(d: f64, dmax: f64) -> [u8; 3] {
    let niveau = ((d / dmax).min(1.0).sqrt() * 255.0) as i32;
    [0, 1, 2].map(|c| (SOMBRE[c] + (CLAIR[c] - SOMBRE[c]) * niveau / 255) as u8)
}

/// Vérifie `densite.png` contre la simulation : sa forme (`k` pixels par case, `k` imposé), puis chaque case, un carré
/// uniforme de la couleur exacte que la spécification lui donne.
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
    // Garde : la rampe est parcourue (des niveaux bas et le niveau 255 de la case la plus dense).
    niveaux.sort_unstable();
    niveaux.dedup();
    assert!(niveaux.len() >= 8 && niveaux.last() == Some(&255), "garde : niveaux {niveaux:?}");
}

/// Joue un scénario du dépôt, de la graine donnée, pendant sa durée.
fn jouer(fichier: &str, graine: u64) -> Simulation {
    let (mut simulation, _, duree) = construire(&texte_du_fichier(fichier), graine);
    simulation.avancer_n(duree);
    simulation
}

// ---------------------------------------------------------------------------------------------------------------------
// Les tests
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn l_exemple_imprime_les_mesures_et_ecrit_l_image_du_mur() {
    delai();
    // La rampe écrite ici, vérifiée d'abord sur ses points connus : le bas, le haut, le quart (racine 1/2).
    assert_eq!(couleur_attendue(1e-300, 1.0), [70, 20, 110]);
    assert_eq!(couleur_attendue(1.0, 1.0), [255, 240, 90]);
    assert_eq!(couleur_attendue(3.0, 2.0), [255, 240, 90], "au-delà du maximum, le haut de la rampe");
    assert_eq!(couleur_attendue(0.25, 1.0), [162, 129, 101], "niveau 127 (division entière vers zéro : 110 − 9)");
    let sortie = dossier_neuf("mur");
    let resultat = lancer(&[&scenario("mur.txt"), "31", sortie.to_str().unwrap()]);
    assert!(resultat.status.success(), "l'exemple échoue : {}", String::from_utf8_lossy(&resultat.stderr));
    let simulation = jouer("mur.txt", 31);
    comparer_aux_mesures(&String::from_utf8_lossy(&resultat.stdout), &simulation);
    // Le mur coupe la traversée gauche-droite : la ligne dit `aucune`.
    assert!(String::from_utf8_lossy(&resultat.stdout).contains("\ntraversee_gauche_droite aucune\n"));
    let image = std::fs::read(sortie.join("densite.png")).expect("densite.png est écrit");
    verifier_l_image(&image, &simulation, 4);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn l_image_des_autres_scenarios_de_la_vitrine_est_juste_et_pese_moins_de_500_ko() {
    delai();
    // Le couplage (161 × 161 : 2 pixels par case pour rester sous 500 Ko), le terrain (deux obstacles), la tache.
    for (fichier, graine, k) in [("couplage.txt", 1u64, 2), ("terrain.txt", 1, 4), ("chimiotropisme.txt", 7, 4)] {
        let sortie = dossier_neuf(fichier);
        let resultat = lancer(&[&scenario(fichier), &graine.to_string(), sortie.to_str().unwrap()]);
        assert!(resultat.status.success(), "{fichier} : {}", String::from_utf8_lossy(&resultat.stderr));
        let simulation = jouer(fichier, graine);
        comparer_aux_mesures(&String::from_utf8_lossy(&resultat.stdout), &simulation);
        let image = std::fs::read(sortie.join("densite.png")).expect("densite.png est écrit");
        verifier_l_image(&image, &simulation, k);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn l_exemple_dessine_une_boite_qui_n_est_pas_carree_sans_echanger_x_et_y() {
    delai();
    for (nom, texte) in AUTRES_SCENARIOS {
        let dossier = dossier_neuf(nom);
        let fichier = dossier.join("scenario.txt");
        std::fs::write(&fichier, texte).expect("le scénario se range");
        // Le dossier de sortie n'existe pas encore : l'exemple le crée.
        let sortie = dossier.join("sortie").join("plus-loin");
        let resultat = lancer(&[fichier.to_str().unwrap(), "2", sortie.to_str().unwrap()]);
        assert!(resultat.status.success(), "{nom} : {}", String::from_utf8_lossy(&resultat.stderr));
        let (mut simulation, _, duree) = construire(texte, 2);
        simulation.avancer_n(duree);
        comparer_aux_mesures(&String::from_utf8_lossy(&resultat.stdout), &simulation);
        let image = std::fs::read(sortie.join("densite.png")).expect("densite.png est écrit");
        verifier_l_image(&image, &simulation, 4);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn le_nombre_de_pixels_par_case_suit_la_regle_des_490_000_octets() {
    delai();
    // `k` est le plus grand entier de 4 à 1 tel que `(hauteur k) × (3 largeur k + 1) ≤ 490 000` : 4 pour 100 × 100
    // (480 400 octets), 3 pour 101 × 101 (à 4, 490 052 octets), 2 pour 161 × 161 (à 3, 700 350 octets).
    let texte = "boite 101 101
spore 50 50
substrat 1.0 0.2
croissance 1.0 8.0 35 20
couplage 0.1 0.002
                 anastomose 0.5 8
duree 40
obstacle 70 10 72 90
";
    let dossier = dossier_neuf("cent-un");
    let fichier = dossier.join("scenario.txt");
    std::fs::write(&fichier, texte).expect("le scénario se range");
    let sortie = dossier.join("sortie");
    let resultat = lancer(&[fichier.to_str().unwrap(), "5", sortie.to_str().unwrap()]);
    assert!(resultat.status.success(), "{}", String::from_utf8_lossy(&resultat.stderr));
    let (mut simulation, _, duree) = construire(texte, 5);
    simulation.avancer_n(duree);
    comparer_aux_mesures(&String::from_utf8_lossy(&resultat.stdout), &simulation);
    let image = std::fs::read(sortie.join("densite.png")).expect("densite.png est écrit");
    verifier_l_image(&image, &simulation, 3);
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn l_exemple_est_deterministe_et_sans_dossier_il_n_ecrit_rien() {
    delai();
    let a = dossier_neuf("det-a");
    let b = dossier_neuf("det-b");
    let un = lancer(&[&scenario("mur-sans.txt"), "31", a.to_str().unwrap()]);
    let deux = lancer(&[&scenario("mur-sans.txt"), "31", b.to_str().unwrap()]);
    assert!(un.status.success() && deux.status.success());
    assert_eq!(un.stdout, deux.stdout, "les mêmes lignes");
    assert_eq!(
        std::fs::read(a.join("densite.png")).unwrap(),
        std::fs::read(b.join("densite.png")).unwrap(),
        "les mêmes octets de PNG"
    );
    // Une autre graine : d'autres mesures.
    let autre = lancer(&[&scenario("mur-sans.txt"), "32"]);
    assert!(autre.status.success());
    assert_ne!(un.stdout, autre.stdout, "une autre graine donne d'autres mesures");
    comparer_aux_mesures(&String::from_utf8_lossy(&autre.stdout), &jouer("mur-sans.txt", 32));

    // Sans dossier : aucune écriture, ni dans le dépôt ni dans le dossier courant.
    let courant = dossier_neuf("sans-dossier");
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join(scenario("mur.txt"));
    let sans = Command::new(exemple()).args([chemin.to_str().unwrap(), "31"]).current_dir(&courant).output().unwrap();
    assert!(sans.status.success(), "{}", String::from_utf8_lossy(&sans.stderr));
    comparer_aux_mesures(&String::from_utf8_lossy(&sans.stdout), &jouer("mur.txt", 31));
    let ecrits: Vec<_> = std::fs::read_dir(&courant).unwrap().collect();
    assert!(ecrits.is_empty(), "sans dossier de sortie, l'exemple n'écrit rien : {ecrits:?}");

    // Le binaire `mycelium` n'est pas modifié par M0 : son code ne parle pas de la pièce.
    let main = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs")).unwrap();
    assert!(!main.contains("piece") && !main.contains("mesurer"), "`src/main.rs` n'est pas modifié par M0");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn l_exemple_refuse_une_commande_fausse_sans_rien_ecrire() {
    delai();
    let sortie = dossier_neuf("refus");
    let dossier = sortie.join("sortie");
    let dossier = dossier.to_str().unwrap();
    let invalide = sortie.join("invalide.txt");
    std::fs::write(&invalide, "boite 10 10\nce n'est pas un scénario\n").unwrap();
    let invalide = invalide.to_str().unwrap();
    let mur = scenario("mur.txt");
    let cas: [(&[&str], &str); 9] = [
        (&[], "aucun argument"),
        (&[&mur], "pas de graine"),
        (&[&mur, "1", dossier, "de trop"], "quatre arguments"),
        (&["scenarios/introuvable.txt", "1", dossier], "scénario introuvable"),
        (&["mur", "1", dossier], "un nom nu n'est pas un chemin"),
        (&[invalide, "1", dossier], "scénario invalide"),
        (&[&mur, "abc", dossier], "graine illisible"),
        (&[&mur, "-1", dossier], "graine négative"),
        (&[&mur, "", dossier], "graine vide"),
    ];
    for (args, raison) in cas {
        let resultat = lancer(args);
        assert_eq!(resultat.status.code(), Some(2), "{raison} : l'exemple sort avec le code 2");
        assert!(resultat.stdout.is_empty(), "{raison} : rien sur la sortie standard");
        assert!(!resultat.stderr.is_empty(), "{raison} : un message d'erreur");
        assert!(!Path::new(dossier).exists(), "{raison} : rien n'est écrit");
    }
    // Un dossier de sortie qui ne peut pas se créer (c'est un fichier) : le code 2, et aucune mesure imprimée.
    let fichier = sortie.join("un-fichier");
    std::fs::write(&fichier, "pas un dossier").unwrap();
    let resultat = lancer(&[&mur, "31", fichier.to_str().unwrap()]);
    assert_eq!(resultat.status.code(), Some(2), "un dossier de sortie impossible");
    assert!(resultat.stdout.is_empty(), "un dossier de sortie impossible : rien sur la sortie standard");
}
