//! Tests d'acceptation de l'étape 21 : la page de la colonie, `docs/colonie.html` + `docs/colonie.js`.
//!
//! Ils font pour la page de la colonie ce que `etape_18_page.rs` fait pour celle du cube :
//!
//! 1. la forme de `colonie.html` : en français, lisible sur un téléphone, un canvas d'id `volume`, et un script
//!    module qui charge `colonie.js` ; ni la page ni son script ne nomment une ressource externe ;
//! 2. `colonie.js` n'importe qu'un module, la copie locale de three.js, et ce fichier existe ;
//! 3. chaque nom `mycelium_…` que `colonie.js` emploie est une exportation du module, et le script emploie les
//!    quinze fonctions `mycelium_colonie_*` ;
//! 4. `colonie.js` ne charge que `mycelium.wasm`, par un seul `fetch` ;
//! 5. chaque `getElementById("…")` de `colonie.js` a son élément dans `colonie.html` ;
//! 6. les deux empreintes de référence écrites dans `colonie.js` sont celles du dépôt
//!    (`references/colonie-cube.empreinte` et `references/colonie-goutte.empreinte`) ;
//! 7. `colonie.html` se lie à `index.html` et à `cube.html`, et `cube.html` à `colonie.html` ;
//! 8. `colonie.js` est du JavaScript valide, quand Node est là.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Les quinze fonctions `mycelium_colonie_*` que la page de la colonie emploie : le moteur calcule, la page dessine.
const FONCTIONS_EMPLOYEES_PAR_COLONIE_JS: [&str; 15] = [
    "mycelium_colonie_choisir",
    "mycelium_colonie_demarrer",
    "mycelium_colonie_avancer",
    "mycelium_colonie_temps",
    "mycelium_colonie_duree",
    "mycelium_colonie_dimension",
    "mycelium_colonie_nb_segments",
    "mycelium_colonie_segment",
    "mycelium_colonie_segment_temps",
    "mycelium_colonie_pointes",
    "mycelium_colonie_ramifications",
    "mycelium_colonie_nourriture_masse",
    "mycelium_colonie_nb_depots",
    "mycelium_colonie_depot",
    "mycelium_colonie_empreinte_mot",
];

/// Le seul module que `colonie.js` importe : la copie locale de three.js, sous `docs/bibliotheques/`.
const MODULE_IMPORTE: &str = "./bibliotheques/three/three.module.min.js";

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
// `etape_18_page.rs`.
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

/// Le code de `colonie.js` sans ses commentaires : sans les blocs `/* … */` et sans les lignes qui ne sont qu'un
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

/// Le code de `docs/colonie.js`, sans ses commentaires.
fn lire_le_code_de_colonie_js() -> String {
    sans_commentaires(&lire_page("colonie.js"))
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

/// Vrai si ce caractère peut appartenir à un identifiant JavaScript.
fn est_un_identifiant(caractere: Option<char>) -> bool {
    caractere.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Les textes littéraux que chaque `import` du script vise, dans l'ordre. Un `import` est le mot seul, ni collé à un
/// identifiant avant, ni à un identifiant après (`importmap`, `important` ne sont pas des imports) ; le premier texte
/// entre guillemets qui le suit est son module.
fn modules_importes(script: &str) -> Vec<String> {
    let mut modules = Vec::new();
    let mut depart = 0;
    while let Some(position) = script[depart..].find("import") {
        let debut = depart + position;
        let apres = debut + "import".len();
        let avant_est_un_identifiant = est_un_identifiant(script[..debut].chars().next_back());
        let apres_est_un_identifiant = est_un_identifiant(script[apres..].chars().next());
        if avant_est_un_identifiant || apres_est_un_identifiant {
            depart = apres;
            continue;
        }
        let queue = &script[apres..];
        let guillemet = queue
            .find(['"', '\'', '`'])
            .unwrap_or_else(|| panic!("un import sans texte littéral : {}", &queue[..queue.len().min(30)]));
        let caractere = queue[guillemet..].chars().next().expect("guillemet de l'import");
        let debut_du_nom = guillemet + 1;
        let fin = queue[debut_du_nom..].find(caractere).expect("guillemet fermant de l'import");
        modules.push(queue[debut_du_nom..debut_du_nom + fin].to_string());
        depart = apres + debut_du_nom + fin + 1;
    }
    modules
}

/// Le nom que `colonie.js` passe à son unique `fetch`, qui doit être un texte littéral.
fn nom_du_fetch(script: &str) -> String {
    let mut occurrences = script.match_indices("fetch(");
    let (position, _) = occurrences.next().expect("colonie.js ne fait aucun fetch");
    assert!(occurrences.next().is_none(), "colonie.js fait plus d'un fetch");
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

/// Les empreintes SHA-256 écrites en toutes lettres dans ce texte : chaque suite d'au moins 64 caractères
/// hexadécimaux, prise quand elle en fait exactement 64 (une empreinte du dépôt).
fn empreintes_ecrites(texte: &str) -> BTreeSet<String> {
    let mut empreintes = BTreeSet::new();
    let mut courant = String::new();
    for caractere in texte.chars().chain(std::iter::once(' ')) {
        if caractere.is_ascii_hexdigit() {
            courant.push(caractere);
        } else {
            if courant.len() == 64 {
                empreintes.insert(courant.to_lowercase());
            }
            courant.clear();
        }
    }
    empreintes
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

/// 1. `docs/colonie.html` est en français (`lang="fr"`), porte la balise viewport, a un canvas d'id `volume`, et un
///    `<script type="module" src="colonie.js">`. Ni la page ni son script ne contient `http://` ni `https://`.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn la_page_de_la_colonie_est_en_francais_et_charge_colonie_js() {
    let html = lire_page("colonie.html");
    let minuscules = html.to_lowercase();
    assert!(minuscules.contains("lang=\"fr\""), "colonie.html n'est pas déclaré en français");
    assert!(minuscules.contains("name=\"viewport\""), "colonie.html n'a pas la balise viewport");

    let balises = balises(&minuscules);
    let canvas: Vec<&str> =
        balises.iter().copied().filter(|balise| balise.trim_start().starts_with("canvas")).collect();
    assert!(!canvas.is_empty(), "colonie.html n'a aucun canvas");
    let ids_canvas: BTreeSet<String> = canvas.iter().filter_map(|balise| attribut_id(balise)).collect();
    assert!(ids_canvas.contains("volume"), "colonie.html n'a pas de canvas d'id « volume » : {ids_canvas:?}");

    let debut = minuscules.find("<script type=\"module\"").expect("pas de <script type=\"module\" dans colonie.html");
    let fin = minuscules[debut..].find('>').expect("balise <script> non fermée");
    let balise = &minuscules[debut..debut + fin];
    assert!(
        balise.contains("src=") && balise.contains("colonie.js"),
        "le script de type module ne charge pas colonie.js : {balise}"
    );

    for nom in ["colonie.html", "colonie.js"] {
        let texte = lire_page(nom);
        for interdit in ["http://", "https://"] {
            assert!(!texte.contains(interdit), "docs/{nom} contient « {interdit} » : une ressource externe ?");
        }
    }
}

/// 2. `colonie.js` n'importe qu'un module, `./bibliotheques/three/three.module.min.js` : chaque `import` du fichier
///    vise ce chemin, il y en a au moins un, et ce fichier existe.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn colonie_js_n_importe_que_three() {
    let modules = modules_importes(&lire_le_code_de_colonie_js());
    assert!(!modules.is_empty(), "colonie.js n'importe aucun module");
    for module in &modules {
        assert_eq!(module, MODULE_IMPORTE, "colonie.js importe « {module} » et non « {MODULE_IMPORTE} »");
    }

    let chemin = racine().join("docs").join(MODULE_IMPORTE.trim_start_matches("./"));
    assert!(chemin.is_file(), "le module importé n'existe pas : {}", chemin.display());
}

/// 3. Chaque nom `mycelium_…` qu'emploie `colonie.js` est une fonction exportée par le module, et le script emploie
///    chacune des quinze fonctions `mycelium_colonie_*`. Les noms s'écrivent en entier dans le script, jamais
///    composés.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn colonie_js_emploie_les_exportations_du_module() {
    let module = analyser(&lire_module());
    let exportees: BTreeSet<String> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.clone()).collect();
    let employes = noms_employes(&lire_le_code_de_colonie_js());
    assert!(!employes.is_empty(), "colonie.js n'emploie aucun nom mycelium_…");

    let inconnus: Vec<&String> = employes.difference(&exportees).collect();
    assert!(inconnus.is_empty(), "colonie.js emploie des noms que le module n'exporte pas : {inconnus:?}");
    for nom in FONCTIONS_EMPLOYEES_PAR_COLONIE_JS {
        assert!(employes.contains(nom), "colonie.js n'emploie pas {nom}");
    }
}

/// 4. `colonie.js` ne charge que `mycelium.wasm` : un seul `fetch`, dont l'argument est ce nom littéral, et aucune
///    autre mention de module.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn colonie_js_ne_charge_que_le_module() {
    let code = lire_le_code_de_colonie_js();
    let nom = nom_du_fetch(&code);
    assert_eq!(nom, "mycelium.wasm", "colonie.js charge « {nom} » et non « mycelium.wasm »");
    assert_eq!(code.matches(".wasm").count(), 1, "colonie.js nomme un module ailleurs qu'à son fetch");
}

/// 5. Chaque identifiant que `colonie.js` demande par `getElementById("…")` existe dans `colonie.html`, et le canvas
///    d'id `volume` est de ceux-là.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn colonie_js_ne_demande_que_des_elements_de_la_page() {
    let html = lire_page("colonie.html");
    let demandes = identifiants_demandes(&lire_le_code_de_colonie_js());
    assert!(!demandes.is_empty(), "colonie.js ne demande aucun élément par getElementById");

    let existants: BTreeSet<String> = balises(&html).iter().filter_map(|balise| attribut_id(balise)).collect();
    for identifiant in &demandes {
        assert!(existants.contains(identifiant), "colonie.html n'a pas d'élément d'id « {identifiant} »");
    }

    assert!(
        demandes.iter().any(|demande| demande == "volume"),
        "colonie.js ne demande pas le canvas d'id « volume » par getElementById"
    );
}

/// 6. Les deux empreintes de référence écrites dans `colonie.js` sont celles du dépôt : les 64 caractères de
///    `references/colonie-cube.empreinte` et de `references/colonie-goutte.empreinte`, et aucune autre.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn colonie_js_ecrit_les_empreintes_de_reference() {
    let mut attendues = BTreeSet::new();
    for nom in ["colonie-cube.empreinte", "colonie-goutte.empreinte"] {
        let texte = std::fs::read_to_string(racine().join("references").join(nom))
            .unwrap_or_else(|erreur| panic!("references/{nom} se lit : {erreur}"));
        let empreinte = texte.trim().to_lowercase();
        assert_eq!(empreinte.len(), 64, "references/{nom} ne porte pas une empreinte SHA-256 : « {empreinte} »");
        assert!(empreinte.chars().all(|c| c.is_ascii_hexdigit()), "references/{nom} n'est pas en hexadécimal");
        attendues.insert(empreinte);
    }

    let ecrites = empreintes_ecrites(&lire_le_code_de_colonie_js());
    assert!(!ecrites.is_empty(), "colonie.js n'écrit aucune empreinte");
    assert_eq!(ecrites, attendues, "les empreintes écrites dans colonie.js ne sont pas celles du dépôt");
}

/// 7. `docs/colonie.html` a un lien vers `index.html` et un lien vers `cube.html`, et `cube.html` en a un vers
///    `colonie.html`.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn les_pages_se_lient() {
    let colonie = lire_page("colonie.html");
    assert!(a_un_lien(&colonie, "index.html"), "colonie.html n'a pas de lien vers index.html");
    assert!(a_un_lien(&colonie, "cube.html"), "colonie.html n'a pas de lien vers cube.html");

    let cube = lire_page("cube.html");
    assert!(a_un_lien(&cube, "colonie.html"), "cube.html n'a pas de lien vers colonie.html");
}

/// 8. `colonie.js` est du JavaScript valide : `node --check docs/colonie.js` réussit. Le contrôle est sauté, avec
///    une note, quand Node est absent.
#[test]
#[cfg_attr(not(feature = "etape-21"), ignore)]
fn colonie_js_est_du_javascript_valide() {
    if !node_est_la() {
        return;
    }
    let script = racine().join("docs").join("colonie.js");
    let sortie = Command::new("node")
        .arg("--check")
        .arg(&script)
        .output()
        .unwrap_or_else(|erreur| panic!("le lancement de Node échoue : {erreur}"));
    assert!(sortie.status.success(), "colonie.js n'est pas valide : {}", String::from_utf8_lossy(&sortie.stderr));
}
