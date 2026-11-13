//! Réseau : le réseau de poutres d'une colonie, et l'essai de poussée qui en donne la raideur (étape 15, M3).
//!
//! **Une raideur de colonie n'est publiée nulle part** : ce module calcule la raideur du réseau de poutres que le banc a
//! simulé, avec des raideurs d'hyphe données par l'appelant, pas celle d'un matériau. **Le nombre dépend du choix des mors**
//! (ici une colonne de cases par bord) : une bande plus large le change de 1 à 15 fois (voir `MODELE.md`).
//!
//! # De la colonie au réseau
//!
//! Une colonie est un **arbre** de [`Segment`]s : chaque segment est une poutre, et deux extrémités égales **bit à bit**
//! sont un même nœud (une ramification naît au bout du segment de sa mère). Une **anastomose** ne crée aucun segment :
//! l'apex s'arrête près d'une autre hyphe, sans la toucher. [`contacts`] retrouve, pour chaque pointe anastomosée, le
//! segment visé ; [`extraire`] le **coupe** au point le plus proche de la pointe (un nœud de plus) et ajoute un **pont**,
//! une poutre de plus de ce point à la pointe. Cette règle de contact est un choix de ce banc : aucune source ne la donne.
//! Toutes les poutres ont les mêmes `EA` et `EI` ([`Hyphe`]).
//!
//! # La fonte des poutres trop courtes
//!
//! Une pointe à 1e-4 case d'une hyphe ferait un pont mille milliards de fois plus raide que les autres poutres, et le
//! solveur prendrait le réseau pour un mécanisme (`systeme`). Une poutre plus courte que [`COURT`] est donc **fondue** : ses
//! deux nœuds n'en font plus qu'un. Pour un pont, c'est la pointe qui rejoint son point d'arrivée (l'hyphe visée reste
//! droite) ; pour un morceau de segment, c'est le nœud le plus récent qui rejoint le plus ancien (un point de coupe rejoint
//! le bout du segment). On recommence tant qu'une poutre est trop courte : dans le réseau rendu, aucune ne l'est.
//!
//! # L'essai de poussée
//!
//! [`pousser`] **tient** les nœuds du bord `de` (leurs trois degrés de liberté à zéro) et **déplace** ceux du bord `vers` de
//! `delta` selon l'axe qui joint les deux bords (l'autre translation et la rotation restent à zéro) : deux mors rigides.
//! Un nœud `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)`, comme partout dans le dépôt, et un mors est la colonne (ou la ligne) de
//! cases du bord. La raideur est `K = F / delta`, où `F` est la somme, sur les nœuds du mors déplacé, des réactions selon
//! l'axe de poussée. **Une colonie qui ne touche pas les deux bords a une raideur nulle** : aucun calcul, pas d'erreur.
//!
//! # Les unités
//!
//! Les longueurs sont celles de la simulation (des cases), `ea` est une force et `ei` une force fois une case au carré :
//! `K` sort en force par case.
//!
//! # Le déterminisme
//!
//! Comme `poutres` : `+ - * /`, `sqrt`, `floor`, `clamp`, des boucles dans un ordre fixe, une `BTreeMap` et des `Vec`,
//! aucune `HashMap`. Le même résultat, bit à bit, sur toutes les machines.

use std::collections::BTreeMap;

use crate::anastomose::Index;
use crate::croissance::Segment;
use crate::piece::Bord;
use crate::poutres::{self, Poutre, SolutionPoutres};
use crate::raideur::{Appui, ErreurRaideur, Noeud, longueur_et_direction};
use crate::simulation::Simulation;

/// Une poutre plus courte que cette longueur (en cases) est fondue : ses deux nœuds n'en font plus qu'un.
pub const COURT: f64 = 0.01;

/// Le plus grand nombre de degrés de liberté (trois par nœud) que [`pousser`] accepte : au-delà, [`ErreurReseau::TropGrand`].
pub const INCONNUES_MAX: usize = 3000;

/// Les raideurs de toutes les poutres d'une colonie : `ea` en force, `ei` en force fois une case au carré.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hyphe {
    /// La raideur axiale.
    pub ea: f64,
    /// La raideur en flexion.
    pub ei: f64,
}

/// Une anastomose : la pointe, au bout `(x1, y1)` du segment `pointe`, est reliée au segment `cible` d'une autre hyphe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact {
    /// L'indice du segment dont le bout `(x1, y1)` est la pointe.
    pub pointe: usize,
    /// L'indice du segment visé.
    pub cible: usize,
}

/// D'où vient une poutre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origine {
    /// Le segment de cet indice, entier ou un de ses morceaux s'il a été coupé.
    Segment(usize),
    /// Le pont du contact de ce rang dans la liste donnée à [`extraire`].
    Pont(usize),
}

/// Le réseau d'une colonie : ses nœuds, ses poutres (`a` et `b` indexent `noeuds`) et l'origine de chaque poutre.
#[derive(Clone, Debug, PartialEq)]
pub struct Reseau {
    /// Les nœuds, rangés comme dit à [`extraire`].
    pub noeuds: Vec<Noeud>,
    /// Les poutres, toutes de mêmes `EA` et `EI`.
    pub poutres: Vec<Poutre>,
    /// L'origine de chaque poutre, dans le même ordre.
    pub origines: Vec<Origine>,
}

/// Ce que rend [`pousser`]. Une colonie qui ne traverse pas a tout à zéro, `solution = None` et aucune énergie de poutre.
#[derive(Clone, Debug, PartialEq)]
pub struct Poussee {
    /// `K = F / delta`, en force par case ; zéro si la colonie ne traverse pas.
    pub raideur: f64,
    /// L'énergie de déformation, `½ K delta²`.
    pub energie: f64,
    /// Le nombre de nœuds du mors tenu.
    pub tenus: usize,
    /// Le nombre de nœuds du mors déplacé.
    pub pousses: usize,
    /// Les déplacements, les réactions et les efforts d'extrémité de chaque poutre (voir `poutres`).
    pub solution: Option<SolutionPoutres>,
    /// L'énergie de déformation de chaque poutre, dans l'ordre des poutres : leur somme est `energie`.
    pub energies: Vec<f64>,
}

/// Ce qui empêche le calcul : jamais un nombre à la place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErreurReseau {
    /// Une donnée impossible : le texte dit laquelle.
    DonneeInvalide(&'static str),
    /// Plus de [`INCONNUES_MAX`] degrés de liberté : le solveur plein serait trop lent et trop gourmand.
    TropGrand,
    /// Le réseau est un mécanisme, ou trop mal conditionné pour qu'on l'en distingue (voir `systeme`).
    Mecanisme,
}

impl From<ErreurRaideur> for ErreurReseau {
    fn from(erreur: ErreurRaideur) -> Self {
        match erreur {
            ErreurRaideur::Mecanisme => ErreurReseau::Mecanisme,
            ErreurRaideur::DonneeInvalide(texte) => ErreurReseau::DonneeInvalide(texte),
        }
    }
}

/// Les anastomoses d'une simulation, dans l'ordre des segments de pointe.
///
/// Le noyau ne dit pas quel segment une pointe a rejoint : on **rejoue** sa recherche, la même, juste après chaque segment
/// (même index rempli dans le même ordre, mêmes réglages, même règle de l'obstacle). Le noyau arrête l'apex dont la recherche
/// trouve un segment : un segment dont la recherche rejouée trouve quelque chose est donc le dernier de son apex, et un apex
/// encore actif, ou arrêté par un obstacle, ne trouve rien. Sans réglage d'anastomose, aucun contact.
pub fn contacts(simulation: &Simulation) -> Vec<Contact> {
    let reglages = simulation.reglages();
    let Some(reglage) = reglages.anastomose else { return Vec::new() };
    let (substrat, a_des_obstacles) = (simulation.substrat(), !simulation.milieu().obstacles.is_empty());
    let (largeur, hauteur) = (reglages.largeur as f64, reglages.hauteur as f64);
    let mut index = Index::new(reglages.largeur, reglages.hauteur);
    let mut trouves = Vec::new();
    for (pointe, s) in simulation.segments().iter().enumerate() {
        index.ajouter(*s);
        let cible = if a_des_obstacles {
            // Comme le noyau : un candidat que sépare un obstacle est écarté, et la recherche continue.
            index.chercher_si(s.x1, s.y1, s.apex, s.temps, &reglage, |_, (px, py)| {
                substrat.segment_libre(s.x1, s.y1, px.clamp(0.0, largeur), py.clamp(0.0, hauteur))
            })
        } else {
            index.chercher(s.x1, s.y1, s.apex, s.temps, &reglage)
        };
        if let Some(cible) = cible {
            trouves.push(Contact { pointe, cible });
        }
    }
    trouves
}

/// Le nœud de coordonnées `(x, y)` : l'existant s'il est égal bit à bit (`-0.0` compte comme `0.0`), sinon un nouveau.
fn noeud_en(noeuds: &mut Vec<Noeud>, rang: &mut BTreeMap<(u64, u64), usize>, x: f64, y: f64) -> usize {
    let cle = ((x + 0.0).to_bits(), (y + 0.0).to_bits());
    *rang.entry(cle).or_insert_with(|| {
        noeuds.push(Noeud { x, y });
        noeuds.len() - 1
    })
}

/// Le nœud dans lequel le nœud `i` a fondu (lui-même s'il n'a pas fondu).
fn representant(fondus: &[usize], mut i: usize) -> usize {
    while fondus[i] != i {
        i = fondus[i];
    }
    i
}

/// Le réseau de poutres des `segments` : une poutre par segment, coupée aux points de contact de `contacts`, un pont par
/// contact, `EA` et `EI` de `hyphe` partout ; puis les poutres plus courtes que [`COURT`] sont fondues (voir le module).
///
/// Les nœuds sont rangés dans l'ordre où les segments les rencontrent (le début du premier segment est le nœud 0, la
/// spore), puis les points de coupe dans l'ordre des contacts ; les poutres dans l'ordre des segments (les morceaux d'un
/// segment coupé de son début vers sa fin, de `a` vers `b`), puis les ponts (`a` est le point d'arrivée, `b` la pointe). Une
/// pointe qui se projette au-delà d'un bout du segment visé est reliée à ce bout, sans coupe. Un contact dont la pointe
/// est sur le segment visé, ou à moins de [`COURT`], ne laisse pas de pont : la pointe **est** le point de contact.
///
/// # Errors
///
/// [`ErreurReseau::DonneeInvalide`] : `ea` ou `ei` qui n'est pas un nombre normal strictement positif ; une coordonnée non
/// finie ou de plus de 1e50 ; un segment de longueur nulle (ou dont le carré n'est pas un nombre normal) ; un contact dont
/// un indice sort de la liste, ou dont la pointe est la cible.
pub fn extraire(segments: &[Segment], contacts: &[Contact], hyphe: Hyphe) -> Result<Reseau, ErreurReseau> {
    let invalide = ErreurReseau::DonneeInvalide;
    if !(hyphe.ea.is_normal() && hyphe.ea > 0.0 && hyphe.ei.is_normal() && hyphe.ei > 0.0) {
        return Err(invalide("hyphe : ea et ei doivent être des nombres normaux strictement positifs"));
    }
    let (mut noeuds, mut rang) = (Vec::new(), BTreeMap::new());
    let mut bouts = Vec::with_capacity(segments.len());
    for s in segments {
        if ![s.x0, s.y0, s.x1, s.y1].iter().all(|v| v.abs() <= 1e50) {
            return Err(invalide("segment : coordonnée non finie ou de plus de 1e50"));
        }
        let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
        if !(dx * dx + dy * dy).is_normal() {
            return Err(invalide("segment : longueur nulle, ou si courte que son carré n'est plus un nombre normal"));
        }
        bouts.push((noeud_en(&mut noeuds, &mut rang, s.x0, s.y0), noeud_en(&mut noeuds, &mut rang, s.x1, s.y1)));
    }

    // Les contacts : le point d'arrivée de chaque pont, à la position `t` (de 0 à 1) le long du segment visé.
    let mut coupes: Vec<Vec<(f64, usize)>> = vec![Vec::new(); segments.len()];
    let mut ponts = Vec::with_capacity(contacts.len());
    for (numero, c) in contacts.iter().enumerate() {
        if c.pointe >= segments.len() || c.cible >= segments.len() || c.pointe == c.cible {
            return Err(invalide("contact : segment hors limites, ou la pointe est la cible"));
        }
        let (pointe, cible) = (&segments[c.pointe], &segments[c.cible]);
        let (dx, dy) = (cible.x1 - cible.x0, cible.y1 - cible.y0);
        let t = (((pointe.x1 - cible.x0) * dx + (pointe.y1 - cible.y0) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        let arrivee = if t == 0.0 {
            bouts[c.cible].0
        } else if t == 1.0 {
            bouts[c.cible].1
        } else {
            let coupe = noeud_en(&mut noeuds, &mut rang, cible.x0 + t * dx, cible.y0 + t * dy);
            coupes[c.cible].push((t, coupe));
            coupe
        };
        ponts.push((arrivee, bouts[c.pointe].1, Origine::Pont(numero)));
    }

    // Les poutres : les morceaux de chaque segment, de son début à sa fin, puis les ponts.
    let mut poutres = Vec::with_capacity(segments.len() + 2 * contacts.len());
    for (k, &(debut, fin)) in bouts.iter().enumerate() {
        coupes[k].sort_by(|p, q| p.0.total_cmp(&q.0));
        let mut precedent = debut;
        for suivant in coupes[k].iter().map(|coupe| coupe.1).chain([fin]) {
            poutres.push((precedent, suivant, Origine::Segment(k)));
            precedent = suivant;
        }
    }
    poutres.extend(ponts);

    // La fonte : tant qu'une poutre est plus courte que `COURT`, un de ses deux nœuds fond dans l'autre.
    let mut fondus: Vec<usize> = (0..noeuds.len()).collect();
    let mut une_a_fondu = true;
    while une_a_fondu {
        une_a_fondu = false;
        for &(a, b, origine) in &poutres {
            let (a, b) = (representant(&fondus, a), representant(&fondus, b));
            let (dx, dy) = (noeuds[b].x - noeuds[a].x, noeuds[b].y - noeuds[a].y);
            if a != b && (dx * dx + dy * dy).sqrt() < COURT {
                // La pointe d'un pont (`b`) rejoint son arrivée (`a`) ; ailleurs, le nœud le plus récent rejoint le plus ancien.
                let (reste, fond) = if matches!(origine, Origine::Pont(_)) || a < b { (a, b) } else { (b, a) };
                fondus[fond] = reste;
                une_a_fondu = true;
            }
        }
    }
    // Les nœuds qui restent, renumérotés dans l'ordre ; une poutre dont les deux bouts n'en font plus qu'un disparaît.
    let mut nouveau = vec![usize::MAX; noeuds.len()];
    let mut restants = Vec::new();
    for (i, noeud) in noeuds.iter().enumerate() {
        if fondus[i] == i {
            nouveau[i] = restants.len();
            restants.push(*noeud);
        }
    }
    let mut reseau = Reseau { noeuds: restants, poutres: Vec::new(), origines: Vec::new() };
    for &(a, b, origine) in &poutres {
        let (a, b) = (nouveau[representant(&fondus, a)], nouveau[representant(&fondus, b)]);
        if a != b {
            reseau.poutres.push(Poutre { a, b, ea: hyphe.ea, ei: hyphe.ei });
            reseau.origines.push(origine);
        }
    }
    Ok(reseau)
}

/// Les nœuds du réseau qui sont dans la colonne (ou la ligne) de cases du bord `bord` d'une boîte de `largeur × hauteur`.
fn noeuds_du_bord(reseau: &Reseau, (largeur, hauteur): (usize, usize), bord: Bord) -> Vec<usize> {
    let dans_le_bord = |noeud: &Noeud| match bord {
        Bord::Gauche => noeud.x.floor() == 0.0,
        Bord::Droite => noeud.x.floor() == (largeur - 1) as f64,
        Bord::Haut => noeud.y.floor() == 0.0,
        Bord::Bas => noeud.y.floor() == (hauteur - 1) as f64,
    };
    (0..reseau.noeuds.len()).filter(|&i| dans_le_bord(&reseau.noeuds[i])).collect()
}

/// L'essai de poussée d'un réseau dans une boîte de `largeur × hauteur` cases : les nœuds du bord `de` sont tenus, ceux du
/// bord `vers` déplacés de `delta` selon l'axe qui joint les deux bords (voir le module). `de` et `vers` sont deux bords
/// opposés (`Gauche` et `Droite`, ou `Haut` et `Bas`) ; `delta` peut être négatif (la raideur est la même).
///
/// # Errors
///
/// - [`ErreurReseau::DonneeInvalide`] : `delta` qui n'est pas un nombre normal ; deux bords qui ne sont pas opposés ; une
///   boîte de moins de deux cases entre les deux bords ; et, si la colonie traverse : une poutre dont un indice sort de la
///   liste, ou dont le cube de la longueur n'est pas un nombre normal (de 3e-103 à 5e102 environ : hors de là, la flexion
///   serait fausse) ; une donnée refusée par `poutres::resoudre` ; un résultat (un déplacement, un effort, la force, la
///   raideur, l'énergie) qui déborde ou s'efface, faute de tenir dans la plage normale de `f64` (c'est aussi le sort d'un
///   réseau en plusieurs morceaux dont aucun ne relie les deux mors : sa force est nulle) ;
/// - [`ErreurReseau::TropGrand`] : la colonie traverse et compte plus de [`INCONNUES_MAX`] degrés de liberté ;
/// - [`ErreurReseau::Mecanisme`] : le réseau est un mécanisme, ou trop mal conditionné pour qu'on l'en distingue.
pub fn pousser(
    reseau: &Reseau,
    boite: (usize, usize),
    de: Bord,
    vers: Bord,
    delta: f64,
) -> Result<Poussee, ErreurReseau> {
    let invalide = ErreurReseau::DonneeInvalide;
    if !delta.is_normal() {
        return Err(invalide("delta : un nombre normal (ni nul, ni non fini, ni sous-normal) est attendu"));
    }
    let (axe, cases) = match (de, vers) {
        (Bord::Gauche, Bord::Droite) | (Bord::Droite, Bord::Gauche) => (0, boite.0),
        (Bord::Haut, Bord::Bas) | (Bord::Bas, Bord::Haut) => (1, boite.1),
        _ => return Err(invalide("bords : deux bords opposés sont attendus")),
    };
    if cases < 2 {
        return Err(invalide("boîte : au moins deux cases entre les deux bords"));
    }
    let (tenus, pousses) = (noeuds_du_bord(reseau, boite, de), noeuds_du_bord(reseau, boite, vers));
    let mut poussee = Poussee {
        raideur: 0.0,
        energie: 0.0,
        tenus: tenus.len(),
        pousses: pousses.len(),
        solution: None,
        energies: Vec::new(),
    };
    if tenus.is_empty() || pousses.is_empty() {
        return Ok(poussee);
    }
    if 3 * reseau.noeuds.len() > INCONNUES_MAX {
        return Err(ErreurReseau::TropGrand);
    }
    let mut longueurs = Vec::with_capacity(reseau.poutres.len());
    for p in &reseau.poutres {
        let (longueur, _, _) = longueur_et_direction(&reseau.noeuds, p.a, p.b)?;
        if !(longueur * longueur * longueur).is_normal() {
            return Err(invalide("poutre : le cube de sa longueur n'est pas un nombre normal"));
        }
        longueurs.push(longueur);
    }
    let mut appuis = Vec::with_capacity(3 * (tenus.len() + pousses.len()));
    for &noeud in &tenus {
        appuis.extend((0..3).map(|a| Appui { noeud, axe: a, valeur: 0.0 }));
    }
    for &noeud in &pousses {
        appuis.extend((0..3).map(|a| Appui { noeud, axe: a, valeur: if a == axe { delta } else { 0.0 } }));
    }
    let solution = poutres::resoudre(&reseau.noeuds, &reseau.poutres, &appuis, &[])?;
    let force: f64 = pousses.iter().map(|&noeud| solution.reactions[3 * noeud + axe]).sum();
    // L'énergie d'une poutre : `N² L/(2 EA) + L (m_a² + m_a m_b + m_b²)/(6 EI)`, où `N` est son effort normal et `m_a = −M_a`,
    // `m_b = M_b` ses moments de flexion aux deux bouts (le moment varie en ligne droite de l'un à l'autre). Chaque carré est
    // écrit `x (x/EA)` : un effort de 1e200 ne fait pas déborder son carré.
    for ((p, e), l) in reseau.poutres.iter().zip(&solution.efforts).zip(&longueurs) {
        let (ma, mb) = (-e[2], e[5]);
        let (ka, kb) = (ma / p.ei, mb / p.ei);
        poussee.energies.push(0.5 * e[3] * (e[3] * l / p.ea) + l / 6.0 * (ma * ka + ma * kb + mb * kb));
    }
    (poussee.raideur, poussee.energie) = (force / delta, solution.energie);
    // Un `delta` ou des raideurs extrêmes font déborder (`inf`, `NaN`) ou s'effacer (zéro, sous-normal) un résultat : on le
    // refuse plutôt que de rendre un nombre faux (`delta` = 1e300 avec `EA` = 1e10 rendrait `NaN`).
    let finis = |v: &[f64]| v.iter().all(|x| x.is_finite());
    let tout_est_fini = finis(&solution.deplacements)
        && finis(&solution.reactions)
        && solution.efforts.iter().all(|e| finis(e))
        && finis(&poussee.energies);
    if !(tout_est_fini && force.is_normal() && poussee.raideur.is_normal() && poussee.energie.is_normal()) {
        return Err(invalide("résultat hors de la plage normale de f64 : delta, ea ou ei trop grand ou trop petit"));
    }
    poussee.solution = Some(solution);
    Ok(poussee)
}
