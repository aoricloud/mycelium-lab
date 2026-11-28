//! Tests d'acceptation de l'étape 25 (c) : le module `docs/mycelium.wasm` reconstruit et les scènes 2, 3 et 4 de la
//! colonie, lues par Node.
//!
//! Comme aux étapes 4, 8, 11, 16, 18 et 21, la CI n'a pas la cible WebAssembly : le module est construit à la main
//! puis commité, et la preuve qu'il est bon est qu'il tourne, sous Node, et donne la même chose que le code natif.
//! Ici, pour les scènes de l'étape 25 : `tests/etape_25/scenes-wasm.mjs` lit les quatre fonctions nouvelles avant
//! tout démarrage, puis, après chaque morceau de pas, l'état des scènes 2 (le chimiotropisme), 3 (le mur) et 4
//! (l'anastomose) : temps, durée, comptes, SHA-256 des segments, obstacles, SHA-256 des soudures et empreinte. Ce test
//! le compare, chaîne pour chaîne, à ce que donne `Colonie3d` construit **sans passer par `vitrine_colonie3d.rs`**
//! (le texte de la scène est lu sur le disque).
//!
//! 1. la forme du module : les quatre exportations nouvelles et leurs signatures (`u32` en `i32`, `f64` en `f64`) ;
//! 2. le résultat du module, lu par Node : les quatre lectures d'avant tout démarrage, le refus de `choisir(9)` et de
//!    `choisir(4294967295)`, puis, pour chaque scène (2, 3 et 4), le départ et les morceaux `1, 0, 9`, le reste de la
//!    durée, et l'état de chacun ;
//! 3. à la fin de chaque scène, l'empreinte lue dans le module est celle de `references/`, la scène 4 finit avec 86
//!    anastomoses et 86 soudures, la scène 3 a les quatre obstacles de `scenarios/colonie-mur.txt`, dans l'ordre du
//!    fichier, et les scènes 2 et 3 n'ont aucune soudure ;
//! 4. les fonctions natives de `vitrine_colonie3d.rs` rendent les mêmes lignes que `Colonie3d`, sous le verrou de ce
//!    fichier (l'état global est unique au processus), avec les mêmes bords, et avancer d'un coup toute la durée donne
//!    les mêmes soudures que par morceaux.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_le_bytes`, `is_nan` et `from`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, lire_scenario_colonie3d};
use mycelium_lab::empreinte;
use mycelium_lab::vitrine_colonie3d::{
    mycelium_colonie_anastomoses, mycelium_colonie_avancer, mycelium_colonie_choisir, mycelium_colonie_demarrer,
    mycelium_colonie_duree, mycelium_colonie_empreinte_mot, mycelium_colonie_nb_obstacles,
    mycelium_colonie_nb_segments, mycelium_colonie_obstacle, mycelium_colonie_pointes, mycelium_colonie_ramifications,
    mycelium_colonie_segment, mycelium_colonie_soudure, mycelium_colonie_temps,
};

/// Une des trois scènes de l'étape 25, telle que son fichier de `scenarios/` la fixe.
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
}

/// Les trois scènes, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas du code.
const SCENES: [Scene; 3] = [
    Scene { numero: 2, nom: "chimio", fichier: "colonie-chimio.txt", reference: "colonie-chimio.empreinte", duree: 85 },
    Scene { numero: 3, nom: "mur", fichier: "colonie-mur.txt", reference: "colonie-mur.empreinte", duree: 100 },
    Scene {
        numero: 4,
        nom: "anastomose",
        fichier: "colonie-anastomose.txt",
        reference: "colonie-anastomose.empreinte",
        duree: 50,
    },
];

/// La graine des empreintes de référence.
const GRAINE: u32 = 1;

/// Les deux numéros que `mycelium_colonie_choisir` doit refuser maintenant que les scènes vont de 0 à 8.
const NUMEROS_REFUSES: [u32; 2] = [9, u32::MAX];

/// Les types de valeurs WebAssembly qui servent ici, tels que le format binaire les code.
const I32: u8 = 0x7F;
const F64: u8 = 0x7C;

/// Les quatre fonctions nouvelles que le module doit exporter, avec leur signature : type des paramètres, type du
/// résultat (`u32` est un `i32` en WebAssembly, `f64` reste un `f64`).
const FONCTIONS: [(&str, &[u8], &[u8]); 4] = [
    ("mycelium_colonie_nb_obstacles", &[], &[I32]),
    ("mycelium_colonie_obstacle", &[I32, I32], &[F64]),
    ("mycelium_colonie_anastomoses", &[], &[I32]),
    ("mycelium_colonie_soudure", &[I32, I32], &[F64]),
];

/// Les quatre lectures d'avant tout démarrage : les entiers rendent 0, les flottants `NaN`.
const AVANT: &str = "0,NaN,0,NaN";

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn chemin_script() -> PathBuf {
    racine().join("tests").join("etape_25").join("scenes-wasm.mjs")
}

fn lire_module() -> Vec<u8> {
    std::fs::read(chemin_module()).unwrap_or_else(|erreur| panic!("docs/mycelium.wasm se lit : {erreur}"))
}

// ---------------------------------------------------------------------------------------------------------------
// Un petit lecteur du format binaire WebAssembly : juste ce qu'il faut pour les sections de types, d'importations,
// de fonctions et d'exportations (le même que celui de `etape_21_artefact.rs`).
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

/// 1. Le module exporte les quatre fonctions nouvelles ; toute autre fonction exportée porte un nom qui commence par
///    `mycelium`, un nom n'est exporté qu'une fois, et la mémoire `memory` est la seule exportation qui ne soit pas
///    une fonction. Le module n'importe toujours rien.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn le_module_exporte_les_quatre_fonctions_nouvelles_avec_leurs_signatures() {
    let module = analyser(&lire_module());
    assert_eq!(module.importations, 0, "le module a {} importation(s)", module.importations);
    let attendues: BTreeSet<&str> = FONCTIONS.iter().map(|(nom, _, _)| *nom).collect();
    assert_eq!(attendues.len(), 4);
    verifier_les_exportations(&module, &attendues);

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

/// Le scénario d'une scène, lu sur le disque (jamais par `include_str!`).
fn scenario_du_disque(scene: &Scene) -> ScenarioColonie3d {
    let chemin = racine().join("scenarios").join(scene.fichier);
    let texte =
        std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", chemin.display()));
    lire_scenario_colonie3d(&texte).expect("le scénario du disque est valide")
}

/// La colonie d'une scène, à la graine `GRAINE`, au temps 0.
fn colonie_du_disque(scene: &Scene) -> Colonie3d {
    let scenario = scenario_du_disque(scene);
    Colonie3d::nouvelle(&scenario, u64::from(GRAINE)).expect("le scénario du disque se construit")
}

/// Les obstacles du scénario de la colonie : le nombre, puis `:x0:y0:z0:x1:y1:z1` pour chacun, dans l'ordre du texte.
fn obstacles_du_scenario(colonie: &Colonie3d) -> String {
    let obstacles = &colonie.scenario().obstacles;
    let mut texte = obstacles.len().to_string();
    for obstacle in obstacles {
        texte.push_str(&format!(
            ":{}:{}:{}:{}:{}:{}",
            obstacle.x0, obstacle.y0, obstacle.z0, obstacle.x1, obstacle.y1, obstacle.z1
        ));
    }
    texte
}

/// Le SHA-256 des f64 petit-boutistes des six coordonnées de chaque segment, dans l'ordre.
fn hacher_segments(colonie: &Colonie3d) -> String {
    let mut octets = Vec::new();
    for segment in colonie.segments() {
        for coordonnee in [segment.x0, segment.y0, segment.z0, segment.x1, segment.y1, segment.z1] {
            octets.extend_from_slice(&coordonnee.to_le_bytes());
        }
    }
    empreinte::sha256_hex(&octets)
}

/// Le SHA-256 des f64 petit-boutistes des trois coordonnées de chaque soudure, dans l'ordre.
fn hacher_soudures(soudures: &[(f64, f64, f64)]) -> String {
    let mut octets = Vec::new();
    for (x, y, z) in soudures {
        for coordonnee in [*x, *y, *z] {
            octets.extend_from_slice(&coordonnee.to_le_bytes());
        }
    }
    empreinte::sha256_hex(&octets)
}

/// Une ligne d'état native : temps, durée, comptes, SHA-256 des segments, obstacles, SHA-256 des soudures, empreinte.
fn ligne_etat(nom: &str, colonie: &Colonie3d, soudures: &[(f64, f64, f64)]) -> String {
    format!(
        "{nom}={},{},{},{},{},{},{},{},{},{}\n",
        colonie.temps(),
        colonie.duree(),
        colonie.segments().len(),
        colonie.pointes(),
        colonie.nb_ramifications(),
        colonie.nb_anastomoses(),
        hacher_segments(colonie),
        obstacles_du_scenario(colonie),
        hacher_soudures(soudures),
        colonie.empreinte()
    )
}

/// Joue une scène pas à pas, à la graine `GRAINE`, et rend ses lignes d'état (le départ puis un par morceau) et la
/// liste de ses soudures.
///
/// Après chaque pas, chaque segment laissé pendant ce pas dont l'apex est inactif à la fin du pas donne une soudure à
/// son arrivée `(x1, y1, z1)` ; le test vérifie à chaque pas que leur nombre égale la hausse de `nb_anastomoses()`.
fn jouer_scene(scene: &Scene) -> (Vec<String>, Vec<(f64, f64, f64)>) {
    let mut colonie = colonie_du_disque(scene);
    assert_eq!(colonie.duree(), scene.duree, "{} : la durée du fichier", scene.nom);
    let mut soudures: Vec<(f64, f64, f64)> = Vec::new();
    let mut lignes = vec![ligne_etat(&format!("scenario_{}_depart", scene.numero), &colonie, &soudures)];
    let duree = colonie.duree();
    for (i, pas) in [1u32, 0, 9, duree - 10].iter().enumerate() {
        for _ in 0..*pas {
            let avant = colonie.segments().len();
            let compteur = colonie.nb_anastomoses();
            colonie.avancer();
            let temps = colonie.temps();
            let mut nouvelles = 0u64;
            for segment in &colonie.segments()[avant..] {
                let inactif = colonie.apex().get(segment.apex as usize).is_some_and(|apex| !apex.actif);
                if inactif {
                    soudures.push((segment.x1, segment.y1, segment.z1));
                    nouvelles += 1;
                }
            }
            assert_eq!(
                colonie.nb_anastomoses() - compteur,
                nouvelles,
                "{} : pas {temps}, le compteur croît d'autant que de soudures",
                scene.nom
            );
        }
        lignes.push(ligne_etat(&format!("scenario_{}_{}", scene.numero, i), &colonie, &soudures));
    }
    (lignes, soudures)
}

/// Ce que `scenes-wasm.mjs` doit écrire : les quatre lectures d'avant tout démarrage, puis, pour chaque scène, le
/// départ et les morceaux `1, 0, 9, duree - 10`.
fn lignes_attendues() -> String {
    let mut texte = format!("avant={AVANT}\n");
    for scene in &SCENES {
        let (lignes, _) = jouer_scene(scene);
        for ligne in lignes {
            texte.push_str(&ligne);
        }
    }
    texte
}

/// 2. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne pour **chacune des scènes 2,
///    3 et 4** les mêmes lignes que `Colonie3d` construit sur le texte du disque et joué un pas à la fois : les
///    quatre lectures d'un module neuf (`0` ou `NaN`), le refus de `choisir(9)` et de `choisir(4294967295)` sans
///    changement, puis le départ et chaque morceau `1, 0, 9, duree - 10` (temps, durée, comptes, SHA-256 des
///    segments, obstacles, SHA-256 des soudures et empreinte). Le script vérifie aussi, à chaque état, les bords des
///    obstacles et des soudures. Si le code change sans que le module soit reconstruit, ce test échoue.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn node_donne_les_memes_lignes_que_la_colonie_native_pour_les_scenes_2_3_et_4() {
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

/// 3. À la fin de chaque scène, l'empreinte lue dans le module est celle du fichier de `references/` de la scène, et le
///    nombre d'anastomoses et le SHA-256 des soudures sont ceux du natif ; la scène 4 finit avec 86 anastomoses et 86
///    soudures ; la scène 3 a les quatre obstacles de `scenarios/colonie-mur.txt`, dans l'ordre du fichier ; les
///    scènes 2 et 3 n'ont aucune soudure.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn la_fin_de_chaque_scene_a_son_empreinte_ses_obstacles_et_ses_soudures() {
    if !node_est_la() {
        return;
    }
    let sortie = sortie_du_module();

    for scene in &SCENES {
        let ligne = derniere_ligne(&sortie, scene);
        let valeurs = champs(ligne);
        assert_eq!(valeurs.len(), 10, "{} : une ligne d'état a dix champs", scene.nom);
        assert_eq!(valeurs[9], empreinte_de_reference(scene.reference), "{} : l'empreinte de la fin", scene.nom);

        let (_, soudures) = jouer_scene(scene);
        assert_eq!(valeurs[5], soudures.len().to_string(), "{} : le nombre d'anastomoses de la fin", scene.nom);
        assert_eq!(valeurs[8], hacher_soudures(&soudures), "{} : le SHA-256 des soudures de la fin", scene.nom);
    }

    // La scène 3 a les quatre obstacles de `scenarios/colonie-mur.txt`, dans l'ordre du fichier.
    let scenario_mur = scenario_du_disque(&SCENES[1]);
    let blocs: Vec<(u32, u32, u32, u32, u32, u32)> =
        scenario_mur.obstacles.iter().map(|o| (o.x0, o.y0, o.z0, o.x1, o.y1, o.z1)).collect();
    assert_eq!(blocs.len(), 4, "colonie-mur : quatre obstacles");
    assert_eq!(
        blocs,
        vec![(16, 0, 0, 17, 13, 31), (16, 18, 0, 17, 31, 31), (16, 14, 0, 17, 17, 13), (16, 14, 18, 17, 17, 31)],
        "colonie-mur : les quatre obstacles, dans l'ordre du fichier"
    );
    assert_eq!(
        champs(derniere_ligne(&sortie, &SCENES[1]))[7],
        "4:16:0:0:17:13:31:16:18:0:17:31:31:16:14:0:17:17:13:16:14:18:17:17:31",
        "colonie-mur : les obstacles lus dans le module, dans l'ordre du fichier"
    );

    // La scène 4 finit avec 86 anastomoses et 86 soudures.
    let (_, soudures_anastomose) = jouer_scene(&SCENES[2]);
    assert_eq!(soudures_anastomose.len(), 86, "colonie-anastomose : 86 soudures");
    assert_eq!(champs(derniere_ligne(&sortie, &SCENES[2]))[5], "86", "colonie-anastomose : 86 anastomoses");

    // Les scènes 2 et 3 n'ont aucune soudure.
    for scene in &SCENES[..2] {
        let (_, soudures) = jouer_scene(scene);
        assert!(soudures.is_empty(), "{} : aucune soudure", scene.nom);
        let valeurs = champs(derniere_ligne(&sortie, scene));
        assert_eq!(valeurs[5], "0", "{} : aucune anastomose", scene.nom);
        assert_eq!(valeurs[8], empreinte::sha256_hex(&[]), "{} : le SHA-256 d'aucune soudure", scene.nom);
    }
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

/// Les quatre lectures d'avant tout démarrage, par les fonctions natives de `vitrine_colonie3d`.
fn etat_natif_avant() -> String {
    format!(
        "{},{},{},{}",
        mycelium_colonie_nb_obstacles(),
        mycelium_colonie_obstacle(0, 0),
        mycelium_colonie_anastomoses(),
        mycelium_colonie_soudure(0, 0)
    )
}

/// Les huit mots de l'empreinte de la colonie en cours, en 64 chiffres hexadécimaux minuscules.
fn empreinte_native() -> String {
    let mut empreinte = String::new();
    for i in 0..8 {
        empreinte.push_str(&format!("{:08x}", mycelium_colonie_empreinte_mot(i)));
    }
    empreinte
}

/// Les obstacles de la colonie en cours, lus par les fonctions natives : le nombre, puis `:x0:y0:z0:x1:y1:z1`.
fn obstacles_natifs() -> String {
    let n = mycelium_colonie_nb_obstacles();
    let mut texte = n.to_string();
    for i in 0..n {
        for k in 0..6 {
            texte.push_str(&format!(":{}", mycelium_colonie_obstacle(i, k) as u32));
        }
    }
    texte
}

/// Les soudures de la colonie en cours, lues par les fonctions natives, dans l'ordre.
fn soudures_natives() -> Vec<(f64, f64, f64)> {
    let n = mycelium_colonie_anastomoses();
    (0..n)
        .map(|i| (mycelium_colonie_soudure(i, 0), mycelium_colonie_soudure(i, 1), mycelium_colonie_soudure(i, 2)))
        .collect()
}

/// Le SHA-256 des f64 petit-boutistes des trois coordonnées de chaque soudure, lu par les fonctions natives.
fn hacher_soudures_natives() -> String {
    hacher_soudures(&soudures_natives())
}

/// Vérifie, par les fonctions natives, qu'un obstacle au-delà du dernier (ou un `k > 5`) rend `NaN` pour `obstacle`,
/// qu'une soudure au-delà de la dernière (ou un `k > 2`) rend `NaN` pour `soudure`.
fn verifier_bords_natifs(nom: &str) {
    let nb_obstacles = mycelium_colonie_nb_obstacles();
    let nb_anastomoses = mycelium_colonie_anastomoses();
    assert!(
        mycelium_colonie_obstacle(nb_obstacles, 0).is_nan(),
        "{nom} : obstacle({nb_obstacles}, 0) au-delà du dernier obstacle"
    );
    assert!(
        mycelium_colonie_soudure(nb_anastomoses, 0).is_nan(),
        "{nom} : soudure({nb_anastomoses}, 0) au-delà de la dernière soudure"
    );
    if nb_obstacles > 0 {
        assert!(mycelium_colonie_obstacle(0, 6).is_nan(), "{nom} : obstacle(0, 6) pour un k hors de portée");
    }
    if nb_anastomoses > 0 {
        assert!(mycelium_colonie_soudure(0, 3).is_nan(), "{nom} : soudure(0, 3) pour un k hors de portée");
    }
}

/// Une ligne d'état lue par les fonctions natives de `vitrine_colonie3d`, dans le même format que `ligne_etat`.
fn ligne_native(nom: &str) -> String {
    let nb_segments = mycelium_colonie_nb_segments();
    let mut coordonnees = Vec::new();
    for i in 0..nb_segments {
        for k in 0..6 {
            coordonnees.extend_from_slice(&mycelium_colonie_segment(i, k).to_le_bytes());
        }
    }
    verifier_bords_natifs(nom);
    format!(
        "{nom}={},{},{},{},{},{},{},{},{},{}\n",
        mycelium_colonie_temps(),
        mycelium_colonie_duree(),
        nb_segments,
        mycelium_colonie_pointes(),
        mycelium_colonie_ramifications(),
        mycelium_colonie_anastomoses(),
        empreinte::sha256_hex(&coordonnees),
        obstacles_natifs(),
        hacher_soudures_natives(),
        empreinte_native()
    )
}

/// Les lignes que rendent les fonctions natives de `vitrine_colonie3d` : l'état neuf, les deux numéros refusés, les
/// trois scènes avec les mêmes morceaux que le script.
fn sequence_native() -> String {
    let avant = etat_natif_avant();
    let mut texte = format!("avant={avant}\n");
    for numero in NUMEROS_REFUSES {
        assert_eq!(mycelium_colonie_choisir(numero), 0, "la colonie native accepte le numéro de scène {numero}");
        let apres = etat_natif_avant();
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

/// 4. La colonie native de `vitrine_colonie3d.rs` rend les mêmes lignes que `Colonie3d` (par `src/colonie3d.rs`), avec
///    les mêmes bords avant comme après un pas : l'état neuf rend `0` ou `NaN`, `choisir(9)` et `choisir(4294967295)`
///    rendent 0 sans rien changer, et les bords des obstacles et des soudures sont ceux du script. Avancer d'un coup
///    toute la durée donne les mêmes soudures que par morceaux. Le verrou rend fiable le « avant tout démarrage » :
///    aucun autre test de ce fichier ne touche à l'état global.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn la_vitrine_native_donne_les_memes_lignes_les_memes_bords_et_les_memes_soudures() {
    let _verrou = verrou();
    assert_eq!(sequence_native(), lignes_attendues(), "la colonie native de vitrine_colonie3d et Colonie3d");

    // Avancer d'un coup toute la durée donne les mêmes soudures que par morceaux.
    for scene in &SCENES {
        let (_, par_morceaux) = jouer_scene(scene);
        assert_eq!(mycelium_colonie_choisir(scene.numero), 1, "{} : la colonie native refuse la scène", scene.nom);
        mycelium_colonie_demarrer(GRAINE);
        mycelium_colonie_avancer(scene.duree);
        assert_eq!(mycelium_colonie_temps(), scene.duree, "{} : le temps est la durée", scene.nom);
        assert_eq!(
            mycelium_colonie_anastomoses() as usize,
            par_morceaux.len(),
            "{} : le nombre d'anastomoses d'un coup",
            scene.nom
        );
        assert_eq!(soudures_natives(), par_morceaux, "{} : les soudures d'un coup et par morceaux", scene.nom);
        verifier_bords_natifs(scene.nom);
    }
}
