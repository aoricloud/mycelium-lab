//! Tests d'acceptation de l'étape 25 (la page) : `docs/colonie.html` + `docs/colonie.js`, qui passent à cinq scènes.
//!
//! Ils font pour les scènes 2, 3 et 4 ce que `etape_21_page.rs` fait pour les deux premières :
//!
//! 1. le `<select id="scene">` de `colonie.html` a exactement neuf `<option>`, de valeurs `"0"`, `"1"`, `"2"`, `"3"`
//!    et `"4"`, dans cet ordre ;
//! 2. `colonie.js` emploie les quatre fonctions nouvelles (`mycelium_colonie_nb_obstacles`,
//!    `mycelium_colonie_obstacle`, `mycelium_colonie_anastomoses`, `mycelium_colonie_soudure`), et chaque nom
//!    `mycelium_…` qu'il emploie est une exportation de `docs/mycelium.wasm` ;
//! 3. les empreintes écrites dans `colonie.js` (chaque suite de 64 chiffres hexadécimaux), lues dans l'ordre du
//!    fichier, sont celles des neuf scènes dans l'ordre de leurs numéros : `colonie-cube`, `colonie-goutte`,
//!    `colonie-chimio`, `colonie-mur`, `colonie-anastomose` (la page prend l'empreinte de référence par le numéro de
//!    la scène) ;
//! 4. `colonie.html` a des éléments d'ids « soudures », « legende-mur » et « legende-soudure », et `colonie.js` les
//!    demande par `getElementById` ;
//! 5. `colonie.js` est du JavaScript valide, quand Node est là : `node --check docs/colonie.js`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Les quatre fonctions nouvelles que la page emploie pour le mur et les soudures : le moteur calcule, la page
/// dessine.
const FONCTIONS_NOUVELLES: [&str; 4] = [
    "mycelium_colonie_nb_obstacles",
    "mycelium_colonie_obstacle",
    "mycelium_colonie_anastomoses",
    "mycelium_colonie_soudure",
];

/// Les neuf empreintes de référence, dans l'ordre des numéros de scène (0 à 8) : la page prend la sienne par le
/// numéro de la scène.
const REFERENCES_DES_SCENES: [&str; 9] = [
    "colonie-cube.empreinte",
    "colonie-goutte.empreinte",
    "colonie-chimio.empreinte",
    "colonie-mur.empreinte",
    "colonie-anastomose.empreinte",
    "colonie-boule.empreinte",
    "colonie-tore.empreinte",
    "colonie-coquille.empreinte",
    "colonie-labyrinthe.empreinte",
];

/// Les éléments que la page demande pour le mur et les soudures.
const IDS_DU_MUR_ET_DES_SOUDURES: [&str; 3] = ["soudures", "legende-mur", "legende-soudure"];

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
// `etape_21_page.rs`.
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

/// Les identifiants que le script demande par `getElementById("…")` (ou avec des apostrophes), dans l'ordre.
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

/// La valeur de l'attribut `nom` d'une balise, s'il y en a un (écrit entre guillemets ou entre apostrophes).
fn valeur_attribut(balise: &str, nom: &str) -> Option<String> {
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

/// La valeur de l'attribut `id` d'une balise, s'il y en a un (écrit entre guillemets ou entre apostrophes).
fn attribut_id(balise: &str) -> Option<String> {
    valeur_attribut(balise, "id")
}

/// Les balises `<option>` du `<select>` d'id `id`, dans l'ordre de la page, chacune réduite à son contenu entre `<`
/// et `>`. Panique si l'élément n'existe pas ou n'est pas fermé.
fn options_du_select(html: &str, id: &str) -> Vec<String> {
    let minuscules = html.to_lowercase();
    let mut depart = 0;
    while let Some(position) = minuscules[depart..].find("<select") {
        let debut = depart + position;
        let fin_balise = debut + minuscules[debut..].find('>').expect("balise <select> non fermée");
        if attribut_id(&minuscules[debut + 1..fin_balise]).as_deref() == Some(id) {
            let interieur = &minuscules[fin_balise + 1..];
            let fin = fin_balise + 1 + interieur.find("</select>").expect("balise </select> absente");
            let contenu = &minuscules[fin_balise + 1..fin];
            return contenu
                .split('<')
                .skip(1)
                .filter_map(|morceau| morceau.split('>').next())
                .filter(|balise| balise.starts_with("option"))
                .map(str::to_string)
                .collect();
        }
        depart = fin_balise + 1;
    }
    panic!("colonie.html n'a pas de <select id=\"{id}\">");
}

/// Les empreintes SHA-256 écrites en toutes lettres dans ce texte, dans l'ordre du fichier : chaque suite d'au moins
/// 64 caractères hexadécimaux, prise quand elle en fait exactement 64 (une empreinte du dépôt).
fn empreintes_dans_l_ordre(texte: &str) -> Vec<String> {
    let mut empreintes = Vec::new();
    let mut courant = String::new();
    for caractere in texte.chars().chain(std::iter::once(' ')) {
        if caractere.is_ascii_hexdigit() {
            courant.push(caractere);
        } else {
            if courant.len() == 64 {
                empreintes.push(courant.to_lowercase());
            }
            courant.clear();
        }
    }
    empreintes
}

/// L'empreinte de référence d'un fichier de `references/` : 64 chiffres hexadécimaux, en minuscules.
fn lire_empreinte_de_reference(nom: &str) -> String {
    let texte = std::fs::read_to_string(racine().join("references").join(nom))
        .unwrap_or_else(|erreur| panic!("references/{nom} se lit : {erreur}"));
    let empreinte = texte.trim().to_lowercase();
    assert_eq!(empreinte.len(), 64, "references/{nom} ne porte pas une empreinte SHA-256 : « {empreinte} »");
    assert!(empreinte.chars().all(|c| c.is_ascii_hexdigit()), "references/{nom} n'est pas en hexadécimal");
    empreinte
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

/// 1. Le `<select id="scene">` de `colonie.html` a exactement neuf `<option>`, de valeurs `"0"`, `"1"`, `"2"`,
///    `"3"` et `"4"`, dans cet ordre.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn le_select_de_la_page_a_les_neuf_scenes_dans_l_ordre() {
    let html = lire_page("colonie.html");
    let options = options_du_select(&html, "scene");
    assert_eq!(options.len(), 9, "le <select id=\"scene\"> n'a pas neuf <option> : {options:?}");

    let valeurs: Vec<String> = options.iter().filter_map(|balise| valeur_attribut(balise, "value")).collect();
    assert_eq!(valeurs.len(), options.len(), "un <option> du <select id=\"scene\"> n'a pas de valeur");
    let attendues: Vec<String> = (0..9).map(|numero| numero.to_string()).collect();
    assert_eq!(valeurs, attendues, "les valeurs des <option> du <select id=\"scene\">, dans l'ordre");
}

/// 2. `colonie.js` emploie les quatre fonctions nouvelles, et chaque nom `mycelium_…` qu'il emploie est une
///    exportation de `docs/mycelium.wasm`.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn colonie_js_emploie_les_quatre_fonctions_nouvelles_et_les_exportations_du_module() {
    let module = analyser(&lire_module());
    let exportees: BTreeSet<String> =
        module.exportations.iter().filter(|e| e.genre == 0).map(|e| e.nom.clone()).collect();
    let employes = noms_employes(&lire_le_code_de_colonie_js());
    assert!(!employes.is_empty(), "colonie.js n'emploie aucun nom mycelium_…");

    let inconnus: Vec<&String> = employes.difference(&exportees).collect();
    assert!(inconnus.is_empty(), "colonie.js emploie des noms que le module n'exporte pas : {inconnus:?}");
    for nom in FONCTIONS_NOUVELLES {
        assert!(employes.contains(nom), "colonie.js n'emploie pas {nom}");
    }
}

/// 3. Les empreintes écrites dans `colonie.js`, lues dans l'ordre du fichier, sont celles des neuf scènes dans
///    l'ordre de leurs numéros : `colonie-cube`, `colonie-goutte`, `colonie-chimio`, `colonie-mur`,
///    `colonie-anastomose`.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn colonie_js_ecrit_les_empreintes_des_neuf_scenes_dans_l_ordre() {
    let ecrites = empreintes_dans_l_ordre(&lire_le_code_de_colonie_js());
    assert_eq!(
        ecrites.len(),
        REFERENCES_DES_SCENES.len(),
        "colonie.js n'écrit pas neuf empreintes, dans l'ordre : {ecrites:?}"
    );
    let attendues: Vec<String> = REFERENCES_DES_SCENES.iter().map(|nom| lire_empreinte_de_reference(nom)).collect();
    assert_eq!(
        ecrites, attendues,
        "les empreintes écrites dans colonie.js, dans l'ordre, ne sont pas celles des neuf scènes"
    );
}

/// 4. `colonie.html` a des éléments d'ids « soudures », « legende-mur » et « legende-soudure », et `colonie.js` les
///    demande par `getElementById`.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
fn colonie_js_demande_les_elements_du_mur_et_des_soudures() {
    let html = lire_page("colonie.html");
    let existants: BTreeSet<String> = balises(&html).iter().filter_map(|balise| attribut_id(balise)).collect();
    let demandes = identifiants_demandes(&lire_le_code_de_colonie_js());
    assert!(!demandes.is_empty(), "colonie.js ne demande aucun élément par getElementById");

    for identifiant in IDS_DU_MUR_ET_DES_SOUDURES {
        assert!(existants.contains(identifiant), "colonie.html n'a pas d'élément d'id « {identifiant} »");
        assert!(
            demandes.iter().any(|demande| demande == identifiant),
            "colonie.js ne demande pas l'élément d'id « {identifiant} » par getElementById"
        );
    }
}

/// 5. `colonie.js` est du JavaScript valide : `node --check docs/colonie.js` réussit. Le contrôle est sauté, avec
///    une note, quand Node est absent.
#[test]
#[cfg_attr(not(feature = "etape-25"), ignore)]
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
