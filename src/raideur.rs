//! Raideur : le calcul de raideur d'un réseau de barres articulées (étape 13, M3).
//!
//! Des [`Noeud`]s dans le plan, des [`Barre`]s articulées entre deux nœuds (elles ne travaillent qu'en traction et en
//! compression, jamais en flexion), des [`Appui`]s (un déplacement imposé : zéro pour un appui, non nul pour « un bord
//! poussé ») et des [`Charge`]s (des forces aux nœuds). [`resoudre`] rend la [`Solution`] : les déplacements, les
//! réactions, l'effort de chaque barre et l'énergie. Le noyau de la simulation n'en sait rien : c'est un calcul à part,
//! sur un réseau **abstrait** (aucun matériau, aucune colonie).
//!
//! # Les conventions
//!
//! - Chaque nœud a deux degrés de liberté, rangés `[u₀, v₀, u₁, v₁, …]` : le degré de liberté de l'axe `axe` (0 pour `x`,
//!   1 pour `y`) du nœud `i` est `2 i + axe`. Les déplacements et les réactions sont rangés ainsi.
//! - Les unités sont celles de l'appelant, cohérentes entre elles : `ea` en force, une longueur en longueur, un
//!   déplacement en longueur. Le code ne connaît ni mètre ni micromètre.
//! - L'effort d'une barre est **positif en traction**. Une réaction est la force que l'appui exerce sur la structure.
//! - **Petits déplacements, comportement linéaire** : la raideur est celle de la forme de départ, jamais remise à jour.
//!
//! # La barre
//!
//! Une barre de longueur `L`, de direction `e = (c, s) = (dx/L, dy/L)` du nœud `a` au nœud `b`, et de raideur axiale
//! `EA` s'allonge de `δ = e·(u_b − u_a)` et tire d'un effort `N = EA δ / L`. Sa matrice de raideur est `(EA/L) g gᵀ` avec
//! `g = (−c, −s, c, s)` : deux barres entre les mêmes nœuds s'ajoutent (parallèle), deux barres bout à bout se partagent
//! l'allongement (série).
//!
//! # Le déterminisme
//!
//! Parmi les fonctions de `f64`, seulement `+ - * /`, `sqrt`, `abs`, `min`, `max`, `clamp` et `as` : jamais `sin`, `cos`,
//! `tan`, `atan2`, `exp`, `ln`, `log2`, `log10`, `powf`, `powi`, `hypot`, `mul_add`. Le cosinus et le sinus d'une barre
//! viennent de `dx/L` et `dy/L`. Des boucles dans un ordre fixe, ni `HashMap` ni fil d'exécution : le même résultat, bit à
//! bit, sur toutes les machines.

use crate::systeme::{self, Mecanisme};

/// Un nœud : ses deux coordonnées.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Noeud {
    pub x: f64,
    pub y: f64,
}

/// Une barre articulée entre les nœuds `a` et `b` (des indices de la liste des nœuds), de raideur axiale `ea` (force).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Barre {
    pub a: usize,
    pub b: usize,
    pub ea: f64,
}

/// Un degré de liberté imposé : le nœud `noeud`, l'axe `axe` (0 pour `x`, 1 pour `y`) prend le déplacement `valeur`.
/// Zéro pour un appui ; non nul pour un bord que l'on pousse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appui {
    pub noeud: usize,
    pub axe: usize,
    pub valeur: f64,
}

/// Une force `valeur` au nœud `noeud`, selon l'axe `axe` (0 pour `x`, 1 pour `y`). Plusieurs charges sur le même degré
/// de liberté s'ajoutent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Charge {
    pub noeud: usize,
    pub axe: usize,
    pub valeur: f64,
}

/// Ce qui empêche le calcul.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErreurRaideur {
    /// La structure a un mécanisme : un mouvement sans énergie (un carré sans diagonale, une barre dont le nœud libre
    /// n'est pas tenu en travers, trop peu d'appuis, un nœud sans barre), ou elle est si mal conditionnée que le calcul
    /// ne la distingue pas d'un mécanisme (voir [`crate::systeme`]). Aucun déplacement n'est rendu.
    Mecanisme,
    /// Une donnée impossible : le texte dit laquelle.
    DonneeInvalide(&'static str),
}

impl From<Mecanisme> for ErreurRaideur {
    fn from(_: Mecanisme) -> Self {
        ErreurRaideur::Mecanisme
    }
}

/// La solution d'un réseau de barres.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// Les déplacements `[u₀, v₀, u₁, v₁, …]`.
    pub deplacements: Vec<f64>,
    /// Les réactions, rangées comme les déplacements : la force de l'appui à chaque degré de liberté imposé, zéro ailleurs.
    /// Avec les charges, leur somme est nulle (équilibre).
    pub reactions: Vec<f64>,
    /// L'effort axial de chaque barre, dans l'ordre des barres : positif en traction.
    pub efforts: Vec<f64>,
    /// L'énergie de déformation `½ uᵀ K u` (égale à `Σ ½ N² L / EA`).
    pub energie: f64,
}

/// Les degrés de liberté imposés : des couples (degré de liberté, valeur).
pub type Imposes = Vec<(usize, f64)>;

/// Contrôle les appuis et les charges d'un réseau de `nombre_de_noeuds` nœuds à `ddl_par_noeud` degrés de liberté chacun
/// (2 pour des barres), et les range : le vecteur des forces, et la liste (degré de liberté, valeur) des degrés de
/// liberté imposés, dans l'ordre des appuis. Sert aussi aux poutres.
///
/// # Errors
///
/// [`ErreurRaideur::DonneeInvalide`] pour un nœud ou un axe hors limites, une valeur non finie, ou un degré de liberté
/// imposé deux fois.
pub fn ranger_appuis_et_charges(
    nombre_de_noeuds: usize,
    ddl_par_noeud: usize,
    appuis: &[Appui],
    charges: &[Charge],
) -> Result<(Vec<f64>, Imposes), ErreurRaideur> {
    let invalide = ErreurRaideur::DonneeInvalide;
    let mut forces = vec![0.0; nombre_de_noeuds * ddl_par_noeud];
    let mut imposes = Imposes::new();
    for charge in charges {
        if charge.noeud >= nombre_de_noeuds || charge.axe >= ddl_par_noeud {
            return Err(invalide("charge : nœud ou axe hors limites"));
        }
        if !charge.valeur.is_finite() {
            return Err(invalide("charge : valeur non finie"));
        }
        forces[charge.noeud * ddl_par_noeud + charge.axe] += charge.valeur;
    }
    for appui in appuis {
        if appui.noeud >= nombre_de_noeuds || appui.axe >= ddl_par_noeud {
            return Err(invalide("appui : nœud ou axe hors limites"));
        }
        if !appui.valeur.is_finite() {
            return Err(invalide("appui : valeur non finie"));
        }
        let ddl = appui.noeud * ddl_par_noeud + appui.axe;
        if imposes.iter().any(|&(deja, _)| deja == ddl) {
            return Err(invalide("appui : le même degré de liberté imposé deux fois"));
        }
        imposes.push((ddl, appui.valeur));
    }
    Ok((forces, imposes))
}

/// La longueur d'un segment et sa direction `(c, s) = (dx/L, dy/L)`, depuis le nœud `a` vers le nœud `b`.
///
/// # Errors
///
/// [`ErreurRaideur::DonneeInvalide`] pour un indice hors limites, une coordonnée non finie ou une longueur nulle.
pub fn longueur_et_direction(noeuds: &[Noeud], a: usize, b: usize) -> Result<(f64, f64, f64), ErreurRaideur> {
    let (Some(na), Some(nb)) = (noeuds.get(a), noeuds.get(b)) else {
        return Err(ErreurRaideur::DonneeInvalide("barre : indice de nœud hors limites"));
    };
    let (dx, dy) = (nb.x - na.x, nb.y - na.y);
    let longueur = (dx * dx + dy * dy).sqrt();
    if !(longueur.is_finite() && longueur > 0.0) {
        return Err(ErreurRaideur::DonneeInvalide("barre : longueur nulle ou non finie"));
    }
    Ok((longueur, dx / longueur, dy / longueur))
}

/// La raideur `EA/L` d'une barre, les quatre degrés de liberté de ses extrémités `[2a, 2a+1, 2b, 2b+1]` et le vecteur
/// `g = (−c, −s, c, s)` : sa matrice de raideur est `(EA/L) g gᵀ` et son effort `(EA/L) g·u`.
fn barre_calculee(noeuds: &[Noeud], barre: &Barre) -> Result<(f64, [usize; 4], [f64; 4]), ErreurRaideur> {
    if !(barre.ea.is_finite() && barre.ea > 0.0) {
        return Err(ErreurRaideur::DonneeInvalide("barre : ea doit être fini et strictement positif"));
    }
    let (longueur, c, s) = longueur_et_direction(noeuds, barre.a, barre.b)?;
    let ddl = [2 * barre.a, 2 * barre.a + 1, 2 * barre.b, 2 * barre.b + 1];
    Ok((barre.ea / longueur, ddl, [-c, -s, c, s]))
}

/// La matrice de raideur du réseau avant tout appui : `2n × 2n` (`n` nœuds), pleine, symétrique, rangée ligne par ligne
/// (voir [`crate::systeme`]). Les barres s'ajoutent dans l'ordre de la liste.
///
/// # Errors
///
/// [`ErreurRaideur::DonneeInvalide`] : une coordonnée non finie ; une barre dont un indice sort de la liste, de longueur
/// nulle, ou dont `ea` n'est pas fini et strictement positif.
pub fn matrice_de_raideur(noeuds: &[Noeud], barres: &[Barre]) -> Result<Vec<f64>, ErreurRaideur> {
    if noeuds.iter().any(|n| !(n.x.is_finite() && n.y.is_finite())) {
        return Err(ErreurRaideur::DonneeInvalide("nœud : coordonnée non finie"));
    }
    let n = 2 * noeuds.len();
    let mut k = vec![0.0; n * n];
    for barre in barres {
        let (raideur, ddl, g) = barre_calculee(noeuds, barre)?;
        for (i, &di) in ddl.iter().enumerate() {
            for (j, &dj) in ddl.iter().enumerate() {
                k[di * n + dj] += raideur * (g[i] * g[j]); // `g[i] * g[j]` d'abord : la matrice est exactement symétrique
            }
        }
    }
    Ok(k)
}

/// Calcule le réseau : assemble la matrice de raideur ([`matrice_de_raideur`]), impose les appuis, résout, puis tire de
/// la solution l'effort de chaque barre.
///
/// # Errors
///
/// - [`ErreurRaideur::DonneeInvalide`] : celles de [`matrice_de_raideur`] ; un appui ou une charge hors limites ou de
///   valeur non finie ; un appui répété sur le même degré de liberté ;
/// - [`ErreurRaideur::Mecanisme`] : la matrice libre est singulière (au seuil de [`systeme::SEUIL_SINGULIER`]).
pub fn resoudre(
    noeuds: &[Noeud],
    barres: &[Barre],
    appuis: &[Appui],
    charges: &[Charge],
) -> Result<Solution, ErreurRaideur> {
    let k = matrice_de_raideur(noeuds, barres)?;
    let (forces, imposes) = ranger_appuis_et_charges(noeuds.len(), 2, appuis, charges)?;
    let resolution = systeme::resoudre_avec_imposes(&k, &forces, &imposes)?;
    let mut efforts = Vec::with_capacity(barres.len());
    for barre in barres {
        let (raideur, ddl, g) = barre_calculee(noeuds, barre)?;
        let mut allongement = 0.0;
        for (&d, &gi) in ddl.iter().zip(&g) {
            allongement += gi * resolution.deplacements[d];
        }
        efforts.push(raideur * allongement);
    }
    Ok(Solution {
        deplacements: resolution.deplacements,
        reactions: resolution.reactions,
        efforts,
        energie: resolution.energie,
    })
}
