//! Substrat : diffusion explicite à cinq points, avec des bords sans flux.
//!
//! Le substrat est un champ de nutriments `c(x, y)` sur une grille rectangulaire de `largeur × hauteur` cases,
//! de côté `dx`. Il suit l'équation de diffusion `∂c/∂t = D ∇²c`, intégrée par le schéma explicite à cinq points
//! (Euler avant en temps, différences centrées en espace) :
//!
//! `c'(i, j) = c(i, j) + r · (c(i-1, j) + c(i+1, j) + c(i, j-1) + c(i, j+1) - 4 c(i, j))`, avec `r = D dt / dx²`.
//!
//! Aux bords, il n'y a aucun flux : un voisin qui sortirait de la grille est remplacé par la case elle-même
//! (condition de Neumann par réflexion), donc rien ne sort de la grille et la masse totale est conservée.
//! Le schéma est stable si `r ≤ 1/4` ; le module refuse tout paramètre qui ne l'est pas, à la construction.
//!
//! Le calcul n'utilise que `+`, `-`, `*`, `/` : il est déterministe d'une machine à l'autre. Le pas lit l'ancien
//! champ en entier avant d'écrire le nouveau (pas de mise à jour en place), donc l'ordre des cases n'influe pas
//! sur le résultat.

/// Pourquoi une construction de [`Substrat`] est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurSubstrat {
    /// La largeur ou la hauteur est nulle.
    DimensionNulle,
    /// `D`, `dt` ou `dx` n'est pas fini, ou `D < 0`, ou `dt ≤ 0`, ou `dx ≤ 0`.
    ParametreInvalide,
    /// `D dt / dx²` dépasse `1/4` : le schéma explicite serait instable.
    PasInstable,
}

/// Un champ de nutriments sur une grille, et ses paramètres de diffusion.
///
/// Contrat : les valeurs sont des `f64` rangés ligne par ligne (`y` en indice lent, `x` en indice rapide). Un
/// champ neuf est nul partout. Les coordonnées `(x, y)` vont de `0` à `largeur - 1` et de `0` à `hauteur - 1`.
/// Tout est déterministe : mêmes paramètres, mêmes dépôts, même nombre de pas, mêmes octets.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // les champs sont lus par le code de la vague suivante
pub struct Substrat {
    largeur: usize,
    hauteur: usize,
    d: f64,
    dt: f64,
    dx: f64,
    cases: Vec<f64>,
}

#[allow(unused_variables)] // les corps sont `todo!()` jusqu'à la vague suivante
impl Substrat {
    /// Construit un champ nul de `largeur × hauteur` cases, de coefficient de diffusion `d`, de pas de temps
    /// `dt` et de côté de case `dx` (`d` en longueur²/temps, `dt` en temps, `dx` en longueur).
    ///
    /// Contrat : refuse avec [`ErreurSubstrat::DimensionNulle`] si une dimension est nulle ; avec
    /// [`ErreurSubstrat::ParametreInvalide`] si `d`, `dt` ou `dx` n'est pas fini, si `d < 0`, `dt ≤ 0` ou
    /// `dx ≤ 0` (`d = 0` est permis : le champ ne bouge pas) ; avec [`ErreurSubstrat::PasInstable`] si
    /// `d * dt / (dx * dx) > 0.25`. La limite `0.25` elle-même est acceptée.
    pub fn nouveau(largeur: usize, hauteur: usize, d: f64, dt: f64, dx: f64) -> Result<Substrat, ErreurSubstrat> {
        todo!()
    }

    /// Nombre de cases en `x`.
    pub fn largeur(&self) -> usize {
        todo!()
    }

    /// Nombre de cases en `y`.
    pub fn hauteur(&self) -> usize {
        todo!()
    }

    /// Le nombre de diffusion `r = D dt / dx²`, toujours dans `[0, 0.25]` pour un substrat construit.
    pub fn nombre_de_diffusion(&self) -> f64 {
        todo!()
    }

    /// La quantité de nutriments dans la case `(x, y)`.
    ///
    /// Contrat : panique si `x >= largeur()` ou `y >= hauteur()`.
    pub fn valeur(&self, x: usize, y: usize) -> f64 {
        todo!()
    }

    /// Ajoute `quantite` à la case `(x, y)` (la masse totale croît d'autant).
    ///
    /// Contrat : panique si la case est hors de la grille, ou si `quantite` n'est pas finie ou est négative.
    pub fn deposer(&mut self, x: usize, y: usize, quantite: f64) {
        todo!()
    }

    /// Avance le champ d'un pas de temps `dt` par le schéma à cinq points, bords sans flux.
    ///
    /// Contrat : la masse totale est conservée (aux erreurs d'arrondi près) ; l'ancien champ est lu en entier
    /// avant d'écrire le nouveau ; l'ordre de calcul est fixe.
    pub fn pas(&mut self) {
        todo!()
    }

    /// Avance le champ de `n` pas : strictement équivalent à appeler [`Substrat::pas`] `n` fois, au même
    /// octet près. `n = 0` ne change rien.
    pub fn pas_n(&mut self, n: usize) {
        todo!()
    }

    /// La somme des valeurs de toutes les cases, additionnées dans l'ordre des lignes puis des colonnes
    /// (`y` lent, `x` rapide).
    pub fn masse_totale(&self) -> f64 {
        todo!()
    }

    /// L'écriture canonique du champ, pour comparer « à l'octet » et pour l'empreinte.
    ///
    /// Format : `largeur` puis `hauteur`, chacun en `u64` petit-boutiste (8 octets), puis la valeur de chaque
    /// case, ligne par ligne (`y` lent, `x` rapide), en `f64::to_bits` petit-boutiste (8 octets). Les paramètres
    /// `d`, `dt` et `dx` n'en font pas partie. Longueur : `16 + 8 * largeur * hauteur` octets.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        todo!()
    }
}
