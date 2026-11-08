//! Simulation : le couplage entre la colonie et le substrat, pas après pas.
//!
//! Une [`Simulation`] rassemble les réglages d'un scénario couplé, ses propres apex, un [`Substrat`] qui diffuse et
//! les nombres cumulés. À chaque pas, le substrat diffuse d'abord, puis chaque apex actif tourne, avance et consomme
//! le substrat de sa case de départ ; il peut ensuite se ramifier ou s'anastomoser. L'état complet — apex, segments,
//! substrat, nombres cumulés — est déterminé par les réglages, la graine et le nombre de pas joués.
//!
//! # Le pas de temps
//!
//! Un pas commence par faire diffuser le substrat ([`Substrat::pas`]) ; c'est seulement ensuite que les apex
//! s'étendent. Chaque apex actif, dans l'ordre de son identifiant :
//!
//! 1. tourne d'un angle entier tiré uniformément dans `-amplitude_marche..=amplitude_marche` ;
//! 2. lit le substrat `s` de la case où il se trouve, en déduit la longueur `(pas · s) / (affinite + s)`, bornée
//!    par ce que la case contient quand `cout > 0` ;
//! 3. si la longueur est positive et que l'arrivée reste dans `[0, largeur) × [0, hauteur)`, consomme
//!    `cout · longueur` dans la case de départ et laisse un [`Segment`] ; sinon il garde sa position et sa nouvelle
//!    direction, sans rien consommer ni se ramifier ;
//! 4. s'il approche un segment étranger assez ancien, s'anastomose et devient inactif ;
//! 5. sinon se ramifie avec la probabilité `1 / intervalle`, en créant un apex qui n'agira qu'au pas suivant.
//!
//! Un relevé est enregistré après chaque pas, avec celui du temps 0 : il y en a donc `temps() + 1`.
//!
//! # L'écriture canonique
//!
//! [`Simulation::ecrire_canonique`] concatène, sans remplissage, tous les entiers en petit-boutiste :
//!
//! 1. `temps` en `u32` (4 octets) ;
//! 2. le nombre de segments en `u64` (8 octets) ;
//! 3. chaque segment dans l'ordre de [`Simulation::segments`], sur 40 octets ([`croissance::OCTETS_PAR_SEGMENT`]) :
//!    `apex` (`u32`), `temps` (`u32`), puis `x0`, `y0`, `x1`, `y1`, chacun en `f64::to_bits().to_le_bytes()` ;
//! 4. chaque apex dans l'ordre de son identifiant, sur 25 octets : `id` (`u32`), `x` et `y` par
//!    `f64::to_bits().to_le_bytes()` (8 octets chacun), `direction` (`u32`), `actif` (`u8`, 1 ou 0) ;
//! 5. les compteurs cumulés en `u64` : ramifications, anastomoses, puis pointes ;
//! 6. [`Substrat::ecrire_canonique`] : `largeur` et `hauteur` en `u64`, puis chaque case par
//!    `f64::to_bits().to_le_bytes()`.
//!
//! Le nombre d'apex n'est pas écrit : il vaut toujours `1 + ramifications`, aucun apex n'étant retiré.
//!
//! # Le scénario couplé
//!
//! Un scénario se lit depuis un texte par [`lire_scenario`]. Chaque ligne non vide et ne commençant pas par `#`
//! porte une directive, ses mots séparés par une espace :
//!
//! - `boite <largeur> <hauteur>` : deux entiers strictement positifs ;
//! - `spore <x> <y>` : deux entiers, coordonnées de la spore dans la boîte ;
//! - `substrat <substrat_initial> <diffusion>` : deux nombres, la quantité initiale dans chaque case et le
//!   coefficient de diffusion ;
//! - `croissance <pas> <intervalle> <angle_ramification> <amplitude_marche>` : les paramètres de
//!   [`croissance::Parametres`] ; `intervalle` accepte le mot `infini` et vaut alors `f64::INFINITY` ;
//! - `couplage <affinite> <cout>` : deux nombres, l'affinité de la colonie pour le substrat et le coût en
//!   substrat d'une unité de biomasse ;
//! - `anastomose <rayon> <delai>` : optionnelle ; son absence désactive l'anastomose ;
//! - `duree <n>` : la durée en pas de temps, un entier non signé.
//!
//! Toutes les directives sauf `anastomose` sont obligatoires. Une ligne inconnue ou mal formée est une
//! [`ErreurLecture::LigneInvalide`], une directive obligatoire absente une [`ErreurLecture::Manquante`].

use crate::alea::Alea;
use crate::anastomose;
use crate::croissance;
use crate::croissance::{COS, NB_DIRECTIONS, SIN, Segment};
use crate::empreinte;
use crate::releves::Releve;
use crate::substrat::{self, Substrat};

/// Les réglages d'une simulation couplée, tels qu'ils se lisent dans un scénario ou se posent à la main.
///
/// Contrat : `largeur` et `hauteur` sont strictement positifs ; `spore` est une case de la boîte, c'est-à-dire
/// `spore.0 < largeur` et `spore.1 < hauteur` ; `substrat_initial`, `diffusion`, `affinite` et `cout` sont finis
/// et positifs ou nuls ; les paramètres de croissance respectent le contrat de
/// [`croissance::Parametres`]. [`Simulation::nouvelle`] refuse les réglages hors contrat, sans paniquer.
#[derive(Clone, Debug, PartialEq)]
pub struct Reglages {
    /// Largeur de la boîte, en cases.
    pub largeur: usize,
    /// Hauteur de la boîte, en cases.
    pub hauteur: usize,
    /// Coordonnées `(x, y)` de la spore dans la boîte.
    pub spore: (usize, usize),
    /// Quantité de substrat déposée dans chaque case au temps 0.
    pub substrat_initial: f64,
    /// Coefficient de diffusion du substrat, en case² par pas de temps.
    pub diffusion: f64,
    /// Les paramètres de la croissance des apex.
    pub croissance: croissance::Parametres,
    /// Affinité de la colonie pour le substrat : plus elle est grande, plus une case donnée soutient l'extension.
    pub affinite: f64,
    /// Coût en substrat d'une unité de biomasse (d'une unité de longueur d'hyphe).
    pub cout: f64,
    /// Réglage de l'anastomose, ou `None` pour la désactiver.
    pub anastomose: Option<anastomose::Reglage>,
}

/// Une tache de nutriments : les cases dont le centre est à moins de `rayon` de la case `(x, y)` démarrent à
/// `quantite`.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// Colonne du centre de la tache.
    pub x: usize,
    /// Ligne du centre de la tache.
    pub y: usize,
    /// Rayon de la tache, en cases : une case `(i, j)` en fait partie si `(i - x)² + (j - y)² ≤ rayon²`, en entiers.
    pub rayon: usize,
    /// Quantité de substrat de chaque case de la tache au temps 0 (elle remplace `substrat_initial`).
    pub quantite: f64,
}

/// Un rectangle de cases infranchissables : `x0..=x1` × `y0..=y1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obstacle {
    /// Première colonne du rectangle.
    pub x0: usize,
    /// Première ligne du rectangle.
    pub y0: usize,
    /// Dernière colonne du rectangle, incluse.
    pub x1: usize,
    /// Dernière ligne du rectangle, incluse.
    pub y1: usize,
}

/// Le milieu d'une simulation : ses taches et ses obstacles. `Milieu::default()` est vide.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Milieu {
    /// Les taches de nutriments, dans l'ordre du texte du scénario.
    pub sources: Vec<Source>,
    /// Les obstacles, dans l'ordre du texte du scénario.
    pub obstacles: Vec<Obstacle>,
}

/// Pourquoi une [`Simulation`] est refusée.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ErreurSimulation {
    /// Un réglage est hors contrat : spore hors de la boîte, quantité négative ou non finie, paramètres de
    /// croissance hors contrat.
    ParametresInvalides,
    /// Le milieu est hors contrat : source hors de la boîte ou de quantité négative ou non finie, obstacle hors de la
    /// boîte, à l'envers, ou qui contient la case de la spore.
    MilieuInvalide,
    /// Le substrat refuse ses paramètres de diffusion (voir [`substrat::ErreurSubstrat`]).
    Substrat(substrat::ErreurSubstrat),
}

/// Un apex de la simulation : sa position, sa direction et son état.
///
/// L'identifiant est aussi l'indice de l'apex dans le vecteur : les apex naissent à la fin, dans l'ordre de leur
/// identifiant, et aucun n'est retiré — un apex anastomosé reste, seulement inactif.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Apex {
    id: u32,
    x: f64,
    y: f64,
    direction: usize,
    actif: bool,
}

/// L'état simulé complet : les apex, le substrat et les nombres cumulés.
///
/// Tous les champs sont privés ; l'état ne s'observe que par les accesseurs de [`Simulation`], dans un ordre fixe,
/// et se compare « à l'octet » par [`Simulation::ecrire_canonique`].
pub struct Simulation {
    reglages: Reglages,
    apex: Vec<Apex>,
    substrat: Substrat,
    index: anastomose::Index,
    alea: Alea,
    temps: u32,
    nb_ramifications: u64,
    nb_anastomoses: u64,
    biomasse: f64,
    rayon: f64,
    releves: Vec<Releve>,
}

impl Simulation {
    /// Construit l'état au temps 0 : substrat initial uniforme, une spore au centre demandé et aucun segment.
    ///
    /// Refuse avec [`ErreurSimulation::ParametresInvalides`] si la spore sort de la boîte, si `substrat_initial`,
    /// `affinite` ou `cout` est négatif ou non fini, ou si les paramètres de croissance sont hors de leur contrat
    /// (c'est alors une erreur, pas une panique) ; avec [`ErreurSimulation::Substrat`] si les paramètres de
    /// diffusion sont refusés par [`Substrat::nouveau`] — notamment un pas de diffusion instable ou une dimension
    /// nulle.
    pub fn nouvelle(reglages: Reglages, graine: u64) -> Result<Simulation, ErreurSimulation> {
        // Le substrat est construit d'abord : une dimension nulle ou un pas de diffusion instable est une erreur
        // de substrat, comme le décrit le contrat de `Reglages`.
        let mut substrat = Substrat::nouveau(reglages.largeur, reglages.hauteur, reglages.diffusion, 1.0, 1.0)
            .map_err(ErreurSimulation::Substrat)?;

        if reglages.spore.0 >= reglages.largeur
            || reglages.spore.1 >= reglages.hauteur
            || !reglages.substrat_initial.is_finite()
            || reglages.substrat_initial < 0.0
            || !reglages.affinite.is_finite()
            || reglages.affinite < 0.0
            || !reglages.cout.is_finite()
            || reglages.cout < 0.0
            || !parametres_valides(&reglages.croissance)
        {
            return Err(ErreurSimulation::ParametresInvalides);
        }

        for y in 0..reglages.hauteur {
            for x in 0..reglages.largeur {
                substrat.deposer(x, y, reglages.substrat_initial);
            }
        }

        // Le premier tirage fixe la direction du premier apex, comme `Colonie::depuis_spore`.
        let mut alea = Alea::new(graine);
        let direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
        let premier =
            Apex { id: 0, x: reglages.spore.0 as f64 + 0.5, y: reglages.spore.1 as f64 + 0.5, direction, actif: true };
        let index = anastomose::Index::new(reglages.largeur, reglages.hauteur);

        let mut simulation = Simulation {
            reglages,
            apex: vec![premier],
            substrat,
            index,
            alea,
            temps: 0,
            nb_ramifications: 0,
            nb_anastomoses: 0,
            biomasse: 0.0,
            rayon: 0.0,
            releves: Vec::new(),
        };
        let initial = simulation.releve_courant();
        simulation.releves.push(initial);
        Ok(simulation)
    }

    /// Comme [`Simulation::nouvelle`], avec un [`Milieu`] : au temps 0, les cases de chaque source valent sa
    /// quantité (la dernière source du milieu l'emporte sur une case commune ; une source dont le centre est dans la
    /// boîte mais dont le disque déborde est tronquée à la boîte) et les cases d'obstacle valent `0.0` pour toujours,
    /// même sous une source. Avec `Milieu::default()`, l'état est le même, octet pour octet, que celui de
    /// [`Simulation::nouvelle`], pour toute graine.
    ///
    /// Refuse avec [`ErreurSimulation::MilieuInvalide`] une source dont le centre est hors de la boîte ou de
    /// quantité négative ou non finie, un obstacle hors de la boîte ou à l'envers (`x0 > x1` ou `y0 > y1`), un
    /// obstacle qui contient la case de la spore ; les autres réglages sont refusés comme par `nouvelle`.
    pub fn nouvelle_avec_milieu(
        reglages: Reglages,
        milieu: Milieu,
        graine: u64,
    ) -> Result<Simulation, ErreurSimulation> {
        todo!("étape 5 : nouvelle_avec_milieu {reglages:?} {milieu:?} {graine}")
    }

    /// Le milieu de la simulation : ses taches et ses obstacles, tels qu'ils ont été donnés.
    pub fn milieu(&self) -> &Milieu {
        todo!("étape 5 : milieu")
    }

    /// Avance la simulation d'un pas de temps : le substrat diffuse, puis les apex s'étendent en le consommant.
    /// Un relevé du nouvel état est ajouté à [`Simulation::releves`].
    pub fn avancer(&mut self) {
        // 1. Le substrat diffuse d'abord, puis le temps avance ; les apex présents au début du pas seulement
        //    agissent, ceux qui naissent pendant le pas attendent le suivant.
        self.substrat.pas();
        self.temps += 1;
        let temps = self.temps;
        let nb_apex_au_debut = self.apex.len();

        let param = self.reglages.croissance;
        let pas = param.pas;
        let amplitude = param.amplitude_marche as u64;
        let angle_ramification = param.angle_ramification as i64;
        let probabilite = param.probabilite_ramification();
        let affinite = self.reglages.affinite;
        let cout = self.reglages.cout;
        let largeur = self.reglages.largeur as f64;
        let hauteur = self.reglages.hauteur as f64;
        let spore_x = self.reglages.spore.0 as f64 + 0.5;
        let spore_y = self.reglages.spore.1 as f64 + 0.5;
        let reglage_anastomose = self.reglages.anastomose;

        for i in 0..nb_apex_au_debut {
            if !self.apex[i].actif {
                continue;
            }
            let mut apex = self.apex[i];

            // a. Tourner, même si l'apex n'avance pas ensuite.
            let delta = (self.alea.suivant() % (2 * amplitude + 1)) as i64 - amplitude as i64;
            apex.direction = (apex.direction as i64 + delta).rem_euclid(NB_DIRECTIONS as i64) as usize;

            // b. La case de départ, où la consommation aura lieu.
            let depart_x = apex.x as usize;
            let depart_y = apex.y as usize;
            let s = self.substrat.valeur(depart_x, depart_y);

            // c. La longueur que le substrat autorise.
            let mut longueur = if s <= 0.0 { 0.0 } else { (pas * s) / (affinite + s) };
            if cout > 0.0 && cout * longueur > s {
                longueur = s / cout;
            }

            // d. L'arrivée.
            let direction = apex.direction;
            let x1 = apex.x + longueur * COS[direction];
            let y1 = apex.y + longueur * SIN[direction];

            // e. Un apex bloqué garde sa position et sa nouvelle direction, sans rien laisser ni consommer. La
            //    borne droite de la boîte est exclue.
            if longueur <= 0.0 || x1 < 0.0 || y1 < 0.0 || x1 >= largeur || y1 >= hauteur {
                self.apex[i] = apex;
                continue;
            }

            // f. Consommer dans la case de départ.
            self.substrat.retirer(depart_x, depart_y, cout * longueur);

            // g. Laisser le segment, avancer l'apex, mettre à jour biomasse et rayon.
            let segment = Segment { apex: apex.id, temps, x0: apex.x, y0: apex.y, x1, y1 };
            self.index.ajouter(segment);
            apex.x = x1;
            apex.y = y1;
            self.apex[i] = apex;

            self.biomasse += segment.longueur();
            let dx = x1 - spore_x;
            let dy = y1 - spore_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > self.rayon {
                self.rayon = distance;
            }

            // h. Une anastomose rend l'apex inactif et empêche sa ramification.
            if let Some(reglage) = reglage_anastomose
                && self.index.chercher(x1, y1, apex.id, temps, &reglage).is_some()
            {
                self.apex[i].actif = false;
                self.nb_anastomoses += 1;
                continue;
            }

            // i. Se ramifier : un nouvel apex naît à l'arrivée, il n'agira qu'au pas suivant.
            if probabilite > 0.0 && self.alea.suivant_f64() < probabilite {
                let cote = self.alea.suivant() % 2;
                let decalage = if cote == 0 { angle_ramification } else { -angle_ramification };
                let direction = (direction as i64 + decalage).rem_euclid(NB_DIRECTIONS as i64) as usize;
                let id = self.apex.len() as u32;
                self.apex.push(Apex { id, x: x1, y: y1, direction, actif: true });
                self.nb_ramifications += 1;
            }
        }

        let releve = self.releve_courant();
        self.releves.push(releve);
    }

    /// Avance la simulation de `n` pas : strictement équivalent à `n` appels à [`Simulation::avancer`].
    pub fn avancer_n(&mut self, n: u32) {
        for _ in 0..n {
            self.avancer();
        }
    }

    /// Nombre de pas de temps joués depuis la création.
    pub fn temps(&self) -> u32 {
        self.temps
    }

    /// Nombre d'apex vivants, c'est-à-dire de pointes de la colonie.
    pub fn pointes(&self) -> usize {
        self.apex.iter().filter(|apex| apex.actif).count()
    }

    /// Nombre cumulé de ramifications depuis la création.
    pub fn ramifications(&self) -> u64 {
        self.nb_ramifications
    }

    /// Nombre cumulé d'anastomoses depuis la création.
    pub fn anastomoses(&self) -> u64 {
        self.nb_anastomoses
    }

    /// Tous les segments laissés par la colonie depuis la création, dans l'ordre fixe où ils ont été déposés.
    pub fn segments(&self) -> &[Segment] {
        self.index.segments()
    }

    /// Biomasse de la colonie : la somme des longueurs de ses segments.
    pub fn biomasse(&self) -> f64 {
        self.biomasse
    }

    /// Rayon de la colonie : la plus grande distance entre la spore et une extrémité de segment.
    pub fn rayon(&self) -> f64 {
        self.rayon
    }

    /// Le substrat courant.
    pub fn substrat(&self) -> &Substrat {
        &self.substrat
    }

    /// Le relevé du temps courant.
    pub fn releve(&self) -> Releve {
        *self.releves.last().expect("un relevé existe au temps 0")
    }

    /// Les relevés, un par pas de temps depuis le temps 0 : il y en a donc `temps() + 1`, et le relevé d'indice
    /// `t` porte `temps == t`.
    pub fn releves(&self) -> &[Releve] {
        &self.releves
    }

    /// L'écriture canonique de l'état, pour comparer « à l'octet » et pour l'empreinte.
    ///
    /// Format, dans cet ordre, sans remplissage, tous les entiers en petit-boutiste :
    ///
    /// - octets `0..4` : [`Simulation::temps`] en `u32` ;
    /// - octets `4..12` : le nombre de segments en `u64` ;
    /// - à partir de l'octet 12, les segments dans l'ordre de [`Simulation::segments`], chacun sur
    ///   [`croissance::OCTETS_PAR_SEGMENT`] octets : `apex` (`u32`), `temps` (`u32`), puis `x0`, `y0`, `x1`, `y1`,
    ///   chacun sur 8 octets, écrits par `f64::to_bits` puis en petit-boutiste ;
    /// - ensuite chaque apex dans l'ordre de son identifiant, sur 25 octets : `id` (`u32`), `x` et `y` par
    ///   `f64::to_bits` en petit-boutiste (8 octets chacun), `direction` (`u32`) et `actif` (`u8`, 1 ou 0) ;
    /// - puis les trois compteurs en `u64` : ramifications, anastomoses, pointes ;
    /// - puis [`Substrat::ecrire_canonique`], dont le format est décrit dans le module `substrat`.
    ///
    /// La graine et les réglages n'y figurent pas : deux simulations de même état ont la même écriture.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let mut octets = Vec::new();
        octets.extend_from_slice(&self.temps.to_le_bytes());
        octets.extend_from_slice(&(self.index.segments().len() as u64).to_le_bytes());
        for segment in self.index.segments() {
            octets.extend_from_slice(&segment.octets());
        }
        for apex in &self.apex {
            octets.extend_from_slice(&apex.id.to_le_bytes());
            octets.extend_from_slice(&apex.x.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.y.to_bits().to_le_bytes());
            octets.extend_from_slice(&(apex.direction as u32).to_le_bytes());
            octets.push(u8::from(apex.actif));
        }
        octets.extend_from_slice(&self.nb_ramifications.to_le_bytes());
        octets.extend_from_slice(&self.nb_anastomoses.to_le_bytes());
        octets.extend_from_slice(&(self.pointes() as u64).to_le_bytes());
        octets.extend_from_slice(&self.substrat.ecrire_canonique());
        octets
    }

    /// Empreinte de l'état : le SHA-256 de [`Simulation::ecrire_canonique`], en hexadécimal minuscule de 64
    /// caractères.
    pub fn empreinte(&self) -> String {
        empreinte::sha256_hex(&self.ecrire_canonique())
    }

    /// Le relevé de l'état courant, sans l'ajouter à la liste.
    fn releve_courant(&self) -> Releve {
        Releve {
            temps: self.temps,
            biomasse: self.biomasse,
            pointes: self.pointes(),
            ramifications: self.nb_ramifications,
            anastomoses: self.nb_anastomoses,
            rayon: self.rayon,
            substrat: self.substrat.masse_totale(),
        }
    }
}

/// Vrai si les paramètres de croissance respectent le contrat de [`croissance::Parametres`].
fn parametres_valides(parametres: &croissance::Parametres) -> bool {
    parametres.pas.is_finite()
        && parametres.pas > 0.0
        && (parametres.intervalle >= 1.0 || parametres.intervalle == f64::INFINITY)
        && (parametres.angle_ramification as usize) < NB_DIRECTIONS
        && (parametres.amplitude_marche as usize) < NB_DIRECTIONS
}

/// Pourquoi la lecture d'un scénario couplé échoue.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ErreurLecture {
    /// Ligne inconnue ou mal formée ; porte le numéro de ligne, à partir de 1.
    LigneInvalide(usize),
    /// Ligne obligatoire absente ; porte son nom, par exemple `"spore"`.
    Manquante(&'static str),
}

/// Lit un scénario couplé et rend ses réglages et sa durée en pas de temps.
///
/// Le format est décrit dans la documentation du module. Les lignes vides et celles dont le premier caractère
/// est `#` sont ignorées. Les directives obligatoires sont `boite`, `spore`, `substrat`, `croissance`,
/// `couplage` et `duree` ; `anastomose` est optionnelle et vaut `None` quand elle est absente. Le mot `infini`
/// est accepté pour l'intervalle de ramification et vaut `f64::INFINITY`.
///
/// Une ligne inconnue ou mal formée rend [`ErreurLecture::LigneInvalide`] avec son numéro, une directive
/// obligatoire absente [`ErreurLecture::Manquante`] avec son nom.
pub fn lire_scenario(texte: &str) -> Result<(Reglages, u32), ErreurLecture> {
    let mut boite: Option<(usize, usize)> = None;
    let mut spore: Option<(usize, usize)> = None;
    let mut substrat: Option<(f64, f64)> = None;
    let mut croissance: Option<croissance::Parametres> = None;
    let mut couplage: Option<(f64, f64)> = None;
    let mut anastomose: Option<anastomose::Reglage> = None;
    let mut duree: Option<u32> = None;

    for (rang, brute) in texte.lines().enumerate() {
        let numero = rang + 1;
        let ligne = brute.trim();
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let mots: Vec<&str> = ligne.split_whitespace().collect();
        match mots.as_slice() {
            ["boite", largeur, hauteur] => match (largeur.parse::<usize>(), hauteur.parse::<usize>()) {
                (Ok(largeur), Ok(hauteur)) if largeur > 0 && hauteur > 0 => {
                    boite = Some((largeur, hauteur));
                }
                _ => return Err(ErreurLecture::LigneInvalide(numero)),
            },
            ["spore", x, y] => match (x.parse::<usize>(), y.parse::<usize>()) {
                (Ok(x), Ok(y)) => spore = Some((x, y)),
                _ => return Err(ErreurLecture::LigneInvalide(numero)),
            },
            ["substrat", quantite, diffusion] => match (quantite.parse::<f64>(), diffusion.parse::<f64>()) {
                (Ok(quantite), Ok(diffusion)) => substrat = Some((quantite, diffusion)),
                _ => return Err(ErreurLecture::LigneInvalide(numero)),
            },
            ["croissance", pas, intervalle, angle, amplitude] => {
                let pas = pas.parse::<f64>();
                let intervalle = if *intervalle == "infini" { Ok(f64::INFINITY) } else { intervalle.parse::<f64>() };
                let angle = angle.parse::<u32>();
                let amplitude = amplitude.parse::<u32>();
                match (pas, intervalle, angle, amplitude) {
                    (Ok(pas), Ok(intervalle), Ok(angle_ramification), Ok(amplitude_marche)) => {
                        croissance =
                            Some(croissance::Parametres { pas, intervalle, angle_ramification, amplitude_marche });
                    }
                    _ => return Err(ErreurLecture::LigneInvalide(numero)),
                }
            }
            ["couplage", affinite, cout] => match (affinite.parse::<f64>(), cout.parse::<f64>()) {
                (Ok(affinite), Ok(cout)) => couplage = Some((affinite, cout)),
                _ => return Err(ErreurLecture::LigneInvalide(numero)),
            },
            ["anastomose", rayon, delai] => match (rayon.parse::<f64>(), delai.parse::<u32>()) {
                (Ok(rayon), Ok(delai)) => anastomose = Some(anastomose::Reglage { rayon, delai }),
                _ => return Err(ErreurLecture::LigneInvalide(numero)),
            },
            ["duree", n] => match n.parse::<u32>() {
                Ok(n) => duree = Some(n),
                Err(_) => return Err(ErreurLecture::LigneInvalide(numero)),
            },
            _ => return Err(ErreurLecture::LigneInvalide(numero)),
        }
    }

    let Some((largeur, hauteur)) = boite else {
        return Err(ErreurLecture::Manquante("boite"));
    };
    let Some(spore) = spore else {
        return Err(ErreurLecture::Manquante("spore"));
    };
    let Some((substrat_initial, diffusion)) = substrat else {
        return Err(ErreurLecture::Manquante("substrat"));
    };
    let Some(croissance) = croissance else {
        return Err(ErreurLecture::Manquante("croissance"));
    };
    let Some((affinite, cout)) = couplage else {
        return Err(ErreurLecture::Manquante("couplage"));
    };
    let Some(duree) = duree else {
        return Err(ErreurLecture::Manquante("duree"));
    };

    let reglages =
        Reglages { largeur, hauteur, spore, substrat_initial, diffusion, croissance, affinite, cout, anastomose };
    Ok((reglages, duree))
}

/// Comme [`lire_scenario`], avec deux directives de plus, optionnelles et répétables, dans n'importe quel ordre :
///
/// - `source <x> <y> <rayon> <quantite>` : trois entiers non signés puis un nombre ;
/// - `obstacle <x0> <y0> <x1> <y1>` : quatre entiers non signés, avec `x0 ≤ x1` et `y0 ≤ y1`.
///
/// Une ligne mal formée rend [`ErreurLecture::LigneInvalide`] avec son numéro. Les contrats « dans la boîte » ne se
/// vérifient pas ici mais à la construction ([`Simulation::nouvelle_avec_milieu`]). Un texte sans ces directives donne
/// les mêmes réglages et la même durée que [`lire_scenario`], et un [`Milieu`] vide. [`lire_scenario`] ne change pas :
/// elle continue de refuser `source` et `obstacle`.
pub fn lire_scenario_complet(texte: &str) -> Result<(Reglages, Milieu, u32), ErreurLecture> {
    todo!("étape 5 : lire_scenario_complet {texte:?}")
}
