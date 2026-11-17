//! Les deux colonies de l'étape 22, vues de dessus côte à côte, écrites dans `apercus/` :
//!
//! - `etape-22-goutte-et-chimio.png` : la colonie de `scenarios/colonie-goutte.txt` (graine 1, sans
//!   chimiotropisme) à gauche, et celle de `scenarios/colonie-chimio.txt` (graine 1, chimiotropisme 10) à droite,
//!   chacune vue de dessus à la fin de ses 85 pas, la case de la nourriture entourée d'un carré blanc.
//!
//! `cargo run --release --example apercus_etape_22` lit les deux scénarios, construit chaque [`Colonie3d`],
//! l'avance de sa durée, puis écrit une image de 520 × 256 pixels, à 8 pixels par case, `x` en largeur et `y` en
//! hauteur : la colonie sans chimiotropisme occupe les pixels 0 à 255, une bande de fond de 8 pixels les sépare, et
//! celle avec chimiotropisme occupe les pixels 264 à 519. Chaque segment est projeté sur le plan `(x, y)` et tracé
//! comme un trait d'un pixel, en marchant de son départ à son arrivée par pas d'un quart de pixel au plus ; sa
//! couleur dit sa profondeur, `t = z_milieu / profondeur`, du bleu (`t = 0`) au jaune (`t = 1`) mélangés
//! linéairement. Les segments se tracent dans l'ordre de [`Colonie3d::segments`], un pixel gardant la dernière
//! couleur posée. Le rendu n'emploie que `+`, `-`, `*`, `/`, `sqrt`, `floor`, `min`, `max`, `clamp` et des
//! conversions `as` : ni fil, ni horloge, ni argument.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::image::ecrire_png;

/// La graine des deux colonies.
const GRAINE: u64 = 1;

/// Fond sombre commun aux deux vues.
const FOND: [u8; 3] = [8, 9, 12];
/// Bleu, `t = 0`.
const BLEU: [u8; 3] = [40, 110, 220];
/// Jaune, `t = 1`.
const JAUNE: [u8; 3] = [250, 236, 72];
/// Blanc du carré qui entoure la nourriture.
const BLANC: [u8; 3] = [255, 255, 255];
/// Côté d'une case, en pixels.
const PIXELS: usize = 8;
/// Côté d'une vue, en pixels.
const COTE: usize = 256;
/// Largeur de la bande de fond entre les deux vues, en pixels.
const BANDE: usize = 8;
/// Largeur de l'image, en pixels.
const LARGEUR: usize = COTE + BANDE + COTE;
/// Colonne du premier pixel de la vue de droite : `256 + 8`.
const DECALAGE_DROITE: usize = COTE + BANDE;
/// Longueur d'un pas de tracé, en pixels.
const PAS_PIXEL: f64 = 0.25;

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

    /// Peint le pixel `(colonne, ligne)` en `couleur`, s'il est dans la toile.
    fn poser(&mut self, colonne: usize, ligne: usize, couleur: [u8; 3]) {
        if colonne >= self.largeur || ligne >= self.hauteur {
            return;
        }
        let i = (ligne * self.largeur + colonne) * 3;
        self.pixels[i..i + 3].copy_from_slice(&couleur);
    }

    /// Entoure la case `(colonne, ligne)` d'un carré d'un pixel d'épaisseur, en `couleur`, la case commençant au
    /// pixel `decalage`.
    fn encadrer(&mut self, colonne: usize, ligne: usize, decalage: usize, couleur: [u8; 3]) {
        let x0 = decalage + colonne * PIXELS;
        let y0 = ligne * PIXELS;
        for d in 0..PIXELS {
            self.poser(x0 + d, y0, couleur);
            self.poser(x0 + d, y0 + PIXELS - 1, couleur);
            self.poser(x0, y0 + d, couleur);
            self.poser(x0 + PIXELS - 1, y0 + d, couleur);
        }
    }

    /// Écrit la toile en PNG à `chemin`.
    fn ecrire(&self, chemin: &Path) -> Result<(), String> {
        let png = ecrire_png(self.largeur as u32, self.hauteur as u32, &self.pixels);
        std::fs::write(chemin, png).map_err(|e| format!("écriture de {} : {e}", chemin.display()))
    }
}

/// Le pixel d'une coordonnée en cases : `floor(8 * valeur)`, borné à `0..=255`, plus `decalage`.
fn pixel(valeur: f64, decalage: usize) -> usize {
    let p = (valeur * PIXELS as f64).floor();
    p.clamp(0.0, (COTE - 1) as f64) as usize + decalage
}

/// La couleur d'un segment de profondeur `z_milieu`, sur `profondeur` cases : `t` va de 0 à 1, la couleur de bleu à
/// jaune, composante par composante.
fn couleur(z_milieu: f64, profondeur: f64) -> [u8; 3] {
    let mut t = 0.0;
    if profondeur > 0.0 {
        t = (z_milieu / profondeur).clamp(0.0, 1.0);
    }
    let mut sortie = [0u8; 3];
    for i in 0..3 {
        let bleu = BLEU[i] as f64;
        let jaune = JAUNE[i] as f64;
        sortie[i] = (bleu + (jaune - bleu) * t) as u8;
    }
    sortie
}

/// Trace un segment en projection sur le plan `(x, y)`, par pas d'un quart de pixel au plus, la vue commençant au
/// pixel `decalage`.
fn tracer(toile: &mut Toile, segment: &Segment3d, profondeur: f64, decalage: usize) {
    let teinte = couleur((segment.z0 + segment.z1) / 2.0, profondeur);
    let dx = (segment.x1 - segment.x0) * PIXELS as f64;
    let dy = (segment.y1 - segment.y0) * PIXELS as f64;
    let longueur = (dx * dx + dy * dy).sqrt();
    let pas = (longueur / PAS_PIXEL) as u64 + 1;
    for i in 0..=pas {
        let part = i as f64 / pas as f64;
        let x = segment.x0 + (segment.x1 - segment.x0) * part;
        let y = segment.y0 + (segment.y1 - segment.y0) * part;
        toile.poser(pixel(x, decalage), pixel(y, 0), teinte);
    }
}

/// Lit un scénario, avance sa colonie de sa durée et la dessine sur `toile`, à partir du pixel `decalage`.
fn dessiner(toile: &mut Toile, chemin: &Path, decalage: usize) -> Result<(), String> {
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("lecture de {} : {e}", chemin.display()))?;
    let scenario = lire_scenario_colonie3d(&texte).map_err(|e| format!("{} : {e:?}", chemin.display()))?;
    let mut colonie = Colonie3d::nouvelle(&scenario, GRAINE).map_err(|e| format!("construction : {e:?}"))?;
    colonie.avancer_n(scenario.duree);

    let profondeur = scenario.profondeur as f64;
    for segment in colonie.segments() {
        tracer(toile, segment, profondeur, decalage);
    }
    for depot in &scenario.depots {
        toile.encadrer(depot.x as usize, depot.y as usize, decalage, BLANC);
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;

    let mut toile = Toile::nouvelle(LARGEUR, COTE, FOND);
    dessiner(&mut toile, Path::new("scenarios/colonie-goutte.txt"), 0)?;
    dessiner(&mut toile, Path::new("scenarios/colonie-chimio.txt"), DECALAGE_DROITE)?;
    toile.ecrire(&dossier.join("etape-22-goutte-et-chimio.png"))
}
