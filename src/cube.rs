//! Le scénario du cube et sa simulation : l'étape 18, une couche à part qui ne change rien au plan 2D.
//!
//! # Format
//!
//! Le texte se lit par lignes. Une ligne vide, ou dont le premier caractère est `#`, est ignorée. Les mots d'une
//! ligne sont séparés par une espace.
//!
//! - `cube <largeur> <hauteur> <profondeur>` (entiers strictement positifs) est obligatoire et unique ;
//! - `substrat <initial> <diffusion>` (nombres finis et positifs ou nuls) est obligatoire et unique ;
//! - `duree <n>` (entier) est obligatoire et unique ;
//! - `nourriture <x> <y> <z> <quantite>` (entiers, puis un nombre fini et positif ou nul) se répète, gardé dans
//!   l'ordre du texte ;
//! - `bloc <x0> <y0> <z0> <x1> <y1> <z1>` (entiers, `x0 ≤ x1`, `y0 ≤ y1`, `z0 ≤ z1`) se répète, gardé dans l'ordre
//!   du texte.
//!
//! Une ligne inconnue ou mal formée donne [`ErreurCube::LigneInvalide`] avec son numéro (à partir de 1). Une
//! directive unique répétée donne [`ErreurCube::EnDouble`], une directive unique absente
//! [`ErreurCube::Manquante`], dans l'ordre `cube`, `substrat`, `duree`. Une fois tout lu, une nourriture ou un bloc
//! qui sort du cube donne [`ErreurCube::HorsDuCube`] (nourritures d'abord, puis blocs), et une nourriture dans un
//! bloc donne [`ErreurCube::DepotBloque`].

use crate::empreinte::sha256_hex;
use crate::substrat3d::{ErreurSubstrat3d, Substrat3d};

/// Une nourriture déposée dans une case du cube.
#[derive(Clone, Debug, PartialEq)]
pub struct Depot {
    /// Colonne du dépôt.
    pub x: u32,
    /// Ligne du dépôt.
    pub y: u32,
    /// Profondeur du dépôt.
    pub z: u32,
    /// Quantité déposée.
    pub quantite: f64,
}

/// Un pavé de cases bloquées, bornes comprises.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bloc {
    /// Première colonne du pavé.
    pub x0: u32,
    /// Première ligne du pavé.
    pub y0: u32,
    /// Première profondeur du pavé.
    pub z0: u32,
    /// Dernière colonne du pavé.
    pub x1: u32,
    /// Dernière ligne du pavé.
    pub y1: u32,
    /// Dernière profondeur du pavé.
    pub z1: u32,
}

/// Le scénario d'un cube : ses dimensions, son substrat, ses dépôts, ses blocs et sa durée.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioCube {
    /// Largeur du cube, en cases.
    pub largeur: u32,
    /// Hauteur du cube, en cases.
    pub hauteur: u32,
    /// Profondeur du cube, en cases.
    pub profondeur: u32,
    /// Quantité de substrat déposée dans chaque case au départ.
    pub substrat_initial: f64,
    /// Coefficient de diffusion du substrat.
    pub diffusion: f64,
    /// Les nourritures, dans l'ordre du texte.
    pub depots: Vec<Depot>,
    /// Les blocs, dans l'ordre du texte.
    pub blocs: Vec<Bloc>,
    /// Durée de la simulation, en pas.
    pub duree: u32,
}

/// Erreur de lecture ou de construction d'un scénario de cube.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurCube {
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne (à partir de 1).
    LigneInvalide(usize),
    /// Une directive unique est répétée ; porte son nom.
    EnDouble(&'static str),
    /// Une directive unique est absente ; porte son nom.
    Manquante(&'static str),
    /// Une nourriture ou un bloc sort du cube ; porte le numéro de ligne.
    HorsDuCube(usize),
    /// Une nourriture tombe dans un bloc ; porte le numéro de ligne de la nourriture.
    DepotBloque(usize),
    /// La construction du substrat en volume a échoué.
    Substrat(ErreurSubstrat3d),
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

/// Lit un scénario de cube depuis son texte (format décrit dans la doc du module).
pub fn lire_scenario_cube(texte: &str) -> Result<ScenarioCube, ErreurCube> {
    let mut cube: Option<(u32, u32, u32)> = None;
    let mut substrat: Option<(f64, f64)> = None;
    let mut duree: Option<u32> = None;
    let mut depots = Vec::new();
    let mut blocs = Vec::new();
    let mut lignes_depots = Vec::new();
    let mut lignes_blocs = Vec::new();
    for (i, ligne) in texte.lines().enumerate() {
        let numero = i + 1;
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let mots: Vec<&str> = ligne.split(' ').collect();
        match mots.as_slice() {
            ["cube", l, h, p] => {
                let (l, h, p) = match (entier(l), entier(h), entier(p)) {
                    (Some(l), Some(h), Some(p)) if l > 0 && h > 0 && p > 0 => (l, h, p),
                    _ => return Err(ErreurCube::LigneInvalide(numero)),
                };
                if cube.is_some() {
                    return Err(ErreurCube::EnDouble("cube"));
                }
                cube = Some((l, h, p));
            }
            ["substrat", initial, diffusion] => {
                let (initial, diffusion) = match (nombre(initial), nombre(diffusion)) {
                    (Some(initial), Some(diffusion)) => (initial, diffusion),
                    _ => return Err(ErreurCube::LigneInvalide(numero)),
                };
                if substrat.is_some() {
                    return Err(ErreurCube::EnDouble("substrat"));
                }
                substrat = Some((initial, diffusion));
            }
            ["duree", n] => {
                let n = entier(n).ok_or(ErreurCube::LigneInvalide(numero))?;
                if duree.is_some() {
                    return Err(ErreurCube::EnDouble("duree"));
                }
                duree = Some(n);
            }
            ["nourriture", x, y, z, q] => {
                let (x, y, z, quantite) = match (entier(x), entier(y), entier(z), nombre(q)) {
                    (Some(x), Some(y), Some(z), Some(quantite)) => (x, y, z, quantite),
                    _ => return Err(ErreurCube::LigneInvalide(numero)),
                };
                depots.push(Depot { x, y, z, quantite });
                lignes_depots.push(numero);
            }
            ["bloc", x0, y0, z0, x1, y1, z1] => {
                let bloc = match (entier(x0), entier(y0), entier(z0), entier(x1), entier(y1), entier(z1)) {
                    (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1))
                        if x0 <= x1 && y0 <= y1 && z0 <= z1 =>
                    {
                        Bloc { x0, y0, z0, x1, y1, z1 }
                    }
                    _ => return Err(ErreurCube::LigneInvalide(numero)),
                };
                blocs.push(bloc);
                lignes_blocs.push(numero);
            }
            _ => return Err(ErreurCube::LigneInvalide(numero)),
        }
    }
    let (largeur, hauteur, profondeur) = cube.ok_or(ErreurCube::Manquante("cube"))?;
    let (substrat_initial, diffusion) = substrat.ok_or(ErreurCube::Manquante("substrat"))?;
    let duree = duree.ok_or(ErreurCube::Manquante("duree"))?;
    for (depot, &ligne) in depots.iter().zip(&lignes_depots) {
        if depot.x >= largeur || depot.y >= hauteur || depot.z >= profondeur {
            return Err(ErreurCube::HorsDuCube(ligne));
        }
    }
    for (bloc, &ligne) in blocs.iter().zip(&lignes_blocs) {
        if bloc.x1 >= largeur || bloc.y1 >= hauteur || bloc.z1 >= profondeur {
            return Err(ErreurCube::HorsDuCube(ligne));
        }
    }
    for (depot, &ligne) in depots.iter().zip(&lignes_depots) {
        let dans_un_bloc = blocs.iter().any(|bloc| {
            depot.x >= bloc.x0
                && depot.x <= bloc.x1
                && depot.y >= bloc.y0
                && depot.y <= bloc.y1
                && depot.z >= bloc.z0
                && depot.z <= bloc.z1
        });
        if dans_un_bloc {
            return Err(ErreurCube::DepotBloque(ligne));
        }
    }
    Ok(ScenarioCube { largeur, hauteur, profondeur, substrat_initial, diffusion, depots, blocs, duree })
}

/// Un cube simulé : son horloge, sa durée et son substrat en volume.
///
/// Contrat : le temps est le nombre de pas déjà faits ; le substrat est la seule chose qui évolue. Tout est
/// déterministe : mêmes paramètres, mêmes dépôts, même nombre de pas, mêmes octets.
#[derive(Clone, Debug, PartialEq)]
pub struct Cube {
    temps: u32,
    duree: u32,
    substrat: Substrat3d,
}

impl Cube {
    /// Construit un cube depuis un scénario : substrat de `dt = 1.0` et `dx = 1.0`, quantité initiale déposée
    /// dans chaque case si elle est strictement positive, puis les blocs dans l'ordre, puis les nourritures dans
    /// l'ordre. Le temps part de `0`.
    ///
    /// Contrat : un refus de [`Substrat3d::nouveau`] est rendu en [`ErreurCube::Substrat`].
    pub fn nouveau(scenario: &ScenarioCube) -> Result<Cube, ErreurCube> {
        let (largeur, hauteur, profondeur) =
            (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
        let mut substrat = Substrat3d::nouveau(largeur, hauteur, profondeur, scenario.diffusion, 1.0, 1.0)
            .map_err(ErreurCube::Substrat)?;
        if scenario.substrat_initial > 0.0 {
            for z in 0..profondeur {
                for y in 0..hauteur {
                    for x in 0..largeur {
                        substrat.deposer(x, y, z, scenario.substrat_initial);
                    }
                }
            }
        }
        for bloc in &scenario.blocs {
            substrat.bloquer(
                bloc.x0 as usize,
                bloc.y0 as usize,
                bloc.z0 as usize,
                bloc.x1 as usize,
                bloc.y1 as usize,
                bloc.z1 as usize,
            );
        }
        for depot in &scenario.depots {
            substrat.deposer(depot.x as usize, depot.y as usize, depot.z as usize, depot.quantite);
        }
        Ok(Cube { temps: 0, duree: scenario.duree, substrat })
    }

    /// Avance la simulation de `n` pas et ajoute `n` au temps.
    pub fn avancer(&mut self, n: u32) {
        self.substrat.pas_n(n as usize);
        self.temps += n;
    }

    /// Le nombre de pas déjà faits.
    pub fn temps(&self) -> u32 {
        self.temps
    }

    /// La durée du scénario, en pas.
    pub fn duree(&self) -> u32 {
        self.duree
    }

    /// Le substrat en volume.
    pub fn substrat(&self) -> &Substrat3d {
        &self.substrat
    }

    /// L'écriture canonique du cube : le temps en `u32` petit-boutiste, puis l'écriture canonique du substrat.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let substrat = self.substrat.ecrire_canonique();
        let mut octets = Vec::with_capacity(4 + substrat.len());
        octets.extend_from_slice(&self.temps.to_le_bytes());
        octets.extend_from_slice(&substrat);
        octets
    }

    /// Empreinte du cube : le SHA-256 de [`Cube::ecrire_canonique`], en hexadécimal minuscule de 64 caractères.
    pub fn empreinte(&self) -> String {
        sha256_hex(&self.ecrire_canonique())
    }
}
