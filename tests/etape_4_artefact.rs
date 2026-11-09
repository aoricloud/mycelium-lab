//! Tests d'acceptation de l'étape 4 (b) : l'artefact commité, c'est-à-dire le module `docs/mycelium.wasm` et la page
//! `docs/index.html` + `docs/app.js`.
//!
//! La CI n'a pas la cible WebAssembly : le module est construit à la main puis commité. La preuve qu'il est bon est
//! qu'il tourne, sous Node, et donne la même empreinte que le code natif. Si le code change sans que le module soit
//! reconstruit, les tests 5 échouent.
//!
//! 4. la forme du module, lue par un petit lecteur du format binaire WebAssembly écrit dans ce fichier ;
//! 5. le résultat du module, lu par Node (`tests/etape_4/empreinte-wasm.mjs` pour l'empreinte,
//!    `tests/etape_4/etat-wasm.mjs` pour ce que la page lit, par les accesseurs) et comparé au code natif ;
//! 6. la page : ses fichiers, ses liens avec le module, et l'absence de toute ressource externe.
//!
//! Ces tests ne touchent pas à l'état global de la vitrine (ils ne comparent qu'à `Simulation`) : ils n'ont pas
//! besoin du verrou de `etape_4_vitrine.rs`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use mycelium_lab::empreinte;
use mycelium_lab::simulation::{Simulation, lire_scenario};

/// Le scénario que fait tourner la vitrine.
const SCENARIO: &str = include_str!("../scenarios/couplage.txt");

/// Le plafond du module : 1 Mo.
const TAILLE_MAX_MODULE: u64 = 1024 * 1024;

/// Le plafond de la page, tous fichiers de `docs/` confondus sauf le module : 200 Ko.
const TAILLE_MAX_PAGE: u64 = 200 * 1024;

/// Les types de valeurs WebAssembly qui servent ici, tels que le format binaire les code.
const I32: u8 = 0x7F;
const F64: u8 = 0x7C;

/// Les fonctions que le module doit exporter, avec leur signature : types des paramètres, types des résultats
/// (`u32` est un `i32` en WebAssembly).
const FONCTIONS: [(&str, &[u8], &[u8]); 12] = [
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
];

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

/// Le script qui lit l'empreinte du module, et celui qui lit son état par les accesseurs.
const SCRIPT_EMPREINTE: &str = "empreinte-wasm.mjs";
const SCRIPT_ETAT: &str = "etat-wasm.mjs";

fn chemin_script(nom: &str) -> PathBuf {
    racine().join("tests").join("etape_4").join(nom)
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
// de fonctions et d'exportations.
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

/// Analyse un module : l'en-tête (`\0asm` puis la version 1), puis les sections repérées par leur numéro. Les
/// sections qui ne servent pas ici (personnalisées, mémoire, code, données…) sont sautées sans être lues.
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

/// 4. Le fichier est un module WebAssembly en version 1, non vide, et il pèse moins de 1 Mo.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn le_module_est_du_webassembly_et_leger() {
    let octets = lire_module();
    analyser(&octets);
    assert!((octets.len() as u64) < TAILLE_MAX_MODULE, "{} octets", octets.len());
}

/// 4 (suite). Le module n'importe rien : ni fonction, ni mémoire, ni table, ni globale. Il est donc instanciable
/// avec des importations vides, et la page n'a rien à lui fournir.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn le_module_n_importe_rien() {
    let module = analyser(&lire_module());
    assert_eq!(module.importations, 0, "le module a {} importation(s)", module.importations);
}

/// 4 (suite). Le module exporte exactement les fonctions `mycelium_*` de la liste, ni plus ni moins ; la mémoire
/// `memory` est tolérée, rien d'autre (aucune autre exportation, de quelque genre que ce soit).
///
/// L'étape 8 ajoute quatre exportations : avec `etape-8`, ce test n'est pas compilé et c'est
/// `le_module_exporte_les_douze_fonctions_de_l_etape_4_et_les_quatre_nouvelles` (`etape_8_artefact.rs`) qui fixe la
/// liste exacte, ces douze et les quatre nouvelles.
#[cfg(not(feature = "etape-8"))]
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn le_module_exporte_exactement_les_fonctions_de_la_vitrine() {
    let module = analyser(&lire_module());

    let fonctions: BTreeSet<&str> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.as_str()).collect();
    let attendues: BTreeSet<&str> = FONCTIONS.iter().map(|(nom, _, _)| *nom).collect();
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

/// 4 (suite). Chaque fonction exportée a la bonne signature : le nombre et le type des paramètres, et du résultat
/// (`i32` pour un `u32`, `f64` pour un `f64`).
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
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

/// Lance ce script avec ces arguments.
fn lancer_le_script(script: &str, arguments: &[&str]) -> Output {
    Command::new("node")
        .arg(chemin_script(script))
        .args(arguments)
        .output()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"))
}

/// La simulation native en face : le scénario embarqué, cette graine, ce nombre de pas.
fn simulation_native(graine: u32, pas: u32) -> Simulation {
    let (reglages, _duree) = lire_scenario(SCENARIO).expect("le scénario est valide");
    let mut simulation = Simulation::nouvelle(reglages, u64::from(graine)).expect("les réglages sont valides");
    simulation.avancer_n(pas);
    simulation
}

/// L'empreinte que calcule le code natif.
fn empreinte_native(graine: u32, pas: u32) -> String {
    simulation_native(graine, pas).empreinte()
}

/// Les octets en hexadécimal, dans l'ordre.
fn hexadecimal(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

/// Ce que `etat-wasm.mjs` doit écrire : l'état de la simulation native, lu comme le script le lit.
fn etat_attendu(graine: u32, pas: u32) -> String {
    let simulation = simulation_native(graine, pas);
    let substrat = simulation.substrat();

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

    format!(
        "temps={}\nlargeur={}\nhauteur={}\nsegments={}\npointes={}\nanastomoses={}\nbiomasse={}\n\
         hache_segments={}\nhache_substrat={}\nbords=1\n",
        simulation.temps(),
        substrat.largeur(),
        substrat.hauteur(),
        simulation.segments().len(),
        simulation.pointes(),
        simulation.anastomoses(),
        hexadecimal(&simulation.biomasse().to_le_bytes()),
        empreinte::sha256_hex(&segments),
        empreinte::sha256_hex(&cases),
    )
}

/// 5. Le module, chargé par Node (`WebAssembly.instantiate`, importations vides), donne la même empreinte que le
///    code natif : pour les graines 1 et 2 à 80 pas, pour une grande graine, pour 0 pas (l'état initial), pour 1 pas
///    et pour une colonie arrivée au bout de sa croissance (graine 7, 300 pas : les pointes se sont éteintes contre
///    les bords). Si le code change sans que le module soit reconstruit, ce test échoue. La sortie du script est
///    exactement les 64 caractères et un saut de ligne.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn node_donne_l_empreinte_du_code_natif() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");

    for (graine, pas) in [(1, 80), (2, 80), (0xDEAD_BEEF, 30), (u32::MAX, 5), (1, 0), (3, 1), (7, 300)] {
        let sortie = lancer_le_script(SCRIPT_EMPREINTE, &[module, &graine.to_string(), &pas.to_string()]);
        let erreur = String::from_utf8_lossy(&sortie.stderr);
        assert!(
            sortie.status.success(),
            "graine {graine}, {pas} pas : {:?}, sortie d'erreur : {erreur}",
            sortie.status
        );
        let attendu = format!("{}\n", empreinte_native(graine, pas));
        assert_eq!(String::from_utf8_lossy(&sortie.stdout), attendu, "graine {graine}, {pas} pas");
    }
}

/// 5 (suite). Pour la graine 1 et 80 pas, la sortie du script est le contenu du fichier de référence du dépôt.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn node_redonne_la_reference_du_depot() {
    if !node_est_la() {
        return;
    }
    let reference = std::fs::read_to_string(racine().join("references").join("couplage-graine-1.empreinte"))
        .expect("la référence se lit");
    let module = chemin_module();
    let sortie = lancer_le_script(SCRIPT_EMPREINTE, &[module.to_str().expect("chemin en UTF-8"), "1", "80"]);
    assert!(sortie.status.success());
    assert_eq!(String::from_utf8_lossy(&sortie.stdout).trim_end(), reference.trim_end());
}

/// 5 (suite). Le module, lu comme la page le lit, par ses accesseurs et non par l'empreinte : le temps, la boîte, les
/// compteurs, la biomasse (bit à bit), tous les segments et toutes les cases du substrat (par leur SHA-256) sont ceux
/// du code natif, et les accesseurs rendent `NaN` (et 0 pour le mot 8) hors de portée. L'empreinte seule ne dit rien
/// des accesseurs : un module construit avant la correction d'un accesseur (`mycelium_segment`, `mycelium_substrat`…)
/// a la même empreinte et dessinerait n'importe quoi. Le script `etat-wasm.mjs` écrit une ligne `clé=valeur` par
/// grandeur ; ce test compare sa sortie, entière, à celle que le code natif donne.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn node_lit_l_etat_comme_la_page() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");

    for (graine, pas) in [(1, 80), (2, 80), (0xDEAD_BEEF, 30), (3, 0)] {
        let sortie = lancer_le_script(SCRIPT_ETAT, &[module, &graine.to_string(), &pas.to_string()]);
        let erreur = String::from_utf8_lossy(&sortie.stderr);
        assert!(
            sortie.status.success(),
            "graine {graine}, {pas} pas : {:?}, sortie d'erreur : {erreur}",
            sortie.status
        );
        assert_eq!(String::from_utf8_lossy(&sortie.stdout), etat_attendu(graine, pas), "graine {graine}, {pas} pas");
    }
}

/// 5 (suite). Les deux scripts refusent des arguments faux avec le code de sortie 1 et un message sur la sortie
/// d'erreur, sans rien écrire sur la sortie standard : fichier absent, fichier qui n'est pas un module, trop ou trop
/// peu d'arguments, graine ou nombre de pas qui n'est pas un entier de 32 bits non signé.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn les_scripts_refusent_des_arguments_faux() {
    if !node_est_la() {
        return;
    }
    let module = chemin_module();
    let module = module.to_str().expect("chemin en UTF-8");
    let absent = racine().join("docs").join("n-existe-pas.wasm");
    let absent = absent.to_str().expect("chemin en UTF-8");
    let pas_un_module = racine().join("Cargo.toml");
    let pas_un_module = pas_un_module.to_str().expect("chemin en UTF-8");

    let cas: [&[&str]; 9] = [
        &[absent, "1", "80"],
        &[pas_un_module, "1", "80"],
        &[],
        &[module],
        &[module, "1"],
        &[module, "1", "80", "en trop"],
        &[module, "abc", "80"],
        &[module, "1", "-1"],
        &[module, "4294967296", "80"],
    ];
    for script in [SCRIPT_EMPREINTE, SCRIPT_ETAT] {
        for arguments in cas {
            let sortie = lancer_le_script(script, arguments);
            assert_eq!(sortie.status.code(), Some(1), "{script}, arguments {arguments:?}");
            assert!(sortie.stdout.is_empty(), "{script}, arguments {arguments:?} : la sortie standard n'est pas vide");
            assert!(!sortie.stderr.is_empty(), "{script}, arguments {arguments:?} : aucun message d'erreur");
        }
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

/// Le code de `docs/app.js`, sans ses commentaires.
fn lire_le_code_de_app_js() -> String {
    sans_commentaires(&lire_page("app.js"))
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

/// La valeur de l'attribut `id` d'une balise, s'il y en a un (écrit entre guillemets ou entre apostrophes).
fn attribut_id(balise: &str) -> Option<String> {
    for (position, _) in balise.match_indices("id=") {
        if !balise[..position].ends_with(char::is_whitespace) {
            continue;
        }
        let apres = &balise[position + "id=".len()..];
        let guillemet = apres.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let valeur = &apres[1..];
        return Some(valeur[..valeur.find(guillemet)?].to_string());
    }
    None
}

/// 6. `docs/index.html` existe, contient un `<canvas` et un `<script type="module"` qui charge `app.js`.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_page_a_un_canvas_et_charge_app_js() {
    let html = lire_page("index.html").to_lowercase();
    assert!(html.contains("<canvas"), "pas de <canvas dans index.html");

    let debut = html.find("<script type=\"module\"").expect("pas de <script type=\"module\" dans index.html");
    let fin = html[debut..].find('>').expect("balise <script> non fermée");
    let balise = &html[debut..debut + fin];
    assert!(
        balise.contains("src=") && balise.contains("app.js"),
        "le script de type module ne charge pas app.js : {balise}"
    );
}

/// 6 (suite). `docs/app.js` existe, désigne le module par le texte littéral `"mycelium.wasm"` (ou `"./mycelium.wasm"`),
/// c'est-à-dire par un chemin relatif à la page, qui est servie depuis `docs/` (jamais `/docs/mycelium.wasm`), et
/// l'instancie par `WebAssembly.instantiate` (ou `instantiateStreaming`, ou `new WebAssembly.Instance`).
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn app_js_charge_le_module() {
    let code = lire_le_code_de_app_js();
    let formes = ["mycelium.wasm", "./mycelium.wasm"];
    let litteral =
        |forme: &str| ["\"", "'", "`"].iter().any(|guillemet| code.contains(&format!("{guillemet}{forme}{guillemet}")));
    assert!(
        formes.iter().any(|forme| litteral(forme)),
        "app.js ne désigne pas le module par le texte \"mycelium.wasm\""
    );
    assert!(
        code.contains("WebAssembly.instantiate") || code.contains("new WebAssembly.Instance"),
        "app.js n'instancie pas le module"
    );
}

/// 6 (suite). `app.js` avance la simulation par `requestAnimationFrame` et dessine sur le canvas par son contexte 2D
/// (`getContext("2d")`).
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn app_js_anime_et_dessine_sur_le_canvas() {
    let code = lire_le_code_de_app_js();
    assert!(code.contains("requestAnimationFrame"), "app.js n'emploie pas requestAnimationFrame");
    assert!(
        code.contains("getContext(\"2d\")") || code.contains("getContext('2d')"),
        "app.js n'emploie pas le contexte 2D du canvas"
    );
}

/// 6 (suite). Chaque nom `mycelium_…` qu'emploie `app.js` est une fonction exportée par le module (test 4), et
/// chacune des fonctions exportées est employée : la page appelle tout ce que la vitrine expose, donc au moins
/// `mycelium_demarrer`, `mycelium_avancer`, `mycelium_nb_segments` et `mycelium_segment`, mais aussi le substrat et
/// l'empreinte qu'elle affiche. Les noms s'écrivent en entier dans le script, jamais composés.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn app_js_emploie_exactement_les_exportations_du_module() {
    let module = analyser(&lire_module());
    let exportees: BTreeSet<String> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.clone()).collect();
    let employes = noms_employes(&lire_le_code_de_app_js());

    let inconnus: Vec<&String> = employes.difference(&exportees).collect();
    assert!(inconnus.is_empty(), "app.js emploie des noms que le module n'exporte pas : {inconnus:?}");
    let oublies: Vec<&String> = exportees.difference(&employes).collect();
    assert!(oublies.is_empty(), "app.js n'emploie pas ces exportations du module : {oublies:?}");
    for nom in ["mycelium_demarrer", "mycelium_avancer", "mycelium_nb_segments", "mycelium_segment"] {
        assert!(employes.contains(nom), "app.js n'emploie pas {nom}");
    }
}

/// 6 (suite). Le canvas de la page porte un `id`, `app.js` le demande par `document.getElementById("…")`, et tous
/// les identifiants que `app.js` demande existent dans la page.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn app_js_ne_demande_que_des_elements_de_la_page() {
    let html = lire_page("index.html");
    let demandes = identifiants_demandes(&lire_le_code_de_app_js());
    assert!(!demandes.is_empty(), "app.js ne demande aucun élément par getElementById");

    let balises = balises(&html);
    let existants: BTreeSet<String> = balises.iter().filter_map(|balise| attribut_id(balise)).collect();
    for identifiant in &demandes {
        assert!(existants.contains(identifiant), "index.html n'a pas d'élément d'id « {identifiant} »");
    }

    let canvas = balises.iter().find(|balise| balise.to_lowercase().starts_with("canvas")).expect("un <canvas");
    let canvas = attribut_id(canvas).expect("le <canvas n'a pas d'attribut id");
    assert!(demandes.contains(&canvas), "app.js ne demande pas le canvas d'id « {canvas} »");
}

/// 6 (suite). `app.js` est du JavaScript valide (module) : `node --check`, le texte lui étant passé sur l'entrée
/// standard. Le contrôle est sauté, comme les autres, quand Node est absent hors de la CI.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn app_js_est_du_javascript_valide() {
    if !node_est_la() {
        return;
    }
    let script = lire_page("app.js");
    let mut node = Command::new("node")
        .args(["--input-type=module", "--check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node se lance");
    node.stdin.take().expect("entrée standard de Node").write_all(script.as_bytes()).expect("écriture de app.js");
    let sortie = node.wait_with_output().expect("Node se termine");
    assert!(sortie.status.success(), "app.js n'est pas valide : {}", String::from_utf8_lossy(&sortie.stderr));
}

/// 6 (suite). Ni `index.html` ni `app.js` ne contiennent `http://`, `https://` ni `//cdn` : aucune ressource
/// externe, ni script, ni police, ni image. Sont aussi refusés les adresses sans protocole (`"//…"`, `(//…)`,
/// `=//…`) et les noms des hébergeurs de bibliothèques et de polices les plus courants.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_page_n_a_aucune_ressource_externe() {
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
        "gstatic",
    ];
    for nom in ["index.html", "app.js"] {
        let texte = lire_page(nom).to_lowercase();
        for interdit in interdits {
            assert!(!texte.contains(interdit), "docs/{nom} contient « {interdit} » : une ressource externe ?");
        }
    }
}

/// Ajoute à `total` la taille des fichiers de `dossier`, sous-dossiers compris, sauf le module.
fn taille_sans_le_module(dossier: &Path, total: &mut u64) {
    let entrees = std::fs::read_dir(dossier).unwrap_or_else(|erreur| panic!("{} se lit : {erreur}", dossier.display()));
    for entree in entrees {
        let chemin = entree.expect("entrée de dossier").path();
        if chemin.is_dir() {
            taille_sans_le_module(&chemin, total);
        } else if chemin != chemin_module() {
            *total += std::fs::metadata(&chemin).expect("métadonnées").len();
        }
    }
}

/// 6 (suite). La page ne dépasse pas 200 Ko en tout : tous les fichiers de `docs/`, hors le module.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_page_ne_depasse_pas_200_ko() {
    let mut total = 0;
    taille_sans_le_module(&racine().join("docs"), &mut total);
    assert!(total > 0, "docs/ est vide");
    assert!(total <= TAILLE_MAX_PAGE, "la page pèse {total} octets");
}
