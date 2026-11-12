//! Outils des tests de l'étape 15 (M3), inclus par `#[path = "etape_15/outils.rs"] mod outils;` : les colonies faites à la
//! main (une table), dont la raideur s'écrit en fractions exactes, et de quoi les extraire, les pousser et jouer une colonie.
//! Parmi les fonctions de `f64`, seulement `+ - * /`.
#![allow(dead_code)]

use mycelium_lab::croissance::Segment;
use mycelium_lab::piece::Bord;
use mycelium_lab::poutres::Poutre;
use mycelium_lab::raideur::Noeud;
use mycelium_lab::reseau::{
    Contact, ErreurReseau, Hyphe, Origine, Poussee, Reseau, contacts as contacts_du_noyau, extraire, pousser,
};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

/// `EA = 5` et `EI = 3` partout, comme les cas d'école de l'étape 14.
pub const HYPHE: Hyphe = Hyphe { ea: 5.0, ei: 3.0 };

/// Une hyphe : la ligne brisée de ses points `(x, y)`, des multiples de 1/4 (exacts en `f64`).
pub type Ligne = &'static [(f64, f64)];

/// Une colonie à la main, dans une boîte de `boite` cases, poussée de la gauche vers la droite. Les segments sont numérotés
/// dans l'ordre des hyphes, puis le long de chacune ; un contact est `(segment de la pointe, segment visé)`. `noeuds`,
/// `poutres` et `ponts` sont ce que l'extraction doit rendre (comptés sur le dessin), `raideur` la raideur exacte (calculée à
/// part, en fractions, avec `EA = 5` et `EI = 3`), `sans_contacts` celle de l'arbre seul.
pub struct Colonie {
    pub nom: &'static str,
    pub boite: (usize, usize),
    pub hyphes: &'static [Ligne],
    pub contacts: &'static [(usize, usize)],
    pub noeuds: usize,
    pub poutres: usize,
    pub ponts: usize,
    pub raideur: f64,
    pub sans_contacts: f64,
}

/// La première hyphe des colonies à deux étages : de (0,5 ; 0,5) à (4,5 ; 0,5) en deux segments (0 et 1).
const BAS: Ligne = &[(0.5, 0.5), (2.0, 0.5), (4.5, 0.5)];
/// La seconde, née à la spore : elle monte puis file à droite jusqu'au mors poussé (segments 2 à 5).
const HAUT: Ligne = &[(0.5, 0.5), (0.5, 2.5), (1.5, 2.5), (3.0, 2.5), (4.5, 2.5)];

/// Les colonies à la main. Les nœuds de la colonne de gauche (`x < 1`) sont tenus, ceux de la colonne de droite poussés.
#[rustfmt::skip]
pub const COLONIES: [Colonie; 9] = [
    // Une hyphe droite en quatre segments inégaux, de x = 0,5 à 4,5 : K = EA/L = 5/4.
    Colonie { nom: "chaîne", boite: (5, 3), hyphes: &[&[(0.5, 1.5), (1.25, 1.5), (2.5, 1.5), (3.0, 1.5), (4.5, 1.5)]],
        contacts: &[], noeuds: 5, poutres: 4, ponts: 0, raideur: 1.25, sans_contacts: 1.25 },
    // Deux hyphes droites en parallèle, reliées dans le mors tenu : K = 2 EA/L = 5/2.
    Colonie { nom: "deux hyphes en parallèle", boite: (5, 4), hyphes: &[BAS, &[(0.5, 0.5), (0.5, 2.5), (3.5, 2.5), (4.5, 2.5)]],
        contacts: &[], noeuds: 6, poutres: 5, ponts: 0, raideur: 2.5, sans_contacts: 2.5 },
    // La chaîne et une branche libre (une console de deux poutres) : la branche ne porte rien, K ne change pas.
    Colonie { nom: "chaîne et console", boite: (5, 5),
        hyphes: &[&[(0.5, 1.5), (2.0, 1.5), (4.5, 1.5)], &[(2.0, 1.5), (2.0, 2.5), (2.0, 3.5)]],
        contacts: &[], noeuds: 5, poutres: 4, ponts: 0, raideur: 1.25, sans_contacts: 1.25 },
    // Une seule poutre inclinée (3-4-5) d'un mors à l'autre : K = EA c²/L + 12 EI s²/L³ = 1701/3125 (la matrice de l'étape 14).
    Colonie { nom: "poutre inclinée", boite: (4, 5), hyphes: &[&[(0.5, 0.5), (3.5, 4.5)]],
        contacts: &[], noeuds: 2, poutres: 1, ponts: 0, raideur: 1701.0 / 3125.0, sans_contacts: 1701.0 / 3125.0 },
    // Un Z : une traverse de 1, un montant de 2 qui fléchit, une traverse de 1 : K = 1260/1399.
    Colonie { nom: "hyphe en Z", boite: (3, 3), hyphes: &[&[(0.5, 0.5), (1.5, 0.5), (1.5, 2.5), (2.5, 2.5)]],
        contacts: &[], noeuds: 4, poutres: 3, ponts: 0, raideur: 1260.0 / 1399.0, sans_contacts: 1260.0 / 1399.0 },
    // Une boucle : la seconde hyphe s'arrête en (2,5 ; 2,5) ; une troisième, née au bout de la première, revient vers elle et
    // s'arrête en (2,75 ; 2,5), au-delà du bout du segment visé (le 3) : un pont de 0,25 va à ce bout. K passe de EA/4 à 2 EA/4.
    Colonie { nom: "boucle fermée par un pont", boite: (5, 4),
        hyphes: &[BAS, &[(0.5, 0.5), (0.5, 2.5), (2.5, 2.5)], &[(4.5, 0.5), (4.5, 2.5), (2.75, 2.5)]],
        contacts: &[(5, 3)], noeuds: 7, poutres: 7, ponts: 1, raideur: 2.5, sans_contacts: 1.25 },
    // Une échelle : une tige née de la seconde hyphe en (3 ; 2,5) descend jusqu'à (3 ; 1) et vise le segment 1, qu'elle coupe en
    // (3 ; 0,5), aux 2/5 : un nœud et un pont vertical de plus. Les deux hyphes s'allongent autant : le barreau ne porte rien.
    Colonie { nom: "échelle : segment coupé et pont", boite: (5, 5), hyphes: &[BAS, HAUT, &[(3.0, 2.5), (3.0, 1.0)]],
        contacts: &[(6, 1)], noeuds: 9, poutres: 9, ponts: 1, raideur: 2.5, sans_contacts: 2.5 },
    // Un pont qui porte : la seconde hyphe s'arrête en (3 ; 2,5) et vise le segment 1 en (3 ; 0,5) : le pont vertical de 2
    // transmet l'effort en fléchissant. K = 8459601460/5420380133 ≈ 1,5607, contre EA/4 sans le contact.
    Colonie { nom: "pont qui porte, en flexion", boite: (5, 4), hyphes: &[BAS, &[(0.5, 0.5), (0.5, 2.5), (3.0, 2.5)]],
        contacts: &[(3, 1)], noeuds: 6, poutres: 6, ponts: 1, raideur: 8459601460.0 / 5420380133.0, sans_contacts: 1.25 },
    // L'échelle dont la tige finit **sur** la première hyphe, en (3 ; 0,5) : le segment est coupé à la pointe, sans pont.
    Colonie { nom: "pointe sur le segment : pas de pont", boite: (5, 5), hyphes: &[BAS, HAUT, &[(3.0, 2.5), (3.0, 0.5)]],
        contacts: &[(6, 1)], noeuds: 8, poutres: 8, ponts: 0, raideur: 2.5, sans_contacts: 2.5 },
];

/// Les segments d'une liste d'hyphes : l'apex est le rang de l'hyphe.
pub fn segments(hyphes: &[&[(f64, f64)]]) -> Vec<Segment> {
    let mut tous = Vec::new();
    for (apex, ligne) in hyphes.iter().enumerate() {
        for p in ligne.windows(2) {
            tous.push(Segment { apex: apex as u32, temps: 1, x0: p[0].0, y0: p[0].1, x1: p[1].0, y1: p[1].1 });
        }
    }
    tous
}

/// Les contacts d'une table.
pub fn contacts(table: &[(usize, usize)]) -> Vec<Contact> {
    table.iter().map(|&(pointe, cible)| Contact { pointe, cible }).collect()
}

/// Le réseau de ces hyphes et de ces contacts (`EA = 5`, `EI = 3`).
pub fn reseau(hyphes: &[&[(f64, f64)]], table: &[(usize, usize)]) -> Reseau {
    extraire(&segments(hyphes), &contacts(table), HYPHE).expect("le réseau s'extrait")
}

/// Un réseau fait à la main, sans passer par l'extraction : des nœuds `(x, y)` et des poutres `(a, b)`.
pub fn a_la_main(points: &[(f64, f64)], liens: &[(usize, usize)]) -> Reseau {
    Reseau {
        noeuds: points.iter().map(|&(x, y)| Noeud { x, y }).collect(),
        poutres: liens.iter().map(|&(a, b)| Poutre { a, b, ea: 5.0, ei: 3.0 }).collect(),
        origines: (0..liens.len()).map(Origine::Segment).collect(),
    }
}

/// Une chaîne faite à la main : des nœuds reliés dans l'ordre.
pub fn chaine(points: &[(f64, f64)]) -> Reseau {
    a_la_main(points, &(1..points.len()).map(|i| (i - 1, i)).collect::<Vec<_>>())
}

/// La poussée d'un réseau de la gauche vers la droite (`delta` = 1).
pub fn pousse(reseau: &Reseau, boite: (usize, usize)) -> Result<Poussee, ErreurReseau> {
    pousser(reseau, boite, Bord::Gauche, Bord::Droite, 1.0)
}

/// Sa raideur.
pub fn raideur(reseau: &Reseau, boite: (usize, usize)) -> f64 {
    pousse(reseau, boite).expect("le réseau se pousse").raideur
}

/// Vrai si le calcul est refusé comme une donnée invalide.
pub fn invalide<T>(r: Result<T, ErreurReseau>) -> bool {
    matches!(r, Err(ErreurReseau::DonneeInvalide(_)))
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
pub fn egal(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() < tolerance
}

/// Une colonie simulée : le scénario `texte` joué `pas` pas avec `graine`, ses contacts et sa boîte.
pub fn jouer(texte: &str, graine: u64, pas: u32) -> (Simulation, Vec<Contact>, (usize, usize)) {
    let (reglages, milieu, tropismes, _) = lire_scenario_tropismes(texte).expect("le scénario se lit");
    let boite = (reglages.largeur, reglages.hauteur);
    let mut simulation =
        Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine).expect("il se construit");
    simulation.avancer_n(pas);
    let liste = contacts_du_noyau(&simulation);
    (simulation, liste, boite)
}

/// Les bits de tous les nombres d'une poussée : deux exécutions égales au bit près ont les mêmes.
pub fn bits(p: &Poussee) -> Vec<u64> {
    let mut v = vec![p.raideur.to_bits(), p.energie.to_bits()];
    v.extend(p.energies.iter().map(|e| e.to_bits()));
    if let Some(s) = &p.solution {
        v.extend(s.deplacements.iter().chain(&s.reactions).map(|x| x.to_bits()));
        v.extend(s.efforts.iter().flatten().map(|x| x.to_bits()));
    }
    v
}
