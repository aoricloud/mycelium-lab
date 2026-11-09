//! Pièce : mesurer ce que la colonie remplit (étape M0). **Version « avant la vague de code » : les signatures, leurs
//! traits et leur contrat, des corps qui ne calculent rien.** Le contrat complet est dans la spécification de M0.
//!
//! La colonie laisse des [`Segment`]s. Ce module en tire, **sans rien changer à la simulation**, cinq mesures sur la
//! « pièce » qu'elle moule dans la boîte : une carte de densité d'hyphe, les cases occupées (à un seuil de densité), le
//! bilan de nourriture, le plus grand morceau connecté et la traversée d'un bord à l'autre.
//!
//! # La règle du plancher
//!
//! Le point `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)` : la case `(i, j)` est le carré semi-ouvert `[i, i+1) × [j, j+1)`.
//! L'axe `y` croît avec l'indice de ligne : la ligne 0 est le **haut** de la boîte, la dernière ligne en est le **bas**.
//!
//! # Les cartes et les grilles de cases
//!
//! Une grille de `largeur × hauteur` valeurs se range ligne par ligne, `y` lent et `x` rapide : la case `(x, y)` est à
//! l'indice `y * largeur + x`. C'est l'ordre de [`Carte::valeurs`] et celui des tranches `occupees: &[bool]`.
//!
//! # Le déterminisme
//!
//! Parmi les fonctions de `f64`, seules `+ - * /`, `sqrt`, `abs`, `floor`, `min`, `max`, `clamp` et les conversions `as`
//! sont permises ; ni `HashMap` ni `HashSet` parcourus ; aucune dépendance (`AGENTS.md`, règle 2).
//!
//! # Ce que ces corps rendent
//!
//! Chaque corps rend une valeur **fausse exprès** (et ne panique jamais) : un test qui passe sur ce stub ne vérifie rien.

use crate::croissance::Segment;
use crate::simulation::Simulation;

/// Une carte de valeurs sur la boîte : `largeur × hauteur` nombres, rangés ligne par ligne (`y` lent, `x` rapide).
#[derive(Clone, Debug, PartialEq)]
pub struct Carte {
    largeur: usize,
    hauteur: usize,
    valeurs: Vec<f64>,
}

impl Carte {
    /// Une carte dont toutes les cases valent `0.0`. Contrat : panique si `largeur` ou `hauteur` est nulle.
    pub fn nouvelle(_largeur: usize, _hauteur: usize) -> Carte {
        Carte { largeur: 0, hauteur: 0, valeurs: Vec::new() }
    }

    /// Largeur de la carte, en cases.
    pub fn largeur(&self) -> usize {
        self.largeur
    }

    /// Hauteur de la carte, en cases.
    pub fn hauteur(&self) -> usize {
        self.hauteur
    }

    /// La valeur de la case `(x, y)`. Contrat : panique si `x >= largeur()` ou `y >= hauteur()`, chaque coordonnée
    /// contrôlée.
    pub fn valeur(&self, _x: usize, _y: usize) -> f64 {
        f64::NAN
    }

    /// Toutes les valeurs, ligne par ligne (`y` lent, `x` rapide).
    pub fn valeurs(&self) -> &[f64] {
        &self.valeurs
    }

    /// La somme des valeurs, additionnées une à une dans l'ordre de [`Carte::valeurs`].
    pub fn somme(&self) -> f64 {
        f64::NAN
    }

    /// Les cases dont la valeur est **strictement** supérieure à `seuil`, une par case, dans l'ordre de
    /// [`Carte::valeurs`]. Contrat : `seuil` fini et positif ou nul (`-0.0` compte comme `0.0`) ; sinon panique.
    pub fn occupees(&self, _seuil: f64) -> Vec<bool> {
        Vec::new()
    }
}

/// La carte de densité d'hyphe : pour chaque case, la somme des longueurs des morceaux de segment qui y passent (coupe
/// aux lignes de grille, case du milieu de chaque morceau, dernière colonne ou ligne pour un milieu posé sur le bord
/// haut). Contrat : coordonnées finies dans `[0, largeur] × [0, hauteur]`, dimensions non nulles ; sinon panique.
pub fn carte_de_densite(_largeur: usize, _hauteur: usize, _segments: &[Segment]) -> Carte {
    Carte { largeur: 0, hauteur: 0, valeurs: Vec::new() }
}

/// Un bord de la boîte : `Gauche` est la colonne 0, `Droite` la dernière colonne, `Haut` la ligne 0 et `Bas` la dernière
/// ligne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bord {
    /// La colonne `x = 0`.
    Gauche,
    /// La colonne `x = largeur − 1`.
    Droite,
    /// La ligne `y = 0`.
    Haut,
    /// La ligne `y = hauteur − 1`.
    Bas,
}

/// Les tailles des morceaux connectés (huit voisins) de cases occupées, de la plus grande à la plus petite. Panique si
/// `occupees` n'a pas `largeur × hauteur` cases ; une boîte sans case n'a aucun morceau.
pub fn morceaux(_largeur: usize, _hauteur: usize, _occupees: &[bool]) -> Vec<usize> {
    vec![usize::MAX]
}

/// La taille du plus grand morceau connecté (huit voisins), `0` s'il n'y a aucune case occupée.
pub fn plus_grand_morceau(_largeur: usize, _hauteur: usize, _occupees: &[bool]) -> usize {
    usize::MAX
}

/// Le nombre de cases du plus court chemin de cases occupées (huit voisins) du bord `de` au bord `vers`, `None` s'il
/// n'y en a pas. Panique si `de == vers` ou si `occupees` n'a pas `largeur × hauteur` cases.
pub fn traversee(_largeur: usize, _hauteur: usize, _occupees: &[bool], _de: Bord, _vers: Bord) -> Option<usize> {
    Some(0)
}

/// Les mesures de la pièce à un instant de la simulation, à un seuil de densité. Voir [`mesurer_au_seuil`].
#[derive(Clone, Debug, PartialEq)]
pub struct MesuresPiece {
    /// Les cases non bloquées.
    pub cases_libres: usize,
    /// Les cases de densité strictement supérieure au seuil.
    pub cases_occupees: usize,
    /// `cases_occupees / cases_libres` (`0.0` s'il n'y a aucune case libre).
    pub remplissage: f64,
    /// La somme de la carte divisée par `cases_libres` (`0.0` s'il n'y a aucune case libre).
    pub densite_moyenne: f64,
    /// La plus grande densité de la carte.
    pub densite_maximale: f64,
    /// La biomasse de la simulation, [`Simulation::biomasse`].
    pub biomasse: f64,
    /// La nourriture au temps 0 : `releves()[0].substrat`.
    pub nourriture_initiale: f64,
    /// La nourriture qui reste : la masse totale du substrat.
    pub nourriture_restante: f64,
    /// La nourriture que la colonie a consommée : `cout × biomasse`.
    pub nourriture_consommee: f64,
    /// `|(restante + consommée) − initiale|`.
    pub ecart_du_bilan: f64,
    /// La taille, en cases, du plus grand morceau connecté de cases occupées au seuil.
    pub plus_grand_morceau: usize,
    /// Le nombre de morceaux connectés de cases occupées au seuil.
    pub nombre_de_morceaux: usize,
    /// Le nombre de cases du plus court chemin occupé de la colonne de gauche à celle de droite, s'il existe.
    pub traversee_gauche_droite: Option<usize>,
    /// Le nombre de cases du plus court chemin occupé de la ligne du haut à celle du bas, s'il existe.
    pub traversee_haut_bas: Option<usize>,
}

/// Les mesures au seuil nul : exactement `mesurer_au_seuil(simulation, 0.0)`.
pub fn mesurer(_simulation: &Simulation) -> MesuresPiece {
    fausses_mesures()
}

/// Les mesures de la pièce, les cases occupées étant celles de densité strictement supérieure à `seuil`. Lecture seule.
/// Contrat : `seuil` fini et positif ou nul ; sinon panique.
pub fn mesurer_au_seuil(_simulation: &Simulation, _seuil: f64) -> MesuresPiece {
    fausses_mesures()
}

/// Des mesures fausses exprès.
fn fausses_mesures() -> MesuresPiece {
    MesuresPiece {
        cases_libres: usize::MAX,
        cases_occupees: usize::MAX,
        remplissage: f64::NAN,
        densite_moyenne: f64::NAN,
        densite_maximale: f64::NAN,
        biomasse: f64::NAN,
        nourriture_initiale: f64::NAN,
        nourriture_restante: f64::NAN,
        nourriture_consommee: f64::NAN,
        ecart_du_bilan: f64::NAN,
        plus_grand_morceau: usize::MAX,
        nombre_de_morceaux: usize::MAX,
        traversee_gauche_droite: Some(0),
        traversee_haut_bas: Some(0),
    }
}
