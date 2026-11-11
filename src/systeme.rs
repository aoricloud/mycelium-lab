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
//! inverse. L'arrondi laisse celle d'un mécanisme sous 1e-15, quelles que soient les raideurs ; celle d'un réseau valide
//! dont les raideurs diffèrent d'un rapport `r` est de l'ordre de `1/r`. Au plus [`SEUIL_SINGULIER`], le système est
//! refusé. Un réseau valide refusé ainsi (des raideurs dans un rapport de plus de 1e10 environ) est si mal conditionné
//! que `f64` ne le distingue plus d'un mécanisme : son résultat n'aurait pas quatre chiffres justes.
//!
//! Le plus petit **pivot** de la factorisation ne convient pas à cette mesure : l'arrondi le laisse monter à 1e-7 pour
//! un mécanisme dès que les raideurs diffèrent (les mesures sont dans `MODELE.md`).
//!
//! # Le déterminisme
//!
//! Des boucles dans un ordre fixe, parmi les fonctions de `f64` seulement `+ - * /` et `sqrt` : le même résultat, bit à
//! bit, sur toutes les machines. Ni `HashMap`, ni fil d'exécution.

/// Un plus petit mode (voir [`plus_petit_mode`]) au plus égal à ce nombre est pris pour un mécanisme. **Mesuré** (voir
/// `MODELE.md`) : le plus petit mode d'un mécanisme ne dépasse pas 1e-15 (réseaux au hasard, raideurs dans un rapport de
/// 1 à 1e15) ; l'erreur relative d'un résultat est d'au plus `2,5e-16 / mode` sur les cas mesurés : trois à quatre chiffres au seuil.
pub const SEUIL_SINGULIER: f64 = 0.0; // bouchon : la vague du code écrit la vraie valeur

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

/// Le plus petit mode de `a` (`n × n`) : la plus petite valeur propre de la matrice mise à l'échelle par la diagonale,
/// estimée par itération inverse : un majorant de la vraie, qui la serre de près dès qu'elle est petite devant les
/// autres (pour une matrice bien conditionnée, il dit seulement « de l'ordre de un »). Zéro si la matrice n'est pas
/// définie positive, `+∞` pour `n = 0`. C'est la mesure que compare [`SEUIL_SINGULIER`].
///
/// # Panics
///
/// Panique si `a` n'a pas `n × n` nombres.
pub fn plus_petit_mode(a: &[f64], n: usize) -> f64 {
    let _ = (a, n);
    -1.0 // bouchon
}

/// Résout `A x = b` pour une matrice symétrique définie positive `A` de `b.len()` lignes.
///
/// # Errors
///
/// [`Mecanisme`] si le plus petit mode de `A` est au plus [`SEUIL_SINGULIER`] : une matrice singulière, ou qui n'est
/// pas définie positive, ou qui porte un nombre non fini.
///
/// # Panics
///
/// Panique si `a` n'a pas `b.len()²` nombres.
pub fn resoudre(a: &[f64], b: &[f64]) -> Result<Vec<f64>, Mecanisme> {
    let _ = a;
    Ok(vec![0.0; b.len()]) // bouchon
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
    let _ = (k, imposes);
    Ok(Resolution { deplacements: vec![0.0; f.len()], reactions: vec![0.0; f.len()], energie: -1.0 }) // bouchon
}
