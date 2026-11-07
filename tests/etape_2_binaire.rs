//! Tests d'acceptation de l'étape 2 : le binaire `mycelium lancer`.
//!
//! Ils lancent le binaire sur `scenarios/couplage.txt` et comparent ce qu'il écrit à ce que rend la bibliothèque.
//! Le scénario de l'étape 0 (`boite 64 48`) garde son propre chemin, celui de [`mycelium_lab::grille::Etat`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mycelium_lab::grille::Etat;
use mycelium_lab::releves::{ENTETE_CSV, en_csv};
use mycelium_lab::scenario::Scenario;
use mycelium_lab::simulation::{Simulation, lire_scenario};

/// Compteur d'instances : deux tests lancés en parallèle ne partagent jamais leur dossier temporaire.
static COMPTEUR: AtomicU64 = AtomicU64::new(0);

/// Un dossier temporaire propre au test, nettoyé à la chute.
struct Temp(PathBuf);

impl Temp {
    fn nouveau(nom: &str) -> Temp {
        let numero = COMPTEUR.fetch_add(1, Ordering::Relaxed);
        let dossier = std::env::temp_dir().join(format!("mycelium_binaire_{nom}_{}_{numero}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("création du dossier temporaire");
        Temp(dossier)
    }

    /// Un dossier de sortie neuf, encore inexistant, sous le dossier temporaire.
    fn sortie(&self, nom: &str) -> PathBuf {
        let dossier = self.0.join(nom);
        let _ = std::fs::remove_dir_all(&dossier);
        dossier
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Le scénario couplé du dépôt, par son chemin depuis la racine du paquet.
fn scenario_couplage() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join("couplage.txt")
}

/// Lance `mycelium lancer <scenario> --graine <graine> --sortie <sortie>` et exige le succès.
fn lancer(scenario: &Path, graine: u64, sortie: &Path) {
    let statut = Command::new(env!("CARGO_BIN_EXE_mycelium"))
        .arg("lancer")
        .arg(scenario)
        .arg("--graine")
        .arg(graine.to_string())
        .arg("--sortie")
        .arg(sortie)
        .status()
        .expect("lancement du binaire");
    assert_eq!(statut.code(), Some(0), "le binaire a échoué");
}

/// Lit un fichier du dossier de sortie.
fn lire(sortie: &Path, nom: &str) -> Vec<u8> {
    std::fs::read(sortie.join(nom)).unwrap_or_else(|e| panic!("lecture de {nom} : {e}"))
}

/// Vrai pour exactement 64 hexadécimaux minuscules suivis d'un saut de ligne.
fn est_empreinte(octets: &[u8]) -> bool {
    octets.len() == 65
        && octets[64] == b'\n'
        && octets[..64].iter().all(|o| o.is_ascii_digit() || (b'a'..=b'f').contains(o))
}

/// 1. Le binaire réussit et écrit `empreinte.txt`, `mesures.csv` et `apercu.png` (signature PNG).
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn le_binaire_ecrit_les_trois_fichiers() {
    let temp = Temp::nouveau("trois_fichiers");
    let sortie = temp.sortie("sortie");
    lancer(&scenario_couplage(), 1, &sortie);

    let empreinte = lire(&sortie, "empreinte.txt");
    assert!(est_empreinte(&empreinte), "empreinte.txt inattendu : {:?}", String::from_utf8_lossy(&empreinte));

    let mesures = lire(&sortie, "mesures.csv");
    assert!(!mesures.is_empty(), "mesures.csv vide");

    let apercu = lire(&sortie, "apercu.png");
    const SIGNATURE_PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    assert!(apercu.starts_with(&SIGNATURE_PNG), "apercu.png n'a pas la signature PNG");
}

/// 2. Le CSV et l'empreinte écrits sont exactement ceux de la simulation de la bibliothèque, et le CSV porte
///    l'en-tête et 81 relevés (temps 0 à 80).
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn les_fichiers_du_binaire_sont_ceux_de_la_simulation() {
    let temp = Temp::nouveau("simulation");
    let sortie = temp.sortie("sortie");
    let scenario = scenario_couplage();
    lancer(&scenario, 1, &sortie);

    let texte = std::fs::read_to_string(&scenario).expect("lecture du scénario couplé");
    let (reglages, duree) = lire_scenario(&texte).expect("le scénario couplé se lit");
    let mut simulation = Simulation::nouvelle(reglages, 1).expect("réglages valides");
    simulation.avancer_n(duree);

    let attendu = en_csv(simulation.releves());
    let obtenu = String::from_utf8(lire(&sortie, "mesures.csv")).expect("mesures.csv est de l'UTF-8");
    assert_eq!(obtenu, attendu, "le CSV du binaire diffère de celui de la simulation");

    let lignes: Vec<&str> = obtenu.lines().collect();
    assert_eq!(lignes.first().copied(), Some(ENTETE_CSV), "en-tête du CSV");
    assert_eq!(lignes.len(), 1 + 81, "l'en-tête et 81 relevés (temps 0 à 80)");
    for (temps, ligne) in lignes.iter().skip(1).enumerate() {
        let champ = ligne.split(',').next().expect("au moins un champ");
        assert_eq!(champ.parse::<u32>().expect("temps entier"), temps as u32, "temps du relevé {temps}");
    }

    let empreinte = String::from_utf8(lire(&sortie, "empreinte.txt")).expect("empreinte.txt est de l'UTF-8");
    assert_eq!(
        empreinte,
        format!("{}\n", simulation.empreinte()),
        "l'empreinte du binaire diffère de celle de la simulation"
    );
}

/// 3. Même graine, mêmes trois fichiers à l'octet ; la graine 2 donne une autre empreinte et un autre CSV.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn meme_graine_memes_fichiers_deux_graines_autres() {
    let temp = Temp::nouveau("reproductibilite");
    let scenario = scenario_couplage();
    let (a, b, c) = (temp.sortie("a"), temp.sortie("b"), temp.sortie("c"));

    lancer(&scenario, 1, &a);
    lancer(&scenario, 1, &b);
    for nom in ["empreinte.txt", "mesures.csv", "apercu.png"] {
        assert_eq!(lire(&a, nom), lire(&b, nom), "{nom} diffère pour la même graine");
    }

    lancer(&scenario, 2, &c);
    assert_ne!(lire(&a, "empreinte.txt"), lire(&c, "empreinte.txt"), "la graine 2 donne la même empreinte");
    assert_ne!(lire(&a, "mesures.csv"), lire(&c, "mesures.csv"), "la graine 2 donne le même CSV");
}

/// 4. L'empreinte écrite pour la graine 1 est celle de `references/couplage-graine-1.empreinte`.
///
/// La référence est lue au moment du test, pas par `include_str!` : la vague du code la produit en lançant le
/// binaire, donc elle peut ne pas exister au moment de la compilation de ces tests.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_empreinte_de_la_graine_1_est_la_reference() {
    let temp = Temp::nouveau("reference");
    let sortie = temp.sortie("sortie");
    lancer(&scenario_couplage(), 1, &sortie);

    let obtenue = String::from_utf8(lire(&sortie, "empreinte.txt")).expect("empreinte.txt est de l'UTF-8");
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join("couplage-graine-1.empreinte");
    let reference =
        std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("lecture de {} : {e}", chemin.display()));

    assert!(est_empreinte(reference.as_bytes()), "référence inattendue : {reference:?}");
    assert_eq!(obtenue, reference, "l'empreinte de la graine 1 n'est pas la référence");
}

/// 5. Un scénario de l'étape 0 garde son chemin : `empreinte.txt` est celle de `Etat`, en un seul lancement.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn un_scenario_de_l_etape_0_garde_le_chemin_de_l_etape_0() {
    let temp = Temp::nouveau("etape_0");
    let scenario = temp.0.join("boite.txt");
    std::fs::write(&scenario, "boite 64 48\n").expect("écriture du scénario de l'étape 0");
    let sortie = temp.sortie("sortie");

    lancer(&scenario, 42, &sortie);

    let obtenue = String::from_utf8(lire(&sortie, "empreinte.txt")).expect("empreinte.txt est de l'UTF-8");
    let texte = std::fs::read_to_string(&scenario).expect("lecture du scénario de l'étape 0");
    let etat = Etat::nouveau(&Scenario::lire(&texte).expect("scénario de l'étape 0 lisible"), 42);
    assert_eq!(
        obtenue,
        format!("{}\n", etat.empreinte()),
        "le scénario de l'étape 0 n'a pas suivi le chemin de l'étape 0"
    );
}
