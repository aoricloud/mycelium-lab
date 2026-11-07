//! Le scénario : une boîte de Petri et ses spores, lus depuis un texte.
//!
//! # Format
//!
//! Le texte se lit par lignes. Une ligne vide, ou dont le premier caractère est `#`, est ignorée.
//! Une ligne `boite <largeur> <hauteur>` (deux entiers décimaux strictement positifs, séparés par une
//! espace) est obligatoire et unique. Des lignes `spore <x> <y>` pourront suivre dans les étapes
//! suivantes ; le scénario vide n'en contient aucune.
//!
//! Exemple exact de scénario vide (se termine par un saut de ligne) :
//!
//! ```text
//! boite 64 48
//! ```
//!
//! soit la chaîne Rust `"boite 64 48\n"`, qui se lit en un [`Scenario`] de largeur 64, de hauteur 48 et
//! sans spore.

/// Une spore posée dans la boîte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spore {
    /// Colonne de la spore.
    pub x: u32,
    /// Ligne de la spore.
    pub y: u32,
}

/// Une boîte de Petri : sa largeur, sa hauteur et ses spores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    /// Largeur de la boîte, en cases.
    pub largeur: u32,
    /// Hauteur de la boîte, en cases.
    pub hauteur: u32,
    /// Les spores, dans l'ordre du texte (vide à l'étape 0).
    pub spores: Vec<Spore>,
}

/// Erreur de lecture d'un scénario.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErreurScenario {
    /// Aucune ligne `boite`.
    BoiteAbsente,
    /// Plusieurs lignes `boite`.
    BoiteEnDouble,
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne (à partir de 1).
    LigneInvalide(usize),
}

impl Scenario {
    /// Lit un scénario depuis son texte (format décrit dans la doc du module).
    #[allow(unused_variables)]
    pub fn lire(texte: &str) -> Result<Scenario, ErreurScenario> {
        todo!()
    }
}
