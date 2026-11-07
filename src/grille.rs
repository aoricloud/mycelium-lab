//! L'état simulé.
//!
//! La graine fait partie de l'état : l'état d'un scénario vide dépend donc de la graine, et deux graines
//! différentes donnent deux empreintes différentes.
//!
//! # Écriture canonique
//!
//! [`Etat::ecrire_canonique`] rend une suite d'octets d'ordre fixe, entiers en little-endian, indépendante
//! du système (taille de mot, endianness).

use crate::empreinte::sha256_hex;
use crate::scenario::Scenario;

/// L'état simulé d'une boîte de Petri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etat {
    /// Largeur de la boîte, en cases.
    pub largeur: u32,
    /// Hauteur de la boîte, en cases.
    pub hauteur: u32,
    /// Graine de la simulation, partie de l'état.
    pub graine: u64,
}

impl Etat {
    /// Construit l'état initial d'un scénario avec une graine.
    pub fn nouveau(scenario: &Scenario, graine: u64) -> Etat {
        Etat { largeur: scenario.largeur, hauteur: scenario.hauteur, graine }
    }

    /// Écriture canonique de l'état en octets (ordre fixe, entiers en little-endian).
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let mut octets = Vec::with_capacity(16);
        octets.extend_from_slice(&self.largeur.to_le_bytes());
        octets.extend_from_slice(&self.hauteur.to_le_bytes());
        octets.extend_from_slice(&self.graine.to_le_bytes());
        octets
    }

    /// Empreinte de l'état : SHA-256 de l'écriture canonique, en hexadécimal minuscule (64 caractères).
    pub fn empreinte(&self) -> String {
        sha256_hex(&self.ecrire_canonique())
    }
}
