//! Une boîte trop grande est refusée avant toute allocation et toute écriture.
//!
//! Les deux chemins du binaire sont couverts : le scénario de l'étape 0 (une ligne `boite`) et le scénario couplé
//! (une spore et les directives d'une simulation, celles de `scenarios/couplage.txt`). Le refus est celui de tout
//! scénario invalide : `mycelium : …` sur stderr, code 1, aucun dossier de sortie. Une boîte de 1000 × 1000 reste
//! acceptée et écrit son `apercu.png`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Compteur d'instances : deux tests lancés en parallèle ne partagent jamais leur dossier temporaire.
static COMPTEUR: AtomicU64 = AtomicU64::new(0);

/// Un dossier temporaire propre au test, nettoyé à la chute.
struct Temp(PathBuf);

impl Temp {
    fn nouveau(nom: &str) -> Temp {
        let numero = COMPTEUR.fetch_add(1, Ordering::Relaxed);
        let dossier = std::env::temp_dir().join(format!("mycelium_boite_{nom}_{}_{numero}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("création du dossier temporaire");
        Temp(dossier)
    }

    /// Écrit un scénario dans le dossier temporaire et rend son chemin.
    fn scenario(&self, nom: &str, texte: &str) -> PathBuf {
        let chemin = self.0.join(nom);
        std::fs::write(&chemin, texte).expect("écriture du scénario");
        chemin
    }

    /// Un dossier de sortie neuf, encore inexistant.
    fn sortie(&self, nom: &str) -> PathBuf {
        let chemin = self.0.join(nom);
        let _ = std::fs::remove_dir_all(&chemin);
        chemin
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Ce que le binaire a dit : son code de sortie (`None` s'il a été tué par un signal) et son stderr.
struct Lancement {
    code: Option<i32>,
    stderr: String,
}

/// Lance `mycelium lancer <scenario> --graine 1 --sortie <sortie>`.
fn lancer(scenario: &Path, sortie: &Path) -> Lancement {
    let resultat = Command::new(env!("CARGO_BIN_EXE_mycelium"))
        .arg("lancer")
        .arg(scenario)
        .args(["--graine", "1", "--sortie"])
        .arg(sortie)
        .output()
        .expect("lancement du binaire");
    Lancement { code: resultat.status.code(), stderr: String::from_utf8_lossy(&resultat.stderr).into_owned() }
}

/// Exige que le scénario soit refusé proprement, sans rien créer.
fn exiger_refus(temp: &Temp, nom: &str, texte: &str) {
    let scenario = temp.scenario(&format!("{nom}.txt"), texte);
    let sortie = temp.sortie(&format!("sortie_{nom}"));
    let resultat = lancer(&scenario, &sortie);
    assert_eq!(
        resultat.code,
        Some(1),
        "{nom} : le binaire devait sortir avec le code 1, pas {:?} (stderr : {:?})",
        resultat.code,
        resultat.stderr
    );
    assert!(
        resultat.stderr.starts_with("mycelium : "),
        "{nom} : stderr ne commence pas par « mycelium : » : {:?}",
        resultat.stderr
    );
    assert!(
        resultat.stderr.contains("trop grande"),
        "{nom} : le message ne dit pas que la boîte est trop grande : {:?}",
        resultat.stderr
    );
    assert!(!sortie.exists(), "{nom} : le dossier de sortie a été créé malgré le refus : {}", sortie.display());
}

/// Le scénario couplé du dépôt, avec sa boîte remplacée par une boîte trop grande : les mêmes lignes `spore`,
/// `substrat`, `croissance`, `couplage`, `anastomose` et `duree`.
fn scenario_couple_trop_grand() -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join("couplage.txt");
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("lecture de {} : {e}", chemin.display()));
    let reste = texte.lines().skip_while(|ligne| !ligne.starts_with("spore ")).collect::<Vec<_>>().join("\n");
    assert!(!reste.is_empty(), "`scenarios/couplage.txt` ne porte pas de ligne `spore`");
    format!("boite 100000 100000\n{reste}\n")
}

/// 1. Une boîte seule trop grande (le chemin de l'étape 0) est refusée, code 1 et rien d'écrit.
#[test]
fn une_boite_seule_trop_grande_est_refusee() {
    let temp = Temp::nouveau("seule");
    exiger_refus(&temp, "seule", "boite 100000 100000\n");
}

/// 2. La même boîte dans une simulation complète (le chemin couplé) est refusée pareillement.
#[test]
fn une_simulation_trop_grande_est_refusee() {
    let temp = Temp::nouveau("couplee");
    exiger_refus(&temp, "couplee", &scenario_couple_trop_grand());
}

/// 3. Une boîte de 1000 × 1000 reste acceptée et écrit son `apercu.png`.
#[test]
fn une_boite_de_1000_sur_1000_reste_acceptee() {
    let temp = Temp::nouveau("acceptee");
    let scenario = temp.scenario("mille.txt", "boite 1000 1000\n");
    let sortie = temp.sortie("sortie_mille");
    let resultat = lancer(&scenario, &sortie);
    assert_eq!(resultat.code, Some(0), "le binaire a refusé 1000 × 1000 (stderr : {:?})", resultat.stderr);
    assert!(sortie.join("empreinte.txt").is_file(), "empreinte.txt manquant pour 1000 × 1000");
    assert!(sortie.join("apercu.png").is_file(), "apercu.png manquant pour 1000 × 1000");
}
