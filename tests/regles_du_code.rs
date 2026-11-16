//! Les règles du code, en un seul endroit : tout `src/`, `examples/` et `tests/`, sous-dossiers compris.
//!
//! - Aucune fonction de `f64` de la liste de `tests/etape_17_exemple.rs` (trigonométrie, exponentielles,
//!   logarithmes, puissances), appelée en méthode (`.nom(`) ou nommée par un chemin (`::nom` qui n'est pas le début
//!   d'un nom plus long, comme `::logarithme`).
//! - Ni `HashMap` ni `HashSet`, nulle part.
//! - Ni fil, ni `Instant`, ni `SystemTime` dans `src/` et `examples/` : les tests emploient des fils pour leurs
//!   délais, la simulation jamais.
//!
//! Les commentaires (`//` une fois les blancs de tête ôtés) sont ignorés : ils nomment ces fonctions pour les
//! interdire. Les noms sont assemblés par `concat!` pour que ce fichier ne se dénonce pas lui-même. Il n'y a aucune
//! exception de fichier : la PR qui ajoutera le fichier de maths copié ajoutera son exception, à part.

use std::path::{Path, PathBuf};

/// Remplit `sortie` des chemins de tous les `.rs` de `dossier`, sous-dossiers compris.
fn fichiers_rs(dossier: &Path, sortie: &mut Vec<PathBuf>) {
    let entrees = std::fs::read_dir(dossier).unwrap_or_else(|e| panic!("{} : {e}", dossier.display()));
    for entree in entrees {
        let chemin = entree.unwrap_or_else(|e| panic!("{} : {e}", dossier.display())).path();
        if chemin.is_dir() {
            fichiers_rs(&chemin, sortie);
        } else if chemin.extension().is_some_and(|extension| extension == "rs") {
            sortie.push(chemin);
        }
    }
}

#[test]
fn le_code_respecte_les_regles_des_sources() {
    // Les noms interdits sont assemblés : ce fichier est lu comme les autres.
    let fonctions = [
        concat!("si", "n"),
        concat!("co", "s"),
        concat!("ta", "n"),
        concat!("asi", "n"),
        concat!("aco", "s"),
        concat!("ata", "n"),
        concat!("atan", "2"),
        concat!("sin", "h"),
        concat!("cos", "h"),
        concat!("tan", "h"),
        concat!("ex", "p"),
        concat!("exp", "2"),
        concat!("exp_", "m1"),
        concat!("l", "n"),
        concat!("ln_", "1p"),
        concat!("lo", "g"),
        concat!("log", "2"),
        concat!("log", "10"),
        concat!("pow", "f"),
        concat!("pow", "i"),
        concat!("hypo", "t"),
        concat!("mul_", "add"),
        concat!("cb", "rt"),
    ];

    let racine = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut fichiers = Vec::new();
    for dossier in ["src", "examples", "tests"] {
        fichiers_rs(&racine.join(dossier), &mut fichiers);
    }
    // L'ordre ne dépend pas du disque : les chemins triés.
    fichiers.sort();

    let de_src =
        fichiers.iter().filter(|chemin| chemin.strip_prefix(racine).is_ok_and(|rel| rel.starts_with("src"))).count();
    assert!(de_src >= 20, "seulement {de_src} fichiers de src/ lus : le parcours est vide ou incomplet");

    for chemin in &fichiers {
        let rel = chemin.strip_prefix(racine).expect("sous la racine du dépôt");
        let texte = std::fs::read_to_string(chemin).unwrap_or_else(|e| panic!("{} : {e}", rel.display()));
        // Un fichier vidé passerait sous les vérifications sans qu'elles le voient : il est lu, donc il a des lignes.
        assert!(texte.lines().next().is_some(), "{} est lu", rel.display());
        let dans_src_ou_examples = rel.starts_with("src") || rel.starts_with("examples");
        for (numero, ligne) in texte.lines().enumerate() {
            let numero = numero + 1;
            if ligne.trim_start().starts_with("//") {
                continue;
            }
            for nom in fonctions {
                let en_methode = ligne.contains(&format!(".{nom}("));
                let motif = format!("::{nom}");
                let par_chemin = ligne.match_indices(&motif).any(|(i, _)| {
                    let suite = &ligne[i + motif.len()..];
                    !suite.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                });
                assert!(!en_methode && !par_chemin, "{}:{numero} : fonction de f64 interdite « {nom} »", rel.display());
            }
            for nom in [concat!("Hash", "Map"), concat!("Hash", "Set")] {
                assert!(!ligne.contains(nom), "{}:{numero} : table de hachage « {nom} »", rel.display());
            }
            if dans_src_ou_examples {
                for nom in [concat!("thr", "ead"), concat!("Inst", "ant"), concat!("System", "Time")] {
                    assert!(
                        !ligne.contains(nom),
                        "{}:{numero} : « {nom} » n'a pas sa place dans src/ ni dans examples/",
                        rel.display()
                    );
                }
            }
        }
    }
}
