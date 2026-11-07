//! Les trois aperçus de l'étape 1, écrits dans `apercus/` :
//!
//! - `etape-1-arbre.png` : une colonie partie d'une spore après quelques centaines de pas, ses segments
//!   tracés clairs sur fond sombre ;
//! - `etape-1-pic.png` : un pic de substrat déposé au centre, puis étalé par diffusion, en niveaux de gris ;
//! - `etape-1-sierpinski.png` : le tapis de Sierpinski de niveau 5.
//!
//! `cargo run --example apercus_etape_1` les écrit. Tout le calcul reste déterministe sur toutes les
//! machines : la colonie vient des tables de [`mycelium_lab::croissance`], le substrat n'utilise que `+`,
//! `-`, `*` et `/`, et le rendu n'emploie que `+`, `-`, `*`, `/` et `sqrt` (aucune fonction de `f64` qui
//! passe par la bibliothèque mathématique du système).

use std::path::Path;

use mycelium_lab::croissance::{Colonie, Parametres};
use mycelium_lab::image::ecrire_png;
use mycelium_lab::substrat::Substrat;

/// Fond sombre commun aux aperçus.
const FOND: [u8; 3] = [16, 18, 22];
/// Couleur claire des hyphes.
const HYPHE: [u8; 3] = [208, 226, 178];
/// Couleur de la spore, au centre de la colonie.
const SPORE: [u8; 3] = [240, 232, 196];

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

    /// Pose un pixel, s'il tombe dans la toile.
    fn poser(&mut self, x: i64, y: i64, couleur: [u8; 3]) {
        if x < 0 || y < 0 || x as usize >= self.largeur || y as usize >= self.hauteur {
            return;
        }
        let i = (y as usize * self.largeur + x as usize) * 3;
        self.pixels[i..i + 3].copy_from_slice(&couleur);
    }

    /// Pose un petit carré de 2 × 2 pixels : un trait d'un pixel serait trop fin à cette échelle.
    fn tache(&mut self, x: i64, y: i64, couleur: [u8; 3]) {
        for dy in 0..2 {
            for dx in 0..2 {
                self.poser(x + dx, y + dy, couleur);
            }
        }
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
            self.tache(x, y, couleur);
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

    /// Pose un disque plein de rayon `rayon`, par test entier de distance au carré.
    fn disque(&mut self, cx: i64, cy: i64, rayon: i64, couleur: [u8; 3]) {
        for dy in -rayon..=rayon {
            for dx in -rayon..=rayon {
                if dx * dx + dy * dy <= rayon * rayon {
                    self.poser(cx + dx, cy + dy, couleur);
                }
            }
        }
    }

    /// Écrit la toile en PNG à `chemin`.
    fn ecrire(&self, chemin: &Path) -> Result<(), String> {
        let png = ecrire_png(self.largeur as u32, self.hauteur as u32, &self.pixels);
        std::fs::write(chemin, png).map_err(|e| format!("écriture de {} : {e}", chemin.display()))
    }
}

/// Arrondit un `f64` à l'entier le plus proche sans fonction de la bibliothèque mathématique : le cast
/// `as i64` tronque vers zéro, on ajoute donc `±0.5` avant.
fn arrondi(v: f64) -> i64 {
    if v >= 0.0 { (v + 0.5) as i64 } else { (v - 0.5) as i64 }
}

/// Une colonie partie d'une spore à l'origine, après `pas` pas, ses segments tracés clairs.
fn apercu_arbre() -> Toile {
    const COTE: usize = 720;
    const MARGE: f64 = 26.0;

    let parametres = Parametres { pas: 1.0, intervalle: 60.0, angle_ramification: 30, amplitude_marche: 12 };
    let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, 1);
    colonie.avancer_n(380);

    // Boîte englobante de tous les segments, pour que l'arbre remplisse la toile.
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for segment in colonie.segments() {
        for x in [segment.x0, segment.x1] {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
        }
        for y in [segment.y0, segment.y1] {
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }
    }

    let disponible = COTE as f64 - 2.0 * MARGE;
    let mut largeur_monde = max_x - min_x;
    let mut hauteur_monde = max_y - min_y;
    if largeur_monde < 1e-9 {
        largeur_monde = 1e-9;
    }
    if hauteur_monde < 1e-9 {
        hauteur_monde = 1e-9;
    }
    let echelle_x = disponible / largeur_monde;
    let echelle_y = disponible / hauteur_monde;
    let echelle = if echelle_x < echelle_y { echelle_x } else { echelle_y };
    let decalage_x = MARGE + (disponible - largeur_monde * echelle) / 2.0;
    let decalage_y = MARGE + (disponible - hauteur_monde * echelle) / 2.0;
    // L'ordonnée de l'image descend, celle du plan monte : on retourne l'axe y.
    let vers_x = |x: f64| arrondi(decalage_x + (x - min_x) * echelle);
    let vers_y = |y: f64| arrondi(COTE as f64 - decalage_y - (y - min_y) * echelle);

    let mut toile = Toile::nouvelle(COTE, COTE, FOND);
    for segment in colonie.segments() {
        toile.trace(vers_x(segment.x0), vers_y(segment.y0), vers_x(segment.x1), vers_y(segment.y1), HYPHE);
    }
    let (cx, cy) = (vers_x(0.0), vers_y(0.0));
    toile.disque(cx, cy, 4, SPORE);
    toile
}

/// Un pic de substrat déposé au centre d'une grille, puis étalé par diffusion, en niveaux de gris.
fn apercu_pic() -> Toile {
    const COTE: usize = 301;
    const PAS: usize = 600;

    let mut substrat = Substrat::nouveau(COTE, COTE, 1.0, 0.25, 1.0).expect("paramètres de diffusion valides");
    let centre = COTE / 2;
    substrat.deposer(centre, centre, 1.0);
    substrat.pas_n(PAS);

    let mut maximum = 0.0f64;
    for y in 0..COTE {
        for x in 0..COTE {
            let valeur = substrat.valeur(x, y);
            if valeur > maximum {
                maximum = valeur;
            }
        }
    }

    let mut toile = Toile::nouvelle(COTE, COTE, [0, 0, 0]);
    for y in 0..COTE {
        for x in 0..COTE {
            // La racine carrée éclaircit les faibles valeurs, pour voir la queue du pic.
            let mut rapport = substrat.valeur(x, y) / maximum;
            if rapport < 0.0 {
                rapport = 0.0;
            }
            let niveau = arrondi(255.0 * rapport.sqrt()).clamp(0, 255) as u8;
            let i = (y * COTE + x) * 3;
            toile.pixels[i..i + 3].copy_from_slice(&[niveau, niveau, niveau]);
        }
    }
    toile
}

/// Le tapis de Sierpinski de niveau 5 : une case est vide si, dans l'écriture en base 3 de ses deux
/// coordonnées, un rang a les deux chiffres égaux à 1.
fn apercu_sierpinski() -> Toile {
    const NIVEAU: u32 = 5;
    let cote = 3usize.pow(NIVEAU);

    let mut toile = Toile::nouvelle(cote, cote, [0, 0, 0]);
    for y in 0..cote {
        for x in 0..cote {
            let (mut a, mut b) = (x, y);
            let mut pleine = true;
            while a > 0 || b > 0 {
                if a % 3 == 1 && b % 3 == 1 {
                    pleine = false;
                    break;
                }
                a /= 3;
                b /= 3;
            }
            if pleine {
                let i = (y * cote + x) * 3;
                toile.pixels[i..i + 3].copy_from_slice(&[236, 236, 236]);
            }
        }
    }
    toile
}

fn main() -> Result<(), String> {
    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;
    apercu_arbre().ecrire(&dossier.join("etape-1-arbre.png"))?;
    apercu_pic().ecrire(&dossier.join("etape-1-pic.png"))?;
    apercu_sierpinski().ecrire(&dossier.join("etape-1-sierpinski.png"))?;
    Ok(())
}
