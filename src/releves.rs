//! Relevés : les mesures faites au fil de la simulation.
//!
//! Un relevé rassemble, à un pas de temps donné, les mesures de la colonie. [`en_csv`] les écrit en CSV,
//! pour être relus et comparés.

/// Un relevé de la simulation, à un pas de temps donné.
///
/// Contrat : `temps` est le pas de temps, `biomasse` la masse de la colonie, `pointes` le nombre d'apex,
/// `ramifications` et `anastomoses` les nombres cumulés de ramifications et d'anastomoses, `rayon` le rayon
/// de la colonie et `substrat` le substrat qui reste. Les sept champs sont publics et se lisent tels quels ;
/// aucune valeur n'est réinterprétée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Releve {
    pub temps: u32,
    pub biomasse: f64,
    pub pointes: usize,
    pub ramifications: u64,
    pub anastomoses: u64,
    pub rayon: f64,
    pub substrat: f64,
}

/// L'en-tête du CSV écrit par [`en_csv`], sans saut de ligne : les noms des sept colonnes, dans l'ordre des
/// champs de [`Releve`], séparés par des virgules sans espace.
pub const ENTETE_CSV: &str = "temps,biomasse,pointes,ramifications,anastomoses,rayon,substrat";

/// Écrit `releves` en CSV : [`ENTETE_CSV`], un saut de ligne, puis une ligne par relevé, chacune finie par un
/// saut de ligne.
///
/// Contrat : chaque ligne porte les sept champs dans l'ordre de [`Releve`], séparés par des virgules sans
/// espace ; chaque nombre est écrit par `{}` (Display) de Rust. Une liste vide rend l'en-tête et son saut de
/// ligne, rien d'autre. Les sauts de ligne sont des `\n` : pas de `\r`, pas de BOM. Le texte se relit sans
/// perte : chaque nombre reparsé redonne la valeur du relevé (Display d'un `f64` produit sa forme la plus
/// courte qui se relit à l'identique).
#[allow(unused_variables)] // le corps est `todo!()` jusqu'à la vague suivante
pub fn en_csv(releves: &[Releve]) -> String {
    todo!()
}
