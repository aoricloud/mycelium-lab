//! Le scénario du cube et sa simulation : l'étape 18, une couche à part qui ne change rien au plan 2D.
//!
//! # Format
//!
//! Le texte se lit par lignes. Une ligne vide, ou dont le premier caractère est `#`, est ignorée. Les mots d'une
//! ligne sont séparés par une espace.
//!
//! - `cube <largeur> <hauteur> <profondeur>` (entiers strictement positifs) est obligatoire et unique ;
//! - `substrat <initial> <diffusion>` (nombres finis et positifs ou nuls) est obligatoire et unique ;
//! - `duree <n>` (entier) est obligatoire et unique ;
//! - `nourriture <x> <y> <z> <quantite>` (entiers, puis un nombre fini et positif ou nul) se répète, gardé dans
//!   l'ordre du texte ;
//! - `bloc <x0> <y0> <z0> <x1> <y1> <z1>` (entiers, `x0 ≤ x1`, `y0 ≤ y1`, `z0 ≤ z1`) se répète, gardé dans l'ordre
//!   du texte.
//!
//! Une ligne inconnue ou mal formée donne [`ErreurCube::LigneInvalide`] avec son numéro (à partir de 1). Une
//! directive unique répétée donne [`ErreurCube::EnDouble`], une directive unique absente
//! [`ErreurCube::Manquante`], dans l'ordre `cube`, `substrat`, `duree`. Une fois tout lu, une nourriture ou un bloc
//! qui sort du cube donne [`ErreurCube::HorsDuCube`] (nourritures d'abord, puis blocs), et une nourriture dans un
//! bloc donne [`ErreurCube::DepotBloque`].

// Les corps à écrire dans la vague du code emploieront leurs paramètres ; la vague du code retire cet attribut.
#![allow(unused_variables)]

use crate::substrat3d::{ErreurSubstrat3d, Substrat3d};

/// Une nourriture déposée dans une case du cube.
#[derive(Clone, Debug, PartialEq)]
pub struct Depot {
    /// Colonne du dépôt.
    pub x: u32,
    /// Ligne du dépôt.
    pub y: u32,
    /// Profondeur du dépôt.
    pub z: u32,
    /// Quantité déposée.
    pub quantite: f64,
}

/// Un pavé de cases bloquées, bornes comprises.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bloc {
    /// Première colonne du pavé.
    pub x0: u32,
    /// Première ligne du pavé.
    pub y0: u32,
    /// Première profondeur du pavé.
    pub z0: u32,
    /// Dernière colonne du pavé.
    pub x1: u32,
    /// Dernière ligne du pavé.
    pub y1: u32,
    /// Dernière profondeur du pavé.
    pub z1: u32,
}

/// Le scénario d'un cube : ses dimensions, son substrat, ses dépôts, ses blocs et sa durée.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioCube {
    /// Largeur du cube, en cases.
    pub largeur: u32,
    /// Hauteur du cube, en cases.
    pub hauteur: u32,
    /// Profondeur du cube, en cases.
    pub profondeur: u32,
    /// Quantité de substrat déposée dans chaque case au départ.
    pub substrat_initial: f64,
    /// Coefficient de diffusion du substrat.
    pub diffusion: f64,
    /// Les nourritures, dans l'ordre du texte.
    pub depots: Vec<Depot>,
    /// Les blocs, dans l'ordre du texte.
    pub blocs: Vec<Bloc>,
    /// Durée de la simulation, en pas.
    pub duree: u32,
}

/// Erreur de lecture ou de construction d'un scénario de cube.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurCube {
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne (à partir de 1).
    LigneInvalide(usize),
    /// Une directive unique est répétée ; porte son nom.
    EnDouble(&'static str),
    /// Une directive unique est absente ; porte son nom.
    Manquante(&'static str),
    /// Une nourriture ou un bloc sort du cube ; porte le numéro de ligne.
    HorsDuCube(usize),
    /// Une nourriture tombe dans un bloc ; porte le numéro de ligne de la nourriture.
    DepotBloque(usize),
    /// La construction du substrat en volume a échoué.
    Substrat(ErreurSubstrat3d),
}

/// Lit un entier décimal non signé (chiffres seulement).
#[allow(dead_code)] // Appelé par les corps à écrire dans la vague du code ; la vague du code retire cet attribut.
fn entier(mot: &str) -> Option<u32> {
    todo!()
}

/// Lit un nombre fini et positif ou nul.
#[allow(dead_code)] // Appelé par les corps à écrire dans la vague du code ; la vague du code retire cet attribut.
fn nombre(mot: &str) -> Option<f64> {
    todo!()
}

/// Lit un scénario de cube depuis son texte (format décrit dans la doc du module).
pub fn lire_scenario_cube(texte: &str) -> Result<ScenarioCube, ErreurCube> {
    todo!()
}

/// Un cube simulé : son horloge, sa durée et son substrat en volume.
///
/// Contrat : le temps est le nombre de pas déjà faits ; le substrat est la seule chose qui évolue. Tout est
/// déterministe : mêmes paramètres, mêmes dépôts, même nombre de pas, mêmes octets.
// Les champs ne sont lus que par les corps à écrire dans la vague du code ; la vague du code retire cet attribut.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct Cube {
    temps: u32,
    duree: u32,
    substrat: Substrat3d,
}

impl Cube {
    /// Construit un cube depuis un scénario : substrat de `dt = 1.0` et `dx = 1.0`, quantité initiale déposée
    /// dans chaque case si elle est strictement positive, puis les blocs dans l'ordre, puis les nourritures dans
    /// l'ordre. Le temps part de `0`.
    ///
    /// Contrat : un refus de [`Substrat3d::nouveau`] est rendu en [`ErreurCube::Substrat`].
    pub fn nouveau(scenario: &ScenarioCube) -> Result<Cube, ErreurCube> {
        todo!()
    }

    /// Avance la simulation de `n` pas et ajoute `n` au temps.
    pub fn avancer(&mut self, n: u32) {
        todo!()
    }

    /// Le nombre de pas déjà faits.
    pub fn temps(&self) -> u32 {
        todo!()
    }

    /// La durée du scénario, en pas.
    pub fn duree(&self) -> u32 {
        todo!()
    }

    /// Le substrat en volume.
    pub fn substrat(&self) -> &Substrat3d {
        todo!()
    }

    /// L'écriture canonique du cube : le temps en `u32` petit-boutiste, puis l'écriture canonique du substrat.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        todo!()
    }

    /// Empreinte du cube : le SHA-256 de [`Cube::ecrire_canonique`], en hexadécimal minuscule de 64 caractères.
    pub fn empreinte(&self) -> String {
        todo!()
    }
}
