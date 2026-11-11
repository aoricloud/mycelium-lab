//! Outils des tests des étapes 13 et 14 (M3), inclus par `#[path = "etape_13/outils.rs"] mod outils;` : un petit
//! générateur pseudo-aléatoire, des grilles de nœuds au hasard, et l'**oracle** du solveur (une élimination de Gauss à
//! pivot partiel, écrite ici autrement que la décomposition de Cholesky de `systeme.rs`). Parmi les fonctions de `f64`,
//! seulement `+ - * /`, `abs` et `max`.
#![allow(dead_code)]

use mycelium_lab::raideur::{Appui, Noeud};

/// Un générateur pseudo-aléatoire (SplitMix64) propre aux tests.
pub struct Hasard(pub u64);

impl Hasard {
    pub fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Un entier de `0` à `n - 1`.
    pub fn jusqu_a(&mut self, n: usize) -> usize {
        (self.suivant() % n as u64) as usize
    }

    /// Un nombre de `[-1, 1)`.
    pub fn pm(&mut self) -> f64 {
        (self.suivant() >> 11) as f64 / 4503599627370496.0 - 1.0
    }

    /// Une raideur au hasard : de 1 à 2, fois `10^k` avec `k` tiré de 0 à `decades` (le **contraste** des raideurs
    /// d'un réseau va donc jusqu'à `2 × 10^decades`).
    pub fn raideur(&mut self, decades: usize) -> f64 {
        (0..self.jusqu_a(decades + 1)).fold(1.5 + 0.5 * self.pm(), |r, _| r * 10.0)
    }
}

/// Une grille au hasard : `largeur × hauteur` nœuds (le nœud `j × largeur + i` près de `(i, j)`, déplacé d'au plus
/// 0,15) et ses segments `(a, b)` : les côtés horizontaux et verticaux, et une diagonale par case. Avec trois appuis
/// (voir [`appuis`]), le réseau de barres ou de poutres posé sur ces segments est valide.
pub fn grille(h: &mut Hasard, largeur: usize, hauteur: usize) -> (Vec<Noeud>, Vec<(usize, usize)>) {
    let (mut noeuds, mut segments) = (Vec::new(), Vec::new());
    for j in 0..hauteur {
        for i in 0..largeur {
            noeuds.push(Noeud { x: i as f64 + 0.15 * h.pm(), y: j as f64 + 0.15 * h.pm() });
            let n = j * largeur + i;
            if i + 1 < largeur {
                segments.push((n, n + 1));
            }
            if j + 1 < hauteur {
                segments.push((n, n + largeur));
            }
            if i + 1 < largeur && j + 1 < hauteur {
                segments.push((n, n + largeur + 1));
            }
        }
    }
    (noeuds, segments)
}

/// Des appuis à zéro : un par couple (nœud, axe).
pub fn appuis(ou: &[(usize, usize)]) -> Vec<Appui> {
    ou.iter().map(|&(noeud, axe)| Appui { noeud, axe, valeur: 0.0 }).collect()
}

/// Le plus grand écart de `a` à `b`, divisé par `max(1, plus grande valeur absolue de b)`.
pub fn ecart(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    let echelle = b.iter().fold(1.0_f64, |m, v| m.max(v.abs()));
    a.iter().zip(b).fold(0.0_f64, |m, (x, y)| m.max((x - y).abs())) / echelle
}

/// Le produit de la matrice `a` (rangée ligne par ligne, `x.len()` colonnes) par le vecteur `x`.
pub fn fois(a: &[f64], x: &[f64]) -> Vec<f64> {
    a.chunks(x.len()).map(|ligne| ligne.iter().zip(x).map(|(p, q)| p * q).sum()).collect()
}

/// La partie de la matrice `k` (`n × n`) qui reste quand on retire les degrés de liberté `bloques`.
pub fn partie_libre(k: &[f64], n: usize, bloques: &[usize]) -> Vec<f64> {
    let libres: Vec<usize> = (0..n).filter(|i| !bloques.contains(i)).collect();
    libres.iter().flat_map(|&i| libres.iter().map(move |&j| k[i * n + j])).collect()
}

/// Résout `A x = b` (`A` de `b.len()` lignes, rangée ligne par ligne) par élimination de Gauss à pivot partiel.
pub fn gauss(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = (0..n).map(|i| a[i * n..(i + 1) * n].iter().copied().chain([b[i]]).collect()).collect();
    for c in 0..n {
        let p = (c..n).fold(c, |meilleur, r| if m[r][c].abs() > m[meilleur][c].abs() { r } else { meilleur });
        m.swap(c, p);
        for r in c + 1..n {
            let (f, ligne_du_pivot) = (m[r][c] / m[c][c], m[c].clone());
            for (x, p) in m[r].iter_mut().zip(&ligne_du_pivot).skip(c) {
                *x -= f * p;
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let reste: f64 = (i + 1..n).map(|k| m[i][k] * x[k]).sum();
        x[i] = (m[i][n] - reste) / m[i][i];
    }
    x
}
