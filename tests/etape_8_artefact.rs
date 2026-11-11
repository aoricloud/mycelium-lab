//! Tests d'acceptation de l'étape 8 (b) : l'artefact commité avec le menu de six scénarios, c'est-à-dire le module
//! `docs/mycelium.wasm` et la page `docs/index.html` + `docs/app.js` + `docs/adresse.js`.
//!
//! Comme à l'étape 4, la CI n'a pas la cible WebAssembly : le module est construit à la main puis commité, et la preuve
//! qu'il est bon est qu'il tourne, sous Node, et donne la même chose que le code natif. Ici, pour chacun des six
//! scénarios. Les scripts de `tests/etape_8/` (`empreinte-wasm.mjs`, `etat-wasm.mjs`, `initial-wasm.mjs`) sont ceux de
//! `tests/etape_4/` avec le choix du scénario en plus ; `adresse.mjs` fait tourner les fonctions pures de `adresse.js`
//! et `page-wasm.mjs` fait tourner la vraie page dans un faux navigateur, avec le vrai module.
//!
//! 7. la forme du module : ses exportations (celles de l'étape 4 plus les quatre nouvelles) et leurs signatures ;
//! 8. le résultat du module, lu par Node et comparé au code natif **construit sans `vitrine.rs`** (le texte du scénario
//!    est lu sur le disque) : l'empreinte, ce que lisent les accesseurs, et ce que fait un module neuf ;
//! 9. la page : le menu, les descriptions, la note sur la limite du banc, le message d'arrêt ; l'adresse (`adresse.js`
//!    sous Node) ; la page entière jouée dans un faux navigateur.
//!
//! Les tests de l'étape 4 qui fixaient la liste des exportations et des éléments de la page ont leur version
//! `_t4` dans `etape_4_artefact.rs`.
//!
//! Ces tests ne touchent pas à l'état global de la vitrine (ils ne comparent qu'à `Simulation`) : ils n'ont pas besoin
//! du verrou de `etape_8_vitrine.rs`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

/// Le plafond du module : 1 Mo.
const TAILLE_MAX_MODULE: u64 = 1024 * 1024;

/// Les types de valeurs WebAssembly qui servent ici, tels que le format binaire les code.
const I32: u8 = 0x7F;
const F64: u8 = 0x7C;

/// Les fonctions que le module doit exporter, avec leur signature : types des paramètres, types des résultats
/// (`u32` est un `i32` en WebAssembly). Les douze premières sont celles de l'étape 4, que rien ne change ; les quatre
/// dernières sont celles de l'étape 8.
const FONCTIONS: [(&str, &[u8], &[u8]); 16] = [
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
];

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn chemin_script(nom: &str) -> PathBuf {
    racine().join("tests").join("etape_8").join(nom)
}

/// Le dossier où les scripts Node écrivent leurs copies de travail : sous `target/`, jamais ailleurs.
fn dossier_de_travail() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("etape_8")
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
// de fonctions et d'exportations (le même que celui de `etape_4_artefact.rs`).
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

/// 7. Le module pèse moins de 1 Mo, il n'importe rien (ni fonction, ni mémoire, ni table, ni globale), et il embarque
///    le texte des six scénarios (`include_str!`) : chacun des six fichiers de `scenarios/`, lu sur le disque, se
///    retrouve tel quel dans ses octets. La page ne charge donc rien d'autre que le module ; et un module construit avant
///    l'étape 8, qui n'embarquait que `couplage.txt`, ne passe pas.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn le_module_est_leger_n_importe_rien_et_embarque_les_six_scenarios() {
    let octets = lire_module();
    let module = analyser(&octets);
    assert!((octets.len() as u64) < TAILLE_MAX_MODULE, "{} octets", octets.len());
    assert_eq!(module.importations, 0, "le module a {} importation(s)", module.importations);
    for scenario in &SCENARIOS {
        let texte = texte(scenario);
        assert!(
            octets.windows(texte.len()).any(|fenetre| fenetre == texte.as_bytes()),
            "le module n'embarque pas scenarios/{} : reconstruit-il les six scénarios ?",
            scenario.fichier
        );
    }
}

/// 7 (suite). Le module exporte exactement les douze fonctions de l'étape 4 et les quatre de l'étape 8, ni plus ni
/// moins ; la mémoire `memory` est tolérée, rien d'autre (aucune autre exportation, de quelque genre que ce soit). Un
/// nom n'est exporté qu'une fois.
///
/// L'étape 11 ajoute deux exportations : avec `etape-11`, ce test n'est pas compilé et c'est
/// `le_module_exporte_exactement_les_dix_huit_fonctions` (`etape_11_artefact.rs`) qui fixe la liste exacte, ces seize et
/// les deux nouvelles.
#[cfg(not(feature = "etape-11"))]
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn le_module_exporte_les_douze_fonctions_de_l_etape_4_et_les_quatre_nouvelles() {
    let module = analyser(&lire_module());

    let fonctions: BTreeSet<&str> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.as_str()).collect();
    let attendues: BTreeSet<&str> = FONCTIONS.iter().map(|(nom, _, _)| *nom).collect();
    assert_eq!(fonctions, attendues, "les fonctions exportées ne sont pas celles de la liste");
    assert_eq!(attendues.len(), 16);

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

/// 7 (suite). Chaque fonction exportée a la bonne signature : le nombre et le type des paramètres, et du résultat
/// (`i32` pour un `u32`, `f64` pour un `f64`). Les douze de l'étape 4 gardent la leur, et les quatre nouvelles sont
/// `choisir_scenario(i32) -> i32`, `duree() -> i32`, `nb_obstacles() -> i32` et `obstacle(i32, i32) -> f64`.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn chaque_fonction_a_la_bonne_signature() {
    let module = analyser(&lire_module());
    assert_eq!(module.importations, 0, "les indices de fonction supposent qu'il n'y a aucune importation");

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

/// Lance ce script avec ces arguments, et ce texte sur son entrée standard.
fn lancer_le_script_avec_entree(script: &str, arguments: &[&str], entree: &str) -> Output {
    let mut node = Command::new("node")
        .arg(chemin_script(script))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"));
    node.stdin.take().expect("entrée standard de Node").write_all(entree.as_bytes()).expect("écriture sur l'entrée");
    node.wait_with_output().expect("Node se termine")
}

/// Lance ce script avec ces arguments.
fn lancer_le_script(script: &str, arguments: &[&str]) -> Output {
    lancer_le_script_avec_entree(script, arguments, "")
}

/// Le texte d'un scénario, lu sur le disque (jamais par `include_str!`, pour qu'un mauvais appariement des numéros et
/// des fichiers soit attrapé).
fn texte(scenario: &Scenario) -> String {
    let chemin = racine().join("scenarios").join(scenario.fichier);
    std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", chemin.display()))
}

/// La simulation native en face : le scénario lu sur le disque, cette graine, ce nombre de pas.
fn simulation_native(scenario: &Scenario, graine: u32, pas: u32) -> Simulation {
    let (reglages, milieu, tropismes, _duree) =
        lire_scenario_tropismes(&texte(scenario)).expect("le scénario du disque est valide");
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages sont valides");
    simulation.avancer_n(pas);
    simulation
}

/// Les octets en hexadécimal, dans l'ordre.
fn hexadecimal(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

/// Ce que `etat-wasm.mjs` doit écrire : l'état de la simulation native, lu comme le script le lit.
fn etat_attendu(scenario: &Scenario, graine: u32, pas: u32) -> String {
    let simulation = simulation_native(scenario, graine, pas);
    let substrat = simulation.substrat();
    let duree = lire_scenario_tropismes(&texte(scenario)).expect("le scénario est valide").3;

    let mut segments = Vec::new();
    for segment in simulation.segments() {
        for valeur in [segment.x0, segment.y0, segment.x1, segment.y1] {
            segments.extend_from_slice(&valeur.to_le_bytes());
        }
    }
    let mut cases = Vec::new();
    for y in 0..substrat.hauteur() {
        for x in 0..substrat.largeur() {
            cases.extend_from_slice(&substrat.valeur(x, y).to_le_bytes());
        }
    }
    let obstacles = &simulation.milieu().obstacles;
    let lignes_obstacles: String = obstacles
        .iter()
        .enumerate()
        .map(|(i, o)| format!("obstacle_{i}={},{},{},{}\n", o.x0, o.y0, o.x1, o.y1))
        .collect();

    format!(
        "temps={}\nlargeur={}\nhauteur={}\nduree={duree}\nsegments={}\npointes={}\nanastomoses={}\nbiomasse={}\n\
         hache_segments={}\nhache_substrat={}\nobstacles={}\n{lignes_obstacles}bords=1\n",
        simulation.temps(),
        substrat.largeur(),
        substrat.hauteur(),
        simulation.segments().len(),
        simulation.pointes(),
        simulation.anastomoses(),
        hexadecimal(&simulation.biomasse().to_le_bytes()),
        empreinte::sha256_hex(&segments),
        empreinte::sha256_hex(&cases),
        obstacles.len(),
    )
}

/// L'empreinte de référence du dépôt : le couplage, graine 1, 80 pas.
fn reference_couplage_graine_1() -> String {
    std::fs::read_to_string(racine().join("references").join("couplage-graine-1.empreinte"))
        .expect("la référence se lit")
        .trim()
        .to_string()
}

/// 8. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne pour **chacun des six
///    scénarios** la même empreinte que le code natif : pour la graine 1 et pour une grande graine au pas de référence,
///    pour une autre graine à 0 pas (l'état initial), à 1 pas, à un pas moyen et plus loin, et pour la plus grande des
///    graines. Si le code change sans que le module soit reconstruit, ce test échoue. La sortie du script est
///    exactement les 64 caractères et un saut de ligne.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn node_donne_l_empreinte_du_code_natif_pour_chaque_scenario() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");

    for scenario in &SCENARIOS {
        let numero = scenario.numero.to_string();
        for (graine, pas) in [
            (1, scenario.duree),
            (0xDEAD_BEEF, scenario.duree),
            (7, 0),
            (31, 1),
            (3, 37),
            (31, scenario.duree + 40),
            (u32::MAX, 5),
        ] {
            let sortie =
                lancer_le_script("empreinte-wasm.mjs", &[module, &numero, &graine.to_string(), &pas.to_string()]);
            let erreur = String::from_utf8_lossy(&sortie.stderr);
            assert!(
                sortie.status.success(),
                "{} : graine {graine}, {pas} pas : {:?}, sortie d'erreur : {erreur}",
                scenario.nom,
                sortie.status
            );
            let attendu = format!("{}\n", simulation_native(scenario, graine, pas).empreinte());
            assert_eq!(
                String::from_utf8_lossy(&sortie.stdout),
                attendu,
                "{} : graine {graine}, {pas} pas",
                scenario.nom
            );
        }
    }
}

/// 8 (suite). Pour le couplage (numéro 0), graine 1 et 80 pas, la sortie du script est le contenu du fichier de
/// référence du dépôt : le module neuf, avec le choix explicite du numéro 0, redonne la référence.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn node_redonne_la_reference_du_depot_pour_le_couplage() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let sortie = lancer_le_script("empreinte-wasm.mjs", &[module.to_str().expect("chemin en UTF-8"), "0", "1", "80"]);
    assert!(sortie.status.success(), "{}", String::from_utf8_lossy(&sortie.stderr));
    assert_eq!(String::from_utf8_lossy(&sortie.stdout).trim_end(), reference_couplage_graine_1());
}

/// 8 (suite). Le module, lu comme la page le lit, par ses accesseurs et non par l'empreinte, pour chacun des six
/// scénarios : le temps, la boîte, la durée du scénario, les compteurs, la biomasse (bit à bit), tous les segments et
/// toutes les cases du substrat (par leur SHA-256), les obstacles (nombre et coordonnées) sont ceux du code natif, et
/// les accesseurs rendent `NaN` hors de portée (obstacle compris) et 0 pour le mot d'empreinte 8. L'empreinte seule ne
/// dit rien des accesseurs : un module construit avant la correction d'un accesseur (`mycelium_obstacle`,
/// `mycelium_duree`…) a la même empreinte et dessinerait n'importe quoi. Le script `etat-wasm.mjs` écrit une ligne
/// `clé=valeur` par grandeur ; ce test compare sa sortie, entière, à celle que le code natif donne.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn node_lit_l_etat_comme_la_page_pour_chaque_scenario() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");

    for scenario in &SCENARIOS {
        let numero = scenario.numero.to_string();
        for (graine, pas) in [(1, scenario.duree), (0xDEAD_BEEF, 37), (7, 0)] {
            let sortie = lancer_le_script("etat-wasm.mjs", &[module, &numero, &graine.to_string(), &pas.to_string()]);
            let erreur = String::from_utf8_lossy(&sortie.stderr);
            assert!(
                sortie.status.success(),
                "{} : graine {graine}, {pas} pas : {:?}, sortie d'erreur : {erreur}",
                scenario.nom,
                sortie.status
            );
            assert_eq!(
                String::from_utf8_lossy(&sortie.stdout),
                etat_attendu(scenario, graine, pas),
                "{} : graine {graine}, {pas} pas",
                scenario.nom
            );
        }
    }
}

/// 8 (suite). Un module qui vient d'être instancié, sans rien démarrer : `mycelium_duree`, `mycelium_temps`,
/// `mycelium_largeur` et `mycelium_nb_obstacles` rendent 0, `mycelium_obstacle` rend `NaN` (c'est le contrat que les
/// tests natifs ne peuvent pas voir, l'état global y étant déjà posé). `mycelium_choisir_scenario` rend 1 pour 0 à 5 et
/// 0 pour 6, 7, 8, 255, 256, 260, 65 540, 2^31 + 4 et `u32::MAX` ; il ne démarre rien (tout est encore à 0). Le
/// démarrage suivant prend le dernier choix accepté (5 : le mur sans tropisme), choisir encore (0) n'agit pas sur la
/// simulation en cours, et le démarrage d'après prend le 0. Un second module neuf, qui ne choisit rien, démarre le
/// couplage et redonne la référence du dépôt.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn un_module_neuf_ne_demarre_rien_et_choisit_le_couplage_par_defaut() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let sortie = lancer_le_script("initial-wasm.mjs", &[module.to_str().expect("chemin en UTF-8")]);
    let erreur = String::from_utf8_lossy(&sortie.stderr);
    assert!(sortie.status.success(), "{:?}, sortie d'erreur : {erreur}", sortie.status);

    let attendu = format!(
        "avant_duree=0\navant_temps=0\navant_largeur=0\navant_obstacles=0\n\
         avant_obstacle=NaN,NaN,NaN,NaN\navant_obstacle_hors=NaN,NaN\n\
         choix_0_a_5=1,1,1,1,1,1\nchoix_refuses=0,0,0,0,0,0,0,0,0\n\
         apres_choix_duree=0\napres_choix_temps=0\napres_choix_largeur=0\napres_choix_obstacles=0\n\
         garde_duree=120\ngarde_largeur=100\ngarde_obstacles=1\n\
         en_cours_duree=120\nen_cours_largeur=100\nen_cours_obstacles=1\nen_cours_temps=5\n\
         suivant_duree=80\nsuivant_largeur=161\nsuivant_obstacles=0\nsuivant_temps=0\n\
         defaut_duree=80\ndefaut_largeur=161\ndefaut_obstacles=0\ndefaut_empreinte={}\n",
        reference_couplage_graine_1()
    );
    assert_eq!(String::from_utf8_lossy(&sortie.stdout), attendu);
}

/// 8 (suite). Les scripts refusent des arguments faux avec le code de sortie 1 et un message sur la sortie d'erreur,
/// sans rien écrire sur la sortie standard : fichier absent, fichier qui n'est pas un module, trop ou trop peu
/// d'arguments, numéro de scénario qui n'est pas de 0 à 5 (refusé par le module) ou qui n'est pas un entier de 32 bits
/// non signé, graine ou nombre de pas qui n'en est pas un. Le témoin d'abord : avec des arguments justes (le
/// scénario 5, la plus grande graine), chaque script réussit et écrit quelque chose ; sinon un refus ne prouverait
/// rien (un module qui n'exporte pas `mycelium_choisir_scenario` ferait échouer tous les cas).
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn les_scripts_refusent_des_arguments_faux() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");
    for (script, arguments) in [
        ("empreinte-wasm.mjs", vec![module, "5", "4294967295", "3"]),
        ("etat-wasm.mjs", vec![module, "5", "4294967295", "3"]),
        ("initial-wasm.mjs", vec![module]),
    ] {
        let sortie = lancer_le_script(script, &arguments);
        assert!(
            sortie.status.success() && !sortie.stdout.is_empty(),
            "{script}, arguments justes {arguments:?} : {:?}, sortie d'erreur : {}",
            sortie.status,
            String::from_utf8_lossy(&sortie.stderr)
        );
    }
    let absent = racine().join("docs").join("n-existe-pas.wasm");
    let absent = absent.to_str().expect("chemin en UTF-8");
    let pas_un_module = racine().join("Cargo.toml");
    let pas_un_module = pas_un_module.to_str().expect("chemin en UTF-8");

    let cas: [&[&str]; 17] = [
        &[absent, "0", "1", "80"],
        &[pas_un_module, "0", "1", "80"],
        &[],
        &[module],
        &[module, "0"],
        &[module, "0", "1"],
        &[module, "0", "1", "80", "en trop"],
        &[module, "6", "1", "80"],
        &[module, "4294967295", "1", "80"],
        &[module, "abc", "1", "80"],
        &[module, "-1", "1", "80"],
        &[module, "4294967296", "1", "80"],
        &[module, "0", "abc", "80"],
        &[module, "0", "-1", "80"],
        &[module, "0", "4294967296", "80"],
        &[module, "0", "1", "-1"],
        &[module, "0", "1", "4294967296"],
    ];
    for script in ["empreinte-wasm.mjs", "etat-wasm.mjs"] {
        for arguments in cas {
            let sortie = lancer_le_script(script, arguments);
            assert_eq!(sortie.status.code(), Some(1), "{script}, arguments {arguments:?}");
            assert!(sortie.stdout.is_empty(), "{script}, arguments {arguments:?} : la sortie standard n'est pas vide");
            assert!(!sortie.stderr.is_empty(), "{script}, arguments {arguments:?} : aucun message d'erreur");
        }
    }
    for arguments in [vec![], vec![absent], vec![pas_un_module], vec![module, "en trop"]] {
        let sortie = lancer_le_script("initial-wasm.mjs", &arguments);
        assert_eq!(sortie.status.code(), Some(1), "initial-wasm.mjs, arguments {arguments:?}");
        assert!(
            sortie.stdout.is_empty(),
            "initial-wasm.mjs, arguments {arguments:?} : la sortie standard n'est pas vide"
        );
        assert!(!sortie.stderr.is_empty(), "initial-wasm.mjs, arguments {arguments:?} : aucun message d'erreur");
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

/// La page telle qu'on la lit : sans les commentaires HTML, ni les blocs `<style>` et `<script>`, ni les balises. Un
/// texte qui n'est que dans un commentaire, dans le CSS ou dans une valeur d'attribut n'est pas sur la page.
fn texte_visible(html: &str) -> String {
    let mut sans = html.to_string();
    for (ouvrante, fermante) in [("<!--", "-->"), ("<style", "</style>"), ("<script", "</script>")] {
        while let Some(debut) = sans.find(ouvrante) {
            let fin = sans[debut..].find(fermante).map_or(sans.len(), |fin| debut + fin + fermante.len());
            sans.replace_range(debut..fin, " ");
        }
    }
    let mut visible = String::new();
    let mut dans_une_balise = false;
    for c in sans.chars() {
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

/// Le texte d'un élément simple (une balise ouvrante `<nom … id="identifiant"…>` jusqu'à `</nom>`), sans ses balises
/// intérieures ; `None` si la page n'a pas cet élément.
fn texte_de_l_element(html: &str, nom: &str, identifiant: &str) -> Option<String> {
    let mut debut = 0;
    while let Some(position) = html[debut..].find(&format!("<{nom}")) {
        let ouvrante = debut + position;
        let fin_ouvrante = ouvrante + html[ouvrante..].find('>')?;
        if attribut(&html[ouvrante + 1..fin_ouvrante], "id").as_deref() == Some(identifiant) {
            let fermante = fin_ouvrante + html[fin_ouvrante..].find(&format!("</{nom}>"))?;
            return Some(texte_visible(&html[fin_ouvrante + 1..fermante]));
        }
        debut = fin_ouvrante;
    }
    None
}

/// 9. Un menu `<select id="scenario">` étiqueté par un `<label for="scenario">`, avec six `<option>` dont les `value`
///    sont les six noms de l'adresse, chacun une fois, chacune avec un libellé qui se lit. Une `<option>` de plus ou de
///    moins, un nom différent, un menu qui n'est pas un `select` : le test échoue.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_page_a_un_menu_de_six_scenarios_etiquete() {
    let html = lire_page("index.html");
    let toutes = balises(&html);

    let select = toutes
        .iter()
        .find(|balise| attribut(balise, "id").as_deref() == Some("scenario"))
        .expect("index.html n'a pas d'élément d'id « scenario »");
    assert!(select.starts_with("select"), "l'élément d'id « scenario » n'est pas un <select> : <{select}>");
    assert!(
        toutes.iter().any(|b| b.starts_with("label") && attribut(b, "for").as_deref() == Some("scenario")),
        "le menu n'a pas de <label for=\"scenario\">"
    );

    // Les options sont celles qui se trouvent entre `<select id="scenario"` et le `</select>` qui suit.
    let debut = html.find("<select").expect("un <select>");
    let fin = html[debut..].find("</select>").map(|fin| debut + fin).expect("un </select>");
    let interieur = &html[debut..fin];
    let options: Vec<&str> = balises(interieur).into_iter().filter(|b| b.starts_with("option")).collect();
    let valeurs: Vec<String> = options.iter().map(|o| attribut(o, "value").expect("une <option> sans value")).collect();
    assert_eq!(valeurs.len(), 6, "le menu a {} options : {valeurs:?}", valeurs.len());
    assert_eq!(
        valeurs.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        SCENARIOS.iter().map(|s| s.nom).collect::<BTreeSet<_>>(),
        "les valeurs du menu ne sont pas les six noms de l'adresse"
    );
    // Chaque option porte un libellé : le texte qui suit sa balise, jusqu'à la suivante.
    for morceau in interieur.split("<option").skip(1) {
        let libelle = texte_visible(&format!("<x{}", morceau.split("</option>").next().unwrap_or("")));
        assert!(!libelle.trim().is_empty(), "une option sans libellé : « {morceau} »");
    }
}

/// 9 (suite). Un paragraphe `<p id="description-scenario">` existe et dit quelque chose ; `app.js` demande le menu et
/// ce paragraphe par `getElementById`, et le menu et les six noms ne sont pas seulement dans le HTML : sans cela, le
/// menu ne ferait rien.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_page_decrit_le_scenario_choisi_et_app_js_s_en_sert() {
    let html = lire_page("index.html");
    let description = texte_de_l_element(&html, "p", "description-scenario")
        .expect("index.html n'a pas de <p id=\"description-scenario\">");
    assert!(!description.trim().is_empty(), "la description du scénario est vide au départ");

    let demandes = identifiants_demandes(&sans_commentaires(&lire_page("app.js")));
    for identifiant in ["scenario", "description-scenario"] {
        assert!(demandes.iter().any(|d| d == identifiant), "app.js ne demande pas l'élément « {identifiant} »");
    }
}

/// 9 (suite). La page dit, sur la page et non dans un commentaire ni dans le CSS, que le banc est **en 2D** et **ne
/// prédit pas la résistance d'un vrai matériau**.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_page_dit_qu_elle_est_en_2d_et_ne_predit_pas_la_resistance_d_un_materiau() {
    let visible = texte_visible(&lire_page("index.html"));
    assert!(visible.contains("2D"), "la page ne dit pas qu'elle est en 2D");
    assert!(
        visible.to_lowercase().contains("ne prédit pas la résistance d'un vrai matériau"),
        "la page ne dit pas qu'elle ne prédit pas la résistance d'un vrai matériau"
    );
}

/// Les identifiants que la page doit avoir et qu'`app.js` doit demander : ceux d'avant l'étape 8 (le bouton du pas de
/// référence s'appelle désormais `pas-reference`) et ceux du menu, de sa description et des deux légendes. Le faux
/// navigateur de `page-wasm.mjs` les emploie.
const IDENTIFIANTS_DE_LA_PAGE: [&str; 20] = [
    "boite",
    "etat",
    "legende-tache",
    "legende-obstacle",
    "scenario",
    "description-scenario",
    "graine",
    "hasard",
    "relancer",
    "pause",
    "pas-reference",
    "vitesse",
    "vitesse-valeur",
    "message",
    "temps",
    "biomasse",
    "pointes",
    "anastomoses",
    "empreinte",
    "note-empreinte",
];

/// 9 (suite). Les identifiants de la page : chacun n'y est qu'une fois (deux éléments du même `id` rendraient
/// `getElementById` ambigu), ceux de `IDENTIFIANTS_DE_LA_PAGE` existent et `app.js` les demande tous, l'ancien `pas80`
/// n'existe plus et `pas-reference` est un `<button>`. L'accessibilité : les deux zones où la page écrit ce qui
/// arrive (`etat`, qui porte le message d'arrêt, et `message`) sont annoncées aux lecteurs d'écran (`role="status"`
/// et `aria-live="polite"`) ; le menu est décrit par le paragraphe de description (`aria-describedby`).
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_page_a_ses_identifiants_une_fois_et_annonce_ses_messages() {
    let html = lire_page("index.html");
    let toutes = balises(&html);
    let identifiants: Vec<String> = toutes.iter().filter_map(|balise| attribut(balise, "id")).collect();
    let uniques: BTreeSet<&str> = identifiants.iter().map(String::as_str).collect();
    assert_eq!(uniques.len(), identifiants.len(), "un identifiant est employé deux fois dans index.html");

    let demandes = identifiants_demandes(&sans_commentaires(&lire_page("app.js")));
    for identifiant in IDENTIFIANTS_DE_LA_PAGE {
        assert!(uniques.contains(identifiant), "index.html n'a pas d'élément d'id « {identifiant} »");
        assert!(demandes.iter().any(|d| d == identifiant), "app.js ne demande pas l'élément « {identifiant} »");
    }
    assert!(!uniques.contains("pas80"), "l'ancien identifiant « pas80 » est resté");

    let element = |identifiant: &str| {
        *toutes
            .iter()
            .find(|balise| attribut(balise, "id").as_deref() == Some(identifiant))
            .unwrap_or_else(|| panic!("index.html n'a pas d'élément d'id « {identifiant} »"))
    };
    assert!(element("pas-reference").starts_with("button"), "« pas-reference » n'est pas un <button>");
    for identifiant in ["etat", "message"] {
        let balise = element(identifiant);
        assert_eq!(attribut(balise, "role").as_deref(), Some("status"), "« {identifiant} » : role");
        assert_eq!(attribut(balise, "aria-live").as_deref(), Some("polite"), "« {identifiant} » : aria-live");
    }
    assert_eq!(
        attribut(element("scenario"), "aria-describedby").as_deref(),
        Some("description-scenario"),
        "le menu n'est pas décrit par le paragraphe de description"
    );
}

/// 9 (suite). Rien de ce que lit la page (l'adresse, le champ de graine) n'est interprété comme du HTML ou du code :
/// ni `app.js` ni `adresse.js` n'emploient `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval` ou
/// `new Function` (la page n'écrit que par `textContent`), et `index.html` n'a aucun attribut de gestionnaire
/// d'événement (`onclick=`, `onload=`…). Le faux navigateur de `page-wasm.mjs` refuse en plus toute écriture dans
/// `innerHTML` et vérifie qu'une adresse piégée ne se retrouve dans aucun texte de la page.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_page_n_interprete_rien_comme_du_html() {
    for nom in ["app.js", "adresse.js"] {
        let code = sans_commentaires(&lire_page(nom));
        for interdit in
            ["innerHTML", "outerHTML", "insertAdjacentHTML", "document.write", "eval(", "new Function", "srcdoc"]
        {
            assert!(!code.contains(interdit), "docs/{nom} emploie « {interdit} »");
        }
    }
    let html = lire_page("index.html");
    for balise in balises(&html) {
        for mot in balise.split_whitespace().skip(1) {
            assert!(
                !(mot.to_lowercase().starts_with("on") && mot.contains('=')),
                "index.html a un gestionnaire d'événement dans une balise : <{balise}>"
            );
        }
    }
}

/// Les règles CSS d'une feuille : pour chacune, sa liste de sélecteurs et ses déclarations. Les commentaires sont
/// ôtés, et les règles écrites dans un `@media` sont lues comme les autres.
fn regles_css(feuille: &str) -> Vec<(String, String)> {
    let mut sans_commentaires = String::new();
    let mut reste = feuille;
    while let Some(debut) = reste.find("/*") {
        sans_commentaires.push_str(&reste[..debut]);
        reste = reste[debut..].find("*/").map_or("", |fin| &reste[debut + fin + 2..]);
    }
    sans_commentaires.push_str(reste);

    let mut regles = Vec::new();
    let mut pile: Vec<String> = Vec::new();
    let mut courant = String::new();
    for c in sans_commentaires.chars() {
        match c {
            '{' => {
                pile.push(courant.trim().to_string());
                courant.clear();
            }
            '}' => {
                let selecteur = pile.pop().unwrap_or_default();
                // Un bloc qui contient des déclarations (et non d'autres règles) est une règle ; un `@media` ne
                // contient que des règles, que la boucle a déjà vues.
                if !selecteur.starts_with('@') {
                    regles.push((selecteur, courant.trim().to_string()));
                }
                courant.clear();
            }
            _ => courant.push(c),
        }
    }
    regles
}

/// 9 (suite). Le message d'arrêt (« La colonie a fini de pousser… », « Limite de la vitrine atteinte… ») s'écrit dans
/// l'élément d'état : cet élément vient **après** le canvas dans la page, et aucune règle CSS qui le vise (par son
/// identifiant, sa classe ou sa balise) ne le sort du flux (`position: absolute` ou `fixed`) : il se lit sous l'image,
/// hors de la boîte, sans la couvrir. Ce test lit le texte de la page ; la mise en page vraie se regarde dans un
/// navigateur.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn le_message_d_arret_est_sous_l_image_et_hors_du_flux_absolu() {
    let html = lire_page("index.html");
    let identifiant = identifiants_demandes(&sans_commentaires(&lire_page("app.js")))
        .into_iter()
        .find(|i| i == "etat")
        .expect("app.js n'écrit plus dans l'élément d'état (id « etat »)");

    let toutes = balises(&html);
    let etat = toutes
        .iter()
        .find(|b| attribut(b, "id").as_deref() == Some(identifiant.as_str()))
        .expect("index.html n'a pas d'élément d'état");
    let canvas = html.to_lowercase().find("<canvas").expect("un <canvas");
    let position_etat = html.find(&format!("id=\"{identifiant}\"")).expect("l'élément d'état");
    assert!(position_etat > canvas, "l'élément d'état est avant le canvas : il ne peut pas être sous l'image");

    let balise = etat.split_whitespace().next().unwrap_or("").to_lowercase();
    let classes: Vec<String> =
        attribut(etat, "class").unwrap_or_default().split_whitespace().map(str::to_string).collect();
    let debut = html.find("<style").expect("une feuille de style");
    let fin = html[debut..].find("</style>").map(|fin| debut + fin).expect("fin de la feuille de style");
    for (selecteurs, declarations) in regles_css(&html[debut..fin]) {
        let vise = selecteurs.split(',').any(|selecteur| {
            let selecteur = selecteur.trim();
            selecteur.contains(&format!("#{identifiant}"))
                || classes.iter().any(|classe| selecteur.contains(&format!(".{classe}")))
                // Le dernier élément du sélecteur est la balise de l'élément : `figure figcaption`, `figcaption`.
                || selecteur.split_whitespace().last().is_some_and(|dernier| dernier == balise)
        });
        if !vise {
            continue;
        }
        let minuscules = declarations.to_lowercase().replace(' ', "");
        assert!(
            !minuscules.contains("position:absolute") && !minuscules.contains("position:fixed"),
            "la règle « {selecteurs} » sort le message d'arrêt du flux : {declarations}"
        );
    }
}

/// 9 (suite). Si `adresse.js` existe, `app.js` l'importe (sinon le fichier est mort) ; `adresse.js` n'emploie aucun nom
/// `mycelium_` (il ne touche pas au module) ; il ne contient aucune ressource externe ; et c'est du JavaScript valide,
/// comme module (`node --check`).
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn adresse_js_est_importe_valide_et_sans_ressource_externe() {
    let chemin = racine().join("docs").join("adresse.js");
    assert!(chemin.exists(), "docs/adresse.js n'existe pas : ce prototype l'a, les tests de l'adresse en dépendent");
    let app = sans_commentaires(&lire_page("app.js"));
    assert!(
        app.contains("from \"./adresse.js\"") || app.contains("from './adresse.js'"),
        "app.js n'importe pas adresse.js"
    );
    let adresse = lire_page("adresse.js");
    assert!(!sans_commentaires(&adresse).contains("mycelium_"), "adresse.js emploie le module WebAssembly");

    let interdits = [
        "http://",
        "https://",
        "//cdn",
        "\"//",
        "'//",
        "`//",
        "(//",
        "=//",
        "unpkg",
        "jsdelivr",
        "cdnjs",
        "googleapis",
    ];
    for interdit in interdits {
        assert!(!adresse.to_lowercase().contains(interdit), "docs/adresse.js contient « {interdit} »");
    }

    if !node_est_la() {
        return;
    }
    let mut node = Command::new("node")
        .args(["--input-type=module", "--check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node se lance");
    node.stdin.take().expect("entrée standard de Node").write_all(adresse.as_bytes()).expect("écriture d'adresse.js");
    let sortie = node.wait_with_output().expect("Node se termine");
    assert!(sortie.status.success(), "adresse.js n'est pas valide : {}", String::from_utf8_lossy(&sortie.stderr));
}

/// Un cas de `adresse.mjs` : la ligne qu'on lui envoie, et la ligne qu'il doit répondre.
fn cas(entree: &str, sortie: &str) -> (String, String) {
    (entree.to_string(), sortie.to_string())
}

/// Les cas de `adresse.js`, qu'`adresse.mjs` joue (voir son en-tête pour le protocole).
fn cas_de_l_adresse() -> Vec<(String, String)> {
    let mut cas_ = Vec::new();

    // Les noms, dans l'ordre des numéros ; la plus grande graine.
    cas_.push(cas("noms", "libre,terrain,tache,tache-sans,mur,mur-sans"));
    cas_.push(cas("max", "4294967295"));

    // Un nom donne son numéro, celui que prend le module ; tout autre nom, même un nom que porte tout objet
    // JavaScript, donne -1.
    for scenario in &SCENARIOS {
        cas_.push(cas(&format!("numero\t{}", scenario.nom), &scenario.numero.to_string()));
    }
    for inconnu in
        ["Mur", "MUR", "", " mur", "mur ", "constructor", "__proto__", "toString", "hasOwnProperty", "length", "0", "1"]
    {
        cas_.push(cas(&format!("numero\t{inconnu}"), "-1"));
    }

    // Ce qui est une graine : des chiffres décimaux de 0 à 4294967295, rien d'autre.
    for (texte, lue) in [("0", "0"), ("1", "1"), ("12", "12"), ("00012", "12"), ("4294967295", "4294967295")] {
        cas_.push(cas(&format!("graine\t{texte}"), lue));
    }
    for invalide in [
        "",
        "abc",
        "-1",
        "-0",
        "+5",
        "1.5",
        "1e3",
        "0x10",
        " 5",
        "5 ",
        "1_0",
        "4294967296",
        "4294967297",
        "9007199254740993",
        "99999999999999999999",
        "٣",
        "５",
    ] {
        cas_.push(cas(&format!("graine\t{invalide}"), "null"));
    }

    // L'adresse écrite, et relue : le tour complet pour chaque nom et des graines aux limites.
    for scenario in &SCENARIOS {
        for graine in [0u32, 1, 31, 0xDEAD_BEEF, u32::MAX] {
            let adresse = format!("#scenario={}&graine={graine}", scenario.nom);
            cas_.push(cas(&format!("ecrire\t{}\t{graine}", scenario.nom), &adresse));
            cas_.push(cas(&format!("lire\t{adresse}\t7"), &format!("{}\t{graine}", scenario.nom)));
        }
    }

    // La lecture : le dièse est facultatif, l'ordre des paramètres libre, les paramètres inconnus sont ignorés.
    let lectures: Vec<(&str, u32, &str, u32)> = vec![
        ("#scenario=mur&graine=31", 1, "mur", 31),
        ("scenario=mur&graine=31", 1, "mur", 31),
        ("#graine=12&scenario=mur-sans", 9, "mur-sans", 12),
        ("#foo=bar&scenario=tache&graine=3&x", 9, "tache", 3),
        ("#scenario=tache&graine=3&", 9, "tache", 3),
        // Absent : le scénario libre, la graine par défaut (le champ), même quand elle vaut 0.
        ("", 5, "libre", 5),
        ("#", 5, "libre", 5),
        ("#scenario=terrain", 9, "terrain", 9),
        ("#scenario=mur", 0, "mur", 0),
        ("#graine=12", 9, "libre", 12),
        ("#graine=0", 9, "libre", 0),
        ("#scenario=&graine=", 9, "libre", 9),
        // Une graine aux limites.
        ("#graine=4294967295", 9, "libre", 4294967295),
        ("#graine=4294967296", 9, "libre", 9),
        ("#graine=007", 9, "libre", 7),
        // Une graine invalide laisse la graine par défaut, jusqu'à son maximum.
        ("#scenario=mur&graine=abc", 4294967295, "mur", 4294967295),
        ("#scenario=mur&graine=-1", 9, "mur", 9),
        ("#scenario=mur&graine=1.5", 9, "mur", 9),
        ("#scenario=mur&graine=1e3", 9, "mur", 9),
        ("#scenario=mur&graine=0x10", 9, "mur", 9),
        ("#scenario=mur&graine=%20", 9, "mur", 9),
        ("#scenario=mur&graine=99999999999999999999", 9, "mur", 9),
        ("#scenario=mur&graine=+5", 9, "mur", 9),
        ("#scenario=mur&graine=5%20", 9, "mur", 9),
        // Une clé qui revient garde sa première valeur, même invalide (tranché, voir la spécification).
        ("#scenario=mur&scenario=tache&graine=3", 9, "mur", 3),
        ("#scenario=inconnu&scenario=tache&graine=3", 9, "libre", 3),
        ("#graine=3&graine=5&scenario=mur", 9, "mur", 3),
        ("#graine=abc&graine=5&scenario=mur", 9, "mur", 9),
        // Une seule `#` s'ôte ; une clé n'est reconnue qu'écrite exactement (ni casse, ni espace, ni `%xx`).
        ("##scenario=mur&graine=3", 9, "libre", 3),
        ("#Scenario=mur&Graine=3", 9, "libre", 9),
        ("#scenario =mur&graine =3", 9, "libre", 9),
        ("#scenario=mur=x&graine=3", 9, "libre", 3),
        ("#scenario&graine", 9, "libre", 9),
        ("&&&", 9, "libre", 9),
        // Une adresse piégée : rien n'en est repris.
        ("#scenario=<img src=x onerror=alert(1)>&graine=5", 9, "libre", 5),
        ("#scenario=mur\"><script>&graine=<b>1</b>", 9, "libre", 9),
    ];
    for (fragment, defaut, scenario, graine) in lectures {
        cas_.push(cas(&format!("lire\t{fragment}\t{defaut}"), &format!("{scenario}\t{graine}")));
    }
    // Un nom inconnu donne « libre », y compris les noms que porte tout objet JavaScript et les noms de casse
    // différente ; la graine, elle, est lue.
    for inconnu in [
        "inconnu",
        "Mur",
        "MUR",
        "%20mur",
        "mur%20",
        "mur-",
        "mur%2Dsans",
        "tache_sans",
        "constructor",
        "__proto__",
        "toString",
        "hasOwnProperty",
        "valueOf",
        "0",
        "1",
        "libre%20",
    ] {
        cas_.push(cas(&format!("lire\t#scenario={inconnu}&graine=5\t4"), "libre\t5"));
        cas_.push(cas(&format!("lire\t#scenario={inconnu}\t4"), "libre\t4"));
    }
    cas_
}

/// 9 (suite). Les fonctions pures de `adresse.js` sous Node : l'aller-retour pour chaque nom et des graines aux limites,
/// les noms inconnus (dont ceux que porte tout objet JavaScript), la graine invalide ou trop grande, les valeurs
/// limites (0, 4294967295, 4294967296), et le numéro de chaque nom (celui que prend le module : c'est ce qui fait que le
/// menu choisit le bon fichier).
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn adresse_js_lit_et_ecrit_l_adresse() {
    if !node_est_la() {
        return;
    }
    let cas = cas_de_l_adresse();
    let entree: String = cas.iter().map(|(ligne, _)| format!("{ligne}\n")).collect();
    let dossier = dossier_de_travail();
    let docs = racine().join("docs");
    let sortie = lancer_le_script_avec_entree(
        "adresse.mjs",
        &[docs.to_str().expect("chemin en UTF-8"), dossier.to_str().expect("chemin en UTF-8")],
        &entree,
    );
    assert!(sortie.status.success(), "adresse.mjs : {}", String::from_utf8_lossy(&sortie.stderr));
    let texte = String::from_utf8_lossy(&sortie.stdout);
    let reponses: Vec<&str> = texte.lines().collect();
    assert_eq!(reponses.len(), cas.len(), "adresse.mjs a répondu {} lignes pour {} cas", reponses.len(), cas.len());
    for ((ligne, attendue), obtenue) in cas.iter().zip(&reponses) {
        assert_eq!(obtenue, attendue, "pour « {} »", ligne.replace('\t', " | "));
    }
    assert!(cas.len() > 150, "il y a {} cas : le test a perdu des cas", cas.len());
}

/// 9 (suite). `adresse.mjs` refuse une commande inconnue et des arguments faux (code 1, rien sur la sortie standard,
/// un message d'erreur), au lieu de répondre n'importe quoi.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn adresse_mjs_refuse_une_commande_inconnue() {
    if !node_est_la() {
        return;
    }
    let dossier = dossier_de_travail();
    let docs = racine().join("docs");
    let (docs, dossier) = (docs.to_str().expect("chemin en UTF-8"), dossier.to_str().expect("chemin en UTF-8"));
    let sortie = lancer_le_script_avec_entree("adresse.mjs", &[docs, dossier], "lire\t#\t1\nbidule\tx\n");
    assert_eq!(sortie.status.code(), Some(1));
    assert!(sortie.stdout.is_empty());
    // C'est bien la commande qui est refusée (et non un fichier absent) : le message la nomme.
    let erreur = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreur.contains("commande inconnue") && erreur.contains("bidule"), "message : « {erreur} »");
    let sortie = lancer_le_script_avec_entree("adresse.mjs", &[docs], "");
    assert_eq!(sortie.status.code(), Some(1));
    assert!(sortie.stdout.is_empty());
    let erreur = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreur.contains("usage"), "message : « {erreur} »");
}

/// 9 (suite). La page entière, jouée dans un faux navigateur avec le vrai module (`page-wasm.mjs`) : l'adresse lue à
/// l'ouverture (valide, inconnue, invalide), le menu qui choisit le scénario et redémarre, « Relancer » et « Au hasard »
/// qui démarrent le scénario choisi, « Jusqu'au pas N » avec N le pas de référence du scénario, les obstacles dessinés
/// (leur taille en cases, bords inclus), le message d'arrêt, l'adresse écrite à chaque démarrage. À chaque fois l'état du
/// module est comparé à celui d'une autre instance pilotée directement. Le script écrit `ok <n>`, n étant le nombre de
/// vérifications faites : un test qui ne vérifie presque rien échoue.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_page_jouee_dans_un_faux_navigateur() {
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
    let nombre: u32 = texte
        .trim_end()
        .strip_prefix("ok ")
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("page-wasm.mjs n'a pas écrit « ok <n> » : « {texte} »"));
    assert!(nombre >= 1000, "page-wasm.mjs n'a fait que {nombre} vérifications");
}
