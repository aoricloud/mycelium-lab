//! Simulation : le couplage entre la colonie et le substrat, pas après pas.
//!
//! Une [`Simulation`] rassemble les réglages d'un scénario couplé, une [`croissance::Colonie`] partie d'une spore
//! et un [`Substrat`] qui diffuse. À chaque pas, la colonie s'étend en consommant le substrat qu'elle rencontre,
//! puis le substrat diffuse ; les deux mécanismes restent ainsi couplés, et l'état complet — colonie, substrat,
//! nombres cumulés — est déterminé par les réglages, la graine et le nombre de pas joués.
//!
//! # Le scénario couplé
//!
//! Un scénario se lit depuis un texte par [`lire_scenario`]. Chaque ligne non vide et ne commençant pas par `#`
//! porte une directive, ses mots séparés par une espace :
//!
//! - `boite <largeur> <hauteur>` : deux entiers strictement positifs ;
//! - `spore <x> <y>` : deux entiers, coordonnées de la spore dans la boîte ;
//! - `substrat <substrat_initial> <diffusion>` : deux nombres, la quantité initiale dans chaque case et le
//!   coefficient de diffusion ;
//! - `croissance <pas> <intervalle> <angle_ramification> <amplitude_marche>` : les paramètres de
//!   [`croissance::Parametres`] ; `intervalle` accepte le mot `infini` et vaut alors `f64::INFINITY` ;
//! - `couplage <affinite> <cout>` : deux nombres, l'affinité de la colonie pour le substrat et le coût en
//!   substrat d'une unité de biomasse ;
//! - `anastomose <rayon> <delai>` : optionnelle ; son absence désactive l'anastomose ;
//! - `duree <n>` : la durée en pas de temps, un entier non signé.
//!
//! Toutes les directives sauf `anastomose` sont obligatoires. Une ligne inconnue ou mal formée est une
//! [`ErreurLecture::LigneInvalide`], une directive obligatoire absente une [`ErreurLecture::Manquante`].

use crate::anastomose;
use crate::croissance;
use crate::croissance::Segment;
use crate::releves::Releve;
use crate::substrat::{self, Substrat};

/// Les réglages d'une simulation couplée, tels qu'ils se lisent dans un scénario ou se posent à la main.
///
/// Contrat : `largeur` et `hauteur` sont strictement positifs ; `spore` est une case de la boîte, c'est-à-dire
/// `spore.0 < largeur` et `spore.1 < hauteur` ; `substrat_initial`, `diffusion`, `affinite` et `cout` sont finis
/// et positifs ou nuls ; les paramètres de croissance respectent le contrat de
/// [`croissance::Parametres`]. [`Simulation::nouvelle`] refuse les réglages hors contrat, sans paniquer.
#[derive(Clone, Debug, PartialEq)]
pub struct Reglages {
    /// Largeur de la boîte, en cases.
    pub largeur: usize,
    /// Hauteur de la boîte, en cases.
    pub hauteur: usize,
    /// Coordonnées `(x, y)` de la spore dans la boîte.
    pub spore: (usize, usize),
    /// Quantité de substrat déposée dans chaque case au temps 0.
    pub substrat_initial: f64,
    /// Coefficient de diffusion du substrat, en case² par pas de temps.
    pub diffusion: f64,
    /// Les paramètres de la croissance des apex.
    pub croissance: croissance::Parametres,
    /// Affinité de la colonie pour le substrat : plus elle est grande, plus une case donnée soutient l'extension.
    pub affinite: f64,
    /// Coût en substrat d'une unité de biomasse (d'une unité de longueur d'hyphe).
    pub cout: f64,
    /// Réglage de l'anastomose, ou `None` pour la désactiver.
    pub anastomose: Option<anastomose::Reglage>,
}

/// Pourquoi une [`Simulation`] est refusée.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ErreurSimulation {
    /// Un réglage est hors contrat : spore hors de la boîte, quantité négative ou non finie, paramètres de
    /// croissance hors contrat.
    ParametresInvalides,
    /// Le substrat refuse ses paramètres de diffusion (voir [`substrat::ErreurSubstrat`]).
    Substrat(substrat::ErreurSubstrat),
}

/// L'état simulé complet : la colonie, le substrat et les nombres cumulés.
///
/// Tous les champs sont privés ; l'état ne s'observe que par les accesseurs de [`Simulation`], dans un ordre fixe,
/// et se compare « à l'octet » par [`Simulation::ecrire_canonique`].
#[allow(dead_code)]
pub struct Simulation {
    reglages: Reglages,
    colonie: croissance::Colonie,
    substrat: Substrat,
    nb_anastomoses: u64,
    releves: Vec<Releve>,
}

impl Simulation {
    /// Construit l'état au temps 0 : substrat initial uniforme, une spore au centre demandé et aucun segment.
    ///
    /// Refuse avec [`ErreurSimulation::ParametresInvalides`] si la spore sort de la boîte, si `substrat_initial`,
    /// `affinite` ou `cout` est négatif ou non fini, ou si les paramètres de croissance sont hors de leur contrat
    /// (c'est alors une erreur, pas une panique) ; avec [`ErreurSimulation::Substrat`] si les paramètres de
    /// diffusion sont refusés par [`Substrat::nouveau`] — notamment un pas de diffusion instable.
    #[allow(unused_variables)]
    pub fn nouvelle(reglages: Reglages, graine: u64) -> Result<Simulation, ErreurSimulation> {
        todo!()
    }

    /// Avance la simulation d'un pas de temps : la colonie s'étend en consommant le substrat, puis le substrat
    /// diffuse. Un relevé du nouvel état est ajouté à [`Simulation::releves`].
    pub fn avancer(&mut self) {
        todo!()
    }

    /// Avance la simulation de `n` pas : strictement équivalent à `n` appels à [`Simulation::avancer`].
    #[allow(unused_variables)]
    pub fn avancer_n(&mut self, n: u32) {
        todo!()
    }

    /// Nombre de pas de temps joués depuis la création.
    pub fn temps(&self) -> u32 {
        todo!()
    }

    /// Nombre d'apex vivants, c'est-à-dire de pointes de la colonie.
    pub fn pointes(&self) -> usize {
        todo!()
    }

    /// Nombre cumulé de ramifications depuis la création.
    pub fn ramifications(&self) -> u64 {
        todo!()
    }

    /// Nombre cumulé d'anastomoses depuis la création.
    pub fn anastomoses(&self) -> u64 {
        todo!()
    }

    /// Tous les segments laissés par la colonie depuis la création, dans l'ordre fixe où ils ont été déposés.
    pub fn segments(&self) -> &[Segment] {
        todo!()
    }

    /// Biomasse de la colonie : la somme des longueurs de ses segments.
    pub fn biomasse(&self) -> f64 {
        todo!()
    }

    /// Rayon de la colonie : la plus grande distance entre la spore et une extrémité de segment.
    pub fn rayon(&self) -> f64 {
        todo!()
    }

    /// Le substrat courant.
    pub fn substrat(&self) -> &Substrat {
        todo!()
    }

    /// Le relevé du temps courant.
    pub fn releve(&self) -> Releve {
        todo!()
    }

    /// Les relevés, un par pas de temps depuis le temps 0 : il y en a donc `temps() + 1`, et le relevé d'indice
    /// `t` porte `temps == t`.
    pub fn releves(&self) -> &[Releve] {
        todo!()
    }

    /// L'écriture canonique de l'état, pour comparer « à l'octet » et pour l'empreinte.
    ///
    /// Format, dans cet ordre, sans remplissage, tous les entiers en petit-boutiste :
    ///
    /// - octets `0..4` : [`Simulation::temps`] en `u32` ;
    /// - octets `4..12` : le nombre de segments en `u64` ;
    /// - à partir de l'octet 12, les segments dans l'ordre de [`Simulation::segments`], chacun sur
    ///   [`croissance::OCTETS_PAR_SEGMENT`] octets : `apex` (`u32`), `temps` (`u32`), puis `x0`, `y0`, `x1`, `y1`,
    ///   chacun sur 8 octets, écrits par `f64::to_bits` puis en petit-boutiste ;
    /// - ensuite [`Simulation::anastomoses`] en `u64` ;
    /// - puis [`Substrat::ecrire_canonique`], dont le format est décrit dans le module `substrat`.
    ///
    /// La graine et les réglages n'y figurent pas : deux simulations de même état ont la même écriture.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        todo!()
    }

    /// Empreinte de l'état : le SHA-256 de [`Simulation::ecrire_canonique`], en hexadécimal minuscule de 64
    /// caractères.
    pub fn empreinte(&self) -> String {
        todo!()
    }
}

/// Pourquoi la lecture d'un scénario couplé échoue.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ErreurLecture {
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne, à partir de 1.
    LigneInvalide(usize),
    /// Ligne obligatoire absente ; porte son nom, par exemple `"spore"`.
    Manquante(&'static str),
}

/// Lit un scénario couplé et rend ses réglages et sa durée en pas de temps.
///
/// Le format est décrit dans la documentation du module. Les lignes vides et celles dont le premier caractère
/// est `#` sont ignorées. Les directives obligatoires sont `boite`, `spore`, `substrat`, `croissance`,
/// `couplage` et `duree` ; `anastomose` est optionnelle et vaut `None` quand elle est absente. Le mot `infini`
/// est accepté pour l'intervalle de ramification et vaut `f64::INFINITY`.
///
/// Une ligne inconnue ou mal formée rend [`ErreurLecture::LigneInvalide`] avec son numéro, une directive
/// obligatoire absente [`ErreurLecture::Manquante`] avec son nom.
#[allow(unused_variables)]
pub fn lire_scenario(texte: &str) -> Result<(Reglages, u32), ErreurLecture> {
    todo!()
}
