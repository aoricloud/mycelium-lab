//! Tests d'acceptation de l'étape 0 : modules `alea` et `empreinte`.

use mycelium_lab::alea::Alea;
use mycelium_lab::empreinte::{sha256, sha256_hex};

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn alea_meme_graine_meme_suite() {
    let mut a = Alea::new(42);
    let mut b = Alea::new(42);
    let suite_a: Vec<u64> = (0..1000).map(|_| a.suivant()).collect();
    let suite_b: Vec<u64> = (0..1000).map(|_| b.suivant()).collect();
    assert_eq!(suite_a, suite_b);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn alea_graines_differentes_suites_differentes() {
    let mut a = Alea::new(1);
    let mut b = Alea::new(2);
    let suite_a: Vec<u64> = (0..1000).map(|_| a.suivant()).collect();
    let suite_b: Vec<u64> = (0..1000).map(|_| b.suivant()).collect();
    assert_ne!(suite_a, suite_b);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn empreinte_sha256_du_vide() {
    assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn empreinte_sha256_de_abc() {
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(sha256(b"abc")[0], 0xba);
}

// ---------------------------------------------------------------------------------------------
// scenario, grille et empreinte
// ---------------------------------------------------------------------------------------------

use mycelium_lab::grille::Etat;
use mycelium_lab::scenario::Scenario;
use std::path::PathBuf;
use std::process::Command;

const SCENARIO_VIDE: &str = "boite 64 48\n";

fn etat(graine: u64) -> Etat {
    let scenario = Scenario::lire(SCENARIO_VIDE).expect("le scénario vide se lit");
    Etat::nouveau(&scenario, graine)
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn scenario_vide_se_lit() {
    let scenario = Scenario::lire(SCENARIO_VIDE).expect("le scénario vide se lit");
    assert_eq!(scenario.largeur, 64);
    assert_eq!(scenario.hauteur, 48);
    assert!(scenario.spores.is_empty());
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn etat_porte_boite_et_graine() {
    let e = etat(7);
    assert_eq!((e.largeur, e.hauteur, e.graine), (64, 48, 7));
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn meme_scenario_meme_graine_meme_empreinte() {
    let a = etat(42).empreinte();
    let b = etat(42).empreinte();
    assert_eq!(a, b);
    assert_eq!(a.len(), 64);
    assert!(a.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn deux_graines_deux_empreintes() {
    assert_ne!(etat(1).empreinte(), etat(2).empreinte());
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn ecriture_canonique_stable_et_en_dur() {
    let e = etat(1);
    let premiere = e.ecrire_canonique();
    assert_eq!(premiere, e.ecrire_canonique());
    // largeur (u32 LE) = 64, hauteur (u32 LE) = 48, graine (u64 LE) = 1.
    let attendu: [u8; 16] = [
        0x40, 0x00, 0x00, 0x00, // largeur
        0x30, 0x00, 0x00, 0x00, // hauteur
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // graine
    ];
    assert_eq!(premiere, attendu);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn empreinte_est_le_sha256_de_l_ecriture_canonique() {
    let e = etat(1);
    assert_eq!(e.empreinte(), sha256_hex(&e.ecrire_canonique()));
}

// ---------------------------------------------------------------------------------------------
// le binaire
// ---------------------------------------------------------------------------------------------

/// Dossier temporaire propre au test, nettoyé à la chute.
struct Temp(PathBuf);

impl Temp {
    fn nouveau(nom: &str) -> Temp {
        let dossier = std::env::temp_dir().join(format!("mycelium_socle_{}_{}", nom, std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("création du dossier temporaire");
        Temp(dossier)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Lance le binaire sur le scénario vide et rend le contenu de `empreinte.txt`.
fn lancer(temp: &Temp, graine: u64, sortie: &str) -> String {
    let scenario = temp.0.join("vide.txt");
    std::fs::write(&scenario, SCENARIO_VIDE).expect("écriture du scénario");
    let dossier_sortie = temp.0.join(sortie);
    let statut = Command::new(env!("CARGO_BIN_EXE_mycelium"))
        .arg("lancer")
        .arg(&scenario)
        .arg("--graine")
        .arg(graine.to_string())
        .arg("--sortie")
        .arg(&dossier_sortie)
        .status()
        .expect("lancement du binaire");
    assert_eq!(statut.code(), Some(0));
    std::fs::read_to_string(dossier_sortie.join("empreinte.txt")).expect("empreinte.txt écrit")
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn binaire_ecrit_l_empreinte_de_la_bibliotheque() {
    let temp = Temp::nouveau("biblio");
    let contenu = lancer(&temp, 42, "sortie");
    assert_eq!(contenu, format!("{}\n", etat(42).empreinte()));
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn binaire_meme_graine_meme_fichier() {
    let temp = Temp::nouveau("meme");
    let a = lancer(&temp, 5, "a");
    let b = lancer(&temp, 5, "b");
    assert_eq!(a, b);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn binaire_deux_graines_deux_fichiers() {
    let temp = Temp::nouveau("deux");
    let a = lancer(&temp, 5, "a");
    let b = lancer(&temp, 6, "b");
    assert_ne!(a, b);
}
