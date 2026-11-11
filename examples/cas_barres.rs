//! Les cas d'école des barres articulées (étape 13, M3) : pour chacun, la valeur calculée par `raideur`, la valeur
//! exacte et le statut de sa source.
//!
//! `cargo run --release --example cas_barres` écrit le tableau ; il ne lit aucun argument et n'écrit aucun fichier. Les
//! tests de l'étape 13 incluent ce fichier comme un module (`#[path]`) et appellent [`cas`].
//!
//! Les valeurs exactes sont écrites à la main, en fractions (et en `√2`, `√3`) ; elles ont été recalculées en fractions
//! exactes, à part du dépôt. Statuts : « lu à la source » (la formule est dans un cours ou un article relu, nommé dans
//! `MODELE.md`) ; « dérivé » (calculé à la main, sans source publiée). Parmi les fonctions de `f64`, seulement
//! `+ - * /`, `abs`, `max` et `sqrt`.

use mycelium_lab::raideur::{Appui, Barre, Charge, Noeud, Solution, resoudre};

pub const LU: &str = "lu à la source";
pub const DERIVE: &str = "dérivé";

/// Une ligne du tableau.
pub struct Cas {
    pub nom: &'static str,
    pub calcule: f64,
    pub exact: f64,
    pub statut: &'static str,
}

impl Cas {
    /// L'erreur relative : l'écart de la valeur calculée à la valeur exacte, divisé par la valeur exacte (l'écart
    /// lui-même quand la valeur exacte est zéro).
    pub fn erreur(&self) -> f64 {
        (self.calcule - self.exact).abs() / if self.exact == 0.0 { 1.0 } else { self.exact.abs() }
    }
}

/// Résout un réseau donné par ses coordonnées, ses barres `(a, b, EA)`, ses appuis à zéro `(nœud, axe)` et ses charges
/// `(nœud, axe, valeur)`.
pub fn reseau(
    points: &[(f64, f64)],
    barres: &[(usize, usize, f64)],
    appuis: &[(usize, usize)],
    charges: &[(usize, usize, f64)],
) -> Solution {
    let noeuds: Vec<Noeud> = points.iter().map(|&(x, y)| Noeud { x, y }).collect();
    let barres: Vec<Barre> = barres.iter().map(|&(a, b, ea)| Barre { a, b, ea }).collect();
    let appuis: Vec<Appui> = appuis.iter().map(|&(noeud, axe)| Appui { noeud, axe, valeur: 0.0 }).collect();
    let charges: Vec<Charge> = charges.iter().map(|&(noeud, axe, valeur)| Charge { noeud, axe, valeur }).collect();
    resoudre(&noeuds, &barres, &appuis, &charges).expect("un cas d'école se résout")
}

/// L'énergie par nœud d'un treillis triangulaire infini (barres de longueur 1, `EA = 1`) sous la déformation moyenne
/// `(ε_xx, ε_yy)` : un nœud libre au centre, ses six voisins déplacés de `(ε_xx x, ε_yy y)`. Chaque barre de l'étoile
/// est partagée par deux nœuds : l'énergie par nœud est la moitié de celle de l'étoile.
pub fn energie_du_treillis(exx: f64, eyy: f64) -> f64 {
    let demi_racine = 0.75_f64.sqrt(); // √3/2
    let voisins =
        [(1.0, 0.0), (0.5, demi_racine), (-0.5, demi_racine), (-1.0, 0.0), (-0.5, -demi_racine), (0.5, -demi_racine)];
    let mut noeuds = vec![Noeud { x: 0.0, y: 0.0 }];
    let (mut barres, mut appuis) = (Vec::new(), Vec::new());
    for (i, &(x, y)) in voisins.iter().enumerate() {
        noeuds.push(Noeud { x, y });
        barres.push(Barre { a: 0, b: i + 1, ea: 1.0 });
        appuis.push(Appui { noeud: i + 1, axe: 0, valeur: exx * x });
        appuis.push(Appui { noeud: i + 1, axe: 1, valeur: eyy * y });
    }
    0.5 * resoudre(&noeuds, &barres, &appuis, &[]).expect("l'étoile se résout").energie
}

/// Le module de Young et le coefficient de Poisson d'un réseau infini dont l'énergie par nœud est `energie(ε_xx, ε_yy)`
/// et l'aire par nœud `aire` : `C11 = 2 W(1, 0)`, `C22 = 2 W(0, 1)`, `C12 = W(1, 1) − W(1, 0) − W(0, 1)` (`W` par unité
/// d'aire), puis `E* = C11 − C12²/C22` et `ν* = C12/C22` (une traction selon `x`, rien selon `y`).
pub fn module_et_poisson(energie: impl Fn(f64, f64) -> f64, aire: f64) -> (f64, f64) {
    let (w10, w01, w11) = (energie(1.0, 0.0) / aire, energie(0.0, 1.0) / aire, energie(1.0, 1.0) / aire);
    let (c11, c22, c12) = (2.0 * w10, 2.0 * w01, w11 - w10 - w01);
    (c11 - c12 * c12 / c22, c12 / c22)
}

/// Tous les cas, dans l'ordre du tableau.
pub fn cas() -> Vec<Cas> {
    let (r2, r3) = (2.0_f64.sqrt(), 3.0_f64.sqrt());
    let mut cas = Vec::new();
    let mut ajoute = |nom, calcule, exact, statut| cas.push(Cas { nom, calcule, exact, statut });

    // Une barre de longueur 2, EA = 3, tirée de 5 : raideur EA/L.
    let s = reseau(&[(0.0, 0.0), (2.0, 0.0)], &[(0, 1, 3.0)], &[(0, 0), (0, 1), (1, 1)], &[(1, 0, 5.0)]);
    ajoute("une barre : déplacement F L/EA", s.deplacements[2], 10.0 / 3.0, LU);
    ajoute("une barre : effort", s.efforts[0], 5.0, LU);
    ajoute("une barre : réaction", s.reactions[0], -5.0, LU);
    ajoute("une barre : énergie", s.energie, 25.0 / 3.0, LU);

    // Deux barres en série (raideurs 2 et 3/2), tirées de 1 : 1/k = 1/k1 + 1/k2.
    let appuis = [(0, 0), (0, 1), (1, 1), (2, 1)];
    let s = reseau(&[(0.0, 0.0), (1.0, 0.0), (3.0, 0.0)], &[(0, 1, 2.0), (1, 2, 3.0)], &appuis, &[(2, 0, 1.0)]);
    ajoute("série : déplacement du milieu", s.deplacements[2], 0.5, DERIVE);
    ajoute("série : déplacement du bout", s.deplacements[4], 7.0 / 6.0, DERIVE);
    ajoute("série : effort de la seconde barre", s.efforts[1], 1.0, DERIVE);

    // Deux barres en parallèle (entre les mêmes nœuds, EA = 2 et 3, longueur 2), tirées de 1 : k = k1 + k2.
    let s = reseau(&[(0.0, 0.0), (2.0, 0.0)], &[(0, 1, 2.0), (0, 1, 3.0)], &[(0, 0), (0, 1), (1, 1)], &[(1, 0, 1.0)]);
    ajoute("parallèle : déplacement", s.deplacements[2], 0.4, DERIVE);
    ajoute("parallèle : effort de la première barre", s.efforts[0], 0.4, DERIVE);
    ajoute("parallèle : effort de la seconde barre", s.efforts[1], 0.6, DERIVE);

    // Deux barres symétriques 3-4-5 : pieds en (−3, 0) et (3, 0), sommet en (0, 4) chargé de (0, −1), EA = 1.
    let pieds = [(0, 0), (0, 1), (1, 0), (1, 1)];
    let s = reseau(&[(-3.0, 0.0), (3.0, 0.0), (0.0, 4.0)], &[(0, 2, 1.0), (1, 2, 1.0)], &pieds, &[(2, 1, -1.0)]);
    ajoute("deux barres : déplacement vertical du sommet", s.deplacements[5], -125.0 / 32.0, DERIVE);
    ajoute("deux barres : déplacement horizontal du sommet", s.deplacements[4], 0.0, DERIVE);
    ajoute("deux barres : effort de la première", s.efforts[0], -5.0 / 8.0, DERIVE);
    ajoute("deux barres : effort de la seconde", s.efforts[1], -5.0 / 8.0, DERIVE);
    ajoute("deux barres : réaction horizontale du premier pied", s.reactions[0], 3.0 / 8.0, DERIVE);
    ajoute("deux barres : réaction verticale du second pied", s.reactions[3], 0.5, DERIVE);

    // Triangle équilatéral de côté 1, EA = 1 : nœud 0 fixe, nœud 1 sur rouleau (tenu en y), sommet 2 tiré de 1 selon x.
    let barres = [(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)];
    let s = reseau(&[(0.0, 0.0), (1.0, 0.0), (0.5, r3 / 2.0)], &barres, &[(0, 0), (0, 1), (1, 1)], &[(2, 0, 1.0)]);
    ajoute("triangle : déplacement du rouleau", s.deplacements[2], 0.5, DERIVE);
    ajoute("triangle : déplacement horizontal du sommet", s.deplacements[4], 9.0 / 4.0, DERIVE);
    ajoute("triangle : déplacement vertical du sommet", s.deplacements[5], -r3 / 12.0, DERIVE);
    ajoute("triangle : effort de la base", s.efforts[0], 0.5, DERIVE);
    ajoute("triangle : effort de la barre du rouleau au sommet", s.efforts[1], -1.0, DERIVE);
    ajoute("triangle : effort de la barre du nœud fixe au sommet", s.efforts[2], 1.0, DERIVE);
    ajoute("triangle : réaction horizontale du nœud fixe", s.reactions[0], -1.0, DERIVE);
    ajoute("triangle : réaction verticale du nœud fixe", s.reactions[1], -r3 / 2.0, DERIVE);
    ajoute("triangle : réaction du rouleau", s.reactions[3], r3 / 2.0, DERIVE);
    ajoute("triangle : énergie", s.energie, 9.0 / 8.0, DERIVE);

    // Carré de côté 1 contreventé par la diagonale 0-2, EA = 1 : nœuds 0 (0, 0), 1 (1, 0), 2 (1, 1), 3 (0, 1) ; nœud 0
    // fixe, nœud 1 tenu en y ; le nœud 3 est tiré de 1 selon x. Les efforts viennent de la statique seule.
    let carre = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let mut barres = vec![(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 0, 1.0), (0, 2, 1.0)];
    let s = reseau(&carre, &barres, &[(0, 0), (0, 1), (1, 1)], &[(3, 0, 1.0)]);
    ajoute("carré : déplacement horizontal du nœud 2", s.deplacements[4], 1.0 + 2.0 * r2, DERIVE);
    ajoute("carré : déplacement vertical du nœud 2", s.deplacements[5], -1.0, DERIVE);
    ajoute("carré : déplacement horizontal du nœud 3", s.deplacements[6], 2.0 + 2.0 * r2, DERIVE);
    ajoute("carré : effort du côté 1-2", s.efforts[1], -1.0, DERIVE);
    ajoute("carré : effort du côté 3-0", s.efforts[3], 0.0, DERIVE);
    ajoute("carré : effort de la diagonale", s.efforts[4], r2, DERIVE);
    ajoute("carré : énergie", s.energie, 1.0 + r2, DERIVE);

    // Le même carré avec ses deux diagonales et les nœuds 0 et 1 fixes : hyperstatique, les efforts dépendent des raideurs.
    barres.push((1, 3, 1.0));
    let s = reseau(&carre, &barres, &[(0, 0), (0, 1), (1, 0), (1, 1)], &[(3, 0, 1.0)]);
    ajoute(
        "carré à deux diagonales : déplacement horizontal du nœud 2",
        s.deplacements[4],
        (5.0 + 24.0 * r2) / 23.0,
        DERIVE,
    );
    ajoute(
        "carré à deux diagonales : déplacement vertical du nœud 2",
        s.deplacements[5],
        (-13.0 + 2.0 * r2) / 23.0,
        DERIVE,
    );
    ajoute(
        "carré à deux diagonales : déplacement horizontal du nœud 3",
        s.deplacements[6],
        (18.0 + 22.0 * r2) / 23.0,
        DERIVE,
    );
    ajoute(
        "carré à deux diagonales : déplacement vertical du nœud 3",
        s.deplacements[7],
        (10.0 + 2.0 * r2) / 23.0,
        DERIVE,
    );
    ajoute("carré à deux diagonales : effort du côté 2-3", s.efforts[2], (-13.0 + 2.0 * r2) / 23.0, DERIVE);
    ajoute("carré à deux diagonales : effort du côté 3-0", s.efforts[3], (10.0 + 2.0 * r2) / 23.0, DERIVE);
    ajoute("carré à deux diagonales : effort de la diagonale 0-2", s.efforts[4], (-4.0 + 13.0 * r2) / 23.0, DERIVE);
    ajoute("carré à deux diagonales : effort de la diagonale 1-3", s.efforts[5], (-4.0 - 10.0 * r2) / 23.0, DERIVE);
    ajoute(
        "carré à deux diagonales : réaction horizontale du nœud 1",
        s.reactions[2],
        (-10.0 - 2.0 * r2) / 23.0,
        DERIVE,
    );
    ajoute("carré à deux diagonales : énergie", s.energie, (9.0 + 11.0 * r2) / 23.0, DERIVE);

    // Le treillis triangulaire infini (barres de longueur l = 1, EA = 1 ; aire par nœud √3/2) : E* = (2/√3) EA/l, ν* = 1/3.
    let (module, poisson) = module_et_poisson(energie_du_treillis, r3 / 2.0);
    ajoute("treillis triangulaire : module E* l/EA", module, 2.0 / r3, LU);
    ajoute("treillis triangulaire : coefficient de Poisson", poisson, 1.0 / 3.0, LU);
    cas
}

/// Le tableau : une ligne par cas, `nom | calculé | exact | erreur | statut`.
pub fn tableau(cas: &[Cas]) -> String {
    let mut texte = String::from("cas | calculé | exact | erreur relative | statut\n");
    for c in cas {
        texte += &format!("{} | {:.15e} | {:.15e} | {:.1e} | {}\n", c.nom, c.calcule, c.exact, c.erreur(), c.statut);
    }
    texte
}

#[allow(dead_code)]
fn main() {
    print!("{}", tableau(&cas()));
}
