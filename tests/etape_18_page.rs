//! Tests d'acceptation de l'étape 18 (d) : la page du cube, `docs/cube.html` + `docs/cube.js`.
//!
//! La page 2D a ses tests d'artefact (`etape_4_artefact.rs`) ; ceux-ci font pour la page du cube ce qu'ils font pour
//! elle, sans rien demander de plus au module que ce que ses propres tests d'artefact vérifient déjà :
//!
//! 1. la forme de `cube.html` : en français, lisible sur un téléphone, deux canvas `volume` et `coupe`, et un script
//!    module qui charge `cube.js` ; ni la page ni son script ne nomment une ressource externe ;
//! 2. chaque nom `mycelium_…` que `cube.js` emploie est une exportation du module, et le script emploie les onze
//!    fonctions `mycelium_cube_*` ;
//! 3. `cube.js` ne charge que `mycelium.wasm`, par un seul `fetch` ;
//! 4. chaque `getElementById("…")` de `cube.js` a son élément dans `cube.html` ;
//! 5. les deux pages se lient l'une à l'autre ;
//! 6. `cube.js` est du JavaScript valide, quand Node est là ;
//! 7. la page entière tient sous le plafond de 200 Ko de `etape_4_artefact.rs`, module excepté.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Le plafond de la page, tous fichiers de `docs/` confondus sauf le module : 200 Ko (le même que
/// `etape_4_artefact.rs`).
const TAILLE_MAX_PAGE: u64 = 200 * 1024;

/// Les onze fonctions `mycelium_cube_*` que la page du cube emploie : le moteur calcule, la page dessine.
const FONCTIONS_EMPLOYEES_PAR_CUBE_JS: [&str; 11] = [
    "mycelium_cube_choisir",
    "mycelium_cube_demarrer",
    "mycelium_cube_avancer",
    "mycelium_cube_temps",
    "mycelium_cube_duree",
    "mycelium_cube_dimension",
    "mycelium_cube_valeur",
    "mycelium_cube_bloquee",
    "mycelium_cube_maximum",
    "mycelium_cube_masse",
    "mycelium_cube_empreinte_mot",
];

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn chemin_module() -> std::path::PathBuf {
    racine().join("docs").join("mycelium.wasm")
}

fn lire_module() -> Vec<u8> {
    std::fs::read(chemin_module()).unwrap_or_else(|erreur| panic!("docs/mycelium.wasm se lit : {erreur}"))
}

fn lire_page(nom: &str) -> String {
    std::fs::read_to_string(racine().join("docs").join(nom))
        .unwrap_or_else(|erreur| panic!("docs/{nom} existe et se lit : {erreur}"))
}

// ---------------------------------------------------------------------------------------------------------------
// Un petit lecteur du format binaire WebAssembly : juste ce qu'il faut pour la section des exportations, comme dans
// `etape_4_artefact.rs`.
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

/// Une exportation : son nom et son genre (0 fonction, 1 table, 2 mémoire, 3 globale).
struct Exportation {
    nom: String,
    genre: u8,
}

/// Ce que le test lit d'un module : ses exportations, et elles seules.
struct Module {
    exportations: Vec<Exportation>,
}

/// Analyse un module : l'en-tête (`\0asm` puis la version 1), puis les sections repérées par leur numéro. Les
/// sections qui ne servent pas ici sont sautées sans être lues.
fn analyser(octets: &[u8]) -> Module {
    assert!(octets.len() >= 8, "module trop court : {} octets", octets.len());
    assert_eq!(&octets[0..4], b"\0asm", "le module ne commence pas par \\0asm");
    assert_eq!(octets[4..8], [1, 0, 0, 0], "le module n'est pas en version 1");

    let mut module = Module { exportations: Vec::new() };
    let mut lecteur = Lecteur::new(&octets[8..]);
    while !lecteur.fini() {
        let numero = lecteur.octet();
        let taille = lecteur.entier() as usize;
        let mut section = Lecteur::new(lecteur.tranche(taille));
        if numero == 7 {
            for _ in 0..section.entier() {
                let nom = section.nom();
                let genre = section.octet();
                // L'indice de l'exportation ne sert pas ici ; il faut seulement avancer le lecteur.
                section.entier();
                module.exportations.push(Exportation { nom, genre });
            }
        }
    }
    module
}

// ---------------------------------------------------------------------------------------------------------------
// Le texte de la page et de son script.
// ---------------------------------------------------------------------------------------------------------------

/// Le code de `cube.js` sans ses commentaires : sans les blocs `/* … */` et sans les lignes qui ne sont qu'un
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

/// Le code de `docs/cube.js`, sans ses commentaires.
fn lire_le_code_de_cube_js() -> String {
    sans_commentaires(&lire_page("cube.js"))
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

/// Le nom que `cube.js` passe à son unique `fetch`, qui doit être un texte littéral.
fn nom_du_fetch(script: &str) -> String {
    let mut occurrences = script.match_indices("fetch(");
    let (position, _) = occurrences.next().expect("cube.js ne fait aucun fetch");
    assert!(occurrences.next().is_none(), "cube.js fait plus d'un fetch");
    let apres = script[position + "fetch(".len()..].trim_start();
    let Some(guillemet) = apres.chars().next().filter(|c| *c == '"' || *c == '\'' || *c == '`') else {
        panic!("fetch doit recevoir un texte littéral : {}", &apres[..apres.len().min(30)]);
    };
    let nom = &apres[1..];
    let fin = nom.find(guillemet).expect("guillemet fermant du fetch");
    nom[..fin].to_string()
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

/// Vrai si la page contient un lien `href="…"` vers cette cible, écrit entre guillemets ou entre apostrophes.
fn a_un_lien(html: &str, cible: &str) -> bool {
    ["\"", "'"].iter().any(|guillemet| html.contains(&format!("href={guillemet}{cible}{guillemet}")))
}

// ---------------------------------------------------------------------------------------------------------------
// Node.
// ---------------------------------------------------------------------------------------------------------------

/// Vrai si Node est là. Sinon une note sur la sortie d'erreur et le test passe : la page se vérifie sans Node, et
/// un poste qui n'en a pas ne bloque pas le contrôle de la syntaxe.
fn node_est_la() -> bool {
    let present = Command::new("node").arg("--version").output().map(|sortie| sortie.status.success()).unwrap_or(false);
    if !present {
        eprintln!("note : Node est absent, ce test ne vérifie rien sur ce poste");
    }
    present
}

// ---------------------------------------------------------------------------------------------------------------
// Les tests.
// ---------------------------------------------------------------------------------------------------------------

/// 1. `docs/cube.html` est en français (`lang="fr"`), porte la balise viewport, a deux canvas d'id `volume` et
///    `coupe`, et un `<script type="module" src="cube.js">`. Ni la page ni son script ne contient `http://` ni
///    `https://`.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn la_page_du_cube_est_en_francais_et_charge_cube_js() {
    let html = lire_page("cube.html");
    let minuscules = html.to_lowercase();
    assert!(minuscules.contains("lang=\"fr\""), "cube.html n'est pas déclaré en français");
    assert!(minuscules.contains("name=\"viewport\""), "cube.html n'a pas la balise viewport");

    let balises = balises(&minuscules);
    let canvas: Vec<&str> =
        balises.iter().copied().filter(|balise| balise.trim_start().starts_with("canvas")).collect();
    assert_eq!(canvas.len(), 2, "cube.html n'a pas exactement deux canvas");
    let ids_canvas: BTreeSet<String> = canvas
        .iter()
        .map(|balise| attribut_id(balise).expect("un <canvas> de cube.html n'a pas d'attribut id"))
        .collect();
    let attendus: BTreeSet<String> = ["volume", "coupe"].iter().map(|identifiant| identifiant.to_string()).collect();
    assert_eq!(ids_canvas, attendus, "les deux canvas ne portent pas exactement les ids « volume » et « coupe »");

    let debut = minuscules.find("<script type=\"module\"").expect("pas de <script type=\"module\" dans cube.html");
    let fin = minuscules[debut..].find('>').expect("balise <script> non fermée");
    let balise = &minuscules[debut..debut + fin];
    assert!(
        balise.contains("src=") && balise.contains("cube.js"),
        "le script de type module ne charge pas cube.js : {balise}"
    );

    for nom in ["cube.html", "cube.js"] {
        let texte = lire_page(nom);
        for interdit in ["http://", "https://"] {
            assert!(!texte.contains(interdit), "docs/{nom} contient « {interdit} » : une ressource externe ?");
        }
    }
}

/// 2. Chaque nom `mycelium_…` qu'emploie `cube.js` est une fonction exportée par le module, et le script emploie
///    chacune des onze fonctions `mycelium_cube_*`. Les noms s'écrivent en entier dans le script, jamais composés.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn cube_js_emploie_les_exportations_du_module() {
    let module = analyser(&lire_module());
    let exportees: BTreeSet<String> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.clone()).collect();
    let employes = noms_employes(&lire_le_code_de_cube_js());

    let inconnus: Vec<&String> = employes.difference(&exportees).collect();
    assert!(inconnus.is_empty(), "cube.js emploie des noms que le module n'exporte pas : {inconnus:?}");
    for nom in FONCTIONS_EMPLOYEES_PAR_CUBE_JS {
        assert!(employes.contains(nom), "cube.js n'emploie pas {nom}");
    }
}

/// 3. `cube.js` ne charge que `mycelium.wasm` : un seul `fetch`, dont l'argument est ce nom littéral, et aucune
///    autre mention de module.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn cube_js_ne_charge_que_le_module() {
    let code = lire_le_code_de_cube_js();
    let nom = nom_du_fetch(&code);
    assert_eq!(nom, "mycelium.wasm", "cube.js charge « {nom} » et non « mycelium.wasm »");
    assert_eq!(code.matches(".wasm").count(), 1, "cube.js nomme un module ailleurs qu'à son fetch");
}

/// 4. Chaque identifiant que `cube.js` demande par `getElementById("…")` existe dans `cube.html`.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn cube_js_ne_demande_que_des_elements_de_la_page() {
    let html = lire_page("cube.html");
    let demandes = identifiants_demandes(&lire_le_code_de_cube_js());
    assert!(!demandes.is_empty(), "cube.js ne demande aucun élément par getElementById");

    let existants: BTreeSet<String> = balises(&html).iter().filter_map(|balise| attribut_id(balise)).collect();
    for identifiant in &demandes {
        assert!(existants.contains(identifiant), "cube.html n'a pas d'élément d'id « {identifiant} »");
    }

    for identifiant in ["volume", "coupe"] {
        assert!(
            demandes.iter().any(|demande| demande == identifiant),
            "cube.js ne demande pas le canvas d'id « {identifiant} » par getElementById"
        );
    }
}

/// 5. `docs/index.html` a un lien vers `cube.html`, et `cube.html` un lien vers `index.html`.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn les_deux_pages_se_lient() {
    let index = lire_page("index.html");
    assert!(a_un_lien(&index, "cube.html"), "index.html n'a pas de lien vers cube.html");

    let cube = lire_page("cube.html");
    assert!(a_un_lien(&cube, "index.html"), "cube.html n'a pas de lien vers index.html");
}

/// 6. `cube.js` est du JavaScript valide : `node --check docs/cube.js` réussit. Le contrôle est sauté, avec une
///    note, quand Node est absent.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn cube_js_est_du_javascript_valide() {
    if !node_est_la() {
        return;
    }
    let script = racine().join("docs").join("cube.js");
    let sortie = Command::new("node")
        .arg("--check")
        .arg(&script)
        .output()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"));
    assert!(sortie.status.success(), "cube.js n'est pas valide : {}", String::from_utf8_lossy(&sortie.stderr));
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

/// 7. La page ne dépasse pas 200 Ko en tout : tous les fichiers de `docs/`, hors le module. C'est le plafond de
///    `etape_4_artefact.rs`, qui compte désormais aussi la page du cube.
#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn la_page_ne_depasse_pas_200_ko() {
    let mut total = 0;
    taille_sans_le_module(&racine().join("docs"), &mut total);
    assert!(total > 0, "docs/ est vide");
    assert!(total <= TAILLE_MAX_PAGE, "la page pèse {total} octets");
}
