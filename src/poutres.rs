//! Poutres : le calcul de raideur d'un réseau de poutres à nœuds rigides (étape 14, M3).
//!
//! Comme [`crate::raideur`], avec des [`Poutre`]s au lieu de barres : une poutre travaille en étirement (`EA`) **et en
//! flexion** (`EI`), et ses extrémités sont rigidement attachées aux nœuds (un nœud a une rotation, que les poutres
//! voisines partagent). C'est le modèle d'**Euler-Bernoulli** : la section reste plane et perpendiculaire à la fibre
//! moyenne, le cisaillement de la poutre n'ajoute aucune déformation (les corrections de cisaillement ne sont pas faites).
//!
//! # Les conventions
//!
//! - Trois degrés de liberté par nœud, `[u₀, v₀, θ₀, u₁, v₁, θ₁, …]` : le degré de liberté de l'axe `axe` (0 pour `u`, 1 pour
//!   `v`, **2 pour la rotation**) du nœud `i` est `3 i + axe`. Les [`Appui`]s et les [`Charge`]s de [`crate::raideur`] s'y
//!   appliquent (une charge d'axe 2 est un **moment**) ; le même vocabulaire, les mêmes refus.
//! - `θ` est la rotation de la fibre, `dv/dx` dans le repère de la poutre, **positive dans le sens anti-horaire** (le sens
//!   trigonométrique : de `x` vers `y`). Un moment positif tourne dans ce sens.
//! - `EA` est en force, `EI` en force × longueur², les longueurs en longueur : les unités de l'appelant, cohérentes.
//! - Les efforts d'une poutre sont ses six forces d'extrémité dans **son repère local** (`x'` de `a` vers `b`, `y'` à gauche),
//!   `[N_a, V_a, M_a, N_b, V_b, M_b]`, ce que les nœuds exercent sur la poutre : `N_b` est l'effort normal (**positif en
//!   traction**), `V` l'effort tranchant (force selon `y'`), `M` le moment (anti-horaire positif). Une console encastrée en `a`
//!   et chargée de `P` selon `y'` au bout a `[0, −P, −P L, 0, P, 0]`.
//!
//! # La matrice de la poutre
//!
//! Dans son repère, avec `a = EA/L` et `b = EI/L³`, pour `[u₁, v₁, θ₁, u₂, v₂, θ₂]` :
//!
//! ```text
//! [ a    0     0     −a    0     0    ]
//! [ 0    12b   6bL   0     −12b  6bL  ]
//! [ 0    6bL   4bL²  0     −6bL  2bL² ]
//! [ −a   0     0     a     0     0    ]
//! [ 0    −12b  −6bL  0     12b   −6bL ]
//! [ 0    6bL   2bL²  0     −6bL  4bL² ]
//! ```
//!
//! Dans le repère du réseau : `K = Tᵀ k T` avec `T = diag(R, R)` et `R = [[c, s, 0], [−s, c, 0], [0, 0, 1]]`, où `c = dx/L` et
//! `s = dy/L` : aucune fonction trigonométrique. Pour une rigidité `EI` constante et des charges aux nœuds, la solution des
//! déplacements et des rotations **aux nœuds est exacte** (la flèche exacte est un polynôme du troisième degré, que les
//! fonctions d'Hermite de la poutre représentent) : un seul élément suffit pour une console, mais il faut **un nœud à chaque
//! charge ponctuelle** (et à chaque changement de section). Une charge répartie se donne par ses charges nodales cohérentes
//! (`qL/2` et `±qL²/12`).
//!
//! # Le déterminisme
//!
//! Comme `raideur` : `+ - * /` et `sqrt` seulement, des boucles dans un ordre fixe, aucune `HashMap`.

use crate::raideur::{Appui, Charge, ErreurRaideur, Noeud, longueur_et_direction, ranger_appuis_et_charges};
use crate::systeme;

/// Une poutre entre les nœuds `a` et `b`, de raideur axiale `ea` (force) et de raideur en flexion `ei` (force × longueur²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Poutre {
    pub a: usize,
    pub b: usize,
    pub ea: f64,
    pub ei: f64,
}

/// La solution d'un réseau de poutres.
#[derive(Clone, Debug, PartialEq)]
pub struct SolutionPoutres {
    /// Les déplacements `[u₀, v₀, θ₀, u₁, v₁, θ₁, …]`.
    pub deplacements: Vec<f64>,
    /// Les réactions, rangées comme les déplacements : la force (ou le moment) de l'appui à chaque degré de liberté imposé,
    /// zéro ailleurs. Avec les charges, elles sont en équilibre (les forces, et les moments).
    pub reactions: Vec<f64>,
    /// Les six efforts d'extrémité de chaque poutre, dans l'ordre des poutres (voir plus haut).
    pub efforts: Vec<[f64; 6]>,
    /// L'énergie de déformation `½ uᵀ K u`.
    pub energie: f64,
}

type Matrice6 = [[f64; 6]; 6];

/// La matrice 6 × 6 d'une poutre de longueur `longueur`, dans son repère (voir plus haut). Exactement symétrique.
pub fn matrice_locale(ea: f64, ei: f64, longueur: f64) -> Matrice6 {
    let (l, a, b) = (longueur, ea / longueur, ei / (longueur * longueur * longueur));
    let (b12, b6l, b4l2, b2l2) = (12.0 * b, 6.0 * b * l, 4.0 * b * l * l, 2.0 * b * l * l);
    [
        [a, 0.0, 0.0, -a, 0.0, 0.0],
        [0.0, b12, b6l, 0.0, -b12, b6l],
        [0.0, b6l, b4l2, 0.0, -b6l, b2l2],
        [-a, 0.0, 0.0, a, 0.0, 0.0],
        [0.0, -b12, -b6l, 0.0, b12, -b6l],
        [0.0, b6l, b2l2, 0.0, -b6l, b4l2],
    ]
}

/// Le produit de deux matrices 6 × 6.
fn fois(a: &Matrice6, b: &Matrice6) -> Matrice6 {
    let mut produit = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            for l in 0..6 {
                produit[i][j] += a[i][l] * b[l][j];
            }
        }
    }
    produit
}

/// La poutre prête à servir : la rotation `T` du repère du réseau vers le sien, le produit `k T` (les efforts d'extrémité
/// de la poutre sont `k T u`), et ses six degrés de liberté.
fn poutre_calculee(noeuds: &[Noeud], p: &Poutre) -> Result<(Matrice6, Matrice6, [usize; 6]), ErreurRaideur> {
    if !(p.ea.is_finite() && p.ea > 0.0 && p.ei.is_finite() && p.ei > 0.0) {
        return Err(ErreurRaideur::DonneeInvalide("poutre : ea et ei doivent être finis et strictement positifs"));
    }
    let (longueur, c, s) = longueur_et_direction(noeuds, p.a, p.b)?;
    let r = [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]];
    let mut t = [[0.0; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            (t[i][j], t[i + 3][j + 3]) = (r[i][j], r[i][j]);
        }
    }
    let ddl = [3 * p.a, 3 * p.a + 1, 3 * p.a + 2, 3 * p.b, 3 * p.b + 1, 3 * p.b + 2];
    Ok((t, fois(&matrice_locale(p.ea, p.ei, longueur), &t), ddl))
}

/// La matrice de raideur du réseau avant tout appui : `3n × 3n` (`n` nœuds), pleine, exactement symétrique, rangée ligne par
/// ligne. `K = Σ Tᵀ k T` sur les poutres, dans l'ordre de la liste.
///
/// # Errors
///
/// [`ErreurRaideur::DonneeInvalide`] : une coordonnée non finie ; une poutre dont un indice sort de la liste, de longueur nulle,
/// ou dont `ea` ou `ei` n'est pas fini et strictement positif.
pub fn matrice_de_raideur(noeuds: &[Noeud], poutres: &[Poutre]) -> Result<Vec<f64>, ErreurRaideur> {
    if noeuds.iter().any(|n| !(n.x.is_finite() && n.y.is_finite())) {
        return Err(ErreurRaideur::DonneeInvalide("nœud : coordonnée non finie"));
    }
    let n = 3 * noeuds.len();
    let mut k = vec![0.0; n * n];
    for p in poutres {
        let (t, kt, ddl) = poutre_calculee(noeuds, p)?;
        // `Tᵀ (k T)` : on calcule le triangle supérieur et on le recopie, pour que la matrice soit exactement symétrique.
        for i in 0..6 {
            for j in i..6 {
                let kij: f64 = (0..6).map(|l| t[l][i] * kt[l][j]).sum();
                k[ddl[i] * n + ddl[j]] += kij;
                if i != j {
                    k[ddl[j] * n + ddl[i]] += kij;
                }
            }
        }
    }
    Ok(k)
}

/// Calcule le réseau : assemble ([`matrice_de_raideur`]), impose les appuis, résout, puis tire de la solution les six
/// efforts d'extrémité de chaque poutre.
///
/// # Errors
///
/// - [`ErreurRaideur::DonneeInvalide`] : celles de [`matrice_de_raideur`] ; un appui ou une charge hors limites (l'axe va de 0
///   à 2) ou de valeur non finie ; un appui répété sur le même degré de liberté ;
/// - [`ErreurRaideur::Mecanisme`] : la matrice libre est singulière (au seuil de [`systeme::SEUIL_SINGULIER`]).
pub fn resoudre(
    noeuds: &[Noeud],
    poutres: &[Poutre],
    appuis: &[Appui],
    charges: &[Charge],
) -> Result<SolutionPoutres, ErreurRaideur> {
    let k = matrice_de_raideur(noeuds, poutres)?;
    let (forces, imposes) = ranger_appuis_et_charges(noeuds.len(), 3, appuis, charges)?;
    let resolution = systeme::resoudre_avec_imposes(&k, &forces, &imposes)?;
    let mut efforts = Vec::with_capacity(poutres.len());
    for p in poutres {
        let (_, kt, ddl) = poutre_calculee(noeuds, p)?;
        efforts.push(kt.map(|ligne| (0..6).map(|j| ligne[j] * resolution.deplacements[ddl[j]]).sum()));
    }
    Ok(SolutionPoutres {
        deplacements: resolution.deplacements,
        reactions: resolution.reactions,
        efforts,
        energie: resolution.energie,
    })
}
