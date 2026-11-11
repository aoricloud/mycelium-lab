//! Tests d'acceptation de l'étape 11 (b) : l'artefact commité avec la case « Densité », c'est-à-dire le module
//! `docs/mycelium.wasm` (reconstruit : 18 exportations) et la page `docs/index.html` + `docs/app.js`.
//!
//! Comme aux étapes 4 et 8, la CI n'a pas la cible WebAssembly : le module est construit à la main puis commité, et la
//! preuve qu'il est bon est qu'il tourne, sous Node, et donne la même chose que le code natif. Ici, pour la carte de
//! densité de chacun des six scénarios. Les scripts de `tests/etape_11/` sont : `densite-wasm.mjs`, qui lit la carte par
//! les deux accesseurs, et `page-wasm.mjs`, la copie étendue de celui de `tests/etape_8/` qui joue la vraie page dans un
//! faux navigateur avec la case cochée.
//!
//! 10. la forme du module : ses 18 exportations et leurs signatures ;
//! 11. le résultat du module, lu par Node et comparé au code natif **construit sans `vitrine.rs`** (le texte du scénario
//!     est lu sur le disque), à partir de `piece::carte_de_densite` et `piece::mesurer` : la carte, case par case (par son
//!     SHA-256), son maximum, et ce que rend un module neuf avant tout démarrage ;
//! 12. la page : la case à cocher, sa légende, ce que `app.js` emploie ; la page entière jouée dans le faux navigateur,
//!     dont l'image de densité est comparée à celle que `piece` donne (la rampe de `densite.png`, écrite ici à part).
//!
//! Le test de l'étape 8 qui fixait « exactement les 16 fonctions » a sa version pour 18 ici, et n'est plus compilé avec
//! `etape-11` (`etape_8_artefact.rs`).
//!
//! Ces tests ne touchent pas à l'état global de la vitrine (ils ne comparent qu'à `Simulation`) : ils n'ont pas besoin du
//! verrou de `etape_11_vitrine.rs`.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `min`, `sqrt` (exactes, la rampe de M0) et `to_le_bytes`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mycelium_lab::empreinte;
use mycelium_lab::piece::{carte_de_densite, mesurer};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

/// Un des six scénarios de la vitrine, tel que la spécification de l'étape 8 le fixe.
struct Scenario {
    /// Le numéro que prend `mycelium_choisir_scenario`.
    numero: u32,
    /// Le nom dans l'adresse de la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// Le côté de la boîte, en cases (elle est carrée).
    boite: usize,
    /// Le pas de référence (`duree` du fichier).
    duree: u32,
}

/// Les six scénarios, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas du code.
const SCENARIOS: [Scenario; 6] = [
    Scenario { numero: 0, nom: "libre", fichier: "couplage.txt", boite: 161, duree: 80 },
    Scenario { numero: 1, nom: "terrain", fichier: "terrain.txt", boite: 100, duree: 80 },
    Scenario { numero: 2, nom: "tache", fichier: "chimiotropisme.txt", boite: 100, duree: 100 },
    Scenario { numero: 3, nom: "tache-sans", fichier: "chimiotropisme-sans.txt", boite: 100, duree: 100 },
    Scenario { numero: 4, nom: "mur", fichier: "mur.txt", boite: 100, duree: 120 },
    Scenario { numero: 5, nom: "mur-sans", fichier: "mur-sans.txt", boite: 100, duree: 120 },
];

/// Les types de valeurs WebAssembly qui servent ici, tels que le format binaire les code.
const I32: u8 = 0x7F;
const F64: u8 = 0x7C;

/// Les fonctions que le module doit exporter, avec leur signature : types des paramètres, types des résultats
/// (`u32` est un `i32` en WebAssembly). Les seize premières sont celles des étapes 4 et 8, que rien ne change ; les deux
/// dernières sont celles de l'étape 11.
const FONCTIONS: [(&str, &[u8], &[u8]); 18] = [
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
];

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn chemin_script(nom: &str) -> PathBuf {
    racine().join("tests").join("etape_11").join(nom)
}

/// Le dossier où les scripts Node écrivent leurs copies de travail : sous `target/`, jamais ailleurs.
fn dossier_de_travail() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("etape_11")
}

fn lire_module() -> Vec<u8> {
    std::fs::read(chemin_module()).unwrap_or_else(|erreur| panic!("docs/mycelium.wasm se lit : {erreur}"))
}

fn lire_page(nom: &str) -> String {
    std::fs::read_to_string(racine().join("docs").join(nom))
        .unwrap_or_else(|erreur| panic!("docs/{nom} existe et se lit : {erreur}"))
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

/// 10. Le module exporte exactement les dix-huit fonctions de la liste : les douze de l'étape 4, les quatre de l'étape 8
///     et `mycelium_densite` et `mycelium_densite_max` ; la mémoire `memory` est tolérée, rien d'autre (aucune autre
///     exportation, de quelque genre que ce soit). Un nom n'est exporté qu'une fois. Un module construit avant l'étape 11
///     (16 exportations) ne passe pas.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn le_module_exporte_exactement_les_dix_huit_fonctions() {
    let module = analyser(&lire_module());

    let fonctions: BTreeSet<&str> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.as_str()).collect();
    let attendues: BTreeSet<&str> = FONCTIONS.iter().map(|(nom, _, _)| *nom).collect();
    assert_eq!(attendues.len(), 18);
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

/// 10 (suite). Chaque fonction exportée a la bonne signature : le nombre et le type des paramètres, et du résultat
/// (`i32` pour un `u32`, `f64` pour un `f64`). Les seize d'avant gardent la leur, et les deux nouvelles sont
/// `densite(i32, i32) -> f64` et `densite_max() -> f64`. Le module n'importe toujours rien.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
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

/// La simulation native en face : le scénario lu sur le disque (jamais par `include_str!`), cette graine, ce nombre
/// de pas.
fn simulation_native(scenario: &Scenario, graine: u32, pas: u32) -> Simulation {
    let chemin = racine().join("scenarios").join(scenario.fichier);
    let texte =
        std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", chemin.display()));
    let (reglages, milieu, tropismes, _duree) =
        lire_scenario_tropismes(&texte).expect("le scénario du disque est valide");
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages sont valides");
    simulation.avancer_n(pas);
    simulation
}

/// Les octets en hexadécimal, dans l'ordre.
fn hexadecimal(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

/// Les octets petit-boutistes des cases de la carte de densité de `simulation`, ligne par ligne : ceux que hache
/// `densite-wasm.mjs`.
fn octets_de_la_carte(simulation: &Simulation) -> Vec<u8> {
    let reglages = simulation.reglages();
    let carte = carte_de_densite(reglages.largeur, reglages.hauteur, simulation.segments());
    carte.valeurs().iter().flat_map(|valeur| valeur.to_le_bytes()).collect()
}

/// Ce que `densite-wasm.mjs` doit écrire pour ce scénario, cette graine et ces morceaux de pas : ce que rend un module
/// neuf avant tout démarrage (`NaN` partout, même après le choix du scénario) et juste après le démarrage (une carte
/// vide, de maximum 0.0), puis, après chaque morceau, le temps, le maximum de `piece::mesurer`, son nombre de cases
/// occupées et le SHA-256 de la carte de `piece::carte_de_densite`, et enfin les bords et la carte vide d'après un
/// second démarrage.
fn sortie_attendue(scenario: &Scenario, graine: u32, morceaux: &[u32]) -> String {
    let mut simulation = simulation_native(scenario, graine, 0);
    let vide = vec![0u8; 8 * scenario.boite * scenario.boite];
    let mut texte = format!(
        "avant_densite=NaN,NaN,NaN\navant_max=NaN\napres_choix=NaN,NaN,NaN,NaN\napres_demarrage={},{}\n",
        hexadecimal(&0.0f64.to_le_bytes()),
        empreinte::sha256_hex(&vide)
    );
    for (i, pas) in morceaux.iter().enumerate() {
        simulation.avancer_n(*pas);
        let mesures = mesurer(&simulation);
        texte.push_str(&format!(
            "carte_{i}={},{},{},{}\n",
            simulation.temps(),
            hexadecimal(&mesures.densite_maximale.to_le_bytes()),
            mesures.cases_occupees,
            empreinte::sha256_hex(&octets_de_la_carte(&simulation))
        ));
    }
    texte.push_str(&format!(
        "bords=1\napres_relance={},{}\n",
        hexadecimal(&0.0f64.to_le_bytes()),
        empreinte::sha256_hex(&vide)
    ));
    texte
}

/// 11. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne pour **chacun des six
///     scénarios** la carte de densité du code natif : pour trois graines (la plus simple, une grande qui ne tient pas sur
///     31 bits, le maximum) et des morceaux de pas différents (cinq moments de 0 au-delà du pas de référence ; un
///     avancement haché en 1, 0, 6, 13, 0 et 60 pas ; un seul morceau de 5 pas), le temps, le maximum, le nombre de cases
///     occupées et la carte entière (par son SHA-256) sont ceux de `piece`. La carte est relue après chaque morceau : une
///     carte gardée trop longtemps par le module se verrait. Le module neuf rend `NaN` avant tout démarrage et, même une
///     fois le scénario choisi, avant le démarrage ; une carte vide (maximum exactement 0.0) juste après ; et `NaN` hors
///     de la boîte (la colonne `largeur`, la ligne `hauteur`, `u32::MAX`). Si le code change sans que le module soit
///     reconstruit, ce test échoue.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn node_donne_la_carte_de_densite_du_code_natif_pour_chaque_scenario() {
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
                lancer_le_script("densite-wasm.mjs", &[module, &numero, &graine.to_string(), &liste.join(",")]);
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

/// 11 (suite). `densite-wasm.mjs` refuse des arguments faux avec le code de sortie 1 et un message sur la sortie d'erreur,
/// sans rien écrire sur la sortie standard : fichier absent, fichier qui n'est pas un module, trop ou trop peu
/// d'arguments, numéro de scénario qui n'est pas de 0 à 5 (refusé par le module) ou qui n'est pas un entier de 32 bits
/// non signé, graine ou nombre de pas qui n'en est pas un, liste de pas vide ou mal formée. Le témoin d'abord : avec des
/// arguments justes, le script réussit et écrit quelque chose ; sinon un refus ne prouverait rien.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn le_script_de_la_carte_refuse_des_arguments_faux() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");
    let temoin = lancer_le_script("densite-wasm.mjs", &[module, "5", "4294967295", "3,0,1"]);
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
    let cas: [&[&str]; 21] = [
        &[absent, "0", "1", "1"],
        &[pas_un_module, "0", "1", "1"],
        &[],
        &[module],
        &[module, "0"],
        &[module, "0", "1"],
        &[module, "0", "1", "1", "en trop"],
        &[module, "6", "1", "1"],
        &[module, "4294967295", "1", "1"],
        &[module, "abc", "1", "1"],
        &[module, "-1", "1", "1"],
        &[module, "4294967296", "1", "1"],
        &[module, "0", "abc", "1"],
        &[module, "0", "-1", "1"],
        &[module, "0", "4294967296", "1"],
        &[module, "0", "1", ""],
        &[module, "0", "1", "abc"],
        &[module, "0", "1", "1,"],
        &[module, "0", "1", "1,,2"],
        &[module, "0", "1", "-1"],
        &[module, "0", "1", "4294967296"],
    ];
    for arguments in cas {
        let sortie = lancer_le_script("densite-wasm.mjs", arguments);
        assert_eq!(sortie.status.code(), Some(1), "arguments {arguments:?}");
        assert!(sortie.stdout.is_empty(), "arguments {arguments:?} : la sortie standard n'est pas vide");
        assert!(!sortie.stderr.is_empty(), "arguments {arguments:?} : aucun message d'erreur");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// La page.
// ---------------------------------------------------------------------------------------------------------------

/// Le code de `app.js` sans ses commentaires : sans les blocs `/* … */` et sans les lignes qui ne sont qu'un
/// commentaire `//`. Un nom qui n'apparaît que dans un commentaire n'est pas employé.
fn sans_commentaires(script: &str) -> String {
    let mut sans_blocs = String::new();
    let mut reste = script;
    while let Some(debut) = reste.find("/*") {
        sans_blocs.push_str(&reste[..debut]);
        reste = reste[debut..].find("*/").map_or("", |fin| &reste[debut + fin + 2..]);
    }
    sans_blocs.push_str(reste);
    let lignes: Vec<&str> = sans_blocs.lines().filter(|ligne| !ligne.trim_start().starts_with("//")).collect();
    lignes.join("\n")
}

/// Les identifiants que le script demande par `getElementById("…")` (ou avec des apostrophes).
fn identifiants_demandes(script: &str) -> Vec<String> {
    let mut identifiants = Vec::new();
    let mut reste = script;
    while let Some(debut) = reste.find("getElementById(") {
        let apres = reste[debut + "getElementById(".len()..].trim_start();
        let Some(guillemet) = apres.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            panic!("getElementById doit recevoir un texte littéral : {}", &apres[..apres.len().min(30)]);
        };
        let nom = &apres[1..];
        let fin = nom.find(guillemet).expect("guillemet fermant de getElementById");
        identifiants.push(nom[..fin].to_string());
        reste = &nom[fin..];
    }
    identifiants
}

/// Les noms `mycelium_…` que le texte emploie : chaque suite de lettres, de chiffres et de `_` qui commence par
/// `mycelium_`.
fn noms_employes(texte: &str) -> BTreeSet<String> {
    let mut noms = BTreeSet::new();
    let mut reste = texte;
    while let Some(debut) = reste.find("mycelium_") {
        let a_partir_du_nom = &reste[debut..];
        let fin =
            a_partir_du_nom.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(a_partir_du_nom.len());
        noms.insert(a_partir_du_nom[..fin].to_string());
        reste = &a_partir_du_nom[fin..];
    }
    noms
}

/// Les balises du HTML, chacune réduite à son contenu entre `<` et `>` (le nom, puis les attributs).
fn balises(html: &str) -> Vec<&str> {
    html.split('<').skip(1).filter_map(|morceau| morceau.split('>').next()).collect()
}

/// La valeur d'un attribut d'une balise (écrite entre guillemets ou entre apostrophes), s'il y en a un. L'attribut
/// doit commencer le mot : `data-id="x"` n'est pas `id`.
fn attribut(balise: &str, nom: &str) -> Option<String> {
    let motif = format!("{nom}=");
    for (position, _) in balise.match_indices(&motif) {
        if !balise[..position].ends_with(char::is_whitespace) {
            continue;
        }
        let apres = &balise[position + motif.len()..];
        let guillemet = apres.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let valeur = &apres[1..];
        return Some(valeur[..valeur.find(guillemet)?].to_string());
    }
    None
}

/// Vrai si la balise porte cet attribut sans valeur (`hidden`, `checked`) ou avec une (`checked="checked"`).
fn a_l_attribut(balise: &str, nom: &str) -> bool {
    balise.split_whitespace().skip(1).any(|mot| mot == nom || mot.starts_with(&format!("{nom}=")))
}

/// Le texte lisible d'un morceau de HTML : sans ses balises, les espaces réduits à une seule.
fn texte_visible(html: &str) -> String {
    let mut visible = String::new();
    let mut dans_une_balise = false;
    for c in html.chars() {
        match c {
            '<' => dans_une_balise = true,
            '>' => {
                dans_une_balise = false;
                visible.push(' ');
            }
            _ if !dans_une_balise => visible.push(c),
            _ => {}
        }
    }
    visible.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le texte lisible de l'élément `<nom …>…</nom>` dont l'attribut `attribut_cherche` vaut `valeur`, et sa balise ouvrante ;
/// `None` si la page n'a pas cet élément.
fn element<'a>(html: &'a str, nom: &str, attribut_cherche: &str, valeur: &str) -> Option<(&'a str, String)> {
    let mut debut = 0;
    while let Some(position) = html[debut..].find(&format!("<{nom}")) {
        let ouvrante = debut + position;
        let fin_ouvrante = ouvrante + html[ouvrante..].find('>')?;
        let balise = &html[ouvrante + 1..fin_ouvrante];
        if attribut(balise, attribut_cherche).as_deref() == Some(valeur) {
            let fermante = fin_ouvrante + html[fin_ouvrante..].find(&format!("</{nom}>"))?;
            return Some((balise, texte_visible(&html[fin_ouvrante + 1..fermante])));
        }
        debut = fin_ouvrante;
    }
    None
}

/// 12. La page a **une seule** case à cocher, d'id `densite`, décochée au départ (aucun attribut `checked`), étiquetée
///     par un `<label for="densite">` qui dit « Densité ». Sa légende est un `<p id="legende-densite">` cachée au départ
///     (attribut `hidden`), placée après le canvas (sous l'image), qui dit « peu » puis « beaucoup ». La légende du
///     substrat est un `<p id="legende-substrat">`, visible au départ, qui dit « Substrat ».
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_page_a_une_seule_case_densite_et_sa_legende_cachee() {
    let html = lire_page("index.html");
    let cases: Vec<&str> = balises(&html)
        .into_iter()
        .filter(|b| b.starts_with("input") && attribut(b, "type").as_deref() == Some("checkbox"))
        .collect();
    assert_eq!(cases.len(), 1, "la page a {} cases à cocher : {cases:?}", cases.len());
    assert_eq!(attribut(cases[0], "id").as_deref(), Some("densite"), "la case n'a pas l'id « densite »");
    assert!(!a_l_attribut(cases[0], "checked"), "la case est cochée au départ : <{}>", cases[0]);

    let (_, etiquette) = element(&html, "label", "for", "densite").expect("pas de <label for=\"densite\">");
    assert!(etiquette.contains("Densité"), "le libellé de la case ne dit pas « Densité » : « {etiquette} »");

    let (balise, legende) = element(&html, "p", "id", "legende-densite").expect("pas de <p id=\"legende-densite\">");
    assert!(a_l_attribut(balise, "hidden"), "la légende n'est pas cachée au départ : <{balise}>");
    let (peu, beaucoup) = (legende.find("peu"), legende.find("beaucoup"));
    assert!(
        peu.is_some() && beaucoup.is_some() && peu < beaucoup,
        "la légende ne dit pas « peu » puis « beaucoup » : « {legende} »"
    );
    let canvas = html.find("<canvas").expect("un <canvas");
    assert!(html.find("id=\"legende-densite\"").expect("la légende") > canvas, "la légende est avant le canvas");

    // La légende du substrat a un identifiant (la page la cache quand la case est cochée) et se voit au départ.
    let (balise, substrat) = element(&html, "p", "id", "legende-substrat").expect("pas de <p id=\"legende-substrat\">");
    assert!(!a_l_attribut(balise, "hidden"), "la légende du substrat est cachée au départ : <{balise}>");
    assert!(substrat.contains("Substrat"), "la légende du substrat ne dit pas « Substrat » : « {substrat} »");
}

/// 12 (suite). La barre de la légende est la rampe de `densite.png`, du violet (à gauche, « peu ») au jaune (à droite,
/// « beaucoup ») : la légende contient un `<span class="echantillon-densite">`, et la règle `.echantillon-densite` de la
/// feuille de style a pour fond `linear-gradient(90deg, rgb(70, 20, 110), rgb(255, 240, 90))` (comparé sans les blancs).
/// La feuille garde aussi la règle `.legende[hidden] { display: none; }` : une légende est en `display: flex`, et sans cette
/// règle l'attribut `hidden`, que la page pose et ôte, ne la cacherait pas dans un vrai navigateur.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_barre_de_la_legende_va_du_violet_au_jaune() {
    let html = lire_page("index.html");
    let debut = html.find("<style").expect("une feuille de style");
    let fin = html[debut..].find("</style>").map(|fin| debut + fin).expect("fin de la feuille de style");
    let feuille: String = html[debut..fin].to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    assert!(feuille.contains(".legende[hidden]{display:none;}"), "la règle .legende[hidden] n'est plus là");
    let regle =
        feuille.find(".echantillon-densite{").expect("pas de règle .echantillon-densite dans la feuille de style");
    let corps = &feuille[regle..];
    let corps = &corps[..corps.find('}').expect("fin de la règle .echantillon-densite")];
    assert!(
        corps.contains("background:linear-gradient(90deg,rgb(70,20,110),rgb(255,240,90))"),
        "la barre ne va pas du violet au jaune : « {corps} »"
    );

    let legende = html.find("id=\"legende-densite\"").expect("la légende");
    let legende = &html[legende..legende + html[legende..].find("</p>").expect("fin de la légende")];
    assert!(legende.contains("class=\"echantillon-densite\""), "la légende n'a pas la barre de la rampe");
}

/// 12 (suite). `app.js` demande la case, sa légende et la légende du substrat (qu'il cache quand la case est cochée) par
/// `getElementById` (les trois identifiants existent dans la page), et emploie les deux nouvelles exportations du module,
/// `mycelium_densite` et `mycelium_densite_max`, chacune écrite en entier (un nom est un mot entier : `mycelium_densite`
/// ne compte pas dans `mycelium_densite_max`).
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn app_js_demande_la_case_et_la_legende_et_emploie_les_deux_exportations() {
    let code = sans_commentaires(&lire_page("app.js"));
    let demandes = identifiants_demandes(&code);
    let html = lire_page("index.html");
    let existants: BTreeSet<String> = balises(&html).iter().filter_map(|b| attribut(b, "id")).collect();
    for identifiant in ["densite", "legende-densite", "legende-substrat"] {
        assert!(demandes.iter().any(|d| d == identifiant), "app.js ne demande pas l'élément « {identifiant} »");
        assert!(existants.contains(identifiant), "index.html n'a pas d'élément d'id « {identifiant} »");
    }
    let employes = noms_employes(&code);
    for nom in ["mycelium_densite", "mycelium_densite_max"] {
        assert!(employes.contains(nom), "app.js n'emploie pas {nom}");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Le dessin de la densité : l'oracle de la page.
// ---------------------------------------------------------------------------------------------------------------

/// La couleur d'une case sans hyphe dans `densite.png` (`examples/mesure_piece.rs`), et les deux bouts de la rampe :
/// du violet (peu) au jaune (la densité maximale de la carte).
const FOND: [i32; 3] = [16, 16, 24];
const RAMPE_SOMBRE: [i32; 3] = [70, 20, 110];
const RAMPE_CLAIRE: [i32; 3] = [255, 240, 90];

/// La couche de densité que la page doit poser sur le canvas, d'après `piece` : un pixel RGBA par case, ligne par ligne.
/// Une case sans hyphe (densité nulle, les cases d'un obstacle comprises : la page dessine les obstacles par-dessus) est du
/// fond ; une case occupée suit la rampe, `niveau = ⌊√min(d / dmax, 1) × 255⌋` (`dmax` : `densite_maximale` de
/// `piece::mesurer`) puis, canal par canal, `sombre + (clair − sombre) × niveau / 255` en entiers (la division de Rust,
/// qui arrondit vers zéro). La page, elle, la calcule en JavaScript : c'est ce test qui lie les deux.
fn couche_attendue(simulation: &Simulation) -> Vec<u8> {
    let reglages = simulation.reglages();
    let carte = carte_de_densite(reglages.largeur, reglages.hauteur, simulation.segments());
    let maximum = mesurer(simulation).densite_maximale;
    let mut octets = Vec::new();
    for &densite in carte.valeurs() {
        if densite > 0.0 {
            let niveau = ((densite / maximum).min(1.0).sqrt() * 255.0) as i32;
            for (sombre, clair) in RAMPE_SOMBRE.iter().zip(RAMPE_CLAIRE.iter()) {
                octets.push((sombre + (clair - sombre) * niveau / 255) as u8);
            }
        } else {
            octets.extend(FOND.iter().map(|&canal| canal as u8));
        }
        octets.push(255);
    }
    octets
}

/// Le nombre de vérifications que fait `page-wasm.mjs` sur la page et le module de cette étape. Il ne dépend ni de la
/// machine ni du temps ; il change si le script change (le recompter alors, et le dire).
const VERIFICATIONS_DE_LA_PAGE: u32 = 1297;

/// 12 (suite). La page entière, jouée dans un faux navigateur avec le vrai module (`page-wasm.mjs`), case cochée : à
/// l'ouverture elle est décochée, sa légende cachée, et la page ne lit pas la densité ; cochée, la couche de densité de la
/// page est la carte du module sur la rampe de `densite.png` (des cases nettes, sans hyphe ni vignettage, les obstacles
/// dessinés par-dessus comme d'habitude, la bordure intacte) ; une case dense est plus claire qu'une case vide, la couleur
/// d'une case vide n'est pas celle du substrat ; cocher puis décocher redonne exactement le dessin d'avant ; la carte se
/// redessine à chaque image, à la pause, au pas de référence, au changement de scénario, à « Relancer », à une adresse
/// changée et à la fin de la croissance ; les légendes disent ce qui est dessiné (cochée : celle de la densité, ni celle
/// du substrat ni celle de la tache). Le script écrit `couche=<scénario>,<graine>,<pas>,<hache>` pour la couche jouée au
/// temps 0 et au pas de référence de chacun des six scénarios, puis `ok <n>` (n : le nombre de vérifications faites,
/// `VERIFICATIONS_DE_LA_PAGE` exactement). Ce test recalcule chaque `hache` avec `piece::carte_de_densite` et sa propre
/// rampe (`couche_attendue`) : la page dessine ce que dessine M0.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_page_jouee_avec_la_case_cochee_dessine_la_carte_de_densite_de_m0() {
    if !node_est_la() {
        return;
    }
    let dossier = dossier_de_travail();
    let docs = racine().join("docs");
    let module = chemin_module();
    let sortie = lancer_le_script(
        "page-wasm.mjs",
        &[
            docs.to_str().expect("chemin en UTF-8"),
            module.to_str().expect("chemin en UTF-8"),
            dossier.to_str().expect("chemin en UTF-8"),
        ],
    );
    let erreur = String::from_utf8_lossy(&sortie.stderr);
    assert!(sortie.status.success(), "page-wasm.mjs : {:?}, sortie d'erreur : {erreur}", sortie.status);
    let texte = String::from_utf8_lossy(&sortie.stdout);
    let lignes: Vec<&str> = texte.lines().collect();
    let (derniere, couches) = lignes.split_last().expect("page-wasm.mjs n'a rien écrit");
    let nombre: u32 = derniere
        .strip_prefix("ok ")
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("page-wasm.mjs n'a pas fini par « ok <n> » : « {texte} »"));
    // Le nombre exact : la page et la simulation sont déterministes, un jeu sauté ou ajouté se voit ici.
    assert_eq!(nombre, VERIFICATIONS_DE_LA_PAGE, "page-wasm.mjs a fait {nombre} vérifications");

    // Les couches : une au temps 0 et une au pas de référence, pour chacun des six scénarios, toutes comparées à `piece`.
    assert_eq!(couches.len(), 12, "{} couches écrites au lieu de 12 : {couches:?}", couches.len());
    let mut vues = BTreeSet::new();
    for ligne in couches {
        let champs: Vec<&str> = ligne
            .strip_prefix("couche=")
            .unwrap_or_else(|| panic!("ligne inattendue : « {ligne} »"))
            .split(',')
            .collect();
        let [nom, graine, pas, hache] = champs[..] else { panic!("ligne mal formée : « {ligne} »") };
        let scenario =
            SCENARIOS.iter().find(|s| s.nom == nom).unwrap_or_else(|| panic!("scénario inconnu : « {nom} »"));
        let (graine, pas): (u32, u32) = (graine.parse().expect("graine"), pas.parse().expect("pas"));
        assert!(pas == 0 || pas == scenario.duree, "{nom} : la couche est jouée au pas {pas}");
        let attendue = couche_attendue(&simulation_native(scenario, graine, pas));
        assert_eq!(attendue.len(), 4 * scenario.boite * scenario.boite);
        assert_eq!(
            hache,
            empreinte::sha256_hex(&attendue),
            "{nom}, graine {graine}, pas {pas} : la couche de la page n'est pas celle de M0"
        );
        vues.insert((nom, pas == 0));
    }
    assert_eq!(vues.len(), 12, "chaque scénario doit être joué au temps 0 et au pas de référence : {vues:?}");
}
