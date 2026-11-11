//! Les cas d'école des poutres à nœuds rigides (étape 14, M3) : pour chacun, la valeur calculée par `poutres`, la
//! valeur exacte et le statut de sa source.
//!
//! `cargo run --release --example cas_poutres` écrit le tableau ; il ne lit aucun argument et n'écrit aucun fichier. Les
//! tests de l'étape 14 incluent ce fichier comme un module (`#[path]`) et appellent [`cas`]. Les valeurs exactes sont
//! écrites à la main, en fractions ; elles ont été recalculées en fractions exactes, à part du dépôt. Les statuts sont
//! ceux de `cas_barres`. Parmi les fonctions de `f64`, seulement `+ - * /`, `abs`, `max` et `sqrt`.

#[path = "cas_barres.rs"]
pub mod barres;

use barres::{Cas, DERIVE, LU, module_et_poisson, tableau};
use mycelium_lab::poutres::{Poutre, SolutionPoutres, resoudre};
use mycelium_lab::raideur::{Appui, Charge, Noeud};

/// Résout un réseau donné par ses coordonnées, ses poutres `(a, b)` de raideurs `ea` et `ei`, ses appuis à zéro
/// `(nœud, axe)` et ses charges `(nœud, axe, valeur)` ; l'axe 2 est la rotation (ou un moment).
pub fn reseau(
    points: &[(f64, f64)],
    poutres: &[(usize, usize)],
    (ea, ei): (f64, f64),
    appuis: &[(usize, usize)],
    charges: &[(usize, usize, f64)],
) -> SolutionPoutres {
    let noeuds: Vec<Noeud> = points.iter().map(|&(x, y)| Noeud { x, y }).collect();
    let poutres: Vec<Poutre> = poutres.iter().map(|&(a, b)| Poutre { a, b, ea, ei }).collect();
    let appuis: Vec<Appui> = appuis.iter().map(|&(noeud, axe)| Appui { noeud, axe, valeur: 0.0 }).collect();
    let charges: Vec<Charge> = charges.iter().map(|&(noeud, axe, valeur)| Charge { noeud, axe, valeur }).collect();
    resoudre(&noeuds, &poutres, &appuis, &charges).expect("un cas d'école se résout")
}

/// L'énergie par cellule d'un nid d'abeille infini (des hexagones réguliers ; une paroi de longueur `l = 1`,
/// d'épaisseur `t = rapport`, de module `Es = 1` : `EA = t × axial`, `EI = t³/12` ; `axial = 1` pour une vraie paroi)
/// sous la déformation moyenne `(ε_xx, ε_yy)`. La cellule a deux nœuds et trois parois : c'est l'étoile d'un nœud libre
/// et de ses trois voisins, déplacés de `(ε_xx x, ε_yy y)` sans tourner. C'est exact : les trois voisins sont les
/// images l'un de l'autre par les symétries du réseau, et chaque paroi est un axe de symétrie (aucun nœud ne tourne).
pub fn energie_du_nid(rapport: f64, axial: f64, exx: f64, eyy: f64) -> f64 {
    let demi_racine = 0.75_f64.sqrt(); // √3/2
    let voisins = [(0.0, -1.0), (demi_racine, 0.5), (-demi_racine, 0.5)];
    let mut noeuds = vec![Noeud { x: 0.0, y: 0.0 }];
    let (mut poutres, mut appuis) = (Vec::new(), Vec::new());
    for (i, &(x, y)) in voisins.iter().enumerate() {
        noeuds.push(Noeud { x, y });
        poutres.push(Poutre { a: 0, b: i + 1, ea: rapport * axial, ei: rapport * rapport * rapport / 12.0 });
        appuis.extend([exx * x, eyy * y, 0.0].iter().enumerate().map(|(axe, &valeur)| Appui {
            noeud: i + 1,
            axe,
            valeur,
        }));
    }
    resoudre(&noeuds, &poutres, &appuis, &[]).expect("l'étoile se résout").energie
}

/// Le module de Young (divisé par `Es`) et le coefficient de Poisson du nid d'abeille, pour une traction selon `x`.
/// L'aire de la cellule est `(3√3/2) l²`.
pub fn nid(rapport: f64, axial: f64) -> (f64, f64) {
    module_et_poisson(|exx, eyy| energie_du_nid(rapport, axial, exx, eyy), 1.5 * 3.0_f64.sqrt())
}

/// Le module du nid d'abeille en flexion seule : `(4/√3) (t/l)³ Es` (MIT 3.054, leçon 4, pages 11 et 15).
pub fn module_en_flexion(rapport: f64) -> f64 {
    4.0 / 3.0_f64.sqrt() * rapport * rapport * rapport
}

/// Tous les cas, dans l'ordre du tableau.
pub fn cas() -> Vec<Cas> {
    let mut cas = Vec::new();
    let mut ajoute = |nom, calcule, exact, statut| cas.push(Cas { nom, calcule, exact, statut });
    let raideurs = (5.0, 3.0); // EA = 5, EI = 3 partout, sauf mention
    let encastre = [(0, 0), (0, 1), (0, 2)];

    // Console de longueur 2, encastrée au nœud 0, chargée de P = 7 en travers au bout : P L³/(3 EI), P L²/(2 EI).
    let console = [(0.0, 0.0), (2.0, 0.0)];
    let s = reseau(&console, &[(0, 1)], raideurs, &encastre, &[(1, 1, 7.0)]);
    ajoute("console : flèche P L³/(3 EI)", s.deplacements[4], 56.0 / 9.0, LU);
    ajoute("console : rotation du bout P L²/(2 EI)", s.deplacements[5], 14.0 / 3.0, LU);
    ajoute("console : réaction de l'encastrement", s.reactions[1], -7.0, DERIVE);
    ajoute("console : moment d'encastrement P L", s.reactions[2], -14.0, DERIVE);
    ajoute("console : effort tranchant au bout", s.efforts[0][4], 7.0, DERIVE);
    ajoute("console : moment au pied de la poutre", s.efforts[0][2], -14.0, DERIVE);
    ajoute("console : moment au bout", s.efforts[0][5], 0.0, DERIVE);
    ajoute("console : énergie", s.energie, 196.0 / 9.0, DERIVE);
    let s = reseau(&console, &[(0, 1)], raideurs, &encastre, &[(1, 0, 7.0)]);
    ajoute("console étirée : allongement F L/EA", s.deplacements[3], 14.0 / 5.0, LU);
    ajoute("console étirée : effort normal", s.efforts[0][3], 7.0, LU);
    let s = reseau(&console, &[(0, 1)], raideurs, &encastre, &[(1, 2, 7.0)]);
    ajoute("console, moment au bout : flèche M L²/(2 EI)", s.deplacements[4], 14.0 / 3.0, DERIVE);
    ajoute("console, moment au bout : rotation M L/EI", s.deplacements[5], 14.0 / 3.0, DERIVE);
    // Une charge répartie q = 7 : ses charges nodales cohérentes au bout sont q L/2 et −q L²/12.
    let s = reseau(&console, &[(0, 1)], raideurs, &encastre, &[(1, 1, 7.0), (1, 2, -7.0 / 3.0)]);
    ajoute("console, charge répartie : flèche q L⁴/(8 EI)", s.deplacements[4], 14.0 / 3.0, LU);
    ajoute("console, charge répartie : rotation q L³/(6 EI)", s.deplacements[5], 28.0 / 9.0, LU);
    // La même console dressée vers le haut, poussée de 7 vers la gauche au bout : elle tourne dans le sens anti-horaire.
    let s = reseau(&[(0.0, 0.0), (0.0, 2.0)], &[(0, 1)], raideurs, &encastre, &[(1, 0, -7.0)]);
    ajoute("console verticale : déplacement du bout", s.deplacements[3], -56.0 / 9.0, DERIVE);
    ajoute("console verticale : rotation (anti-horaire)", s.deplacements[5], 14.0 / 3.0, DERIVE);
    ajoute("console verticale : moment d'encastrement", s.reactions[2], -14.0, DERIVE);

    // Poutre de longueur 4 sur deux appuis simples, P = 7 vers le bas au milieu (un nœud au milieu) : P L³/(48 EI).
    let poutre = [(0.0, 0.0), (2.0, 0.0), (4.0, 0.0)];
    let s = reseau(&poutre, &[(0, 1), (1, 2)], raideurs, &[(0, 0), (0, 1), (2, 1)], &[(1, 1, -7.0)]);
    ajoute("poutre appuyée : flèche P L³/(48 EI)", s.deplacements[4], -28.0 / 9.0, LU);
    ajoute("poutre appuyée : rotation de l'appui P L²/(16 EI)", s.deplacements[2], -7.0 / 3.0, DERIVE);
    ajoute("poutre appuyée : réaction", s.reactions[7], 3.5, DERIVE);
    ajoute("poutre appuyée : moment au milieu P L/4", s.efforts[0][5], 7.0, DERIVE);
    // La même, encastrée aux deux bouts : moment d'encastrement P L/8, flèche P L³/(192 EI).
    let deux_bouts = [(0, 0), (0, 1), (0, 2), (2, 0), (2, 1), (2, 2)];
    let s = reseau(&poutre, &[(0, 1), (1, 2)], raideurs, &deux_bouts, &[(1, 1, -7.0)]);
    ajoute("poutre encastrée aux deux bouts : flèche P L³/(192 EI)", s.deplacements[4], -7.0 / 9.0, DERIVE);
    ajoute("poutre encastrée aux deux bouts : moment d'encastrement P L/8", s.reactions[2], 3.5, LU);
    ajoute("poutre encastrée aux deux bouts : moment de l'autre encastrement", s.reactions[8], -3.5, LU);
    ajoute("poutre encastrée aux deux bouts : énergie", s.energie, 49.0 / 18.0, DERIVE);
    // Encastrée aux deux bouts, la charge au quart (a = 1, b = 3) : moments P a b²/L² et P a² b/L², flèche P a³ b³/(3 EI L³).
    let s = reseau(&[(0.0, 0.0), (1.0, 0.0), (4.0, 0.0)], &[(0, 1), (1, 2)], raideurs, &deux_bouts, &[(1, 1, -7.0)]);
    ajoute("charge au quart : flèche P a³ b³/(3 EI L³)", s.deplacements[4], -21.0 / 64.0, DERIVE);
    ajoute("charge au quart : rotation sous la charge", s.deplacements[5], -21.0 / 64.0, DERIVE);
    ajoute("charge au quart : moment d'encastrement P a b²/L²", s.reactions[2], 63.0 / 16.0, LU);
    ajoute("charge au quart : moment de l'autre encastrement P a² b/L²", s.reactions[8], -21.0 / 16.0, LU);
    ajoute("charge au quart : réaction du bout proche", s.reactions[1], 189.0 / 32.0, DERIVE);
    // Une poutre de longueur 3 (EI = 1) tenue en translation aux deux bouts, un moment 2 au second : ce n'est pas un mécanisme.
    let s = reseau(&[(0.0, 0.0), (3.0, 0.0)], &[(0, 1)], (1.0, 1.0), &[(0, 0), (0, 1), (1, 1)], &[(1, 2, 2.0)]);
    ajoute("poutre sur deux appuis, moment au bout : rotation M L/(3 EI)", s.deplacements[5], 2.0, DERIVE);
    ajoute("poutre sur deux appuis, moment au bout : rotation de l'autre bout", s.deplacements[2], -1.0, DERIVE);
    ajoute("poutre sur deux appuis, moment au bout : réaction", s.reactions[1], 2.0 / 3.0, DERIVE);

    // Portique en L : un montant du nœud 0 (0, 0), encastré, au nœud 1 (0, 3), une traverse jusqu'au nœud 2 (4, 3),
    // libre et chargé de 7 vers le bas.
    let s = reseau(&[(0.0, 0.0), (0.0, 3.0), (4.0, 3.0)], &[(0, 1), (1, 2)], raideurs, &encastre, &[(2, 1, -7.0)]);
    ajoute("portique en L : déplacement horizontal du coin", s.deplacements[3], 42.0, DERIVE);
    ajoute("portique en L : déplacement vertical du coin", s.deplacements[4], -21.0 / 5.0, DERIVE);
    ajoute("portique en L : rotation du coin", s.deplacements[5], -28.0, DERIVE);
    ajoute("portique en L : déplacement vertical du bout", s.deplacements[7], -7469.0 / 45.0, DERIVE);
    ajoute("portique en L : rotation du bout", s.deplacements[8], -140.0 / 3.0, DERIVE);
    ajoute("portique en L : moment d'encastrement", s.reactions[2], 28.0, DERIVE);
    ajoute("portique en L : effort normal du montant", s.efforts[0][3], -7.0, DERIVE);
    ajoute("portique en L : énergie", s.energie, 52283.0 / 90.0, DERIVE);
    // Portique à deux montants : nœuds 0 (0, 0), 1 (0, 3), 2 (4, 3), 3 (4, 0) ; les deux pieds encastrés ; le nœud 1
    // poussé de 7 vers la droite. Hyperstatique : les efforts dépendent des raideurs.
    let pieds = [(0, 0), (0, 1), (0, 2), (3, 0), (3, 1), (3, 2)];
    let points = [(0.0, 0.0), (0.0, 3.0), (4.0, 3.0), (4.0, 0.0)];
    let s = reseau(&points, &[(0, 1), (1, 2), (2, 3)], raideurs, &pieds, &[(1, 0, 7.0)]);
    ajoute("portique : déplacement horizontal du nœud poussé", s.deplacements[3], 298137.0 / 50635.0, DERIVE);
    ajoute("portique : déplacement vertical du nœud poussé", s.deplacements[4], 567.0 / 494.0, DERIVE);
    ajoute("portique : rotation du nœud poussé", s.deplacements[5], -371427.0 / 202540.0, DERIVE);
    ajoute("portique : déplacement horizontal de l'autre coin", s.deplacements[6], 184023.0 / 50635.0, DERIVE);
    ajoute("portique : rotation de l'autre coin", s.deplacements[8], -205443.0 / 202540.0, DERIVE);
    ajoute("portique : réaction horizontale du premier pied", s.reactions[0], -343.0 / 82.0, DERIVE);
    ajoute("portique : réaction verticale du premier pied", s.reactions[1], -945.0 / 494.0, DERIVE);
    ajoute("portique : moment du premier pied", s.reactions[2], 821121.0 / 101270.0, DERIVE);
    ajoute("portique : réaction horizontale du second pied", s.reactions[9], -231.0 / 82.0, DERIVE);
    ajoute("portique : moment du second pied", s.reactions[11], 530649.0 / 101270.0, DERIVE);
    ajoute("portique : moment de la traverse au second coin", s.efforts[1][5], -162603.0 / 50635.0, DERIVE);
    ajoute("portique : énergie", s.energie, 2086959.0 / 101270.0, DERIVE);
    // Le toit 3-4-5 des barres, en poutres (EA = 1, EI = 1), pieds articulés : le nœud rigide du sommet le raidit.
    // Flèche −(125/32)/(1 + 27 EI/(400 EA)) : celle des barres quand EI tend vers zéro, nulle quand EI grandit.
    let s = reseau(
        &[(-3.0, 0.0), (3.0, 0.0), (0.0, 4.0)],
        &[(0, 2), (1, 2)],
        (1.0, 1.0),
        &[(0, 0), (0, 1), (1, 0), (1, 1)],
        &[(2, 1, -1.0)],
    );
    ajoute("toit en poutres : flèche du sommet", s.deplacements[7], -3125.0 / 854.0, DERIVE);
    ajoute("toit en poutres : effort normal", s.efforts[0][3], -250.0 / 427.0, DERIVE);

    // Le nid d'abeille infini, avec l'étirement des parois : E*/Es = (4/√3)(t/l)³/(1 + 3 (t/l)²), ν* = (1 − (t/l)²)/(1 + 3 (t/l)²).
    for (rapport, nom_du_module, nom_du_poisson) in [
        (0.02, "nid d'abeille, t/l = 0,02 : module E*/Es", "nid d'abeille, t/l = 0,02 : coefficient de Poisson"),
        (0.1, "nid d'abeille, t/l = 0,1 : module E*/Es", "nid d'abeille, t/l = 0,1 : coefficient de Poisson"),
        (0.3, "nid d'abeille, t/l = 0,3 : module E*/Es", "nid d'abeille, t/l = 0,3 : coefficient de Poisson"),
    ] {
        let ((module, poisson), carre) = (nid(rapport, 1.0), rapport * rapport);
        ajoute(nom_du_module, module, module_en_flexion(rapport) / (1.0 + 3.0 * carre), DERIVE);
        ajoute(nom_du_poisson, poisson, (1.0 - carre) / (1.0 + 3.0 * carre), DERIVE);
    }
    cas
}

#[allow(dead_code)]
fn main() {
    print!("{}", tableau(&cas()));
}
