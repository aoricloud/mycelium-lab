//! Les deux aperçus de l'étape 3, écrits dans `apercus/` :
//!
//! - `etape-3-colonie.png` : la colonie du modèle de la figure 3 (`voisinage::Colonie`, graine 1, 400 pas,
//!   depuis la spore `(0, 0)`), ses segments clairs sur fond sombre, à l'échelle qui la fait tenir dans
//!   l'image ;
//! - `etape-3-figure.png` : la figure refaite, le rayon en fonction du temps pour les graines 1 à 5 sur 400
//!   pas (cinq courbes de couleurs distinctes), la droite de régression de la pente moyenne sur la fenêtre
//!   `]200, 400]` en blanc, et la droite de pente 1 (la vitesse d'un apex) en pointillés ; des axes simples,
//!   sans texte.
//!
//! `cargo run --example apercus_etape_3` les écrit. Le rendu n'emploie que `+`, `-`, `*`, `/`, `sqrt` et des
//! conversions `as` : ni `round` ni `floor`, donc le même résultat sur toutes les machines.

use std::path::Path;

use mycelium_lab::croissance::Segment;
use mycelium_lab::image::ecrire_png;
use mycelium_lab::voisinage::{Colonie, Parametres, ajuster_rayon};

/// Nombre de pas de temps simulés, pour la colonie comme pour la figure.
const PAS_TEMPS: u32 = 400;
/// Début (exclu) de la fenêtre tardive de la régression.
const T_DEBUT: u32 = 200;
/// Fin (incluse) de la fenêtre tardive de la régression.
const T_FIN: u32 = 400;
/// Nombre de graines tracées dans la figure.
const GRAINES: u64 = 5;

/// Fond sombre commun aux aperçus.
const FOND: [u8; 3] = [14, 16, 20];
/// Couleur claire des hyphes, dans l'image de la colonie.
const HYPHE: [u8; 3] = [210, 228, 180];
/// Couleur de la spore, au centre de la colonie.
const SPORE: [u8; 3] = [245, 238, 200];
/// Couleur des axes et des graduations.
const AXE: [u8; 3] = [112, 118, 128];
/// Couleur de la droite de régression.
const REGRESSION: [u8; 3] = [246, 246, 246];
/// Couleur de la droite de pente 1, en pointillés.
const VITESSE: [u8; 3] = [172, 184, 204];

/// Une couleur distincte par graine de la figure.
const COULEURS: [[u8; 3]; GRAINES as usize] =
    [[236, 112, 94], [242, 202, 88], [126, 222, 142], [96, 184, 240], [208, 142, 238]];

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

    /// Trace un segment en pointillés, avec un pinceau de 2 pixels : 9 pixels pleins, puis 9 pixels vides, et
    /// ainsi de suite.
    fn trace_pointilles(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, couleur: [u8; 3]) {
        const TIRET: i64 = 9;
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
        let mut rang = 0i64;
        let (mut x, mut y) = (x0, y0);
        loop {
            if (rang / TIRET) % 2 == 0 {
                self.tache(x, y, 2, couleur);
            }
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
            rang += 1;
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

/// Arrondit un `f64` à l'entier le plus proche sans fonction de la bibliothèque mathématique : le cast `as
/// i64` tronque vers zéro, on ajoute donc `±0.5` avant.
fn arrondi(v: f64) -> i64 {
    if v >= 0.0 { (v + 0.5) as i64 } else { (v - 0.5) as i64 }
}

/// La colonie de la graine 1 après [`PAS_TEMPS`] pas, ses segments clairs sur fond sombre, à l'échelle qui
/// la fait tenir dans une image carrée.
fn toile_colonie(segments: &[Segment]) -> Toile {
    const COTE: usize = 800;
    const MARGE: f64 = 28.0;

    // Boîte englobante de tous les segments, pour que la colonie remplisse la toile.
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for segment in segments {
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
    for segment in segments {
        toile.trace(vers_x(segment.x0), vers_y(segment.y0), vers_x(segment.x1), vers_y(segment.y1), 2, HYPHE);
    }
    let (cx, cy) = (vers_x(0.0), vers_y(0.0));
    toile.disque(cx, cy, 5, SPORE);
    toile
}

/// La figure refaite : le rayon en fonction du temps pour chaque graine, la régression de la pente moyenne
/// sur la fenêtre tardive, et la droite de pente 1.
fn toile_figure(series: &[Vec<f64>]) -> Toile {
    const LARGEUR: usize = 960;
    const HAUTEUR: usize = 640;
    const MARGE_GAUCHE: f64 = 76.0;
    const MARGE_DROITE: f64 = 36.0;
    const MARGE_HAUT: f64 = 36.0;
    const MARGE_BAS: f64 = 56.0;

    // Le plus grand rayon observé, et au moins la portée de la droite de pente 1 au temps [`T_FIN`].
    let mut maximum = 1.0f64;
    for rayons in series {
        for &rayon in rayons {
            if rayon > maximum {
                maximum = rayon;
            }
        }
    }
    if maximum < T_FIN as f64 {
        maximum = T_FIN as f64;
    }
    let rayon_max = maximum * 1.06;

    let largeur_utile = LARGEUR as f64 - MARGE_GAUCHE - MARGE_DROITE;
    let hauteur_utile = HAUTEUR as f64 - MARGE_HAUT - MARGE_BAS;
    let vers_x = |t: f64| arrondi(MARGE_GAUCHE + t / T_FIN as f64 * largeur_utile);
    let vers_y = |r: f64| arrondi(HAUTEUR as f64 - MARGE_BAS - r / rayon_max * hauteur_utile);

    let mut toile = Toile::nouvelle(LARGEUR, HAUTEUR, FOND);

    // Les axes, avec une graduation tous les cent pas (ou cent unités de rayon) : simples, sans texte.
    let axe_x = vers_x(0.0);
    let axe_y = vers_y(0.0);
    toile.trace(axe_x, vers_y(rayon_max), axe_x, axe_y, 1, AXE);
    toile.trace(axe_x, axe_y, vers_x(T_FIN as f64), axe_y, 1, AXE);
    let mut t = 0u32;
    while t <= T_FIN {
        let x = vers_x(t as f64);
        toile.trace(x, axe_y, x, axe_y + 6, 1, AXE);
        t += 100;
    }
    let mut r = 0.0f64;
    while r <= rayon_max {
        let y = vers_y(r);
        toile.trace(axe_x - 6, y, axe_x, y, 1, AXE);
        r += 100.0;
    }

    // La droite de pente 1 : la vitesse d'un apex, en pointillés.
    toile.trace_pointilles(vers_x(0.0), vers_y(0.0), vers_x(T_FIN as f64), vers_y(T_FIN as f64), VITESSE);

    // Une courbe par graine, du temps 0 à [`T_FIN`].
    for (rang, rayons) in series.iter().enumerate() {
        let couleur = COULEURS[rang];
        let (mut precedent_x, mut precedent_y) = (vers_x(0.0), vers_y(0.0));
        for (indice, &rayon) in rayons.iter().enumerate() {
            let x = vers_x((indice + 1) as f64);
            let y = vers_y(rayon);
            toile.trace(precedent_x, precedent_y, x, y, 1, couleur);
            precedent_x = x;
            precedent_y = y;
        }
    }

    // La droite de régression de la pente moyenne sur la fenêtre `]T_DEBUT, T_FIN]`, en blanc.
    let mut pente = 0.0f64;
    let mut ordonnee = 0.0f64;
    for rayons in series {
        let ajustement = ajuster_rayon(rayons, T_DEBUT, T_FIN);
        pente += ajustement.pente;
        ordonnee += ajustement.ordonnee;
    }
    let nombre = series.len() as f64;
    pente /= nombre;
    ordonnee /= nombre;
    let debut = T_DEBUT as f64;
    let fin = T_FIN as f64;
    toile.trace(
        vers_x(debut),
        vers_y(pente * debut + ordonnee),
        vers_x(fin),
        vers_y(pente * fin + ordonnee),
        2,
        REGRESSION,
    );

    toile
}

fn main() -> Result<(), String> {
    const GRAINE_COLONIE: u64 = 1;

    let parametres = Parametres::figure_3();
    let mut series: Vec<Vec<f64>> = Vec::with_capacity(GRAINES as usize);
    let mut segments_colonie: Vec<Segment> = Vec::new();

    for graine in 1..=GRAINES {
        let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
        colonie.avancer_n(PAS_TEMPS);
        if graine == GRAINE_COLONIE {
            segments_colonie = colonie.segments().to_vec();
        }
        series.push(colonie.rayons().to_vec());
    }

    let dossier = Path::new("apercus");
    std::fs::create_dir_all(dossier).map_err(|e| format!("création de {} : {e}", dossier.display()))?;
    toile_colonie(&segments_colonie).ecrire(&dossier.join("etape-3-colonie.png"))?;
    toile_figure(&series).ecrire(&dossier.join("etape-3-figure.png"))?;
    Ok(())
}
