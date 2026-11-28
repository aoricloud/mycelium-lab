//! Le moule en volume : donner une forme à l'espace libre du cube, et mesurer quelle part la colonie en atteint (étape
//! 26, marche 3 de la 3D).
//!
//! C'est le portage en volume de [`crate::moule`]. Un **moule** est un ensemble de cases libres du cube, la *cavité* ;
//! tout le reste est un obstacle. La colonie en volume sait déjà bloquer des pavés (`obstacle x0 y0 z0 x1 y1 z1` dans
//! un scénario de [`crate::colonie3d`]) : ce module est un **préprocesseur de scénario**. Une ligne `moule …` est
//! remplacée, en texte, par les lignes `obstacle …` qui bloquent le complément de la cavité ([`developper_3d`]) ; le
//! noyau de la colonie et la lecture des scénarios ne sont pas touchés, ils lisent le texte développé. Le module ne dit
//! rien de la raideur ni de la résistance d'un vrai matériau : un moule est un modèle à nous, et aucune publication ne
//! valide « la colonie remplit un moule de cette forme ».
//!
//! # Les conventions
//!
//! Comme dans [`crate::substrat3d`], la case `(x, y, z)` est à l'indice `(z * hauteur + y) * largeur + x` ; le point
//! `(x, y, z)` est dans la case `(⌊x⌋, ⌊y⌋, ⌊z⌋)` (règle du plancher). Toutes les coordonnées d'une forme sont des
//! numéros de cases, bornes incluses.
//!
//! # Ce que contient le module
//!
//! - les [`Forme3d`]s (pavé, coquille, boule, tore, labyrinthe) calculées **en entiers** : [`libres_3d`] donne la
//!   cavité, [`obstacles_3d`] son complément en pavés disjoints ;
//! - [`developper_3d`], le préprocesseur du texte d'un scénario de colonie ;
//! - le labyrinthe parfait en volume, [`chambre_3d`] et [`centre_de_la_chambre_3d`] ;
//! - la mesure du remplissage : [`cases_atteintes`], [`part_remplie`] et [`courbe_de_remplissage_3d`].
//!
//! # Le déterminisme
//!
//! Les formes se calculent en entiers (la boule et le tore aussi : leurs tests s'écrivent en carrés d'entiers, sans
//! racine). Parmi les fonctions de `f64`, le module n'emploie que `floor`, `/` et les conversions `as` ; il n'utilise
//! ni `HashMap` ni `HashSet`. Le labyrinthe est entièrement déterminé par sa graine : le même moule, bit à bit, sur
//! toutes les machines.
//!
//! # Le labyrinthe
//!
//! **La géométrie.** Pas `q = couloir + 1`. La chambre `(c, r, e)` (`c < colonnes`, `r < rangees`, `e < etages`) est
//! faite des cases `x ∈ [c q + 1, c q + couloir]`, `y ∈ [r q + 1, r q + couloir]`, `z ∈ [e q + 1, e q + couloir]`. Les
//! murs sont les plans `x = c q`, `y = r q` et `z = e q`, le contour compris. Le labyrinthe occupe `colonnes q + 1` ×
//! `rangees q + 1` × `etages q + 1` cases dans le coin `(0, 0, 0)` du cube ; les cases au-delà sont bloquées. Un
//! **passage** entre deux chambres voisines ouvre, dans leur mur commun, les `couloir²` cases en face de la chambre :
//! entre `(c, r, e)` et `(c + 1, r, e)`, les cases `x = (c + 1) q`, `y ∈ [r q + 1, r q + couloir]`,
//! `z ∈ [e q + 1, e q + couloir]` ; de même en `y` et en `z`.
//!
//! **La génération.** Celle de [`crate::moule`], une direction de plus : une recherche en profondeur aléatoire,
//! entièrement déterminée par la graine. Le générateur est `Alea::new(graine)` ; un tableau « visitée » ; une pile qui
//! contient la chambre `(0, 0, 0)`, visitée. Tant que la pile n'est pas vide, soit `(c, r, e)` son sommet. Ses voisines
//! non visitées, dans l'ordre fixe **`(c, r − 1, e)`, `(c + 1, r, e)`, `(c, r + 1, e)`, `(c − 1, r, e)`,
//! `(c, r, e − 1)`, `(c, r, e + 1)`** (celles qui existent) : s'il n'y en a aucune, on dépile ; sinon on tire
//! `i = Alea::suivant() % n` (`n` leur nombre, en `u64`), on prend la `i`-ième, on ouvre le passage, on la marque
//! visitée et on l'empile. Le tirage a lieu à chaque fois que `n ≥ 1`, même si `n = 1` : sauter ce tirage décalerait
//! toute la suite.
//!
//! Le résultat est **parfait au niveau des chambres** : un arbre couvrant, avec `colonnes × rangees × etages − 1`
//! passages. Il y a donc `N couloir³ + (N − 1) couloir²` cases libres, `N` étant le nombre de chambres.

use crate::alea::Alea;
use crate::colonie3d::Colonie3d;
use crate::cube::Bloc;

/// La forme de la cavité d'un moule en volume. Toutes les coordonnées sont des numéros de cases, bornes incluses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forme3d {
    /// La cavité est le pavé `x0..=x1` × `y0..=y1` × `z0..=z1`.
    Pave { x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize },
    /// Une brique creuse : les cases du pavé `x0..=x1` × `y0..=y1` × `z0..=z1` à moins de `epaisseur` cases d'une de
    /// ses six faces (`x − x0 < epaisseur`, ou `x1 − x < epaisseur`, et de même en `y` et en `z`). Quand
    /// `2 · epaisseur` atteint le plus petit côté du pavé, la coquille est le pavé plein.
    Coquille { x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize, epaisseur: usize },
    /// Les cases `(x, y, z)` telles que `(x − cx)² + (y − cy)² + (z − cz)² ≤ rayon²`, en entiers. `rayon = 0` donne la
    /// case seule.
    Boule { cx: usize, cy: usize, cz: usize, rayon: usize },
    /// Un tore d'axe parallèle à `z`, de centre `(cx, cy, cz)` : les cases telles que `(ρ − R)² + (z − cz)² ≤ r²`, avec
    /// `ρ² = (x − cx)² + (y − cy)²`, `R = grand_rayon` et `r = petit_rayon` ; il faut `petit_rayon < grand_rayon`. Le
    /// calcul se fait en entiers, sans racine (voir [`libres_3d`]).
    Tore { cx: usize, cy: usize, cz: usize, grand_rayon: usize, petit_rayon: usize },
    /// Un labyrinthe parfait de `colonnes × rangees × etages` chambres cubiques de `couloir` cases de côté, tiré avec
    /// `graine` (voir « Le labyrinthe » dans la doc du module). Il faut `colonnes`, `rangees`, `etages` et `couloir`
    /// au moins 1.
    Labyrinthe { colonnes: usize, rangees: usize, etages: usize, couloir: usize, graine: u64 },
}

/// Pourquoi un moule en volume est refusé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurMoule3d {
    /// Une ligne `moule` sans ligne `cube` bien formée avant elle.
    CubeAbsent,
    /// Deux lignes `moule`.
    MoulesEnDouble,
    /// Ligne `moule` inconnue ou mal formée ; porte le numéro de ligne, à partir de 1.
    LigneInvalide(usize),
    /// Paramètres hors contrat : voir [`libres_3d`].
    FormeInvalide,
    /// Une case de la cavité sort du cube.
    HorsDuCube,
}

// ---------------------------------------------------------------------------------------------------------------------
// Les formes
// ---------------------------------------------------------------------------------------------------------------------

/// Les cases libres du cube pour cette forme : `largeur * hauteur * profondeur` booléens, à l'indice
/// `(z * hauteur + y) * largeur + x`, vrai = case de la cavité.
///
/// # Le contrat des formes
///
/// Sinon [`ErreurMoule3d::FormeInvalide`] :
///
/// - `Pave` : `x0 ≤ x1`, `y0 ≤ y1` et `z0 ≤ z1` ;
/// - `Coquille` : de même, et `epaisseur ≥ 1` ;
/// - `Boule` : tout rayon est permis ;
/// - `Tore` : `petit_rayon < grand_rayon` ;
/// - `Labyrinthe` : `colonnes`, `rangees`, `etages` et `couloir` valent au moins 1.
///
/// Une cavité qui sort du cube (une case de la forme à `x ≥ largeur`, `y ≥ hauteur` ou `z ≥ profondeur`, ou de
/// coordonnée négative, comme un `cx − rayon` négatif) est [`ErreurMoule3d::HorsDuCube`]. Les paramètres sont vérifiés
/// avant le cube : `FormeInvalide` l'emporte sur `HorsDuCube`. Un cube de dimension nulle ne contient aucune forme :
/// toute forme valide y est `HorsDuCube`, sans panique. Les contrôles se font avant d'allouer quoi que ce soit, sur des
/// entiers qui ne débordent pas : des paramètres énormes donnent une erreur, pas une panique. (Le cube, lui, doit
/// pouvoir s'allouer : `largeur × hauteur × profondeur` booléens.)
///
/// # Le tore en entiers
///
/// Avec `s = (x − cx)² + (y − cy)²`, `d = (z − cz)²` et `A = s + R² + d − r²` (en `i128`) : la case est dans le tore si
/// `A ≤ 0`, ou si `A > 0` et `A² ≤ 4 R² s`. C'est `(√s − R)² + d ≤ r²` élevé au carré, sans racine.
pub fn libres_3d(
    largeur: usize,
    hauteur: usize,
    profondeur: usize,
    forme: &Forme3d,
) -> Result<Vec<bool>, ErreurMoule3d> {
    verifier_le_contrat_3d(forme)?;
    let (largeur_requise, hauteur_requise, profondeur_requise) =
        encombrement_3d(forme).ok_or(ErreurMoule3d::HorsDuCube)?;
    if largeur_requise > largeur || hauteur_requise > hauteur || profondeur_requise > profondeur {
        return Err(ErreurMoule3d::HorsDuCube);
    }
    if let &Forme3d::Labyrinthe { colonnes, rangees, etages, couloir, graine } = forme {
        return Ok(labyrinthe_3d((largeur, hauteur, profondeur), colonnes, rangees, etages, couloir, graine));
    }
    let mut libres = vec![false; largeur * hauteur * profondeur];
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                libres[(z * hauteur + y) * largeur + x] = dans_la_forme_3d(forme, x, y, z);
            }
        }
    }
    Ok(libres)
}

/// Vérifie les paramètres de la forme (voir [`libres_3d`]), sans regarder le cube.
fn verifier_le_contrat_3d(forme: &Forme3d) -> Result<(), ErreurMoule3d> {
    let valide = match *forme {
        Forme3d::Pave { x0, y0, z0, x1, y1, z1 } => x0 <= x1 && y0 <= y1 && z0 <= z1,
        Forme3d::Coquille { x0, y0, z0, x1, y1, z1, epaisseur } => x0 <= x1 && y0 <= y1 && z0 <= z1 && epaisseur >= 1,
        Forme3d::Boule { .. } => true,
        Forme3d::Tore { grand_rayon, petit_rayon, .. } => petit_rayon < grand_rayon,
        Forme3d::Labyrinthe { colonnes, rangees, etages, couloir, .. } => {
            colonnes >= 1 && rangees >= 1 && etages >= 1 && couloir >= 1
        }
    };
    if valide { Ok(()) } else { Err(ErreurMoule3d::FormeInvalide) }
}

/// La plus petite boîte `(largeur, hauteur, profondeur)` qui contient la forme (supposée valide), ou `None` si aucune
/// boîte ne la contient : une case de la forme à coordonnée négative, ou des coordonnées qui débordent `usize`.
fn encombrement_3d(forme: &Forme3d) -> Option<(usize, usize, usize)> {
    match *forme {
        Forme3d::Pave { x1, y1, z1, .. } | Forme3d::Coquille { x1, y1, z1, .. } => {
            Some((x1.checked_add(1)?, y1.checked_add(1)?, z1.checked_add(1)?))
        }
        Forme3d::Boule { cx, cy, cz, rayon } => {
            // Les six cases extrêmes `(cx ± rayon, cy, cz)`, `(cx, cy ± rayon, cz)` et `(cx, cy, cz ± rayon)` sont dans
            // la boule : sa boîte englobante est exactement le cube de côté `2 rayon + 1` centré sur `(cx, cy, cz)`.
            if cx < rayon || cy < rayon || cz < rayon {
                return None;
            }
            Some((
                cx.checked_add(rayon)?.checked_add(1)?,
                cy.checked_add(rayon)?.checked_add(1)?,
                cz.checked_add(rayon)?.checked_add(1)?,
            ))
        }
        Forme3d::Tore { cx, cy, cz, grand_rayon, petit_rayon } => {
            // La cavité atteint `cx ± (R + r)`, `cy ± (R + r)` et `cz ± r` : sa boîte englobante est
            // `[cx − R − r, cx + R + r] × [cy − R − r, cy + R + r] × [cz − r, cz + r]`.
            let etendue = grand_rayon.checked_add(petit_rayon)?;
            if cx < etendue || cy < etendue || cz < petit_rayon {
                return None;
            }
            Some((
                cx.checked_add(etendue)?.checked_add(1)?,
                cy.checked_add(etendue)?.checked_add(1)?,
                cz.checked_add(petit_rayon)?.checked_add(1)?,
            ))
        }
        Forme3d::Labyrinthe { colonnes, rangees, etages, couloir, .. } => {
            let pas = couloir.checked_add(1)?;
            Some((
                colonnes.checked_mul(pas)?.checked_add(1)?,
                rangees.checked_mul(pas)?.checked_add(1)?,
                etages.checked_mul(pas)?.checked_add(1)?,
            ))
        }
    }
}

/// Vrai si la case `(x, y, z)` est dans la cavité d'une forme valide qui tient dans le cube, sauf le labyrinthe (voir
/// [`labyrinthe_3d`]). Les carrés se calculent en `i128` : les coordonnées d'une forme qui tient dans le cube sont trop
/// petites pour qu'ils débordent.
fn dans_la_forme_3d(forme: &Forme3d, x: usize, y: usize, z: usize) -> bool {
    let (x, y, z) = (x as i128, y as i128, z as i128);
    match *forme {
        Forme3d::Pave { x0, y0, z0, x1, y1, z1 } => {
            let (x0, y0, z0, x1, y1, z1) = (x0 as i128, y0 as i128, z0 as i128, x1 as i128, y1 as i128, z1 as i128);
            (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (z0..=z1).contains(&z)
        }
        Forme3d::Coquille { x0, y0, z0, x1, y1, z1, epaisseur } => {
            let (x0, y0, z0, x1, y1, z1) = (x0 as i128, y0 as i128, z0 as i128, x1 as i128, y1 as i128, z1 as i128);
            let epaisseur = epaisseur as i128;
            let dedans = (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (z0..=z1).contains(&z);
            dedans
                && (x - x0 < epaisseur
                    || x1 - x < epaisseur
                    || y - y0 < epaisseur
                    || y1 - y < epaisseur
                    || z - z0 < epaisseur
                    || z1 - z < epaisseur)
        }
        Forme3d::Boule { cx, cy, cz, rayon } => {
            let (dx, dy, dz) = (x - cx as i128, y - cy as i128, z - cz as i128);
            let rayon = rayon as i128;
            dx * dx + dy * dy + dz * dz <= rayon * rayon
        }
        Forme3d::Tore { cx, cy, cz, grand_rayon, petit_rayon } => {
            let (dx, dy, dz) = (x - cx as i128, y - cy as i128, z - cz as i128);
            let grand = grand_rayon as i128;
            let petit = petit_rayon as i128;
            let s = dx * dx + dy * dy;
            let d = dz * dz;
            let a = s + grand * grand + d - petit * petit;
            a <= 0 || a * a <= 4 * grand * grand * s
        }
        Forme3d::Labyrinthe { .. } => unreachable!("le labyrinthe se calcule par `labyrinthe_3d`"),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Les obstacles
// ---------------------------------------------------------------------------------------------------------------------

/// Les obstacles du moule : le complément de la cavité dans le cube, en pavés disjoints. Mêmes erreurs que
/// [`libres_3d`].
///
/// # La découpe
///
/// Elle est déterministe, pour que tous écrivent les mêmes lignes. Chaque tranche `z` est découpée en rectangles comme
/// le fait [`crate::moule::obstacles`] dans le plan : on parcourt ses lignes `y` croissantes ; dans chaque ligne, les
/// suites maximales de cases bloquées `[xa, xb]`, par `x` croissant ; une suite **prolonge** le rectangle ouvert de
/// même `(xa, xb)` qui finissait à la ligne `y − 1`, s'il existe, sinon elle en ouvre un ; un rectangle non prolongé
/// est fermé. Puis, de tranche en tranche : un rectangle `(x0, y0, x1, y1)` de la tranche `z` **prolonge** le pavé
/// ouvert de même rectangle qui finissait à la tranche `z − 1`, s'il existe, sinon il en ouvre un ; un pavé non
/// prolongé est fermé. Le résultat est trié par `(z0, y0, x0)` croissants.
///
/// Les pavés sont disjoints, dans le cube, et leur union est exactement le complément de la cavité. Si la cavité est
/// tout le cube, la liste est vide.
pub fn obstacles_3d(
    largeur: usize,
    hauteur: usize,
    profondeur: usize,
    forme: &Forme3d,
) -> Result<Vec<Bloc>, ErreurMoule3d> {
    let libres = libres_3d(largeur, hauteur, profondeur, forme)?;
    Ok(decomposer_3d(largeur, hauteur, profondeur, &libres))
}

/// Découpe en pavés les cases bloquées (les `false`) d'un cube `largeur × hauteur × profondeur` (voir [`obstacles_3d`]).
fn decomposer_3d(largeur: usize, hauteur: usize, profondeur: usize, libres: &[bool]) -> Vec<Bloc> {
    let tranche = largeur * hauteur;
    let mut paves = Vec::new();
    // Les pavés ouverts à la tranche précédente : `(x0, y0, x1, y1, z0)`, par rectangle croissant.
    let mut ouverts: Vec<(usize, usize, usize, usize, usize)> = Vec::new();
    for z in 0..profondeur {
        let rectangles = rectangles_de_la_tranche(largeur, hauteur, &libres[z * tranche..(z + 1) * tranche]);
        let mut suivants = Vec::new();
        for (x0, y0, x1, y1) in rectangles {
            let debut = match ouverts.iter().position(|&(a, b, c, d, _)| (a, b, c, d) == (x0, y0, x1, y1)) {
                Some(i) => ouverts.remove(i).4,
                None => z,
            };
            suivants.push((x0, y0, x1, y1, debut));
        }
        // Ceux que la tranche n'a pas prolongés finissent à la tranche précédente.
        for (x0, y0, x1, y1, debut) in ouverts {
            paves.push(bloc(x0, y0, debut, x1, y1, z - 1));
        }
        ouverts = suivants;
    }
    for (x0, y0, x1, y1, debut) in ouverts {
        paves.push(bloc(x0, y0, debut, x1, y1, profondeur - 1));
    }
    paves.sort_by_key(|p| (p.z0, p.y0, p.x0));
    paves
}

/// Un [`Bloc`] depuis six numéros de cases `usize`.
fn bloc(x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize) -> Bloc {
    Bloc { x0: x0 as u32, y0: y0 as u32, z0: z0 as u32, x1: x1 as u32, y1: y1 as u32, z1: z1 as u32 }
}

/// Découpe en rectangles `(x0, y0, x1, y1)` les cases bloquées (les `false`) d'une tranche `largeur × hauteur`, comme
/// [`crate::moule::obstacles`] découpe le plan (voir [`obstacles_3d`]).
fn rectangles_de_la_tranche(largeur: usize, hauteur: usize, tranche: &[bool]) -> Vec<(usize, usize, usize, usize)> {
    let mut rectangles = Vec::new();
    // Les rectangles ouverts à la ligne précédente : `(xa, xb, y0)`, par `xa` croissant.
    let mut ouverts: Vec<(usize, usize, usize)> = Vec::new();
    for y in 0..hauteur {
        let ligne = &tranche[y * largeur..(y + 1) * largeur];
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
            rectangles.push((xa, debut, xb, y - 1));
        }
        ouverts = suivants;
    }
    for (xa, xb, debut) in ouverts {
        rectangles.push((xa, debut, xb, hauteur - 1));
    }
    rectangles
}

// ---------------------------------------------------------------------------------------------------------------------
// Le texte d'un scénario
// ---------------------------------------------------------------------------------------------------------------------

/// Remplace la ligne `moule …` du texte d'un scénario de colonie par les lignes `obstacle x0 y0 z0 x1 y1 z1` de ses
/// [`obstacles_3d`] ; sans ligne `moule`, rend le texte tel quel, octet pour octet.
///
/// # Le texte
///
/// Une ligne `moule` s'écrit avec des entiers décimaux non signés (des chiffres seulement : ni signe, ni point) :
///
/// ```text
/// moule pave x0 y0 z0 x1 y1 z1
/// moule coquille x0 y0 z0 x1 y1 z1 epaisseur
/// moule boule cx cy cz rayon
/// moule tore cx cy cz grand_rayon petit_rayon
/// moule labyrinthe colonnes rangees etages couloir graine
/// ```
///
/// # La lecture
///
/// Les lignes sont coupées sur `\n` (une fin `\r\n` comprise), rognées, et leurs mots séparés par des blancs, comme
/// [`crate::colonie3d::lire_scenario_colonie3d`] les lit : une ligne vide ou qui commence par `#` est ignorée, mais
/// conservée dans le texte rendu. La ligne `cube <l> <h> <p>` donne les dimensions et doit venir **avant** la ligne
/// `moule` ; la dernière `cube` bien formée (trois entiers décimaux non signés) avant la ligne `moule` l'emporte, et
/// une `cube` mal formée est laissée à la lecture du scénario, qui la refusera (rien d'autre n'est vérifié ici : c'est
/// `lire_scenario_colonie3d` qui refuse, par exemple, une spore dans un obstacle).
///
/// La ligne `moule` est remplacée, à sa place, par ses lignes `obstacle` (une par pavé, dans l'ordre de
/// [`obstacles_3d`]), chacune terminée par un saut de ligne ; le reste du texte est conservé tel quel, y compris les
/// sauts de ligne et une dernière ligne sans saut de ligne.
///
/// # Les erreurs
///
/// Une ligne `moule` est lue dans cet ordre, la première faute l'emporte : une forme inconnue, un nombre de mots faux
/// ou un mot qui n'est pas un entier décimal non signé (`u64` pour la graine, `usize` pour le reste) rend
/// [`ErreurMoule3d::LigneInvalide`] avec son numéro ; une seconde ligne `moule`, [`ErreurMoule3d::MoulesEnDouble`] ;
/// une ligne `moule` avant toute ligne `cube` bien formée, [`ErreurMoule3d::CubeAbsent`] ; puis les erreurs de
/// [`obstacles_3d`] ([`ErreurMoule3d::FormeInvalide`], [`ErreurMoule3d::HorsDuCube`]).
pub fn developper_3d(texte: &str) -> Result<String, ErreurMoule3d> {
    let mut rendu = String::with_capacity(texte.len());
    let mut cube: Option<(usize, usize, usize)> = None;
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
            ["cube", largeur, hauteur, profondeur] => {
                if let (Some(largeur), Some(hauteur), Some(profondeur)) =
                    (nombre(largeur), nombre(hauteur), nombre(profondeur))
                {
                    cube = Some((largeur, hauteur, profondeur));
                }
                rendu.push_str(brute);
            }
            ["moule", parametres @ ..] => {
                let forme = lire_la_forme_3d(parametres).ok_or(ErreurMoule3d::LigneInvalide(numero))?;
                if moule_vu {
                    return Err(ErreurMoule3d::MoulesEnDouble);
                }
                let (largeur, hauteur, profondeur) = cube.ok_or(ErreurMoule3d::CubeAbsent)?;
                for o in obstacles_3d(largeur, hauteur, profondeur, &forme)? {
                    rendu.push_str(&format!("obstacle {} {} {} {} {} {}\n", o.x0, o.y0, o.z0, o.x1, o.y1, o.z1));
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

/// La forme que décrivent les mots d'une ligne `moule` après le mot `moule`, ou `None` si la forme est inconnue, si le
/// nombre de mots est faux ou si un mot n'est pas un entier décimal non signé (`u64` pour la graine, `usize` pour le
/// reste).
fn lire_la_forme_3d(mots: &[&str]) -> Option<Forme3d> {
    match mots {
        ["pave", x0, y0, z0, x1, y1, z1] => Some(Forme3d::Pave {
            x0: nombre(x0)?,
            y0: nombre(y0)?,
            z0: nombre(z0)?,
            x1: nombre(x1)?,
            y1: nombre(y1)?,
            z1: nombre(z1)?,
        }),
        ["coquille", x0, y0, z0, x1, y1, z1, epaisseur] => Some(Forme3d::Coquille {
            x0: nombre(x0)?,
            y0: nombre(y0)?,
            z0: nombre(z0)?,
            x1: nombre(x1)?,
            y1: nombre(y1)?,
            z1: nombre(z1)?,
            epaisseur: nombre(epaisseur)?,
        }),
        ["boule", cx, cy, cz, rayon] => {
            Some(Forme3d::Boule { cx: nombre(cx)?, cy: nombre(cy)?, cz: nombre(cz)?, rayon: nombre(rayon)? })
        }
        ["tore", cx, cy, cz, grand_rayon, petit_rayon] => Some(Forme3d::Tore {
            cx: nombre(cx)?,
            cy: nombre(cy)?,
            cz: nombre(cz)?,
            grand_rayon: nombre(grand_rayon)?,
            petit_rayon: nombre(petit_rayon)?,
        }),
        ["labyrinthe", colonnes, rangees, etages, couloir, graine] => Some(Forme3d::Labyrinthe {
            colonnes: nombre(colonnes)?,
            rangees: nombre(rangees)?,
            etages: nombre(etages)?,
            couloir: nombre(couloir)?,
            graine: nombre(graine)?,
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le labyrinthe
// ---------------------------------------------------------------------------------------------------------------------

/// Les cases de la chambre `(colonne, rangee, etage)` d'un labyrinthe de couloir `couloir` : le pavé
/// `(x0, y0, z0, x1, y1, z1)`, bornes incluses, avec `x0 = colonne q + 1`, `x1 = x0 + couloir − 1` (de même en `y` et
/// en `z`) et `q = couloir + 1`.
pub fn chambre_3d(
    couloir: usize,
    colonne: usize,
    rangee: usize,
    etage: usize,
) -> (usize, usize, usize, usize, usize, usize) {
    let q = couloir + 1;
    let (x0, y0, z0) = (colonne * q + 1, rangee * q + 1, etage * q + 1);
    (x0, y0, z0, x0 + couloir - 1, y0 + couloir - 1, z0 + couloir - 1)
}

/// La case centre de la chambre `(colonne, rangee, etage)` d'un labyrinthe de couloir `couloir` :
/// `(colonne q + 1 + couloir / 2, rangee q + 1 + couloir / 2, etage q + 1 + couloir / 2)` avec `q = couloir + 1` et `/`
/// la division entière.
pub fn centre_de_la_chambre_3d(couloir: usize, colonne: usize, rangee: usize, etage: usize) -> (usize, usize, usize) {
    let q = couloir + 1;
    (colonne * q + 1 + couloir / 2, rangee * q + 1 + couloir / 2, etage * q + 1 + couloir / 2)
}

/// Le labyrinthe parfait, dans un cube `(largeur, hauteur, profondeur)` qui le contient (déjà vérifié par
/// [`libres_3d`]), ses chambres, son couloir et sa graine : la cavité du cube, tranche par tranche.
fn labyrinthe_3d(
    (largeur, hauteur, profondeur): (usize, usize, usize),
    colonnes: usize,
    rangees: usize,
    etages: usize,
    couloir: usize,
    graine: u64,
) -> Vec<bool> {
    let q = couloir + 1;
    let mut libres = vec![false; largeur * hauteur * profondeur];
    // Les chambres, entières.
    for etage in 0..etages {
        for rangee in 0..rangees {
            for colonne in 0..colonnes {
                for z in etage * q + 1..=etage * q + couloir {
                    for y in rangee * q + 1..=rangee * q + couloir {
                        for x in colonne * q + 1..=colonne * q + couloir {
                            libres[(z * hauteur + y) * largeur + x] = true;
                        }
                    }
                }
            }
        }
    }

    // La recherche en profondeur aléatoire, entièrement déterminée par la graine.
    let mut alea = Alea::new(graine);
    let mut visitee = vec![false; colonnes * rangees * etages];
    visitee[0] = true;
    let mut pile = vec![(0usize, 0usize, 0usize)];
    while let Some(&(c, r, e)) = pile.last() {
        // Les voisines non visitées, dans l'ordre (c, r − 1, e), (c + 1, r, e), (c, r + 1, e), (c − 1, r, e),
        // (c, r, e − 1), (c, r, e + 1).
        let mut voisines = Vec::with_capacity(6);
        if r > 0 {
            voisines.push((c, r - 1, e));
        }
        if c + 1 < colonnes {
            voisines.push((c + 1, r, e));
        }
        if r + 1 < rangees {
            voisines.push((c, r + 1, e));
        }
        if c > 0 {
            voisines.push((c - 1, r, e));
        }
        if e > 0 {
            voisines.push((c, r, e - 1));
        }
        if e + 1 < etages {
            voisines.push((c, r, e + 1));
        }
        voisines.retain(|&(vc, vr, ve)| !visitee[(ve * rangees + vr) * colonnes + vc]);
        if voisines.is_empty() {
            pile.pop();
            continue;
        }
        let (vc, vr, ve) = voisines[(alea.suivant() % voisines.len() as u64) as usize];

        // Le passage entre (c, r, e) et (vc, vr, ve) : dans leur mur commun, les `couloir²` cases en face de la chambre.
        if vr == r && ve == e {
            let x = c.max(vc) * q;
            for z in e * q + 1..=e * q + couloir {
                for y in r * q + 1..=r * q + couloir {
                    libres[(z * hauteur + y) * largeur + x] = true;
                }
            }
        } else if vc == c && ve == e {
            let y = r.max(vr) * q;
            for z in e * q + 1..=e * q + couloir {
                for x in c * q + 1..=c * q + couloir {
                    libres[(z * hauteur + y) * largeur + x] = true;
                }
            }
        } else {
            let z = e.max(ve) * q;
            for y in r * q + 1..=r * q + couloir {
                for x in c * q + 1..=c * q + couloir {
                    libres[(z * hauteur + y) * largeur + x] = true;
                }
            }
        }
        visitee[(ve * rangees + vr) * colonnes + vc] = true;
        pile.push((vc, vr, ve));
    }
    libres
}

// ---------------------------------------------------------------------------------------------------------------------
// Le remplissage
// ---------------------------------------------------------------------------------------------------------------------

/// Les cases atteintes par la colonie : `largeur * hauteur * profondeur` booléens, à l'indice
/// `(z * hauteur + y) * largeur + x`, vrai pour la case où **finit** au moins un segment (son arrivée
/// `(x1, y1, z1)`, règle du plancher ; une coordonnée égale à la dimension, sur le bord fermé du cube, compte dans la
/// dernière case).
pub fn cases_atteintes(colonie: &Colonie3d) -> Vec<bool> {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let mut atteintes = vec![false; largeur * hauteur * profondeur];
    for segment in colonie.segments() {
        let x = (segment.x1.floor() as usize).min(largeur - 1);
        let y = (segment.y1.floor() as usize).min(hauteur - 1);
        let z = (segment.z1.floor() as usize).min(profondeur - 1);
        atteintes[(z * hauteur + y) * largeur + x] = true;
    }
    atteintes
}

/// La part de la cavité atteinte : le nombre de cases non bloquées du substrat qui sont atteintes
/// ([`cases_atteintes`]), divisé par le nombre de cases non bloquées, en `f64` (`atteintes as f64 / libres as f64`).
/// Sans moule, la cavité est tout le cube. Une cavité vide (toutes les cases bloquées) rend `0.0`.
pub fn part_remplie(colonie: &Colonie3d) -> f64 {
    let libres = cases_non_bloquees(colonie);
    if libres == 0 {
        return 0.0;
    }
    compter(&cases_atteintes(colonie)) as f64 / libres as f64
}

/// Le nombre de cases du substrat que la colonie n'a pas bloquées : la taille de sa cavité.
fn cases_non_bloquees(colonie: &Colonie3d) -> u64 {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let mut libres = 0u64;
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                if !colonie.substrat().est_bloquee(x, y, z) {
                    libres += 1;
                }
            }
        }
    }
    libres
}

/// Le nombre de cases vraies d'un tableau.
fn compter(cases: &[bool]) -> u64 {
    cases.iter().filter(|&&libre| libre).count() as u64
}

/// Un point de la courbe de remplissage : après `pas` pas, `atteintes` cases de la cavité sur `libres`, soit la part
/// `part` ([`part_remplie`]).
#[derive(Clone, Debug, PartialEq)]
pub struct PointDeCourbe3d {
    /// Le temps de la colonie, en pas.
    pub pas: u32,
    /// Les cases non bloquées atteintes.
    pub atteintes: u64,
    /// Les cases non bloquées.
    pub libres: u64,
    /// `atteintes as f64 / libres as f64`, ou `0.0` si `libres` vaut 0.
    pub part: f64,
}

/// La courbe de remplissage : un point au temps présent de la colonie, puis, tant que son temps est inférieur à
/// `jusqu_au_pas`, un [`Colonie3d::avancer`] suivi d'un point. La colonie est avancée en place ; si son temps atteint
/// déjà `jusqu_au_pas`, la courbe n'a qu'un point.
pub fn courbe_de_remplissage_3d(colonie: &mut Colonie3d, jusqu_au_pas: u32) -> Vec<PointDeCourbe3d> {
    let mut courbe = vec![point_de_courbe_3d(colonie)];
    while colonie.temps() < jusqu_au_pas {
        colonie.avancer();
        courbe.push(point_de_courbe_3d(colonie));
    }
    courbe
}

/// Le point de la courbe au temps présent de la colonie.
fn point_de_courbe_3d(colonie: &Colonie3d) -> PointDeCourbe3d {
    let libres = cases_non_bloquees(colonie);
    let atteintes = compter(&cases_atteintes(colonie));
    let part = if libres == 0 { 0.0 } else { atteintes as f64 / libres as f64 };
    PointDeCourbe3d { pas: colonie.temps(), atteintes, libres, part }
}
