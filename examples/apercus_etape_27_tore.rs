//! Les deux vues de l'étape 27, écrites dans `apercus/` :
//!
//! - `etape-27-tore-dessus.png` : la colonie de `scenarios/colonie-tore.txt` (graine 1) dans l'anneau du tore, vue de
//!   dessus à la fin de ses 130 pas (`x` en largeur, `y` en hauteur) ;
//! - `etape-27-tore-cote.png` : la même colonie vue de côté (`x` en largeur, `z` en hauteur, `z = 0` en haut).
//!
//! `cargo run --release --example apercus_etape_27_tore` lit le scénario, le développe par `developper_3d`, construit
//! la [`Colonie3d`], l'avance de sa durée, puis écrit les deux images de 256 × 256 pixels, à 8 pixels par case (256 /
//! largeur pour ce cube de 32). Les cases d'obstacle se dessinent avant les segments : une case de la vue dont toutes
//! les cases de la profondeur sont bloquées est en gris ardoise, une case dont certaines seulement le sont en gris
//! plus sombre, ce qui dessine la silhouette du moule. Chaque segment est projeté sur le plan de la vue et tracé comme
//! un trait d'un pixel, en marchant de son départ à son arrivée par pas d'un quart de pixel au plus ; sa couleur dit sa
//! profondeur, `t = z_milieu / profondeur`, du bleu (`t = 0`) au jaune (`t = 1`) mélangés linéairement. Les segments
//! se tracent dans l'ordre de [`Colonie3d::segments`], un pixel gardant la dernière couleur posée. La case de la
//! goutte, s'il y en a une, est entourée d'un carré blanc ; cette scène n'en a pas. Le rendu n'emploie que `+`, `-`,
//! `*`, `/`, `sqrt`, `floor`, `min`, `max`, `clamp` et des conversions `as` : ni fil, ni horloge, ni argument.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ScenarioColonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::image::ecrire_png;
use mycelium_lab::moule3d::developper_3d;
use mycelium_lab::substrat3d::Substrat3d;

/// La graine de la colonie.
const GRAINE: u64 = 1;

/// Fond sombre commun aux deux vues.
const FOND: [u8; 3] = [8, 9, 12];
/// Gris ardoise d'une case dont toute la profondeur est bloquée.
const ARDOISE: [u8; 3] = [112, 128, 144];
/// Gris plus sombre d'une case dont une partie seulement de la profondeur est bloquée.
const GRIS_SOMBRE: [u8; 3] = [56, 64, 72];
/// Bleu, `t = 0`.
const BLEU: [u8; 3] = [40, 110, 220];
/// Jaune, `t = 1`.
const JAUNE: [u8; 3] = [250, 236, 72];
/// Blanc du carré qui entoure la goutte.
const BLANC: [u8; 3] = [255, 255, 255];
/// Côté d'une case, en pixels : 256 / largeur, ici 256 / 32.
const PIXELS: usize = 8;
/// Côté d'une image, en pixels.
const COTE: usize = 256;
/// Longueur d'un pas de tracé, en pixels.
const PAS_PIXEL: f64 = 0.25;

/// Le plan d'une vue : ce qu'elle met en largeur et en hauteur, et l'axe qu'elle enfouit.
#[derive(Clone, Copy)]
enum Vue {
    /// De dessus : `x` en largeur, `y` en hauteur, `z` enfoui.
    Dessus,
    /// De côté : `x` en largeur, `z` en hauteur (`z = 0` en haut), `y` enfoui.
    Cote,
}

impl Vue {
    /// Les nombres de cases de la vue en largeur et en hauteur.
    fn dimensions(&self, substrat: &Substrat3d) -> (usize, usize) {
        match self {
            Vue::Dessus => (substrat.largeur(), substrat.hauteur()),
            Vue::Cote => (substrat.largeur(), substrat.profondeur()),
        }
    }

    /// Le nombre de cases bloquées et le nombre de cases de la colonne enfouie sous la case
    /// `(colonne, ligne)` de la vue.
    fn colonne(&self, substrat: &Substrat3d, colonne: usize, ligne: usize) -> (usize, usize) {
        let mut bloquees = 0;
        let mut total = 0;
        match self {
            Vue::Dessus => {
                for z in 0..substrat.profondeur() {
                    total += 1;
                    if substrat.est_bloquee(colonne, ligne, z) {
                        bloquees += 1;
                    }
                }
            }
            Vue::Cote => {
                for y in 0..substrat.hauteur() {
                    total += 1;
                    if substrat.est_bloquee(colonne, y, ligne) {
                        bloquees += 1;
                    }
                }
            }
        }
        (bloquees, total)
    }
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

    /// Peint la case `(colonne, ligne)` entière en `couleur`.
    fn case(&mut self, colonne: usize, ligne: usize, couleur: [u8; 3]) {
        let x0 = colonne * PIXELS;
        let y0 = ligne * PIXELS;
        for y in y0..y0 + PIXELS {
            for dx in 0..PIXELS {
                self.poser(x0 + dx, y, couleur);
            }
        }
    }

    /// Entoure la case `(colonne, ligne)` d'un carré d'un pixel d'épaisseur, en `couleur`.
    fn encadrer(&mut self, colonne: usize, ligne: usize, couleur: [u8; 3]) {
        let x0 = colonne * PIXELS;
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

/// La teinte d'une case de la vue selon la profondeur bloquée sous elle : toute la colonne en gris ardoise, une
/// partie seulement en gris plus sombre, aucune en fond.
fn teinte_obstacle(bloquees: usize, total: usize) -> [u8; 3] {
    if bloquees == 0 {
        FOND
    } else if bloquees == total {
        ARDOISE
    } else {
        GRIS_SOMBRE
    }
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

/// Dessine une colonie dans `vue` : les cases d'obstacle d'abord, puis les segments, puis le carré de la goutte.
fn apercu(scenario: &ScenarioColonie3d, colonie: &Colonie3d, vue: Vue) -> Toile {
    let substrat = colonie.substrat();
    let mut toile = Toile::nouvelle(COTE, COTE, FOND);
    let (largeur, hauteur) = vue.dimensions(substrat);
    for ligne in 0..hauteur {
        for colonne in 0..largeur {
            let (bloquees, total) = vue.colonne(substrat, colonne, ligne);
            let teinte = teinte_obstacle(bloquees, total);
            if teinte != FOND {
                toile.case(colonne, ligne, teinte);
            }
        }
    }

    let profondeur = scenario.profondeur as f64;
    for segment in colonie.segments() {
        tracer(&mut toile, segment, profondeur, vue);
    }
    for depot in &scenario.depots {
        match vue {
            Vue::Dessus => toile.encadrer(depot.x as usize, depot.y as usize, BLANC),
            Vue::Cote => toile.encadrer(depot.x as usize, depot.z as usize, BLANC),
        }
    }
    toile
}

fn main() -> Result<(), String> {
    let chemin = Path::new("scenarios/colonie-tore.txt");
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("lecture de {} : {e}", chemin.display()))?;
    let developpe = developper_3d(&texte).map_err(|e| format!("{} : moule refusé {e:?}", chemin.display()))?;
    let scenario = lire_scenario_colonie3d(&developpe).map_err(|e| format!("{} : {e:?}", chemin.display()))?;
    let mut colonie = Colonie3d::nouvelle(&scenario, GRAINE).map_err(|e| format!("construction : {e:?}"))?;
    colonie.avancer_n(scenario.duree);

    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;
    apercu(&scenario, &colonie, Vue::Dessus).ecrire(&dossier.join("etape-27-tore-dessus.png"))?;
    apercu(&scenario, &colonie, Vue::Cote).ecrire(&dossier.join("etape-27-tore-cote.png"))?;
    Ok(())
}
