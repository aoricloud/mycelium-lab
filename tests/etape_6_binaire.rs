//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie binaire : `mycelium lancer` sur un scénario avec la
//! directive `chimiotropisme`.
//!
//! Ils fixent : un scénario avec `chimiotropisme 2.5` fait écrire les trois mêmes fichiers qu'avant (`empreinte.txt`,
//! `mesures.csv`, `apercu.png`, rien d'autre), et son `empreinte.txt` et son `mesures.csv` sont ceux de la bibliothèque
//! (`lire_scenario_tropismes` + `nouvelle_avec_tropismes`) ; le même scénario avec `chimiotropisme 0`, `chimiotropisme 0.0`
//! ou sans la ligne donne les mêmes octets pour les trois fichiers (et ceux d'avant T0, relevés sur le binaire d'avant, pour
//! deux scénarios sans milieu) ; la force change l'état (empreinte et mesures différentes de celles de la force nulle,
//! quand il y a une tache) ; deux lancements donnent les mêmes octets ; une force invalide (négative, hors borne, `nan`,
//! `inf`) ou une ligne mal formée ou en double fait échouer le binaire proprement, sans panique (code 101).
//!
//! Ils laissent ouverts : le texte des messages d'erreur ; le contenu exact de `apercu.png` pour un scénario avec force
//! non nulle (seuls les trois fichiers existent et le PNG a sa signature) ; que `chimiotropisme` compte ou non parmi
//! les directives qui rendent un scénario « couplé » : le juge l'a établi non observable (un texte qui porte
//! `chimiotropisme` sans autre directive couplée échoue par l'un ou l'autre chemin, code de sortie 1, aucun fichier
//! écrit ; seul le message d'erreur diffère), c'est donc libre.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mycelium_lab::empreinte::sha256_hex;
use mycelium_lab::releves::en_csv;
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use mycelium_lab::tropisme::Tropismes;

/// Compteur d'instances : deux tests lancés en parallèle ne partagent jamais leur dossier temporaire.
static COMPTEUR: AtomicU64 = AtomicU64::new(0);

/// Un dossier temporaire propre au test, nettoyé à la chute.
struct Temp(PathBuf);

impl Temp {
    fn nouveau(nom: &str) -> Temp {
        let numero = COMPTEUR.fetch_add(1, Ordering::Relaxed);
        let dossier = std::env::temp_dir().join(format!("mycelium_etape6_{nom}_{}_{numero}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("création du dossier temporaire");
        Temp(dossier)
    }

    fn sortie(&self, nom: &str) -> PathBuf {
        self.0.join(nom)
    }

    /// Écrit un scénario dans le dossier temporaire et rend son chemin.
    fn scenario(&self, nom: &str, texte: &str) -> PathBuf {
        let chemin = self.0.join(nom);
        std::fs::write(&chemin, texte).expect("écriture du scénario");
        chemin
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

/// Lance `mycelium lancer <scenario> --graine <graine> --sortie <sortie>` et rend son code de sortie.
fn lancer(scenario: &Path, graine: u64, sortie: &Path) -> Option<i32> {
    Command::new(env!("CARGO_BIN_EXE_mycelium"))
        .arg("lancer")
        .arg(scenario)
        .arg("--graine")
        .arg(graine.to_string())
        .arg("--sortie")
        .arg(sortie)
        .status()
        .expect("lancement du binaire")
        .code()
}

fn lancer_et_reussir(scenario: &Path, graine: u64, sortie: &Path) {
    assert_eq!(lancer(scenario, graine, sortie), Some(0), "le binaire a échoué sur {}", scenario.display());
}

fn lire(sortie: &Path, nom: &str) -> Vec<u8> {
    std::fs::read(sortie.join(nom)).unwrap_or_else(|e| panic!("lecture de {nom} : {e}"))
}

/// Des tropismes de force `f`, sans littéral complet : T2 ajoutera un champ à `Tropismes`, et `..Tropismes::default()` garde
/// les tests valables. Tant qu'il n'y a qu'un champ, clippy trouve la mise à jour inutile (`needless_update`) : l'`allow`
/// tombera avec T2.
#[allow(clippy::needless_update)]
fn force(f: f64) -> Tropismes {
    Tropismes { chimiotropisme: f, ..Tropismes::default() }
}

/// Les trois fichiers que le binaire écrit.
const FICHIERS: [&str; 3] = ["empreinte.txt", "mesures.csv", "apercu.png"];

/// Un scénario avec une grosse tache riche à l'est de la spore (donc un gradient), sans directive de tropisme : en 40 pas,
/// la colonie en sent le gradient. Les variantes ajoutent la ligne `chimiotropisme`.
const SCENARIO_AVEC_TACHE: &str = "boite 40 30\nspore 8 15\nsubstrat 0.5 0.2\ncroissance 0.2 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 40\nsource 25 15 6 4.0\n";

/// Le même sans milieu (le gradient n'apparaît qu'avec la consommation).
const SCENARIO_SANS_MILIEU: &str = "boite 60 50\nspore 10 25\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 15\n";

/// 10. Un scénario avec `chimiotropisme 2.5` : le binaire réussit, écrit exactement `empreinte.txt`, `mesures.csv` et
///     `apercu.png`, et les deux premiers sont ceux de la bibliothèque (`lire_scenario_tropismes`,
///     `nouvelle_avec_tropismes`).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_binaire_ecrit_ses_fichiers_et_l_empreinte_de_la_bibliotheque() {
    let texte = format!("{SCENARIO_AVEC_TACHE}chimiotropisme 2.5\n");
    let temp = Temp::nouveau("fichiers");
    let scenario = temp.scenario("avec_tropisme.txt", &texte);
    let sortie = temp.sortie("sortie");
    lancer_et_reussir(&scenario, 5, &sortie);

    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(&texte).expect("le scénario se lit");
    assert_eq!(duree, 40);
    assert_eq!(tropismes, force(2.5));
    let mut simulation =
        Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, 5).expect("le scénario est valide");
    simulation.avancer_n(duree);

    let empreinte = String::from_utf8(lire(&sortie, "empreinte.txt")).expect("empreinte.txt est de l'UTF-8");
    assert_eq!(
        empreinte,
        format!("{}\n", simulation.empreinte()),
        "l'empreinte du binaire diffère de celle de la bibliothèque"
    );
    let mesures = String::from_utf8(lire(&sortie, "mesures.csv")).expect("mesures.csv est de l'UTF-8");
    assert_eq!(mesures, en_csv(simulation.releves()), "le CSV du binaire diffère de celui de la bibliothèque");
    let apercu = lire(&sortie, "apercu.png");
    assert!(
        apercu.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
        "apercu.png n'a pas la signature PNG"
    );

    let mut noms: Vec<String> = std::fs::read_dir(&sortie)
        .expect("lecture du dossier de sortie")
        .map(|entree| entree.expect("entrée du dossier").file_name().to_string_lossy().into_owned())
        .collect();
    noms.sort();
    assert_eq!(noms, ["apercu.png", "empreinte.txt", "mesures.csv"], "fichiers écrits par le binaire");
}

/// 10 (suite). Avec `chimiotropisme 0`, `chimiotropisme 0.0`, `chimiotropisme -0` (en fin, en tête ou au milieu du texte) ou sans la ligne, le
///     binaire écrit les mêmes octets, fichier par fichier, pour un scénario avec tache et pour un scénario sans milieu ;
///     et la force 2,5 change `empreinte.txt` et `mesures.csv` quand il y a une tache (la ligne agit).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_force_nulle_ou_absente_donne_les_memes_octets_et_la_force_non_nulle_les_change() {
    let temp = Temp::nouveau("force_nulle");
    for (nom_du_cas, base) in [("tache", SCENARIO_AVEC_TACHE), ("sans milieu", SCENARIO_SANS_MILIEU)] {
        let sans_ligne = temp.scenario("sans.txt", base);
        let en_fin_0 = temp.scenario("fin_0.txt", &format!("{base}chimiotropisme 0\n"));
        let en_fin_0_0 = temp.scenario("fin_0_0.txt", &format!("{base}chimiotropisme 0.0\n"));
        let en_fin_moins_0 = temp.scenario(
            "fin_moins_0.txt",
            &format!(
                "{base}chimiotropisme -0
"
            ),
        );
        let en_tete = temp.scenario("tete.txt", &format!("chimiotropisme 0\n{base}"));
        let au_milieu = temp.scenario(
            "milieu.txt",
            &format!(
                "{}chimiotropisme 0\n{}",
                &base[..base.find("croissance").unwrap()],
                &base[base.find("croissance").unwrap()..]
            ),
        );
        let reference = temp.sortie(&format!("ref_{nom_du_cas}"));
        lancer_et_reussir(&sans_ligne, 5, &reference);
        for (k, scenario) in [en_fin_0, en_fin_0_0, en_fin_moins_0, en_tete, au_milieu].into_iter().enumerate() {
            let sortie = temp.sortie(&format!("var_{nom_du_cas}_{k}"));
            lancer_et_reussir(&scenario, 5, &sortie);
            for fichier in FICHIERS {
                assert_eq!(
                    lire(&reference, fichier),
                    lire(&sortie, fichier),
                    "{nom_du_cas}, variante {k} : {fichier} diffère de celui du scénario sans la ligne"
                );
            }
        }
    }
    // La force non nulle change l'état quand il y a une tache.
    let sans_ligne = temp.scenario("tache_sans.txt", SCENARIO_AVEC_TACHE);
    let avec_force = temp.scenario("tache_avec.txt", &format!("{SCENARIO_AVEC_TACHE}chimiotropisme 2.5\n"));
    let (a, b) = (temp.sortie("a"), temp.sortie("b"));
    lancer_et_reussir(&sans_ligne, 5, &a);
    lancer_et_reussir(&avec_force, 5, &b);
    assert_ne!(lire(&a, "empreinte.txt"), lire(&b, "empreinte.txt"), "la force n'a pas changé l'empreinte");
    assert_ne!(lire(&a, "mesures.csv"), lire(&b, "mesures.csv"), "la force n'a pas changé les mesures");
}

/// 1 et 10 (suite). À force nulle, le binaire écrit les mêmes octets qu'avant T0, fichier par fichier : `empreinte.txt` en
///     clair, `mesures.csv` et `apercu.png` par leur empreinte SHA-256, relevées sur le binaire d'avant (les mêmes valeurs
///     que `etape_5_binaire.rs`), avec `chimiotropisme 0` ajouté au scénario.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn avec_chimiotropisme_0_les_trois_fichiers_sont_ceux_d_avant_t0() {
    let petit = "boite 41 29\nspore 20 14\nsubstrat 0.5 0.15\ncroissance 1.0 6.0 40 15\ncouplage 0.2 0.01\nanastomose 0.5 5\nduree 40\n";
    let temp = Temp::nouveau("avant_t0");
    let petit = temp.scenario("petit.txt", &format!("{petit}chimiotropisme 0\n"));
    let couplage = temp.scenario(
        "couplage.txt",
        &format!(
            "{}chimiotropisme 0\n",
            std::fs::read_to_string(scenario_couplage()).expect("lecture de couplage.txt")
        ),
    );
    let cas: [(&str, PathBuf, u64, &str, &str, &str); 2] = [
        (
            "couplage.txt, graine 1",
            couplage,
            1,
            "2223a1f3bcbc8627866aba084052cac06e93b05842bacc0ba94778bc23ed1083",
            "37f100bdb0ef19d6123f169df107636a297141231a6705791fbd32a6dcf3233d",
            "938724b78c53cae38889f7b76ee4c852032d695028b23c4c0acd725675b01777",
        ),
        (
            "petite boîte, graine 7",
            petit,
            7,
            "aa9a5a811b586442a19c50d5e00855ac7cdda94b8e92c3723b77f6ef5ac008ba",
            "8b1e9616afce74e511d1b4380907d83011cb6185dace5b5c2d3b29623f276095",
            "0995672d43b9d920bfe09492fc57a20ae306d794110e2bc0beb45daf57e4c80e",
        ),
    ];
    for (nom, scenario, graine, empreinte, mesures, apercu) in cas {
        let sortie = temp.sortie(&format!("sortie_{graine}"));
        lancer_et_reussir(&scenario, graine, &sortie);
        assert_eq!(
            String::from_utf8(lire(&sortie, "empreinte.txt")).unwrap(),
            format!("{empreinte}\n"),
            "{nom} : empreinte.txt diffère de celui d'avant T0"
        );
        assert_eq!(
            sha256_hex(&lire(&sortie, "mesures.csv")),
            mesures,
            "{nom} : mesures.csv diffère de celui d'avant T0"
        );
        assert_eq!(sha256_hex(&lire(&sortie, "apercu.png")), apercu, "{nom} : apercu.png diffère de celui d'avant T0");
    }
}

/// 10 (suite). Deux lancements du même scénario avec force, même graine, écrivent les mêmes trois fichiers à l'octet ; une
///     autre graine donne une autre empreinte ; deux forces donnent deux empreintes.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_binaire_est_reproductible_avec_une_force() {
    let temp = Temp::nouveau("reproductible");
    let force_2_5 = temp.scenario("f25.txt", &format!("{SCENARIO_AVEC_TACHE}chimiotropisme 2.5\n"));
    let force_4 = temp.scenario("f4.txt", &format!("{SCENARIO_AVEC_TACHE}chimiotropisme 4\n"));
    let (a, b, c, d) = (temp.sortie("a"), temp.sortie("b"), temp.sortie("c"), temp.sortie("d"));
    lancer_et_reussir(&force_2_5, 5, &a);
    lancer_et_reussir(&force_2_5, 5, &b);
    lancer_et_reussir(&force_2_5, 6, &c);
    lancer_et_reussir(&force_4, 5, &d);
    for fichier in FICHIERS {
        assert_eq!(lire(&a, fichier), lire(&b, fichier), "{fichier} diffère pour la même graine et la même force");
    }
    assert_ne!(lire(&a, "empreinte.txt"), lire(&c, "empreinte.txt"), "deux graines, deux empreintes");
    assert_ne!(lire(&a, "empreinte.txt"), lire(&d, "empreinte.txt"), "deux forces, deux empreintes");
}

/// 10 (suite). Une force invalide ou une ligne `chimiotropisme` fautive fait échouer le binaire proprement : un code de
///     sortie non nul qui n'est pas celui d'une panique (101) ; la force invalide (négative, hors borne, `nan`, `inf`) est
///     refusée à la lecture ou à la construction, le résultat est le même ; un doublon, un mot en trop ou en moins aussi.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn une_force_invalide_fait_echouer_le_binaire_sans_paniquer() {
    let temp = Temp::nouveau("invalide");
    // Garde : la même base avec une force valide réussit (le test n'échoue pas pour une autre raison).
    let valide = temp.scenario("valide.txt", &format!("{SCENARIO_SANS_MILIEU}chimiotropisme 2.5\n"));
    lancer_et_reussir(&valide, 1, &temp.sortie("valide"));
    for (k, supplement) in [
        "chimiotropisme -1\n",
        "chimiotropisme -0.001\n",
        "chimiotropisme 361\n",
        "chimiotropisme 360\n",
        "chimiotropisme 46\n",
        "chimiotropisme 45.5\n",
        "chimiotropisme 1000000\n",
        "chimiotropisme nan\n",
        "chimiotropisme NaN\n",
        "chimiotropisme inf\n",
        "chimiotropisme -inf\n",
        "chimiotropisme abc\n",
        "chimiotropisme\n",
        "chimiotropisme 1 2\n",
        "chimiotropisme 1\nchimiotropisme 2\n",
        "chimiotropisme 0\nchimiotropisme 0\n",
    ]
    .into_iter()
    .enumerate()
    {
        let scenario = temp.scenario(&format!("invalide_{k}.txt"), &format!("{SCENARIO_SANS_MILIEU}{supplement}"));
        let code = lancer(&scenario, 1, &temp.sortie(&format!("sortie_{k}")));
        assert!(
            code.is_some() && code != Some(0),
            "le scénario avec {supplement:?} aurait dû faire échouer le binaire (code {code:?})"
        );
        assert_ne!(code, Some(101), "le binaire a paniqué sur {supplement:?} au lieu d'échouer proprement");
    }
}
