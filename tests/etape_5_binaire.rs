//! Tests d'acceptation de l'étape 5 (T0, le terrain), partie binaire : `mycelium lancer` sur un scénario avec tache et
//! obstacle.
//!
//! Ils fixent : le binaire lit les directives `source` et `obstacle`, écrit ses trois fichiers, et son
//! `empreinte.txt` est celle de la bibliothèque ; l'aperçu `apercu.png` dessine les obstacles en gris ; un scénario
//! sans milieu donne les mêmes octets qu'avant T0 (les trois fichiers, relevés sur le binaire d'avant) ; un milieu
//! hors contrat fait échouer le binaire sans le faire paniquer.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mycelium_lab::empreinte::sha256_hex;
use mycelium_lab::releves::en_csv;
use mycelium_lab::simulation::{Simulation, lire_scenario_complet};

/// Compteur d'instances : deux tests lancés en parallèle ne partagent jamais leur dossier temporaire.
static COMPTEUR: AtomicU64 = AtomicU64::new(0);

/// Un dossier temporaire propre au test, nettoyé à la chute.
struct Temp(PathBuf);

impl Temp {
    fn nouveau(nom: &str) -> Temp {
        let numero = COMPTEUR.fetch_add(1, Ordering::Relaxed);
        let dossier = std::env::temp_dir().join(format!("mycelium_etape5_{nom}_{}_{numero}", std::process::id()));
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

/// Un scénario avec une tache et un obstacle loin de la spore : en 15 pas, la colonie ne les atteint pas.
const SCENARIO_AVEC_MILIEU: &str = "boite 60 50\nspore 10 25\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 15\nsource 50 40 3 4.0\nobstacle 40 10 43 30\n";

/// Le même scénario sans le milieu.
const SCENARIO_SANS_MILIEU: &str = "boite 60 50\nspore 10 25\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 15\n";

/// 8. Le binaire lit un scénario avec tache et obstacle, écrit ses trois fichiers, et `empreinte.txt`, `mesures.csv`
///    sont exactement ceux de la bibliothèque (`lire_scenario_complet`, `nouvelle_avec_milieu`).
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn le_binaire_ecrit_ses_fichiers_et_l_empreinte_de_la_bibliotheque() {
    let temp = Temp::nouveau("fichiers");
    let scenario = temp.scenario("avec_milieu.txt", SCENARIO_AVEC_MILIEU);
    let sortie = temp.sortie("sortie");
    lancer_et_reussir(&scenario, 5, &sortie);

    let (reglages, milieu, duree) = lire_scenario_complet(SCENARIO_AVEC_MILIEU).expect("le scénario se lit");
    assert_eq!(duree, 15);
    assert_eq!(milieu.sources.len(), 1);
    assert_eq!(milieu.obstacles.len(), 1);
    let mut simulation = Simulation::nouvelle_avec_milieu(reglages, milieu, 5).expect("le milieu est valide");
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

    // Rien d'autre n'est écrit : le dossier de sortie ne porte que les trois mêmes fichiers qu'avant T0.
    let mut noms: Vec<String> = std::fs::read_dir(&sortie)
        .expect("lecture du dossier de sortie")
        .map(|entree| entree.expect("entrée du dossier").file_name().to_string_lossy().into_owned())
        .collect();
    noms.sort();
    assert_eq!(noms, ["apercu.png", "empreinte.txt", "mesures.csv"], "fichiers écrits par le binaire");
}

/// 8 (suite). Le milieu compte : le même scénario sans les lignes `source` et `obstacle` donne une autre empreinte,
///    celle de la bibliothèque sans milieu ; et deux lancements du scénario avec milieu, même graine, écrivent les
///    mêmes trois fichiers à l'octet.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn le_milieu_change_l_empreinte_et_le_binaire_est_reproductible() {
    let temp = Temp::nouveau("milieu_compte");
    let avec = temp.scenario("avec.txt", SCENARIO_AVEC_MILIEU);
    let sans = temp.scenario("sans.txt", SCENARIO_SANS_MILIEU);
    let (a, b, c) = (temp.sortie("a"), temp.sortie("b"), temp.sortie("c"));
    lancer_et_reussir(&avec, 5, &a);
    lancer_et_reussir(&avec, 5, &b);
    lancer_et_reussir(&sans, 5, &c);
    for nom in ["empreinte.txt", "mesures.csv", "apercu.png"] {
        assert_eq!(lire(&a, nom), lire(&b, nom), "{nom} diffère pour la même graine et le même milieu");
    }
    assert_ne!(lire(&a, "empreinte.txt"), lire(&c, "empreinte.txt"), "le milieu n'a pas changé l'empreinte");

    let (reglages, milieu, duree) = lire_scenario_complet(SCENARIO_SANS_MILIEU).expect("le scénario se lit");
    assert!(milieu.sources.is_empty() && milieu.obstacles.is_empty());
    let mut simulation = Simulation::nouvelle_avec_milieu(reglages, milieu, 5).expect("réglages valides");
    simulation.avancer_n(duree);
    assert_eq!(
        String::from_utf8(lire(&c, "empreinte.txt")).unwrap(),
        format!("{}\n", simulation.empreinte()),
        "sans milieu, l'empreinte du binaire diffère de celle de la bibliothèque"
    );
}

/// 1 et 8 (suite). Sans milieu, le binaire écrit les mêmes octets qu'avant T0, fichier par fichier : `empreinte.txt`
///    en clair, `mesures.csv` et `apercu.png` par leur empreinte SHA-256, relevées sur le binaire d'avant. Deux
///    scénarios : celui du dépôt (graine 1, dont `empreinte.txt` est `references/couplage-graine-1.empreinte`) et
///    une petite boîte à ramifications serrées (graine 7).
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn sans_milieu_les_trois_fichiers_sont_ceux_d_avant_t0() {
    let petit = "boite 41 29\nspore 20 14\nsubstrat 0.5 0.15\ncroissance 1.0 6.0 40 15\ncouplage 0.2 0.01\nanastomose 0.5 5\nduree 40\n";
    let temp = Temp::nouveau("avant_t0");
    let petit = temp.scenario("petit.txt", petit);
    let cas: [(&str, PathBuf, u64, &str, &str, &str); 2] = [
        (
            "couplage.txt, graine 1",
            scenario_couplage(),
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

/// Une image PNG décodée : RGB 8 bits, ligne par ligne.
struct Image {
    largeur: usize,
    hauteur: usize,
    pixels: Vec<u8>,
}

impl Image {
    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.largeur + x) * 3;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]]
    }
}

/// Décode un PNG écrit par `image::ecrire_png` : RGB 8 bits, sans entrelacement, flux zlib en blocs stockés, filtre
/// 0 sur chaque ligne (c'est ce que l'écrivain du dépôt produit ; ses propres tests le fixent).
fn decoder_png(octets: &[u8]) -> Image {
    assert!(octets.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]), "signature PNG");
    let (mut largeur, mut hauteur) = (0usize, 0usize);
    let mut idat = Vec::new();
    let mut i = 8;
    while i + 12 <= octets.len() {
        let n = u32::from_be_bytes(octets[i..i + 4].try_into().unwrap()) as usize;
        let type_ = &octets[i + 4..i + 8];
        let donnees = &octets[i + 8..i + 8 + n];
        if type_ == b"IHDR" {
            largeur = u32::from_be_bytes(donnees[0..4].try_into().unwrap()) as usize;
            hauteur = u32::from_be_bytes(donnees[4..8].try_into().unwrap()) as usize;
            assert_eq!(&donnees[8..13], &[8, 2, 0, 0, 0], "RGB 8 bits sans entrelacement attendu");
        } else if type_ == b"IDAT" {
            idat.extend_from_slice(donnees);
        }
        i += 12 + n;
    }
    let mut brut = Vec::new();
    let mut j = 2;
    loop {
        let entete = idat[j];
        assert_eq!((entete >> 1) & 3, 0, "bloc zlib non stocké : le décodeur du test ne lit que les blocs stockés");
        let longueur = u16::from_le_bytes([idat[j + 1], idat[j + 2]]) as usize;
        brut.extend_from_slice(&idat[j + 5..j + 5 + longueur]);
        j += 5 + longueur;
        if entete & 1 == 1 {
            break;
        }
    }
    assert_eq!(brut.len(), hauteur * (1 + 3 * largeur), "taille des pixels décodés");
    let mut pixels = Vec::with_capacity(largeur * hauteur * 3);
    for ligne in brut.chunks(1 + 3 * largeur) {
        assert_eq!(ligne[0], 0, "filtre PNG nul attendu");
        pixels.extend_from_slice(&ligne[1..]);
    }
    Image { largeur, hauteur, pixels }
}

/// 8 (suite). L'aperçu dessine les obstacles en gris : chaque case d'obstacle est un bloc de pixels d'une seule
///    couleur, grise (rouge = vert = bleu), ni noire (le noir est une case sans substrat) ni la couleur que la même
///    case avait sans obstacle ; hors de l'obstacle, l'aperçu est celui du même scénario sans obstacle, pixel pour
///    pixel. L'échelle (pixels par case) se déduit de la taille de l'image.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn l_apercu_dessine_les_obstacles_en_gris() {
    let temp = Temp::nouveau("apercu");
    // Un obstacle seul : le substrat reste uniforme autour de lui, donc rien d'autre ne change dans l'aperçu.
    let avec = temp.scenario("avec.txt", &format!("{SCENARIO_SANS_MILIEU}obstacle 40 10 43 30\n"));
    let sans = temp.scenario("sans.txt", SCENARIO_SANS_MILIEU);
    let (a, b) = (temp.sortie("a"), temp.sortie("b"));
    lancer_et_reussir(&avec, 5, &a);
    lancer_et_reussir(&sans, 5, &b);
    let avec = decoder_png(&lire(&a, "apercu.png"));
    let sans = decoder_png(&lire(&b, "apercu.png"));
    assert_eq!((avec.largeur, avec.hauteur), (sans.largeur, sans.hauteur), "les deux aperçus n'ont pas la même taille");
    assert!(
        avec.largeur.is_multiple_of(60) && avec.hauteur.is_multiple_of(50),
        "l'aperçu ({}×{}) n'est pas un multiple de la boîte 60×50",
        avec.largeur,
        avec.hauteur
    );
    let echelle = avec.largeur / 60;
    assert!(echelle >= 1 && avec.hauteur / 50 == echelle, "échelle de l'aperçu");

    for case_y in 0..50usize {
        for case_x in 0..60usize {
            let obstacle = (40..=43).contains(&case_x) && (10..=30).contains(&case_y);
            let premier = avec.pixel(case_x * echelle, case_y * echelle);
            let de_sans = sans.pixel(case_x * echelle, case_y * echelle);
            for py in 0..echelle {
                for px in 0..echelle {
                    let (x, y) = (case_x * echelle + px, case_y * echelle + py);
                    if obstacle {
                        assert_eq!(
                            avec.pixel(x, y),
                            premier,
                            "la case d'obstacle ({case_x}, {case_y}) n'est pas d'une seule couleur"
                        );
                    } else {
                        assert_eq!(
                            avec.pixel(x, y),
                            sans.pixel(x, y),
                            "l'aperçu diffère hors de l'obstacle, en pixel ({x}, {y})"
                        );
                    }
                }
            }
            if obstacle {
                assert!(
                    premier[0] == premier[1] && premier[1] == premier[2],
                    "la case d'obstacle ({case_x}, {case_y}) n'est pas grise : {premier:?}"
                );
                assert_ne!(
                    premier,
                    [0, 0, 0],
                    "la case d'obstacle ({case_x}, {case_y}) est noire, comme une case sans substrat"
                );
                assert_ne!(
                    premier, de_sans,
                    "la case d'obstacle ({case_x}, {case_y}) a la couleur qu'elle a sans obstacle"
                );
            }
        }
    }
}

/// 6 (suite). Un milieu hors contrat fait échouer le binaire proprement : un code de sortie non nul qui n'est pas
///    celui d'une panique (101), pour un obstacle sur la spore, une source hors de la boîte, un obstacle hors de la
///    boîte, une ligne mal formée.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_milieu_invalide_fait_echouer_le_binaire_sans_paniquer() {
    let temp = Temp::nouveau("invalide");
    for (k, supplement) in [
        "obstacle 8 23 12 27\n",
        "source 60 10 2 1.0\n",
        "obstacle 50 40 70 45\n",
        "source 5 5 2 -1.0\n",
        "obstacle 5 5 4 4\n",
        "source 1 2 3\n",
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
