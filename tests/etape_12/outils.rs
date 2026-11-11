//! Outils des tests de l'étape 12 (M2), inclus par `#[path = "etape_12/outils.rs"] mod outils;` : des relevés écrits à
//! la main, un petit générateur pseudo-aléatoire, et les **oracles** (la fenêtre, la longueur par pointe moyenne et le
//! taux écrits ici autrement que dans `calage.rs`). Parmi les fonctions de `f64`, seulement `+ - * / abs`.
#![allow(dead_code)]

use mycelium_lab::mesures::logarithme;
use mycelium_lab::releves::Releve;

/// Un générateur pseudo-aléatoire (SplitMix64) propre aux tests.
pub struct Hasard(pub u64);

impl Hasard {
    /// Un entier de `0` à `n - 1`.
    pub fn jusqu_a(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}

/// Des relevés aux pas 0, 1, 2, … à partir de couples `(biomasse, pointes)`.
pub fn releves(couples: &[(f64, usize)]) -> Vec<Releve> {
    let releve = |(t, &(biomasse, pointes)): (usize, &(f64, usize))| Releve {
        temps: t as u32,
        biomasse,
        pointes,
        ramifications: 0,
        anastomoses: 0,
        rayon: 0.0,
        substrat: 0.0,
    };
    couples.iter().enumerate().map(releve).collect()
}

/// Des relevés dont seuls les nombres de pointes sont donnés (biomasse = 10 × pointes).
pub fn pointes(nombres: &[usize]) -> Vec<Releve> {
    releves(&nombres.iter().map(|&p| (10.0 * p as f64, p)).collect::<Vec<_>>())
}

/// La fenêtre par une machine à états : fermée, elle s'ouvre au premier relevé de 40 pointes ou plus ; ouverte, elle
/// se referme au premier relevé de plus de 200. Pas refermée à la fin : pas de fenêtre.
pub fn fenetre_oracle(releves: &[Releve]) -> Option<(usize, usize)> {
    let mut ouverte = None;
    for (i, releve) in releves.iter().enumerate() {
        match ouverte {
            None if releve.pointes > 200 => return None,
            None if releve.pointes >= 40 => ouverte = Some(i),
            Some(premier) if releve.pointes > 200 => return if i > premier { Some((premier, i - 1)) } else { None },
            _ => {}
        }
    }
    None
}

/// La moyenne des `biomasse / pointes` des relevés `premier..=dernier`.
pub fn longueur_oracle(releves: &[Releve], premier: usize, dernier: usize) -> f64 {
    let rapports: Vec<f64> = releves[premier..=dernier].iter().map(|r| r.biomasse / r.pointes as f64).collect();
    rapports.iter().sum::<f64>() / rapports.len() as f64
}

/// La pente de `ln(valeur)` sur `t` par la formule `(n Σxy − Σx Σy) / (n Σxx − (Σx)²)`.
pub fn taux_oracle(valeurs: &[f64], debut: usize, fin: usize) -> f64 {
    let (mut n, mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (t, &v) in valeurs.iter().enumerate().take(fin + 1).skip(debut) {
        let (x, y) = (t as f64, logarithme(v));
        (n, sx, sy, sxx, sxy) = (n + 1.0, sx + x, sy + y, sxx + x * x, sxy + x * y);
    }
    (n * sxy - sx * sy) / (n * sxx - sx * sx)
}
