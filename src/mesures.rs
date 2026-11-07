//! Mesures : biomasse et dimension fractale par comptage de boîtes.
//!
//! Le module ne dépend d'aucune fonction de `f64` qui passe par la bibliothèque mathématique du système
//! (`ln`, `exp`, `powf`, `sin`, etc.) : le logarithme est écrit ici, [`logarithme`], pour que les résultats
//! soient identiques sur toutes les machines.

use std::f64::consts::LN_2;

/// Bits de l'exposant d'un `f64` (11 bits).
const MASQUE_EXPOSANT: u64 = 0x7ff0_0000_0000_0000;
/// Bits de la mantisse d'un `f64` (52 bits).
const MASQUE_MANTISSE: u64 = 0x000f_ffff_ffff_ffff;
/// Biais de l'exposant d'un `f64`.
const BIAIS_EXPOSANT: i32 = 1023;

/// Image binaire : une grille rectangulaire de cases pleines (`true`) ou vides (`false`).
///
/// Les coordonnées sont `(x, y)` avec `x` dans `0..largeur` (colonne) et `y` dans `0..hauteur` (ligne),
/// l'origine `(0, 0)` étant le coin haut-gauche. Une image neuve est entièrement vide. Une image peut avoir
/// une largeur ou une hauteur nulle (elle n'a alors aucune case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBinaire {
    largeur: usize,
    hauteur: usize,
    cases: Vec<bool>,
}

impl ImageBinaire {
    /// Crée une image vide de `largeur` × `hauteur` cases.
    pub fn new(largeur: usize, hauteur: usize) -> Self {
        Self { largeur, hauteur, cases: vec![false; largeur * hauteur] }
    }

    /// Largeur de l'image, en cases.
    pub fn largeur(&self) -> usize {
        self.largeur
    }

    /// Hauteur de l'image, en cases.
    pub fn hauteur(&self) -> usize {
        self.hauteur
    }

    /// Lit la case `(x, y)` : `true` si elle est pleine.
    ///
    /// # Panics
    ///
    /// Panique si `x >= largeur()` ou `y >= hauteur()`.
    pub fn lire(&self, x: usize, y: usize) -> bool {
        assert!(x < self.largeur, "x = {x} hors de 0..{}", self.largeur);
        assert!(y < self.hauteur, "y = {y} hors de 0..{}", self.hauteur);
        self.cases[y * self.largeur + x]
    }

    /// Écrit la case `(x, y)` : pleine si `pleine` vaut `true`, vide sinon.
    ///
    /// # Panics
    ///
    /// Panique si `x >= largeur()` ou `y >= hauteur()`.
    pub fn ecrire(&mut self, x: usize, y: usize, pleine: bool) {
        assert!(x < self.largeur, "x = {x} hors de 0..{}", self.largeur);
        assert!(y < self.hauteur, "y = {y} hors de 0..{}", self.hauteur);
        self.cases[y * self.largeur + x] = pleine;
    }
}

/// Biomasse de l'image : le nombre exact de cases pleines (0 pour une image vide).
pub fn biomasse(image: &ImageBinaire) -> usize {
    image.cases.iter().filter(|&&pleine| pleine).count()
}

/// Logarithme népérin de `x`, calculé sans `f64::ln` ni aucune fonction de la bibliothèque mathématique
/// du système : le résultat est identique sur toutes les machines.
///
/// Contrat : pour `x` fini et strictement positif, l'erreur absolue sur le résultat est inférieure à
/// `1e-9` (sur le domaine utile, de 1 à 1e7 au moins). Pour `x <= 0` ou `x` NaN, renvoie `f64::NAN`.
/// Le calcul est déterministe : même entrée, même résultat bit à bit.
pub fn logarithme(x: f64) -> f64 {
    if x.is_nan() || x <= 0.0 {
        return f64::NAN;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }

    // Décompose x = m * 2^e avec m dans [1, 2). Les sous-normaux sont d'abord ramenés parmi les
    // normaux en les multipliant par des puissances de 2 (chaque multiplication est exacte).
    let mut y = x;
    let mut ajustement = 0i32;
    while y.to_bits() & MASQUE_EXPOSANT == 0 {
        y *= 2.0;
        ajustement -= 1;
    }

    let bits = y.to_bits();
    let exposant = ((bits & MASQUE_EXPOSANT) >> 52) as i32 - BIAIS_EXPOSANT + ajustement;
    let m = f64::from_bits((bits & MASQUE_MANTISSE) | 0x3ff0_0000_0000_0000);

    // ln(m) = 2 * atanh((m - 1) / (m + 1)) = 2 * (z + z^3/3 + z^5/5 + ...), avec z dans [0, 1/3].
    // La série converge d'un facteur 1/9 par terme : trente termes dépassent largement 1e-16.
    let z = (m - 1.0) / (m + 1.0);
    let z2 = z * z;
    let mut terme = z;
    let mut somme = z;
    for i in 1..=30 {
        terme *= z2;
        somme += terme / (2 * i + 1) as f64;
    }

    2.0 * somme + exposant as f64 * LN_2
}

/// Compte les boîtes de `cote` × `cote` cases qui contiennent au moins une case pleine.
fn compter_boites(image: &ImageBinaire, cote: usize) -> usize {
    let colonnes = image.largeur().div_ceil(cote);
    let lignes = image.hauteur().div_ceil(cote);
    let mut compte = 0;
    for boite_y in 0..lignes {
        for boite_x in 0..colonnes {
            let x0 = boite_x * cote;
            let y0 = boite_y * cote;
            let x1 = x0.saturating_add(cote).min(image.largeur());
            let y1 = y0.saturating_add(cote).min(image.hauteur());
            let mut pleine = false;
            'cases: for y in y0..y1 {
                for x in x0..x1 {
                    if image.lire(x, y) {
                        pleine = true;
                        break 'cases;
                    }
                }
            }
            if pleine {
                compte += 1;
            }
        }
    }
    compte
}

/// Dimension fractale de l'image par comptage de boîtes.
///
/// Pour chaque côté `c` de `cotes`, l'image est découpée en boîtes carrées de `c` × `c` cases à partir de
/// l'origine `(0, 0)` (les boîtes du bord droit et du bas peuvent être tronquées) ; `N(c)` est le nombre
/// de boîtes qui contiennent au moins une case pleine. La dimension est l'opposé de la pente de la
/// régression en moindres carrés ordinaires de `logarithme(N(c))` sur `logarithme(c)`, les logarithmes
/// étant ceux de [`logarithme`]. Les côtés sont pris dans l'ordre donné et leur ordre n'influe pas sur le
/// résultat à l'arrondi près.
///
/// # Panics
///
/// Panique si `cotes` contient moins de deux valeurs distinctes, si un côté vaut 0, ou si l'image est
/// vide (aucune case pleine : `N(c)` serait nul et son logarithme indéfini).
pub fn dimension_fractale(image: &ImageBinaire, cotes: &[usize]) -> f64 {
    assert!(biomasse(image) > 0, "dimension fractale d'une image vide");

    let mut distincts = cotes.to_vec();
    distincts.sort_unstable();
    distincts.dedup();
    assert!(distincts.len() >= 2, "il faut au moins deux côtés distincts");

    for &cote in cotes {
        assert!(cote > 0, "un côté de boîte ne peut pas valoir 0");
    }

    let mut n = 0.0;
    let mut somme_x = 0.0;
    let mut somme_y = 0.0;
    let mut somme_xx = 0.0;
    let mut somme_xy = 0.0;
    for &cote in cotes {
        let lx = logarithme(cote as f64);
        let ly = logarithme(compter_boites(image, cote) as f64);
        n += 1.0;
        somme_x += lx;
        somme_y += ly;
        somme_xx += lx * lx;
        somme_xy += lx * ly;
    }

    let denominateur = n * somme_xx - somme_x * somme_x;
    let pente = (n * somme_xy - somme_x * somme_y) / denominateur;
    -pente
}
