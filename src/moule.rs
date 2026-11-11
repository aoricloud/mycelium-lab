//! Moule : donner une forme à l'espace libre, et mesurer comment la colonie le remplit (étape M1).
//!
//! Un **moule** est un ensemble de cases libres dans la boîte, la *cavité* ; tout le reste est un obstacle. Le noyau de
//! la simulation sait déjà bloquer des rectangles (`obstacle x0 y0 x1 y1` dans un scénario, [`Obstacle`]) : ce module est
//! un **préprocesseur de scénario**. Une ligne `moule …` est remplacée, en texte, par les lignes `obstacle …` qui
//! bloquent le complément de la cavité ([`developper`]) ; le noyau, le binaire `mycelium` et la lecture des scénarios ne
//! sont pas touchés, ils lisent le texte développé. Le module ne dit rien de la raideur ni de la résistance d'un vrai
//! matériau : un moule est un modèle à nous, et aucune publication ne valide « la colonie remplit un moule de cette
//! forme ».
//!
//! # Les conventions
//!
//! Comme partout dans le dépôt, la case `(x, y)` est à l'indice `y * largeur + x` ; `y` croît vers le bas (la ligne 0 est
//! le **haut** de la boîte) ; le point `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)` (règle du plancher).
//!
//! # Ce que contient le module
//!
//! - les [`Forme`]s (brique, L, disque, anneau, labyrinthe) calculées **en entiers** : [`libres`] donne la cavité,
//!   [`obstacles`] son complément en rectangles disjoints ;
//! - [`developper`], le préprocesseur du texte d'un scénario ;
//! - le labyrinthe parfait, [`chambre`] (la sortie : la dernière chambre entière) et [`centre_de_la_chambre`] (l'entrée, et
//!   les deux bouts du plus court chemin) ;
//! - trois recherches de chemin sur une grille de cases libres : [`plus_court_chemin`], [`chemin_a_etoile`] et
//!   [`chemin_en_profondeur`] ;
//! - la mesure du remplissage : [`courbe_de_remplissage`], [`pas_pour_remplir`], et le [`temps_de_sortie`] : le premier pas
//!   où la colonie atteint un rectangle de cases.
//!
//! # Le déterminisme
//!
//! Les formes se calculent en entiers. Parmi les fonctions de `f64`, le module n'emploie que `+ - * /` et les conversions
//! `as` (pour la mesure, par `piece`, qui n'ajoute que `sqrt`, `floor` et `max`) ; il n'utilise ni `HashMap` ni `HashSet`.
//! Le labyrinthe est entièrement déterminé par sa graine : le même moule, bit à bit, sur toutes les machines.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use crate::alea::Alea;
use crate::croissance::Segment;
use crate::piece::mesurer;
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
    /// Un labyrinthe parfait de `colonnes × rangees` chambres carrées de `couloir` cases de côté, tiré avec `graine` (voir
    /// « Le labyrinthe » plus bas). Il faut `colonnes ≥ 1`, `rangees ≥ 1` et `couloir ≥ 1`.
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

// ---------------------------------------------------------------------------------------------------------------------
// Les formes
// ---------------------------------------------------------------------------------------------------------------------

/// Les cases libres de la boîte pour cette forme : `largeur * hauteur` booléens, ligne par ligne, vrai = case de la cavité.
///
/// # Le contrat des formes
///
/// Sinon [`ErreurMoule::FormeInvalide`] :
///
/// - `Brique` : `x0 ≤ x1` et `y0 ≤ y1` ;
/// - `L` : de même, et `1 ≤ epaisseur ≤ min(x1 − x0 + 1, y1 − y0 + 1)` ;
/// - `Disque` : tout rayon est permis ;
/// - `Anneau` : `rayon_interieur < rayon_exterieur` ;
/// - `Labyrinthe` : `colonnes`, `rangees` et `couloir` valent au moins 1.
///
/// Une cavité qui sort de la boîte (une case de la forme à `x ≥ largeur`, à `y ≥ hauteur`, ou au-delà du bord gauche ou
/// haut, c'est-à-dire un `cx − rayon` négatif) est [`ErreurMoule::HorsDeLaBoite`]. Les paramètres sont vérifiés avant la
/// boîte : `FormeInvalide` l'emporte sur `HorsDeLaBoite`. Une boîte de dimension nulle ne contient aucune forme : toute
/// forme valide y est `HorsDeLaBoite`, sans panique. Les contrôles se font avant d'allouer quoi que ce soit, sur des
/// entiers qui ne débordent pas : des paramètres énormes de la forme donnent une erreur, pas une panique. (La boîte, elle,
/// doit pouvoir s'allouer : `largeur × hauteur` booléens.)
pub fn libres(largeur: usize, hauteur: usize, forme: &Forme) -> Result<Vec<bool>, ErreurMoule> {
    verifier_le_contrat(forme)?;
    let (largeur_requise, hauteur_requise) = encombrement(forme).ok_or(ErreurMoule::HorsDeLaBoite)?;
    if largeur_requise > largeur || hauteur_requise > hauteur {
        return Err(ErreurMoule::HorsDeLaBoite);
    }
    if let &Forme::Labyrinthe { colonnes, rangees, couloir, graine } = forme {
        return Ok(labyrinthe(largeur, hauteur, colonnes, rangees, couloir, graine));
    }
    let mut libres = vec![false; largeur * hauteur];
    for y in 0..hauteur {
        for x in 0..largeur {
            libres[y * largeur + x] = dans_la_forme(forme, x, y);
        }
    }
    Ok(libres)
}

/// Vérifie les paramètres de la forme (voir [`libres`]), sans regarder la boîte.
fn verifier_le_contrat(forme: &Forme) -> Result<(), ErreurMoule> {
    let valide = match *forme {
        Forme::Brique { x0, y0, x1, y1 } => x0 <= x1 && y0 <= y1,
        // `epaisseur − 1 ≤ min(x1 − x0, y1 − y0)` dit `epaisseur ≤ min(largeur, hauteur)` sans additionner 1 à `x1 − x0`,
        // qui peut valoir `usize::MAX`.
        Forme::L { x0, y0, x1, y1, epaisseur } => {
            x0 <= x1 && y0 <= y1 && epaisseur >= 1 && epaisseur - 1 <= (x1 - x0).min(y1 - y0)
        }
        Forme::Disque { .. } => true,
        Forme::Anneau { rayon_exterieur, rayon_interieur, .. } => rayon_interieur < rayon_exterieur,
        Forme::Labyrinthe { colonnes, rangees, couloir, .. } => colonnes >= 1 && rangees >= 1 && couloir >= 1,
    };
    if valide { Ok(()) } else { Err(ErreurMoule::FormeInvalide) }
}

/// La plus petite boîte `(largeur, hauteur)` qui contient la forme (supposée valide), ou `None` si aucune boîte ne la
/// contient : une case de la forme à gauche ou au-dessus de la case `(0, 0)`, ou des coordonnées qui débordent `usize`.
fn encombrement(forme: &Forme) -> Option<(usize, usize)> {
    match *forme {
        Forme::Brique { x1, y1, .. } | Forme::L { x1, y1, .. } => Some((x1.checked_add(1)?, y1.checked_add(1)?)),
        Forme::Disque { cx, cy, rayon } | Forme::Anneau { cx, cy, rayon_exterieur: rayon, .. } => {
            // Les cases extrêmes `(cx ± rayon, cy)` et `(cx, cy ± rayon)` sont dans la forme : sa boîte englobante est
            // exactement le carré de côté `2 rayon + 1` centré sur `(cx, cy)`.
            if cx < rayon || cy < rayon {
                return None;
            }
            Some((cx.checked_add(rayon)?.checked_add(1)?, cy.checked_add(rayon)?.checked_add(1)?))
        }
        Forme::Labyrinthe { colonnes, rangees, couloir, .. } => {
            let pas = couloir.checked_add(1)?;
            Some((colonnes.checked_mul(pas)?.checked_add(1)?, rangees.checked_mul(pas)?.checked_add(1)?))
        }
    }
}

/// Vrai si la case `(x, y)` est dans la cavité d'une forme valide qui tient dans la boîte, sauf le labyrinthe (voir
/// [`labyrinthe`]). Les carrés se calculent en `u64` : la boîte est trop petite pour qu'ils débordent.
fn dans_la_forme(forme: &Forme, x: usize, y: usize) -> bool {
    let carre_de_la_distance = |cx: usize, cy: usize| {
        let (dx, dy) = (x.abs_diff(cx) as u64, y.abs_diff(cy) as u64);
        dx * dx + dy * dy
    };
    match *forme {
        Forme::Brique { x0, y0, x1, y1 } => (x0..=x1).contains(&x) && (y0..=y1).contains(&y),
        Forme::L { x0, y0, x1, y1, epaisseur } => {
            // Dans le rectangle englobant, la barre de gauche (`epaisseur` premières colonnes) ou celle du bas (`epaisseur`
            // dernières lignes : à moins de `epaisseur` de la ligne `y1`).
            (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (x - x0 < epaisseur || y1 - y < epaisseur)
        }
        Forme::Disque { cx, cy, rayon } => carre_de_la_distance(cx, cy) <= (rayon as u64) * (rayon as u64),
        Forme::Anneau { cx, cy, rayon_exterieur, rayon_interieur } => {
            let d = carre_de_la_distance(cx, cy);
            d > (rayon_interieur as u64) * (rayon_interieur as u64)
                && d <= (rayon_exterieur as u64) * (rayon_exterieur as u64)
        }
        Forme::Labyrinthe { .. } => unreachable!("le labyrinthe se calcule par `labyrinthe`"),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Les obstacles
// ---------------------------------------------------------------------------------------------------------------------

/// Les obstacles du moule : le complément de la cavité dans la boîte, en rectangles disjoints. Mêmes erreurs que
/// [`libres`].
///
/// # La décomposition
///
/// Elle est déterministe, pour que tous écrivent les mêmes lignes. On parcourt les lignes `y` de haut en bas ; dans chaque
/// ligne, les suites maximales de cases bloquées `[xa, xb]`, par `x` croissant. Une suite **prolonge** le rectangle ouvert
/// de même `(xa, xb)` qui finissait à la ligne `y − 1`, s'il existe ; sinon elle en ouvre un. Un rectangle non prolongé est
/// fermé. Le résultat est trié par `(y0, x0)` croissants.
///
/// Les rectangles sont disjoints, dans la boîte, et leur union est exactement le complément de la cavité. Si la cavité
/// est toute la boîte, la liste est vide.
pub fn obstacles(largeur: usize, hauteur: usize, forme: &Forme) -> Result<Vec<Obstacle>, ErreurMoule> {
    let libres = libres(largeur, hauteur, forme)?;
    Ok(decomposer(largeur, hauteur, &libres))
}

/// Découpe en rectangles les cases bloquées (les `false`) d'une grille `largeur × hauteur` (voir [`obstacles`]).
fn decomposer(largeur: usize, hauteur: usize, libres: &[bool]) -> Vec<Obstacle> {
    let mut rectangles = Vec::new();
    // Les rectangles ouverts à la ligne précédente : `(xa, xb, y0)`, par `xa` croissant.
    let mut ouverts: Vec<(usize, usize, usize)> = Vec::new();
    for y in 0..hauteur {
        let ligne = &libres[y * largeur..(y + 1) * largeur];
        let mut suivants = Vec::new();
        let mut x = 0;
        while x < largeur {
            if ligne[x] {
                x += 1;
                continue;
            }
            // Une suite maximale de cases bloquées `[xa, xb]`.
            let xa = x;
            while x < largeur && !ligne[x] {
                x += 1;
            }
            let xb = x - 1;
            let debut = match ouverts.iter().position(|&(a, b, _)| (a, b) == (xa, xb)) {
                Some(i) => ouverts.remove(i).2,
                None => y,
            };
            suivants.push((xa, xb, debut));
        }
        // Ceux que la ligne n'a pas prolongés finissent à la ligne précédente.
        for (xa, xb, debut) in ouverts {
            rectangles.push(Obstacle { x0: xa, y0: debut, x1: xb, y1: y - 1 });
        }
        ouverts = suivants;
    }
    for (xa, xb, debut) in ouverts {
        rectangles.push(Obstacle { x0: xa, y0: debut, x1: xb, y1: hauteur - 1 });
    }
    rectangles.sort_by_key(|o| (o.y0, o.x0));
    rectangles
}

// ---------------------------------------------------------------------------------------------------------------------
// Le texte d'un scénario
// ---------------------------------------------------------------------------------------------------------------------

/// Remplace la ligne `moule …` du texte d'un scénario par les lignes `obstacle x0 y0 x1 y1` de ses [`obstacles`] ; sans
/// ligne `moule`, rend le texte tel quel, octet pour octet.
///
/// # Le texte
///
/// Une ligne `moule` s'écrit, comme les autres, avec des espaces simples et des entiers décimaux non signés (des
/// chiffres seulement : ni signe, ni espace dans le nombre) :
///
/// ```text
/// moule brique x0 y0 x1 y1
/// moule l x0 y0 x1 y1 epaisseur
/// moule disque cx cy rayon
/// moule anneau cx cy rayon_exterieur rayon_interieur
/// moule labyrinthe colonnes rangees couloir graine
/// ```
///
/// # La lecture
///
/// Les lignes sont lues comme `lire_scenario` les lit (coupées sur `\n`, une fin `\r\n` comprise, rognées, mots séparés par
/// des blancs) : une ligne vide ou qui commence par `#` est ignorée, mais conservée dans le texte rendu. La ligne
/// `boite <l> <h>` donne la boîte et doit venir **avant** la ligne `moule` ; la dernière `boite` bien formée avant la ligne
/// `moule` l'emporte, et une `boite` mal formée est laissée à `lire_scenario` (rien d'autre n'est vérifié ici : c'est
/// `lire_scenario_tropismes` qui refuse, par exemple, une spore dans un obstacle).
///
/// La ligne `moule` est remplacée, à sa place, par ses lignes `obstacle` (une par rectangle, dans l'ordre de [`obstacles`]),
/// chacune terminée par un saut de ligne ; le reste du texte est conservé tel quel, y compris les sauts de ligne et une
/// dernière ligne sans saut de ligne.
///
/// # Les erreurs
///
/// Une ligne `moule` est lue dans cet ordre, la première faute l'emporte : une forme inconnue, un nombre de mots faux ou un
/// mot qui n'est pas un entier décimal non signé (`u64` pour la graine, `usize` pour le reste) rend
/// [`ErreurMoule::LigneInvalide`] avec son numéro ; une seconde ligne `moule`, [`ErreurMoule::MoulesEnDouble`] ; une ligne
/// `moule` avant toute ligne `boite`, [`ErreurMoule::BoiteAbsente`] ; puis les erreurs de [`obstacles`]
/// ([`ErreurMoule::FormeInvalide`], [`ErreurMoule::HorsDeLaBoite`]).
pub fn developper(texte: &str) -> Result<String, ErreurMoule> {
    let mut rendu = String::with_capacity(texte.len());
    let mut boite: Option<(usize, usize)> = None;
    let mut moule_vu = false;
    for (rang, brute) in texte.split_inclusive('\n').enumerate() {
        let numero = rang + 1;
        let ligne = brute.trim();
        // Une ligne vide ou un commentaire est ignorée, mais conservée.
        if ligne.is_empty() || ligne.starts_with('#') {
            rendu.push_str(brute);
            continue;
        }
        let mots: Vec<&str> = ligne.split_whitespace().collect();
        match mots.as_slice() {
            ["boite", largeur, hauteur] => {
                if let (Some(largeur), Some(hauteur)) = (nombre(largeur), nombre(hauteur)) {
                    boite = Some((largeur, hauteur));
                }
                rendu.push_str(brute);
            }
            ["moule", parametres @ ..] => {
                let forme = lire_la_forme(parametres).ok_or(ErreurMoule::LigneInvalide(numero))?;
                if moule_vu {
                    return Err(ErreurMoule::MoulesEnDouble);
                }
                let (largeur, hauteur) = boite.ok_or(ErreurMoule::BoiteAbsente)?;
                for o in obstacles(largeur, hauteur, &forme)? {
                    rendu.push_str(&format!("obstacle {} {} {} {}\n", o.x0, o.y0, o.x1, o.y1));
                }
                moule_vu = true;
            }
            // Toute autre ligne est conservée telle quelle.
            _ => rendu.push_str(brute),
        }
    }
    Ok(rendu)
}

/// Un entier décimal non signé : des chiffres seulement (`"+3"`, `"-0"` et `""` n'en sont pas), qui tient dans le type `T`.
fn nombre<T: std::str::FromStr>(mot: &str) -> Option<T> {
    if mot.is_empty() || !mot.bytes().all(|octet| octet.is_ascii_digit()) {
        return None;
    }
    mot.parse().ok()
}

/// La forme que décrivent les mots d'une ligne `moule` après le mot `moule`, ou `None` si la forme est inconnue, si le nombre
/// de mots est faux ou si un mot n'est pas un entier décimal non signé (`u64` pour la graine, `usize` pour le reste).
fn lire_la_forme(mots: &[&str]) -> Option<Forme> {
    match mots {
        ["brique", x0, y0, x1, y1] => {
            Some(Forme::Brique { x0: nombre(x0)?, y0: nombre(y0)?, x1: nombre(x1)?, y1: nombre(y1)? })
        }
        ["l", x0, y0, x1, y1, epaisseur] => Some(Forme::L {
            x0: nombre(x0)?,
            y0: nombre(y0)?,
            x1: nombre(x1)?,
            y1: nombre(y1)?,
            epaisseur: nombre(epaisseur)?,
        }),
        ["disque", cx, cy, rayon] => Some(Forme::Disque { cx: nombre(cx)?, cy: nombre(cy)?, rayon: nombre(rayon)? }),
        ["anneau", cx, cy, rayon_exterieur, rayon_interieur] => Some(Forme::Anneau {
            cx: nombre(cx)?,
            cy: nombre(cy)?,
            rayon_exterieur: nombre(rayon_exterieur)?,
            rayon_interieur: nombre(rayon_interieur)?,
        }),
        ["labyrinthe", colonnes, rangees, couloir, graine] => Some(Forme::Labyrinthe {
            colonnes: nombre(colonnes)?,
            rangees: nombre(rangees)?,
            couloir: nombre(couloir)?,
            graine: nombre(graine)?,
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le labyrinthe
// ---------------------------------------------------------------------------------------------------------------------

/// Le labyrinthe parfait, dans une boîte `largeur × hauteur` qui le contient (déjà vérifié par [`libres`]).
///
/// # La géométrie
///
/// Pas `p = couloir + 1`. La chambre `(c, r)` (`0 ≤ c < colonnes`, `0 ≤ r < rangees`) est faite des cases
/// `x ∈ [c p + 1, c p + couloir]`, `y ∈ [r p + 1, r p + couloir]`. Les murs sont les lignes `x = c p` et `y = r p`, le
/// contour compris (`x = 0`, `y = 0`, `x = colonnes p`, `y = rangees p`). Le labyrinthe occupe `W = colonnes p + 1` colonnes
/// et `H = rangees p + 1` lignes, en haut à gauche de la boîte (la case `(0, 0)` est un coin de mur) ; les cases au-delà de
/// `W` ou de `H` sont bloquées. Un **passage** entre deux chambres voisines ouvre, dans leur mur commun, les `couloir`
/// cases du côté de la chambre : vers la droite entre `(c, r)` et `(c+1, r)`, les cases `x = (c+1) p`,
/// `y ∈ [r p + 1, r p + couloir]` ; vers le bas entre `(c, r)` et `(c, r+1)`, les cases `y = (r+1) p`,
/// `x ∈ [c p + 1, c p + couloir]`.
///
/// # La génération
///
/// Une recherche en profondeur aléatoire, entièrement déterminée par la graine. Le générateur est `Alea::new(graine)` ; un
/// tableau « visitée » ; une pile qui contient la chambre `(0, 0)`, visitée. Tant que la pile n'est pas vide, soit `(c, r)`
/// son sommet. Ses voisines non visitées, dans l'ordre fixe **haut `(c, r−1)`, droite `(c+1, r)`, bas `(c, r+1)`, gauche
/// `(c−1, r)`** (celles qui existent) : s'il n'y en a aucune, on dépile ; sinon on tire `i = Alea::suivant() % n` (`n` leur
/// nombre, calculé en `u64`), on prend la `i`-ième, on ouvre le passage entre le sommet et elle, on la marque visitée et on
/// l'empile. Le tirage a lieu à chaque fois que `n ≥ 1`, même si `n = 1` : sauter ce tirage décalerait toute la suite.
///
/// Le résultat est **parfait** : les chambres sont les sommets d'un arbre couvrant, avec `colonnes × rangees − 1` passages
/// et un seul chemin simple entre deux chambres *au niveau des chambres* ; au niveau des cases, c'est vrai aussi pour un
/// couloir de 1 case, mais pas pour un couloir plus large (une chambre est un carré de `couloir²` cases, où il y a plusieurs
/// chemins simples). Il y a donc `colonnes rangees couloir² + (colonnes rangees − 1) couloir`
/// cases libres.
fn labyrinthe(
    largeur: usize,
    hauteur: usize,
    colonnes: usize,
    rangees: usize,
    couloir: usize,
    graine: u64,
) -> Vec<bool> {
    let pas = couloir + 1;
    let mut libres = vec![false; largeur * hauteur];
    for rangee in 0..rangees {
        for colonne in 0..colonnes {
            for y in rangee * pas + 1..=rangee * pas + couloir {
                for x in colonne * pas + 1..=colonne * pas + couloir {
                    libres[y * largeur + x] = true;
                }
            }
        }
    }

    let mut alea = Alea::new(graine);
    let mut visitee = vec![false; colonnes * rangees];
    visitee[0] = true;
    let mut pile = vec![(0usize, 0usize)];
    while let Some(&(c, r)) = pile.last() {
        // Les voisines non visitées, dans l'ordre haut, droite, bas, gauche.
        let mut voisines = Vec::with_capacity(4);
        if r > 0 {
            voisines.push((c, r - 1));
        }
        if c + 1 < colonnes {
            voisines.push((c + 1, r));
        }
        if r + 1 < rangees {
            voisines.push((c, r + 1));
        }
        if c > 0 {
            voisines.push((c - 1, r));
        }
        voisines.retain(|&(vc, vr)| !visitee[vr * colonnes + vc]);
        if voisines.is_empty() {
            pile.pop();
            continue;
        }
        let (vc, vr) = voisines[(alea.suivant() % voisines.len() as u64) as usize];

        // Le passage entre (c, r) et (vc, vr) : dans leur mur commun, les `couloir` cases du côté de la chambre.
        if vr == r {
            let x = c.max(vc) * pas;
            for y in r * pas + 1..=r * pas + couloir {
                libres[y * largeur + x] = true;
            }
        } else {
            let y = r.max(vr) * pas;
            for x in c * pas + 1..=c * pas + couloir {
                libres[y * largeur + x] = true;
            }
        }
        visitee[vr * colonnes + vc] = true;
        pile.push((vc, vr));
    }
    libres
}

/// Les cases de la chambre `(colonne, rangee)` d'un labyrinthe de couloir `couloir` : le rectangle `(x0, y0, x1, y1)`, bornes
/// incluses, avec `x0 = colonne p + 1`, `x1 = x0 + couloir − 1`, `y0 = rangee p + 1`, `y1 = y0 + couloir − 1` et `p = couloir + 1`.
///
/// La sortie d'un scénario de labyrinthe est la chambre `(colonnes − 1, rangees − 1)` entière : c'est la cible de
/// [`temps_de_sortie`] (la case centre seule serait un instrument bruité : un segment finit rarement dans une case donnée).
pub fn chambre(couloir: usize, colonne: usize, rangee: usize) -> (usize, usize, usize, usize) {
    let pas = couloir + 1;
    let (x0, y0) = (colonne * pas + 1, rangee * pas + 1);
    (x0, y0, x0 + couloir - 1, y0 + couloir - 1)
}

/// La case centre de la chambre `(colonne, rangee)` d'un labyrinthe de couloir `couloir` :
/// `(colonne p + 1 + couloir / 2, rangee p + 1 + couloir / 2)` avec `p = couloir + 1` et `/` la division entière.
///
/// La spore d'un scénario de labyrinthe est au centre de la chambre `(0, 0)` ; le plus court chemin se mesure du centre de la
/// chambre `(0, 0)` à celui de la chambre `(colonnes − 1, rangees − 1)`.
pub fn centre_de_la_chambre(couloir: usize, colonne: usize, rangee: usize) -> (usize, usize) {
    let pas = couloir + 1;
    (colonne * pas + 1 + couloir / 2, rangee * pas + 1 + couloir / 2)
}

// ---------------------------------------------------------------------------------------------------------------------
// Les chemins
// ---------------------------------------------------------------------------------------------------------------------

// Les trois recherches de chemin vont de case en case **par les côtés** : haut, droite, bas, gauche (quatre voisins). Ce
// n'est pas le plus court chemin géométrique, qui peut couper un angle en diagonale : un chemin de cases par les côtés est
// plus long, d'un mouvement de coin à l'autre, que le segment droit qui coupe l'angle. `libres` est celui de [`libres`]
// (`largeur * hauteur` booléens, ligne par ligne) ; une tranche d'une autre longueur fait paniquer chaque recherche.

/// Les indices des quatre voisines de la case `i` dans l'ordre haut, droite, bas, gauche ; `None` quand elle est hors de la
/// grille.
fn voisines(largeur: usize, hauteur: usize, i: usize) -> [Option<usize>; 4] {
    let (x, y) = (i % largeur, i / largeur);
    [
        (y > 0).then(|| i - largeur),
        (x + 1 < largeur).then(|| i + 1),
        (y + 1 < hauteur).then(|| i + largeur),
        (x > 0).then(|| i - 1),
    ]
}

/// Les indices du départ et de l'arrivée, ou `None` si l'une des deux cases est hors de la grille ou bloquée. Panique si
/// `libres` n'a pas `largeur × hauteur` cases.
fn extremites(
    largeur: usize,
    hauteur: usize,
    libres: &[bool],
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> Option<(usize, usize)> {
    assert_eq!(
        libres.len(),
        largeur * hauteur,
        "la tranche a {} cases, la boîte {largeur} × {hauteur} en demande {}",
        libres.len(),
        largeur * hauteur
    );
    let indice =
        |(x, y): (usize, usize)| (x < largeur && y < hauteur && libres[y * largeur + x]).then(|| y * largeur + x);
    Some((indice(depart)?, indice(arrivee)?))
}

/// Le plus court chemin de cases libres (4 voisins : haut, droite, bas, gauche), en nombre de pas ; `Some(0)` si
/// `depart == arrivee` ; `None` si une des cases est bloquée ou hors de la grille, ou si l'arrivée est inatteignable.
///
/// Un parcours en largeur depuis le départ. Panique si `libres` n'a pas `largeur × hauteur` cases.
pub fn plus_court_chemin(
    largeur: usize,
    hauteur: usize,
    libres: &[bool],
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> Option<usize> {
    let (depart, arrivee) = extremites(largeur, hauteur, libres, depart, arrivee)?;
    // `distance[i]` : le nombre de pas du départ à la case `i`, `usize::MAX` tant qu'elle n'est pas atteinte.
    let mut distance = vec![usize::MAX; libres.len()];
    distance[depart] = 0;
    let mut file = VecDeque::from([depart]);
    while let Some(i) = file.pop_front() {
        if i == arrivee {
            return Some(distance[i]);
        }
        for voisine in voisines(largeur, hauteur, i).into_iter().flatten() {
            if libres[voisine] && distance[voisine] == usize::MAX {
                distance[voisine] = distance[i] + 1;
                file.push_back(voisine);
            }
        }
    }
    None
}

/// Même réponse que [`plus_court_chemin`], trouvée par A\* : l'heuristique est la distance de Manhattan à l'arrivée, qui est
/// admissible (jamais plus grande que le vrai chemin) et cohérente (elle baisse d'au plus 1 par pas), donc la première fois
/// que l'arrivée sort du tas, son chemin est le plus court. Le départage est déterministe : à `f = g + h` égal, la case
/// d'indice le plus petit sort la première. Panique si `libres` n'a pas `largeur × hauteur` cases.
pub fn chemin_a_etoile(
    largeur: usize,
    hauteur: usize,
    libres: &[bool],
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> Option<usize> {
    let (depart, arrivee) = extremites(largeur, hauteur, libres, depart, arrivee)?;
    let manhattan = |i: usize| (i % largeur).abs_diff(arrivee % largeur) + (i / largeur).abs_diff(arrivee / largeur);
    // `cout[i]` : le meilleur nombre de pas connu du départ à la case `i` ; `fermee[i]` : son chemin est définitif.
    let mut cout = vec![usize::MAX; libres.len()];
    let mut fermee = vec![false; libres.len()];
    cout[depart] = 0;
    let mut tas = BinaryHeap::from([Reverse((manhattan(depart), depart))]);
    while let Some(Reverse((_, i))) = tas.pop() {
        if fermee[i] {
            // Une entrée périmée : la case est sortie du tas avec un meilleur coût.
            continue;
        }
        fermee[i] = true;
        if i == arrivee {
            return Some(cout[i]);
        }
        for voisine in voisines(largeur, hauteur, i).into_iter().flatten() {
            if libres[voisine] && !fermee[voisine] && cout[i] + 1 < cout[voisine] {
                cout[voisine] = cout[i] + 1;
                tas.push(Reverse((cout[voisine] + manhattan(voisine), voisine)));
            }
        }
    }
    None
}

/// La longueur (en pas) du premier chemin simple que trouve une recherche en profondeur, voisines essayées dans l'ordre haut,
/// droite, bas, gauche ; `None` si l'arrivée est inatteignable ou si une des cases est bloquée ou hors de la grille. Elle est
/// au moins celle de [`plus_court_chemin`] ; elle lui est égale quand un seul chemin simple existe, par exemple dans un labyrinthe
/// parfait de couloir 1 (ses cases libres forment un arbre). Avec un couloir plus large, une chambre est un carré où elle peut
/// tourner en rond : elle est alors plus longue.
///
/// C'est la recherche en profondeur ordinaire : une case, une fois visitée, ne l'est plus, et le chemin trouvé est celui de
/// la pile quand l'arrivée est atteinte. Elle est **itérative**, avec une pile explicite de `(case, prochaine voisine à
/// essayer)` : un labyrinthe de 10 000 cases ne fait pas déborder la pile d'appels. Panique si `libres` n'a pas
/// `largeur × hauteur` cases.
pub fn chemin_en_profondeur(
    largeur: usize,
    hauteur: usize,
    libres: &[bool],
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> Option<usize> {
    let (depart, arrivee) = extremites(largeur, hauteur, libres, depart, arrivee)?;
    let mut visitee = vec![false; libres.len()];
    visitee[depart] = true;
    let mut pile = vec![(depart, 0usize)];
    while let Some(sommet) = pile.last_mut() {
        let (i, direction) = *sommet;
        if i == arrivee {
            // La pile est le chemin du départ à l'arrivée : un pas entre deux cases consécutives.
            return Some(pile.len() - 1);
        }
        if direction == 4 {
            pile.pop();
            continue;
        }
        sommet.1 += 1;
        if let Some(voisine) = voisines(largeur, hauteur, i)[direction]
            && libres[voisine]
            && !visitee[voisine]
        {
            visitee[voisine] = true;
            pile.push((voisine, 0));
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------------------------------
// Le remplissage et le temps de sortie
// ---------------------------------------------------------------------------------------------------------------------

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
/// `jusqu_au_pas` inclus (la simulation avance de `jusqu_au_pas − temps()` pas ; si `jusqu_au_pas ≤ temps()`, un seul point,
/// sans avancer). Chaque point est tiré de [`mesurer`] (seuil nul) : `cases_occupees`, `cases_libres` et `remplissage`.
///
/// Aucune case ne se vide : la courbe est croissante au sens large (une case occupée garde son hyphe).
pub fn courbe_de_remplissage(simulation: &mut Simulation, jusqu_au_pas: u32) -> Vec<PointDeCourbe> {
    let mut courbe = vec![point_de_courbe(simulation)];
    while simulation.temps() < jusqu_au_pas {
        simulation.avancer();
        courbe.push(point_de_courbe(simulation));
    }
    courbe
}

/// Le point de la courbe à l'instant courant de la simulation.
fn point_de_courbe(simulation: &Simulation) -> PointDeCourbe {
    let mesures = mesurer(simulation);
    PointDeCourbe {
        pas: simulation.temps(),
        cases_occupees: mesures.cases_occupees,
        cases_libres: mesures.cases_libres,
        remplissage: mesures.remplissage,
    }
}

/// Le premier pas où la part remplie atteint `part` (`remplissage ≥ part`, comparé tel quel, sans tolérance) ; `None` si la
/// courbe n'y arrive pas. `part = 0` donne le pas du premier point.
///
/// Contrat : panique si `part` n'est pas dans `[0, 1]` (NaN compris).
pub fn pas_pour_remplir(courbe: &[PointDeCourbe], part: f64) -> Option<u32> {
    assert!((0.0..=1.0).contains(&part), "la part à remplir est dans [0, 1] : {part}");
    courbe.iter().find(|point| point.remplissage >= part).map(|point| point.pas)
}

/// Le temps de sortie : le premier pas, au plus `pas_max`, où la colonie atteint la `cible`, un **rectangle de cases**
/// `(x0, y0, x1, y1)`, bornes incluses. La colonie l'atteint au pas `t` si un segment posé au pas `t` a sa **fin** `(x1, y1)`
/// dans une case du rectangle (règle du plancher) ; elle l'atteint au pas 0 si la case de la spore (`reglages().spore`) est
/// dans le rectangle. La simulation avance, un pas à la fois, seulement autant qu'il le faut, et jamais au-delà de `pas_max`.
///
/// # La règle exacte
///
/// 1. Si la case de la spore est dans la cible : `Some(0)`, sans avancer.
/// 2. Sinon, soit `m` le plus petit `temps` parmi les segments déjà posés dont `temps ≤ pas_max` et dont la fin est dans la
///    cible : s'il existe, `Some(m)`, sans avancer.
/// 3. Sinon, tant que `temps() < pas_max` : avancer d'un pas ; si un segment de ce pas finit dans la cible, `Some(temps())`.
/// 4. Sinon `None`.
///
/// Le résultat ne dépend donc que du scénario, de la graine, de la cible et de `pas_max`, pas du nombre de pas déjà joués :
/// c'est le plus petit `t ≤ pas_max` où la colonie est dans la cible, ou `None` s'il n'y en a pas.
///
/// Contrat : `x0 ≤ x1` et `y0 ≤ y1`, sinon panique. Une cible qui sort de la boîte (`x1 ≥ largeur` ou `y1 ≥ hauteur`) rend
/// `None` sans avancer.
pub fn temps_de_sortie(simulation: &mut Simulation, cible: (usize, usize, usize, usize), pas_max: u32) -> Option<u32> {
    let (x0, y0, x1, y1) = cible;
    assert!(x0 <= x1 && y0 <= y1, "la cible est un rectangle à l'endroit : {cible:?}");
    let reglages = simulation.reglages();
    if x1 >= reglages.largeur || y1 >= reglages.hauteur {
        return None;
    }
    let dans_la_cible = |x: usize, y: usize| (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
    if dans_la_cible(reglages.spore.0, reglages.spore.1) {
        return Some(0);
    }
    let finit_dans_la_cible = |s: &Segment| dans_la_cible(s.x1.floor() as usize, s.y1.floor() as usize);
    let deja_pose =
        simulation.segments().iter().filter(|s| s.temps <= pas_max && finit_dans_la_cible(s)).map(|s| s.temps).min();
    if deja_pose.is_some() {
        return deja_pose;
    }
    while simulation.temps() < pas_max {
        let deja_vus = simulation.segments().len();
        simulation.avancer();
        if simulation.segments()[deja_vus..].iter().any(finit_dans_la_cible) {
            return Some(simulation.temps());
        }
    }
    None
}
