//! Mesure du moule : comment la colonie remplit une cavité, et comment elle sort d'un labyrinthe (étape M1).
//!
//! `cargo run --release --example mesure_moule -- <scénario> <graine> [<dossier de sortie>]`
//!
//! Lit le scénario (`<scénario>` est le chemin d'un fichier), le **développe** par `moule::developper` (la ligne `moule …`
//! devient des lignes `obstacle …`), lit le texte développé par `lire_scenario_tropismes`, puis fait tourner la simulation
//! pendant sa `duree` avec la graine donnée en prenant la courbe de remplissage (`courbe_de_remplissage`, du pas 0 au pas
//! `duree`). Sortie standard, rien d'autre :
//!
//! - `moule <cases_libres>` : la taille de la cavité ;
//! - une ligne `courbe <pas> <cases_occupees> <remplissage>` tous les 10 pas (pas = 0, 10, 20, … jusqu'à `duree`), plus le
//!   dernier pas `duree` s'il n'est pas un multiple de 10 ;
//! - les quatorze lignes de `piece::mesurer` à la fin (le format de `mesure_piece`) ;
//! - si la ligne `moule` du scénario est un `labyrinthe` : `sortie <temps de sortie ou aucun>` et
//!   `chemin <plus court chemin ou aucun>`. Le temps de sortie est celui de `moule::temps_de_sortie` vers la chambre
//!   (colonnes − 1, rangees − 1) entière (`moule::chambre`), cherché sur la simulation **déjà avancée jusqu'à `duree`** avec
//!   `pas_max = duree` : `aucun` si la colonie n'y est pas arrivée. Le chemin est le `plus_court_chemin` (4 voisins) entre le
//!   centre de la chambre (0, 0) et celui de la chambre (colonnes − 1, rangees − 1), sur les cases que le substrat ne bloque pas.
//!
//! Si un dossier de sortie est donné (créé au besoin), il reçoit aussi `densite.png`, la carte de densité, du même dessin que
//! `mesure_piece` (le code du dessin en est copié : un exemple ne partage pas son code avec un autre). Sur une erreur
//! (nombre d'arguments, graine qui n'est pas un entier de 0 à 2⁶⁴ − 1, scénario illisible, moule ou scénario invalide), le
//! message va sur la sortie d'erreur, rien n'est imprimé ni écrit, et le code de sortie est 2.
//!
//! # L'image
//!
//! Chaque case est un carré uniforme de `k × k` pixels, la case `(x, y)` en `(x k, y k)` (la ligne 0 en haut) : `k` est le plus
//! grand entier de 4 à 1 pour lequel les octets bruts de l'image, `(hauteur k) × (3 largeur k + 1)`, tiennent dans 490 000
//! octets. Une case bloquée est grise (48, 48, 48), une case libre sans hyphe a la couleur du fond (16, 16, 24), une case
//! occupée suit la rampe du violet (70, 20, 110) au jaune (255, 240, 90) selon `sqrt(min(d / dmax, 1))`, en entiers.

use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use mycelium_lab::image::ecrire_png;
use mycelium_lab::moule::{
    centre_de_la_chambre, chambre, courbe_de_remplissage, developper, plus_court_chemin, temps_de_sortie,
};
use mycelium_lab::piece::{Carte, MesuresPiece, carte_de_densite, mesurer};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

const USAGE: &str = "usage : mesure_moule <scénario> <graine> [<dossier de sortie>]";

/// Une ligne `courbe` tous les `PAS_DE_LA_COURBE` pas.
const PAS_DE_LA_COURBE: u32 = 10;
/// Le côté d'une case, en pixels, quand la boîte est assez petite.
const PIXELS_PAR_CASE: usize = 4;
/// Ce que pèsent, au plus, les pixels bruts de l'image (octets de filtre compris).
const OCTETS_BRUTS_MAXIMAUX: usize = 490_000;
/// Couleur d'une case libre sans hyphe.
const FOND: [u8; 3] = [16, 16, 24];
/// Couleur d'une case bloquée.
const BLOQUEE: [u8; 3] = [48, 48, 48];
/// Bas de la rampe (la plus faible densité) : un violet sombre.
const RAMPE_SOMBRE: [i32; 3] = [70, 20, 110];
/// Haut de la rampe (la densité maximale) : un jaune clair.
const RAMPE_CLAIRE: [i32; 3] = [255, 240, 90];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match lancer(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn lancer(args: &[String]) -> Result<(), String> {
    let (scenario, graine, sortie) = match args {
        [scenario, graine] => (scenario, graine, None),
        [scenario, graine, sortie] => (scenario, graine, Some(Path::new(sortie.as_str()))),
        _ => return Err("deux ou trois arguments attendus".to_string()),
    };
    let graine: u64 = graine.parse().map_err(|_| format!("graine illisible : {graine}"))?;

    let chemin = Path::new(scenario.as_str());
    let texte = std::fs::read_to_string(chemin).map_err(|erreur| format!("{} : {erreur}", chemin.display()))?;
    let developpe = developper(&texte).map_err(|erreur| format!("{} : {erreur:?}", chemin.display()))?;
    let (reglages, milieu, tropismes, duree) =
        lire_scenario_tropismes(&developpe).map_err(|erreur| format!("{} : {erreur:?}", chemin.display()))?;
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine)
        .map_err(|erreur| format!("{} : {erreur:?}", chemin.display()))?;

    // Tout se calcule d'abord ; rien n'est imprimé tant que tout n'a pas réussi.
    let courbe = courbe_de_remplissage(&mut simulation, duree);
    let mesures = mesurer(&simulation);
    let mut rapport = format!("moule {}\n", mesures.cases_libres);
    for point in &courbe {
        if point.pas % PAS_DE_LA_COURBE == 0 || point.pas == duree {
            let _ = writeln!(rapport, "courbe {} {} {}", point.pas, point.cases_occupees, point.remplissage);
        }
    }
    rapport.push_str(&mesures_en_texte(&mesures));
    if let Some((colonnes, rangees, couloir)) = labyrinthe_du_texte(&texte) {
        // La sortie est la dernière chambre entière ; la simulation est déjà à `duree`, elle n'avance pas.
        let temps = temps_de_sortie(&mut simulation, chambre(couloir, colonnes - 1, rangees - 1), duree);
        let _ = writeln!(rapport, "sortie {}", temps.map_or_else(|| "aucun".to_string(), |t| t.to_string()));
        // Le plus court chemin va du centre de la première chambre au centre de la dernière.
        let sortie_du_labyrinthe = centre_de_la_chambre(couloir, colonnes - 1, rangees - 1);
        let (largeur, hauteur) = (simulation.reglages().largeur, simulation.reglages().hauteur);
        let mut libres = Vec::with_capacity(largeur * hauteur);
        for y in 0..hauteur {
            for x in 0..largeur {
                libres.push(!simulation.substrat().est_bloquee(x, y));
            }
        }
        let depart = centre_de_la_chambre(couloir, 0, 0);
        let plus_court = plus_court_chemin(largeur, hauteur, &libres, depart, sortie_du_labyrinthe);
        let _ = writeln!(rapport, "chemin {}", plus_court.map_or_else(|| "aucun".to_string(), |n| n.to_string()));
    }

    // L'image d'abord : si elle ne peut pas s'écrire, rien n'est imprimé.
    if let Some(dossier) = sortie {
        std::fs::create_dir_all(dossier).map_err(|erreur| format!("{} : {erreur}", dossier.display()))?;
        let fichier = dossier.join("densite.png");
        std::fs::write(&fichier, image_de_densite(&simulation))
            .map_err(|erreur| format!("{} : {erreur}", fichier.display()))?;
    }
    print!("{rapport}");
    Ok(())
}

/// Les `(colonnes, rangees, couloir)` de la ligne `moule labyrinthe …` du texte, si le scénario en a une (le texte a déjà
/// été accepté par `developper`, la ligne est donc bien formée).
fn labyrinthe_du_texte(texte: &str) -> Option<(usize, usize, usize)> {
    texte.lines().find_map(|ligne| {
        let mots: Vec<&str> = ligne.split_whitespace().collect();
        match mots.as_slice() {
            ["moule", "labyrinthe", colonnes, rangees, couloir, _graine] => {
                Some((colonnes.parse().ok()?, rangees.parse().ok()?, couloir.parse().ok()?))
            }
            _ => None,
        }
    })
}

/// Les quatorze lignes `nom valeur` des mesures, dans l'ordre des champs de `MesuresPiece` (le format de `mesure_piece`).
fn mesures_en_texte(m: &MesuresPiece) -> String {
    let traversee = |t: Option<usize>| t.map_or_else(|| "aucune".to_string(), |n| n.to_string());
    let mut texte = String::new();
    let _ = writeln!(texte, "cases_libres {}", m.cases_libres);
    let _ = writeln!(texte, "cases_occupees {}", m.cases_occupees);
    let _ = writeln!(texte, "remplissage {}", m.remplissage);
    let _ = writeln!(texte, "densite_moyenne {}", m.densite_moyenne);
    let _ = writeln!(texte, "densite_maximale {}", m.densite_maximale);
    let _ = writeln!(texte, "biomasse {}", m.biomasse);
    let _ = writeln!(texte, "nourriture_initiale {}", m.nourriture_initiale);
    let _ = writeln!(texte, "nourriture_restante {}", m.nourriture_restante);
    let _ = writeln!(texte, "nourriture_consommee {}", m.nourriture_consommee);
    let _ = writeln!(texte, "ecart_du_bilan {}", m.ecart_du_bilan);
    let _ = writeln!(texte, "plus_grand_morceau {}", m.plus_grand_morceau);
    let _ = writeln!(texte, "nombre_de_morceaux {}", m.nombre_de_morceaux);
    let _ = writeln!(texte, "traversee_gauche_droite {}", traversee(m.traversee_gauche_droite));
    let _ = writeln!(texte, "traversee_haut_bas {}", traversee(m.traversee_haut_bas));
    texte
}

/// Le côté d'une case en pixels : 4 si l'image brute tient sous `OCTETS_BRUTS_MAXIMAUX`, sinon le plus grand côté qui
/// tient, au moins 1.
fn pixels_par_case(largeur: usize, hauteur: usize) -> usize {
    (1..=PIXELS_PAR_CASE).rev().find(|k| (hauteur * k) * (largeur * k * 3 + 1) <= OCTETS_BRUTS_MAXIMAUX).unwrap_or(1)
}

/// La couleur d'une case occupée de densité `densite`, sur une rampe de `densite_maximale` (strictement positive).
fn couleur_de_densite(densite: f64, densite_maximale: f64) -> [u8; 3] {
    // Un niveau entier de 0 à 255, sur la racine du rapport (la racine est exacte et la même partout).
    let rapport = (densite / densite_maximale).min(1.0);
    let niveau = (rapport.sqrt() * 255.0) as i32;
    let mut couleur = [0u8; 3];
    for (canal, (sombre, clair)) in couleur.iter_mut().zip(RAMPE_SOMBRE.iter().zip(RAMPE_CLAIRE.iter())) {
        *canal = (sombre + (clair - sombre) * niveau / 255) as u8;
    }
    couleur
}

/// Les octets du PNG de la carte de densité : la case `(x, y)` est à `(x, y) × k` pixels, `y = 0` en haut.
fn image_de_densite(simulation: &Simulation) -> Vec<u8> {
    let reglages = simulation.reglages();
    let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
    let carte: Carte = carte_de_densite(largeur, hauteur, simulation.segments());
    let mut densite_maximale = 0.0f64;
    for &densite in carte.valeurs() {
        densite_maximale = densite_maximale.max(densite);
    }

    let k = pixels_par_case(largeur, hauteur);
    let mut pixels = Vec::with_capacity(largeur * k * hauteur * k * 3);
    for py in 0..hauteur * k {
        let y = py / k;
        for px in 0..largeur * k {
            let x = px / k;
            let couleur = if simulation.substrat().est_bloquee(x, y) {
                BLOQUEE
            } else if carte.valeur(x, y) > 0.0 {
                couleur_de_densite(carte.valeur(x, y), densite_maximale)
            } else {
                FOND
            };
            pixels.extend_from_slice(&couleur);
        }
    }
    ecrire_png((largeur * k) as u32, (hauteur * k) as u32, &pixels)
}
