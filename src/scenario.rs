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

/// Lit un entier décimal non signé (chiffres seulement).
fn entier(mot: &str) -> Option<u32> {
    if mot.is_empty() || !mot.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    mot.parse().ok()
}

impl Scenario {
    /// Lit un scénario depuis son texte (format décrit dans la doc du module).
    pub fn lire(texte: &str) -> Result<Scenario, ErreurScenario> {
        let mut boite: Option<(u32, u32)> = None;
        let mut spores = Vec::new();
        for (i, ligne) in texte.lines().enumerate() {
            let numero = i + 1;
            if ligne.is_empty() || ligne.starts_with('#') {
                continue;
            }
            let mots: Vec<&str> = ligne.split(' ').collect();
            match mots.as_slice() {
                ["boite", l, h] => {
                    let (l, h) = match (entier(l), entier(h)) {
                        (Some(l), Some(h)) if l > 0 && h > 0 => (l, h),
                        _ => return Err(ErreurScenario::LigneInvalide(numero)),
                    };
                    if boite.is_some() {
                        return Err(ErreurScenario::BoiteEnDouble);
                    }
                    boite = Some((l, h));
                }
                ["spore", x, y] => match (entier(x), entier(y)) {
                    (Some(x), Some(y)) => spores.push(Spore { x, y }),
                    _ => return Err(ErreurScenario::LigneInvalide(numero)),
                },
                _ => return Err(ErreurScenario::LigneInvalide(numero)),
            }
        }
        let (largeur, hauteur) = boite.ok_or(ErreurScenario::BoiteAbsente)?;
        Ok(Scenario { largeur, hauteur, spores })
    }
}
