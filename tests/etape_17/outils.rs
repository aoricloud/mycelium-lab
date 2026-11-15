//! Outils des tests de l'étape 17 (M4), inclus par `#[path = "etape_17/outils.rs"] mod outils;` après
//! `mod exemple;` : des colonies faites à la main (hyphes parallèles, Z, nid d'abeille) et leur lecture par `lire`, celle
//! de `examples/nuage_raideur.rs`. Parmi les fonctions de `f64`, seulement `+ - * /`, `sqrt` et `abs`.
#![allow(dead_code)]

use crate::exemple::lire;
use mycelium_lab::croissance::Segment;
use mycelium_lab::nuage::{Cavite, Point};
use mycelium_lab::reseau::Hyphe;

/// `EA = 5` et `EI = 3`, comme les cas d'école des étapes 14 et 15.
pub const HYPHE: Hyphe = Hyphe { ea: 5.0, ei: 3.0 };

/// Une cavité sans obstacle.
pub fn cavite(largeur: usize, hauteur: usize) -> Cavite {
    Cavite { largeur, hauteur, libres: vec![true; largeur * hauteur] }
}

/// Un segment de `(x0, y0)` à `(x1, y1)`.
pub fn segment(x0: f64, y0: f64, x1: f64, y1: f64) -> Segment {
    Segment { apex: 0, temps: 1, x0, y0, x1, y1 }
}

/// La somme des longueurs de ces segments.
pub fn longueur(segments: &[Segment]) -> f64 {
    segments.iter().map(|s| ((s.x1 - s.x0) * (s.x1 - s.x0) + (s.y1 - s.y0) * (s.y1 - s.y0)).sqrt()).sum()
}

/// `n` hyphes droites parallèles d'un mors à l'autre, de `x = 0,5` à `x = largeur − 0,5`, à `y = 0,5`, `1,5`, …
pub fn paralleles(n: usize, largeur: usize) -> Vec<Segment> {
    (0..n).map(|i| segment(0.5, i as f64 + 0.5, largeur as f64 - 0.5, i as f64 + 0.5)).collect()
}

/// Le miroir gauche-droite de ces segments dans une boîte de `largeur` cases : `x` devient `largeur − x`.
pub fn miroir(segments: &[Segment], largeur: usize) -> Vec<Segment> {
    let l = largeur as f64;
    segments.iter().map(|s| segment(l - s.x0, s.y0, l - s.x1, s.y1)).collect()
}

/// Le point d'une colonie faite à la main, sans anastomose : elle doit traverser.
pub fn point(cavite: &Cavite, segments: &[Segment], hyphe: Hyphe) -> Point {
    lire(cavite, segments, &[], (0, 0), hyphe).expect("la colonie traverse et le solveur l'accepte")
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
pub fn egal(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() < tolerance
}

/// Un nid d'abeille régulier, des hexagones de côté `a`, sur `largeur − 1` cases d'un mors à l'autre : `rangees` lignes
/// en zigzag de `demi_periodes` segments chacune (de `x = 0,5` à `x = largeur − 0,5`), reliées par des tiges verticales.
/// Rend les segments (tous de longueur `a = 2 (largeur − 1) / (√3 demi_periodes)`) et la hauteur du panneau,
/// `(3 rangees − 2) a / 2`. Chaque sommet est calculé par la même formule : deux segments qui se rejoignent ont des
/// extrémités égales bit à bit, donc un nœud commun.
pub fn nid_d_abeilles(demi_periodes: usize, largeur: usize, rangees: usize) -> (Vec<Segment>, f64) {
    let dx = (largeur as f64 - 1.0) / demi_periodes as f64;
    let a = 2.0 * dx / 3.0_f64.sqrt();
    // Le sommet `j` de la rangée `r` : bas si `j + r` est pair, haut (une demi-longueur plus loin en `y`) sinon.
    let sommet = |r: usize, j: usize| {
        let bas = 0.5 + 1.5 * a * r as f64;
        (0.5 + dx * j as f64, if (j + r).is_multiple_of(2) { bas } else { bas + 0.5 * a })
    };
    let mut segments = Vec::new();
    for r in 0..rangees {
        for j in 0..demi_periodes {
            let (p, q) = (sommet(r, j), sommet(r, j + 1));
            segments.push(segment(p.0, p.1, q.0, q.1));
        }
        // Une tige de chaque sommet haut de la rangée au sommet bas de la rangée suivante, à la verticale.
        for j in (0..=demi_periodes).filter(|j| r + 1 < rangees && !(j + r).is_multiple_of(2)) {
            let (p, q) = (sommet(r, j), sommet(r + 1, j));
            segments.push(segment(p.0, p.1, q.0, q.1));
        }
    }
    (segments, (3 * rangees - 2) as f64 * a / 2.0)
}
