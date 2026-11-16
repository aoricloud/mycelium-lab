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
    let mut dimensions: Option<(u32, u32, u32)> = None;
    let mut substrat: Option<(f64, f64)> = None;
    let mut duree: Option<u32> = None;
    let mut depots: Vec<Depot> = Vec::new();
    let mut depots_lignes: Vec<usize> = Vec::new();
    let mut blocs: Vec<Bloc> = Vec::new();
    let mut blocs_lignes: Vec<usize> = Vec::new();

    for (indice, ligne) in texte.lines().enumerate() {
        let numero = indice + 1;
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let mots: Vec<&str> = ligne.split(' ').collect();
        if mots.iter().any(|mot| mot.is_empty()) {
            return Err(ErreurCube::LigneInvalide(numero));
        }
        match mots[0] {
            "cube" => {
                if mots.len() != 4 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                let (Some(largeur), Some(hauteur), Some(profondeur)) =
                    (entier(mots[1]), entier(mots[2]), entier(mots[3]))
                else {
                    return Err(ErreurCube::LigneInvalide(numero));
                };
                if largeur == 0 || hauteur == 0 || profondeur == 0 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                if dimensions.is_some() {
                    return Err(ErreurCube::EnDouble("cube"));
                }
                dimensions = Some((largeur, hauteur, profondeur));
            }
            "substrat" => {
                if mots.len() != 3 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                let (Some(initial), Some(diffusion)) = (nombre(mots[1]), nombre(mots[2])) else {
                    return Err(ErreurCube::LigneInvalide(numero));
                };
                if substrat.is_some() {
                    return Err(ErreurCube::EnDouble("substrat"));
                }
                substrat = Some((initial, diffusion));
            }
            "duree" => {
                if mots.len() != 2 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                let Some(valeur) = entier(mots[1]) else {
                    return Err(ErreurCube::LigneInvalide(numero));
                };
                if duree.is_some() {
                    return Err(ErreurCube::EnDouble("duree"));
                }
                duree = Some(valeur);
            }
            "nourriture" => {
                if mots.len() != 5 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                let (Some(x), Some(y), Some(z), Some(quantite)) =
                    (entier(mots[1]), entier(mots[2]), entier(mots[3]), nombre(mots[4]))
                else {
                    return Err(ErreurCube::LigneInvalide(numero));
                };
                depots.push(Depot { x, y, z, quantite });
                depots_lignes.push(numero);
            }
            "bloc" => {
                if mots.len() != 7 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                let (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) = (
                    entier(mots[1]),
                    entier(mots[2]),
                    entier(mots[3]),
                    entier(mots[4]),
                    entier(mots[5]),
                    entier(mots[6]),
                ) else {
                    return Err(ErreurCube::LigneInvalide(numero));
                };
                if x0 > x1 || y0 > y1 || z0 > z1 {
                    return Err(ErreurCube::LigneInvalide(numero));
                }
                blocs.push(Bloc { x0, y0, z0, x1, y1, z1 });
                blocs_lignes.push(numero);
            }
            _ => return Err(ErreurCube::LigneInvalide(numero)),
        }
    }

    let (largeur, hauteur, profondeur) = dimensions.ok_or(ErreurCube::Manquante("cube"))?;
    let (substrat_initial, diffusion) = substrat.ok_or(ErreurCube::Manquante("substrat"))?;
    let duree = duree.ok_or(ErreurCube::Manquante("duree"))?;

    for (depot, &numero) in depots.iter().zip(&depots_lignes) {
        if depot.x >= largeur || depot.y >= hauteur || depot.z >= profondeur {
            return Err(ErreurCube::HorsDuCube(numero));
        }
    }
    for (bloc, &numero) in blocs.iter().zip(&blocs_lignes) {
        if bloc.x1 >= largeur || bloc.y1 >= hauteur || bloc.z1 >= profondeur {
            return Err(ErreurCube::HorsDuCube(numero));
        }
    }
    for (depot, &numero) in depots.iter().zip(&depots_lignes) {
        let dans_un_bloc = blocs.iter().any(|bloc| {
            bloc.x0 <= depot.x
                && depot.x <= bloc.x1
                && bloc.y0 <= depot.y
                && depot.y <= bloc.y1
                && bloc.z0 <= depot.z
                && depot.z <= bloc.z1
        });
        if dans_un_bloc {
            return Err(ErreurCube::DepotBloque(numero));
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
        let mut octets = self.temps.to_le_bytes().to_vec();
        octets.extend_from_slice(&self.substrat.ecrire_canonique());
        octets
    }

    /// Empreinte du cube : le SHA-256 de [`Cube::ecrire_canonique`], en hexadécimal minuscule de 64 caractères.
    pub fn empreinte(&self) -> String {
        crate::empreinte::sha256_hex(&self.ecrire_canonique())
    }
}
