//! Système : le solveur linéaire du calcul de raideur (étape 13, M3).
//!
//! Une matrice **pleine**, **symétrique**, rangée ligne par ligne (`a[i * n + j]`), et la décomposition de **Cholesky**
//! `A = L Lᵀ`, en `f64`. Elle ne sert qu'à quelques milliers d'inconnues au plus (n³/3 opérations, n² nombres).
//!
//! # La mise à l'échelle par la diagonale
//!
//! Avant de factoriser, on divise la ligne et la colonne `i` par `√A[i][i]` : la matrice `Â` a une diagonale de un.
//! Sans cela, des degrés de liberté qui n'ont pas la même unité (une translation et une rotation) ne se compareraient pas.
//!
//! # Le mécanisme
//!
//! Un **mécanisme** est un mouvement qui ne coûte aucune énergie : la matrice est singulière, sa plus petite valeur
//! propre est nulle. **Un système singulier ne rend jamais un nombre** : il rend [`Mecanisme`]. La mesure est le
//! **plus petit mode** ([`plus_petit_mode`]) : la plus petite valeur propre de `Â`, estimée par deux pas d'itération
//! inverse. L'arrondi laisse celle d'un mécanisme sous quelques 1e-15 (le plus souvent vers 5e-16), quelles que soient
//! les raideurs ; celle d'un réseau valide dont les raideurs diffèrent d'un rapport `r` est de l'ordre de `1/r` pour
//! une chaîne, cent à mille fois moins pour les grilles mesurées. Au plus [`SEUIL_SINGULIER`], le système est refusé.
//! Un réseau valide refusé ainsi (des raideurs dans un rapport de plus de 1e10 environ) est si mal conditionné que
//! `f64` ne le distingue plus d'un mécanisme : son résultat pourrait ne pas avoir quatre chiffres justes.
//!
//! Le plus petit **pivot** de la factorisation ne convient pas à cette mesure : l'arrondi le laisse monter à 5e-7 pour
//! un mécanisme dès que les raideurs diffèrent (les mesures sont dans `MODELE.md`).
//!
//! # Le déterminisme
//!
//! Des boucles dans un ordre fixe, parmi les fonctions de `f64` seulement `+ - * /` et `sqrt` : le même résultat, bit à
//! bit, sur toutes les machines. Ni `HashMap`, ni fil d'exécution.

/// Un plus petit mode (voir [`plus_petit_mode`]) au plus égal à ce nombre est pris pour un mécanisme. **Mesuré** (voir
/// `MODELE.md`) : le plus petit mode d'un mécanisme ne dépasse pas quelques 1e-15 (réseaux au hasard, raideurs dans un
/// rapport de 1 à 1e15 ; le plus grand mesuré : 5,3e-15) ; l'erreur relative d'un résultat est d'au plus
/// `2,5e-16 / mode` sur les cas mesurés : trois à quatre chiffres au seuil.
pub const SEUIL_SINGULIER: f64 = 1e-12;

/// Le système est singulier : la structure a un mécanisme (un mouvement sans énergie) ou un degré de liberté sans
/// raideur, ou elle est si mal conditionnée que le calcul ne la distingue pas d'un mécanisme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mecanisme;

/// Ce que rend [`resoudre_avec_imposes`] pour `n` degrés de liberté.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    /// Les `n` déplacements, les degrés de liberté imposés compris.
    pub deplacements: Vec<f64>,
    /// Les `n` réactions : ce que l'appui exerce sur la structure à chaque degré de liberté imposé, zéro (exactement)
    /// ailleurs. Avec les forces données, elles font un système en équilibre.
    pub reactions: Vec<f64>,
    /// L'énergie de déformation `½ uᵀ K u`.
    pub energie: f64,
}

/// La factorisation `Â = L Lᵀ` de la matrice mise à l'échelle.
struct Facteur {
    n: usize,
    /// `1 / √A[i][i]`.
    echelle: Vec<f64>,
    /// `L`, triangle inférieur rangé dans un carré (le haut est zéro).
    l: Vec<f64>,
}

/// Factorise `a` (`n × n`), ou rend `None` si elle n'est pas définie positive à l'arrondi près (une diagonale ou un
/// pivot qui n'est pas un nombre strictement positif).
fn factoriser(a: &[f64], n: usize) -> Option<Facteur> {
    assert_eq!(a.len(), n * n, "une matrice de {n} × {n} a {} nombres, pas {}", n * n, a.len());
    let mut echelle = vec![0.0; n];
    for i in 0..n {
        let diagonale = a[i * n + i];
        if !(diagonale.is_finite() && diagonale > 0.0) {
            return None; // pas de raideur (ou une raideur infinie ou NaN) sur ce degré de liberté
        }
        echelle[i] = 1.0 / diagonale.sqrt();
    }
    let mut l = vec![0.0; n * n];
    for j in 0..n {
        for i in 0..j {
            let mut somme = a[j * n + i] * echelle[j] * echelle[i];
            for k in 0..i {
                somme -= l[j * n + k] * l[i * n + k];
            }
            l[j * n + i] = somme / l[i * n + i];
        }
        let mut pivot = 1.0; // la diagonale de la matrice mise à l'échelle
        for k in 0..j {
            pivot -= l[j * n + k] * l[j * n + k];
        }
        if pivot > 0.0 {
            l[j * n + j] = pivot.sqrt();
        } else {
            return None; // nul, négatif ou NaN
        }
    }
    Some(Facteur { n, echelle, l })
}

impl Facteur {
    /// Remplace `x` par la solution `y` de `Â y = x` : une descente (`L z = x`) puis une remontée (`Lᵀ y = z`).
    fn resoudre_a_l_echelle(&self, x: &mut [f64]) {
        let (n, l) = (self.n, &self.l);
        for i in 0..n {
            for k in 0..i {
                x[i] -= l[i * n + k] * x[k];
            }
            x[i] /= l[i * n + i];
        }
        for i in (0..n).rev() {
            for k in i + 1..n {
                x[i] -= l[k * n + i] * x[k];
            }
            x[i] /= l[i * n + i];
        }
    }

    /// La plus petite valeur propre de `Â`, estimée par deux pas d'itération inverse. Le vecteur de départ pèse chaque
    /// degré de liberté par l'inverse de son pivot (`1 / L[i][i]²`) : un mouvement sans raideur y a toujours sa part.
    /// Chaque pas résout `Â z = d` (`d` : le vecteur précédent) et estime `(z·d) / (z·z)`.
    fn plus_petit_mode(&self) -> f64 {
        let n = self.n;
        if n == 0 {
            return f64::INFINITY; // sans inconnue, rien n'est singulier
        }
        let mut z: Vec<f64> = (0..n).map(|i| 1.0 / (self.l[i * n + i] * self.l[i * n + i])).collect();
        let mut mode = 0.0;
        for _ in 0..2 {
            let depart = z.clone();
            self.resoudre_a_l_echelle(&mut z);
            mode = produit(&z, &depart) / produit(&z, &z);
        }
        mode
    }
}

/// Le produit scalaire de deux vecteurs de même longueur.
fn produit(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |somme, (x, y)| somme + x * y)
}

/// Le plus petit mode de `a` (`n × n`) : la plus petite valeur propre de la matrice mise à l'échelle par la diagonale,
/// estimée par itération inverse : un majorant de la vraie, qui la serre de près dès qu'elle est petite devant les
/// autres (pour une matrice bien conditionnée, il dit seulement « de l'ordre de un »). Zéro si la matrice n'est pas
/// définie positive, `+∞` pour `n = 0`. C'est la mesure que compare [`SEUIL_SINGULIER`].
///
/// # Panics
///
/// Panique si `a` n'a pas `n × n` nombres.
pub fn plus_petit_mode(a: &[f64], n: usize) -> f64 {
    factoriser(a, n).map_or(0.0, |facteur| facteur.plus_petit_mode())
}

/// Résout `A x = b` pour une matrice symétrique définie positive `A` de `b.len()` lignes.
///
/// # Errors
///
/// [`Mecanisme`] si le plus petit mode de `A` est au plus [`SEUIL_SINGULIER`], ou n'est pas un nombre : une matrice
/// singulière, ou qui n'est pas définie positive. Seuls la diagonale et le triangle inférieur de `A` sont lus ; un
/// second membre `b` non fini rend des `NaN` (l'appelant contrôle ses données : `raideur` le fait).
///
/// # Panics
///
/// Panique si `a` n'a pas `b.len()²` nombres.
pub fn resoudre(a: &[f64], b: &[f64]) -> Result<Vec<f64>, Mecanisme> {
    let Some(facteur) = factoriser(a, b.len()) else { return Err(Mecanisme) };
    // Un pivot minuscule fait déborder l'itération inverse : le mode n'est alors pas un nombre, et la matrice est refusée.
    let mode = facteur.plus_petit_mode();
    if mode.is_nan() || mode <= SEUIL_SINGULIER {
        return Err(Mecanisme);
    }
    // `A = S⁻¹ Â S⁻¹` (`S` : l'échelle), donc `x = S Â⁻¹ S b`.
    let mut x: Vec<f64> = b.iter().zip(&facteur.echelle).map(|(bi, si)| bi * si).collect();
    facteur.resoudre_a_l_echelle(&mut x);
    for (xi, si) in x.iter_mut().zip(&facteur.echelle) {
        *xi *= si;
    }
    Ok(x)
}

/// Résout `K u = f` quand certains degrés de liberté sont imposés : `imposes` donne des couples (degré de liberté,
/// valeur), sans répétition et dans `0..f.len()`. Les inconnues sont les autres degrés de liberté (les « libres ») :
/// `K_ll u_l = f_l − K_li u_i`. Les réactions aux degrés de liberté imposés sont ensuite `K u − f` à ces places.
///
/// # Errors
///
/// [`Mecanisme`] si la partie libre de `K` est singulière (voir [`resoudre`]).
///
/// # Panics
///
/// Panique si `k` n'a pas `f.len()²` nombres, ou si un degré de liberté imposé sort de `0..f.len()` ou se répète.
pub fn resoudre_avec_imposes(k: &[f64], f: &[f64], imposes: &[(usize, f64)]) -> Result<Resolution, Mecanisme> {
    let n = f.len();
    assert_eq!(k.len(), n * n, "une matrice de {n} × {n} a {} nombres, pas {}", n * n, k.len());
    let mut est_impose = vec![false; n];
    let mut deplacements = vec![0.0; n];
    for &(ddl, valeur) in imposes {
        assert!(ddl < n && !est_impose[ddl], "degré de liberté {ddl} hors de 0..{n} ou imposé deux fois");
        (est_impose[ddl], deplacements[ddl]) = (true, valeur);
    }
    let libres: Vec<usize> = (0..n).filter(|&i| !est_impose[i]).collect();
    let m = libres.len();
    let (mut k_libre, mut f_libre) = (vec![0.0; m * m], vec![0.0; m]);
    for (p, &i) in libres.iter().enumerate() {
        f_libre[p] = f[i];
        for &(j, valeur) in imposes {
            f_libre[p] -= k[i * n + j] * valeur;
        }
        for (q, &j) in libres.iter().enumerate() {
            k_libre[p * m + q] = k[i * n + j];
        }
    }
    for (&i, u) in libres.iter().zip(resoudre(&k_libre, &f_libre)?) {
        deplacements[i] = u;
    }
    let (mut reactions, mut energie) = (vec![0.0; n], 0.0);
    for i in 0..n {
        let ku = produit(&k[i * n..(i + 1) * n], &deplacements);
        if est_impose[i] {
            reactions[i] = ku - f[i];
        }
        energie += 0.5 * deplacements[i] * ku;
    }
    Ok(Resolution { deplacements, reactions, energie })
}
