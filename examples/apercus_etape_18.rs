//! Les trois coupes de l'étape 18, écrites dans `apercus/` :
//!
//! - `etape-18-mur-dessus.png` : la tranche `z = 8`, celle de la goutte, `x` en largeur et `y` en hauteur ;
//! - `etape-18-mur-face.png` : la tranche `y = 16`, `x` en largeur et `z` en hauteur, `z = 0` en haut ;
//! - `etape-18-mur-cote.png` : la tranche `x = 16`, `y` en largeur et `z` en hauteur, `z = 0` en haut.
//!
//! `cargo run --release --example apercus_etape_18` lit `scenarios/cube-mur.txt`, construit le [`Cube`], l'avance
//! de sa durée, puis écrit les trois images. Une case bloquée est en gris ardoise ; une case libre de valeur `v`
//! se colore sur une même échelle, `t = sqrt(v / m)` où `m` est le maximum des cases libres des trois tranches,
//! du bleu nuit (`t = 0`) au jaune (`t = 1`) en passant par le vert. Le rendu n'emploie que `+`, `-`, `*`, `/`,
//! `sqrt`, `min`, `max` et des conversions `as` : ni fil, ni horloge, ni argument.

use std::path::Path;

use mycelium_lab::cube::{Cube, lire_scenario_cube};
use mycelium_lab::image::ecrire_png;

/// Fond sombre commun aux trois images.
const FOND: [u8; 3] = [8, 9, 12];
/// Gris ardoise d'une case bloquée.
const ARDOISE: [u8; 3] = [112, 128, 144];
/// Bleu nuit, `t = 0`.
const BLEU_NUIT: [u8; 3] = [16, 32, 88];
/// Vert, au milieu de l'échelle.
const VERT: [u8; 3] = [44, 194, 96];
/// Jaune, `t = 1`.
const JAUNE: [u8; 3] = [250, 236, 72];
/// Côté d'une case, en pixels.
const PIXELS: usize = 8;

/// Les indices des trois coupes, dans le cube de `scenarios/cube-mur.txt`.
const GOUTTE_Z: usize = 8;
const FACE_Y: usize = 16;
const COTE_X: usize = 16;

/// Une image RGB, ligne par ligne, prête pour [`ecrire_png`].
struct Toile {
    largeur: usize,
    hauteur: usize,
    pixels: Vec<u8>,
}

impl Toile {
    /// Une toile remplie d'une couleur uniforme.
    fn nouvelle(largeur: usize, hauteur: usize, fond: [u8; 3]) -> Toile {
        let mut pixels = Vec::with_capacity(largeur * hauteur * 3);
        for _ in 0..largeur * hauteur {
            pixels.extend_from_slice(&fond);
        }
        Toile { largeur, hauteur, pixels }
    }

    /// Peint la case `(colonne, ligne)` entière en `couleur`.
    fn case(&mut self, colonne: usize, ligne: usize, couleur: [u8; 3]) {
        let x0 = colonne * PIXELS;
        let y0 = ligne * PIXELS;
        for y in y0..y0 + PIXELS {
            let debut = (y * self.largeur + x0) * 3;
            for dx in 0..PIXELS {
                let i = debut + dx * 3;
                self.pixels[i..i + 3].copy_from_slice(&couleur);
            }
        }
    }

    /// Écrit la toile en PNG à `chemin`.
    fn ecrire(&self, chemin: &Path) -> Result<(), String> {
        let png = ecrire_png(self.largeur as u32, self.hauteur as u32, &self.pixels);
        std::fs::write(chemin, png).map_err(|e| format!("écriture de {} : {e}", chemin.display()))
    }
}

/// La couleur d'une case libre de valeur `valeur`, sur l'échelle commune de maximum `maximum` : `t` va de 0 à 1,
/// la couleur de bleu nuit à jaune en passant par le vert, composante par composante.
fn couleur(valeur: f64, maximum: f64) -> [u8; 3] {
    let mut t = 0.0;
    if maximum > 0.0 {
        t = (valeur / maximum).sqrt();
    }
    let (bas, haut, part) = if t <= 0.5 { (BLEU_NUIT, VERT, t * 2.0) } else { (VERT, JAUNE, (t - 0.5) * 2.0) };
    let mut sortie = [0u8; 3];
    for i in 0..3 {
        let a = bas[i] as f64;
        let b = haut[i] as f64;
        sortie[i] = (a + (b - a) * part) as u8;
    }
    sortie
}

/// Le maximum des valeurs des cases libres des trois tranches.
fn maximum_des_tranches(cube: &Cube) -> f64 {
    let substrat = cube.substrat();
    let mut maximum = 0.0f64;
    for y in 0..substrat.hauteur() {
        for x in 0..substrat.largeur() {
            if !substrat.est_bloquee(x, y, GOUTTE_Z) {
                let valeur = substrat.valeur(x, y, GOUTTE_Z);
                if valeur > maximum {
                    maximum = valeur;
                }
            }
        }
    }
    for z in 0..substrat.profondeur() {
        for x in 0..substrat.largeur() {
            if !substrat.est_bloquee(x, FACE_Y, z) {
                let valeur = substrat.valeur(x, FACE_Y, z);
                if valeur > maximum {
                    maximum = valeur;
                }
            }
        }
    }
    for z in 0..substrat.profondeur() {
        for y in 0..substrat.hauteur() {
            if !substrat.est_bloquee(COTE_X, y, z) {
                let valeur = substrat.valeur(COTE_X, y, z);
                if valeur > maximum {
                    maximum = valeur;
                }
            }
        }
    }
    maximum
}

/// La tranche `z = 8` : `x` en largeur, `y` en hauteur.
fn apercu_dessus(cube: &Cube, maximum: f64) -> Toile {
    let substrat = cube.substrat();
    let mut toile = Toile::nouvelle(substrat.largeur() * PIXELS, substrat.hauteur() * PIXELS, FOND);
    for y in 0..substrat.hauteur() {
        for x in 0..substrat.largeur() {
            let teinte = if substrat.est_bloquee(x, y, GOUTTE_Z) {
                ARDOISE
            } else {
                couleur(substrat.valeur(x, y, GOUTTE_Z), maximum)
            };
            toile.case(x, y, teinte);
        }
    }
    toile
}

/// La tranche `y = 16` : `x` en largeur, `z` en hauteur, `z = 0` en haut.
fn apercu_face(cube: &Cube, maximum: f64) -> Toile {
    let substrat = cube.substrat();
    let mut toile = Toile::nouvelle(substrat.largeur() * PIXELS, substrat.profondeur() * PIXELS, FOND);
    for z in 0..substrat.profondeur() {
        for x in 0..substrat.largeur() {
            let teinte = if substrat.est_bloquee(x, FACE_Y, z) {
                ARDOISE
            } else {
                couleur(substrat.valeur(x, FACE_Y, z), maximum)
            };
            toile.case(x, z, teinte);
        }
    }
    toile
}

/// La tranche `x = 16` : `y` en largeur, `z` en hauteur, `z = 0` en haut.
fn apercu_cote(cube: &Cube, maximum: f64) -> Toile {
    let substrat = cube.substrat();
    let mut toile = Toile::nouvelle(substrat.hauteur() * PIXELS, substrat.profondeur() * PIXELS, FOND);
    for z in 0..substrat.profondeur() {
        for y in 0..substrat.hauteur() {
            let teinte = if substrat.est_bloquee(COTE_X, y, z) {
                ARDOISE
            } else {
                couleur(substrat.valeur(COTE_X, y, z), maximum)
            };
            toile.case(y, z, teinte);
        }
    }
    toile
}

fn main() -> Result<(), String> {
    let chemin = Path::new("scenarios/cube-mur.txt");
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("lecture de {} : {e}", chemin.display()))?;
    let scenario = lire_scenario_cube(&texte).map_err(|e| format!("{} : {e:?}", chemin.display()))?;
    let mut cube = Cube::nouveau(&scenario).map_err(|e| format!("construction du cube : {e:?}"))?;
    cube.avancer(scenario.duree);
    let maximum = maximum_des_tranches(&cube);

    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;
    apercu_dessus(&cube, maximum).ecrire(&dossier.join("etape-18-mur-dessus.png"))?;
    apercu_face(&cube, maximum).ecrire(&dossier.join("etape-18-mur-face.png"))?;
    apercu_cote(&cube, maximum).ecrire(&dossier.join("etape-18-mur-cote.png"))?;
    Ok(())
}
