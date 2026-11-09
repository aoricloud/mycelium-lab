//! `mycelium lancer <scénario> --graine <n> --sortie <dossier>` : lit le scénario, construit l'état, écrit
//! `empreinte.txt` et `apercu.png` dans le dossier de sortie.
//!
//! Un scénario de l'étape 0 (une ligne `boite`) garde son chemin : `empreinte.txt` et `apercu.png`, tirés de
//! [`Etat`]. Un scénario couplé — un texte qui porte une directive `substrat`, `croissance`, `couplage`,
//! `anastomose`, `duree`, `source`, `obstacle`, `chimiotropisme` ou `thigmotropisme` — passe par [`lire_scenario_tropismes`] puis
//! [`Simulation::nouvelle_avec_tropismes`], et écrit en plus `mesures.csv`. Les obstacles du milieu sont dessinés
//! en gris dans `apercu.png`.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mycelium_lab::grille::Etat;
use mycelium_lab::image::ecrire_png;
use mycelium_lab::releves::en_csv;
use mycelium_lab::scenario::Scenario;
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

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

/// Les directives qui font d'un scénario un scénario couplé.
const DIRECTIVES_COUPLEES: [&str; 9] = [
    "substrat",
    "croissance",
    "couplage",
    "anastomose",
    "duree",
    "source",
    "obstacle",
    "chimiotropisme",
    "thigmotropisme",
];

/// Vrai si le texte porte au moins une directive du chemin couplé.
fn est_couple(texte: &str) -> bool {
    texte.lines().any(|brute| {
        let ligne = brute.trim();
        if ligne.is_empty() || ligne.starts_with('#') {
            return false;
        }
        ligne.split_whitespace().next().is_some_and(|mot| DIRECTIVES_COUPLEES.contains(&mot))
    })
}

/// `mycelium lancer` : le chemin de l'étape 0 ou celui du couplage, selon le texte du scénario.
fn executer(a: &Arguments) -> Result<(), String> {
    let texte =
        std::fs::read_to_string(&a.scenario).map_err(|e| format!("lecture de {} : {e}", a.scenario.display()))?;
    if est_couple(&texte) { executer_couple(&texte, a) } else { executer_etape_0(&texte, a) }
}

/// Le chemin de l'étape 0 : une boîte et son état, `empreinte.txt` et `apercu.png`.
fn executer_etape_0(texte: &str, a: &Arguments) -> Result<(), String> {
    let scenario = Scenario::lire(texte).map_err(|e| format!("scénario invalide : {e:?}"))?;
    let etat = Etat::nouveau(&scenario, a.graine);
    std::fs::create_dir_all(&a.sortie).map_err(|e| format!("création de {} : {e}", a.sortie.display()))?;
    ecrire(&a.sortie.join("empreinte.txt"), format!("{}\n", etat.empreinte()).as_bytes())?;
    let png = ecrire_png(etat.largeur, etat.hauteur, &apercu(etat.largeur, etat.hauteur));
    ecrire(&a.sortie.join("apercu.png"), &png)
}

/// Le chemin couplé : la simulation jouée `duree` pas, puis l'empreinte, les mesures et l'aperçu.
fn executer_couple(texte: &str, a: &Arguments) -> Result<(), String> {
    let (reglages, milieu, tropismes, duree) =
        lire_scenario_tropismes(texte).map_err(|e| format!("scénario couplé invalide : {e:?}"))?;
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, a.graine)
        .map_err(|e| format!("réglages, milieu ou tropismes couplés invalides : {e:?}"))?;
    simulation.avancer_n(duree);
    std::fs::create_dir_all(&a.sortie).map_err(|e| format!("création de {} : {e}", a.sortie.display()))?;
    ecrire(&a.sortie.join("empreinte.txt"), format!("{}\n", simulation.empreinte()).as_bytes())?;
    ecrire(&a.sortie.join("mesures.csv"), en_csv(simulation.releves()).as_bytes())?;
    let (largeur, hauteur) = (simulation.substrat().largeur(), simulation.substrat().hauteur());
    let png = ecrire_png((largeur * 4) as u32, (hauteur * 4) as u32, &apercu_couple(&simulation));
    ecrire(&a.sortie.join("apercu.png"), &png)
}

/// Nombre de pixels par case dans l'aperçu couplé.
const PIXELS_PAR_CASE: usize = 4;

/// Couleur claire de la colonie.
const COLONIE: [u8; 3] = [236, 240, 214];

/// Couleur des cases d'obstacle : un gris foncé, net sur un substrat intact (vers 175) et distinct du noir d'une case sans substrat.
const OBSTACLE: [u8; 3] = [48, 48, 48];

/// La boîte vue de dessus : un bloc de `PIXELS_PAR_CASE` pixels par case, dont la luminosité suit le substrat
/// restant, et chaque segment de la colonie tracé en clair par l'algorithme de Bresenham entier.
fn apercu_couple(simulation: &Simulation) -> Vec<u8> {
    let substrat = simulation.substrat();
    let (largeur, hauteur) = (substrat.largeur(), substrat.hauteur());
    let (l, h) = (largeur * PIXELS_PAR_CASE, hauteur * PIXELS_PAR_CASE);

    // Le substrat le plus abondant donne le blanc du fond ; les cases consommées s'assombrissent.
    let mut maximum = 0.0f64;
    for y in 0..hauteur {
        for x in 0..largeur {
            let valeur = substrat.valeur(x, y);
            if valeur > maximum {
                maximum = valeur;
            }
        }
    }

    let mut toile = Toile::nouvelle(l, h);
    for y in 0..hauteur {
        for x in 0..largeur {
            let valeur = substrat.valeur(x, y);
            let rapport = if maximum > 0.0 { valeur / maximum } else { 0.0 };
            // 180 au plus : la colonie, plus claire, reste visible sur le fond le plus lumineux.
            let niveau = (180.0 * rapport) as u8;
            for py in 0..PIXELS_PAR_CASE {
                for px in 0..PIXELS_PAR_CASE {
                    toile.poser((x * PIXELS_PAR_CASE + px) as i64, (y * PIXELS_PAR_CASE + py) as i64, [niveau; 3]);
                }
            }
        }
    }

    for segment in simulation.segments() {
        let x0 = (segment.x0 * PIXELS_PAR_CASE as f64) as i64;
        let y0 = (segment.y0 * PIXELS_PAR_CASE as f64) as i64;
        let x1 = (segment.x1 * PIXELS_PAR_CASE as f64) as i64;
        let y1 = (segment.y1 * PIXELS_PAR_CASE as f64) as i64;
        toile.trace(x0, y0, x1, y1, COLONIE);
    }

    // Les obstacles passent en dernier : chaque case d'obstacle est un bloc d'une seule couleur grise, quel que
    // soit le substrat ou la colonie dessous.
    for obstacle in &simulation.milieu().obstacles {
        for y in obstacle.y0..=obstacle.y1 {
            for x in obstacle.x0..=obstacle.x1 {
                for py in 0..PIXELS_PAR_CASE {
                    for px in 0..PIXELS_PAR_CASE {
                        toile.poser((x * PIXELS_PAR_CASE + px) as i64, (y * PIXELS_PAR_CASE + py) as i64, OBSTACLE);
                    }
                }
            }
        }
    }
    toile.pixels
}

/// Une image RGB en cours de dessin, ligne par ligne.
struct Toile {
    largeur: usize,
    hauteur: usize,
    pixels: Vec<u8>,
}

impl Toile {
    /// Une toile noire de `largeur × hauteur` pixels.
    fn nouvelle(largeur: usize, hauteur: usize) -> Toile {
        Toile { largeur, hauteur, pixels: vec![0u8; largeur * hauteur * 3] }
    }

    /// Pose un pixel, s'il tombe dans la toile.
    fn poser(&mut self, x: i64, y: i64, couleur: [u8; 3]) {
        if x < 0 || y < 0 || x as usize >= self.largeur || y as usize >= self.hauteur {
            return;
        }
        let i = (y as usize * self.largeur + x as usize) * 3;
        self.pixels[i..i + 3].copy_from_slice(&couleur);
    }

    /// Trace un segment entre deux pixels, par l'algorithme de Bresenham entier.
    fn trace(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, couleur: [u8; 3]) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let pas_x = if x0 < x1 { 1 } else { -1 };
        let pas_y = if y0 < y1 { 1 } else { -1 };
        let (mut x, mut y) = (x0, y0);
        let mut erreur = dx + dy;
        loop {
            self.poser(x, y, couleur);
            if x == x1 && y == y1 {
                break;
            }
            let erreur_2 = 2 * erreur;
            if erreur_2 >= dy {
                erreur += dy;
                x += pas_x;
            }
            if erreur_2 <= dx {
                erreur += dx;
                y += pas_y;
            }
        }
    }
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
