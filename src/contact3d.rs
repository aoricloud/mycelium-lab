//! Le contact d'un segment avec un obstacle en volume : le parcours des cases d'Amanatides et Woo, porté en 3D.
//!
//! C'est le pendant de [`crate::substrat::Substrat::segment_libre`] et [`crate::substrat::Substrat::face_de_contact`]
//! pour un [`Substrat3d`] : on suit le segment case par case, de son départ à son arrivée, et on rend la première case
//! bloquée qu'il touche, avec la face franchie pour y entrer.
//!
//! # La règle du plancher
//!
//! Chaque point `(x, y, z)` est dans la case `(⌊x⌋, ⌊y⌋, ⌊z⌋)` (la case `(i, j, k)` est le pavé semi-ouvert
//! `[i, i+1) × [j, j+1) × [k, k+1)`). Le segment fermé touche une case bloquée si l'un de ses points, extrémités
//! comprises, est dans une case bloquée : finir, longer ou passer par l'arête ou le coin bas d'une case bloquée, c'est
//! la toucher. Une case hors du cube n'est jamais bloquée (aucun indice hors du cube n'est lu).
//!
//! # Le parcours
//!
//! Pour chaque axe `a` (dans l'ordre `x`, `y`, `z`), de la coordonnée `a0` du départ à la coordonnée `a1` de
//! l'arrivée : si `a1 > a0`, la case de départ du parcours est `⌊a0⌋`, sa dernière case `⌈a1⌉ − 1` et le sens `+1` ;
//! si `a1 < a0`, la case de départ est `⌈a0⌉ − 1`, la dernière `⌊a1⌋` et le sens `−1` ; si `a1 == a0`, les deux
//! valent `⌊a0⌋` et le sens `0`. Le plafond s'écrit `-(-a).floor()`. Avec `F0` la case du départ (règle du plancher)
//! et `c` la case courante du parcours, dans cet ordre :
//!
//! 1. si `F0` est bloquée : [`Traversee3d::DepartBloque`] ;
//! 2. si `c ≠ F0` et `c` est bloquée : contact dans `c`, entré depuis `F0` ;
//! 3. tant que `c` n'est pas la dernière case sur tous les axes : pour chaque axe `a` qui n'y est pas encore, la
//!    frontière suivante `f_a = c_a + 1` (sens `+1`) ou `c_a` (sens `−1`), et `t_a = (f_a − a0) / (a1 − a0)` (un axe
//!    arrivé, ou de sens `0`, a `t_a = +∞`) ; `t = min t_a`, et `S` l'ensemble des axes de `t_a == t` (égalité exacte
//!    des `f64`) ;
//!    - si `S` a deux axes ou plus (le segment passe exactement par une arête ou un coin), le point de passage est
//!      dans la case `P` (`P_a = f_a` pour `a ∈ S`, `c_a` sinon) ; si `P ≠ c` et `P` est bloquée : contact dans `P`,
//!      entré depuis `c` ;
//!    - puis `c` avance d'un cran de son sens sur chaque axe de `S` ; si la nouvelle case est bloquée : contact dans
//!      celle-ci, entré depuis l'ancienne ;
//! 4. enfin, avec `F1` la case de l'arrivée (règle du plancher) : si `F1 ≠ c` et `F1` est bloquée, contact dans `F1`,
//!    entré depuis `c` ;
//! 5. sinon [`Traversee3d::Libre`].
//!
//! # La face d'entrée
//!
//! On entre dans la case `n` depuis la case `p`. Soit `D` les axes où elles diffèrent, dans l'ordre `x`, `y`, `z`. Si
//! `D` n'a qu'un axe, c'est la face. Sinon (entrée exacte par une arête ou un coin), la face est le premier axe `a` de
//! `D` tel que la case `p + (n_a − p_a) · e_a` (traverser cet axe seul) soit bloquée ; si aucune ne l'est, le premier
//! axe de `D`. Le sens de la face est le signe de `n_a − p_a` : le sens du mouvement du segment le long de cet axe.
//! En 2D, cette règle redonne exactement celle de [`crate::substrat::Substrat::face_de_contact`] (verticale si la case
//! atteinte en traversant `x` seul est bloquée, sinon horizontale si celle de `y` seul l'est, sinon verticale).
//!
//! Le calcul n'emploie que `+`, `-`, `/`, `floor` et des comparaisons : le même résultat sur toutes les machines.

use crate::substrat3d::Substrat3d;

/// Un axe de coordonnées du cube.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axe3d {
    /// L'axe `x` (la largeur).
    X,
    /// L'axe `y` (la hauteur).
    Y,
    /// L'axe `z` (la profondeur).
    Z,
}

impl Axe3d {
    /// Les trois axes, dans l'ordre `x`, `y`, `z` : l'ordre de toutes les égalités du module.
    pub const TOUS: [Axe3d; 3] = [Axe3d::X, Axe3d::Y, Axe3d::Z];

    /// Le rang de l'axe : `0` pour `x`, `1` pour `y`, `2` pour `z`.
    pub fn rang(self) -> usize {
        match self {
            Axe3d::X => 0,
            Axe3d::Y => 1,
            Axe3d::Z => 2,
        }
    }
}

/// Le contact d'un segment avec une case bloquée : la case touchée et la face franchie pour y entrer.
///
/// Contrat : `case` est dans le cube et bloquée ; la face est le plan `axe = entier` franchi, et `sens` (`1` ou `-1`)
/// est le sens du mouvement du segment le long de `axe` à ce franchissement (la normale sortante de l'obstacle sur
/// cette face est donc `−sens · e_axe`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact3d {
    /// La première case bloquée touchée, `(x, y, z)`.
    pub case: (usize, usize, usize),
    /// L'axe normal à la face franchie.
    pub axe: Axe3d,
    /// Le sens du mouvement le long de `axe`, `1` ou `-1`.
    pub sens: i8,
}

/// Ce que rend [`traverser_3d`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Traversee3d {
    /// Aucun point du segment fermé n'est dans une case bloquée.
    Libre,
    /// Le segment touche une case bloquée : la première, dans l'ordre du parcours.
    Contact(Contact3d),
    /// La case du départ est elle-même bloquée.
    DepartBloque,
    /// Une coordonnée n'est pas finie, ou sort du cube fermé `[0, largeur] × [0, hauteur] × [0, profondeur]`.
    Invalide,
}

/// Le parcours du segment fermé de `depart` à `arrivee` dans les cases de `substrat` (règle, ordre et face d'entrée
/// dans la doc du module).
///
/// Contrat : rend [`Traversee3d::Invalide`] si une coordonnée n'est pas finie ou sort du cube fermé, puis
/// [`Traversee3d::DepartBloque`] si la case du départ est bloquée, puis le premier contact du parcours, ou
/// [`Traversee3d::Libre`]. Ne lit aucune case hors du cube et ne panique pas.
pub fn traverser_3d(substrat: &Substrat3d, depart: (f64, f64, f64), arrivee: (f64, f64, f64)) -> Traversee3d {
    let depart = [depart.0, depart.1, depart.2];
    let arrivee = [arrivee.0, arrivee.1, arrivee.2];
    if !dans_le_cube(substrat, depart, arrivee) {
        return Traversee3d::Invalide;
    }
    let f0 = case_du_plancher(depart);
    if case_bloquee(substrat, f0) {
        return Traversee3d::DepartBloque;
    }
    let (mut case, fin, sens) = parcours_par_axe(depart, arrivee);
    if case != f0 && case_bloquee(substrat, case) {
        return Traversee3d::Contact(contact_entre(substrat, f0, case));
    }
    while case != fin {
        // La frontière suivante sur chaque axe pas encore arrivé, et le paramètre `t_a` de son franchissement.
        let mut suivante = [0i64; 3];
        let mut t_de = [f64::INFINITY; 3];
        let mut t = f64::INFINITY;
        for a in 0..3 {
            if case[a] == fin[a] {
                continue;
            }
            let f = if sens[a] > 0 { case[a] + 1 } else { case[a] };
            suivante[a] = f;
            let t_a = (f as f64 - depart[a]) / (arrivee[a] - depart[a]);
            t_de[a] = t_a;
            if t_a < t {
                t = t_a;
            }
        }
        let egal = [t_de[0] == t, t_de[1] == t, t_de[2] == t];
        if egal.iter().filter(|e| **e).count() >= 2 {
            // Le segment passe exactement par une arête ou un coin : la case du point de passage.
            let mut passage = case;
            for a in 0..3 {
                if egal[a] {
                    passage[a] = suivante[a];
                }
            }
            if passage != case && case_bloquee(substrat, passage) {
                return Traversee3d::Contact(contact_entre(substrat, case, passage));
            }
        }
        // `case` avance d'un cran de son sens sur chaque axe de l'ensemble des plus proches.
        let ancienne = case;
        for a in 0..3 {
            if egal[a] {
                case[a] += sens[a];
            }
        }
        if case_bloquee(substrat, case) {
            return Traversee3d::Contact(contact_entre(substrat, ancienne, case));
        }
    }
    let f1 = case_du_plancher(arrivee);
    if f1 != case && case_bloquee(substrat, f1) {
        return Traversee3d::Contact(contact_entre(substrat, case, f1));
    }
    Traversee3d::Libre
}

/// Vrai si les six coordonnées sont finies et dans le cube fermé : une coordonnée `NaN` ou infinie échoue à la
/// comparaison, comme une coordonnée hors des bornes.
fn dans_le_cube(substrat: &Substrat3d, depart: [f64; 3], arrivee: [f64; 3]) -> bool {
    let bornes = [substrat.largeur() as f64, substrat.hauteur() as f64, substrat.profondeur() as f64];
    (0..3).all(|a| (0.0..=bornes[a]).contains(&depart[a]) && (0.0..=bornes[a]).contains(&arrivee[a]))
}

/// La case d'un point `(x, y, z)` par la règle du plancher, en indices entiers.
fn case_du_plancher(point: [f64; 3]) -> [i64; 3] {
    [point[0].floor() as i64, point[1].floor() as i64, point[2].floor() as i64]
}

/// Vrai si la case `c` est bloquée ; faux dès qu'un indice sort du cube (rien n'est jamais lu hors du cube).
fn case_bloquee(substrat: &Substrat3d, c: [i64; 3]) -> bool {
    if c[0] < 0 || c[1] < 0 || c[2] < 0 {
        return false;
    }
    let (x, y, z) = (c[0] as usize, c[1] as usize, c[2] as usize);
    if x >= substrat.largeur() || y >= substrat.hauteur() || z >= substrat.profondeur() {
        return false;
    }
    substrat.est_bloquee(x, y, z)
}

/// Les cases de départ et d'arrivée du parcours et le sens, axe par axe : comme [`axe`], en tableau.
fn parcours_par_axe(depart: [f64; 3], arrivee: [f64; 3]) -> ([i64; 3], [i64; 3], [i64; 3]) {
    let mut debut = [0i64; 3];
    let mut fin = [0i64; 3];
    let mut sens = [0i64; 3];
    for a in 0..3 {
        let (d, f, s) = axe(depart[a], arrivee[a]);
        debut[a] = d;
        fin[a] = f;
        sens[a] = s;
    }
    (debut, fin, sens)
}

/// La case de départ du parcours, sa dernière case et le sens sur un axe, de `a0` à `a1` : `⌊a0⌋` et `⌈a1⌉ − 1`
/// en montant, `⌈a0⌉ − 1` et `⌊a1⌋` en descendant, `⌊a0⌋` des deux côtés et le sens `0` si `a1 == a0`. Le plafond
/// s'écrit `-(-a).floor()`.
fn axe(a0: f64, a1: f64) -> (i64, i64, i64) {
    if a1 > a0 {
        (a0.floor() as i64, (-(-a1).floor()) as i64 - 1, 1)
    } else if a1 < a0 {
        ((-(-a0).floor()) as i64 - 1, a1.floor() as i64, -1)
    } else {
        let i = a0.floor() as i64;
        (i, i, 0)
    }
}

/// Le contact dans la case `n`, entrée depuis la case `p` : la case `n` et la face d'entrée de [`face_entree`].
fn contact_entre(substrat: &Substrat3d, p: [i64; 3], n: [i64; 3]) -> Contact3d {
    let (axe, sens) = face_entree(substrat, p, n);
    Contact3d { case: (n[0] as usize, n[1] as usize, n[2] as usize), axe, sens }
}

/// La face franchie pour entrer dans la case `n` depuis la case `p` : le seul axe qui diffère, ou, pour une entrée
/// par une arête ou un coin, le premier axe `a` (dans l'ordre `x`, `y`, `z`) dont la case `p + (n_a − p_a) · e_a`
/// est bloquée, à défaut le premier axe qui diffère. Le sens est le signe de `n_a − p_a`.
fn face_entree(substrat: &Substrat3d, p: [i64; 3], n: [i64; 3]) -> (Axe3d, i8) {
    let differents = [n[0] != p[0], n[1] != p[1], n[2] != p[2]];
    let premier = (0..3).find(|&a| differents[a]).unwrap_or(0);
    let rang = if differents.iter().filter(|d| **d).count() == 1 {
        premier
    } else {
        let mut choisi = premier;
        let mut trouve = false;
        for a in 0..3 {
            if !differents[a] || trouve {
                continue;
            }
            let mut voisine = p;
            voisine[a] = n[a];
            if case_bloquee(substrat, voisine) {
                choisi = a;
                trouve = true;
            }
        }
        choisi
    };
    let sens = if n[rang] > p[rang] { 1 } else { -1 };
    (Axe3d::TOUS[rang], sens)
}
