//! Les deux aperçus de l'étape 2, écrits dans `apercus/` :
//!
//! - `etape-2-colonie.png` : la colonie du scénario `scenarios/couplage.txt` (graine 1, 80 pas), à
//!   4 pixels par case, ses segments clairs sur un fond dont la luminosité suit le substrat restant ;
//! - `etape-2-animation.png` : la même colonie aux temps 10, 20, 30, 45, 60 et 80, à 2 pixels par case,
//!   en deux lignes de trois, pour voir la colonie pousser et le substrat s'épuiser derrière elle.
//!
//! `cargo run --example apercus_etape_2` les écrit. Le rendu n'emploie que `+`, `-`, `*`, `/`, `sqrt` et des
//! conversions `as` : ni `round` ni `floor`, donc le même résultat sur toutes les machines.

use std::path::Path;

use mycelium_lab::croissance::Segment;
use mycelium_lab::image::ecrire_png;
use mycelium_lab::simulation::{Reglages, Simulation, lire_scenario};
use mycelium_lab::substrat::Substrat;

/// Couleur claire des hyphes.
const HYPHE: [u8; 3] = [226, 238, 198];
/// Couleur de la spore, à son point de départ.
const SPORE: [u8; 3] = [250, 244, 210];
/// Fond des séparateurs de la planche.
const SEPARATEUR: [u8; 3] = [6, 7, 9];

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

    /// Remplit un rectangle de `largeur` × `hauteur` pixels, à partir du coin `(x, y)`.
    fn rectangle(&mut self, x: i64, y: i64, largeur: i64, hauteur: i64, couleur: [u8; 3]) {
        for dy in 0..hauteur {
            for dx in 0..largeur {
                self.poser(x + dx, y + dy, couleur);
            }
        }
    }

    /// Pose un carré de `cote` pixels dont le coin est centré sur `(x, y)`.
    fn tache(&mut self, x: i64, y: i64, cote: i64, couleur: [u8; 3]) {
        let demi = cote / 2;
        self.rectangle(x - demi, y - demi, cote, cote, couleur);
    }

    /// Trace un segment entre deux pixels, par l'algorithme de Bresenham entier, avec un pinceau carré.
    fn trace(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, cote: i64, couleur: [u8; 3]) {
        let mut dx = x1 - x0;
        if dx < 0 {
            dx = -dx;
        }
        let mut dy = y1 - y0;
        if dy < 0 {
            dy = -dy;
        }
        let pas_x = if x0 < x1 { 1 } else { -1 };
        let pas_y = if y0 < y1 { 1 } else { -1 };
        let mut erreur = dx - dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.tache(x, y, cote, couleur);
            if x == x1 && y == y1 {
                break;
            }
            let erreur_2 = 2 * erreur;
            if erreur_2 > -dy {
                erreur -= dy;
                x += pas_x;
            }
            if erreur_2 < dx {
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

    /// Copie la toile dans `cible`, au coin `(ox, oy)`.
    fn copier_dans(&self, cible: &mut Toile, ox: i64, oy: i64) {
        for y in 0..self.hauteur {
            for x in 0..self.largeur {
                let i = (y * self.largeur + x) * 3;
                let couleur = [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]];
                cible.poser(ox + x as i64, oy + y as i64, couleur);
            }
        }
    }

    /// Écrit la toile en PNG à `chemin`.
    fn ecrire(&self, chemin: &Path) -> Result<(), String> {
        let png = ecrire_png(self.largeur as u32, self.hauteur as u32, &self.pixels);
        std::fs::write(chemin, png).map_err(|e| format!("écriture de {} : {e}", chemin.display()))
    }
}

/// La couleur de fond d'une case : plus il reste de substrat, plus elle est lumineuse.
///
/// L'intervalle `minimum..maximum` observé sur toute la simulation est étalé sur la plage de luminosité, pour
/// que les faibles différences de substrat restant se voient. La puissance huit accentue le contraste près du
/// plein et laisse apparaître les traînées d'épuisement derrière la colonie.
fn couleur_fond(valeur: f64, minimum: f64, maximum: f64) -> [u8; 3] {
    let etendue = maximum - minimum;
    let rapport = if etendue > 0.0 { (valeur - minimum) / etendue } else { 1.0 };
    let rapport_2 = rapport * rapport;
    let rapport_4 = rapport_2 * rapport_2;
    let rapport_8 = rapport_4 * rapport_4;
    let niveau = (10.0 + 100.0 * rapport_8) as u8;
    [niveau, niveau + 2, niveau + 6]
}

/// Dessine une colonie et le substrat restant, à `echelle` pixels par case.
fn dessiner(
    reglages: &Reglages,
    echelle: usize,
    segments: &[Segment],
    substrat: &Substrat,
    minimum: f64,
    maximum: f64,
    cote_trait: i64,
) -> Toile {
    let mut toile = Toile::nouvelle(reglages.largeur * echelle, reglages.hauteur * echelle, [0, 0, 0]);
    let e = echelle as i64;

    for y in 0..reglages.hauteur {
        for x in 0..reglages.largeur {
            let couleur = couleur_fond(substrat.valeur(x, y), minimum, maximum);
            toile.rectangle(x as i64 * e, y as i64 * e, e, e, couleur);
        }
    }

    let e_f = echelle as f64;
    for segment in segments {
        let x0 = (segment.x0 * e_f) as i64;
        let y0 = (segment.y0 * e_f) as i64;
        let x1 = (segment.x1 * e_f) as i64;
        let y1 = (segment.y1 * e_f) as i64;
        toile.trace(x0, y0, x1, y1, cote_trait, HYPHE);
    }

    let cx = ((reglages.spore.0 as f64 + 0.5) * e_f) as i64;
    let cy = ((reglages.spore.1 as f64 + 0.5) * e_f) as i64;
    let rayon = if echelle >= 2 { (echelle / 2) as i64 } else { 1 };
    toile.disque(cx, cy, rayon, SPORE);
    toile
}

/// Assemble six états de la colonie en deux lignes de trois, à 2 pixels par case.
fn planche(
    reglages: &Reglages,
    segments: &[Segment],
    etats: &[(Substrat, usize)],
    minimum: f64,
    maximum: f64,
) -> Toile {
    const ECHELLE: usize = 2;
    const COLONNES: usize = 3;
    const LIGNES: usize = 2;
    const ECART: usize = 4;

    let panneau_largeur = reglages.largeur * ECHELLE;
    let panneau_hauteur = reglages.hauteur * ECHELLE;
    let largeur = COLONNES * panneau_largeur + (COLONNES - 1) * ECART;
    let hauteur = LIGNES * panneau_hauteur + (LIGNES - 1) * ECART;
    let mut planche = Toile::nouvelle(largeur, hauteur, SEPARATEUR);

    for (rang, (substrat, nb_segments)) in etats.iter().enumerate() {
        let colonne = rang % COLONNES;
        let ligne = rang / COLONNES;
        let ox = colonne * (panneau_largeur + ECART);
        let oy = ligne * (panneau_hauteur + ECART);
        let panneau = dessiner(reglages, ECHELLE, &segments[..*nb_segments], substrat, minimum, maximum, 1);
        panneau.copier_dans(&mut planche, ox as i64, oy as i64);
    }

    planche
}

fn main() -> Result<(), String> {
    // La graine demandée pour tous les aperçus.
    const GRAINE: u64 = 1;
    // Les temps de la planche, dans l'ordre de lecture.
    const TEMPS: [u32; 6] = [10, 20, 30, 45, 60, 80];

    let chemin = Path::new("scenarios/couplage.txt");
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("lecture de {} : {e}", chemin.display()))?;
    let (reglages, _duree) = lire_scenario(&texte).map_err(|e| format!("lecture de {} : {e:?}", chemin.display()))?;

    let mut simulation =
        Simulation::nouvelle(reglages.clone(), GRAINE).map_err(|e| format!("construction de la simulation : {e:?}"))?;

    let mut etats: Vec<(Substrat, usize)> = Vec::new();
    for temps in TEMPS {
        while simulation.temps() < temps {
            simulation.avancer();
        }
        etats.push((simulation.substrat().clone(), simulation.segments().len()));
    }

    // La même échelle de luminosité pour tous les temps : le fond s'assombrit à mesure que le substrat s'épuise.
    let (mut minimum, mut maximum) = (f64::INFINITY, f64::NEG_INFINITY);
    for (substrat, _) in &etats {
        for y in 0..substrat.hauteur() {
            for x in 0..substrat.largeur() {
                let valeur = substrat.valeur(x, y);
                if valeur < minimum {
                    minimum = valeur;
                }
                if valeur > maximum {
                    maximum = valeur;
                }
            }
        }
    }

    let segments = simulation.segments();
    let (substrat_final, _) = etats.last().expect("le dernier état existe");
    let colonie = dessiner(&reglages, 4, segments, substrat_final, minimum, maximum, 2);
    let animation = planche(&reglages, segments, &etats, minimum, maximum);

    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;
    colonie.ecrire(&dossier.join("etape-2-colonie.png"))?;
    animation.ecrire(&dossier.join("etape-2-animation.png"))?;
    Ok(())
}
