//! `mycelium lancer <scénario> --graine <n> --sortie <dossier>` : lit le scénario, construit l'état, écrit
//! `empreinte.txt` et `apercu.png` dans le dossier de sortie.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mycelium_lab::grille::Etat;
use mycelium_lab::image::ecrire_png;
use mycelium_lab::scenario::Scenario;

const USAGE: &str = "usage : mycelium lancer <scénario> --graine <n> --sortie <dossier>";

struct Arguments {
    scenario: PathBuf,
    graine: u64,
    sortie: PathBuf,
}

fn lire_arguments(args: &[String]) -> Option<Arguments> {
    let [commande, reste @ ..] = args else {
        return None;
    };
    if commande != "lancer" {
        return None;
    }
    let mut scenario = None;
    let mut graine = None;
    let mut sortie = None;
    let mut i = 0;
    while i < reste.len() {
        match reste[i].as_str() {
            "--graine" => {
                let v = reste.get(i + 1)?;
                if graine.is_some() || v.is_empty() || !v.bytes().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                graine = Some(v.parse::<u64>().ok()?);
                i += 2;
            }
            "--sortie" => {
                let v = reste.get(i + 1)?;
                if sortie.is_some() {
                    return None;
                }
                sortie = Some(PathBuf::from(v));
                i += 2;
            }
            mot => {
                if mot.starts_with("--") || scenario.is_some() {
                    return None;
                }
                scenario = Some(PathBuf::from(mot));
                i += 1;
            }
        }
    }
    Some(Arguments { scenario: scenario?, graine: graine?, sortie: sortie? })
}

/// La boîte vue de dessus : le disque inscrit dans la grille, clair sur fond sombre, en entiers.
fn apercu(largeur: u32, hauteur: u32) -> Vec<u8> {
    const FOND: [u8; 3] = [24, 24, 28];
    const DISQUE: [u8; 3] = [214, 208, 190];
    let (l, h) = (i64::from(largeur), i64::from(hauteur));
    let diametre = l.min(h);
    let mut pixels = Vec::with_capacity(largeur as usize * hauteur as usize * 3);
    for y in 0..h {
        for x in 0..l {
            // Coordonnées doublées, relatives au centre de la grille.
            let dx = 2 * x + 1 - l;
            let dy = 2 * y + 1 - h;
            let dedans = dx * dx + dy * dy <= diametre * diametre;
            pixels.extend_from_slice(if dedans { &DISQUE } else { &FOND });
        }
    }
    pixels
}

fn executer(a: &Arguments) -> Result<(), String> {
    let texte =
        std::fs::read_to_string(&a.scenario).map_err(|e| format!("lecture de {} : {e}", a.scenario.display()))?;
    let scenario = Scenario::lire(&texte).map_err(|e| format!("scénario invalide : {e:?}"))?;
    let etat = Etat::nouveau(&scenario, a.graine);
    std::fs::create_dir_all(&a.sortie).map_err(|e| format!("création de {} : {e}", a.sortie.display()))?;
    ecrire(&a.sortie.join("empreinte.txt"), format!("{}\n", etat.empreinte()).as_bytes())?;
    let png = ecrire_png(etat.largeur, etat.hauteur, &apercu(etat.largeur, etat.hauteur));
    ecrire(&a.sortie.join("apercu.png"), &png)
}

fn ecrire(chemin: &Path, octets: &[u8]) -> Result<(), String> {
    std::fs::write(chemin, octets).map_err(|e| format!("écriture de {} : {e}", chemin.display()))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(arguments) = lire_arguments(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match executer(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("mycelium : {message}");
            ExitCode::FAILURE
        }
    }
}
