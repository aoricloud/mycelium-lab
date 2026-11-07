//! Anastomose : fusion de deux hyphes qui se rencontrent.
//!
//! # Le modèle
//!
//! Un apex qui avance laisse des segments. Quand il approche un segment venu d'un autre apex, assez ancien pour
//! que la fusion ait un sens, il peut s'y anastomoser : l'index rend le numéro du segment le plus proche, et à
//! distance égale le plus petit numéro.
//!
//! [`Reglage`] porte les deux seuils : `rayon`, la distance maximale entre l'apex et le segment, et `delai`, le
//! nombre de pas de temps minimal entre le dépôt d'un segment et sa prise en compte. Un segment du même apex que
//! l'apex qui cherche n'est jamais candidat : un apex ne fusionne pas avec lui-même.
//!
//! # Les garanties
//!
//! Le module ne dépend de rien hors de la bibliothèque standard et n'utilise, parmi les fonctions de `f64`, que
//! `sqrt`, `floor` et `clamp` ; ce sont des opérations exactes (racine correctement arrondie, arrondi vers le bas,
//! bornes) et déterministes, et tout est déterministe d'une machine à l'autre. L'[`Index`] possède ses segments et
//! les rend dans l'ordre de leur ajout ; aucun parcours de `HashMap` ni de `HashSet` n'intervient.

use crate::croissance::Segment;

/// Les réglages d'une recherche d'anastomose.
///
/// Contrat : `rayon` est une distance maximale, positive ou nulle, exprimée dans l'unité du plan ; la distance
/// égale au rayon compte comme dedans, et un rayon négatif ne trouve rien. `delai` est un nombre de pas de temps :
/// un segment déposé au temps `t` n'est candidat qu'à partir du temps courant `t + delai`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reglage {
    /// Distance maximale entre l'apex qui cherche et le segment candidat.
    pub rayon: f64,
    /// Nombre minimal de pas de temps entre le dépôt d'un segment et sa prise en compte.
    pub delai: u32,
}

/// Distance du point `(x, y)` au segment `segment`, c'est-à-dire à son point le plus proche, extrémités comprises :
/// jamais la distance à la droite qui prolonge le segment. Un segment réduit à un point rend la distance à ce
/// point. Une distance nulle signifie que le point est sur le segment.
///
/// Le calcul n'utilise que `+`, `-`, `*`, `/`, `clamp` et `sqrt`.
pub fn distance_au_segment(x: f64, y: f64, segment: &Segment) -> f64 {
    let dx = segment.x1 - segment.x0;
    let dy = segment.y1 - segment.y0;
    let carre = dx * dx + dy * dy;

    // Un segment réduit à un point n'a pas de direction : la distance est celle au point.
    let (proche_x, proche_y) = if carre == 0.0 {
        (segment.x0, segment.y0)
    } else {
        // Projection orthogonale, bornée aux deux extrémités.
        let t = (((x - segment.x0) * dx + (y - segment.y0) * dy) / carre).clamp(0.0, 1.0);
        (segment.x0 + t * dx, segment.y0 + t * dy)
    };

    let ex = x - proche_x;
    let ey = y - proche_y;
    (ex * ex + ey * ey).sqrt()
}

/// Un index des segments contenus dans une boîte rectangulaire fermée.
///
/// Contrat : la boîte est `[0, largeur] × [0, hauteur]`, bornes comprises. L'index possède ses segments et les
/// numérote dans l'ordre de leur ajout, à partir de 0. Toutes les extrémités ajoutées appartiennent à la boîte.
/// Les recherches sont déterministes et ne dépendent que des segments, de la requête et des [`Reglage`]s.
pub struct Index {
    largeur: usize,
    hauteur: usize,
    segments: Vec<Segment>,
    /// Grille de cases de côté 1 : `cellules[ligne * (largeur + 1) + colonne]` contient les numéros des segments
    /// dont le milieu tombe dans la case. Une `Vec` par case, jamais de `HashMap`.
    cellules: Vec<Vec<usize>>,
    /// La plus grande longueur de segment ajoutée, pour élargir la recherche d'autant de la moitié.
    longueur_max: f64,
}

impl Index {
    /// Crée un index vide pour la boîte fermée `[0, largeur] × [0, hauteur]`.
    ///
    /// Les coordonnées admises vont de `0.0` à `largeur as f64` en abscisse et de `0.0` à `hauteur as f64` en
    /// ordonnée, bornes comprises.
    pub fn new(largeur: usize, hauteur: usize) -> Index {
        let colonnes = largeur + 1;
        let lignes = hauteur + 1;
        Index {
            largeur,
            hauteur,
            segments: Vec::new(),
            cellules: vec![Vec::new(); colonnes * lignes],
            longueur_max: 0.0,
        }
    }

    /// Ajoute `segment` à l'index et rend son numéro : `0` pour le premier ajout, puis `1`, `2`, ...
    ///
    /// # Panics
    ///
    /// Si l'une des extrémités n'appartient pas à la boîte fermée : `x0` ou `x1` hors de `[0, largeur]`, `y0` ou
    /// `y1` hors de `[0, hauteur]`. Les bornes elles-mêmes sont admises ; une coordonnée non finie est hors de la
    /// boîte.
    pub fn ajouter(&mut self, segment: Segment) -> usize {
        let largeur = self.largeur as f64;
        let hauteur = self.hauteur as f64;
        let dans_la_boite = |x: f64, y: f64| (0.0..=largeur).contains(&x) && (0.0..=hauteur).contains(&y);
        assert!(
            dans_la_boite(segment.x0, segment.y0) && dans_la_boite(segment.x1, segment.y1),
            "une extrémité du segment sort de la boîte fermée"
        );

        // Le segment est rangé dans la case de son milieu, où qu'il aille ensuite.
        let milieu_x = (segment.x0 + segment.x1) / 2.0;
        let milieu_y = (segment.y0 + segment.y1) / 2.0;
        let colonne = milieu_x.floor() as usize;
        let ligne = milieu_y.floor() as usize;
        let colonnes = self.largeur + 1;

        let numero = self.segments.len();
        self.cellules[ligne * colonnes + colonne].push(numero);

        let dx = segment.x1 - segment.x0;
        let dy = segment.y1 - segment.y0;
        let longueur = (dx * dx + dy * dy).sqrt();
        if longueur > self.longueur_max {
            self.longueur_max = longueur;
        }

        self.segments.push(segment);
        numero
    }

    /// Les segments ajoutés, dans l'ordre de leur numéro : le segment `i` est celui rendu par le `i`-ième
    /// [`Index::ajouter`].
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Cherche pour l'apex `apex`, au temps courant `temps`, le segment candidat à une anastomose au point
    /// `(x, y)`.
    ///
    /// Un segment `s` est candidat si les trois conditions tiennent :
    ///
    /// - `s.apex != apex` : un apex ne fusionne pas avec ses propres segments ;
    /// - `temps >= s.temps + reglage.delai` : le segment est assez ancien (la somme est faite sans débordement) ;
    /// - `distance_au_segment(x, y, s) <= reglage.rayon` : le segment est assez proche, l'égalité au rayon
    ///   comptant comme dedans.
    ///
    /// Rend le numéro du candidat le plus proche, et à distance égale le plus petit numéro (donc le plus ancien
    /// dans l'ordre des ajouts). Rend `None` si aucun segment n'est candidat.
    pub fn chercher(&self, x: f64, y: f64, apex: u32, temps: u32, reglage: &Reglage) -> Option<usize> {
        if reglage.rayon < 0.0 {
            return None;
        }

        // Un segment candidat a son milieu à `rayon + longueur / 2` au plus du point ; on visite donc toutes les
        // cases qu'une telle boule peut toucher, jamais seulement celle du point.
        let portee = reglage.rayon + self.longueur_max / 2.0;
        let colonnes = self.largeur + 1;
        let lignes = self.hauteur + 1;
        let derniere_colonne = (colonnes - 1) as f64;
        let derniere_ligne = (lignes - 1) as f64;
        let colonne_min = ((x - portee).floor()).max(0.0).min(derniere_colonne) as usize;
        let colonne_max = ((x + portee).floor()).max(0.0).min(derniere_colonne) as usize;
        let ligne_min = ((y - portee).floor()).max(0.0).min(derniere_ligne) as usize;
        let ligne_max = ((y + portee).floor()).max(0.0).min(derniere_ligne) as usize;

        let mut meilleur: Option<(usize, f64)> = None;
        for ligne in ligne_min..=ligne_max {
            for colonne in colonne_min..=colonne_max {
                for &numero in &self.cellules[ligne * colonnes + colonne] {
                    let segment = &self.segments[numero];
                    if segment.apex == apex {
                        continue;
                    }
                    if u64::from(temps) < u64::from(segment.temps) + u64::from(reglage.delai) {
                        continue;
                    }
                    let distance = distance_au_segment(x, y, segment);
                    if distance > reglage.rayon {
                        continue;
                    }
                    let meilleur_que = match meilleur {
                        None => true,
                        Some((numero_vu, distance_vue)) => {
                            distance < distance_vue || (distance == distance_vue && numero < numero_vu)
                        }
                    };
                    if meilleur_que {
                        meilleur = Some((numero, distance));
                    }
                }
            }
        }

        meilleur.map(|(numero, _)| numero)
    }
}
