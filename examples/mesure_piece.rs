//! Mesure de la pièce : ce que la colonie remplit à la fin d'un scénario (étape M0).
//!
//! `cargo run --release --example mesure_piece -- <scénario> <graine> [<dossier de sortie>]`
//!
//! Lit le scénario (`<scénario>` est le chemin d'un fichier, lu par `lire_scenario_tropismes`), fait tourner la
//! simulation pendant sa `duree` avec la graine donnée, puis imprime les mesures de `mesurer` (seuil nul) sur la sortie
//! standard : quatorze lignes `nom valeur`, dans l'ordre des champs de `MesuresPiece`, le nom du champ, une espace, la
//! valeur (les entiers en décimal, les `f64` par `{}`, `aucune` pour une traversée qui n'existe pas), et rien d'autre.
//! Si un dossier de sortie est donné (créé au besoin), il reçoit aussi `densite.png`, la carte de densité en couleurs.
//! Sur une erreur (nombre d'arguments, graine qui n'est pas un entier de 0 à 2⁶⁴ − 1, scénario illisible ou invalide),
//! le message va sur la sortie d'erreur, rien n'est imprimé ni écrit, et le code de sortie est 2.
//!
//! # L'image
//!
//! Chaque case est un carré uniforme de `k × k` pixels, la case `(x, y)` en `(x k, y k)` (la ligne 0 en haut) : `k` est
//! le plus grand entier de 4 à 1 pour lequel les octets bruts de l'image, `(hauteur k) × (3 largeur k + 1)`, tiennent
//! dans 490 000 octets (l'écrivain PNG du dépôt ne compresse pas ; l'image pèse moins de 500 Ko). Une boîte de 100 × 100
//! cases est dessinée à 4 pixels par case, une boîte de 161 × 161 à 2.
//!
//! - une case bloquée est grise (48, 48, 48) ;
//! - une case libre sans hyphe a la couleur du fond (16, 16, 24) ;
//! - une case occupée (densité `d > 0`) suit une rampe du sombre (violet, (70, 20, 110)) au clair (jaune,
//!   (255, 240, 90)) : `niveau = (sqrt(min(d / dmax, 1)) × 255) as i32` (`dmax` la densité maximale de la carte), puis,
//!   canal par canal, `sombre + (clair − sombre) × niveau / 255` en entiers (division entière de Rust). La racine étire
//!   les faibles densités, qui sont presque toute la colonie.
//!
//! Le calcul n'emploie que des entiers, `+ - * /`, `min` et `sqrt`.

use std::path::Path;
use std::process::ExitCode;

use mycelium_lab::image::ecrire_png;
use mycelium_lab::piece::{Carte, MesuresPiece, carte_de_densite, mesurer};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

const USAGE: &str = "usage : mesure_piece <scénario> <graine> [<dossier de sortie>]";

/// Le côté d'une case, en pixels, quand la boîte est assez petite.
const PIXELS_PAR_CASE: usize = 4;
/// Ce que pèsent, au plus, les pixels bruts de l'image (octets de filtre compris) : l'image entière reste sous 500 Ko.
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
    let (reglages, milieu, tropismes, duree) =
        lire_scenario_tropismes(&texte).map_err(|erreur| format!("{} : {erreur:?}", chemin.display()))?;
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine)
        .map_err(|erreur| format!("{} : {erreur:?}", chemin.display()))?;
    simulation.avancer_n(duree);

    // L'image d'abord : si elle ne peut pas s'écrire, rien n'est imprimé.
    if let Some(dossier) = sortie {
        std::fs::create_dir_all(dossier).map_err(|erreur| format!("{} : {erreur}", dossier.display()))?;
        let fichier = dossier.join("densite.png");
        std::fs::write(&fichier, image_de_densite(&simulation))
            .map_err(|erreur| format!("{} : {erreur}", fichier.display()))?;
    }
    imprimer(&mesurer(&simulation));
    Ok(())
}

/// Imprime les mesures, un nombre par ligne : `nom valeur`.
fn imprimer(m: &MesuresPiece) {
    let traversee = |t: Option<usize>| t.map_or_else(|| "aucune".to_string(), |n| n.to_string());
    println!("cases_libres {}", m.cases_libres);
    println!("cases_occupees {}", m.cases_occupees);
    println!("remplissage {}", m.remplissage);
    println!("densite_moyenne {}", m.densite_moyenne);
    println!("densite_maximale {}", m.densite_maximale);
    println!("biomasse {}", m.biomasse);
    println!("nourriture_initiale {}", m.nourriture_initiale);
    println!("nourriture_restante {}", m.nourriture_restante);
    println!("nourriture_consommee {}", m.nourriture_consommee);
    println!("ecart_du_bilan {}", m.ecart_du_bilan);
    println!("plus_grand_morceau {}", m.plus_grand_morceau);
    println!("nombre_de_morceaux {}", m.nombre_de_morceaux);
    println!("traversee_gauche_droite {}", traversee(m.traversee_gauche_droite));
    println!("traversee_haut_bas {}", traversee(m.traversee_haut_bas));
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
