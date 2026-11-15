//! Nuage : la raideur d'une colonie selon sa densité (étape 17, M4).
//!
//! **C'est un modèle à nous, pas une reproduction** : pour beaucoup de colonies simulées (des graines, des durées), on lit
//! deux nombres, la **densité** des hyphes dans la cavité et la **raideur** de la colonie (l'essai de poussée de l'étape 15,
//! inchangé), on trace le logarithme de l'une contre le logarithme de l'autre et on ajuste une droite : sa **pente** dit
//! comment la raideur monte avec la densité. Les puissances publiées (1 quand les parois s'étirent, 2 pour les mousses 3D
//! ouvertes, 3 pour le nid d'abeille 2D aux parois minces) sont des **repères**, jamais des cibles : le banc densifie sa
//! colonie par plus d'hyphes, pas par des parois plus épaisses. Aucune pente de colonie n'est publiée.
//!
//! Ce module ne calcule aucune raideur : il la reçoit comme un nombre. Jouer la colonie, la pousser et classer le résultat
//! est la composition de `examples/nuage_raideur.rs`, comme `colonie_raideur.rs` l'a fait à l'étape 15.
//!
//! # La densité
//!
//! `ρ` = la longueur d'hyphe dans la cavité, divisée par le nombre de cases libres de la cavité (en cases⁻¹ : une longueur
//! par surface). [`Cavite::densite`] la lit sur la carte de [`crate::piece::carte_de_densite`], sommée sur les cases
//! libres : ce qui passe dans un mur ne compte pas. Une colonie simulée ne sort pas de sa cavité : sa densité est alors la
//! somme des longueurs de ses segments divisée par le nombre de cases libres, à l'arrondi près. Le diamètre d'une hyphe ne
//! change pas la pente (il multiplierait toutes les densités par un même nombre).
//!
//! # Un point, un nuage
//!
//! Un [`Point`] est `(graine, pas, ρ, K, nœuds)` : une colonie qui traverse, avec `ρ > 0` et `K > 0`. Une colonie
//! **écartée** n'est jamais perdue en silence : elle est rangée avec sa [`Raison`] dans le [`Nuage`], qui les compte
//! ([`Nuage::comptes`]).
//!
//! # L'ajustement
//!
//! [`ajuster`] : les moindres carrés ordinaires de `logarithme(K)` sur `logarithme(ρ)` ([`crate::mesures::logarithme`],
//! aucune fonction de `f64` interdite). Les paires sont d'abord triées : l'ordre donné n'a aucun effet, même au bit près.
//! Ce qui ne s'ajuste pas est une erreur ([`ErreurAjustement`]), jamais un nombre.
//!
//! # Le déterminisme
//!
//! Des `Vec`, des boucles dans un ordre fixe, `+ - * /` : le même résultat, bit à bit, sur toutes les machines.

use crate::croissance::Segment;
use crate::mesures::logarithme;
use crate::piece::carte_de_densite;
use crate::simulation::Simulation;

/// La cavité : la boîte et, case par case (ligne par ligne), celles qu'aucun obstacle ne bloque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cavite {
    /// La largeur de la boîte, en cases.
    pub largeur: usize,
    /// La hauteur de la boîte, en cases.
    pub hauteur: usize,
    /// `largeur × hauteur` booléens, ligne par ligne : vrai pour une case libre.
    pub libres: Vec<bool>,
}

impl Cavite {
    /// La cavité d'une simulation : les cases que son substrat ne bloque pas.
    pub fn de(simulation: &Simulation) -> Cavite {
        let (largeur, hauteur) = (simulation.reglages().largeur, simulation.reglages().hauteur);
        let libres =
            (0..largeur * hauteur).map(|i| !simulation.substrat().est_bloquee(i % largeur, i / largeur)).collect();
        Cavite { largeur, hauteur, libres }
    }

    /// Le nombre de cases libres.
    pub fn cases_libres(&self) -> usize {
        self.libres.iter().filter(|&&libre| libre).count()
    }

    /// La densité `ρ` de ces segments : la somme de la carte de densité sur les cases libres, divisée par leur nombre
    /// (zéro sans segment).
    ///
    /// `None`, jamais un nombre faux ni une panique, si la densité n'existe pas : aucune case libre, une liste `libres`
    /// qui n'a pas `largeur × hauteur` cases, ou un segment qui sort de la boîte (une coordonnée hors de `[0, largeur]`
    /// ou de `[0, hauteur]`, ou qui n'est pas un nombre).
    pub fn densite(&self, segments: &[Segment]) -> Option<f64> {
        let (largeur, hauteur) = (self.largeur as f64, self.hauteur as f64);
        let dans_la_boite = |s: &Segment| {
            [s.x0, s.x1].iter().all(|x| (0.0..=largeur).contains(x))
                && [s.y0, s.y1].iter().all(|y| (0.0..=hauteur).contains(y))
        };
        let libres = self.cases_libres();
        let bien_formee = libres > 0 && self.largeur.checked_mul(self.hauteur) == Some(self.libres.len());
        if !(bien_formee && segments.iter().all(dans_la_boite)) {
            return None;
        }
        let carte = carte_de_densite(self.largeur, self.hauteur, segments);
        let mut somme = 0.0;
        for i in (0..self.libres.len()).filter(|&i| self.libres[i]) {
            somme += carte.valeur(i % self.largeur, i / self.largeur);
        }
        Some(somme / libres as f64)
    }
}

/// Un point du nuage : une colonie qui traverse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// La graine de la simulation.
    pub graine: u64,
    /// Le nombre de pas joués.
    pub pas: u32,
    /// La densité `ρ`, en cases⁻¹ (strictement positive).
    pub densite: f64,
    /// La raideur `K`, en force par case (strictement positive).
    pub raideur: f64,
    /// Le nombre de nœuds de la colonie poussée.
    pub noeuds: usize,
}

/// Pourquoi une colonie n'est pas un point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raison {
    /// Elle ne touche pas les deux mors : `K = 0`, pas de logarithme.
    NeTraversePas,
    /// Elle traverse, mais compte trop de nœuds pour le solveur.
    TropGrande,
    /// Un autre refus : le texte dit lequel.
    Refus(String),
}

/// Une colonie écartée, avec sa raison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ecarte {
    /// La graine de la simulation.
    pub graine: u64,
    /// Le nombre de pas joués.
    pub pas: u32,
    /// Pourquoi elle est écartée.
    pub raison: Raison,
}

/// Les points retenus et les colonies écartées, dans l'ordre où elles ont été lues.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Nuage {
    /// Les colonies qui traversent.
    pub points: Vec<Point>,
    /// Les autres, chacune avec sa raison.
    pub ecartes: Vec<Ecarte>,
}

/// Les comptes d'un nuage : leur somme est le nombre de colonies lues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comptes {
    /// Les points.
    pub retenus: usize,
    /// Les colonies qui ne traversent pas.
    pub ne_traversent_pas: usize,
    /// Les colonies trop grandes pour le solveur.
    pub trop_grandes: usize,
    /// Les autres refus.
    pub refus: usize,
}

impl Nuage {
    /// Les comptes : les points, et les colonies écartées par raison.
    pub fn comptes(&self) -> Comptes {
        let compte = |f: fn(&Raison) -> bool| self.ecartes.iter().filter(|e| f(&e.raison)).count();
        Comptes {
            retenus: self.points.len(),
            ne_traversent_pas: compte(|r| matches!(r, Raison::NeTraversePas)),
            trop_grandes: compte(|r| matches!(r, Raison::TropGrande)),
            refus: compte(|r| matches!(r, Raison::Refus(_))),
        }
    }

    /// Les paires `(ρ, K)` des points, dans leur ordre.
    pub fn paires(&self) -> Vec<(f64, f64)> {
        self.points.iter().map(|p| (p.densite, p.raideur)).collect()
    }
}

/// La droite `log K = ordonnee + pente · log ρ` des moindres carrés.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Droite {
    /// La pente : la puissance de la loi `K = c ρ^pente`.
    pub pente: f64,
    /// L'ordonnée à l'origine : `log c`, la valeur de `log K` pour `ρ = 1`.
    pub ordonnee: f64,
    /// Le coefficient de détermination, de 0 à 1 : le carré de la corrélation de `log ρ` et `log K` (1 si tous les `K`
    /// sont égaux : la droite, plate, passe par tous les points).
    pub r2: f64,
    /// Le nombre de paires ajustées.
    pub points: usize,
}

/// Pourquoi on n'ajuste pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErreurAjustement {
    /// Une densité ou une raideur nulle, négative ou non finie : elle n'a pas de logarithme.
    ValeurInvalide,
    /// Moins de deux densités distinctes : la pente n'existe pas.
    DensitesIdentiques,
}

/// Ajuste `log K` sur `log ρ` aux moindres carrés ordinaires, sur des paires `(ρ, K)`.
///
/// Les paires sont d'abord triées, puis les sommes se font sur les valeurs centrées (leur écart à la moyenne) : le
/// résultat ne dépend pas de l'ordre donné, au bit près.
///
/// # Errors
///
/// [`ErreurAjustement::ValeurInvalide`] si une valeur est nulle, négative ou non finie (ce refus passe avant l'autre) ;
/// [`ErreurAjustement::DensitesIdentiques`] si tous les `log ρ` sont égaux, ou s'il y a moins de deux paires.
pub fn ajuster(paires: &[(f64, f64)]) -> Result<Droite, ErreurAjustement> {
    let valide = |v: f64| v.is_finite() && v > 0.0;
    if !paires.iter().all(|&(rho, k)| valide(rho) && valide(k)) {
        return Err(ErreurAjustement::ValeurInvalide);
    }
    let mut triees = paires.to_vec();
    triees.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let logs: Vec<(f64, f64)> = triees.iter().map(|&(rho, k)| (logarithme(rho), logarithme(k))).collect();
    let n = logs.len() as f64;
    let moyenne_x = logs.iter().map(|p| p.0).sum::<f64>() / n;
    let moyenne_y = logs.iter().map(|p| p.1).sum::<f64>() / n;
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for &(x, y) in &logs {
        let (dx, dy) = (x - moyenne_x, y - moyenne_y);
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    // On décide sur les logarithmes eux-mêmes, pas seulement sur les sommes : la moyenne de logarithmes égaux peut
    // tomber à un bit d'eux, et `sxx` (ou `syy`) valoir 1e-32 au lieu de 0. Deux valeurs voisines d'un bit peuvent avoir
    // le même logarithme : c'est lui qui compte. Sans paire, `all` est vrai.
    if logs.iter().all(|p| p.0 == logs[0].0) || sxx <= 0.0 {
        return Err(ErreurAjustement::DensitesIdentiques);
    }
    let pente = sxy / sxx;
    let raideurs_egales = logs.iter().all(|p| p.1 == logs[0].1);
    let r2 = if raideurs_egales || syy <= 0.0 { 1.0 } else { (sxy * sxy / (sxx * syy)).min(1.0) };
    Ok(Droite { pente, ordonnee: moyenne_y - pente * moyenne_x, r2, points: logs.len() })
}
