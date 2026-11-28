//! Tests d'acceptation de l'étape 28 (c) : le module `docs/mycelium.wasm` reconstruit et les scènes 5, 6, 7 et 8 de
//! la colonie, lues par Node.
//!
//! Comme aux étapes 4, 8, 11, 16, 18, 21 et 25, la CI n'a pas la cible WebAssembly : le module est construit à la main
//! puis commité, et la preuve qu'il est bon est qu'il tourne, sous Node, et donne la même chose que le code natif. Ici,
//! pour les scènes de l'étape 27 : `tests/etape_28/scenes-wasm.mjs` lit la part de la cavité atteinte avant tout
//! démarrage, puis, après chaque morceau de pas, l'état des scènes 5 (la boule), 6 (le tore), 7 (la coquille) et 8
//! (le labyrinthe) : temps, durée, dimensions, comptes, SHA-256 des obstacles, bits de la part remplie et empreinte.
//! Ce test le compare, chaîne pour chaîne, à ce que donne `Colonie3d` construit **sans passer par
//! `vitrine_colonie3d.rs`** (le texte de la scène est lu sur le disque, puis développé par `moule3d::developper_3d`).
//!
//! 1. la forme du module : l'exportation nouvelle `mycelium_colonie_part_remplie` et sa signature (aucun paramètre,
//!    `f64` en résultat) ;
//! 2. le résultat du module, lu par Node : la part d'avant tout démarrage (`NaN`), le refus de `choisir(9)` et de
//!    `choisir(4294967295)`, puis, pour chaque scène (5, 6, 7 et 8), le départ et les morceaux `1, 0, 9`, le reste de
//!    la durée, et l'état de chacun ;
//! 3. à la fin de chaque scène, l'empreinte lue dans le module est celle de `references/`, la part vaut exactement la
//!    valeur écrite, les nombres d'obstacles sont ceux des fichiers, et le cube a ses dimensions ;
//! 4. les fonctions natives de `vitrine_colonie3d.rs` rendent les mêmes lignes que `Colonie3d`, sous le verrou de ce
//!    fichier (l'état global est unique au processus) ; pour les cinq scènes d'avant (0 à 4), à la fin de leur durée,
//!    la part lue dans la vitrine est `part_remplie` de la colonie native (pour colonie-cube, `828.0 / 32768.0`
//!    exactement), et un refus ne change ni la part ni l'empreinte de la scène en cours.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_bits` et `to_le_bytes`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, lire_scenario_colonie3d};
use mycelium_lab::empreinte;
use mycelium_lab::moule3d::{developper_3d, part_remplie};
use mycelium_lab::vitrine_colonie3d::{
    mycelium_colonie_avancer, mycelium_colonie_choisir, mycelium_colonie_demarrer, mycelium_colonie_dimension,
    mycelium_colonie_duree, mycelium_colonie_empreinte_mot, mycelium_colonie_nb_obstacles,
    mycelium_colonie_nb_segments, mycelium_colonie_obstacle, mycelium_colonie_part_remplie, mycelium_colonie_pointes,
    mycelium_colonie_ramifications, mycelium_colonie_temps,
};

/// Une des quatre scènes de l'étape 27, telle que son fichier de `scenarios/` la fixe.
struct Scene {
    /// Le numéro que prend `mycelium_colonie_choisir`.
    numero: u32,
    /// Le nom dans la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// Le fichier d'empreinte sous `references/`.
    reference: &'static str,
    /// La durée (`duree` du fichier).
    duree: u32,
    /// La largeur du cube.
    largeur: u32,
    /// La hauteur du cube.
    hauteur: u32,
    /// La profondeur du cube.
    profondeur: u32,
    /// Le nombre d'obstacles du moule développé.
    obstacles: u32,
    /// La part de la cavité atteinte à la fin, graine 1 (`part_remplie`).
    part: f64,
}

/// Les quatre scènes, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas du code.
const SCENES: [Scene; 4] = [
    Scene {
        numero: 5,
        nom: "boule",
        fichier: "colonie-boule.txt",
        reference: "colonie-boule.empreinte",
        duree: 100,
        largeur: 32,
        hauteur: 32,
        profondeur: 32,
        obstacles: 570,
        part: 0.756101797967515,
    },
    Scene {
        numero: 6,
        nom: "tore",
        fichier: "colonie-tore.txt",
        reference: "colonie-tore.empreinte",
        duree: 130,
        largeur: 32,
        hauteur: 32,
        profondeur: 32,
        obstacles: 291,
        part: 0.8205213903743316,
    },
    Scene {
        numero: 7,
        nom: "coquille",
        fichier: "colonie-coquille.txt",
        reference: "colonie-coquille.empreinte",
        duree: 95,
        largeur: 32,
        hauteur: 32,
        profondeur: 32,
        obstacles: 7,
        part: 0.6778131634819533,
    },
    Scene {
        numero: 8,
        nom: "labyrinthe",
        fichier: "colonie-labyrinthe.txt",
        reference: "colonie-labyrinthe.empreinte",
        duree: 120,
        largeur: 16,
        hauteur: 16,
        profondeur: 16,
        obstacles: 37,
        part: 0.7402052238805971,
    },
];

/// La graine des empreintes de référence.
const GRAINE: u32 = 1;

/// Les deux numéros que `mycelium_colonie_choisir` doit refuser maintenant que les scènes vont de 0 à 8.
const NUMEROS_REFUSES: [u32; 2] = [9, u32::MAX];

/// Les cinq scènes d'avant (0 à 4), dans l'ordre de leurs numéros ; leurs fichiers sous `scenarios/`.
const ANCIENNES: [&str; 5] =
    ["colonie-cube.txt", "colonie-goutte.txt", "colonie-chimio.txt", "colonie-mur.txt", "colonie-anastomose.txt"];

/// Le type de valeur WebAssembly qui sert ici, tel que le format binaire le code : `f64` reste `f64`.
const F64: u8 = 0x7C;

/// La fonction nouvelle que le module doit exporter, avec sa signature : aucun paramètre, un `f64` en résultat.
const FONCTION: (&str, &[u8], &[u8]) = ("mycelium_colonie_part_remplie", &[], &[F64]);

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn chemin_script() -> PathBuf {
    racine().join("tests").join("etape_28").join("scenes-wasm.mjs")
}

fn lire_module() -> Vec<u8> {
    std::fs::read(chemin_module()).unwrap_or_else(|erreur| panic!("docs/mycelium.wasm se lit : {erreur}"))
}

// ---------------------------------------------------------------------------------------------------------------
// Un petit lecteur du format binaire WebAssembly : juste ce qu'il faut pour les sections de types, d'importations,
// de fonctions et d'exportations (le même que celui de `etape_25_artefact.rs`).
// ---------------------------------------------------------------------------------------------------------------

/// Un lecteur d'octets avec une position.
struct Lecteur<'a> {
    octets: &'a [u8],
    position: usize,
}

impl<'a> Lecteur<'a> {
    fn new(octets: &'a [u8]) -> Lecteur<'a> {
        Lecteur { octets, position: 0 }
    }

    fn fini(&self) -> bool {
        self.position >= self.octets.len()
    }

    fn octet(&mut self) -> u8 {
        let octet = *self.octets.get(self.position).expect("fin inattendue du module");
        self.position += 1;
        octet
    }

    /// Un entier non signé de 32 bits en LEB128 : 7 bits par octet, le bit de poids fort dit qu'il y en a un autre.
    fn entier(&mut self) -> u32 {
        let mut valeur: u64 = 0;
        for rang in 0..5 {
            let octet = self.octet();
            valeur |= u64::from(octet & 0x7F) << (7 * rang);
            if octet & 0x80 == 0 {
                return u32::try_from(valeur).expect("entier LEB128 trop grand pour 32 bits");
            }
        }
        panic!("entier LEB128 de plus de 5 octets");
    }

    fn tranche(&mut self, longueur: usize) -> &'a [u8] {
        let fin = self.position.checked_add(longueur).filter(|fin| *fin <= self.octets.len());
        let fin = fin.expect("section plus longue que le module");
        let tranche = &self.octets[self.position..fin];
        self.position = fin;
        tranche
    }

    fn nom(&mut self) -> String {
        let longueur = self.entier() as usize;
        String::from_utf8(self.tranche(longueur).to_vec()).expect("nom en UTF-8")
    }
}

/// Une exportation : son nom, son genre (0 fonction, 1 table, 2 mémoire, 3 globale) et son indice.
struct Exportation {
    nom: String,
    genre: u8,
    indice: u32,
}

/// Ce que le test lit d'un module.
struct Module {
    /// Le nombre d'importations (0 si la section est absente).
    importations: u32,
    /// La section des types : pour chacun, les types des paramètres et ceux des résultats.
    types: Vec<(Vec<u8>, Vec<u8>)>,
    /// La section des fonctions : l'indice de type de chaque fonction définie dans le module.
    fonctions: Vec<u32>,
    exportations: Vec<Exportation>,
}

/// Analyse un module : l'en-tête (`\0asm` puis la version 1), puis les sections repérées par leur numéro.
fn analyser(octets: &[u8]) -> Module {
    assert!(octets.len() >= 8, "module trop court : {} octets", octets.len());
    assert_eq!(&octets[0..4], b"\0asm", "le module ne commence pas par \\0asm");
    assert_eq!(octets[4..8], [1, 0, 0, 0], "le module n'est pas en version 1");

    let mut module = Module { importations: 0, types: Vec::new(), fonctions: Vec::new(), exportations: Vec::new() };
    let mut lecteur = Lecteur::new(&octets[8..]);
    while !lecteur.fini() {
        let numero = lecteur.octet();
        let taille = lecteur.entier() as usize;
        let mut section = Lecteur::new(lecteur.tranche(taille));
        match numero {
            1 => {
                for _ in 0..section.entier() {
                    assert_eq!(section.octet(), 0x60, "un type de fonction commence par 0x60");
                    let parametres = (0..section.entier()).map(|_| section.octet()).collect();
                    let resultats = (0..section.entier()).map(|_| section.octet()).collect();
                    module.types.push((parametres, resultats));
                }
            }
            2 => module.importations = section.entier(),
            3 => {
                for _ in 0..section.entier() {
                    module.fonctions.push(section.entier());
                }
            }
            7 => {
                for _ in 0..section.entier() {
                    let nom = section.nom();
                    let genre = section.octet();
                    let indice = section.entier();
                    module.exportations.push(Exportation { nom, genre, indice });
                }
            }
            _ => {}
        }
    }
    module
}

/// Vérifie la forme des exportations : chaque fonction attendue est exportée, toute autre fonction exportée porte un
/// nom qui commence par `mycelium`, un nom n'est exporté qu'une fois, et hors des fonctions seule `memory` est
/// exportée.
fn verifier_les_exportations(module: &Module, attendues: &BTreeSet<&str>) {
    let fonctions: BTreeSet<&str> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.as_str()).collect();
    for nom in attendues {
        assert!(fonctions.contains(nom), "la fonction {nom} n'est pas exportée");
    }
    for nom in fonctions.difference(attendues) {
        assert!(nom.starts_with("mycelium"), "fonction exportée inattendue : « {nom} » ne commence pas par mycelium");
    }

    let noms: Vec<&str> = module.exportations.iter().map(|e| e.nom.as_str()).collect();
    let uniques: BTreeSet<&str> = noms.iter().copied().collect();
    assert_eq!(uniques.len(), noms.len(), "un nom est exporté deux fois");

    let autres: Vec<(&str, u8)> =
        module.exportations.iter().filter(|e| e.genre != 0).map(|e| (e.nom.as_str(), e.genre)).collect();
    for (nom, genre) in &autres {
        assert!((*nom, *genre) == ("memory", 2), "exportation inattendue : « {nom} » (genre {genre})");
    }
    assert_eq!(module.exportations.len(), fonctions.len() + autres.len());
}

/// 1. Le module exporte `mycelium_colonie_part_remplie` avec sa signature (aucun paramètre, `f64` en résultat) ;
///    toute autre fonction exportée porte un nom qui commence par `mycelium`, un nom n'est exporté qu'une fois, et la
///    mémoire `memory` est la seule exportation qui ne soit pas une fonction. Le module n'importe toujours rien.
#[test]
#[cfg_attr(not(feature = "etape-28"), ignore)]
fn le_module_exporte_la_part_remplie_avec_sa_signature() {
    let module = analyser(&lire_module());
    assert_eq!(module.importations, 0, "le module a {} importation(s)", module.importations);
    let attendues: BTreeSet<&str> = BTreeSet::from([FONCTION.0]);
    verifier_les_exportations(&module, &attendues);

    let (nom, parametres, resultats) = FONCTION;
    let exportation = module
        .exportations
        .iter()
        .find(|e| e.genre == 0 && e.nom == nom)
        .unwrap_or_else(|| panic!("la fonction {nom} n'est pas exportée"));
    let indice_de_type = *module
        .fonctions
        .get(exportation.indice as usize)
        .unwrap_or_else(|| panic!("{nom} : indice de fonction {} hors de la section", exportation.indice));
    let (obtenus, rendus) = module
        .types
        .get(indice_de_type as usize)
        .unwrap_or_else(|| panic!("{nom} : type {indice_de_type} absent de la section des types"));
    assert_eq!(obtenus.as_slice(), parametres, "{nom} : types des paramètres");
    assert_eq!(rendus.as_slice(), resultats, "{nom} : types des résultats");
}

// ---------------------------------------------------------------------------------------------------------------
// Node.
// ---------------------------------------------------------------------------------------------------------------

/// Vrai si Node est là. Sinon : le test échoue si la variable d'environnement `CI` est définie (les machines de la
/// CI ont Node, et un test qui ne vérifie rien n'y est pas acceptable) ; autrement une note sur la sortie d'erreur
/// et le test passe (un poste sans Node ne bloque pas le développement).
fn node_est_la() -> bool {
    let present = Command::new("node").arg("--version").output().map(|sortie| sortie.status.success()).unwrap_or(false);
    if !present {
        assert!(std::env::var_os("CI").is_none(), "Node est introuvable alors que la variable CI est définie");
        eprintln!("note : Node est absent, ce test ne vérifie rien sur ce poste");
    }
    present
}

/// Lance `scenes-wasm.mjs` sur le module et rend sa sortie standard, en exigeant qu'il réussisse.
fn sortie_du_module() -> String {
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");
    let sortie: Output = Command::new("node")
        .arg(chemin_script())
        .arg(module)
        .output()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"));
    assert!(
        sortie.status.success(),
        "scenes-wasm.mjs : {:?}, sortie d'erreur : {}",
        sortie.status,
        String::from_utf8_lossy(&sortie.stderr)
    );
    String::from_utf8(sortie.stdout).expect("la sortie du script est en UTF-8")
}

// ---------------------------------------------------------------------------------------------------------------
// Les lignes d'état, calculées sans passer par `vitrine_colonie3d.rs` : la scène se lit sur le disque.
// ---------------------------------------------------------------------------------------------------------------

/// Le scénario d'un fichier de `scenarios/`, lu sur le disque (jamais par `include_str!`) et développé par le moule.
fn scenario_du_disque(fichier: &str) -> ScenarioColonie3d {
    let chemin = racine().join("scenarios").join(fichier);
    let texte =
        std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", chemin.display()));
    let developpe = developper_3d(&texte).unwrap_or_else(|erreur| panic!("{fichier} : moule refusé {erreur:?}"));
    lire_scenario_colonie3d(&developpe).unwrap_or_else(|erreur| panic!("{fichier} : scénario refusé {erreur:?}"))
}

/// La colonie d'une scène, à la graine `GRAINE`, au temps 0.
fn colonie_du_disque(scene: &Scene) -> Colonie3d {
    let scenario = scenario_du_disque(scene.fichier);
    Colonie3d::nouvelle(&scenario, u64::from(GRAINE)).expect("le scénario du disque se construit")
}

/// Le SHA-256 des coordonnées des obstacles, six entiers u32 petit-boutistes par obstacle, dans l'ordre du scénario.
fn hacher_obstacles(obstacles: &[mycelium_lab::cube::Bloc]) -> String {
    let mut octets = Vec::new();
    for obstacle in obstacles {
        for coordonnee in [obstacle.x0, obstacle.y0, obstacle.z0, obstacle.x1, obstacle.y1, obstacle.z1] {
            octets.extend_from_slice(&coordonnee.to_le_bytes());
        }
    }
    empreinte::sha256_hex(&octets)
}

/// Une ligne d'état native : temps, durée, dimensions, comptes, SHA-256 des obstacles, bits de la part, empreinte.
fn ligne_etat(nom: &str, colonie: &Colonie3d) -> String {
    let scenario = colonie.scenario();
    format!(
        "{nom}={},{},{},{},{},{},{},{},{},{},{:016x},{}\n",
        colonie.temps(),
        colonie.duree(),
        scenario.largeur,
        scenario.hauteur,
        scenario.profondeur,
        colonie.segments().len(),
        colonie.pointes(),
        colonie.nb_ramifications(),
        scenario.obstacles.len(),
        hacher_obstacles(&scenario.obstacles),
        part_remplie(colonie).to_bits(),
        colonie.empreinte()
    )
}

/// Joue une scène pas à pas, à la graine `GRAINE`, et rend ses lignes d'état : le départ puis une par morceau.
fn jouer_scene(scene: &Scene) -> Vec<String> {
    let mut colonie = colonie_du_disque(scene);
    assert_eq!(colonie.duree(), scene.duree, "{} : la durée du fichier", scene.nom);
    assert_eq!(
        colonie.scenario().obstacles.len() as u32,
        scene.obstacles,
        "{} : le nombre d'obstacles du moule",
        scene.nom
    );
    let mut lignes = vec![ligne_etat(&format!("scenario_{}_depart", scene.numero), &colonie)];
    let duree = colonie.duree();
    for (i, pas) in [1u32, 0, 9, duree - 10].iter().enumerate() {
        colonie.avancer_n(*pas);
        lignes.push(ligne_etat(&format!("scenario_{}_{}", scene.numero, i), &colonie));
    }
    lignes
}

/// Ce que `scenes-wasm.mjs` doit écrire : la part d'avant tout démarrage, puis, pour chaque scène, le départ et les
/// morceaux `1, 0, 9, duree - 10`.
fn lignes_attendues() -> String {
    let mut texte = format!("avant={:016x}\n", f64::NAN.to_bits());
    for scene in &SCENES {
        for ligne in jouer_scene(scene) {
            texte.push_str(&ligne);
        }
    }
    texte
}

/// 2. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne pour **chacune des scènes 5,
///    6, 7 et 8** les mêmes lignes que `Colonie3d` construit sur le texte du disque et joué un pas à la fois : la
///    part d'un module neuf (`NaN`), le refus de `choisir(9)` et de `choisir(4294967295)` sans changement, puis le
///    départ et chaque morceau `1, 0, 9, duree - 10` (temps, durée, dimensions, comptes, SHA-256 des obstacles, bits
///    de la part remplie et empreinte). Si le code change sans que le module soit reconstruit, ce test échoue.
#[test]
#[cfg_attr(not(feature = "etape-28"), ignore)]
fn node_donne_les_memes_lignes_que_la_colonie_native_pour_les_scenes_5_a_8() {
    if !node_est_la() {
        return;
    }
    assert_eq!(sortie_du_module(), lignes_attendues(), "le module et la colonie native de chaque scène");
}

/// L'empreinte de référence d'un fichier de `references/` : 64 chiffres hexadécimaux minuscules, une ligne.
fn empreinte_de_reference(fichier: &str) -> String {
    let chemin = racine().join("references").join(fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} : {erreur}", chemin.display()));
    assert_eq!(texte.lines().count(), 1, "{fichier} : une seule ligne");
    let empreinte = texte.trim();
    assert_eq!(empreinte.len(), 64, "{fichier} : 64 chiffres hexadécimaux");
    assert!(
        empreinte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "{fichier} : des chiffres hexadécimaux minuscules"
    );
    empreinte.to_string()
}

/// La dernière ligne d'état d'une scène, dans la sortie du script.
fn derniere_ligne<'a>(sortie: &'a str, scene: &Scene) -> &'a str {
    let prefixe = format!("scenario_{}_", scene.numero);
    sortie
        .lines()
        .rev()
        .find(|ligne| ligne.starts_with(&prefixe))
        .unwrap_or_else(|| panic!("{} : aucune ligne d'état", scene.nom))
}

/// Les champs d'une ligne d'état, après son `nom=`.
fn champs(ligne: &str) -> Vec<&str> {
    let (_, valeurs) = ligne.split_once('=').expect("une ligne d'état a un signe égal");
    valeurs.split(',').collect()
}

/// Le motif de bits d'une part lue dans une ligne d'état, en `u64` (seize chiffres hexadécimaux minuscules).
fn bits_de_la_part(ligne: &str) -> u64 {
    let valeur = champs(ligne)[10];
    assert_eq!(valeur.len(), 16, "les bits de la part font seize chiffres hexadécimaux");
    u64::from_str_radix(valeur, 16).expect("les bits de la part sont hexadécimaux")
}

/// 3. À la fin de chaque scène, l'empreinte lue dans le module est celle du fichier de `references/` de la scène, la
///    part vaut exactement la valeur écrite, le nombre d'obstacles est celui du fichier, et le cube a ses dimensions.
#[test]
#[cfg_attr(not(feature = "etape-28"), ignore)]
fn la_fin_de_chaque_scene_a_son_empreinte_sa_part_et_ses_obstacles() {
    if !node_est_la() {
        return;
    }
    let sortie = sortie_du_module();

    for scene in &SCENES {
        let ligne = derniere_ligne(&sortie, scene);
        let valeurs = champs(ligne);
        assert_eq!(valeurs.len(), 12, "{} : une ligne d'état a douze champs", scene.nom);
        assert_eq!(valeurs[11], empreinte_de_reference(scene.reference), "{} : l'empreinte de la fin", scene.nom);
        assert_eq!(bits_de_la_part(ligne), scene.part.to_bits(), "{} : la part de la fin", scene.nom);
        assert_eq!(valeurs[8], scene.obstacles.to_string(), "{} : le nombre d'obstacles", scene.nom);

        let (largeur, hauteur, profondeur) = (scene.largeur, scene.hauteur, scene.profondeur);
        assert_eq!(
            [valeurs[2], valeurs[3], valeurs[4]],
            [largeur.to_string(), hauteur.to_string(), profondeur.to_string()],
            "{} : les dimensions du cube",
            scene.nom
        );
        assert_eq!(valeurs[0], scene.duree.to_string(), "{} : le temps de la fin", scene.nom);
    }

    // La part écrite est exactement la valeur du fichier de l'étape 27, pas une approximation.
    assert_eq!(SCENES[0].part, 8705.0 / 11513.0, "la boule : 8705 cases sur 11513");
    assert_eq!(SCENES[1].part, 2455.0 / 2992.0, "le tore : 2455 cases sur 2992");
    assert_eq!(SCENES[2].part, 7662.0 / 11304.0, "la coquille : 7662 cases sur 11304");
    assert_eq!(SCENES[3].part, 1587.0 / 2144.0, "le labyrinthe : 1587 cases sur 2144");
}

// ---------------------------------------------------------------------------------------------------------------
// La colonie native de `vitrine_colonie3d.rs`, sous le verrou de ce fichier : l'état global est unique au processus,
// et aucun autre test de ce fichier ne le touche.
// ---------------------------------------------------------------------------------------------------------------

/// Sérialise les tests de ce fichier qui emploient l'état global de `vitrine_colonie3d`.
static VERROU: Mutex<()> = Mutex::new(());

/// Prend le verrou ; un test qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// La part d'avant tout démarrage, par les fonctions natives de `vitrine_colonie3d` : les seize chiffres hexadécimaux
/// de ses bits (`NaN`).
fn part_native_avant() -> String {
    format!("{:016x}", mycelium_colonie_part_remplie().to_bits())
}

/// Les huit mots de l'empreinte de la colonie en cours, en 64 chiffres hexadécimaux minuscules.
fn empreinte_native() -> String {
    let mut empreinte = String::new();
    for i in 0..8 {
        empreinte.push_str(&format!("{:08x}", mycelium_colonie_empreinte_mot(i)));
    }
    empreinte
}

/// Le SHA-256 des obstacles de la colonie en cours, lus par les fonctions natives.
fn hacher_obstacles_natifs() -> String {
    let n = mycelium_colonie_nb_obstacles();
    let mut octets = Vec::new();
    for i in 0..n {
        for k in 0..6 {
            octets.extend_from_slice(&(mycelium_colonie_obstacle(i, k) as u32).to_le_bytes());
        }
    }
    empreinte::sha256_hex(&octets)
}

/// Une ligne d'état lue par les fonctions natives de `vitrine_colonie3d`, dans le même format que `ligne_etat`.
fn ligne_native(nom: &str) -> String {
    format!(
        "{nom}={},{},{},{},{},{},{},{},{},{},{:016x},{}\n",
        mycelium_colonie_temps(),
        mycelium_colonie_duree(),
        mycelium_colonie_dimension(0),
        mycelium_colonie_dimension(1),
        mycelium_colonie_dimension(2),
        mycelium_colonie_nb_segments(),
        mycelium_colonie_pointes(),
        mycelium_colonie_ramifications(),
        mycelium_colonie_nb_obstacles(),
        hacher_obstacles_natifs(),
        mycelium_colonie_part_remplie().to_bits(),
        empreinte_native()
    )
}

/// Les lignes que rendent les fonctions natives de `vitrine_colonie3d` : l'état neuf, les deux numéros refusés, les
/// quatre scènes avec les mêmes morceaux que le script.
fn sequence_native() -> String {
    let avant = part_native_avant();
    let mut texte = format!("avant={avant}\n");
    for numero in NUMEROS_REFUSES {
        assert_eq!(mycelium_colonie_choisir(numero), 0, "la colonie native accepte le numéro de scène {numero}");
        let apres = part_native_avant();
        assert_eq!(apres, avant, "choisir({numero}) a changé l'état alors qu'il devait rendre 0 sans rien changer");
    }

    for scene in &SCENES {
        assert_eq!(mycelium_colonie_choisir(scene.numero), 1, "{} : la colonie native refuse la scène", scene.nom);
        mycelium_colonie_demarrer(GRAINE);
        texte.push_str(&ligne_native(&format!("scenario_{}_depart", scene.numero)));
        let duree = mycelium_colonie_duree();
        assert_eq!(duree, scene.duree, "{} : la durée de la colonie démarrée", scene.nom);
        for (i, pas) in [1u32, 0, 9, duree - 10].iter().enumerate() {
            mycelium_colonie_avancer(*pas);
            texte.push_str(&ligne_native(&format!("scenario_{}_{}", scene.numero, i)));
        }
    }
    texte
}

/// Vérifie, pour les cinq scènes d'avant (0 à 4), qu'à la fin de leur durée la part lue dans la vitrine est
/// `part_remplie` de la colonie native de même scène et de même graine ; colonie-cube vaut `828.0 / 32768.0`.
fn verifier_les_scenes_d_avant() {
    for (numero, fichier) in ANCIENNES.iter().enumerate() {
        let numero = numero as u32;
        assert_eq!(mycelium_colonie_choisir(numero), 1, "{fichier} : la colonie native refuse la scène {numero}");
        mycelium_colonie_demarrer(GRAINE);

        let scenario = scenario_du_disque(fichier);
        let mut native = Colonie3d::nouvelle(&scenario, u64::from(GRAINE)).expect("le scénario du disque se construit");
        let duree = native.duree();
        assert_eq!(mycelium_colonie_duree(), duree, "{fichier} : la durée de la colonie démarrée");

        mycelium_colonie_avancer(duree);
        native.avancer_n(duree);
        assert_eq!(mycelium_colonie_temps(), duree, "{fichier} : le temps est la durée");

        let part = mycelium_colonie_part_remplie();
        assert_eq!(part, part_remplie(&native), "{fichier} : la part de la vitrine et celle de la colonie native");
        if numero == 0 {
            assert_eq!(part, 828.0 / 32768.0, "colonie-cube : 828 cases atteintes sur 32768");
        }
    }
}

/// 4. La colonie native de `vitrine_colonie3d.rs` rend les mêmes lignes que `Colonie3d` (par `src/colonie3d.rs`),
///    avec les mêmes bords avant comme après un pas : l'état neuf rend `NaN`, `choisir(9)` et `choisir(4294967295)`
///    rendent 0 sans rien changer. Pour les cinq scènes d'avant (0 à 4), à la fin de leur durée, la part lue dans la
///    vitrine est celle de la colonie native, et un refus ne change ni la part ni l'empreinte de la scène en cours.
///    Le verrou rend fiable le « avant tout démarrage » : aucun autre test de ce fichier ne touche à l'état global.
#[test]
#[cfg_attr(not(feature = "etape-28"), ignore)]
fn la_vitrine_native_donne_les_memes_lignes_et_la_meme_part() {
    let _verrou = verrou();
    assert_eq!(sequence_native(), lignes_attendues(), "la colonie native de vitrine_colonie3d et Colonie3d");

    verifier_les_scenes_d_avant();

    // Un refus après une scène ne change ni sa part ni son empreinte.
    let part = mycelium_colonie_part_remplie().to_bits();
    let empreinte = empreinte_native();
    assert_eq!(mycelium_colonie_choisir(9), 0, "la colonie native accepte le numéro de scène 9 après une scène");
    assert_eq!(mycelium_colonie_part_remplie().to_bits(), part, "choisir(9) a changé la part de la scène en cours");
    assert_eq!(empreinte_native(), empreinte, "choisir(9) a changé l'empreinte de la scène en cours");
}
