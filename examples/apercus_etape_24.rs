//! Les deux vues de l'étape 24, écrites dans `apercus/` :
//!
//! - `etape-24-anastomose-dessus.png` : la colonie de `scenarios/colonie-anastomose.txt` (graine 1), vue de dessus à
//!   la fin de ses 50 pas (`x` en largeur, `y` en hauteur) ;
//! - `etape-24-anastomose-cote.png` : la même colonie vue de côté (`x` en largeur, `z` en hauteur, `z = 0` en haut).
//!
//! `cargo run --release --example apercus_etape_24` lit le scénario, construit la [`Colonie3d`], l'avance pas par pas
//! jusqu'à sa durée, puis écrit les deux images de 256 × 256 pixels, à 8 pixels par case. Chaque segment est projeté
//! sur le plan de la vue et tracé comme un trait d'un pixel, en marchant de son départ à son arrivée par pas d'un quart
//! de pixel au plus ; sa couleur dit sa profondeur, `t = z_milieu / profondeur`, du bleu (`t = 0`) au jaune (`t = 1`)
//! mélangés linéairement. Les segments se tracent dans l'ordre de [`Colonie3d::segments`], un pixel gardant la dernière
//! couleur posée.
//!
//! Chaque soudure est marquée, après les segments, d'un carré plein de 5 × 5 pixels magenta, centré sur le point où la
//! pointe s'est anastomosée. Sans obstacle, une pointe active au début d'un pas qui laisse un segment pendant ce pas
//! et finit le pas inactive s'est anastomosée, à l'arrivée de ce segment (la doc de [`Colonie3d::avancer`] le dit) ;
//! l'exemple vérifie que leur nombre égale [`Colonie3d::nb_anastomoses`]. Le rendu n'emploie que `+`, `-`, `*`, `/`,
//! `sqrt`, `floor`, `min`, `max`, `clamp` et des conversions `as` : ni fil, ni horloge, ni argument.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::image::ecrire_png;

/// La graine de la colonie.
const GRAINE: u64 = 1;

/// Fond sombre commun aux deux vues.
const FOND: [u8; 3] = [8, 9, 12];
/// Bleu, `t = 0`.
const BLEU: [u8; 3] = [40, 110, 220];
/// Jaune, `t = 1`.
const JAUNE: [u8; 3] = [250, 236, 72];
/// Magenta du carré qui marque une soudure.
const MAGENTA: [u8; 3] = [255, 0, 255];
/// Côté d'une case, en pixels.
const PIXELS: usize = 8;
/// Côté d'une image, en pixels.
const COTE: usize = 256;
/// Longueur d'un pas de tracé, en pixels.
const PAS_PIXEL: f64 = 0.25;
/// Côté du carré d'une soudure, en pixels.
const SOUDURE_COTE: usize = 5;

/// Le plan d'une vue : ce qu'elle met en largeur et en hauteur, et l'axe qu'elle enfouit.
#[derive(Clone, Copy)]
enum Vue {
    /// De dessus : `x` en largeur, `y` en hauteur, `z` enfoui.
    Dessus,
    /// De côté : `x` en largeur, `z` en hauteur (`z = 0` en haut), `y` enfoui.
    Cote,
}

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

    /// Peint un carré plein de `cote` pixels de côté centré sur le pixel `(colonne, ligne)`.
    fn carre(&mut self, colonne: usize, ligne: usize, cote: usize, couleur: [u8; 3]) {
        let avant = (cote / 2) as isize;
        let centre_x = colonne as isize;
        let centre_y = ligne as isize;
        for dy in -avant..=(cote as isize - 1 - avant) {
            for dx in -avant..=(cote as isize - 1 - avant) {
                let (x, y) = (centre_x + dx, centre_y + dy);
                if x >= 0 && y >= 0 {
                    self.poser(x as usize, y as usize, couleur);
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

/// Le pixel d'une coordonnée en cases : `floor(8 * valeur)`, borné à `0..=255`.
fn pixel(valeur: f64) -> usize {
    let p = (valeur * PIXELS as f64).floor();
    p.clamp(0.0, (COTE - 1) as f64) as usize
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

/// Trace un segment en projection sur le plan de la vue, par pas d'un quart de pixel au plus.
fn tracer(toile: &mut Toile, segment: &Segment3d, profondeur: f64, vue: Vue) {
    let teinte = couleur((segment.z0 + segment.z1) / 2.0, profondeur);
    let (x0, v0) = match vue {
        Vue::Dessus => (segment.x0, segment.y0),
        Vue::Cote => (segment.x0, segment.z0),
    };
    let (x1, v1) = match vue {
        Vue::Dessus => (segment.x1, segment.y1),
        Vue::Cote => (segment.x1, segment.z1),
    };
    let dx = (x1 - x0) * PIXELS as f64;
    let dv = (v1 - v0) * PIXELS as f64;
    let longueur = (dx * dx + dv * dv).sqrt();
    let pas = (longueur / PAS_PIXEL) as u64 + 1;
    for i in 0..=pas {
        let part = i as f64 / pas as f64;
        let x = x0 + (x1 - x0) * part;
        let v = v0 + (v1 - v0) * part;
        toile.poser(pixel(x), pixel(v), teinte);
    }
}

/// Joue la colonie pas par pas et rend le point d'arrivée de chaque segment dont la pointe s'est anastomosée.
///
/// Sans obstacle, une pointe active au début d'un pas qui laisse un segment pendant ce pas et finit le pas inactive
/// s'est anastomosée, à l'arrivée de ce segment : c'est ce qu'affirme la doc de [`Colonie3d::avancer`].
fn soudures(colonie: &mut Colonie3d) -> Vec<(f64, f64, f64)> {
    let mut points = Vec::new();
    for _ in 0..colonie.duree() {
        let actifs: Vec<u32> = colonie.apex().iter().filter(|apex| apex.actif).map(|apex| apex.id).collect();
        let avant = colonie.segments().len();
        colonie.avancer();
        let temps = colonie.temps();
        for id in actifs {
            let Some(segment) = colonie.segments()[avant..].iter().find(|s| s.apex == id && s.temps == temps) else {
                continue;
            };
            let inactif = colonie.apex().iter().any(|apex| apex.id == id && !apex.actif);
            if inactif {
                points.push((segment.x1, segment.y1, segment.z1));
            }
        }
    }
    points
}

/// Dessine une colonie dans `vue` : les segments, puis un carré magenta sur chaque soudure.
fn apercu(scenario: &ScenarioColonie3d, colonie: &Colonie3d, vue: Vue, soudures: &[(f64, f64, f64)]) -> Toile {
    let mut toile = Toile::nouvelle(COTE, COTE, FOND);

    let profondeur = scenario.profondeur as f64;
    for segment in colonie.segments() {
        tracer(&mut toile, segment, profondeur, vue);
    }
    for &(x, y, z) in soudures {
        let (colonne, ligne) = match vue {
            Vue::Dessus => (pixel(x), pixel(y)),
            Vue::Cote => (pixel(x), pixel(z)),
        };
        toile.carre(colonne, ligne, SOUDURE_COTE, MAGENTA);
    }
    toile
}

fn main() -> Result<(), String> {
    let chemin = Path::new("scenarios/colonie-anastomose.txt");
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("lecture de {} : {e}", chemin.display()))?;
    let scenario = lire_scenario_colonie3d(&texte).map_err(|e| format!("{} : {e:?}", chemin.display()))?;
    let mut colonie = Colonie3d::nouvelle(&scenario, GRAINE).map_err(|e| format!("construction : {e:?}"))?;
    let soudures = soudures(&mut colonie);
    if soudures.len() as u64 != colonie.nb_anastomoses() {
        return Err(format!("{} soudures trouvées, {} anastomoses comptées", soudures.len(), colonie.nb_anastomoses()));
    }

    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;
    apercu(&scenario, &colonie, Vue::Dessus, &soudures).ecrire(&dossier.join("etape-24-anastomose-dessus.png"))?;
    apercu(&scenario, &colonie, Vue::Cote, &soudures).ecrire(&dossier.join("etape-24-anastomose-cote.png"))?;
    Ok(())
}
