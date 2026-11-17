//! La colonie en volume : des apex qui marchent, se nourrissent et se ramifient dans un cube de substrat.
//!
//! C'est le portage en 3D de la marche, de la longueur de pas tirée du substrat, de la consommation et de la
//! ramification de [`crate::simulation`]. Le chimiotropisme, les obstacles, le thigmotropisme, l'anastomose et la
//! mémoire de direction n'y sont pas (des étapes plus tard). Le substrat est un [`Substrat3d`] ; une direction est une
//! [`Direction3d`], qu'on fait tourner d'angles entiers.
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
//! - `duree <n>` (entier) : obligatoire et unique.
//!
//! Un entier s'écrit en chiffres seulement (ni signe ni point). Une ligne `bloc`, quelle que soit sa forme, donne
//! [`ErreurColonie3d::BlocRefuse`] avec son numéro, dès qu'elle est lue : les obstacles viendront plus tard. Une
//! ligne inconnue ou mal formée (nombre de mots, nombre illisible ou hors de ses bornes) donne
//! [`ErreurColonie3d::LigneInvalide`] avec son numéro (à partir de 1). Une directive unique répétée donne
//! [`ErreurColonie3d::EnDouble`] (sur la seconde ligne, une fois celle-ci lue sans faute). Une fois tout lu, une
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
//! 5. [`Substrat3d::ecrire_canonique`].

use crate::alea::Alea;
use crate::cube::Depot;
use crate::direction3d::Direction3d;
use crate::substrat3d::{ErreurSubstrat3d, Substrat3d};

/// Le plus grand angle permis pour `angle` et `amplitude`, en degrés.
pub const ANGLE_MAXIMAL: u32 = 180;

/// Le plus grand nombre de cases d'un cube que [`Colonie3d::nouvelle`] accepte (`256³`, 128 Mio de substrat) : au-delà,
/// [`ErreurColonie3d::ReglageInvalide`] de nom `cube`, avant toute allocation.
pub const CASES_MAXIMALES: u64 = 256 * 256 * 256;

/// Le scénario d'une colonie en volume : le cube, son substrat, ses nourritures, la spore, la croissance, le couplage
/// et la durée.
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

/// Lit un entier décimal non signé : des chiffres seulement (ni signe ni point).
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
fn nombre_positif(mot: &str) -> Option<f64> {
    let valeur: f64 = mot.parse().ok()?;
    if valeur.is_finite() && valeur > 0.0 { Some(valeur) } else { None }
}

/// Vrai si la case `(x, y, z)` est dans un cube de ces dimensions.
fn dans_le_cube(x: u32, y: u32, z: u32, largeur: u32, hauteur: u32, profondeur: u32) -> bool {
    x < largeur && y < hauteur && z < profondeur
}

/// Lit un scénario de colonie en volume depuis son texte (format et ordre des erreurs dans la doc du module).
pub fn lire_scenario_colonie3d(texte: &str) -> Result<ScenarioColonie3d, ErreurColonie3d> {
    let mut dimensions: Option<(u32, u32, u32)> = None;
    let mut substrat: Option<(f64, f64)> = None;
    let mut croissance: Option<(f64, f64, u32, u32)> = None;
    let mut couplage: Option<(f64, f64)> = None;
    let mut spore: Option<(u32, u32, u32)> = None;
    let mut spore_ligne = 0;
    let mut duree: Option<u32> = None;
    let mut depots: Vec<Depot> = Vec::new();
    let mut depots_lignes: Vec<usize> = Vec::new();

    for (indice, brute) in texte.lines().enumerate() {
        let numero = indice + 1;
        let ligne = brute.trim();
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let mots: Vec<&str> = ligne.split_whitespace().collect();
        match mots.as_slice() {
            ["cube", largeur, hauteur, profondeur] => match (entier(largeur), entier(hauteur), entier(profondeur)) {
                (Some(largeur), Some(hauteur), Some(profondeur)) if largeur > 0 && hauteur > 0 && profondeur > 0 => {
                    if dimensions.is_some() {
                        return Err(ErreurColonie3d::EnDouble("cube"));
                    }
                    dimensions = Some((largeur, hauteur, profondeur));
                }
                _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
            },
            ["substrat", initiale, diffusion] => match (nombre(initiale), nombre(diffusion)) {
                (Some(initiale), Some(diffusion)) => {
                    if substrat.is_some() {
                        return Err(ErreurColonie3d::EnDouble("substrat"));
                    }
                    substrat = Some((initiale, diffusion));
                }
                _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
            },
            ["nourriture", x, y, z, quantite] => match (entier(x), entier(y), entier(z), nombre(quantite)) {
                (Some(x), Some(y), Some(z), Some(quantite)) => {
                    depots.push(Depot { x, y, z, quantite });
                    depots_lignes.push(numero);
                }
                _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
            },
            ["spore", x, y, z] => match (entier(x), entier(y), entier(z)) {
                (Some(x), Some(y), Some(z)) => {
                    if spore.is_some() {
                        return Err(ErreurColonie3d::EnDouble("spore"));
                    }
                    spore = Some((x, y, z));
                    spore_ligne = numero;
                }
                _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
            },
            ["croissance", pas, intervalle, angle, amplitude] => {
                let pas = nombre_positif(pas);
                let intervalle = if *intervalle == "infini" { Some(f64::INFINITY) } else { nombre_positif(intervalle) };
                let angle = entier(angle).filter(|angle| *angle <= ANGLE_MAXIMAL);
                let amplitude = entier(amplitude).filter(|amplitude| *amplitude <= ANGLE_MAXIMAL);
                match (pas, intervalle, angle, amplitude) {
                    (Some(pas), Some(intervalle), Some(angle), Some(amplitude)) => {
                        if croissance.is_some() {
                            return Err(ErreurColonie3d::EnDouble("croissance"));
                        }
                        croissance = Some((pas, intervalle, angle, amplitude));
                    }
                    _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
                }
            }
            ["couplage", affinite, cout] => match (nombre_positif(affinite), nombre(cout)) {
                (Some(affinite), Some(cout)) => {
                    if couplage.is_some() {
                        return Err(ErreurColonie3d::EnDouble("couplage"));
                    }
                    couplage = Some((affinite, cout));
                }
                _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
            },
            ["duree", valeur] => match entier(valeur) {
                Some(valeur) => {
                    if duree.is_some() {
                        return Err(ErreurColonie3d::EnDouble("duree"));
                    }
                    duree = Some(valeur);
                }
                None => return Err(ErreurColonie3d::LigneInvalide(numero)),
            },
            _ if mots.first() == Some(&"bloc") => return Err(ErreurColonie3d::BlocRefuse(numero)),
            _ => return Err(ErreurColonie3d::LigneInvalide(numero)),
        }
    }

    let (largeur, hauteur, profondeur) = dimensions.ok_or(ErreurColonie3d::Manquante("cube"))?;
    let (substrat_initial, diffusion) = substrat.ok_or(ErreurColonie3d::Manquante("substrat"))?;
    let (spore, spore_ligne) = spore.map(|spore| (spore, spore_ligne)).ok_or(ErreurColonie3d::Manquante("spore"))?;
    let (pas, intervalle, angle, amplitude) = croissance.ok_or(ErreurColonie3d::Manquante("croissance"))?;
    let (affinite, cout) = couplage.ok_or(ErreurColonie3d::Manquante("couplage"))?;
    let duree = duree.ok_or(ErreurColonie3d::Manquante("duree"))?;

    if !dans_le_cube(spore.0, spore.1, spore.2, largeur, hauteur, profondeur) {
        return Err(ErreurColonie3d::HorsDuCube(spore_ligne));
    }
    for (depot, &numero) in depots.iter().zip(&depots_lignes) {
        if !dans_le_cube(depot.x, depot.y, depot.z, largeur, hauteur, profondeur) {
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
    })
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
        // Les réglages du format, vérifiés avant toute allocation. Le nombre de cases se calcule en `u128` : le
        // produit de trois `u32` déborde un `u64`.
        let cases = (scenario.largeur as u128) * (scenario.hauteur as u128) * (scenario.profondeur as u128);
        if cases > CASES_MAXIMALES as u128 {
            return Err(ErreurColonie3d::ReglageInvalide("cube"));
        }
        if !scenario.substrat_initial.is_finite()
            || scenario.substrat_initial < 0.0
            || !scenario.diffusion.is_finite()
            || scenario.diffusion < 0.0
        {
            return Err(ErreurColonie3d::ReglageInvalide("substrat"));
        }
        let intervalle_valide =
            scenario.intervalle == f64::INFINITY || (scenario.intervalle.is_finite() && scenario.intervalle > 0.0);
        if !scenario.pas.is_finite()
            || scenario.pas <= 0.0
            || !intervalle_valide
            || scenario.angle > ANGLE_MAXIMAL
            || scenario.amplitude > ANGLE_MAXIMAL
        {
            return Err(ErreurColonie3d::ReglageInvalide("croissance"));
        }
        if !scenario.affinite.is_finite()
            || scenario.affinite <= 0.0
            || !scenario.cout.is_finite()
            || scenario.cout < 0.0
        {
            return Err(ErreurColonie3d::ReglageInvalide("couplage"));
        }
        for depot in &scenario.depots {
            if !depot.quantite.is_finite() || depot.quantite < 0.0 {
                return Err(ErreurColonie3d::ReglageInvalide("nourriture"));
            }
        }

        // La spore puis les nourritures, dans l'ordre du texte : une dimension nulle ne contient aucune case, donc
        // la spore n'y tient pas.
        let (sx, sy, sz) = scenario.spore;
        if !dans_le_cube(sx, sy, sz, scenario.largeur, scenario.hauteur, scenario.profondeur) {
            return Err(ErreurColonie3d::HorsDuCube(0));
        }
        for depot in &scenario.depots {
            if !dans_le_cube(depot.x, depot.y, depot.z, scenario.largeur, scenario.hauteur, scenario.profondeur) {
                return Err(ErreurColonie3d::HorsDuCube(0));
            }
        }

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

        // Le premier tirage fixe la direction du premier apex : `au_hasard` en consomme deux.
        let mut alea = Alea::new(graine);
        let direction = Direction3d::au_hasard(&mut alea);
        let premier =
            Apex3d { id: 0, x: sx as f64 + 0.5, y: sy as f64 + 0.5, z: sz as f64 + 0.5, direction, actif: true };

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
    ///    3. calcule l'arrivée `x + longueur * d.x()`, et de même en `y` et `z` ;
    ///    4. si `longueur ≤ 0` ou si l'arrivée sort de `[0, largeur) × [0, hauteur) × [0, profondeur)`, reste où il est
    ///       avec sa nouvelle direction, sans consommer, laisser, se ramifier ni tirer davantage ;
    ///    5. sinon retire `cout * longueur` de sa case de départ ([`Substrat3d::retirer`], qui ne retire jamais plus
    ///       que la case ne contient), laisse un [`Segment3d`] du départ à l'arrivée, et s'y déplace ;
    ///    6. si `intervalle` est fini, tire `alea.suivant_f64()` ; s'il est `< 1.0 / intervalle`, tire
    ///       `phi = alea.suivant() % 360`, et un apex neuf, actif, d'identifiant suivant, naît à l'arrivée dans la
    ///       direction `direction.tourner(angle, phi)` ; les ramifications croissent de 1.
    pub fn avancer(&mut self) {
        // 1. Le substrat diffuse d'abord, puis le temps avance ; seuls les apex présents au début du pas agissent.
        self.substrat.pas();
        self.temps += 1;
        let temps = self.temps;
        let nb_apex_au_debut = self.apex.len();

        let pas = self.scenario.pas;
        let amplitude = self.scenario.amplitude as u64;
        let angle = self.scenario.angle;
        let intervalle = self.scenario.intervalle;
        let affinite = self.scenario.affinite;
        let cout = self.scenario.cout;
        let largeur = self.scenario.largeur as f64;
        let hauteur = self.scenario.hauteur as f64;
        let profondeur = self.scenario.profondeur as f64;

        for i in 0..nb_apex_au_debut {
            if !self.apex[i].actif {
                continue;
            }
            let mut apex = self.apex[i];

            // a. Tourner : deux tirages toujours, même à amplitude nulle.
            let theta = (self.alea.suivant() % (amplitude + 1)) as u32;
            let phi = (self.alea.suivant() % 360) as u32;
            apex.direction = apex.direction.tourner(theta, phi);

            // b. La case de départ, son substrat et la longueur qu'il autorise : aucun tirage.
            let depart_x = apex.x as usize;
            let depart_y = apex.y as usize;
            let depart_z = apex.z as usize;
            let s = self.substrat.valeur(depart_x, depart_y, depart_z);
            let mut longueur = if s <= 0.0 { 0.0 } else { pas * s / (affinite + s) };
            if cout > 0.0 && cout * longueur > s {
                longueur = s / cout;
            }

            // c. L'arrivée, dans la direction de la pointe.
            let x1 = apex.x + longueur * apex.direction.x();
            let y1 = apex.y + longueur * apex.direction.y();
            let z1 = apex.z + longueur * apex.direction.z();

            // d. Une pointe bloquée garde sa position et sa nouvelle direction, sans rien consommer, laisser,
            //    ramifier, ni tirer de plus.
            if longueur <= 0.0 || x1 < 0.0 || y1 < 0.0 || z1 < 0.0 || x1 >= largeur || y1 >= hauteur || z1 >= profondeur
            {
                self.apex[i] = apex;
                continue;
            }

            // e. Consommer dans la case de départ, laisser le segment et s'y déplacer.
            self.substrat.retirer(depart_x, depart_y, depart_z, cout * longueur);
            let segment = Segment3d { apex: apex.id, temps, x0: apex.x, y0: apex.y, z0: apex.z, x1, y1, z1 };
            self.segments.push(segment);
            apex.x = x1;
            apex.y = y1;
            apex.z = z1;
            self.apex[i] = apex;

            // f. Se ramifier : un tirage par pas, puis un second pour `phi` ; l'apex neuf n'agira qu'au pas suivant.
            if intervalle.is_finite() && self.alea.suivant_f64() < 1.0 / intervalle {
                let phi = (self.alea.suivant() % 360) as u32;
                let direction = apex.direction.tourner(angle, phi);
                let id = self.apex.len() as u32;
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
    /// `4 + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases` octets.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let mut octets = Vec::new();
        octets.extend_from_slice(&self.temps.to_le_bytes());
        octets.extend_from_slice(&(self.segments.len() as u64).to_le_bytes());
        for segment in &self.segments {
            octets.extend_from_slice(&segment.apex.to_le_bytes());
            octets.extend_from_slice(&segment.temps.to_le_bytes());
            octets.extend_from_slice(&segment.x0.to_bits().to_le_bytes());
            octets.extend_from_slice(&segment.y0.to_bits().to_le_bytes());
            octets.extend_from_slice(&segment.z0.to_bits().to_le_bytes());
            octets.extend_from_slice(&segment.x1.to_bits().to_le_bytes());
            octets.extend_from_slice(&segment.y1.to_bits().to_le_bytes());
            octets.extend_from_slice(&segment.z1.to_bits().to_le_bytes());
        }
        octets.extend_from_slice(&(self.apex.len() as u64).to_le_bytes());
        for apex in &self.apex {
            octets.extend_from_slice(&apex.id.to_le_bytes());
            octets.extend_from_slice(&apex.x.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.y.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.z.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.direction.x().to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.direction.y().to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.direction.z().to_bits().to_le_bytes());
            octets.push(u8::from(apex.actif));
        }
        octets.extend_from_slice(&self.nb_ramifications.to_le_bytes());
        octets.extend_from_slice(&self.substrat.ecrire_canonique());
        octets
    }

    /// L'empreinte de la colonie : le SHA-256 de [`Colonie3d::ecrire_canonique`], en hexadécimal minuscule de 64
    /// caractères.
    pub fn empreinte(&self) -> String {
        crate::empreinte::sha256_hex(&self.ecrire_canonique())
    }
}
