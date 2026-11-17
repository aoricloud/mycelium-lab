//! Les bibliothèques copiées dans `docs/bibliotheques/` : three.js 0.185.1, octet pour octet.
//!
//! Rien ne se charge d'un autre site : les fichiers sont dans le dépôt, fixés par leur SHA-256, et leur notice
//! (`LISEZ-MOI.md`) nomme la version, la licence et les empreintes. Changer de version, c'est changer les trois
//! fichiers, la notice et les constantes ci-dessous dans la même PR. Ce test n'a pas de feature : il tourne toujours.

use mycelium_lab::empreinte::sha256_hex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SHA_MODULE: &str = "86bcee248b64f44bcfc23c331ae74619061957d59cab040171dcb6fb5900beb6";
const SHA_CORE: &str = "05b2609338c76cd65daf74f3ac515bc9a5045e1b3b33edc07d8c9bd55250fa90";
const SHA_LICENCE: &str = "8b378ebe60e2fe500158cb0ac71cb5e8b7d92953c2abcc63a0eb90499653b5bc";

/// Le dossier de three.js.
fn dossier_three() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs").join("bibliotheques").join("three")
}

/// Les octets d'un fichier de `docs/bibliotheques/three/`, qui doit exister et ne pas être vide.
fn octets(nom: &str) -> Vec<u8> {
    let chemin = dossier_three().join(nom);
    let contenu = std::fs::read(&chemin).unwrap_or_else(|e| panic!("{nom} : lecture impossible ({e})"));
    assert!(!contenu.is_empty(), "{nom} est vide");
    contenu
}

/// Le texte d'un fichier de `docs/bibliotheques/three/` (UTF-8 exigé).
fn texte(nom: &str) -> String {
    String::from_utf8(octets(nom)).unwrap_or_else(|e| panic!("{nom} n'est pas de l'UTF-8 ({e})"))
}

#[test]
fn les_fichiers_de_three_ont_leur_sha256() {
    for (nom, attendu) in
        [("three.module.min.js", SHA_MODULE), ("three.core.min.js", SHA_CORE), ("LICENSE", SHA_LICENCE)]
    {
        let obtenu = sha256_hex(&octets(nom));
        assert_eq!(obtenu, attendu, "{nom} : le SHA-256 n'est pas celui de three.js 0.185.1");
    }
}

#[test]
fn le_dossier_de_three_a_exactement_ses_quatre_fichiers() {
    let dossier = dossier_three();
    let mut trouves = BTreeSet::new();
    let entrees = std::fs::read_dir(&dossier).unwrap_or_else(|e| panic!("{} : {e}", dossier.display()));
    for entree in entrees {
        let entree = entree.unwrap_or_else(|e| panic!("{} : {e}", dossier.display()));
        let nom = entree.file_name().to_string_lossy().into_owned();
        let genre = entree.file_type().unwrap_or_else(|e| panic!("{nom} : {e}"));
        assert!(genre.is_file(), "{nom} : docs/bibliotheques/three/ ne contient que des fichiers, pas de sous-dossier");
        trouves.insert(nom);
    }
    let attendus: BTreeSet<String> =
        ["three.module.min.js", "three.core.min.js", "LICENSE", "LISEZ-MOI.md"].into_iter().map(String::from).collect();
    assert_eq!(trouves, attendus, "docs/bibliotheques/three/ doit contenir exactement ces quatre fichiers");
}

#[test]
fn la_notice_nomme_la_version_la_licence_et_les_empreintes() {
    let notice = texte("LISEZ-MOI.md");
    for (voulu, nom) in [
        ("0.185.1", "la version"),
        ("MIT", "la licence"),
        (SHA_MODULE, "le SHA-256 de three.module.min.js"),
        (SHA_CORE, "le SHA-256 de three.core.min.js"),
        (SHA_LICENCE, "le SHA-256 de LICENSE"),
    ] {
        assert!(notice.contains(voulu), "LISEZ-MOI.md ne nomme pas {nom} « {voulu} »");
    }
}

#[test]
fn la_revision_et_la_licence_sont_celles_attendues() {
    let noyau = texte("three.core.min.js");
    assert!(noyau.contains("const t=\"185\""), "three.core.min.js ne porte pas la révision « const t=\"185\" »");
    let licence = texte("LICENSE");
    assert!(licence.starts_with("The MIT License"), "LICENSE ne commence pas par « The MIT License »");
}

#[test]
fn le_module_n_importe_que_le_noyau() {
    let module = texte("three.module.min.js");
    let attendu = "./three.core.min.js";
    let mut importations = 0;
    for motif in ["from\"", "from \""] {
        for (i, _) in module.match_indices(motif) {
            let suite = &module[i + motif.len()..];
            assert!(
                suite.starts_with(attendu) && suite[attendu.len()..].starts_with('"'),
                "three.module.min.js importe autre chose que « {attendu} » : {}",
                suite.chars().take(40).collect::<String>()
            );
            importations += 1;
        }
    }
    assert!(importations >= 1, "three.module.min.js n'importe pas « {attendu} » : aucun `from\"` trouvé");
    for interdit in ["import(", "http://unpkg", "cdn."] {
        assert!(
            !module.contains(interdit),
            "three.module.min.js contient « {interdit} » : rien ne se charge d'ailleurs"
        );
    }
}

/// Somme des octets de tous les fichiers de `dossier`, sous-dossiers compris.
fn poids(dossier: &Path) -> u64 {
    let mut total = 0;
    let entrees = std::fs::read_dir(dossier).unwrap_or_else(|e| panic!("{} : {e}", dossier.display()));
    for entree in entrees {
        let entree = entree.unwrap_or_else(|e| panic!("{} : {e}", dossier.display()));
        let chemin = entree.path();
        if chemin.is_dir() {
            total += poids(&chemin);
        } else {
            total += entree.metadata().unwrap_or_else(|e| panic!("{} : {e}", chemin.display())).len();
        }
    }
    total
}

#[test]
fn les_bibliotheques_pesent_un_mo_au_plus() {
    let dossier = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs").join("bibliotheques");
    let total = poids(&dossier);
    assert!(total >= 1, "docs/bibliotheques/ est vide");
    assert!(total <= 1024 * 1024, "docs/bibliotheques/ pèse {total} octets, plus de 1 Mo (1048576)");
}
