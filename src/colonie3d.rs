//! La colonie en volume : des apex qui marchent, se nourrissent et se ramifient dans un cube de substrat.
//!
//! C'est le portage en 3D de la marche, de la longueur de pas tirée du substrat, de la consommation et de la
//! ramification de [`crate::simulation`]. Le chimiotropisme, les obstacles, le thigmotropisme, l'anastomose et la
//! mémoire de direction n'y sont pas (des étapes plus tard). Le substrat est un [`Substrat3d`] ; une direction est une
//! [`Direction3d`], qu'on fait tourner d'angles entiers.
//!
//! # Format du scénario
//!
//! Le texte se lit par lignes. Chaque ligne est débarrassée de ses blancs de tête et de queue ; une ligne alors vide,
//! ou dont le premier caractère est `#`, est ignorée. Les mots sont séparés par des blancs (un ou plusieurs, comme
//! [`crate::simulation::lire_scenario`]).
//!
//! - `cube <largeur> <hauteur> <profondeur>` (entiers strictement positifs) : obligatoire et unique ;
//! - `substrat <initiale> <diffusion>` (nombres finis et positifs ou nuls) : obligatoire et unique ; `initiale` est
//!   déposée dans chaque case, `diffusion` est le coefficient `D` du [`Substrat3d`], de `dt = 1` et `dx = 1` : le
//!   nombre de diffusion vaut donc `diffusion`, et la construction refuse `6 · diffusion > 1` ;
//! - `nourriture <x> <y> <z> <quantite>` (entiers, puis un nombre fini et positif ou nul) : se répète, gardé dans
//!   l'ordre du texte ;
//! - `spore <x> <y> <z>` (entiers) : obligatoire et unique ;
//! - `croissance <pas> <intervalle> <angle> <amplitude>` : obligatoire et unique ; `pas` est un nombre fini
//!   strictement positif, `intervalle` un nombre fini strictement positif ou le mot `infini`, `angle` et `amplitude`
//!   des entiers de `0` à `180` ;
//! - `couplage <affinite> <cout>` : obligatoire et unique ; `affinite` est un nombre fini strictement positif, `cout`
//!   un nombre fini positif ou nul ;
//! - `duree <n>` (entier) : obligatoire et unique.
//!
//! Un entier s'écrit en chiffres seulement (ni signe ni point). Une ligne `bloc`, quelle que soit sa forme, donne
//! [`ErreurColonie3d::BlocRefuse`] avec son numéro, dès qu'elle est lue : les obstacles viendront plus tard. Une
//! ligne inconnue ou mal formée (nombre de mots, nombre illisible ou hors de ses bornes) donne
//! [`ErreurColonie3d::LigneInvalide`] avec son numéro (à partir de 1). Une directive unique répétée donne
//! [`ErreurColonie3d::EnDouble`] (sur la seconde ligne, une fois celle-ci lue sans faute). Une fois tout lu, une
//! directive unique absente donne [`ErreurColonie3d::Manquante`], dans l'ordre `cube`, `substrat`, `spore`,
//! `croissance`, `couplage`, `duree` ; puis une spore hors du cube, puis une nourriture hors du cube (dans l'ordre du
//! texte) donnent [`ErreurColonie3d::HorsDuCube`] avec le numéro de leur ligne.
//!
//! # Le pas de temps
//!
//! [`Colonie3d::avancer`] fait diffuser le substrat, avance l'horloge, puis fait agir chaque apex actif présent au
//! début du pas, dans l'ordre de son identifiant ; les apex nés pendant le pas attendent le suivant. Le détail, tirage
//! par tirage, est dans la doc de [`Colonie3d::avancer`].
//!
//! # L'écriture canonique
//!
//! [`Colonie3d::ecrire_canonique`] concatène, sans remplissage, en petit-boutiste, les `f64` par `to_bits` :
//!
//! 1. `temps` en `u32` ;
//! 2. le nombre de segments en `u64`, puis chaque segment dans l'ordre de [`Colonie3d::segments`], sur 56 octets :
//!    `apex` (`u32`), `temps` (`u32`), `x0`, `y0`, `z0`, `x1`, `y1`, `z1` ;
//! 3. le nombre d'apex en `u64`, puis chaque apex dans l'ordre de son identifiant, sur 53 octets : `id` (`u32`),
//!    `x`, `y`, `z`, les composantes `x`, `y`, `z` de la direction, `actif` (`u8`, 1 ou 0) ;
//! 4. le nombre de ramifications en `u64` ;
//! 5. [`Substrat3d::ecrire_canonique`].
//!
//! Étape 20, vague a : le contrat seul ; le code vient à la vague b.
#![allow(unused_variables)]

use crate::alea::Alea;
use crate::cube::Depot;
use crate::direction3d::Direction3d;
use crate::substrat3d::{ErreurSubstrat3d, Substrat3d};

/// Le plus grand angle permis pour `angle` et `amplitude`, en degrés.
pub const ANGLE_MAXIMAL: u32 = 180;

/// Le plus grand nombre de cases d'un cube que [`Colonie3d::nouvelle`] accepte (`256³`, 128 Mio de substrat) : au-delà,
/// [`ErreurColonie3d::ReglageInvalide`] de nom `cube`, avant toute allocation.
pub const CASES_MAXIMALES: u64 = 256 * 256 * 256;

/// Le scénario d'une colonie en volume : le cube, son substrat, ses nourritures, la spore, la croissance, le couplage
/// et la durée.
///
/// Contrat : un scénario rendu par [`lire_scenario_colonie3d`] respecte les bornes du format (voir la doc du module),
/// et sa spore et ses nourritures sont dans le cube. [`Colonie3d::nouvelle`] revérifie tout cela, sans paniquer, pour
/// un scénario construit à la main.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioColonie3d {
    /// Largeur du cube, en cases (`x`).
    pub largeur: u32,
    /// Hauteur du cube, en cases (`y`).
    pub hauteur: u32,
    /// Profondeur du cube, en cases (`z`).
    pub profondeur: u32,
    /// Quantité de substrat déposée dans chaque case au départ.
    pub substrat_initial: f64,
    /// Coefficient de diffusion du substrat (le nombre de diffusion, `dt = dx = 1`).
    pub diffusion: f64,
    /// Les nourritures, dans l'ordre du texte, déposées après le remplissage uniforme.
    pub depots: Vec<Depot>,
    /// La case de la spore `(x, y, z)`.
    pub spore: (u32, u32, u32),
    /// Longueur de pas maximale d'un apex, en cases.
    pub pas: f64,
    /// Nombre moyen de pas entre deux ramifications d'un apex ; `f64::INFINITY` : aucune ramification.
    pub intervalle: f64,
    /// L'écart d'une branche à la direction de sa mère, en degrés.
    pub angle: u32,
    /// Le plus grand virage d'un pas de marche, en degrés.
    pub amplitude: u32,
    /// Affinité de la colonie pour le substrat.
    pub affinite: f64,
    /// Coût en substrat d'une unité de longueur d'hyphe.
    pub cout: f64,
    /// Durée de la simulation, en pas.
    pub duree: u32,
}

/// Erreur de lecture ou de construction d'une colonie en volume.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurColonie3d {
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne (à partir de 1).
    LigneInvalide(usize),
    /// Une directive unique est répétée ; porte son nom.
    EnDouble(&'static str),
    /// Une directive unique est absente ; porte son nom.
    Manquante(&'static str),
    /// La spore ou une nourriture sort du cube ; porte le numéro de ligne (`0` pour un scénario fait à la main).
    HorsDuCube(usize),
    /// Une ligne `bloc` : les obstacles ne sont pas encore permis ; porte le numéro de ligne.
    BlocRefuse(usize),
    /// Un réglage hors de ses bornes dans un scénario fait à la main ; porte le nom de la directive.
    ReglageInvalide(&'static str),
    /// La construction du substrat en volume a échoué.
    Substrat(ErreurSubstrat3d),
}

/// Un apex : une pointe de la colonie, à une position réelle, avec sa direction.
///
/// Contrat : `id` est son rang de naissance (0, 1, 2… sans trou) ; la position est toujours dans
/// `[0, largeur) × [0, hauteur) × [0, profondeur)`. Dans cette étape, un apex n'est jamais rendu inactif.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apex3d {
    /// Identifiant de l'apex.
    pub id: u32,
    /// Position en `x`.
    pub x: f64,
    /// Position en `y`.
    pub y: f64,
    /// Position en `z`.
    pub z: f64,
    /// Direction de l'apex.
    pub direction: Direction3d,
    /// Vrai si l'apex agit encore.
    pub actif: bool,
}

/// Un segment d'hyphe laissé par un apex pendant un pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment3d {
    /// Identifiant de l'apex qui l'a laissé.
    pub apex: u32,
    /// Le pas où il a été laissé (`1` pour le premier pas).
    pub temps: u32,
    /// Départ en `x`.
    pub x0: f64,
    /// Départ en `y`.
    pub y0: f64,
    /// Départ en `z`.
    pub z0: f64,
    /// Arrivée en `x`.
    pub x1: f64,
    /// Arrivée en `y`.
    pub y1: f64,
    /// Arrivée en `z`.
    pub z1: f64,
}

impl Segment3d {
    /// La longueur du segment : `sqrt(dx * dx + dy * dy + dz * dz)`, dans cet ordre, avec `dx = x1 - x0`, etc.
    pub fn longueur(&self) -> f64 {
        todo!()
    }
}

/// Lit un scénario de colonie en volume depuis son texte (format et ordre des erreurs dans la doc du module).
pub fn lire_scenario_colonie3d(texte: &str) -> Result<ScenarioColonie3d, ErreurColonie3d> {
    todo!()
}

/// Une colonie en volume : ses apex, ses segments, son substrat, son aléa et son horloge.
///
/// Contrat : l'état complet est déterminé par le scénario, la graine et le nombre de pas joués ; mêmes entrées, mêmes
/// octets ([`Colonie3d::ecrire_canonique`]). Aucun apex n'est retiré : il y en a toujours `1 + ramifications`.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Colonie3d {
    scenario: ScenarioColonie3d,
    substrat: Substrat3d,
    alea: Alea,
    apex: Vec<Apex3d>,
    segments: Vec<Segment3d>,
    temps: u32,
    nb_ramifications: u64,
}

impl Colonie3d {
    /// Construit la colonie au temps 0.
    ///
    /// Le substrat est un [`Substrat3d`] du cube, de coefficient `diffusion`, `dt = 1.0` et `dx = 1.0` ; la quantité
    /// `substrat_initial` est déposée dans chaque case si elle est strictement positive, puis chaque nourriture dans
    /// l'ordre. Un seul apex, d'identifiant `0`, actif, part du centre de la case de la spore
    /// `(sx + 0.5, sy + 0.5, sz + 0.5)`, dans la direction [`Direction3d::au_hasard`] tirée de `Alea::new(graine)`.
    ///
    /// Contrat : rend [`ErreurColonie3d::ReglageInvalide`] pour un réglage hors des bornes du format ou un cube de plus
    /// de [`CASES_MAXIMALES`] cases, [`ErreurColonie3d::HorsDuCube`] (de numéro `0`) pour une spore ou une nourriture
    /// hors du cube (une dimension nulle en fait partie : la spore n'y tient pas), et [`ErreurColonie3d::Substrat`]
    /// pour un refus de [`Substrat3d::nouveau`] (`6 · diffusion > 1`). Ne panique pas.
    pub fn nouvelle(scenario: &ScenarioColonie3d, graine: u64) -> Result<Colonie3d, ErreurColonie3d> {
        todo!()
    }

    /// Avance la colonie d'un pas.
    ///
    /// Contrat, dans cet ordre :
    ///
    /// 1. le substrat diffuse ([`Substrat3d::pas`]), puis `temps` croît de 1 ;
    /// 2. chaque apex actif présent au début du pas, dans l'ordre de son identifiant (les naissances du pas attendent
    ///    le suivant) :
    ///    1. tourne : `theta = alea.suivant() % (amplitude + 1)`, puis `phi = alea.suivant() % 360`, et
    ///       `direction = direction.tourner(theta, phi)` — deux tirages toujours, même à `amplitude = 0` ;
    ///    2. lit `s`, le substrat de sa case (`x`, `y`, `z` tronqués vers le bas), et en tire
    ///       `longueur = si s ≤ 0 { 0 } sinon { pas * s / (affinite + s) }`, ramenée à `s / cout` si `cout > 0` et
    ///       `cout * longueur > s` ;
    ///    3. calcule l'arrivée `x + longueur * d.x()`, et de même en `y` et `z` ;
    ///    4. si `longueur ≤ 0` ou si l'arrivée sort de `[0, largeur) × [0, hauteur) × [0, profondeur)`, reste où il est
    ///       avec sa nouvelle direction, sans consommer, laisser, se ramifier ni tirer davantage ;
    ///    5. sinon retire `cout * longueur` de sa case de départ ([`Substrat3d::retirer`], qui ne retire jamais plus
    ///       que la case ne contient), laisse un [`Segment3d`] du départ à l'arrivée, et s'y déplace ;
    ///    6. si `intervalle` est fini, tire `alea.suivant_f64()` ; s'il est `< 1.0 / intervalle`, tire
    ///       `phi = alea.suivant() % 360`, et un apex neuf, actif, d'identifiant suivant, naît à l'arrivée dans la
    ///       direction `direction.tourner(angle, phi)` ; les ramifications croissent de 1.
    pub fn avancer(&mut self) {
        todo!()
    }

    /// Avance la colonie de `n` pas : strictement équivalent à `n` appels à [`Colonie3d::avancer`].
    pub fn avancer_n(&mut self, n: u32) {
        todo!()
    }

    /// Le nombre de pas déjà joués.
    pub fn temps(&self) -> u32 {
        todo!()
    }

    /// La durée du scénario, en pas.
    pub fn duree(&self) -> u32 {
        todo!()
    }

    /// Le scénario de la colonie, tel qu'il a été donné.
    pub fn scenario(&self) -> &ScenarioColonie3d {
        todo!()
    }

    /// Le substrat courant.
    pub fn substrat(&self) -> &Substrat3d {
        todo!()
    }

    /// Tous les apex, actifs ou non, dans l'ordre de leur identifiant (0, 1, 2… sans trou).
    pub fn apex(&self) -> &[Apex3d] {
        todo!()
    }

    /// Tous les segments, dans l'ordre où ils ont été laissés (par pas, puis par identifiant d'apex).
    pub fn segments(&self) -> &[Segment3d] {
        todo!()
    }

    /// Le nombre cumulé de ramifications.
    pub fn nb_ramifications(&self) -> u64 {
        todo!()
    }

    /// Le nombre d'apex actifs : les pointes de la colonie.
    pub fn pointes(&self) -> usize {
        todo!()
    }

    /// L'écriture canonique de la colonie (format dans la doc du module). Longueur :
    /// `4 + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases` octets.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        todo!()
    }

    /// L'empreinte de la colonie : le SHA-256 de [`Colonie3d::ecrire_canonique`], en hexadécimal minuscule de 64
    /// caractères.
    pub fn empreinte(&self) -> String {
        todo!()
    }
}
