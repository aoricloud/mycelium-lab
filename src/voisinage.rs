//! Voisinage : la colonie de la figure 3 de Meškauskas, Fricker et Moore (2004).
//!
//! Le module décrit une colonie qui pousse depuis une spore, avec ramification selon la densité, et les mesures
//! qui servent à reproduire la figure publiée : ajustement linéaire du rayon, vitesses glissantes et exposant de
//! la croissance.
//!
//! # Le modèle
//!
//! Une colonie est un ensemble d'apex, chacun portant une position et un cap (un vecteur unitaire), et une suite de
//! segments d'hyphe laissés derrière eux. À chaque pas de temps, tous les apex présents au début du pas lisent le
//! même état :
//!
//! 1. le **champ de densité** en leur pointe (éq. 3 et 4 de Meškauskas, McNulty et Moore, 2004) est la somme des
//!    contributions des segments déjà laissés, sauf ceux de leur section courante, une section étant l'internode
//!    entre deux ramifications : un segment d'une section de `n` segments pèse `lc / n` et compte pour
//!    `1 / (d² + ε²)` à la distance `d`, les segments au-delà de `portee` étant ignorés. La section que la pointe est
//!    en train de faire pousser est exclue : une pointe ne se repousse pas elle-même, mais les internodes plus
//!    anciens de la même hyphe sont des voisins comme les autres ;
//! 2. le **vecteur d'évitement** (éq. 11) somme, pour chaque segment, la direction qui s'en écarte, pondérée par le
//!    poids `lc / n` de sa section et par `1 / (d² + ε²)³` : chaque segment repousse la pointe, d'autant plus fort
//!    qu'il est proche et lourd ;
//! 3. le **cap** (éq. 13) mélange l'ancien cap et la direction d'évitement, avec le poids `persistance` : `k = 1`
//!    supprime l'autotropisme et un champ nul laisse le cap inchangé ;
//! 4. l'apex avance de `pas` dans le nouveau cap et laisse un segment ;
//! 5. si la densité lue à sa pointe est sous `seuil_densite`, il se ramifie avec la probabilité
//!    `probabilite_ramification` : un apex neuf naît au point où le parent se trouvait au début du pas, son cap
//!    faisant un angle de `0` à `angle_max` degrés avec celui du parent, d'un côté ou de l'autre. Il avance dès ce
//!    pas de naissance, mais ne se ramifie qu'aux pas suivants.
//!
//! # Le déterminisme
//!
//! Aucune fonction de la bibliothèque mathématique du système n'est appelée : les directions de départ et de
//! ramification viennent de [`croissance::COS`] et [`croissance::SIN`], les seules racines carrées servent à
//! normaliser un cap ou à mesurer une distance, et tout le hasard vient de [`crate::alea::Alea`]. Les apex sont
//! lus puis écrits dans un ordre fixe, et un pas sépare la perception de l'avancée : tous les apex présents au
//! début du pas lisent le même état, puis avancent avec les caps et les densités mémorisés (mise à jour synchrone),
//! les apex nés pendant le pas avançant ensuite, dans l'ordre de leur naissance.

use crate::alea::Alea;
use crate::croissance;
use crate::empreinte::sha256_hex;

/// Paramètres d'une colonie du modèle publié.
///
/// # Contrat
///
/// - [`pas`](Parametres::pas) est fini et strictement positif ;
/// - [`persistance`](Parametres::persistance) est dans `[0, 1]` ; `1,0` supprime l'autotropisme ;
/// - [`seuil_densite`](Parametres::seuil_densite) est strictement positif, ou `f64::INFINITY` pour n'imposer
///   aucune limite de densité ;
/// - [`probabilite_ramification`](Parametres::probabilite_ramification) est dans `[0, 1]` ;
/// - [`lc`](Parametres::lc), [`epsilon`](Parametres::epsilon) et [`portee`](Parametres::portee) sont finis et
///   strictement positifs ;
/// - [`angle_max`](Parametres::angle_max) est au plus 180 ;
/// - [`nb_apex_depart`](Parametres::nb_apex_depart) est au moins 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parametres {
    /// Vitesse d'un apex, en unités de longueur par pas de temps.
    pub pas: f64,
    /// Persistance de la marche, `k`, dans `[0, 1]` ; `1,0` supprime l'autotropisme.
    pub persistance: f64,
    /// Seuil de densité, strictement positif ; `f64::INFINITY` supprime la limite de densité.
    pub seuil_densite: f64,
    /// Probabilité qu'un apex se ramifie à un pas donné, dans `[0, 1]`.
    pub probabilite_ramification: f64,
    /// Constante `lc` de l'éq. 3 : poids total d'une section dans le champ de densité (un segment d'une section de
    /// `n` segments pèse `lc / n`), finie et strictement positive. Ce n'est pas une portée.
    pub lc: f64,
    /// Adoucissement `epsilon` du noyau : un segment à la distance `d` compte pour `1 / (d² + epsilon²)`, ce qui évite
    /// la divergence à distance nulle ; fini et strictement positif.
    pub epsilon: f64,
    /// Coupure du champ de densité : les segments à plus de `portee` d'une pointe ne comptent pas ; finie et
    /// strictement positive.
    pub portee: f64,
    /// Angle maximal de ramification, en degrés entiers, au plus 180.
    pub angle_max: u32,
    /// Nombre d'apex au départ, au moins 1.
    pub nb_apex_depart: usize,
}

impl Parametres {
    /// Les paramètres publiés de la figure 3 : pas `1,0` ; persistance `0,1` ; seuil de densité `0,06` ;
    /// probabilité de ramification `0,4` ; `lc` `20,0` ; `epsilon` `1,0` ; `portee` `60,0` ; angle maximal
    /// `180` degrés ; un seul apex au départ.
    pub fn figure_3() -> Parametres {
        Parametres {
            pas: 1.0,
            persistance: 0.1,
            seuil_densite: 0.06,
            probabilite_ramification: 0.4,
            lc: 20.0,
            epsilon: 1.0,
            portee: 60.0,
            angle_max: 180,
            nb_apex_depart: 1,
        }
    }

    /// Vérifie le contrat décrit sur [`Parametres`], et panique s'il est rompu.
    fn verifier(&self) {
        assert!(self.pas.is_finite() && self.pas > 0.0, "pas doit être fini et strictement positif : {}", self.pas);
        assert!(
            self.persistance >= 0.0 && self.persistance <= 1.0,
            "persistance doit être dans [0, 1] : {}",
            self.persistance
        );
        assert!(self.seuil_densite > 0.0, "seuil_densite doit être strictement positif : {}", self.seuil_densite);
        assert!(
            self.probabilite_ramification >= 0.0 && self.probabilite_ramification <= 1.0,
            "probabilite_ramification doit être dans [0, 1] : {}",
            self.probabilite_ramification
        );
        assert!(self.lc.is_finite() && self.lc > 0.0, "lc doit être fini et strictement positif : {}", self.lc);
        assert!(
            self.epsilon.is_finite() && self.epsilon > 0.0,
            "epsilon doit être fini et strictement positif : {}",
            self.epsilon
        );
        assert!(
            self.portee.is_finite() && self.portee > 0.0,
            "portee doit être finie et strictement positive : {}",
            self.portee
        );
        assert!(self.angle_max <= 180, "angle_max doit être au plus 180 : {}", self.angle_max);
        assert!(self.nb_apex_depart >= 1, "nb_apex_depart doit être au moins 1");
    }
}

/// Un apex vivant d'une colonie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apex {
    /// Identifiant, unique dans la colonie.
    pub id: u32,
    /// Abscisse de la pointe.
    pub x: f64,
    /// Ordonnée de la pointe.
    pub y: f64,
    /// Composante `x` du cap, unitaire avec `hy`.
    pub hx: f64,
    /// Composante `y` du cap, unitaire avec `hx`.
    pub hy: f64,
    /// Numéro de section de l'apex.
    pub section: u32,
}

/// Une ramification survenue pendant la croissance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramification {
    /// Numéro du pas de temps (à partir de 1) où la ramification a eu lieu.
    pub temps: u32,
    /// Identifiant de l'apex parent, celui qui s'est ramifié.
    pub apex: u32,
    /// Densité lue à la pointe du parent au moment de la ramification.
    pub densite: f64,
}

/// Taille en octets de la partie variable de l'écriture canonique d'un apex : `id`, `section`, puis quatre `f64`.
const OCTETS_PAR_APEX: usize = 4 + 4 + 4 * 8;

/// Une colonie qui pousse depuis une spore.
///
/// Une `Colonie` doit rester `Send + Sync` : les tests la rangent dans un `static` (`std::sync::OnceLock`).
pub struct Colonie {
    parametres: Parametres,
    centre: (f64, f64),
    apex: Vec<Apex>,
    segments: Vec<croissance::Segment>,
    /// Numéro de section de chaque segment, en regard de [`Colonie::segments`].
    segments_section: Vec<u32>,
    /// Nombre de segments déjà laissés par chaque section : `nb_segments_par_section[s]` est le `n` de l'éq. 3
    /// pour la section `s`. Une section est un internode, la portion d'hyphe entre deux ramifications.
    nb_segments_par_section: Vec<u32>,
    temps: u32,
    ramifications: Vec<Ramification>,
    rayon: f64,
    rayons: Vec<f64>,
    pointes_par_pas: Vec<u32>,
    alea: Alea,
    /// Index spatial des segments, pour ne parcourir que le voisinage d'une pointe.
    index: IndexSegments,
}

impl Colonie {
    /// Crée une colonie dont la spore est au `centre`, avec les `parametres` et la `graine` donnés.
    ///
    /// # Panics
    ///
    /// Panique si les paramètres sont hors du contrat décrit sur [`Parametres`].
    pub fn depuis_spore(centre: (f64, f64), parametres: Parametres, graine: u64) -> Colonie {
        parametres.verifier();
        let mut alea = Alea::new(graine);

        // Les apex de départ forment une étoile régulière : des caps espacés de `360 / n` degrés, tournés d'un
        // angle tiré au hasard pour que la graine serve dès l'instant zéro. Chaque apex est sa propre section.
        let rotation = (alea.suivant() % croissance::NB_DIRECTIONS as u64) as u32;
        let mut apex = Vec::with_capacity(parametres.nb_apex_depart);
        for i in 0..parametres.nb_apex_depart {
            let pas_angulaire = (i as u32 * croissance::NB_DIRECTIONS as u32) / parametres.nb_apex_depart as u32;
            let direction = (rotation + pas_angulaire) % croissance::NB_DIRECTIONS as u32;
            let d = direction as usize;
            apex.push(Apex {
                id: i as u32,
                x: centre.0,
                y: centre.1,
                hx: croissance::COS[d],
                hy: croissance::SIN[d],
                section: i as u32,
            });
        }

        Colonie {
            parametres,
            centre,
            apex,
            segments: Vec::new(),
            segments_section: Vec::new(),
            nb_segments_par_section: vec![0; parametres.nb_apex_depart],
            temps: 0,
            ramifications: Vec::new(),
            rayon: 0.0,
            rayons: Vec::new(),
            pointes_par_pas: Vec::new(),
            alea,
            index: IndexSegments::nouveau(centre, parametres.pas, parametres.portee),
        }
    }

    /// Avance la colonie d'un pas de temps.
    pub fn avancer(&mut self) {
        self.temps += 1;
        let nb_depart = self.apex.len();

        // Temps 1 : perception. Tous les apex lisent le champ de l'état du début du pas ; les caps et les densités
        // sont mémorisés avant qu'aucun segment ne soit ajouté, ce qui rend la mise à jour synchrone.
        let mut caps = Vec::with_capacity(nb_depart);
        let mut densites = Vec::with_capacity(nb_depart);
        for i in 0..nb_depart {
            let apex = self.apex[i];
            let (densite, evx, evy) = self.champ(apex.section, apex.x, apex.y);
            let cap = self.cap_suivant(apex.hx, apex.hy, evx, evy);
            caps.push(cap);
            densites.push(densite);
        }

        // Temps 2 : avancée. Chaque apex du début du pas laisse un segment. Une ramification éventuelle crée un
        // apex neuf, rangé pour avancer plus bas.
        let mut naissances = Vec::new();
        let mut nouvelles_ramifications = Vec::new();
        for i in 0..nb_depart {
            let mut apex = self.apex[i];
            let (hx, hy) = caps[i];
            apex.hx = hx;
            apex.hy = hy;

            let (x0, y0) = (apex.x, apex.y);
            let x1 = x0 + self.parametres.pas * hx;
            let y1 = y0 + self.parametres.pas * hy;
            apex.x = x1;
            apex.y = y1;
            self.apex[i] = apex;
            self.laisser_segment(apex.id, apex.section, x0, y0, x1, y1);

            // La ramification n'a lieu que là où la densité lue à la pointe est sous le seuil.
            if self.parametres.probabilite_ramification > 0.0
                && self.alea.suivant_f64() < self.parametres.probabilite_ramification
                && densites[i] < self.parametres.seuil_densite
            {
                let cote = self.alea.suivant() % 2;
                let angle = self.angle_tire();
                let (hx_fille, hy_fille) = tourner(apex.hx, apex.hy, angle, cote);

                // Une ramification coupe l'internode : la branche neuve et la continuation du parent sont deux
                // sections neuves, qui commencent au point de ramification. Le segment que le parent vient de
                // laisser part de ce point : on le rattache à sa nouvelle section.
                let section_fille = self.nb_segments_par_section.len() as u32;
                self.nb_segments_par_section.push(0);
                let section_parent = self.nb_segments_par_section.len() as u32;
                self.nb_segments_par_section.push(1);
                self.nb_segments_par_section[apex.section as usize] -= 1;
                let dernier = self.segments.len() - 1;
                self.segments_section[dernier] = section_parent;
                apex.section = section_parent;
                self.apex[i] = apex;

                let id = (nb_depart + naissances.len()) as u32;
                naissances.push(Apex {
                    id,
                    // La branche naît au point où le parent se trouvait au début du pas : c'est le point de
                    // ramification, et son propre premier segment y commence aussi.
                    x: x0,
                    y: y0,
                    hx: hx_fille,
                    hy: hy_fille,
                    section: section_fille,
                });
                nouvelles_ramifications.push(Ramification { temps: self.temps, apex: apex.id, densite: densites[i] });
            }
        }

        // Temps 3 : les apex nés pendant ce pas avancent eux aussi, sans se ramifier avant le pas suivant.
        for mut apex in naissances {
            let (x0, y0) = (apex.x, apex.y);
            let x1 = x0 + self.parametres.pas * apex.hx;
            let y1 = y0 + self.parametres.pas * apex.hy;
            apex.x = x1;
            apex.y = y1;
            self.laisser_segment(apex.id, apex.section, x0, y0, x1, y1);
            self.apex.push(apex);
        }

        // Temps 4 : on enregistre les séries et les ramifications du pas.
        self.ramifications.extend(nouvelles_ramifications);
        self.rayons.push(self.rayon);
        self.pointes_par_pas.push(self.apex.len() as u32);
    }

    /// Laisse un segment et tient à jour l'index spatial et le rayon de la colonie.
    fn laisser_segment(&mut self, apex: u32, section: u32, x0: f64, y0: f64, x1: f64, y1: f64) {
        let segment = croissance::Segment { apex, temps: self.temps, x0, y0, x1, y1 };
        self.segments.push(segment);
        self.segments_section.push(section);
        self.nb_segments_par_section[section as usize] += 1;

        let (dx, dy) = (x1 - self.centre.0, y1 - self.centre.1);
        let distance = (dx * dx + dy * dy).sqrt();
        if distance > self.rayon {
            self.rayon = distance;
        }

        let indice = self.segments.len() as u32 - 1;
        self.index.inserer(indice, &segment);
    }

    /// Tire l'angle de ramification, en degrés entiers dans `0..=angle_max`.
    fn angle_tire(&mut self) -> u32 {
        if self.parametres.angle_max == 0 {
            0
        } else {
            (self.alea.suivant() % (self.parametres.angle_max as u64 + 1)) as u32
        }
    }

    /// Nouveau cap après l'éq. 13 : mélange de l'ancien cap et de la direction d'évitement, puis normalisation.
    /// Un vecteur d'évitement nul (pointe isolée) laisse le cap inchangé, comme `persistance = 1`.
    fn cap_suivant(&self, hx: f64, hy: f64, evx: f64, evy: f64) -> (f64, f64) {
        let k = self.parametres.persistance;
        if k >= 1.0 || (evx == 0.0 && evy == 0.0) {
            return (hx, hy);
        }

        let norme_evitement = (evx * evx + evy * evy).sqrt();
        let (ex, ey) = (evx / norme_evitement, evy / norme_evitement);
        let nx = k * hx + (1.0 - k) * ex;
        let ny = k * hy + (1.0 - k) * ey;
        let norme = (nx * nx + ny * ny).sqrt();
        if norme > 0.0 { (nx / norme, ny / norme) } else { (hx, hy) }
    }

    /// Champ de densité et vecteur d'évitement (éq. 3, 4 et 11) en `(x, y)`, les segments de la section `propre`
    /// (l'internode en cours) étant exclus : une pointe ne se repousse pas elle-même. Rend `(densite, evx, evy)`.
    fn champ(&self, propre: u32, x: f64, y: f64) -> (f64, f64, f64) {
        let epsilon2 = self.parametres.epsilon * self.parametres.epsilon;
        let portee2 = self.parametres.portee * self.parametres.portee;
        let mut densite = 0.0;
        let mut evx = 0.0;
        let mut evy = 0.0;

        self.index.parcourir(x, y, |indice| {
            let segment = &self.segments[indice as usize];
            let section = self.segments_section[indice as usize];
            if section == propre {
                return;
            }
            let (distance2, qx, qy) = distance2_segment(x, y, segment);
            if distance2 > portee2 {
                return;
            }
            let n = self.nb_segments_par_section[section as usize];
            let poids = self.parametres.lc / n as f64;
            let contribution = poids / (distance2 + epsilon2);
            densite += contribution;

            // Direction d'évitement (éq. 11) : `(x - q)` va du segment vers la pointe. Le poids de sa section est
            // encore divisé par `(d² + ε²)` en plus du noyau de densité, si bien que les segments les plus proches
            // dominent largement.
            let gradient = contribution / ((distance2 + epsilon2) * (distance2 + epsilon2));
            evx += gradient * (x - qx);
            evy += gradient * (y - qy);
        });

        (densite, evx, evy)
    }

    /// Avance la colonie de `n` pas de temps : l'état obtenu est le même que par `n` appels à [`avancer`].
    pub fn avancer_n(&mut self, n: u32) {
        for _ in 0..n {
            self.avancer();
        }
    }

    /// Nombre de pas de temps déjà effectués.
    pub fn temps(&self) -> u32 {
        self.temps
    }

    /// Nombre d'apex vivants.
    pub fn pointes(&self) -> usize {
        self.apex.len()
    }

    /// Les apex vivants, dans l'ordre de leur identifiant.
    pub fn apex(&self) -> &[Apex] {
        &self.apex
    }

    /// Les segments déjà laissés, dans l'ordre où ils ont été laissés.
    pub fn segments(&self) -> &[croissance::Segment] {
        &self.segments
    }

    /// Rayon de la colonie après le dernier pas effectué : la plus grande distance entre la spore et une extrémité
    /// de segment (0 avant le premier pas).
    pub fn rayon(&self) -> f64 {
        self.rayon
    }

    /// Un rayon par pas : `rayons()[t - 1]` est le rayon après `t` pas. Le vecteur est vide au temps 0.
    pub fn rayons(&self) -> &[f64] {
        &self.rayons
    }

    /// Un élément par pas : `pointes_par_pas()[t - 1]` est le nombre de pointes après `t` pas. Le vecteur est
    /// vide au temps 0.
    pub fn pointes_par_pas(&self) -> &[u32] {
        &self.pointes_par_pas
    }

    /// Les ramifications survenues, dans l'ordre où elles ont eu lieu.
    pub fn ramifications(&self) -> &[Ramification] {
        &self.ramifications
    }

    /// Écriture canonique de la colonie, dans cet ordre et sans remplissage : `temps` (`u32`) ; le nombre de segments
    /// (`u64`) puis les segments dans l'ordre de leur dépôt, chacun par [`croissance::Segment::octets`] ; le nombre
    /// d'apex (`u64`) puis les apex dans l'ordre de leur identifiant, chacun par `id` et `section` (`u32`), puis `x`,
    /// `y`, `hx` et `hy` (`f64`) ; le nombre de ramifications (`u64`) puis les ramifications dans l'ordre où elles ont
    /// eu lieu, chacune par `temps` et `apex` (`u32`) puis `densite` (`f64`). Tous les entiers sont en petit-boutiste
    /// (`u32::to_le_bytes`, `u64::to_le_bytes`) et chaque `f64` est écrit par les 8 octets de son motif de bits
    /// (`f64::to_bits`), lui aussi en petit-boutiste. Deux colonies ont la même écriture si et seulement si leurs
    /// états sont identiques bit à bit.
    pub fn octets(&self) -> Vec<u8> {
        let mut octets = Vec::with_capacity(
            4 + 8
                + self.segments.len() * croissance::OCTETS_PAR_SEGMENT
                + 8
                + self.apex.len() * OCTETS_PAR_APEX
                + 8
                + self.ramifications.len() * 16,
        );
        octets.extend_from_slice(&self.temps.to_le_bytes());

        octets.extend_from_slice(&(self.segments.len() as u64).to_le_bytes());
        for segment in &self.segments {
            octets.extend_from_slice(&segment.octets());
        }

        octets.extend_from_slice(&(self.apex.len() as u64).to_le_bytes());
        for apex in &self.apex {
            octets.extend_from_slice(&apex.id.to_le_bytes());
            octets.extend_from_slice(&apex.section.to_le_bytes());
            octets.extend_from_slice(&apex.x.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.y.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.hx.to_bits().to_le_bytes());
            octets.extend_from_slice(&apex.hy.to_bits().to_le_bytes());
        }

        octets.extend_from_slice(&(self.ramifications.len() as u64).to_le_bytes());
        for ramification in &self.ramifications {
            octets.extend_from_slice(&ramification.temps.to_le_bytes());
            octets.extend_from_slice(&ramification.apex.to_le_bytes());
            octets.extend_from_slice(&ramification.densite.to_bits().to_le_bytes());
        }
        octets
    }

    /// Empreinte de la colonie : le SHA-256 de [`Colonie::octets`], en hexadécimal minuscule (64 caractères).
    pub fn empreinte(&self) -> String {
        sha256_hex(&self.octets())
    }
}

/// Tourne le cap `(hx, hy)` de `angle` degrés, dans le sens trigonométrique si `cote` vaut 0, dans l'autre sinon.
fn tourner(hx: f64, hy: f64, angle: u32, cote: u64) -> (f64, f64) {
    let c = croissance::COS[angle as usize];
    let s = croissance::SIN[angle as usize];
    if cote == 0 { (hx * c - hy * s, hx * s + hy * c) } else { (hx * c + hy * s, -hx * s + hy * c) }
}

/// Carré de la distance entre `(px, py)` et le segment `segment`, et coordonnées du point le plus proche.
fn distance2_segment(px: f64, py: f64, segment: &croissance::Segment) -> (f64, f64, f64) {
    let dx = segment.x1 - segment.x0;
    let dy = segment.y1 - segment.y0;
    let longueur2 = dx * dx + dy * dy;
    let t = if longueur2 > 0.0 {
        let t = ((px - segment.x0) * dx + (py - segment.y0) * dy) / longueur2;
        // Projection bornée au segment : `clamp` ne fait que des comparaisons, sans passer par la bibliothèque
        // mathématique.
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let qx = segment.x0 + t * dx;
    let qy = segment.y0 + t * dy;
    let ex = px - qx;
    let ey = py - qy;
    (ex * ex + ey * ey, qx, qy)
}

/// Index spatial : les segments sont rangés dans la case de leur milieu. Une requête ne parcourt que les cases
/// assez proches pour contenir un segment à moins de `portee`.
struct IndexSegments {
    centre: (f64, f64),
    cellule: f64,
    portee: f64,
    pas: f64,
    min_i: i64,
    min_j: i64,
    colonnes: usize,
    /// Une ligne par case en ordonnée, une colonne par case en abscisse ; chaque case est la liste des segments.
    lignes: Vec<Vec<Vec<u32>>>,
}

impl IndexSegments {
    fn nouveau(centre: (f64, f64), pas: f64, portee: f64) -> IndexSegments {
        // Une case d'environ huit unités : assez grande pour peu de cases à balayer, assez petite pour que
        // chaque case reste peu peuplée. Pour une portée plus fine que cela, la case suit la portée.
        let cellule = if portee < 8.0 { portee } else { 8.0 };
        IndexSegments { centre, cellule, portee, pas, min_i: 0, min_j: 0, colonnes: 1, lignes: vec![vec![Vec::new()]] }
    }

    /// Indice de case, plancher de la coordonnée relative divisée par la taille de case.
    fn case(&self, x: f64) -> i64 {
        plancher((x - self.centre.0) / self.cellule)
    }

    /// Insère un segment dans la case de son milieu.
    fn inserer(&mut self, indice: u32, segment: &croissance::Segment) {
        let mx = 0.5 * (segment.x0 + segment.x1);
        let my = 0.5 * (segment.y0 + segment.y1);
        let i = self.case(mx);
        let j = self.case(my);

        // Agrandit la grille à gauche et à droite si la case sort des bornes.
        if i < self.min_i {
            let ajout = (self.min_i - i) as usize;
            for ligne in &mut self.lignes {
                for _ in 0..ajout {
                    ligne.insert(0, Vec::new());
                }
            }
            self.min_i = i;
            self.colonnes += ajout;
        } else if i >= self.min_i + self.colonnes as i64 {
            let ajout = (i - (self.min_i + self.colonnes as i64) + 1) as usize;
            for ligne in &mut self.lignes {
                ligne.resize(self.colonnes + ajout, Vec::new());
            }
            self.colonnes += ajout;
        }

        // Puis en bas et en haut.
        if j < self.min_j {
            let ajout = (self.min_j - j) as usize;
            for _ in 0..ajout {
                self.lignes.insert(0, vec![Vec::new(); self.colonnes]);
            }
            self.min_j = j;
        } else if j >= self.min_j + self.lignes.len() as i64 {
            let ajout = (j - (self.min_j + self.lignes.len() as i64) + 1) as usize;
            for _ in 0..ajout {
                self.lignes.push(vec![Vec::new(); self.colonnes]);
            }
        }

        let colonne = (i - self.min_i) as usize;
        let ligne = (j - self.min_j) as usize;
        self.lignes[ligne][colonne].push(indice);
    }

    /// Parcourt les segments susceptibles d'être à moins de `portee` de `(x, y)`.
    fn parcourir<F: FnMut(u32)>(&self, x: f64, y: f64, mut f: F) {
        // Chaque segment est rangé à sa case de milieu. Son point le plus proche d'une pointe est au plus à
        // `pas / 2` de ce milieu ; on élargit donc le rayon de cases visitées d'autant.
        let rayon = plafond((self.portee + 0.5 * self.pas) / self.cellule);
        let ci = self.case(x);
        let cj = self.case(y);
        for j in (cj - rayon)..=(cj + rayon) {
            let ligne = j - self.min_j;
            if ligne < 0 || ligne >= self.lignes.len() as i64 {
                continue;
            }
            for i in (ci - rayon)..=(ci + rayon) {
                let colonne = i - self.min_i;
                if colonne < 0 || colonne >= self.colonnes as i64 {
                    continue;
                }
                for &indice in &self.lignes[ligne as usize][colonne as usize] {
                    f(indice);
                }
            }
        }
    }
}

/// Plancher d'un `f64`, sans passer par la bibliothèque mathématique du système.
fn plancher(x: f64) -> i64 {
    let troncature = x as i64;
    if x < troncature as f64 { troncature - 1 } else { troncature }
}

/// Plafond d'un `f64`, sans passer par la bibliothèque mathématique du système.
fn plafond(x: f64) -> i64 {
    let troncature = x as i64;
    if x > troncature as f64 { troncature + 1 } else { troncature }
}

/// Résultat d'une régression linéaire par moindres carrés.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ajustement {
    /// Pente de la droite ajustée.
    pub pente: f64,
    /// Ordonnée à l'origine de la droite ajustée.
    pub ordonnee: f64,
    /// Coefficient de détermination : `1 - somme des carrés résiduels / somme des carrés totaux`.
    pub r2: f64,
}

/// Régression linéaire par moindres carrés de `y` sur `x` : renvoie la pente, l'ordonnée à l'origine et le
/// coefficient de détermination `r2 = 1 - SCR / SCT`, où `SCR` est la somme des carrés résiduels et `SCT` la
/// somme des carrés totaux par rapport à la moyenne de `y`.
///
/// # Panics
///
/// Panique si `x` et `y` n'ont pas la même longueur, si la longueur est inférieure à 2, ou si les `x` sont
/// tous égaux (la pente serait indéfinie).
pub fn ajuster(x: &[f64], y: &[f64]) -> Ajustement {
    assert_eq!(x.len(), y.len(), "x et y n'ont pas la même longueur");
    assert!(x.len() >= 2, "il faut au moins deux points");

    let n = x.len() as f64;
    let mut somme_x = 0.0;
    let mut somme_y = 0.0;
    let mut somme_xx = 0.0;
    let mut somme_xy = 0.0;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        somme_x += xi;
        somme_y += yi;
        somme_xx += xi * xi;
        somme_xy += xi * yi;
    }

    let denominateur = n * somme_xx - somme_x * somme_x;
    assert!(denominateur != 0.0, "les x sont tous égaux : la pente est indéfinie");
    let pente = (n * somme_xy - somme_x * somme_y) / denominateur;
    let ordonnee = (somme_y - pente * somme_x) / n;

    let moyenne_y = somme_y / n;
    let mut scr = 0.0;
    let mut sct = 0.0;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        let residu = yi - (pente * xi + ordonnee);
        scr += residu * residu;
        let ecart = yi - moyenne_y;
        sct += ecart * ecart;
    }

    Ajustement { pente, ordonnee, r2: 1.0 - scr / sct }
}

/// Régression du rayon sur le temps, pour les temps `t` dans `]t_debut, t_fin]`. Le rayon au temps `t` est
/// `rayons[t - 1]` ; le temps compte à partir de 1.
///
/// # Panics
///
/// Panique si la fenêtre contient moins de deux points, ou si `t_debut` ou `t_fin` sort des bornes de
/// `rayons`.
pub fn ajuster_rayon(rayons: &[f64], t_debut: u32, t_fin: u32) -> Ajustement {
    assert!((t_fin as usize) <= rayons.len(), "t_fin = {t_fin} hors des bornes de rayons");
    assert!(t_debut < t_fin, "fenêtre vide : ]{t_debut}, {t_fin}]");
    assert!(t_fin - t_debut >= 2, "il faut au moins deux points dans ]{t_debut}, {t_fin}]");

    let mut temps = Vec::with_capacity((t_fin - t_debut) as usize);
    let mut valeurs = Vec::with_capacity((t_fin - t_debut) as usize);
    for t in (t_debut + 1)..=t_fin {
        temps.push(t as f64);
        valeurs.push(rayons[(t - 1) as usize]);
    }
    ajuster(&temps, &valeurs)
}

/// Pentes de régressions du rayon sur le temps, sur des fenêtres de `largeur` pas décalées de `pas`, dans
/// `]t_debut, t_fin]`. La première fenêtre est `]t_debut, t_debut + largeur]`, la suivante
/// `]t_debut + pas, t_debut + pas + largeur]`, et ainsi de suite tant que la fin tient dans `t_fin`. Par
/// exemple, `vitesses_glissantes(r, 200, 400, 50, 10)` rend 16 pentes, celles des fenêtres
/// `]200, 250]`, `]210, 260]`, …, `]350, 400]`.
///
/// # Panics
///
/// Panique si `largeur` ou `pas` vaut 0, ou si une fenêtre sort des bornes de `rayons`.
pub fn vitesses_glissantes(rayons: &[f64], t_debut: u32, t_fin: u32, largeur: u32, pas: u32) -> Vec<f64> {
    assert!(largeur > 0, "largeur nulle");
    assert!(pas > 0, "pas nul");

    let mut vitesses = Vec::new();
    let mut debut = t_debut;
    while let Some(fin) = debut.checked_add(largeur) {
        if fin > t_fin {
            break;
        }
        vitesses.push(ajuster_rayon(rayons, debut, fin).pente);
        match debut.checked_add(pas) {
            Some(suivant) => debut = suivant,
            None => break,
        }
    }
    vitesses
}

/// Exposant `a` de la loi de puissance `serie(t) ~ t^a`, estimé par moindres carrés de `ln serie` sur `ln t`,
/// pour les temps `t` dans `]t_debut, t_fin]`. Les logarithmes sont ceux de [`crate::mesures::logarithme`],
/// de sorte que le résultat est identique sur toutes les machines. La série au temps `t` est `serie[t - 1]`.
///
/// # Panics
///
/// Panique si la fenêtre contient moins de deux points ou si elle sort des bornes de `serie`.
pub fn exposant(serie: &[f64], t_debut: u32, t_fin: u32) -> f64 {
    assert!((t_fin as usize) <= serie.len(), "t_fin = {t_fin} hors des bornes de serie");
    assert!(t_debut < t_fin, "fenêtre vide : ]{t_debut}, {t_fin}]");
    assert!(t_fin - t_debut >= 2, "il faut au moins deux points dans ]{t_debut}, {t_fin}]");

    let mut temps = Vec::with_capacity((t_fin - t_debut) as usize);
    let mut valeurs = Vec::with_capacity((t_fin - t_debut) as usize);
    for t in (t_debut + 1)..=t_fin {
        temps.push(crate::mesures::logarithme(t as f64));
        valeurs.push(crate::mesures::logarithme(serie[(t - 1) as usize]));
    }
    ajuster(&temps, &valeurs).pente
}
