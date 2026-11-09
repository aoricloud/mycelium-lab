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
//! 3. si la longueur est positive et que l'arrivée reste dans `[0, largeur) × [0, hauteur)` **et que le segment ne
//!    traverse aucune case bloquée**, consomme `cout · longueur` dans la case de départ et laisse un [`Segment`] ;
//!    sinon il garde sa position et sa nouvelle direction, sans rien consommer ni se ramifier (un segment qui
//!    traverserait une case bloquée est un **contact**, traité par le thigmotropisme, voir plus bas) ;
//! 4. s'il approche un segment étranger assez ancien, et qu'aucun obstacle ne les sépare, s'anastomose et devient
//!    inactif ;
//! 5. sinon se ramifie avec la probabilité `1 / intervalle`, en créant un apex qui n'agira qu'au pas suivant.
//!
//! Un relevé est enregistré après chaque pas, avec celui du temps 0 : il y en a donc `temps() + 1`.
//!
//! # Les obstacles et le thigmotropisme (T2)
//!
//! Un obstacle n'est jamais touché, à tout réglage ([`Substrat::segment_libre`], règle du plancher). Si le segment
//! prévu touche une case bloquée, c'est un **contact**. À `thigmotropisme 0`, la pointe reste où elle est, avec sa
//! nouvelle direction, comme le cas « apex bloqué » : elle retentera au pas suivant. À `thigmotropisme a > 0`, la face
//! touchée ([`Substrat::face_de_contact`]) donne l'angle de contact `α` ([`tropisme::angle_de_contact`]) :
//!
//! - `α < a` : la pointe **longe** l'obstacle : elle mémorise `(cap, g)`, `cap` étant sa direction `d` de ce pas et `g`
//!   la direction de glissement ([`tropisme::direction_de_glissement`]), et refait son pas dans la direction `g` ; si ce
//!   pas sort de la boîte, elle reste là et oublie ; s'il touche aussi (coin concave), c'est un arrêt ;
//! - `α ≥ a` : elle **s'arrête et se ramifie** : elle devient inactive (sans mémoire) et, pour chacune des deux
//!   directions parallèles à la face ([`tropisme::branches_de_contact`], dans cet ordre), une pointe neuve, **sans
//!   mémoire**, naît à sa position si un pas plein (`pas`) dans cette direction est libre ; chaque naissance est une
//!   ramification.
//!
//! Une pointe qui a une mémoire `(cap, g)` tire sa marche au hasard (résultat écrasé), puis toujours un écart `δ` dans
//! `-MEMOIRE_DE_DIRECTION..=MEMOIRE_DE_DIRECTION`, et ne subit pas le chimiotropisme. Si un pas plein dans la direction
//! `cap` est libre (le bout du mur est passé) **et** si le pas réel dans la direction `cap + δ` l'est aussi, elle
//! retrouve son cap : direction `cap + δ`, mémoire effacée ; sinon elle continue de longer, direction `g`. Une pointe
//! bloquée (longueur nulle ou arrivée hors de la boîte), anastomosée ou arrêtée perd sa mémoire.
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
use crate::tropisme::{self, Tropismes};

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
    /// Un tropisme est hors contrat : `chimiotropisme` non fini, négatif ou supérieur à 45
    /// ([`crate::tropisme::FORCE_MAXIMALE`]), ou `thigmotropisme` non fini, négatif ou supérieur à 90
    /// ([`crate::tropisme::ANGLE_CRITIQUE_MAXIMAL`]).
    TropismeInvalide,
}

/// Un apex de la simulation : sa position, sa direction, son état et, s'il longe un obstacle, sa mémoire `(cap, g)`.
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
    /// La mémoire d'une pointe qui longe un obstacle : `(cap, g)`, la direction d'avant le contact et la direction
    /// de glissement. `None` pour toute autre pointe.
    memoire: Option<(usize, usize)>,
}

/// L'état simulé complet : les apex, le substrat et les nombres cumulés.
///
/// Tous les champs sont privés ; l'état ne s'observe que par les accesseurs de [`Simulation`], dans un ordre fixe,
/// et se compare « à l'octet » par [`Simulation::ecrire_canonique`].
pub struct Simulation {
    reglages: Reglages,
    milieu: Milieu,
    tropismes: Tropismes,
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
        Simulation::nouvelle_avec_milieu(reglages, Milieu::default(), graine)
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
        Simulation::nouvelle_avec_tropismes(reglages, milieu, Tropismes::default(), graine)
    }

    /// Comme [`Simulation::nouvelle_avec_milieu`], avec des [`Tropismes`] : avec `Tropismes::default()` (force nulle),
    /// l'état est le même, octet pour octet, que celui de `nouvelle_avec_milieu`, pour toute graine et tout milieu.
    ///
    /// Refuse avec [`ErreurSimulation::TropismeInvalide`] un `chimiotropisme` non fini, négatif ou supérieur à 45, ou
    /// un `thigmotropisme` non fini, négatif ou supérieur à 90 ; les contrôles des réglages et du milieu sont ceux
    /// d'avant, et passent en premier.
    pub fn nouvelle_avec_tropismes(
        reglages: Reglages,
        milieu: Milieu,
        tropismes: Tropismes,
        graine: u64,
    ) -> Result<Simulation, ErreurSimulation> {
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

        // Les contrats du milieu se vérifient ici, pas à la lecture d'un scénario : le centre d'une source est dans
        // la boîte (son disque, lui, peut déborder et sera tronqué), sa quantité est finie et positive ou nulle ;
        // un obstacle est dans la boîte, à l'endroit, et ne contient pas la case de la spore.
        for source in &milieu.sources {
            if source.x >= reglages.largeur
                || source.y >= reglages.hauteur
                || !source.quantite.is_finite()
                || source.quantite < 0.0
            {
                return Err(ErreurSimulation::MilieuInvalide);
            }
        }
        for obstacle in &milieu.obstacles {
            if obstacle.x0 > obstacle.x1
                || obstacle.y0 > obstacle.y1
                || obstacle.x1 >= reglages.largeur
                || obstacle.y1 >= reglages.hauteur
            {
                return Err(ErreurSimulation::MilieuInvalide);
            }
            let contient_la_spore = (obstacle.x0..=obstacle.x1).contains(&reglages.spore.0)
                && (obstacle.y0..=obstacle.y1).contains(&reglages.spore.1);
            if contient_la_spore {
                return Err(ErreurSimulation::MilieuInvalide);
            }
        }

        // Le tropisme se vérifie après les réglages et le milieu, et avant de remplir le substrat : `-0.0` et les
        // bornes 0 et 45 sont acceptées, comme les bornes 0 et 90 du thigmotropisme.
        if !tropismes.chimiotropisme.is_finite()
            || tropismes.chimiotropisme < 0.0
            || tropismes.chimiotropisme > tropisme::FORCE_MAXIMALE
            || !tropismes.thigmotropisme.is_finite()
            || tropismes.thigmotropisme < 0.0
            || tropismes.thigmotropisme > tropisme::ANGLE_CRITIQUE_MAXIMAL
        {
            return Err(ErreurSimulation::TropismeInvalide);
        }

        // Au temps 0, chaque case vaut le substrat initial, remplacé par la dernière source qui la contient. La
        // boucle et son ordre sont ceux d'avant T0 : sans source, chaque case reçoit exactement `substrat_initial`.
        for y in 0..reglages.hauteur {
            for x in 0..reglages.largeur {
                let mut quantite = reglages.substrat_initial;
                for source in &milieu.sources {
                    if dans_le_disque(x, y, source.x, source.y, source.rayon) {
                        quantite = source.quantite;
                    }
                }
                substrat.deposer(x, y, quantite);
            }
        }

        // Les obstacles sont posés après les sources, donc une case d'obstacle vaut `0.0` même sous une source.
        for obstacle in &milieu.obstacles {
            substrat.bloquer(obstacle.x0, obstacle.y0, obstacle.x1, obstacle.y1);
        }

        // Le premier tirage fixe la direction du premier apex, comme `Colonie::depuis_spore`. Le milieu n'en tire
        // aucun : sans source ni obstacle, l'état est celui d'avant T0, octet pour octet.
        let mut alea = Alea::new(graine);
        let direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
        let premier = Apex {
            id: 0,
            x: reglages.spore.0 as f64 + 0.5,
            y: reglages.spore.1 as f64 + 0.5,
            direction,
            actif: true,
            memoire: None,
        };
        let index = anastomose::Index::new(reglages.largeur, reglages.hauteur);

        let mut simulation = Simulation {
            reglages,
            milieu,
            tropismes,
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

    /// Les réglages de la simulation, tels qu'ils ont été donnés (lecture seule : la boîte, le coût en substrat, etc.).
    /// L'accesseur ne change aucun état.
    pub fn reglages(&self) -> &Reglages {
        &self.reglages
    }

    /// Le milieu de la simulation : ses taches et ses obstacles, tels qu'ils ont été donnés.
    pub fn milieu(&self) -> &Milieu {
        &self.milieu
    }

    /// Les tropismes de la simulation, tels qu'ils ont été donnés.
    pub fn tropismes(&self) -> &Tropismes {
        &self.tropismes
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
        let force_chimiotropique = self.tropismes.chimiotropisme;
        let angle_critique = self.tropismes.thigmotropisme;
        let a_des_obstacles = !self.milieu.obstacles.is_empty();

        for i in 0..nb_apex_au_debut {
            if !self.apex[i].actif {
                continue;
            }
            let mut apex = self.apex[i];

            // a. Tourner, même si l'apex n'avance pas ensuite.
            let delta = (self.alea.suivant() % (2 * amplitude + 1)) as i64 - amplitude as i64;
            apex.direction = (apex.direction as i64 + delta).rem_euclid(NB_DIRECTIONS as i64) as usize;

            // b-c. La case de départ, où la consommation aura lieu, la valeur `s` du substrat et la longueur que le
            //      substrat autorise. Cette lecture passe avant le chimiotropisme et ne tire aucun nombre.
            let depart_x = apex.x as usize;
            let depart_y = apex.y as usize;
            let s = self.substrat.valeur(depart_x, depart_y);
            let mut longueur = if s <= 0.0 { 0.0 } else { (pas * s) / (affinite + s) };
            if cout > 0.0 && cout * longueur > s {
                longueur = s / cout;
            }

            // a ter. La mémoire d'une pointe qui longe : la marche au hasard est écrasée, un écart `δ` est toujours
            //        tiré, et la pointe retrouve son cap si le bout du mur est passé.
            if let Some((cap, g)) = apex.memoire {
                let ecart = (self.alea.suivant() % u64::from(2 * tropisme::MEMOIRE_DE_DIRECTION + 1)) as i64
                    - i64::from(tropisme::MEMOIRE_DE_DIRECTION);
                let retrouvee = (cap as i64 + ecart).rem_euclid(NB_DIRECTIONS as i64) as usize;
                if self.pas_libre(apex.x, apex.y, pas, cap) && self.pas_libre(apex.x, apex.y, longueur, retrouvee) {
                    apex.direction = retrouvee;
                    apex.memoire = None;
                } else {
                    apex.direction = g;
                }
            } else if force_chimiotropique > 0.0 {
                // Le chimiotropisme de T1 : une pointe qui a une mémoire n'y est pas soumise.
                let gradient = self.substrat.gradient(apex.x as usize, apex.y as usize);
                apex.direction =
                    tropisme::tourner_chimiotropique(apex.direction, gradient, force_chimiotropique, &mut self.alea);
            }

            // d. L'arrivée, dans la direction de la pointe.
            let mut direction = apex.direction;
            let mut x1 = apex.x + longueur * COS[direction];
            let mut y1 = apex.y + longueur * SIN[direction];

            // e. Un apex bloqué garde sa position et sa nouvelle direction, sans rien laisser ni consommer, et perd sa
            //    mémoire. La borne droite de la boîte est exclue.
            if longueur <= 0.0 || x1 < 0.0 || y1 < 0.0 || x1 >= largeur || y1 >= hauteur {
                apex.memoire = None;
                self.apex[i] = apex;
                continue;
            }

            // e bis. S'il y a des obstacles et que le segment touche une case bloquée, c'est un contact.
            let mut arret: Option<tropisme::Face> = None;
            if a_des_obstacles && let Some(face) = self.substrat.face_de_contact(apex.x, apex.y, x1, y1) {
                if angle_critique <= 0.0 {
                    // À angle nul, la pointe reste, garde sa direction, et retentera au pas suivant.
                    self.apex[i] = apex;
                    continue;
                }
                if (tropisme::angle_de_contact(direction, face) as f64) < angle_critique {
                    // Elle longe : mémoire du cap et de la direction de glissement, puis le pas est refait.
                    let g = tropisme::direction_de_glissement(direction, face);
                    apex.memoire = Some((direction, g));
                    apex.direction = g;
                    direction = g;
                    x1 = apex.x + longueur * COS[g];
                    y1 = apex.y + longueur * SIN[g];
                    if x1 < 0.0 || y1 < 0.0 || x1 >= largeur || y1 >= hauteur {
                        apex.memoire = None;
                        self.apex[i] = apex;
                        continue;
                    }
                    if let Some(face2) = self.substrat.face_de_contact(apex.x, apex.y, x1, y1) {
                        // Un coin concave : c'est un arrêt contre la seconde face.
                        arret = Some(face2);
                    }
                } else {
                    arret = Some(face);
                }
            }

            // Un arrêt : la pointe devient inactive, sans mémoire, et chaque branche libre naît à sa position.
            if let Some(face) = arret {
                apex.actif = false;
                apex.memoire = None;
                self.apex[i] = apex;
                let (a, b) = tropisme::branches_de_contact(face);
                for b in [a, b] {
                    if self.pas_libre(apex.x, apex.y, pas, b) {
                        let id = self.apex.len() as u32;
                        self.apex.push(Apex { id, x: apex.x, y: apex.y, direction: b, actif: true, memoire: None });
                        self.nb_ramifications += 1;
                    }
                }
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

            // h. Une anastomose rend l'apex inactif, lui fait perdre sa mémoire et empêche sa ramification. S'il y a
            //    des obstacles, un candidat que sépare un obstacle est écarté et la recherche continue.
            if let Some(reglage) = reglage_anastomose {
                let candidat = if a_des_obstacles {
                    self.index.chercher_si(x1, y1, apex.id, temps, &reglage, |_, (px, py)| {
                        self.substrat.segment_libre(x1, y1, px.clamp(0.0, largeur), py.clamp(0.0, hauteur))
                    })
                } else {
                    self.index.chercher(x1, y1, apex.id, temps, &reglage)
                };
                if candidat.is_some() {
                    self.apex[i].actif = false;
                    self.apex[i].memoire = None;
                    self.nb_anastomoses += 1;
                    continue;
                }
            }

            // i. Se ramifier : un nouvel apex naît à l'arrivée, sans mémoire, dans la direction du pas de la mère
            //    (celle de glissement pour une pointe qui longe), à plus ou moins `angle_ramification`. Il n'agira
            //    qu'au pas suivant, et la mère garde sa mémoire.
            if probabilite > 0.0 && self.alea.suivant_f64() < probabilite {
                let cote = self.alea.suivant() % 2;
                let decalage = if cote == 0 { angle_ramification } else { -angle_ramification };
                let direction = (direction as i64 + decalage).rem_euclid(NB_DIRECTIONS as i64) as usize;
                let id = self.apex.len() as u32;
                self.apex.push(Apex { id, x: x1, y: y1, direction, actif: true, memoire: None });
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

    /// Vrai si un pas de longueur `longueur` dans la direction `direction`, depuis `(x, y)`, arrive dans la boîte
    /// (borne droite et borne basse exclues) et ne touche aucune case bloquée.
    fn pas_libre(&self, x: f64, y: f64, longueur: f64, direction: usize) -> bool {
        let x1 = x + longueur * COS[direction];
        let y1 = y + longueur * SIN[direction];
        x1 >= 0.0
            && y1 >= 0.0
            && x1 < self.reglages.largeur as f64
            && y1 < self.reglages.hauteur as f64
            && self.substrat.segment_libre(x, y, x1, y1)
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
    /// - puis [`Substrat::ecrire_canonique`], dont le format est décrit dans le module `substrat` ;
    /// - puis, seulement si le milieu n'est pas vide ou si un tropisme (`chimiotropisme` ou `thigmotropisme`) n'est pas
    ///   nul, la queue du milieu : le nombre de sources en `u64`, chacune (`x`, `y`, `rayon` en `u32`, `quantite` par
    ///   `f64::to_bits`) en petit-boutiste, le nombre d'obstacles en `u64`, puis chacun (`x0`, `y0`, `x1`, `y1` en
    ///   `u32`), tout dans l'ordre des vecteurs du milieu (un milieu vide, avec un tropisme actif, écrit deux `u64` à
    ///   zéro) ;
    /// - puis, seulement si `chimiotropisme` n'est pas nul, `chimiotropisme.to_bits()` en petit-boutiste (8 octets) ;
    /// - puis, seulement si `thigmotropisme` n'est pas nul, `thigmotropisme.to_bits()` en petit-boutiste (8 octets),
    ///   suivi, pour chaque apex dans l'ordre des identifiants, de 9 octets : un drapeau (`u8`, 1 si l'apex a une
    ///   mémoire), puis `cap` (`u32`) et `g` (`u32`) en petit-boutiste, nuls sans mémoire.
    ///   Une force nulle (ou `-0.0`), un angle critique nul (ou `-0.0`) et un milieu vide n'ajoutent rien : l'écriture
    ///   est celle d'avant T0.
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
        if !self.milieu.sources.is_empty()
            || !self.milieu.obstacles.is_empty()
            || self.tropismes.chimiotropisme != 0.0
            || self.tropismes.thigmotropisme != 0.0
        {
            octets.extend_from_slice(&(self.milieu.sources.len() as u64).to_le_bytes());
            for source in &self.milieu.sources {
                octets.extend_from_slice(&(source.x as u32).to_le_bytes());
                octets.extend_from_slice(&(source.y as u32).to_le_bytes());
                octets.extend_from_slice(&(source.rayon as u32).to_le_bytes());
                octets.extend_from_slice(&source.quantite.to_bits().to_le_bytes());
            }
            octets.extend_from_slice(&(self.milieu.obstacles.len() as u64).to_le_bytes());
            for obstacle in &self.milieu.obstacles {
                octets.extend_from_slice(&(obstacle.x0 as u32).to_le_bytes());
                octets.extend_from_slice(&(obstacle.y0 as u32).to_le_bytes());
                octets.extend_from_slice(&(obstacle.x1 as u32).to_le_bytes());
                octets.extend_from_slice(&(obstacle.y1 as u32).to_le_bytes());
            }
        }
        if self.tropismes.chimiotropisme != 0.0 {
            octets.extend_from_slice(&self.tropismes.chimiotropisme.to_bits().to_le_bytes());
        }
        if self.tropismes.thigmotropisme != 0.0 {
            octets.extend_from_slice(&self.tropismes.thigmotropisme.to_bits().to_le_bytes());
            for apex in &self.apex {
                match apex.memoire {
                    Some((cap, g)) => {
                        octets.push(1);
                        octets.extend_from_slice(&(cap as u32).to_le_bytes());
                        octets.extend_from_slice(&(g as u32).to_le_bytes());
                    }
                    None => {
                        octets.push(0);
                        octets.extend_from_slice(&0u32.to_le_bytes());
                        octets.extend_from_slice(&0u32.to_le_bytes());
                    }
                }
            }
        }
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

/// Vrai si la case `(i, j)` est dans le disque de centre `(x, y)` et de rayon `rayon`, en entiers signés sur 128
/// bits : un très grand rayon ne déborde pas. Les carrés sont saturants pour rester sûrs même au-delà de 2⁶³.
fn dans_le_disque(i: usize, j: usize, x: usize, y: usize, rayon: usize) -> bool {
    let di = i as i128 - x as i128;
    let dj = j as i128 - y as i128;
    let r = rayon as i128;
    di.saturating_mul(di).saturating_add(dj.saturating_mul(dj)) <= r.saturating_mul(r)
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
    let (reglages, _, _, duree) = lire_scenario_commun(texte, false, false)?;
    Ok((reglages, duree))
}

/// La lecture commune à [`lire_scenario`] (`avec_milieu == false`, `avec_tropismes == false`),
/// [`lire_scenario_complet`] (`avec_milieu == true`, `avec_tropismes == false`) et
/// [`lire_scenario_tropismes`] (`avec_milieu == true`, `avec_tropismes == true`). Quand `avec_milieu` est faux, les
/// directives `source` et `obstacle` restent des lignes inconnues ; quand `avec_tropismes` est faux, `chimiotropisme`
/// et `thigmotropisme` en sont aussi.
fn lire_scenario_commun(
    texte: &str,
    avec_milieu: bool,
    avec_tropismes: bool,
) -> Result<(Reglages, Milieu, Tropismes, u32), ErreurLecture> {
    let mut boite: Option<(usize, usize)> = None;
    let mut spore: Option<(usize, usize)> = None;
    let mut substrat: Option<(f64, f64)> = None;
    let mut croissance: Option<croissance::Parametres> = None;
    let mut couplage: Option<(f64, f64)> = None;
    let mut anastomose: Option<anastomose::Reglage> = None;
    let mut duree: Option<u32> = None;
    let mut chimiotropisme: Option<f64> = None;
    let mut thigmotropisme: Option<f64> = None;
    let mut milieu = Milieu::default();

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
            ["chimiotropisme", force] if avec_tropismes => {
                // Seule la lecture se fait ici : `nan`, `inf` et les valeurs hors de `0.0..=45.0` se lisent, c'est
                // la construction qui les refuse. Une seconde ligne, même identique, est une faute.
                match force.parse::<f64>() {
                    Ok(force) if chimiotropisme.is_none() => chimiotropisme = Some(force),
                    _ => return Err(ErreurLecture::LigneInvalide(numero)),
                }
            }
            ["thigmotropisme", angle] if avec_tropismes => {
                // Comme le chimiotropisme : la valeur se lit telle quelle (`parse::<f64>`) et n'est vérifiée qu'à la
                // construction. Une seconde ligne, même identique, est une faute.
                match angle.parse::<f64>() {
                    Ok(angle) if thigmotropisme.is_none() => thigmotropisme = Some(angle),
                    _ => return Err(ErreurLecture::LigneInvalide(numero)),
                }
            }
            ["source", x, y, rayon, quantite] if avec_milieu => {
                match (x.parse::<usize>(), y.parse::<usize>(), rayon.parse::<usize>(), quantite.parse::<f64>()) {
                    (Ok(x), Ok(y), Ok(rayon), Ok(quantite)) => {
                        milieu.sources.push(Source { x, y, rayon, quantite });
                    }
                    _ => return Err(ErreurLecture::LigneInvalide(numero)),
                }
            }
            ["obstacle", x0, y0, x1, y1] if avec_milieu => {
                match (x0.parse::<usize>(), y0.parse::<usize>(), x1.parse::<usize>(), y1.parse::<usize>()) {
                    (Ok(x0), Ok(y0), Ok(x1), Ok(y1)) if x0 <= x1 && y0 <= y1 => {
                        milieu.obstacles.push(Obstacle { x0, y0, x1, y1 });
                    }
                    _ => return Err(ErreurLecture::LigneInvalide(numero)),
                }
            }
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
    let mut tropismes = Tropismes::default();
    if let Some(chimiotropisme) = chimiotropisme {
        tropismes.chimiotropisme = chimiotropisme;
    }
    if let Some(thigmotropisme) = thigmotropisme {
        tropismes.thigmotropisme = thigmotropisme;
    }
    Ok((reglages, milieu, tropismes, duree))
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
    let (reglages, milieu, _, duree) = lire_scenario_commun(texte, true, false)?;
    Ok((reglages, milieu, duree))
}

/// Comme [`lire_scenario_complet`], avec une directive de plus, optionnelle et au plus une fois, à n'importe quelle
/// place :
///
/// - `chimiotropisme <force>` : un nombre (`f64`), la force du chimiotropisme ;
/// - `thigmotropisme <angle>` : un nombre (`f64`), l'angle critique du thigmotropisme, en degrés.
///
/// Une seconde ligne `chimiotropisme` ou `thigmotropisme`, ou une ligne mal formée (mot en trop ou en moins, nombre
/// illisible), rend [`ErreurLecture::LigneInvalide`] avec son numéro. La valeur n'est pas vérifiée ici mais à la
/// construction ([`Simulation::nouvelle_avec_tropismes`]). Sans ces lignes, les [`Tropismes`] sont ceux de
/// `Tropismes::default()`. [`lire_scenario`] et [`lire_scenario_complet`] ne changent pas : elles continuent de refuser
/// ces directives.
pub fn lire_scenario_tropismes(texte: &str) -> Result<(Reglages, Milieu, Tropismes, u32), ErreurLecture> {
    lire_scenario_commun(texte, true, true)
}
