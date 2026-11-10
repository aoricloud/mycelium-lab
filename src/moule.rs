//! Moule : donner une forme à l'espace libre, et mesurer comment la colonie le remplit (étape M1). **Version « avant la
//! vague de code » : les signatures, leurs traits et leur contrat, des corps qui ne calculent rien.** Le contrat complet
//! est dans la spécification de M1.
//!
//! Un **moule** est un ensemble de cases libres dans la boîte, la *cavité* ; tout le reste est un obstacle. Le noyau de
//! la simulation sait déjà bloquer des rectangles (`obstacle x0 y0 x1 y1` dans un scénario, [`Obstacle`]) : ce module est
//! un **préprocesseur de scénario**. Une ligne `moule …` est remplacée, en texte, par les lignes `obstacle …` qui
//! bloquent le complément de la cavité ([`developper`]) ; le noyau, le binaire `mycelium` et la lecture des scénarios ne
//! sont pas touchés, ils lisent le texte développé.
//!
//! # Les conventions
//!
//! Comme partout dans le dépôt, la case `(x, y)` est à l'indice `y * largeur + x` ; `y` croît vers le bas (la ligne 0 est
//! le **haut** de la boîte) ; le point `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)` (règle du plancher).
//!
//! # Le déterminisme
//!
//! Les formes se calculent en entiers. Parmi les fonctions de `f64`, le module n'emploie que `+ - * /`, `floor` et les
//! conversions `as` ; ni `HashMap` ni `HashSet` ; aucune dépendance (`AGENTS.md`, règle 2).
//!
//! # Ce que ces corps rendent
//!
//! Chaque corps rend une valeur **fausse exprès** (et ne panique jamais) : un test qui passe sur ce stub ne vérifie rien.

use crate::simulation::{Obstacle, Simulation};

/// La forme de la cavité d'un moule. Toutes les coordonnées sont des numéros de cases, bornes incluses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forme {
    /// La cavité est le rectangle `x0..=x1` × `y0..=y1`.
    Brique { x0: usize, y0: usize, x1: usize, y1: usize },
    /// Un L : dans le rectangle englobant `x0..=x1` × `y0..=y1`, la barre de gauche (les `epaisseur` premières colonnes, sur
    /// toute la hauteur) et la barre du bas (les `epaisseur` dernières lignes, sur toute la largeur). Quand `epaisseur`
    /// égale le plus petit côté du rectangle, le L est le rectangle plein.
    L { x0: usize, y0: usize, x1: usize, y1: usize, epaisseur: usize },
    /// Les cases `(x, y)` telles que `(x − cx)² + (y − cy)² ≤ rayon²`, en entiers. `rayon = 0` donne la case seule.
    Disque { cx: usize, cy: usize, rayon: usize },
    /// Les cases telles que `rayon_interieur² < (x − cx)² + (y − cy)² ≤ rayon_exterieur²`, en entiers ; il faut
    /// `rayon_interieur < rayon_exterieur`.
    Anneau { cx: usize, cy: usize, rayon_exterieur: usize, rayon_interieur: usize },
    /// Un labyrinthe parfait de `colonnes × rangees` chambres carrées de `couloir` cases de côté, tiré avec `graine`. Il
    /// faut `colonnes ≥ 1`, `rangees ≥ 1` et `couloir ≥ 1`.
    Labyrinthe { colonnes: usize, rangees: usize, couloir: usize, graine: u64 },
}

/// Pourquoi un moule est refusé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurMoule {
    /// Une ligne `moule` sans ligne `boite` avant elle.
    BoiteAbsente,
    /// Deux lignes `moule`.
    MoulesEnDouble,
    /// Ligne `moule` inconnue ou mal formée ; porte le numéro de ligne, à partir de 1.
    LigneInvalide(usize),
    /// Paramètres hors contrat : voir [`libres`].
    FormeInvalide,
    /// Une case de la cavité sort de la boîte.
    HorsDeLaBoite,
}

/// Les cases libres de la boîte pour cette forme : `largeur * hauteur` booléens, ligne par ligne, vrai = case de la cavité.
/// Contrat : `FormeInvalide` si les paramètres sont hors contrat, `HorsDeLaBoite` si une case de la cavité sort de la boîte
/// (`FormeInvalide` l'emporte) ; une boîte de dimension nulle rend `HorsDeLaBoite` ; aucune panique.
pub fn libres(_largeur: usize, _hauteur: usize, _forme: &Forme) -> Result<Vec<bool>, ErreurMoule> {
    Ok(Vec::new())
}

/// Les obstacles du moule : le complément de la cavité dans la boîte, en rectangles disjoints (la décomposition de la
/// spécification, triée par `(y0, x0)`). Mêmes erreurs que [`libres`].
pub fn obstacles(_largeur: usize, _hauteur: usize, _forme: &Forme) -> Result<Vec<Obstacle>, ErreurMoule> {
    Ok(Vec::new())
}

/// Remplace la ligne `moule …` du texte d'un scénario par les lignes `obstacle x0 y0 x1 y1` de ses [`obstacles`] ; sans
/// ligne `moule`, rend le texte tel quel, octet pour octet. Les erreurs : `LigneInvalide(numéro)`, `MoulesEnDouble`,
/// `BoiteAbsente`, puis celles de [`obstacles`], dans cet ordre.
pub fn developper(_texte: &str) -> Result<String, ErreurMoule> {
    Ok(String::new())
}

/// Les cases de la chambre `(colonne, rangee)` d'un labyrinthe de couloir `couloir` : le rectangle `(x0, y0, x1, y1)`, bornes
/// incluses, avec `x0 = colonne p + 1`, `x1 = x0 + couloir − 1`, `y0 = rangee p + 1`, `y1 = y0 + couloir − 1` et `p = couloir + 1`.
pub fn chambre(_couloir: usize, _colonne: usize, _rangee: usize) -> (usize, usize, usize, usize) {
    (usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

/// La case centre de la chambre `(colonne, rangee)` d'un labyrinthe de couloir `couloir` :
/// `(colonne p + 1 + couloir / 2, rangee p + 1 + couloir / 2)` avec `p = couloir + 1` et `/` la division entière.
pub fn centre_de_la_chambre(_couloir: usize, _colonne: usize, _rangee: usize) -> (usize, usize) {
    (usize::MAX, usize::MAX)
}

/// Le plus court chemin de cases libres (4 voisins : haut, droite, bas, gauche), en nombre de pas ; `Some(0)` si
/// `depart == arrivee` ; `None` si une des cases est bloquée ou hors de la grille, ou si l'arrivée est inatteignable.
/// Panique si `libres` n'a pas `largeur × hauteur` cases.
pub fn plus_court_chemin(
    _largeur: usize,
    _hauteur: usize,
    _libres: &[bool],
    _depart: (usize, usize),
    _arrivee: (usize, usize),
) -> Option<usize> {
    Some(usize::MAX)
}

/// Même réponse que [`plus_court_chemin`], trouvée par A\* (heuristique : la distance de Manhattan ; départage
/// déterministe). Panique si `libres` n'a pas `largeur × hauteur` cases.
pub fn chemin_a_etoile(
    _largeur: usize,
    _hauteur: usize,
    _libres: &[bool],
    _depart: (usize, usize),
    _arrivee: (usize, usize),
) -> Option<usize> {
    Some(usize::MAX)
}

/// La longueur (en pas) du premier chemin simple que trouve une recherche en profondeur **itérative**, voisines essayées
/// dans l'ordre haut, droite, bas, gauche ; `None` dans les mêmes cas que [`plus_court_chemin`]. Panique si `libres` n'a
/// pas `largeur × hauteur` cases.
pub fn chemin_en_profondeur(
    _largeur: usize,
    _hauteur: usize,
    _libres: &[bool],
    _depart: (usize, usize),
    _arrivee: (usize, usize),
) -> Option<usize> {
    Some(0)
}

/// Un point de la courbe de remplissage : à `pas`, `cases_occupees` des `cases_libres` du moule sont occupées.
#[derive(Clone, Debug, PartialEq)]
pub struct PointDeCourbe {
    /// Le temps de la simulation (son nombre de pas joués) à ce point.
    pub pas: u32,
    /// Les cases occupées au seuil nul : celles où passe de l'hyphe, si peu que ce soit.
    pub cases_occupees: usize,
    /// Les cases non bloquées : la taille de la cavité.
    pub cases_libres: usize,
    /// `cases_occupees / cases_libres` (`0.0` s'il n'y a aucune case libre).
    pub remplissage: f64,
}

/// La part du moule remplie, pas après pas : un point au temps courant de la simulation, puis un par pas jusqu'au pas
/// `jusqu_au_pas` inclus (si `jusqu_au_pas ≤ temps()`, un seul point, sans avancer). Chaque point est tiré de
/// `piece::mesurer` (seuil nul).
pub fn courbe_de_remplissage(_simulation: &mut Simulation, _jusqu_au_pas: u32) -> Vec<PointDeCourbe> {
    Vec::new()
}

/// Le premier pas où la part remplie atteint `part` (`remplissage ≥ part`, sans tolérance) ; `None` si la courbe n'y
/// arrive pas. Contrat : panique si `part` n'est pas dans `[0, 1]`.
pub fn pas_pour_remplir(_courbe: &[PointDeCourbe], _part: f64) -> Option<u32> {
    None
}

/// Le temps de sortie : le premier pas, au plus `pas_max`, où la colonie atteint la `cible`, un rectangle de cases
/// `(x0, y0, x1, y1)`, bornes incluses (la spore dans la cible : `Some(0)` ; sinon un segment dont la fin est dans la
/// cible ; la simulation n'avance qu'autant qu'il le faut, jamais au-delà de `pas_max`). Contrat : `x0 ≤ x1` et `y0 ≤ y1`,
/// sinon panique ; une cible qui sort de la boîte rend `None` sans avancer.
pub fn temps_de_sortie(
    _simulation: &mut Simulation,
    _cible: (usize, usize, usize, usize),
    _pas_max: u32,
) -> Option<u32> {
    Some(u32::MAX)
}
