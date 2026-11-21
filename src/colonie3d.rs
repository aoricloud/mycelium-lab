//! La colonie en volume : des apex qui marchent, se nourrissent et se ramifient dans un cube de substrat.
//!
//! C'est le portage en 3D de la marche, de la longueur de pas tirée du substrat, de la consommation, de la
//! ramification, du chimiotropisme, des obstacles et du thigmotropisme (avec la mémoire de direction) de
//! [`crate::simulation`] et [`crate::tropisme`]. L'anastomose n'y est pas (une étape plus tard). Le substrat est un
//! [`Substrat3d`] ; une direction est une [`Direction3d`], qu'on fait tourner d'angles entiers.
//!
//! # Le chimiotropisme en volume
//!
//! Comme en 2D ([`crate::tropisme`]), une pointe se penche vers le côté où il y a plus de nutriments, d'un virage
//! réel `force · s · sinus`, avec `s = n / (n + G₀)` ([`GRADIENT_DE_REFERENCE`], la constante de la 2D), `n` la norme
//! du gradient et `sinus` le sinus de l'angle entre la pointe et le gradient ; le virage est arrondi au sort, d'un
//! seul tirage. En volume, la pointe tourne dans le plan qui contient sa direction et le gradient, vers le gradient.
//! Le calcul, opération par opération, est dans la doc de [`Colonie3d::avancer`].
//!
//! # Les obstacles et le thigmotropisme en volume
//!
//! Un obstacle est un pavé de cases bloquées du [`Substrat3d`] ; il n'est jamais touché par un segment, à tout réglage
//! (le parcours de [`traverser_3d`], règle du plancher). Le gradient y voit un bord sans flux ([`gradient_3d`]).
//! Quand le segment prévu d'une pointe touche une case bloquée, c'est un **contact**, avec la face franchie (son axe
//! normal `n`, un axe de coordonnées). À `thigmotropisme 0` (le défaut), la pointe reste où elle est, avec sa nouvelle
//! direction, et retentera au pas suivant. À `thigmotropisme a > 0`, l'angle de contact `α` est l'angle entre la
//! direction `d` et le plan de la face ; sans trigonométrie, `α < a` se lit `|d_n| < SIN[a]`, `d_n` étant la composante
//! de `d` sur l'axe de la face (le produit scalaire `d · n`) :
//!
//! - `α < a` : la pointe **longe** l'obstacle, dans la direction `g`, projection de `d` sur le plan de la face
//!   (`d` dont la composante `d_n` est remplacée par `0.0`, puis normée) ; elle garde en mémoire `(cap, g)`, `cap`
//!   étant sa direction `d` de ce pas, et refait son pas dans la direction `g` ; si ce pas sort du cube, elle reste là
//!   et oublie ; s'il touche aussi (coin concave), c'est un arrêt contre la seconde face ;
//! - `α ≥ a` : elle **s'arrête et se ramifie** : elle devient inactive (sans mémoire) et, pour `+p` puis `−p`, `p`
//!   étant la projection de sa direction sur le plan de la face (comme `g`), une pointe neuve, sans mémoire, naît à sa
//!   position si un pas plein (`pas`) dans cette direction est libre ; chaque naissance est une ramification.
//!
//! Une pointe qui a une mémoire tire sa marche au hasard (résultat écrasé), puis toujours un écart de deux tirages,
//! et ne subit pas le chimiotropisme ; elle retrouve son cap, à [`MEMOIRE_DE_DIRECTION`] degrés près, quand un pas
//! plein dans la direction `cap` est libre (le bout du mur est passé). Le détail est dans la doc de
//! [`Colonie3d::avancer`]. Sans obstacle, rien de tout cela n'agit : la colonie est celle d'avant, bit pour bit.
//!
//! # Format du scénario
//!
//! Le texte se lit par lignes. Chaque ligne est débarrassée de ses blancs de tête et de queue ; une ligne alors vide,
//! ou dont le premier caractère est `#`, est ignorée. Les mots sont séparés par des blancs (un ou plusieurs, comme
//! [`crate::simulation::lire_scenario`]).
//!
//! - `cube <largeur> <hauteur> <profondeur>` (entiers strictement positifs) : obligatoire et unique ;
//! - `substrat <initiale> <diffusion>` (nombres finis et positifs ou nuls) : obligatoire et unique ; `initiale` est
//!   déposée dans chaque case, `diffusion` est le coefficient `D` du [`Substrat3d`], de `dt = 1` et `dx = 1` : le
//!   nombre de diffusion vaut donc `diffusion`, et la construction refuse `6 · diffusion > 1` ;
//! - `nourriture <x> <y> <z> <quantite>` (entiers, puis un nombre fini et positif ou nul) : se répète, gardé dans
//!   l'ordre du texte ;
//! - `spore <x> <y> <z>` (entiers) : obligatoire et unique ;
//! - `croissance <pas> <intervalle> <angle> <amplitude>` : obligatoire et unique ; `pas` est un nombre fini
//!   strictement positif, `intervalle` un nombre fini strictement positif ou le mot `infini`, `angle` et `amplitude`
//!   des entiers de `0` à `180` ;
//! - `couplage <affinite> <cout>` : obligatoire et unique ; `affinite` est un nombre fini strictement positif, `cout`
//!   un nombre fini positif ou nul ;
//! - `duree <n>` (entier) : obligatoire et unique ;
//! - `chimiotropisme <force>` : facultatif, au plus une fois ; `force` est un nombre fini de `0` à `45`
//!   ([`FORCE_MAXIMALE`], la borne de la 2D), lu comme les autres nombres (`parse::<f64>`, comme
//!   [`crate::simulation::lire_scenario_tropismes`]) ; absent, il vaut `0.0`, qui éteint le chimiotropisme ;
//! - `obstacle <x0> <y0> <z0> <x1> <y1> <z1>` (entiers, `x0 ≤ x1`, `y0 ≤ y1`, `z0 ≤ z1`) : se répète, gardé dans
//!   l'ordre du texte ; le pavé de cases `x0..=x1 × y0..=y1 × z0..=z1` est bloqué (comme `bloc` dans [`crate::cube`]) ;
//! - `thigmotropisme <a>` : facultatif, au plus une fois ; `a` est un **entier** de `0` à `90`
//!   ([`ANGLE_CRITIQUE_MAXIMAL`]), en degrés ; absent, il vaut `0`.
//!
//! Un entier s'écrit en chiffres seulement (ni signe ni point). Une ligne `bloc`, quelle que soit sa forme, donne
//! [`ErreurColonie3d::BlocRefuse`] avec son numéro, dès qu'elle est lue : les obstacles d'une colonie s'écrivent
//! `obstacle`. Une ligne inconnue ou mal formée (nombre de mots, nombre illisible ou hors de ses bornes, obstacle à
//! l'envers) donne [`ErreurColonie3d::LigneInvalide`] avec son numéro (à partir de 1). Une directive unique répétée, ou
//! une seconde ligne `chimiotropisme` ou `thigmotropisme`, donne [`ErreurColonie3d::EnDouble`] (sur la seconde ligne,
//! une fois celle-ci lue sans faute). Une fois tout lu, une directive unique absente donne
//! [`ErreurColonie3d::Manquante`], dans l'ordre `cube`, `substrat`, `spore`, `croissance`, `couplage`, `duree` ; puis
//! une spore hors du cube, puis une nourriture hors du cube, puis un obstacle qui déborde du cube (`x1`, `y1` ou `z1`
//! hors du cube), chacun dans l'ordre du texte, donnent [`ErreurColonie3d::HorsDuCube`] avec le numéro de sa ligne ;
//! puis une spore dans un obstacle, puis une nourriture dans un obstacle (dans l'ordre du texte) donnent
//! [`ErreurColonie3d::DansUnObstacle`] avec le numéro de sa ligne.
//!
//! # Le pas de temps
//!
//! [`Colonie3d::avancer`] fait diffuser le substrat, avance l'horloge, puis fait agir chaque apex actif présent au
//! début du pas, dans l'ordre de son identifiant ; les apex nés pendant le pas attendent le suivant. Le détail, tirage
//! par tirage, est dans la doc de [`Colonie3d::avancer`].
//!
//! # L'écriture canonique
//!
//! [`Colonie3d::ecrire_canonique`] concatène, sans remplissage, en petit-boutiste, les `f64` par `to_bits` :
//!
//! 1. `temps` en `u32` ;
//! 2. le nombre de segments en `u64`, puis chaque segment dans l'ordre de [`Colonie3d::segments`], sur 56 octets :
//!    `apex` (`u32`), `temps` (`u32`), `x0`, `y0`, `z0`, `x1`, `y1`, `z1` ;
//! 3. le nombre d'apex en `u64`, puis chaque apex dans l'ordre de son identifiant, sur 53 octets : `id` (`u32`),
//!    `x`, `y`, `z`, les composantes `x`, `y`, `z` de la direction, `actif` (`u8`, 1 ou 0) ;
//! 4. le nombre de ramifications en `u64` ;
//! 5. [`Substrat3d::ecrire_canonique`] ;
//! 6. seulement si `chimiotropisme != 0.0`, `chimiotropisme.to_bits()` (`u64`, 8 octets) ;
//! 7. seulement s'il y a au moins un obstacle ou si `thigmotropisme != 0`, la queue des obstacles : le nombre
//!    d'obstacles en `u64`, puis chacun dans l'ordre du texte sur 24 octets (`x0`, `y0`, `z0`, `x1`, `y1`, `z1` en
//!    `u32`) ; puis `thigmotropisme` en `u32` ; puis, pour chaque apex dans l'ordre de son identifiant, sa mémoire sur
//!    49 octets : un drapeau (`u8`, 1 si l'apex a une mémoire), puis les composantes `x`, `y`, `z` de `cap`, puis
//!    celles de `g` (six `f64`), toutes `0.0` sans mémoire.
//!
//! Sans chimiotropisme, sans obstacle et à `thigmotropisme 0`, l'écriture est donc celle d'avant, octet pour octet
//! (les empreintes des scènes `colonie-cube` et `colonie-goutte` ne changent pas, ni celle de `colonie-chimio`).

use crate::alea::Alea;
use crate::contact3d::{Axe3d, Traversee3d, traverser_3d};
use crate::croissance::{COS, SIN};
use crate::cube::{Bloc, Depot};
use crate::direction3d::Direction3d;
use crate::substrat3d::{ErreurSubstrat3d, Substrat3d};
use crate::tropisme::{ANGLE_CRITIQUE_MAXIMAL, FORCE_MAXIMALE, GRADIENT_DE_REFERENCE, MEMOIRE_DE_DIRECTION};

/// Le plus grand angle permis pour `angle` et `amplitude`, en degrés.
pub const ANGLE_MAXIMAL: u32 = 180;

/// Le plus grand nombre de cases d'un cube que [`Colonie3d::nouvelle`] accepte (`256³`, 128 Mio de substrat) : au-delà,
/// [`ErreurColonie3d::ReglageInvalide`] de nom `cube`, avant toute allocation.
pub const CASES_MAXIMALES: u64 = 256 * 256 * 256;

/// Le scénario d'une colonie en volume : le cube, son substrat, ses nourritures, la spore, la croissance, le couplage,
/// la durée, le chimiotropisme, les obstacles et le thigmotropisme.
///
/// Contrat : un scénario rendu par [`lire_scenario_colonie3d`] respecte les bornes du format (voir la doc du module) ;
/// sa spore, ses nourritures et ses obstacles sont dans le cube, et ni la spore ni une nourriture n'est dans un
/// obstacle. [`Colonie3d::nouvelle`] revérifie tout cela, sans paniquer, pour un scénario construit à la main.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioColonie3d {
    /// Largeur du cube, en cases (`x`).
    pub largeur: u32,
    /// Hauteur du cube, en cases (`y`).
    pub hauteur: u32,
    /// Profondeur du cube, en cases (`z`).
    pub profondeur: u32,
    /// Quantité de substrat déposée dans chaque case au départ.
    pub substrat_initial: f64,
    /// Coefficient de diffusion du substrat (le nombre de diffusion, `dt = dx = 1`).
    pub diffusion: f64,
    /// Les nourritures, dans l'ordre du texte, déposées après le remplissage uniforme.
    pub depots: Vec<Depot>,
    /// La case de la spore `(x, y, z)`.
    pub spore: (u32, u32, u32),
    /// Longueur de pas maximale d'un apex, en cases.
    pub pas: f64,
    /// Nombre moyen de pas entre deux ramifications d'un apex ; `f64::INFINITY` : aucune ramification.
    pub intervalle: f64,
    /// L'écart d'une branche à la direction de sa mère, en degrés.
    pub angle: u32,
    /// Le plus grand virage d'un pas de marche, en degrés.
    pub amplitude: u32,
    /// Affinité de la colonie pour le substrat.
    pub affinite: f64,
    /// Coût en substrat d'une unité de longueur d'hyphe.
    pub cout: f64,
    /// Durée de la simulation, en pas.
    pub duree: u32,
    /// Force du chimiotropisme, en degrés, dans `0.0..=45.0` ; `0.0` (le défaut, ligne absente) l'éteint.
    pub chimiotropisme: f64,
    /// Les obstacles, dans l'ordre du texte : des pavés de cases bloquées, bornes comprises.
    pub obstacles: Vec<Bloc>,
    /// L'angle critique du thigmotropisme, en degrés entiers, dans `0..=90` ; `0` (le défaut, ligne absente) : une
    /// pointe qui touche un obstacle reste où elle est.
    pub thigmotropisme: u32,
}

/// Erreur de lecture ou de construction d'une colonie en volume.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurColonie3d {
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne (à partir de 1).
    LigneInvalide(usize),
    /// Une directive unique est répétée ; porte son nom.
    EnDouble(&'static str),
    /// Une directive unique est absente ; porte son nom.
    Manquante(&'static str),
    /// La spore, une nourriture ou un obstacle sort du cube ; porte le numéro de ligne (`0` pour un scénario fait à la
    /// main).
    HorsDuCube(usize),
    /// Une ligne `bloc` : les obstacles d'une colonie s'écrivent `obstacle` ; porte le numéro de ligne.
    BlocRefuse(usize),
    /// La spore ou une nourriture est dans un obstacle ; porte le numéro de ligne (`0` pour un scénario fait à la
    /// main).
    DansUnObstacle(usize),
    /// Un réglage hors de ses bornes dans un scénario fait à la main ; porte le nom de la directive.
    ReglageInvalide(&'static str),
    /// La construction du substrat en volume a échoué.
    Substrat(ErreurSubstrat3d),
}

/// Un apex : une pointe de la colonie, à une position réelle, avec sa direction.
///
/// Contrat : `id` est son rang de naissance (0, 1, 2… sans trou) ; la position est toujours dans
/// `[0, largeur) × [0, hauteur) × [0, profondeur)`, jamais dans une case bloquée. Un apex n'est rendu inactif que par
/// un arrêt contre un obstacle ; un apex inactif n'a pas de mémoire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apex3d {
    /// Identifiant de l'apex.
    pub id: u32,
    /// Position en `x`.
    pub x: f64,
    /// Position en `y`.
    pub y: f64,
    /// Position en `z`.
    pub z: f64,
    /// Direction de l'apex.
    pub direction: Direction3d,
    /// Vrai si l'apex agit encore.
    pub actif: bool,
    /// La mémoire d'une pointe qui longe un obstacle : `(cap, g)`, sa direction au contact et sa direction de
    /// glissement ; `None` sinon.
    pub memoire: Option<(Direction3d, Direction3d)>,
}

/// Un segment d'hyphe laissé par un apex pendant un pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment3d {
    /// Identifiant de l'apex qui l'a laissé.
    pub apex: u32,
    /// Le pas où il a été laissé (`1` pour le premier pas).
    pub temps: u32,
    /// Départ en `x`.
    pub x0: f64,
    /// Départ en `y`.
    pub y0: f64,
    /// Départ en `z`.
    pub z0: f64,
    /// Arrivée en `x`.
    pub x1: f64,
    /// Arrivée en `y`.
    pub y1: f64,
    /// Arrivée en `z`.
    pub z1: f64,
}

impl Segment3d {
    /// La longueur du segment : `sqrt(dx * dx + dy * dy + dz * dz)`, dans cet ordre, avec `dx = x1 - x0`, etc.
    pub fn longueur(&self) -> f64 {
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        let dz = self.z1 - self.z0;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Lit un entier décimal non signé (chiffres seulement).
fn entier(mot: &str) -> Option<u32> {
    if mot.is_empty() || !mot.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    mot.parse().ok()
}

/// Lit un nombre fini et positif ou nul.
fn nombre(mot: &str) -> Option<f64> {
    let valeur: f64 = mot.parse().ok()?;
    if valeur.is_finite() && valeur >= 0.0 { Some(valeur) } else { None }
}

/// Lit un nombre fini et strictement positif.
fn positif(mot: &str) -> Option<f64> {
    nombre(mot).filter(|&valeur| valeur > 0.0)
}

/// Lit un angle entier de `0` à [`ANGLE_MAXIMAL`].
fn angle(mot: &str) -> Option<u32> {
    entier(mot).filter(|&valeur| valeur <= ANGLE_MAXIMAL)
}

/// Lit un scénario de colonie en volume depuis son texte (format et ordre des erreurs dans la doc du module).
pub fn lire_scenario_colonie3d(texte: &str) -> Result<ScenarioColonie3d, ErreurColonie3d> {
    let mut dimensions: Option<(u32, u32, u32)> = None;
    let mut substrat: Option<(f64, f64)> = None;
    let mut spore: Option<((u32, u32, u32), usize)> = None;
    let mut croissance: Option<(f64, f64, u32, u32)> = None;
    let mut couplage: Option<(f64, f64)> = None;
    let mut duree: Option<u32> = None;
    let mut chimiotropisme: Option<f64> = None;
    let mut thigmotropisme: Option<u32> = None;
    let mut depots: Vec<Depot> = Vec::new();
    let mut depots_lignes: Vec<usize> = Vec::new();
    let mut obstacles: Vec<Bloc> = Vec::new();
    let mut obstacles_lignes: Vec<usize> = Vec::new();

    for (indice, brute) in texte.lines().enumerate() {
        let numero = indice + 1;
        let ligne = brute.trim();
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let mots: Vec<&str> = ligne.split_whitespace().collect();
        let invalide = ErreurColonie3d::LigneInvalide(numero);
        match mots.as_slice() {
            ["bloc", ..] => return Err(ErreurColonie3d::BlocRefuse(numero)),
            ["cube", l, h, p] => {
                let (Some(l), Some(h), Some(p)) = (entier(l), entier(h), entier(p)) else { return Err(invalide) };
                if l == 0 || h == 0 || p == 0 {
                    return Err(invalide);
                }
                if dimensions.is_some() {
                    return Err(ErreurColonie3d::EnDouble("cube"));
                }
                dimensions = Some((l, h, p));
            }
            ["substrat", initiale, diffusion] => {
                let (Some(initiale), Some(diffusion)) = (nombre(initiale), nombre(diffusion)) else {
                    return Err(invalide);
                };
                if substrat.is_some() {
                    return Err(ErreurColonie3d::EnDouble("substrat"));
                }
                substrat = Some((initiale, diffusion));
            }
            ["nourriture", x, y, z, q] => {
                let (Some(x), Some(y), Some(z), Some(quantite)) = (entier(x), entier(y), entier(z), nombre(q)) else {
                    return Err(invalide);
                };
                depots.push(Depot { x, y, z, quantite });
                depots_lignes.push(numero);
            }
            ["spore", x, y, z] => {
                let (Some(x), Some(y), Some(z)) = (entier(x), entier(y), entier(z)) else { return Err(invalide) };
                if spore.is_some() {
                    return Err(ErreurColonie3d::EnDouble("spore"));
                }
                spore = Some(((x, y, z), numero));
            }
            ["croissance", pas, intervalle, a, amplitude] => {
                let intervalle = if *intervalle == "infini" { Some(f64::INFINITY) } else { positif(intervalle) };
                let (Some(pas), Some(intervalle), Some(a), Some(amplitude)) =
                    (positif(pas), intervalle, angle(a), angle(amplitude))
                else {
                    return Err(invalide);
                };
                if croissance.is_some() {
                    return Err(ErreurColonie3d::EnDouble("croissance"));
                }
                croissance = Some((pas, intervalle, a, amplitude));
            }
            ["couplage", affinite, cout] => {
                let (Some(affinite), Some(cout)) = (positif(affinite), nombre(cout)) else { return Err(invalide) };
                if couplage.is_some() {
                    return Err(ErreurColonie3d::EnDouble("couplage"));
                }
                couplage = Some((affinite, cout));
            }
            ["duree", n] => {
                let Some(n) = entier(n) else { return Err(invalide) };
                if duree.is_some() {
                    return Err(ErreurColonie3d::EnDouble("duree"));
                }
                duree = Some(n);
            }
            ["chimiotropisme", force] => {
                let Some(force) = nombre(force).filter(|&force| force <= FORCE_MAXIMALE) else { return Err(invalide) };
                if chimiotropisme.is_some() {
                    return Err(ErreurColonie3d::EnDouble("chimiotropisme"));
                }
                chimiotropisme = Some(force);
            }
            ["obstacle", x0, y0, z0, x1, y1, z1] => {
                let (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) =
                    (entier(x0), entier(y0), entier(z0), entier(x1), entier(y1), entier(z1))
                else {
                    return Err(invalide);
                };
                if x0 > x1 || y0 > y1 || z0 > z1 {
                    return Err(invalide);
                }
                obstacles.push(Bloc { x0, y0, z0, x1, y1, z1 });
                obstacles_lignes.push(numero);
            }
            ["thigmotropisme", a] => {
                let Some(a) = entier(a).filter(|&a| f64::from(a) <= ANGLE_CRITIQUE_MAXIMAL) else {
                    return Err(invalide);
                };
                if thigmotropisme.is_some() {
                    return Err(ErreurColonie3d::EnDouble("thigmotropisme"));
                }
                thigmotropisme = Some(a);
            }
            _ => return Err(invalide),
        }
    }

    let (largeur, hauteur, profondeur) = dimensions.ok_or(ErreurColonie3d::Manquante("cube"))?;
    let (substrat_initial, diffusion) = substrat.ok_or(ErreurColonie3d::Manquante("substrat"))?;
    let (spore, spore_ligne) = spore.ok_or(ErreurColonie3d::Manquante("spore"))?;
    let (pas, intervalle, angle, amplitude) = croissance.ok_or(ErreurColonie3d::Manquante("croissance"))?;
    let (affinite, cout) = couplage.ok_or(ErreurColonie3d::Manquante("couplage"))?;
    let duree = duree.ok_or(ErreurColonie3d::Manquante("duree"))?;

    let dans_le_cube = |x: u32, y: u32, z: u32| x < largeur && y < hauteur && z < profondeur;
    if !dans_le_cube(spore.0, spore.1, spore.2) {
        return Err(ErreurColonie3d::HorsDuCube(spore_ligne));
    }
    for (depot, &numero) in depots.iter().zip(&depots_lignes) {
        if !dans_le_cube(depot.x, depot.y, depot.z) {
            return Err(ErreurColonie3d::HorsDuCube(numero));
        }
    }
    for (obstacle, &numero) in obstacles.iter().zip(&obstacles_lignes) {
        if !dans_le_cube(obstacle.x1, obstacle.y1, obstacle.z1) {
            return Err(ErreurColonie3d::HorsDuCube(numero));
        }
    }
    if dans_un_obstacle(&obstacles, spore) {
        return Err(ErreurColonie3d::DansUnObstacle(spore_ligne));
    }
    for (depot, &numero) in depots.iter().zip(&depots_lignes) {
        if dans_un_obstacle(&obstacles, (depot.x, depot.y, depot.z)) {
            return Err(ErreurColonie3d::DansUnObstacle(numero));
        }
    }

    Ok(ScenarioColonie3d {
        largeur,
        hauteur,
        profondeur,
        substrat_initial,
        diffusion,
        depots,
        spore,
        pas,
        intervalle,
        angle,
        amplitude,
        affinite,
        cout,
        duree,
        chimiotropisme: chimiotropisme.unwrap_or(0.0),
        obstacles,
        thigmotropisme: thigmotropisme.unwrap_or(0),
    })
}

/// Vrai si la case `(x, y, z)` est dans l'un des pavés (bornes comprises).
fn dans_un_obstacle(obstacles: &[Bloc], (x, y, z): (u32, u32, u32)) -> bool {
    obstacles.iter().any(|o| o.x0 <= x && x <= o.x1 && o.y0 <= y && y <= o.y1 && o.z0 <= z && z <= o.z1)
}

/// Vérifie un scénario fait à la main : les mêmes bornes que la lecture, sans numéro de ligne.
fn verifier(scenario: &ScenarioColonie3d) -> Result<(), ErreurColonie3d> {
    let fini_positif = |v: f64| v.is_finite() && v >= 0.0;
    let s = scenario;
    // En `u128` : trois `u32` ne débordent pas.
    let cases = u128::from(s.largeur) * u128::from(s.hauteur) * u128::from(s.profondeur);
    if cases > u128::from(CASES_MAXIMALES) {
        return Err(ErreurColonie3d::ReglageInvalide("cube"));
    }
    if !fini_positif(s.substrat_initial) || !fini_positif(s.diffusion) {
        return Err(ErreurColonie3d::ReglageInvalide("substrat"));
    }
    let intervalle_permis = s.intervalle == f64::INFINITY || (s.intervalle.is_finite() && s.intervalle > 0.0);
    if !(s.pas.is_finite() && s.pas > 0.0) || !intervalle_permis || s.angle > ANGLE_MAXIMAL {
        return Err(ErreurColonie3d::ReglageInvalide("croissance"));
    }
    if s.amplitude > ANGLE_MAXIMAL {
        return Err(ErreurColonie3d::ReglageInvalide("croissance"));
    }
    if !(s.affinite.is_finite() && s.affinite > 0.0) || !fini_positif(s.cout) {
        return Err(ErreurColonie3d::ReglageInvalide("couplage"));
    }
    if !fini_positif(s.chimiotropisme) || s.chimiotropisme > FORCE_MAXIMALE {
        return Err(ErreurColonie3d::ReglageInvalide("chimiotropisme"));
    }
    if s.depots.iter().any(|d| !fini_positif(d.quantite)) {
        return Err(ErreurColonie3d::ReglageInvalide("nourriture"));
    }
    let dans_le_cube = |x: u32, y: u32, z: u32| x < s.largeur && y < s.hauteur && z < s.profondeur;
    if !dans_le_cube(s.spore.0, s.spore.1, s.spore.2) || s.depots.iter().any(|d| !dans_le_cube(d.x, d.y, d.z)) {
        return Err(ErreurColonie3d::HorsDuCube(0));
    }
    if f64::from(s.thigmotropisme) > ANGLE_CRITIQUE_MAXIMAL {
        return Err(ErreurColonie3d::ReglageInvalide("thigmotropisme"));
    }
    if s.obstacles.iter().any(|o| o.x0 > o.x1 || o.y0 > o.y1 || o.z0 > o.z1) {
        return Err(ErreurColonie3d::ReglageInvalide("obstacle"));
    }
    if s.obstacles.iter().any(|o| !dans_le_cube(o.x1, o.y1, o.z1)) {
        return Err(ErreurColonie3d::HorsDuCube(0));
    }
    if dans_un_obstacle(&s.obstacles, s.spore)
        || s.depots.iter().any(|d| dans_un_obstacle(&s.obstacles, (d.x, d.y, d.z)))
    {
        return Err(ErreurColonie3d::DansUnObstacle(0));
    }
    Ok(())
}

/// Le gradient du substrat dans la case `(x, y, z)`, par différences centrées sur ses six voisines.
///
/// Contrat : avec `c = substrat.valeur(x, y, z)` et `v(…)` la valeur d'une voisine, ou `c` si elle sort du cube **ou
/// si elle est bloquée** (aucun flux, comme dans [`Substrat3d::pas`] et comme [`crate::substrat::Substrat::gradient`]
/// en 2D), le résultat est `((v(x+1) − v(x−1)) * 0.5, (v(y+1) − v(y−1)) * 0.5, (v(z+1) − v(z−1)) * 0.5)` : la
/// soustraction d'abord, puis le produit par `0.5` (qui donne, au bit près, la division par 2). Le gradient d'une case
/// bloquée est `(0.0, 0.0, 0.0)`, comme en 2D. Sans case bloquée, le résultat est celui de l'étape 22, bit pour bit.
/// Rend `None` si la case est hors du cube, sans paniquer (la colonie ne l'appelle que sur la case d'un apex, toujours
/// dans le cube).
pub fn gradient_3d(substrat: &Substrat3d, x: usize, y: usize, z: usize) -> Option<(f64, f64, f64)> {
    let (largeur, hauteur, profondeur) = (substrat.largeur(), substrat.hauteur(), substrat.profondeur());
    if x >= largeur || y >= hauteur || z >= profondeur {
        return None;
    }
    if substrat.est_bloquee(x, y, z) {
        return Some((0.0, 0.0, 0.0));
    }
    let c = substrat.valeur(x, y, z);
    // Un voisin hors du cube ou bloqué vaut la case elle-même (aucun flux), comme dans `Substrat::gradient`.
    let v = |vx: usize, vy: usize, vz: usize| {
        if substrat.est_bloquee(vx, vy, vz) { c } else { substrat.valeur(vx, vy, vz) }
    };
    let gauche = if x == 0 { c } else { v(x - 1, y, z) };
    let droite = if x + 1 == largeur { c } else { v(x + 1, y, z) };
    let haut = if y == 0 { c } else { v(x, y - 1, z) };
    let bas = if y + 1 == hauteur { c } else { v(x, y + 1, z) };
    let devant = if z == 0 { c } else { v(x, y, z - 1) };
    let derriere = if z + 1 == profondeur { c } else { v(x, y, z + 1) };
    Some(((droite - gauche) * 0.5, (bas - haut) * 0.5, (derriere - devant) * 0.5))
}

/// La nouvelle direction d'une pointe de direction `d` qui se penche vers le gradient `g = (gx, gy, gz)`, de force
/// `force` (en degrés) : le chimiotropisme de [`crate::tropisme::tourner_chimiotropique`], porté en volume.
///
/// Contrat, dans cet ordre d'opérations :
///
/// 1. si `force ≤ 0` : rend `d`, sans tirage ;
/// 2. `n = sqrt((gx * gx + gy * gy) + gz * gz)` ; si `n == 0` : rend `d`, sans tirage ;
/// 3. `dg = d.x * gx + d.y * gy + d.z * gz` (l'ordre de [`Direction3d::scalaire`]), puis `p = g − dg · d`
///    composante par composante (`px = gx - dg * d.x`, …) : la part du gradient perpendiculaire à `d` ;
///    `m = sqrt((px * px + py * py) + pz * pz)` ;
/// 4. `sinus = (m / n).min(1.0)` (le sinus de l'angle entre `d` et `g`, dans `[0, 1]`) ;
///    `s = n / (n + G₀)` avec `G₀ =` [`GRADIENT_DE_REFERENCE`] (`0.01`, la constante de la 2D) ;
///    `virage = (force * s) * sinus`, en degrés, dans `[0, force]` ;
/// 5. l'arrondi au sort de la 2D : `t = floor(virage)`, `frac = virage − t`, un tirage `alea.suivant_f64()`,
///    toujours (même pour `sinus = 0`) ; s'il est `< frac`, `t` croît de 1 ; `t` est ensuite borné à `180` ;
/// 6. si `t == 0` ou `m == 0` : rend `d` ; sinon, avec `a = p / m` (`ax = px / m`, …), rend
///    `Direction3d::nouvelle(COS[t] * d.x + SIN[t] * ax, …)` (chaque produit, puis leur somme), ou `d` si cette
///    construction échouait. La pointe tourne de `t` degrés dans le plan `(d, g)`, vers `g`.
///
/// Ne panique pas.
pub fn tourner_chimiotropique_3d(d: Direction3d, g: (f64, f64, f64), force: f64, alea: &mut Alea) -> Direction3d {
    if force <= 0.0 {
        return d;
    }
    let (gx, gy, gz) = g;
    let n = ((gx * gx + gy * gy) + gz * gz).sqrt();
    if n == 0.0 {
        return d;
    }
    let dg = d.x() * gx + d.y() * gy + d.z() * gz;
    let px = gx - dg * d.x();
    let py = gy - dg * d.y();
    let pz = gz - dg * d.z();
    let m = ((px * px + py * py) + pz * pz).sqrt();
    let sinus = (m / n).min(1.0);
    let s = n / (n + GRADIENT_DE_REFERENCE);
    let virage = (force * s) * sinus;
    let bas = virage.floor();
    let frac = virage - bas;
    let mut t = bas as usize;
    if alea.suivant_f64() < frac {
        t += 1;
    }
    t = t.min(180);
    if t == 0 || m == 0.0 {
        return d;
    }
    let ax = px / m;
    let ay = py / m;
    let az = pz / m;
    let cx = COS[t] * d.x() + SIN[t] * ax;
    let cy = COS[t] * d.y() + SIN[t] * ay;
    let cz = COS[t] * d.z() + SIN[t] * az;
    Direction3d::nouvelle(cx, cy, cz).unwrap_or(d)
}

/// L'arrivée d'un pas de `longueur` dans la direction `d` depuis l'apex : `x + longueur * d.x()`, et de même en `y`
/// et `z`.
fn arrivee(apex: &Apex3d, longueur: f64, d: Direction3d) -> (f64, f64, f64) {
    (apex.x + longueur * d.x(), apex.y + longueur * d.y(), apex.z + longueur * d.z())
}

/// La composante de `d` sur l'axe `axe` : le produit scalaire `d · n` avec `n` le vecteur unitaire de l'axe.
fn composante(d: Direction3d, axe: Axe3d) -> f64 {
    match axe {
        Axe3d::X => d.x(),
        Axe3d::Y => d.y(),
        Axe3d::Z => d.z(),
    }
}

/// `d` dont la composante sur `axe` est remplacée par `0.0` : la projection de `d` sur le plan de la face, non normée.
fn sans_composante(d: Direction3d, axe: Axe3d) -> (f64, f64, f64) {
    match axe {
        Axe3d::X => (0.0, d.y(), d.z()),
        Axe3d::Y => (d.x(), 0.0, d.z()),
        Axe3d::Z => (d.x(), d.y(), 0.0),
    }
}

/// La direction de glissement : la projection de `d` sur le plan de la face d'axe `axe`, normée ; `None` si elle est
/// nulle (`d` perpendiculaire à la face).
fn projection(d: Direction3d, axe: Axe3d) -> Option<Direction3d> {
    let (vx, vy, vz) = sans_composante(d, axe);
    Direction3d::nouvelle(vx, vy, vz).ok()
}

/// Les deux directions des naissances d'un arrêt contre la face d'axe `axe`, avec la direction `d` : `(plus, moins)`
/// de l'étape 8 de [`Colonie3d::avancer`].
fn branches_d_arret(d: Direction3d, axe: Axe3d) -> (Direction3d, Direction3d) {
    let (vx, vy, vz) = sans_composante(d, axe);
    if let (Ok(plus), Ok(moins)) = (Direction3d::nouvelle(vx, vy, vz), Direction3d::nouvelle(-vx, -vy, -vz)) {
        return (plus, moins);
    }
    let (ex, ey) = if axe == Axe3d::X { (0.0, 1.0) } else { (1.0, 0.0) };
    // Deux vecteurs unitaires, finis et non nuls : `nouvelle` les accepte toujours ; `d` ne sert qu'à ne pas paniquer.
    let plus = Direction3d::nouvelle(ex, ey, 0.0).unwrap_or(d);
    let moins = Direction3d::nouvelle(-ex, -ey, 0.0).unwrap_or(d);
    (plus, moins)
}

/// Une colonie en volume : ses apex, ses segments, son substrat, son aléa et son horloge.
///
/// Contrat : l'état complet est déterminé par le scénario, la graine et le nombre de pas joués ; mêmes entrées, mêmes
/// octets ([`Colonie3d::ecrire_canonique`]). Aucun apex n'est retiré : il y en a toujours `1 + ramifications`, actifs
/// ou non (une naissance contre un obstacle est une ramification).
#[derive(Clone, Debug)]
pub struct Colonie3d {
    scenario: ScenarioColonie3d,
    substrat: Substrat3d,
    alea: Alea,
    apex: Vec<Apex3d>,
    segments: Vec<Segment3d>,
    temps: u32,
    nb_ramifications: u64,
}

impl Colonie3d {
    /// Construit la colonie au temps 0.
    ///
    /// Le substrat est un [`Substrat3d`] du cube, de coefficient `diffusion`, `dt = 1.0` et `dx = 1.0` ; la quantité
    /// `substrat_initial` est déposée dans chaque case si elle est strictement positive, puis chaque obstacle est
    /// bloqué dans l'ordre ([`Substrat3d::bloquer`], qui retire ce que ses cases contenaient : la masse totale baisse
    /// d'autant), puis chaque nourriture est déposée dans l'ordre — l'ordre de [`crate::cube::Cube::nouveau`]. Un seul
    /// apex, d'identifiant `0`, actif, sans mémoire, part du centre de la case de la spore
    /// `(sx + 0.5, sy + 0.5, sz + 0.5)`, dans la direction [`Direction3d::au_hasard`] tirée de `Alea::new(graine)`.
    ///
    /// Contrat, dans cet ordre : rend [`ErreurColonie3d::ReglageInvalide`] pour un réglage hors des bornes du format ou
    /// un cube de plus de [`CASES_MAXIMALES`] cases ; [`ErreurColonie3d::HorsDuCube`] (de numéro `0`) pour une spore ou
    /// une nourriture hors du cube (une dimension nulle en fait partie : la spore n'y tient pas) ;
    /// [`ErreurColonie3d::ReglageInvalide`] de nom `thigmotropisme` pour un angle de plus de `90`, puis de nom
    /// `obstacle` pour un obstacle à l'envers (`x0 > x1`, `y0 > y1` ou `z0 > z1`) ; [`ErreurColonie3d::HorsDuCube`] (de
    /// numéro `0`) pour un obstacle qui déborde du cube ; [`ErreurColonie3d::DansUnObstacle`] (de numéro `0`) pour une
    /// spore ou une nourriture dans un obstacle ; enfin [`ErreurColonie3d::Substrat`] pour un refus de
    /// [`Substrat3d::nouveau`] (`6 · diffusion > 1`). Ne panique pas.
    pub fn nouvelle(scenario: &ScenarioColonie3d, graine: u64) -> Result<Colonie3d, ErreurColonie3d> {
        verifier(scenario)?;
        let (largeur, hauteur, profondeur) =
            (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
        let mut substrat = Substrat3d::nouveau(largeur, hauteur, profondeur, scenario.diffusion, 1.0, 1.0)
            .map_err(ErreurColonie3d::Substrat)?;
        if scenario.substrat_initial > 0.0 {
            for z in 0..profondeur {
                for y in 0..hauteur {
                    for x in 0..largeur {
                        substrat.deposer(x, y, z, scenario.substrat_initial);
                    }
                }
            }
        }
        for o in &scenario.obstacles {
            let (x0, y0, z0) = (o.x0 as usize, o.y0 as usize, o.z0 as usize);
            substrat.bloquer(x0, y0, z0, o.x1 as usize, o.y1 as usize, o.z1 as usize);
        }
        for depot in &scenario.depots {
            substrat.deposer(depot.x as usize, depot.y as usize, depot.z as usize, depot.quantite);
        }
        let mut alea = Alea::new(graine);
        let direction = Direction3d::au_hasard(&mut alea);
        let premier = Apex3d {
            id: 0,
            x: scenario.spore.0 as f64 + 0.5,
            y: scenario.spore.1 as f64 + 0.5,
            z: scenario.spore.2 as f64 + 0.5,
            direction,
            actif: true,
            memoire: None,
        };
        Ok(Colonie3d {
            scenario: scenario.clone(),
            substrat,
            alea,
            apex: vec![premier],
            segments: Vec::new(),
            temps: 0,
            nb_ramifications: 0,
        })
    }

    /// Avance la colonie d'un pas.
    ///
    /// Contrat, dans cet ordre :
    ///
    /// 1. le substrat diffuse ([`Substrat3d::pas`]), puis `temps` croît de 1 ;
    /// 2. chaque apex actif présent au début du pas, dans l'ordre de son identifiant (les naissances du pas attendent
    ///    le suivant) :
    ///    1. tourne : `theta = alea.suivant() % (amplitude + 1)`, puis `phi = alea.suivant() % 360`, et
    ///       `direction = direction.tourner(theta, phi)` — deux tirages toujours, même à `amplitude = 0` ;
    ///    2. lit `s`, le substrat de sa case (`x`, `y`, `z` tronqués vers le bas), et en tire
    ///       `longueur = si s ≤ 0 { 0 } sinon { pas * s / (affinite + s) }`, ramenée à `s / cout` si `cout > 0` et
    ///       `cout * longueur > s` ;
    ///    3. s'il a une mémoire `(cap, g)` : tire `theta = alea.suivant() % (MEMOIRE_DE_DIRECTION + 1)` puis
    ///       `phi = alea.suivant() % 360` (deux tirages toujours), forme `candidat = cap.tourner(theta, phi)` ; si un
    ///       pas de longueur `pas` dans la direction `cap` est libre **et** qu'un pas de longueur `longueur` dans la
    ///       direction `candidat` l'est aussi (un pas est libre si son arrivée, calculée comme à l'étape 5, est dans
    ///       `[0, largeur) × [0, hauteur) × [0, profondeur)` et si [`traverser_3d`] du départ à l'arrivée rend
    ///       [`Traversee3d::Libre`]), `direction = candidat` et la mémoire s'efface ; sinon `direction = g`. Le
    ///       résultat de l'étape 1 est donc écrasé, et le chimiotropisme ne s'applique pas ;
    ///    4. sinon, si `chimiotropisme > 0`, se penche vers le gradient du substrat ([`tourner_chimiotropique_3d`], sur
    ///       la case de l'étape 2 et la direction de l'étape 1), même si `longueur` est nulle : aucun tirage si le
    ///       gradient est nul, un tirage `alea.suivant_f64()` sinon ; à `chimiotropisme = 0`, rien, aucun tirage ;
    ///    5. calcule l'arrivée `x + longueur * d.x()`, et de même en `y` et `z`, avec `d` la direction des étapes 3-4 ;
    ///    6. si `longueur ≤ 0` ou si l'arrivée sort de `[0, largeur) × [0, hauteur) × [0, profondeur)`, reste où il est
    ///       avec sa nouvelle direction et perd sa mémoire, sans consommer, laisser, se ramifier ni tirer davantage ;
    ///    7. s'il y a au moins un obstacle et que [`traverser_3d`] du départ à l'arrivée rend un contact, d'axe `n` :
    ///       - à `thigmotropisme 0`, il reste où il est avec sa direction `d`, sans rien changer d'autre ni tirer ;
    ///       - sinon, avec `d_n` la composante de `d` sur l'axe `n` : si `|d_n| < SIN[thigmotropisme]` et que la
    ///         projection `g = Direction3d::nouvelle(d avec sa composante sur n remplacée par 0.0)` existe, il
    ///         **longe** : sa mémoire devient `(d, g)`, sa direction `g`, et l'arrivée est recalculée comme à l'étape 5
    ///         avec `g` (même `longueur`) ; si elle sort du cube, il reste où il est, direction `g`, et perd sa
    ///         mémoire (fin de son pas) ; si [`traverser_3d`] y rend encore un contact (coin concave), c'est un arrêt
    ///         contre ce second contact, avec la direction `g` ;
    ///       - sinon c'est un arrêt contre ce contact, avec la direction `d` ;
    ///    8. un **arrêt** contre un contact d'axe `n`, avec la direction `d` : l'apex devient inactif et perd sa
    ///       mémoire ; avec `v = d` dont la composante sur `n` est remplacée par `0.0`, `plus = nouvelle(v)` et
    ///       `moins = nouvelle(−v)` (chaque composante négée) ; si `v` est nul, `plus` est le premier axe du plan de la
    ///       face dans l'ordre `x`, `y`, `z` (`y` pour `n = x`, `x` sinon), en sens positif, et `moins` son opposé ;
    ///       pour `plus` puis `moins`, si un pas de longueur `pas` dans cette direction est libre (comme à l'étape 3),
    ///       un apex neuf, actif, sans mémoire, d'identifiant suivant, naît à la position de l'apex arrêté, et les
    ///       ramifications croissent de 1. Aucun tirage ; fin du pas de l'apex ;
    ///    9. sinon retire `cout * longueur` de sa case de départ ([`Substrat3d::retirer`], qui ne retire jamais plus
    ///       que la case ne contient), laisse un [`Segment3d`] du départ à l'arrivée, et s'y déplace ;
    ///    10. si `intervalle` est fini, tire `alea.suivant_f64()` ; s'il est `< 1.0 / intervalle`, tire
    ///        `phi = alea.suivant() % 360`, et un apex neuf, actif, sans mémoire, d'identifiant suivant, naît à
    ///        l'arrivée dans la direction `d.tourner(angle, phi)`, `d` étant la direction du segment laissé (`g` pour
    ///        un apex qui longe, qui garde sa mémoire) ; les ramifications croissent de 1.
    ///
    /// Sans obstacle, l'étape 7 ne lit rien, aucun apex n'a de mémoire, et le pas est celui de l'étape 22, bit pour
    /// bit.
    pub fn avancer(&mut self) {
        self.substrat.pas();
        self.temps += 1;
        let temps = self.temps;
        let nb_apex_au_debut = self.apex.len();

        let s = &self.scenario;
        let (pas, affinite, cout, force) = (s.pas, s.affinite, s.cout, s.chimiotropisme);
        let amplitude = u64::from(s.amplitude);
        let angle = s.angle;
        let probabilite = if s.intervalle.is_finite() { Some(1.0 / s.intervalle) } else { None };
        let a_des_obstacles = !s.obstacles.is_empty();
        // Borné à 90 par la lecture et par `verifier`.
        let angle_critique = s.thigmotropisme as usize;

        for i in 0..nb_apex_au_debut {
            if !self.apex[i].actif {
                continue;
            }
            let mut apex = self.apex[i];

            // 1. Tourner, même si l'apex n'avance pas ensuite.
            let theta = (self.alea.suivant() % (amplitude + 1)) as u32;
            let phi = (self.alea.suivant() % 360) as u32;
            apex.direction = apex.direction.tourner(theta, phi);

            // 2. La longueur que le substrat de la case de départ autorise.
            let (cx, cy, cz) = (apex.x as usize, apex.y as usize, apex.z as usize);
            let valeur = self.substrat.valeur(cx, cy, cz);
            let mut longueur = if valeur <= 0.0 { 0.0 } else { (pas * valeur) / (affinite + valeur) };
            if cout > 0.0 && cout * longueur > valeur {
                longueur = valeur / cout;
            }

            // 3. La mémoire d'une pointe qui longe : la marche est écrasée, l'écart est toujours tiré, et la pointe
            //    retrouve son cap si le bout de l'obstacle est passé.
            if let Some((cap, g)) = apex.memoire {
                let theta = (self.alea.suivant() % (u64::from(MEMOIRE_DE_DIRECTION) + 1)) as u32;
                let phi = (self.alea.suivant() % 360) as u32;
                let candidat = cap.tourner(theta, phi);
                if self.pas_libre(&apex, pas, cap) && self.pas_libre(&apex, longueur, candidat) {
                    apex.direction = candidat;
                    apex.memoire = None;
                } else {
                    apex.direction = g;
                }
            } else if force > 0.0 {
                // 4. Le chimiotropisme, après la longueur et avant l'arrivée, comme en 2D.
                let g = gradient_3d(&self.substrat, cx, cy, cz).unwrap_or((0.0, 0.0, 0.0));
                apex.direction = tourner_chimiotropique_3d(apex.direction, g, force, &mut self.alea);
            }

            // 5-6. L'arrivée ; un apex bloqué garde sa position et sa nouvelle direction, et perd sa mémoire.
            let mut d = apex.direction;
            let (mut x1, mut y1, mut z1) = arrivee(&apex, longueur, d);
            if longueur <= 0.0 || self.dehors(x1, y1, z1) {
                apex.memoire = None;
                self.apex[i] = apex;
                continue;
            }

            // 7. Le contact avec un obstacle.
            let mut arret: Option<Axe3d> = None;
            if a_des_obstacles && let Some(contact) = self.contact(&apex, (x1, y1, z1)) {
                if angle_critique == 0 {
                    // À angle nul, la pointe reste, garde sa direction, et retentera au pas suivant.
                    self.apex[i] = apex;
                    continue;
                }
                let rasant = composante(d, contact.axe).abs() < SIN[angle_critique];
                match projection(d, contact.axe) {
                    Some(g) if rasant => {
                        // Elle longe : mémoire du cap et de la direction de glissement, puis le pas est refait.
                        apex.memoire = Some((d, g));
                        apex.direction = g;
                        d = g;
                        (x1, y1, z1) = arrivee(&apex, longueur, g);
                        if self.dehors(x1, y1, z1) {
                            apex.memoire = None;
                            self.apex[i] = apex;
                            continue;
                        }
                        if let Some(second) = self.contact(&apex, (x1, y1, z1)) {
                            // Un coin concave : c'est un arrêt contre le second contact.
                            arret = Some(second.axe);
                        }
                    }
                    _ => arret = Some(contact.axe),
                }
            }

            // 8. Un arrêt : la pointe devient inactive, sans mémoire, et chaque branche libre naît à sa position.
            if let Some(axe) = arret {
                apex.actif = false;
                apex.memoire = None;
                self.apex[i] = apex;
                let (plus, moins) = branches_d_arret(d, axe);
                for direction in [plus, moins] {
                    if self.pas_libre(&apex, pas, direction) {
                        let id = self.apex.len() as u32;
                        let (x, y, z) = (apex.x, apex.y, apex.z);
                        self.apex.push(Apex3d { id, x, y, z, direction, actif: true, memoire: None });
                        self.nb_ramifications += 1;
                    }
                }
                continue;
            }

            // 9. Consommer dans la case de départ, laisser le segment, avancer.
            self.substrat.retirer(cx, cy, cz, cout * longueur);
            self.segments.push(Segment3d { apex: apex.id, temps, x0: apex.x, y0: apex.y, z0: apex.z, x1, y1, z1 });
            apex.x = x1;
            apex.y = y1;
            apex.z = z1;
            self.apex[i] = apex;

            // 10. Se ramifier : la branche naît à l'arrivée et n'agira qu'au pas suivant.
            if let Some(probabilite) = probabilite
                && self.alea.suivant_f64() < probabilite
            {
                let phi = (self.alea.suivant() % 360) as u32;
                let id = self.apex.len() as u32;
                let direction = d.tourner(angle, phi);
                self.apex.push(Apex3d { id, x: x1, y: y1, z: z1, direction, actif: true, memoire: None });
                self.nb_ramifications += 1;
            }
        }
    }

    /// Vrai si le point sort de `[0, largeur) × [0, hauteur) × [0, profondeur)` (bornes hautes exclues).
    fn dehors(&self, x: f64, y: f64, z: f64) -> bool {
        let s = &self.scenario;
        let (largeur, hauteur, profondeur) = (s.largeur as f64, s.hauteur as f64, s.profondeur as f64);
        x < 0.0 || y < 0.0 || z < 0.0 || x >= largeur || y >= hauteur || z >= profondeur
    }

    /// Le contact du segment de l'apex à `fin`, s'il y en a un. Le départ d'un apex n'est jamais bloqué et `fin` est
    /// dans le cube : [`traverser_3d`] ne rend alors que `Libre` ou `Contact`.
    fn contact(&self, apex: &Apex3d, fin: (f64, f64, f64)) -> Option<crate::contact3d::Contact3d> {
        match traverser_3d(&self.substrat, (apex.x, apex.y, apex.z), fin) {
            Traversee3d::Contact(contact) => Some(contact),
            _ => None,
        }
    }

    /// Vrai si un pas de `longueur` dans la direction `d`, depuis l'apex, arrive dans le cube et ne touche aucune case
    /// bloquée.
    fn pas_libre(&self, apex: &Apex3d, longueur: f64, d: Direction3d) -> bool {
        let (x1, y1, z1) = arrivee(apex, longueur, d);
        !self.dehors(x1, y1, z1)
            && traverser_3d(&self.substrat, (apex.x, apex.y, apex.z), (x1, y1, z1)) == Traversee3d::Libre
    }

    /// Avance la colonie de `n` pas : strictement équivalent à `n` appels à [`Colonie3d::avancer`].
    pub fn avancer_n(&mut self, n: u32) {
        for _ in 0..n {
            self.avancer();
        }
    }

    /// Le nombre de pas déjà joués.
    pub fn temps(&self) -> u32 {
        self.temps
    }

    /// La durée du scénario, en pas.
    pub fn duree(&self) -> u32 {
        self.scenario.duree
    }

    /// Le scénario de la colonie, tel qu'il a été donné.
    pub fn scenario(&self) -> &ScenarioColonie3d {
        &self.scenario
    }

    /// Le substrat courant.
    pub fn substrat(&self) -> &Substrat3d {
        &self.substrat
    }

    /// Tous les apex, actifs ou non, dans l'ordre de leur identifiant (0, 1, 2… sans trou).
    pub fn apex(&self) -> &[Apex3d] {
        &self.apex
    }

    /// Tous les segments, dans l'ordre où ils ont été laissés (par pas, puis par identifiant d'apex).
    pub fn segments(&self) -> &[Segment3d] {
        &self.segments
    }

    /// Le nombre cumulé de ramifications.
    pub fn nb_ramifications(&self) -> u64 {
        self.nb_ramifications
    }

    /// Le nombre d'apex actifs : les pointes de la colonie.
    pub fn pointes(&self) -> usize {
        self.apex.iter().filter(|apex| apex.actif).count()
    }

    /// L'écriture canonique de la colonie (format dans la doc du module). Longueur :
    /// `4 + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases` octets, plus 8 si `chimiotropisme != 0.0`, plus
    /// `8 + 24 · obstacles + 4 + 49 · apex` s'il y a un obstacle ou si `thigmotropisme != 0`.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let substrat = self.substrat.ecrire_canonique();
        let mut octets = Vec::with_capacity(28 + 56 * self.segments.len() + 53 * self.apex.len() + substrat.len());
        octets.extend_from_slice(&self.temps.to_le_bytes());
        octets.extend_from_slice(&(self.segments.len() as u64).to_le_bytes());
        for segment in &self.segments {
            octets.extend_from_slice(&segment.apex.to_le_bytes());
            octets.extend_from_slice(&segment.temps.to_le_bytes());
            for v in [segment.x0, segment.y0, segment.z0, segment.x1, segment.y1, segment.z1] {
                octets.extend_from_slice(&v.to_bits().to_le_bytes());
            }
        }
        octets.extend_from_slice(&(self.apex.len() as u64).to_le_bytes());
        for apex in &self.apex {
            octets.extend_from_slice(&apex.id.to_le_bytes());
            let d = apex.direction;
            for v in [apex.x, apex.y, apex.z, d.x(), d.y(), d.z()] {
                octets.extend_from_slice(&v.to_bits().to_le_bytes());
            }
            octets.push(u8::from(apex.actif));
        }
        octets.extend_from_slice(&self.nb_ramifications.to_le_bytes());
        octets.extend_from_slice(&substrat);
        if self.scenario.chimiotropisme != 0.0 {
            octets.extend_from_slice(&self.scenario.chimiotropisme.to_bits().to_le_bytes());
        }
        if !self.scenario.obstacles.is_empty() || self.scenario.thigmotropisme != 0 {
            octets.extend_from_slice(&(self.scenario.obstacles.len() as u64).to_le_bytes());
            for o in &self.scenario.obstacles {
                for v in [o.x0, o.y0, o.z0, o.x1, o.y1, o.z1] {
                    octets.extend_from_slice(&v.to_le_bytes());
                }
            }
            octets.extend_from_slice(&self.scenario.thigmotropisme.to_le_bytes());
            for apex in &self.apex {
                let (drapeau, valeurs) = match apex.memoire {
                    Some((cap, g)) => (1u8, [cap.x(), cap.y(), cap.z(), g.x(), g.y(), g.z()]),
                    None => (0u8, [0.0; 6]),
                };
                octets.push(drapeau);
                for v in valeurs {
                    octets.extend_from_slice(&v.to_bits().to_le_bytes());
                }
            }
        }
        octets
    }

    /// L'empreinte de la colonie : le SHA-256 de [`Colonie3d::ecrire_canonique`], en hexadécimal minuscule de 64
    /// caractères.
    pub fn empreinte(&self) -> String {
        crate::empreinte::sha256_hex(&self.ecrire_canonique())
    }
}
