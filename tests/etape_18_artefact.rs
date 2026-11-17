//! Tests d'acceptation de l'étape 18 (c) : le module `docs/mycelium.wasm` reconstruit et le cube de la page, lu par
//! Node.
//!
//! Comme aux étapes 4, 8, 11 et 16, la CI n'a pas la cible WebAssembly : le module est construit à la main puis
//! commité, et la preuve qu'il est bon est qu'il tourne, sous Node, et donne la même chose que le code natif. Ici,
//! pour le cube de l'étape 18 : `tests/etape_18/cube-wasm.mjs` lit, avant tout démarrage puis après chaque morceau
//! de pas, l'état du cube (temps, durée, dimensions, bits de la masse et du maximum, SHA-256 des valeurs, nombre de
//! cases bloquées et empreinte), et ce test le compare, chaîne pour chaîne, à ce que donne `Cube` construit
//! **sans passer par `vitrine_cube.rs`** (le texte de la scène est lu sur le disque).
//!
//! 1. la forme du module : ses onze exportations `mycelium_cube_*` et leurs signatures (les vingt et une d'avant
//!    gardent la leur, ce que disent les tests des étapes 4, 8, 11 et 16 sur ce même module) ;
//! 2. le résultat du module, lu par Node : ce qu'il rend avant tout démarrage, le refus de `choisir(2)`, puis, pour
//!    chaque scène (la goutte puis le mur), les morceaux `1, 0, 49`, le reste de la durée, et l'état de chacun ;
//! 3. à la fin de chaque scène, l'empreinte lue dans le module est celle de `references/` ;
//! 4. hors du volume, `valeur` rend `NaN` et `bloquee` 0, et le cube natif de `vitrine_cube.rs` rend les mêmes
//!    lignes que `Cube` : le test natif prend le verrou de ce fichier, car l'état global est unique au processus.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_bits`, `to_le_bytes`, `is_nan` et `from`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

use mycelium_lab::cube::{Cube, lire_scenario_cube};
use mycelium_lab::empreinte;
use mycelium_lab::vitrine_cube::{
    mycelium_cube_avancer, mycelium_cube_bloquee, mycelium_cube_choisir, mycelium_cube_demarrer,
    mycelium_cube_dimension, mycelium_cube_duree, mycelium_cube_empreinte_mot, mycelium_cube_masse,
    mycelium_cube_maximum, mycelium_cube_temps, mycelium_cube_valeur,
};

/// Une des deux scènes du cube, telle que la spécification de l'étape 18 la fixe.
struct Scene {
    /// Le numéro que prend `mycelium_cube_choisir`.
    numero: u32,
    /// Le nom dans la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// Le fichier d'empreinte sous `references/`.
    reference: &'static str,
    /// La durée (`duree` du fichier).
    duree: u32,
}

/// Les deux scènes, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas du code.
const SCENES: [Scene; 2] = [
    Scene { numero: 0, nom: "goutte", fichier: "cube-goutte.txt", reference: "cube-goutte.empreinte", duree: 300 },
    Scene { numero: 1, nom: "mur", fichier: "cube-mur.txt", reference: "cube-mur.empreinte", duree: 400 },
];

/// Les types de valeurs WebAssembly qui servent ici, tels que le format binaire les code.
const I32: u8 = 0x7F;
const F64: u8 = 0x7C;

/// Les onze fonctions que le module doit exporter, avec leur signature : types des paramètres, types des résultats
/// (`u32` est un `i32` en WebAssembly, `f64` reste un `f64`).
const FONCTIONS: [(&str, &[u8], &[u8]); 11] = [
    ("mycelium_cube_choisir", &[I32], &[I32]),
    ("mycelium_cube_demarrer", &[], &[]),
    ("mycelium_cube_avancer", &[I32], &[]),
    ("mycelium_cube_temps", &[], &[I32]),
    ("mycelium_cube_duree", &[], &[I32]),
    ("mycelium_cube_dimension", &[I32], &[I32]),
    ("mycelium_cube_valeur", &[I32, I32, I32], &[F64]),
    ("mycelium_cube_bloquee", &[I32, I32, I32], &[I32]),
    ("mycelium_cube_maximum", &[], &[F64]),
    ("mycelium_cube_masse", &[], &[F64]),
    ("mycelium_cube_empreinte_mot", &[I32], &[I32]),
];

/// Les douze lectures d'un module neuf, avant tout démarrage : les entiers rendent 0, les flottants `NaN`.
const AVANT: &str = "0,0,0,0,0,0,NaN,0,NaN,NaN,0,0";

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn chemin_script(nom: &str) -> PathBuf {
    racine().join("tests").join("etape_18").join(nom)
}

fn lire_module() -> Vec<u8> {
    std::fs::read(chemin_module()).unwrap_or_else(|erreur| panic!("docs/mycelium.wasm se lit : {erreur}"))
}

// ---------------------------------------------------------------------------------------------------------------
// Un petit lecteur du format binaire WebAssembly : juste ce qu'il faut pour les sections de types, d'importations,
// de fonctions et d'exportations (le même que celui de `etape_16_artefact.rs`).
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

/// 1. Le module exporte les onze fonctions `mycelium_cube_*` de la liste ; toute autre fonction exportée porte un nom
///    qui commence par `mycelium`, un nom n'est exporté qu'une fois, et la mémoire `memory` est la seule exportation
///    qui ne soit pas une fonction.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn le_module_exporte_les_onze_fonctions_du_cube() {
    let module = analyser(&lire_module());
    let attendues: BTreeSet<&str> = FONCTIONS.iter().map(|(nom, _, _)| *nom).collect();
    assert_eq!(attendues.len(), 11);
    verifier_les_exportations(&module, &attendues);
}

/// 1 (suite). Chaque fonction du cube a la bonne signature : le nombre et le type des paramètres, et du résultat
/// (`i32` pour un `u32`, `f64` pour un `f64`). `mycelium_cube_demarrer` n'a ni paramètre ni résultat. Le module
/// n'importe toujours rien.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn chaque_fonction_du_cube_a_la_bonne_signature() {
    let module = analyser(&lire_module());
    assert_eq!(module.importations, 0, "le module a {} importation(s)", module.importations);

    for (nom, parametres, resultats) in FONCTIONS {
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

/// Lance `cube-wasm.mjs` avec ces arguments.
fn lancer_le_script(arguments: &[&str]) -> Output {
    Command::new("node")
        .arg(chemin_script("cube-wasm.mjs"))
        .args(arguments)
        .output()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"))
}

/// Lance le script sur le module et rend sa sortie standard, en exigeant qu'il réussisse.
fn sortie_du_module() -> String {
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");
    let sortie = lancer_le_script(&[module]);
    assert!(
        sortie.status.success(),
        "cube-wasm.mjs : {:?}, sortie d'erreur : {}",
        sortie.status,
        String::from_utf8_lossy(&sortie.stderr)
    );
    String::from_utf8(sortie.stdout).expect("la sortie du script est en UTF-8")
}

/// Le cube natif d'une scène : le texte lu sur le disque (jamais par `include_str!`), au temps 0.
fn cube_du_disque(scene: &Scene) -> Cube {
    let chemin = racine().join("scenarios").join(scene.fichier);
    let texte =
        std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", chemin.display()));
    let scenario = lire_scenario_cube(&texte).expect("le scénario du disque est valide");
    Cube::nouveau(&scenario).expect("le scénario du disque se construit")
}

/// Une ligne d'état du cube natif : temps, durée, dimensions, bits de la masse et du maximum, SHA-256 des valeurs
/// petit-boutistes (`z` lent, puis `y`, `x` rapide), nombre de cases bloquées, empreinte.
fn ligne_cube(nom: &str, cube: &Cube) -> String {
    let substrat = cube.substrat();
    let mut octets = Vec::new();
    let mut bloquees = 0u32;
    let mut maximum = 0.0f64;
    for z in 0..substrat.profondeur() {
        for y in 0..substrat.hauteur() {
            for x in 0..substrat.largeur() {
                let valeur = substrat.valeur(x, y, z);
                octets.extend_from_slice(&valeur.to_le_bytes());
                maximum = maximum.max(valeur);
                if substrat.est_bloquee(x, y, z) {
                    bloquees += 1;
                }
            }
        }
    }
    format!(
        "{nom}={},{},{},{},{},{:016x},{:016x},{},{},{}\n",
        cube.temps(),
        cube.duree(),
        substrat.largeur(),
        substrat.hauteur(),
        substrat.profondeur(),
        substrat.masse_totale().to_bits(),
        maximum.to_bits(),
        empreinte::sha256_hex(&octets),
        bloquees,
        cube.empreinte()
    )
}

/// Ce que `cube-wasm.mjs` doit écrire : les lectures d'avant tout démarrage, celles d'après le refus du numéro 2, puis,
/// pour chaque scène, le départ et les morceaux `1, 0, 49, duree - 50`.
fn lignes_attendues() -> String {
    let mut texte = format!("avant={AVANT}\napres_choix_refuse={AVANT}\n");
    for scene in &SCENES {
        let mut cube = cube_du_disque(scene);
        assert_eq!(cube.duree(), scene.duree, "{} : la durée du fichier", scene.nom);
        texte.push_str(&ligne_cube(&format!("scenario_{}_depart", scene.numero), &cube));
        let duree = cube.duree();
        for (i, pas) in [1u32, 0, 49, duree - 50].iter().enumerate() {
            cube.avancer(*pas);
            texte.push_str(&ligne_cube(&format!("scenario_{}_{}", scene.numero, i), &cube));
        }
    }
    texte
}

/// 2. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne pour **chacune des deux
///    scènes** les mêmes lignes que `Cube` construit sur le texte du disque : les douze lectures d'un module neuf
///    (`0` ou `NaN`), le refus de `choisir(2)` sans changement, puis le départ et chaque morceau `1, 0, 49,
///    duree - 50` (temps, durée, dimensions, bits de la masse et du maximum, SHA-256 de toutes les valeurs, nombre de
///    cases bloquées et empreinte). Le script vérifie aussi, à chaque état, les bords du volume sur le module. Si le
///    code change sans que le module soit reconstruit, ce test échoue.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn node_donne_les_memes_lignes_que_le_cube_natif_pour_chaque_scene() {
    if !node_est_la() {
        return;
    }
    assert_eq!(sortie_du_module(), lignes_attendues(), "le module et le cube natif de chaque scène");
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

/// 3. À la fin de chaque scène, l'empreinte lue dans le module (la dernière ligne `scenario_<n>_<i>`) est celle du
///    fichier de `references/` de la scène.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn l_empreinte_de_la_fin_de_chaque_scene_est_celle_des_references() {
    if !node_est_la() {
        return;
    }
    let sortie = sortie_du_module();
    for scene in &SCENES {
        let prefixe = format!("scenario_{}_", scene.numero);
        let ligne = sortie
            .lines()
            .rev()
            .find(|ligne| ligne.starts_with(&prefixe))
            .unwrap_or_else(|| panic!("{} : aucune ligne d'état", scene.nom));
        let empreinte = ligne.rsplit(',').next().expect("une ligne d'état a une empreinte");
        assert_eq!(empreinte.len(), 64, "{} : l'empreinte fait 64 chiffres", scene.nom);
        assert_eq!(empreinte, empreinte_de_reference(scene.reference), "{} : l'empreinte de la fin", scene.nom);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Le cube natif de `vitrine_cube.rs`, sous le verrou de ce fichier : l'état global est unique au processus, et aucun
// autre test de ce fichier ne le touche.
// ---------------------------------------------------------------------------------------------------------------

/// Sérialise les tests de ce fichier qui emploient l'état global de `vitrine_cube`.
static VERROU: Mutex<()> = Mutex::new(());

/// Prend le verrou ; un test qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// Les douze lectures d'avant tout démarrage, par les fonctions natives de `vitrine_cube`.
fn etat_natif_avant() -> String {
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{}",
        mycelium_cube_temps(),
        mycelium_cube_duree(),
        mycelium_cube_dimension(0),
        mycelium_cube_dimension(1),
        mycelium_cube_dimension(2),
        mycelium_cube_dimension(3),
        mycelium_cube_valeur(0, 0, 0),
        mycelium_cube_bloquee(0, 0, 0),
        mycelium_cube_maximum(),
        mycelium_cube_masse(),
        mycelium_cube_empreinte_mot(0),
        mycelium_cube_empreinte_mot(8)
    )
}

/// Vérifie, par les fonctions natives, qu'une case hors du volume rend `NaN` pour `valeur` et 0 pour `bloquee`, et
/// que `dimension` et `empreinte_mot` rendent 0 au-delà de leur portée.
fn verifier_bords_natifs(nom: &str, largeur: u32, hauteur: u32, profondeur: u32) {
    let dehors = [
        (largeur, 0, 0),
        (0, hauteur, 0),
        (0, 0, profondeur),
        (largeur, hauteur, profondeur),
        (u32::MAX, u32::MAX, u32::MAX),
    ];
    for (x, y, z) in dehors {
        assert!(mycelium_cube_valeur(x, y, z).is_nan(), "{nom} : valeur({x}, {y}, {z}) hors du volume");
        assert_eq!(mycelium_cube_bloquee(x, y, z), 0, "{nom} : bloquee({x}, {y}, {z}) hors du volume");
    }
    assert_eq!(mycelium_cube_dimension(3), 0, "{nom} : dimension(3)");
    assert_eq!(mycelium_cube_dimension(u32::MAX), 0, "{nom} : dimension(u32::MAX)");
    assert_eq!(mycelium_cube_empreinte_mot(8), 0, "{nom} : empreinte_mot(8)");
    assert_eq!(mycelium_cube_empreinte_mot(u32::MAX), 0, "{nom} : empreinte_mot(u32::MAX)");
}

/// Une ligne d'état lue par les fonctions natives de `vitrine_cube`, dans le même format que `ligne_cube`.
fn ligne_native(nom: &str) -> String {
    let (largeur, hauteur, profondeur) =
        (mycelium_cube_dimension(0), mycelium_cube_dimension(1), mycelium_cube_dimension(2));
    let mut octets = Vec::new();
    let mut bloquees = 0u32;
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                octets.extend_from_slice(&mycelium_cube_valeur(x, y, z).to_le_bytes());
                bloquees += mycelium_cube_bloquee(x, y, z);
            }
        }
    }
    verifier_bords_natifs(nom, largeur, hauteur, profondeur);
    let mut empreinte = String::new();
    for i in 0..8 {
        empreinte.push_str(&format!("{:08x}", mycelium_cube_empreinte_mot(i)));
    }
    format!(
        "{nom}={},{},{},{},{},{:016x},{:016x},{},{},{}\n",
        mycelium_cube_temps(),
        mycelium_cube_duree(),
        largeur,
        hauteur,
        profondeur,
        mycelium_cube_masse().to_bits(),
        mycelium_cube_maximum().to_bits(),
        empreinte::sha256_hex(&octets),
        bloquees,
        empreinte
    )
}

/// Les lignes que rendent les fonctions natives de `vitrine_cube` : l'état neuf, le refus de `choisir(2)`, puis les
/// deux scènes avec les mêmes morceaux que le script.
fn sequence_native() -> String {
    let avant = etat_natif_avant();
    let mut texte = format!("avant={avant}\n");
    assert_eq!(mycelium_cube_choisir(2), 0, "le cube natif accepte le numéro de scène 2");
    let apres = etat_natif_avant();
    assert_eq!(apres, avant, "choisir(2) a changé l'état alors qu'il devait rendre 0 sans rien changer");
    texte.push_str(&format!("apres_choix_refuse={apres}\n"));

    for scene in &SCENES {
        assert_eq!(mycelium_cube_choisir(scene.numero), 1, "{} : le cube natif refuse la scène", scene.nom);
        mycelium_cube_demarrer();
        texte.push_str(&ligne_native(&format!("scenario_{}_depart", scene.numero)));
        let duree = mycelium_cube_duree();
        assert_eq!(duree, scene.duree, "{} : la durée du cube démarré", scene.nom);
        for (i, pas) in [1u32, 0, 49, duree - 50].iter().enumerate() {
            mycelium_cube_avancer(*pas);
            texte.push_str(&ligne_native(&format!("scenario_{}_{}", scene.numero, i)));
        }
    }
    texte
}

/// 4. Le cube natif de `vitrine_cube.rs` rend les mêmes lignes que `Cube` (par `src/cube.rs`), et hors du volume
///    `valeur` rend `NaN` et `bloquee` 0, avant comme après un pas : l'état neuf rend 0 ou `NaN`, `choisir(2)` rend 0
///    sans rien changer, et les bords sont ceux du script. Le verrou rend fiable le « avant tout démarrage » : aucun
///    autre test de ce fichier ne touche à l'état global.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn le_cube_natif_donne_les_memes_lignes_et_rend_nan_hors_du_volume() {
    let _verrou = verrou();
    assert_eq!(sequence_native(), lignes_attendues(), "le cube natif de vitrine_cube et Cube");

    // Les bords, une dernière fois, sur le dernier état de la dernière scène.
    let (largeur, hauteur, profondeur) =
        (mycelium_cube_dimension(0), mycelium_cube_dimension(1), mycelium_cube_dimension(2));
    assert!(largeur > 0 && hauteur > 0 && profondeur > 0, "un cube démarré a des dimensions non nulles");
    verifier_bords_natifs("fin", largeur, hauteur, profondeur);
    assert!(!mycelium_cube_valeur(0, 0, 0).is_nan(), "une case du volume ne rend pas NaN");
}
