//! La colonie en volume : des apex qui marchent, se nourrissent et se ramifient dans un cube de substrat.
//!
//! C'est le portage en 3D de la marche, de la longueur de pas tirée du substrat, de la consommation, de la
//! ramification et du chimiotropisme de [`crate::simulation`] et [`crate::tropisme`]. Les obstacles, le
//! thigmotropisme, l'anastomose et la mémoire de direction n'y sont pas (des étapes plus tard). Le substrat est un
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
//!   [`crate::simulation::lire_scenario_tropismes`]) ; absent, il vaut `0.0`, qui éteint le chimiotropisme.
//!
//! Un entier s'écrit en chiffres seulement (ni signe ni point). Une ligne `bloc`, quelle que soit sa forme, donne
//! [`ErreurColonie3d::BlocRefuse`] avec son numéro, dès qu'elle est lue : les obstacles viendront plus tard. Une
//! ligne inconnue ou mal formée (nombre de mots, nombre illisible ou hors de ses bornes) donne
//! [`ErreurColonie3d::LigneInvalide`] avec son numéro (à partir de 1). Une directive unique répétée, ou une seconde
//! ligne `chimiotropisme`, donne [`ErreurColonie3d::EnDouble`] (sur la seconde ligne, une fois celle-ci lue sans faute). Une fois tout lu, une
//! directive unique absente donne [`ErreurColonie3d::Manquante`], dans l'ordre `cube`, `substrat`, `spore`,
//! `croissance`, `couplage`, `duree` ; puis une spore hors du cube, puis une nourriture hors du cube (dans l'ordre du
//! texte) donnent [`ErreurColonie3d::HorsDuCube`] avec le numéro de leur ligne.
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
//! 6. seulement si `chimiotropisme != 0.0`, `chimiotropisme.to_bits()` (`u64`, 8 octets), tout à la fin.
//!
//! Sans chimiotropisme, l'écriture est donc celle d'avant, octet pour octet (les empreintes des scènes
//! `colonie-cube` et `colonie-goutte` ne changent pas).

use crate::alea::Alea;
use crate::croissance::{COS, SIN};
use crate::cube::Depot;
use crate::direction3d::Direction3d;
use crate::substrat3d::{ErreurSubstrat3d, Substrat3d};
use crate::tropisme::{FORCE_MAXIMALE, GRADIENT_DE_REFERENCE};

/// Le plus grand angle permis pour `angle` et `amplitude`, en degrés.
pub const ANGLE_MAXIMAL: u32 = 180;

/// Le plus grand nombre de cases d'un cube que [`Colonie3d::nouvelle`] accepte (`256³`, 128 Mio de substrat) : au-delà,
/// [`ErreurColonie3d::ReglageInvalide`] de nom `cube`, avant toute allocation.
pub const CASES_MAXIMALES: u64 = 256 * 256 * 256;

/// Le scénario d'une colonie en volume : le cube, son substrat, ses nourritures, la spore, la croissance, le couplage,
/// la durée et le chimiotropisme.
///
/// Contrat : un scénario rendu par [`lire_scenario_colonie3d`] respecte les bornes du format (voir la doc du module),
/// et sa spore et ses nourritures sont dans le cube. [`Colonie3d::nouvelle`] revérifie tout cela, sans paniquer, pour
/// un scénario construit à la main.
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
    /// La spore ou une nourriture sort du cube ; porte le numéro de ligne (`0` pour un scénario fait à la main).
    HorsDuCube(usize),
    /// Une ligne `bloc` : les obstacles ne sont pas encore permis ; porte le numéro de ligne.
    BlocRefuse(usize),
    /// Un réglage hors de ses bornes dans un scénario fait à la main ; porte le nom de la directive.
    ReglageInvalide(&'static str),
    /// La construction du substrat en volume a échoué.
    Substrat(ErreurSubstrat3d),
}

/// Un apex : une pointe de la colonie, à une position réelle, avec sa direction.
///
/// Contrat : `id` est son rang de naissance (0, 1, 2… sans trou) ; la position est toujours dans
/// `[0, largeur) × [0, hauteur) × [0, profondeur)`. Dans cette étape, un apex n'est jamais rendu inactif.
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
    let mut depots: Vec<Depot> = Vec::new();
    let mut depots_lignes: Vec<usize> = Vec::new();

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
    })
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
    Ok(())
}

/// Le gradient du substrat dans la case `(x, y, z)`, par différences centrées sur ses six voisines.
///
/// Contrat : avec `c = substrat.valeur(x, y, z)` et `v(…)` la valeur d'une voisine, ou `c` si elle sort du cube (aucun
/// flux, comme au bord de [`Substrat3d::pas`] et comme [`crate::substrat::Substrat::gradient`] en 2D), le résultat
/// est `((v(x+1) − v(x−1)) * 0.5, (v(y+1) − v(y−1)) * 0.5, (v(z+1) − v(z−1)) * 0.5)` : la soustraction d'abord, puis
/// le produit par `0.5` (qui donne, au bit près, la division par 2). Une case bloquée vaut `0.0`
/// ([`Substrat3d::valeur`]) ; la colonie n'en a pas encore. Rend `None` si la case est hors du cube, sans paniquer
/// (la colonie ne l'appelle que sur la case d'un apex, toujours dans le cube).
pub fn gradient_3d(substrat: &Substrat3d, x: usize, y: usize, z: usize) -> Option<(f64, f64, f64)> {
    let (largeur, hauteur, profondeur) = (substrat.largeur(), substrat.hauteur(), substrat.profondeur());
    if x >= largeur || y >= hauteur || z >= profondeur {
        return None;
    }
    let c = substrat.valeur(x, y, z);
    // Un voisin hors du cube vaut la case elle-même (aucun flux), comme dans `Substrat::gradient`.
    let gauche = if x == 0 { c } else { substrat.valeur(x - 1, y, z) };
    let droite = if x + 1 == largeur { c } else { substrat.valeur(x + 1, y, z) };
    let haut = if y == 0 { c } else { substrat.valeur(x, y - 1, z) };
    let bas = if y + 1 == hauteur { c } else { substrat.valeur(x, y + 1, z) };
    let devant = if z == 0 { c } else { substrat.valeur(x, y, z - 1) };
    let derriere = if z + 1 == profondeur { c } else { substrat.valeur(x, y, z + 1) };
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

/// Une colonie en volume : ses apex, ses segments, son substrat, son aléa et son horloge.
///
/// Contrat : l'état complet est déterminé par le scénario, la graine et le nombre de pas joués ; mêmes entrées, mêmes
/// octets ([`Colonie3d::ecrire_canonique`]). Aucun apex n'est retiré : il y en a toujours `1 + ramifications`.
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
    /// `substrat_initial` est déposée dans chaque case si elle est strictement positive, puis chaque nourriture dans
    /// l'ordre. Un seul apex, d'identifiant `0`, actif, part du centre de la case de la spore
    /// `(sx + 0.5, sy + 0.5, sz + 0.5)`, dans la direction [`Direction3d::au_hasard`] tirée de `Alea::new(graine)`.
    ///
    /// Contrat : rend [`ErreurColonie3d::ReglageInvalide`] pour un réglage hors des bornes du format ou un cube de plus
    /// de [`CASES_MAXIMALES`] cases, [`ErreurColonie3d::HorsDuCube`] (de numéro `0`) pour une spore ou une nourriture
    /// hors du cube (une dimension nulle en fait partie : la spore n'y tient pas), et [`ErreurColonie3d::Substrat`]
    /// pour un refus de [`Substrat3d::nouveau`] (`6 · diffusion > 1`). Ne panique pas.
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
    ///    3. si `chimiotropisme > 0`, se penche vers le gradient du substrat ([`tourner_chimiotropique_3d`], sur la
    ///       case de l'étape 2 et la direction de l'étape 1), même si `longueur` est nulle : aucun tirage si le
    ///       gradient est nul, un tirage `alea.suivant_f64()` sinon ; à `chimiotropisme = 0`, rien, aucun tirage ;
    ///    4. calcule l'arrivée `x + longueur * d.x()`, et de même en `y` et `z`, avec la direction de l'étape 3 ;
    ///    5. si `longueur ≤ 0` ou si l'arrivée sort de `[0, largeur) × [0, hauteur) × [0, profondeur)`, reste où il est
    ///       avec sa nouvelle direction, sans consommer, laisser, se ramifier ni tirer davantage ;
    ///    6. sinon retire `cout * longueur` de sa case de départ ([`Substrat3d::retirer`], qui ne retire jamais plus
    ///       que la case ne contient), laisse un [`Segment3d`] du départ à l'arrivée, et s'y déplace ;
    ///    7. si `intervalle` est fini, tire `alea.suivant_f64()` ; s'il est `< 1.0 / intervalle`, tire
    ///       `phi = alea.suivant() % 360`, et un apex neuf, actif, d'identifiant suivant, naît à l'arrivée dans la
    ///       direction `direction.tourner(angle, phi)` ; les ramifications croissent de 1.
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
        let (largeur, hauteur, profondeur) = (s.largeur as f64, s.hauteur as f64, s.profondeur as f64);

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

            // 3. Le chimiotropisme, après la longueur et avant l'arrivée, comme en 2D.
            if force > 0.0 {
                let g = gradient_3d(&self.substrat, cx, cy, cz).unwrap_or((0.0, 0.0, 0.0));
                apex.direction = tourner_chimiotropique_3d(apex.direction, g, force, &mut self.alea);
            }

            // 4-5. L'arrivée ; un apex bloqué garde sa position et sa nouvelle direction. Bornes hautes exclues.
            let d = apex.direction;
            let x1 = apex.x + longueur * d.x();
            let y1 = apex.y + longueur * d.y();
            let z1 = apex.z + longueur * d.z();
            let dehors = x1 < 0.0 || y1 < 0.0 || z1 < 0.0 || x1 >= largeur || y1 >= hauteur || z1 >= profondeur;
            if longueur <= 0.0 || dehors {
                self.apex[i] = apex;
                continue;
            }

            // 6. Consommer dans la case de départ, laisser le segment, avancer.
            self.substrat.retirer(cx, cy, cz, cout * longueur);
            self.segments.push(Segment3d { apex: apex.id, temps, x0: apex.x, y0: apex.y, z0: apex.z, x1, y1, z1 });
            apex.x = x1;
            apex.y = y1;
            apex.z = z1;
            self.apex[i] = apex;

            // 7. Se ramifier : la branche naît à l'arrivée et n'agira qu'au pas suivant.
            if let Some(probabilite) = probabilite
                && self.alea.suivant_f64() < probabilite
            {
                let phi = (self.alea.suivant() % 360) as u32;
                let id = self.apex.len() as u32;
                let direction = d.tourner(angle, phi);
                self.apex.push(Apex3d { id, x: x1, y: y1, z: z1, direction, actif: true });
                self.nb_ramifications += 1;
            }
        }
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
    /// `4 + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases` octets, plus 8 si `chimiotropisme != 0.0`.
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
        octets
    }

    /// L'empreinte de la colonie : le SHA-256 de [`Colonie3d::ecrire_canonique`], en hexadécimal minuscule de 64
    /// caractères.
    pub fn empreinte(&self) -> String {
        crate::empreinte::sha256_hex(&self.ecrire_canonique())
    }
}
