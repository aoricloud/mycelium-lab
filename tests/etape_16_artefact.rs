//! Tests d'acceptation de l'étape 16 (c) : le module `docs/mycelium.wasm` reconstruit (21 exportations) et les trois lectures
//! de la page, lues par Node.
//!
//! Comme aux étapes 4, 8 et 11, la CI n'a pas la cible WebAssembly : le module est construit à la main puis commité, et la
//! preuve qu'il est bon est qu'il tourne, sous Node, et donne la même chose que le code natif. Ici, pour les trois lectures
//! de l'étape 16 : `tests/etape_16/pointes-wasm.mjs` lit, avant tout démarrage puis après des pas, le pas de naissance et
//! l'hyphe de chaque segment et la position de chaque pointe vivante, et ce test les compare au bit près (par leur SHA-256)
//! à ce que donne `Simulation` construite **sans passer par `vitrine.rs`** (le texte du scénario est lu sur le disque).
//!
//! 1. la forme du module : ses 21 exportations et leurs signatures (les 18 d'avant gardent la leur) ;
//! 2. le résultat du module, lu par Node : ce que rend un module neuf avant tout démarrage, puis, pour chaque scénario et
//!    plusieurs graines et découpages des pas, les trois lectures, l'empreinte lue **après** elles (lire ne change rien) et
//!    les bords ;
//! 3. le script refuse des arguments faux.
//!
//! Le test de l'étape 11 qui fixait « exactement les 18 fonctions » a sa version pour 21 ici, et n'est plus compilé avec
//! `etape-16` (`etape_11_artefact.rs`). Que les 18 d'avant rendent ce qu'elles rendaient, c'est ce que disent, sans une ligne
//! changée, les tests des étapes 4, 8 et 11 sur ce même module.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_le_bytes`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mycelium_lab::empreinte;
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

/// Un des six scénarios de la vitrine, tel que la spécification de l'étape 8 le fixe.
struct Scenario {
    /// Le numéro que prend `mycelium_choisir_scenario`.
    numero: u32,
    /// Le nom dans l'adresse de la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// Le pas de référence (`duree` du fichier).
    duree: u32,
}

/// Les six scénarios, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas du code.
const SCENARIOS: [Scenario; 6] = [
    Scenario { numero: 0, nom: "libre", fichier: "couplage.txt", duree: 80 },
    Scenario { numero: 1, nom: "terrain", fichier: "terrain.txt", duree: 80 },
    Scenario { numero: 2, nom: "tache", fichier: "chimiotropisme.txt", duree: 100 },
    Scenario { numero: 3, nom: "tache-sans", fichier: "chimiotropisme-sans.txt", duree: 100 },
    Scenario { numero: 4, nom: "mur", fichier: "mur.txt", duree: 120 },
    Scenario { numero: 5, nom: "mur-sans", fichier: "mur-sans.txt", duree: 120 },
];

/// Les types de valeurs WebAssembly qui servent ici, tels que le format binaire les code.
const I32: u8 = 0x7F;
const F64: u8 = 0x7C;

/// Les fonctions que le module doit exporter, avec leur signature : types des paramètres, types des résultats
/// (`u32` est un `i32` en WebAssembly). Les dix-huit premières sont celles des étapes 4, 8 et 11, que rien ne change ; les
/// trois dernières sont celles de l'étape 16 : deux entiers rendus en `f64` (`NaN` pour « hors de portée ») et une position.
const FONCTIONS: [(&str, &[u8], &[u8]); 21] = [
    ("mycelium_demarrer", &[I32], &[]),
    ("mycelium_avancer", &[I32], &[]),
    ("mycelium_temps", &[], &[I32]),
    ("mycelium_largeur", &[], &[I32]),
    ("mycelium_hauteur", &[], &[I32]),
    ("mycelium_nb_segments", &[], &[I32]),
    ("mycelium_segment", &[I32, I32], &[F64]),
    ("mycelium_substrat", &[I32, I32], &[F64]),
    ("mycelium_biomasse", &[], &[F64]),
    ("mycelium_pointes", &[], &[I32]),
    ("mycelium_anastomoses", &[], &[I32]),
    ("mycelium_empreinte_mot", &[I32], &[I32]),
    ("mycelium_choisir_scenario", &[I32], &[I32]),
    ("mycelium_duree", &[], &[I32]),
    ("mycelium_nb_obstacles", &[], &[I32]),
    ("mycelium_obstacle", &[I32, I32], &[F64]),
    ("mycelium_densite", &[I32, I32], &[F64]),
    ("mycelium_densite_max", &[], &[F64]),
    ("mycelium_segment_temps", &[I32], &[F64]),
    ("mycelium_segment_apex", &[I32], &[F64]),
    ("mycelium_pointe", &[I32, I32], &[F64]),
];

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn chemin_script(nom: &str) -> PathBuf {
    racine().join("tests").join("etape_16").join(nom)
}

fn lire_module() -> Vec<u8> {
    std::fs::read(chemin_module()).unwrap_or_else(|erreur| panic!("docs/mycelium.wasm se lit : {erreur}"))
}

// ---------------------------------------------------------------------------------------------------------------
// Un petit lecteur du format binaire WebAssembly : juste ce qu'il faut pour les sections de types, d'importations,
// de fonctions et d'exportations (le même que celui de `etape_8_artefact.rs`).
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

/// 1. Le module exporte exactement les vingt et une fonctions de la liste : les dix-huit des étapes 4, 8 et 11 et
///    `mycelium_segment_temps`, `mycelium_segment_apex` et `mycelium_pointe` ; la mémoire `memory` est tolérée, rien d'autre
///    (aucune autre exportation, de quelque genre que ce soit). Un nom n'est exporté qu'une fois. Un module construit avant
///    l'étape 16 (18 exportations) ne passe pas.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn le_module_exporte_exactement_les_vingt_et_une_fonctions() {
    let module = analyser(&lire_module());

    let fonctions: BTreeSet<&str> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.as_str()).collect();
    let attendues: BTreeSet<&str> = FONCTIONS.iter().map(|(nom, _, _)| *nom).collect();
    assert_eq!(attendues.len(), 21);
    assert_eq!(fonctions, attendues, "les fonctions exportées ne sont pas celles de la liste");

    let autres: Vec<(&str, u8)> =
        module.exportations.iter().filter(|e| e.genre != 0).map(|e| (e.nom.as_str(), e.genre)).collect();
    for (nom, genre) in &autres {
        assert!((*nom, *genre) == ("memory", 2), "exportation inattendue : « {nom} » (genre {genre})");
    }
    assert_eq!(module.exportations.len(), fonctions.len() + autres.len());
    assert_eq!(
        module.exportations.iter().map(|e| e.nom.as_str()).collect::<BTreeSet<_>>().len(),
        module.exportations.len(),
        "un nom est exporté deux fois"
    );
}

/// 1 (suite). Le module et la fonctionnalité `etape-16` vont ensemble : le module exporte les trois lectures si et
/// seulement si `etape-16` est active, et `etape-16` se déclare après `etape-15`. **Ce test n'est jamais ignoré** :
/// c'est lui qui voit, une fois le module reconstruit, `etape-16` oubliée de `default` (les autres tests de l'étape
/// seraient alors ignorés, et la suite verte).
#[test]
fn le_module_a_les_trois_lectures_si_et_seulement_si_etape_16_est_active() {
    let cargo = std::fs::read_to_string(racine().join("Cargo.toml")).expect("Cargo.toml se lit");
    assert!(cargo.contains("\netape-15 = []\netape-16 = []\n"), "etape-16 se déclare après etape-15");
    let module = analyser(&lire_module());
    let lectures = FONCTIONS[18..].iter().filter(|(nom, _, _)| module.exportations.iter().any(|e| e.nom == *nom));
    let attendu = if cfg!(feature = "etape-16") { 3 } else { 0 };
    assert_eq!(lectures.count(), attendu, "trois lectures exportées sans etape-16, ou etape-16 sans elles");
}

/// 1 (suite). Chaque fonction exportée a la bonne signature : le nombre et le type des paramètres, et du résultat (`i32` pour
/// un `u32`, `f64` pour un `f64`). Les dix-huit d'avant gardent la leur, et les trois nouvelles sont
/// `segment_temps(i32) -> f64`, `segment_apex(i32) -> f64` et `pointe(i32, i32) -> f64`. Le module n'importe toujours rien.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn chaque_fonction_a_la_bonne_signature() {
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

/// Lance ce script avec ces arguments.
fn lancer_le_script(script: &str, arguments: &[&str]) -> Output {
    Command::new("node")
        .arg(chemin_script(script))
        .args(arguments)
        .output()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"))
}

/// La simulation native en face : le scénario lu sur le disque (jamais par `include_str!`), cette graine, au temps 0.
fn simulation_native(scenario: &Scenario, graine: u32) -> Simulation {
    let chemin = racine().join("scenarios").join(scenario.fichier);
    let texte =
        std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", chemin.display()));
    let (reglages, milieu, tropismes, _duree) =
        lire_scenario_tropismes(&texte).expect("le scénario du disque est valide");
    Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages sont valides")
}

/// Le SHA-256 en hexadécimal d'une suite de `f64`, petit-boutistes, mis bout à bout : ce que hache `pointes-wasm.mjs`.
fn hache(valeurs: impl Iterator<Item = f64>) -> String {
    let octets: Vec<u8> = valeurs.flat_map(f64::to_le_bytes).collect();
    empreinte::sha256_hex(&octets)
}

/// Les lignes `lecture_<nom>` et `bords_<nom>` que `pointes-wasm.mjs` doit écrire pour cet état de la simulation native :
/// le temps, le nombre de segments et de pointes vivantes, le SHA-256 des pas de naissance, des hyphes et des positions des
/// pointes (`x` puis `y`, dans l'ordre des identifiants), l'empreinte de l'état, et 1 pour les bords.
fn lignes_attendues(nom: &str, simulation: &Simulation) -> String {
    let segments = simulation.segments();
    let vivantes: Vec<_> = simulation.etat_des_pointes().filter(|pointe| pointe.actif).collect();
    format!(
        "lecture_{nom}={},{},{},{},{},{},{}\nbords_{nom}=1\n",
        simulation.temps(),
        segments.len(),
        vivantes.len(),
        hache(segments.iter().map(|segment| f64::from(segment.temps))),
        hache(segments.iter().map(|segment| f64::from(segment.apex))),
        hache(vivantes.iter().flat_map(|pointe| [pointe.x, pointe.y])),
        simulation.empreinte()
    )
}

/// Ce que `pointes-wasm.mjs` doit écrire pour ce scénario, cette graine et ces morceaux de pas : `NaN` partout pour un module
/// neuf, même après le choix du scénario, puis les lignes du départ et de chaque morceau.
fn sortie_attendue(scenario: &Scenario, graine: u32, morceaux: &[u32]) -> String {
    let mut simulation = simulation_native(scenario, graine);
    let mut texte = String::from("avant=NaN,NaN,NaN,NaN\napres_choix=NaN,NaN,NaN,NaN\n");
    texte.push_str(&lignes_attendues("depart", &simulation));
    for (i, pas) in morceaux.iter().enumerate() {
        simulation.avancer_n(*pas);
        texte.push_str(&lignes_attendues(&i.to_string(), &simulation));
    }
    texte
}

/// 2. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne pour **chacun des six scénarios** les
///    trois lectures du code natif : pour trois graines (la plus simple, une grande qui ne tient pas sur 31 bits, le maximum)
///    et des morceaux de pas différents (cinq moments de 0 au-delà du pas de référence ; un avancement haché en 1, 0, 6, 13,
///    0 et 60 pas ; un seul morceau de 5 pas), le pas de naissance et l'hyphe de **chaque** segment et la position de
///    **chaque** pointe vivante sont ceux de `Simulation` (par leur SHA-256, au bit près), le nombre de segments et de
///    pointes aussi, et l'empreinte lue après les lectures est celle d'une simulation jamais lue. Lectures dans les deux
///    ordres (pointes d'abord aux morceaux impairs). Le module neuf rend `NaN` avant tout démarrage, et même une fois le
///    scénario choisi ; `NaN` hors de portée aux bords de chaque état. Si le code change sans que le module soit
///    reconstruit, ce test échoue.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn node_donne_les_trois_lectures_du_code_natif_pour_chaque_scenario() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");

    for scenario in &SCENARIOS {
        let numero = scenario.numero.to_string();
        let jusqu_au_pas_de_reference = [0, 1, 36, scenario.duree - 37, 40];
        for (graine, morceaux) in
            [(1, jusqu_au_pas_de_reference.to_vec()), (0xDEAD_BEEF, vec![1, 0, 6, 13, 0, 60]), (u32::MAX, vec![5])]
        {
            let liste: Vec<String> = morceaux.iter().map(u32::to_string).collect();
            let sortie =
                lancer_le_script("pointes-wasm.mjs", &[module, &numero, &graine.to_string(), &liste.join(",")]);
            let erreur = String::from_utf8_lossy(&sortie.stderr);
            assert!(
                sortie.status.success(),
                "{} : graine {graine}, morceaux {morceaux:?} : {:?}, sortie d'erreur : {erreur}",
                scenario.nom,
                sortie.status
            );
            assert_eq!(
                String::from_utf8_lossy(&sortie.stdout),
                sortie_attendue(scenario, graine, &morceaux),
                "{} : graine {graine}, morceaux {morceaux:?}",
                scenario.nom
            );
        }
    }
}

/// 3. `pointes-wasm.mjs` refuse des arguments faux avec le code de sortie 1 et un message sur la sortie d'erreur, sans rien
///    écrire sur la sortie standard : fichier absent, fichier qui n'est pas un module, trop ou trop peu d'arguments, numéro de
///    scénario qui n'est pas de 0 à 5 (refusé par le module) ou qui n'est pas un entier de 32 bits non signé, graine ou nombre
///    de pas qui n'en est pas un, liste de pas vide ou mal formée. Le témoin d'abord : avec des arguments justes, le script
///    réussit et écrit quelque chose ; sinon un refus ne prouverait rien.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn le_script_des_pointes_refuse_des_arguments_faux() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");
    let temoin = lancer_le_script("pointes-wasm.mjs", &[module, "5", "4294967295", "3,0,1"]);
    assert!(
        temoin.status.success() && !temoin.stdout.is_empty(),
        "arguments justes : {:?}, sortie d'erreur : {}",
        temoin.status,
        String::from_utf8_lossy(&temoin.stderr)
    );

    let absent = racine().join("docs").join("n-existe-pas.wasm");
    let absent = absent.to_str().expect("chemin en UTF-8");
    let pas_un_module = racine().join("Cargo.toml");
    let pas_un_module = pas_un_module.to_str().expect("chemin en UTF-8");
    let cas: [&[&str]; 13] = [
        &[absent, "0", "1", "1"],
        &[pas_un_module, "0", "1", "1"],
        &[],
        &[module, "0", "1"],
        &[module, "0", "1", "1", "en trop"],
        &[module, "6", "1", "1"],
        &[module, "4294967296", "1", "1"],
        &[module, "0", "-1", "1"],
        &[module, "0", "4294967296", "1"],
        &[module, "0", "1", ""],
        &[module, "0", "1", "1,"],
        &[module, "0", "1", "1,,2"],
        &[module, "0", "1", "4294967296"],
    ];
    for arguments in cas {
        let sortie = lancer_le_script("pointes-wasm.mjs", arguments);
        assert_eq!(sortie.status.code(), Some(1), "arguments {arguments:?}");
        assert!(sortie.stdout.is_empty(), "arguments {arguments:?} : la sortie standard n'est pas vide");
        assert!(!sortie.stderr.is_empty(), "arguments {arguments:?} : aucun message d'erreur");
    }
}
