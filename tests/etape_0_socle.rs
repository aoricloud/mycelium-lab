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
