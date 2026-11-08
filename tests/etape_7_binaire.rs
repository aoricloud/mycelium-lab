//! Tests d'acceptation de l'étape 7 (T2, le thigmotropisme), partie binaire : `mycelium lancer` sur un scénario avec la
//! directive `thigmotropisme`.
//!
//! Ils fixent : `scenarios/mur.txt` (qui porte `thigmotropisme 35`) fait écrire les trois mêmes fichiers qu'avant
//! (`empreinte.txt`, `mesures.csv`, `apercu.png`, rien d'autre), et son `empreinte.txt` et son `mesures.csv` sont ceux de
//! la bibliothèque (`lire_scenario_tropismes` + `nouvelle_avec_tropismes`) ; le même scénario avec `thigmotropisme 0`,
//! `thigmotropisme -0` ou sans la ligne donne les mêmes octets pour les trois fichiers ; sans contact avec un obstacle et à
//! angle nul, le binaire écrit les mêmes octets qu'avant T2, fichier par fichier (`empreinte.txt` en clair, `mesures.csv` et
//! `apercu.png` par leur empreinte SHA-256, relevées sur le binaire d'avant T2, avec `thigmotropisme 0` ajouté) ; l'angle
//! 35 change l'état (empreinte et mesures différentes de celles de l'angle nul, sur le mur) ; deux lancements donnent les
//! mêmes octets, une autre graine ou un autre angle une autre empreinte ; un angle invalide (négatif, supérieur à 90, `nan`,
//! `inf`) ou une ligne mal formée ou en double fait échouer le binaire proprement, sans panique (code 101) ; l'aperçu
//! d'un scénario avec obstacles est un PNG de la taille attendue (4 pixels par case).
//!
//! Ils laissent ouverts : le texte des messages d'erreur ; le contenu exact de `apercu.png` pour un scénario où l'angle est
//! non nul (seuls les trois fichiers existent et le PNG a sa signature et sa taille).
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_7/outils.rs"]
mod outils;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mycelium_lab::empreinte::sha256_hex;
use mycelium_lab::releves::en_csv;
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

/// Compteur d'instances : deux tests lancés en parallèle ne partagent jamais leur dossier temporaire.
static COMPTEUR: AtomicU64 = AtomicU64::new(0);

/// Un dossier temporaire propre au test, nettoyé à la chute.
struct Temp(PathBuf);

impl Temp {
    fn nouveau(nom: &str) -> Temp {
        let numero = COMPTEUR.fetch_add(1, Ordering::Relaxed);
        let dossier = std::env::temp_dir().join(format!("mycelium_etape7_{nom}_{}_{numero}", std::process::id()));
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

/// Le texte du scénario du dépôt `scenarios/mur.txt`.
fn texte_mur() -> &'static str {
    include_str!("../scenarios/mur.txt")
}

/// Le même, sans la ligne `thigmotropisme 35` (angle nul).
fn texte_mur_sans_angle() -> String {
    texte_mur().replace("thigmotropisme 35\n", "")
}

/// Lance `mycelium lancer <scenario> --graine <graine> --sortie <sortie>` et rend son code de sortie.
fn lancer(scenario: &Path, graine: u64, sortie: &Path) -> Option<i32> {
    let mut enfant = Command::new(env!("CARGO_BIN_EXE_mycelium"))
        .arg("lancer")
        .arg(scenario)
        .arg("--graine")
        .arg(graine.to_string())
        .arg("--sortie")
        .arg(sortie)
        .spawn()
        .expect("lancement du binaire");
    // Un binaire qui ne se termine pas en 120 s est tué : le test échoue, il ne pend pas.
    for _ in 0..1200 {
        if let Some(statut) = enfant.try_wait().expect("attente du binaire") {
            return statut.code();
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = enfant.kill();
    panic!("le binaire ne se termine pas en 120 s sur {}", scenario.display());
}

fn lancer_et_reussir(scenario: &Path, graine: u64, sortie: &Path) {
    assert_eq!(lancer(scenario, graine, sortie), Some(0), "le binaire a échoué sur {}", scenario.display());
}

fn lire(sortie: &Path, nom: &str) -> Vec<u8> {
    std::fs::read(sortie.join(nom)).unwrap_or_else(|e| panic!("lecture de {nom} : {e}"))
}

/// Les trois fichiers que le binaire écrit.
const FICHIERS: [&str; 3] = ["empreinte.txt", "mesures.csv", "apercu.png"];

/// Un scénario sans obstacle : la colonie ne peut rien toucher. (Le même que celui qui a servi à relever les empreintes
/// d'avant T2 : 60 × 50, spore en (10, 25), 40 pas.)
const SCENARIO_SANS_OBSTACLE: &str = "boite 60 50\nspore 10 25\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 40\n";

/// 10. `scenarios/mur.txt` : le binaire réussit, écrit exactement `empreinte.txt`, `mesures.csv` et `apercu.png`, et les deux
///     premiers sont ceux de la bibliothèque ; l'aperçu est un PNG de 400 × 400 pixels (100 × 100 cases à 4 pixels).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_binaire_ecrit_ses_fichiers_et_l_empreinte_de_la_bibliotheque() {
    outils::delai();
    let texte = texte_mur();
    let temp = Temp::nouveau("fichiers");
    let scenario = temp.scenario("mur.txt", texte);
    let sortie = temp.sortie("sortie");
    lancer_et_reussir(&scenario, 2, &sortie);

    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(texte).expect("le scénario se lit");
    assert_eq!(tropismes.thigmotropisme, 35.0);
    let mut simulation =
        Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, 2).expect("le scénario est valide");
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
    // IHDR : largeur et hauteur en grand-boutiste aux octets 16..24.
    assert_eq!(u32::from_be_bytes(apercu[16..20].try_into().unwrap()), 400, "largeur du PNG");
    assert_eq!(u32::from_be_bytes(apercu[20..24].try_into().unwrap()), 400, "hauteur du PNG");

    let mut noms: Vec<String> = std::fs::read_dir(&sortie)
        .expect("lecture du dossier de sortie")
        .map(|entree| entree.expect("entrée du dossier").file_name().to_string_lossy().into_owned())
        .collect();
    noms.sort();
    assert_eq!(noms, ["apercu.png", "empreinte.txt", "mesures.csv"], "fichiers écrits par le binaire");
}

/// 10 (suite). Avec `thigmotropisme 0`, `thigmotropisme 0.0`, `thigmotropisme -0` (en fin, en tête ou au milieu du texte) ou
/// sans la ligne, le binaire écrit les mêmes octets, fichier par fichier, sur le mur (qui est touché) comme sur un scénario
/// sans milieu ; et l'angle 35 change `empreinte.txt` et `mesures.csv` sur le mur (la ligne agit).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn l_angle_nul_ou_absent_donne_les_memes_octets_et_l_angle_non_nul_les_change() {
    outils::delai();
    let temp = Temp::nouveau("angle_nul");
    for (nom_du_cas, base) in [("mur", texte_mur_sans_angle()), ("sans obstacle", SCENARIO_SANS_OBSTACLE.to_string())] {
        let sans_ligne = temp.scenario("sans.txt", &base);
        let en_fin_0 = temp.scenario("fin_0.txt", &format!("{base}thigmotropisme 0\n"));
        let en_fin_0_0 = temp.scenario("fin_0_0.txt", &format!("{base}thigmotropisme 0.0\n"));
        let en_fin_moins_0 = temp.scenario("fin_moins_0.txt", &format!("{base}thigmotropisme -0\n"));
        let en_tete = temp.scenario("tete.txt", &format!("thigmotropisme 0\n{base}"));
        let coupe = base.find("croissance").expect("une ligne croissance");
        let au_milieu = temp.scenario("milieu.txt", &format!("{}thigmotropisme 0\n{}", &base[..coupe], &base[coupe..]));
        let reference = temp.sortie(&format!("ref_{nom_du_cas}"));
        lancer_et_reussir(&sans_ligne, 2, &reference);
        for (k, scenario) in [en_fin_0, en_fin_0_0, en_fin_moins_0, en_tete, au_milieu].into_iter().enumerate() {
            let sortie = temp.sortie(&format!("var_{nom_du_cas}_{k}"));
            lancer_et_reussir(&scenario, 2, &sortie);
            for fichier in FICHIERS {
                assert_eq!(
                    lire(&reference, fichier),
                    lire(&sortie, fichier),
                    "{nom_du_cas}, variante {k} : {fichier} diffère de celui du scénario sans la ligne"
                );
            }
        }
    }
    // L'angle 35 change l'état quand la colonie touche le mur (graine 2 : elle le touche).
    let sans_ligne = temp.scenario("mur_sans.txt", &texte_mur_sans_angle());
    let avec_angle = temp.scenario("mur_avec.txt", texte_mur());
    let (a, b) = (temp.sortie("a"), temp.sortie("b"));
    lancer_et_reussir(&sans_ligne, 2, &a);
    lancer_et_reussir(&avec_angle, 2, &b);
    assert_ne!(lire(&a, "empreinte.txt"), lire(&b, "empreinte.txt"), "l'angle n'a pas changé l'empreinte");
    assert_ne!(lire(&a, "mesures.csv"), lire(&b, "mesures.csv"), "l'angle n'a pas changé les mesures");
}

/// 1 et 10 (suite). Sans contact et à angle nul, le binaire écrit les mêmes octets qu'avant T2, fichier par fichier :
/// `empreinte.txt` en clair, `mesures.csv` et `apercu.png` par leur empreinte SHA-256, relevées sur le binaire d'avant T2 ;
/// avec `thigmotropisme 0` ajouté au scénario. Trois cas : `couplage.txt` graine 1 et une petite boîte graine 7 (les mêmes
/// valeurs que `etape_5_binaire.rs` et `etape_6_binaire.rs`), et le scénario sans obstacle ci-dessus graine 5.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn avec_thigmotropisme_0_les_trois_fichiers_sont_ceux_d_avant_t2() {
    outils::delai();
    let petit = "boite 41 29\nspore 20 14\nsubstrat 0.5 0.15\ncroissance 1.0 6.0 40 15\ncouplage 0.2 0.01\nanastomose 0.5 5\nduree 40\n";
    let temp = Temp::nouveau("avant_t2");
    let petit = temp.scenario("petit.txt", &format!("{petit}thigmotropisme 0\n"));
    let couplage =
        temp.scenario("couplage.txt", &format!("{}thigmotropisme 0\n", include_str!("../scenarios/couplage.txt")));
    let sans = temp.scenario("sans.txt", &format!("{SCENARIO_SANS_OBSTACLE}thigmotropisme 0\n"));
    let cas: [(&str, PathBuf, u64, &str, &str, &str); 3] = [
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
        (
            "sans obstacle, graine 5",
            sans,
            5,
            "377caf4c4b866f3874377a97efcbd43e7f0176e5b5821d8b790ebaf774149d20",
            "643e8714de46514714449cc9231b2868095b730c0a41ee05d7621aaf19499e58",
            "d80bf31bdcdafe42dcd4323e3cad3a64d2f1c6b62892830e7e4227844c1184f0",
        ),
    ];
    for (nom, scenario, graine, empreinte, mesures, apercu) in cas {
        let sortie = temp.sortie(&format!("sortie_{graine}"));
        lancer_et_reussir(&scenario, graine, &sortie);
        assert_eq!(
            String::from_utf8(lire(&sortie, "empreinte.txt")).unwrap(),
            format!("{empreinte}\n"),
            "{nom} : empreinte.txt diffère de celui d'avant T2"
        );
        assert_eq!(
            sha256_hex(&lire(&sortie, "mesures.csv")),
            mesures,
            "{nom} : mesures.csv diffère de celui d'avant T2"
        );
        assert_eq!(sha256_hex(&lire(&sortie, "apercu.png")), apercu, "{nom} : apercu.png diffère de celui d'avant T2");
    }
}

/// 10 (suite). Deux lancements du même scénario avec angle, même graine, écrivent les mêmes trois fichiers à l'octet ; une
/// autre graine donne une autre empreinte ; deux angles (35 et 60) donnent deux empreintes.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn le_binaire_est_reproductible_avec_un_angle() {
    outils::delai();
    let temp = Temp::nouveau("reproductible");
    let angle_35 = temp.scenario("a35.txt", texte_mur());
    let angle_60 = temp.scenario("a60.txt", &texte_mur().replace("thigmotropisme 35", "thigmotropisme 60"));
    let (a, b, c, d) = (temp.sortie("a"), temp.sortie("b"), temp.sortie("c"), temp.sortie("d"));
    lancer_et_reussir(&angle_35, 2, &a);
    lancer_et_reussir(&angle_35, 2, &b);
    lancer_et_reussir(&angle_35, 3, &c);
    lancer_et_reussir(&angle_60, 2, &d);
    for fichier in FICHIERS {
        assert_eq!(lire(&a, fichier), lire(&b, fichier), "{fichier} diffère pour la même graine et le même angle");
    }
    assert_ne!(lire(&a, "empreinte.txt"), lire(&c, "empreinte.txt"), "deux graines, deux empreintes");
    assert_ne!(lire(&a, "empreinte.txt"), lire(&d, "empreinte.txt"), "deux angles, deux empreintes");
}

/// 10 (suite). Un angle invalide ou une ligne `thigmotropisme` fautive fait échouer le binaire proprement : un code de sortie
/// non nul qui n'est pas celui d'une panique (101) ; l'angle invalide (négatif, supérieur à 90, `nan`, `inf`) est refusé à
/// la lecture ou à la construction, le résultat est le même ; un doublon, un mot en trop ou en moins aussi.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn un_angle_invalide_fait_echouer_le_binaire_sans_paniquer() {
    outils::delai();
    let temp = Temp::nouveau("invalide");
    // Garde : la même base avec un angle valide réussit (le test n'échoue pas pour une autre raison).
    let valide = temp.scenario("valide.txt", &format!("{SCENARIO_SANS_OBSTACLE}thigmotropisme 35\n"));
    lancer_et_reussir(&valide, 1, &temp.sortie("valide"));
    for (k, supplement) in [
        "thigmotropisme -1\n",
        "thigmotropisme -0.001\n",
        "thigmotropisme 91\n",
        "thigmotropisme 90.5\n",
        "thigmotropisme 360\n",
        "thigmotropisme 1000000\n",
        "thigmotropisme nan\n",
        "thigmotropisme NaN\n",
        "thigmotropisme inf\n",
        "thigmotropisme -inf\n",
        "thigmotropisme abc\n",
        "thigmotropisme\n",
        "thigmotropisme 1 2\n",
        "thigmotropisme 35\nthigmotropisme 35\n",
        "thigmotropisme 0\nthigmotropisme 0\n",
    ]
    .into_iter()
    .enumerate()
    {
        let scenario = temp.scenario(&format!("invalide_{k}.txt"), &format!("{SCENARIO_SANS_OBSTACLE}{supplement}"));
        let code = lancer(&scenario, 1, &temp.sortie(&format!("sortie_{k}")));
        assert!(
            code.is_some() && code != Some(0),
            "le scénario avec {supplement:?} aurait dû faire échouer le binaire (code {code:?})"
        );
        assert_ne!(code, Some(101), "le binaire a paniqué sur {supplement:?} au lieu d'échouer proprement");
    }
}
