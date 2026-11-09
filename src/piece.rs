//! Pièce : mesurer ce que la colonie remplit (étape M0).
//!
//! La colonie laisse des [`Segment`]s. Ce module en tire, **sans rien changer à la simulation**, cinq mesures sur la
//! « pièce » qu'elle moule dans la boîte : une carte de densité d'hyphe, les cases occupées, le bilan de nourriture, le
//! plus grand morceau connecté et la traversée d'un bord à l'autre. Il ne dit rien de la raideur ni de la résistance
//! d'un vrai matériau : il compte des cases et des longueurs.
//!
//! Une case est **occupée au seuil `s`** si sa densité est strictement supérieure à `s` ([`Carte::occupees`]). Au seuil
//! nul ([`mesurer`]), une colonie née d'une seule spore est d'un seul tenant ; un seuil plus haut ne garde que les
//! cases denses, et la pièce se découpe alors en morceaux ([`mesurer_au_seuil`]).
//!
//! # La règle du plancher
//!
//! Comme partout dans le dépôt, le point `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)` : la case `(i, j)` est le carré
//! semi-ouvert `[i, i+1) × [j, j+1)`. L'axe `y` croît avec l'indice de ligne : la ligne 0 est le **haut** de la boîte
//! (comme dans l'image `densite.png`), la dernière ligne en est le **bas**.
//!
//! # Les cartes et les grilles de cases
//!
//! Une grille de `largeur × hauteur` valeurs se range ligne par ligne, `y` lent et `x` rapide : la case `(x, y)` est à
//! l'indice `y * largeur + x`. C'est l'ordre de [`Carte::valeurs`] et celui des tranches `occupees: &[bool]`.
//!
//! # Le déterminisme
//!
//! Le calcul n'emploie, parmi les fonctions de `f64`, que `+ - * /`, `sqrt`, `floor`, `max`, `is_finite` et les
//! conversions `as`, et n'utilise ni `HashMap` ni `HashSet` : le même résultat, bit à bit, sur toutes les machines.

use std::collections::VecDeque;

use crate::croissance::Segment;
use crate::simulation::Simulation;

/// Une carte de valeurs sur la boîte : `largeur × hauteur` nombres, rangés ligne par ligne (`y` lent, `x` rapide).
#[derive(Clone, Debug, PartialEq)]
pub struct Carte {
    largeur: usize,
    hauteur: usize,
    valeurs: Vec<f64>,
}

impl Carte {
    /// Une carte dont toutes les cases valent `0.0`.
    ///
    /// Contrat : panique si `largeur` ou `hauteur` est nulle.
    pub fn nouvelle(largeur: usize, hauteur: usize) -> Carte {
        assert!(largeur > 0 && hauteur > 0, "une carte a au moins une case : {largeur} × {hauteur}");
        Carte { largeur, hauteur, valeurs: vec![0.0; largeur * hauteur] }
    }

    /// Largeur de la carte, en cases.
    pub fn largeur(&self) -> usize {
        self.largeur
    }

    /// Hauteur de la carte, en cases.
    pub fn hauteur(&self) -> usize {
        self.hauteur
    }

    /// La valeur de la case `(x, y)`.
    ///
    /// Contrat : panique si `x >= largeur()` ou `y >= hauteur()`. Le contrôle porte sur chaque coordonnée : un `x` trop
    /// grand ne glisse pas sur la ligne suivante.
    pub fn valeur(&self, x: usize, y: usize) -> f64 {
        assert!(x < self.largeur, "x = {x} hors de 0..{}", self.largeur);
        assert!(y < self.hauteur, "y = {y} hors de 0..{}", self.hauteur);
        self.valeurs[y * self.largeur + x]
    }

    /// Toutes les valeurs, ligne par ligne (`y` lent, `x` rapide).
    pub fn valeurs(&self) -> &[f64] {
        &self.valeurs
    }

    /// La somme des valeurs, additionnées une à une **dans l'ordre de [`Carte::valeurs`]** (l'ordre fixe rend la somme
    /// reproductible au bit près).
    pub fn somme(&self) -> f64 {
        let mut somme = 0.0;
        for &valeur in &self.valeurs {
            somme += valeur;
        }
        somme
    }

    /// Les cases « occupées au seuil `seuil` » : celles dont la valeur est **strictement** supérieure à `seuil`, une par
    /// case, dans l'ordre de [`Carte::valeurs`]. Au seuil `0.0`, ce sont les cases où passe de l'hyphe, si peu que ce soit.
    ///
    /// Contrat : `seuil` est fini et positif ou nul (`-0.0` compte comme `0.0`) ; sinon la fonction panique.
    pub fn occupees(&self, seuil: f64) -> Vec<bool> {
        verifier_le_seuil(seuil);
        self.valeurs.iter().map(|&valeur| valeur > seuil).collect()
    }
}

/// Panique si `seuil` n'est pas un nombre fini positif ou nul (NaN compris).
fn verifier_le_seuil(seuil: f64) {
    assert!(seuil.is_finite() && seuil >= 0.0, "le seuil de densité est un nombre fini positif ou nul : {seuil}");
}

/// La carte de densité d'hyphe : pour chaque case, la somme des longueurs des morceaux de segment qui y passent.
///
/// # La coupe
///
/// Chaque segment est coupé à chaque ligne de grille qu'il franchit (`x` entier, `y` entier). Avec le paramètre
/// `t ∈ [0, 1]` (`P(t) = P0 + t (P1 − P0)`), les instants de coupe sont ceux où `x` (ou `y`) est un entier `n`
/// **strictement** entre le début et la fin du segment, `t = (n − c0) / (c1 − c0)` ; les deux listes sont déjà triées, on
/// les fusionne (les doublons restent : un passage exact par un coin donne deux instants égaux, donc un morceau de
/// longueur nulle, qui ne compte pour rien), et on met `0` devant et `1` derrière.
///
/// Chaque morceau, entre deux instants consécutifs `ta < tb`, appartient à la case qui contient **son milieu**
/// `m = (ta + tb) / 2`, le point `(x0 + m dx, y0 + m dy)` (règle du plancher), et lui apporte `(tb − ta) × L`, avec
/// `L = sqrt(dx² + dy²)`. Prendre le milieu plutôt que le début ou la fin règle seul les cas difficiles : un morceau ne
/// touche une ligne de grille que par ses extrémités, son milieu est donc toujours à l'intérieur d'une case, quel que
/// soit le sens du segment.
///
/// - Un segment de longueur nulle ne contribue à rien (ni un segment si court que `dx² + dy²` s'arrondit à 0) ; un
///   segment dans une seule case lui apporte exactement `L`.
/// - Un segment qui longe une ligne de grille (`dx = 0` avec `x` entier, ou `dy = 0` avec `y` entier) n'a aucune coupe sur
///   cet axe, son milieu est sur la ligne, et le plancher donne la case dont c'est le bord gauche (ou haut) : `x = 3,0`,
///   colonne 3 ; `y = 2,0`, ligne 2.
///
/// # Contrat
///
/// Toutes les coordonnées sont finies et dans `[0, largeur] × [0, hauteur]`, la borne haute comprise (un segment peut
/// finir sur le bord de la boîte) ; sinon la fonction panique. Panique aussi si `largeur` ou `hauteur` est nulle.
///
/// # La case d'un milieu
///
/// Un morceau qui avance sur un axe a son milieu strictement entre ses deux extrémités, donc, en arithmétique exacte,
/// strictement avant la borne haute : sa case est dans la boîte. Deux cas font pourtant tomber le plancher sur
/// `largeur` (ou `hauteur`), une case qui n'existe pas : un segment qui **longe** exactement le bord haut (`x = largeur`
/// sur toute sa longueur, permis par le contrat), et, en flottants, un morceau d'un ou deux ulps contre le bord dont le
/// milieu s'arrondit sur la borne. Dans les deux cas le morceau est au bord de la dernière case : on la lui donne
/// (`min(largeur − 1)`), sinon la somme des cases ne serait plus la somme des longueurs. Les coordonnées ne sont jamais
/// négatives, le plancher ne sort donc pas de la boîte par le bas.
///
/// # L'ordre des additions
///
/// Chaque apport `(tb − ta) × L` s'ajoute directement à sa case, dans l'ordre des segments, puis, pour un segment, dans
/// l'ordre des `t` croissants : la carte est la même au bit près sur toutes les machines.
///
/// # La somme
///
/// La somme des cases est la somme des longueurs des segments, à l'arrondi près. Aucune case n'est négative.
pub fn carte_de_densite(largeur: usize, hauteur: usize, segments: &[Segment]) -> Carte {
    let mut carte = Carte::nouvelle(largeur, hauteur);
    let (borne_x, borne_y) = (largeur as f64, hauteur as f64);
    for segment in segments {
        // `contains` rend faux pour NaN : un nombre non fini ou hors de la boîte panique.
        let dans_la_boite = (0.0..=borne_x).contains(&segment.x0)
            && (0.0..=borne_x).contains(&segment.x1)
            && (0.0..=borne_y).contains(&segment.y0)
            && (0.0..=borne_y).contains(&segment.y1);
        assert!(
            dans_la_boite,
            "segment ({}, {}) → ({}, {}) hors de [0, {largeur}] × [0, {hauteur}]",
            segment.x0, segment.y0, segment.x1, segment.y1
        );
        repartir_le_segment(&mut carte, segment);
    }
    carte
}

/// Ajoute à `carte` les morceaux d'un segment (dans le contrat de [`carte_de_densite`]), un par case traversée.
fn repartir_le_segment(carte: &mut Carte, segment: &Segment) {
    let (dx, dy) = (segment.x1 - segment.x0, segment.y1 - segment.y0);
    let longueur = (dx * dx + dy * dy).sqrt();
    if longueur == 0.0 {
        return;
    }

    // Les instants de coupe : 0, puis les coupes en `x` et en `y` fusionnées, puis 1.
    let coupes_x = instants_de_coupe(segment.x0, segment.x1);
    let coupes_y = instants_de_coupe(segment.y0, segment.y1);
    let mut instants = Vec::with_capacity(coupes_x.len() + coupes_y.len() + 2);
    instants.push(0.0);
    fusionner(&coupes_x, &coupes_y, &mut instants);
    instants.push(1.0);

    let (largeur, hauteur) = (carte.largeur, carte.hauteur);
    for paire in instants.windows(2) {
        let (ta, tb) = (paire[0], paire[1]);
        if tb <= ta {
            // Un passage exact par un coin (ou un arrondi qui colle deux instants) : longueur nulle.
            continue;
        }
        let milieu = (ta + tb) / 2.0;
        let x = segment.x0 + milieu * dx;
        let y = segment.y0 + milieu * dy;
        // Le plancher ; `min` ramène sur la dernière case un milieu posé sur le bord haut (voir « La case d'un milieu »).
        let colonne = (x.floor() as usize).min(largeur - 1);
        let ligne = (y.floor() as usize).min(hauteur - 1);
        carte.valeurs[ligne * largeur + colonne] += (tb - ta) * longueur;
    }
}

/// Les instants `t ∈ (0, 1]` (croissants) où une coordonnée qui va de `c0` à `c1` franchit une valeur entière
/// **strictement** comprise entre `c0` et `c1` : `t = (n − c0) / (c1 − c0)`. Vide si la coordonnée ne change pas.
///
/// Le premier entier visé est le premier strictement après `c0` dans le sens du déplacement (si `c0` est lui-même un
/// entier, il est exclu : le segment y est déjà). Les entiers restent exacts en `f64` : la boîte est petite devant 2⁵³.
fn instants_de_coupe(c0: f64, c1: f64) -> Vec<f64> {
    let d = c1 - c0;
    let mut instants = Vec::new();
    if d > 0.0 {
        let mut n = c0.floor() + 1.0;
        while n < c1 {
            instants.push((n - c0) / d);
            n += 1.0;
        }
    } else if d < 0.0 {
        // Le plus grand entier strictement plus petit que `c0`.
        let mut n = if c0.floor() == c0 { c0 - 1.0 } else { c0.floor() };
        while n > c1 {
            instants.push((n - c0) / d);
            n -= 1.0;
        }
    }
    instants
}

/// Fusionne deux listes croissantes en les ajoutant à `sortie`, doublons gardés.
fn fusionner(a: &[f64], b: &[f64], sortie: &mut Vec<f64>) {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            sortie.push(a[i]);
            i += 1;
        } else {
            sortie.push(b[j]);
            j += 1;
        }
    }
    sortie.extend_from_slice(&a[i..]);
    sortie.extend_from_slice(&b[j..]);
}

/// Un bord de la boîte : `Gauche` est la colonne 0, `Droite` la dernière colonne, `Haut` la ligne 0 et `Bas` la dernière
/// ligne (l'axe `y` descend avec l'indice de ligne, comme dans l'image).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bord {
    /// La colonne `x = 0`.
    Gauche,
    /// La colonne `x = largeur − 1`.
    Droite,
    /// La ligne `y = 0`.
    Haut,
    /// La ligne `y = hauteur − 1`.
    Bas,
}

/// Les huit directions : une case et sa voisine se touchent par un côté ou par un coin.
const VOISINES: [(isize, isize); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Les indices des cases voisines (huit directions) de la case d'indice `i`, celles qui sont dans la boîte.
fn voisines(largeur: usize, hauteur: usize, i: usize) -> impl Iterator<Item = usize> {
    let (x, y) = ((i % largeur) as isize, (i / largeur) as isize);
    VOISINES.into_iter().filter_map(move |(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        let dedans = nx >= 0 && ny >= 0 && (nx as usize) < largeur && (ny as usize) < hauteur;
        dedans.then(|| ny as usize * largeur + nx as usize)
    })
}

/// Panique si `occupees` n'a pas exactement `largeur × hauteur` cases.
fn verifier_la_grille(largeur: usize, hauteur: usize, occupees: &[bool]) {
    assert_eq!(
        occupees.len(),
        largeur * hauteur,
        "la tranche a {} cases, la boîte {largeur} × {hauteur} en demande {}",
        occupees.len(),
        largeur * hauteur
    );
}

/// Les tailles des morceaux connectés de cases occupées, de la plus grande à la plus petite (vide s'il n'y a aucune
/// case occupée).
///
/// **Connexité à huit voisins** : deux cases occupées sont voisines si elles se touchent par un côté ou par un coin. Un
/// hyphe qui passe exactement par un coin de case (tous les 45° depuis un centre de case) ne laisse que des cases en
/// diagonale, et elles font un seul morceau.
///
/// `occupees` contient `largeur × hauteur` booléens, ligne par ligne (`y` lent, `x` rapide). Panique si sa longueur n'est
/// pas `largeur × hauteur`. Une boîte sans case (largeur ou hauteur nulle, tranche vide) n'a aucun morceau.
pub fn morceaux(largeur: usize, hauteur: usize, occupees: &[bool]) -> Vec<usize> {
    verifier_la_grille(largeur, hauteur, occupees);
    let mut vues = vec![false; occupees.len()];
    let mut tailles = Vec::new();
    let mut pile = Vec::new();
    for depart in 0..occupees.len() {
        if !occupees[depart] || vues[depart] {
            continue;
        }
        // Un parcours en profondeur, sans récursion : la pile est explicite.
        vues[depart] = true;
        pile.push(depart);
        let mut taille = 0;
        while let Some(i) = pile.pop() {
            taille += 1;
            for voisine in voisines(largeur, hauteur, i) {
                if occupees[voisine] && !vues[voisine] {
                    vues[voisine] = true;
                    pile.push(voisine);
                }
            }
        }
        tailles.push(taille);
    }
    tailles.sort_unstable_by(|a, b| b.cmp(a));
    tailles
}

/// La taille du plus grand morceau connecté (huit voisins), `0` s'il n'y a aucune case occupée. Voir [`morceaux`].
pub fn plus_grand_morceau(largeur: usize, hauteur: usize, occupees: &[bool]) -> usize {
    morceaux(largeur, hauteur, occupees).first().copied().unwrap_or(0)
}

/// Le **nombre de cases** du plus court chemin de cases occupées (huit voisins) qui part d'une case occupée du bord `de`
/// (toute la colonne ou la ligne extrême) et finit sur une case occupée du bord `vers`. Une seule case occupée qui est
/// sur les deux bords (boîte d'une case ou d'une seule ligne, case de coin pour deux bords voisins) donne `Some(1)`.
/// `None` s'il n'y a pas de chemin.
///
/// Parcours en largeur depuis toutes les cases occupées du bord `de` à la fois (distance 1) : la première case du bord
/// `vers` sortie de la file est à la plus courte distance. Sans `HashMap`.
///
/// `occupees` contient `largeur × hauteur` booléens, ligne par ligne. Panique si `de == vers` ou si la longueur de
/// `occupees` n'est pas `largeur × hauteur`. Une boîte sans case (largeur ou hauteur nulle, tranche vide) donne `None`.
pub fn traversee(largeur: usize, hauteur: usize, occupees: &[bool], de: Bord, vers: Bord) -> Option<usize> {
    assert!(de != vers, "une traversée va d'un bord à un autre : {de:?} deux fois");
    verifier_la_grille(largeur, hauteur, occupees);
    let sur_le_bord = |bord: Bord, i: usize| match bord {
        Bord::Gauche => i.is_multiple_of(largeur),
        Bord::Droite => i % largeur == largeur - 1,
        Bord::Haut => i / largeur == 0,
        Bord::Bas => i / largeur == hauteur - 1,
    };

    // `distance[i]` : le nombre de cases du plus court chemin occupé jusqu'à la case `i` ; 0 tant qu'elle n'est pas
    // atteinte (un chemin a au moins une case).
    let mut distance = vec![0usize; occupees.len()];
    let mut file = VecDeque::new();
    for i in 0..occupees.len() {
        if occupees[i] && sur_le_bord(de, i) {
            distance[i] = 1;
            file.push_back(i);
        }
    }
    while let Some(i) = file.pop_front() {
        if sur_le_bord(vers, i) {
            return Some(distance[i]);
        }
        for voisine in voisines(largeur, hauteur, i) {
            if occupees[voisine] && distance[voisine] == 0 {
                distance[voisine] = distance[i] + 1;
                file.push_back(voisine);
            }
        }
    }
    None
}

/// Les mesures de la pièce à un instant de la simulation, à un seuil de densité. Voir [`mesurer_au_seuil`].
///
/// Les champs « de cases » (`cases_occupees`, `remplissage`, `plus_grand_morceau`, `nombre_de_morceaux` et les deux
/// traversées) ne regardent que les cases de densité **strictement supérieure au seuil** ([`Carte::occupees`]) ; les
/// autres (`cases_libres`, les densités, la biomasse et le bilan de nourriture) ne dépendent pas du seuil.
#[derive(Clone, Debug, PartialEq)]
pub struct MesuresPiece {
    /// Les cases non bloquées.
    pub cases_libres: usize,
    /// Les cases de densité strictement supérieure au seuil.
    pub cases_occupees: usize,
    /// `cases_occupees / cases_libres` (`0.0` s'il n'y a aucune case libre).
    pub remplissage: f64,
    /// La somme de la carte divisée par `cases_libres`.
    pub densite_moyenne: f64,
    /// La plus grande densité de la carte.
    pub densite_maximale: f64,
    /// La biomasse de la simulation, [`Simulation::biomasse`].
    pub biomasse: f64,
    /// La nourriture au temps 0 : le substrat total du premier relevé.
    pub nourriture_initiale: f64,
    /// La nourriture qui reste : la masse totale du substrat.
    pub nourriture_restante: f64,
    /// La nourriture que la colonie a consommée : `cout × biomasse`.
    pub nourriture_consommee: f64,
    /// `|restante + consommée − initiale|` : de l'ordre de l'arrondi si le bilan tient.
    pub ecart_du_bilan: f64,
    /// La taille, en cases, du plus grand morceau connecté (huit voisins) de cases occupées au seuil ; `0` s'il n'y en a
    /// aucune.
    pub plus_grand_morceau: usize,
    /// Le nombre de morceaux connectés (huit voisins) de cases occupées au seuil.
    pub nombre_de_morceaux: usize,
    /// Le nombre de cases du plus court chemin de cases occupées au seuil, de la colonne de gauche à celle de droite,
    /// s'il existe.
    pub traversee_gauche_droite: Option<usize>,
    /// Le nombre de cases du plus court chemin de cases occupées au seuil, de la ligne du haut à celle du bas, s'il
    /// existe.
    pub traversee_haut_bas: Option<usize>,
}

/// Mesure la pièce que la colonie remplit à l'instant courant, au seuil nul : une case est occupée dès qu'il y passe de
/// l'hyphe. C'est exactement `mesurer_au_seuil(simulation, 0.0)`.
pub fn mesurer(simulation: &Simulation) -> MesuresPiece {
    mesurer_au_seuil(simulation, 0.0)
}

/// Mesure la pièce que la colonie remplit à l'instant courant, en ne comptant comme occupées que les cases de densité
/// strictement supérieure à `seuil`. Lecture seule : rien n'est changé dans la simulation.
///
/// La carte de densité est construite avec les segments de la simulation ; les cases occupées sont
/// `carte.occupees(seuil)`. Les cases libres sont celles que le substrat ne bloque pas ; les cases bloquées ont une
/// densité nulle (les segments n'y entrent jamais), donc ne sont jamais occupées. Le bilan de nourriture compare ce qui
/// reste de substrat, plus ce que coûtent les hyphes (`cout × biomasse`), à la nourriture de départ.
///
/// Contrat : `seuil` est fini et positif ou nul ; sinon la fonction panique (avant tout calcul).
pub fn mesurer_au_seuil(simulation: &Simulation, seuil: f64) -> MesuresPiece {
    verifier_le_seuil(seuil);
    let reglages = simulation.reglages();
    let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
    let carte = carte_de_densite(largeur, hauteur, simulation.segments());

    let substrat = simulation.substrat();
    let mut cases_libres = 0;
    for y in 0..hauteur {
        for x in 0..largeur {
            if !substrat.est_bloquee(x, y) {
                cases_libres += 1;
            }
        }
    }

    let occupees = carte.occupees(seuil);
    let cases_occupees = occupees.iter().filter(|&&occupee| occupee).count();
    let mut densite_maximale = 0.0f64;
    for &densite in carte.valeurs() {
        densite_maximale = densite_maximale.max(densite);
    }

    // Les tailles des morceaux, de la plus grande à la plus petite : le premier est le plus grand, leur nombre est celui
    // des morceaux (les mêmes valeurs que `plus_grand_morceau` et `morceaux`, calculées une seule fois).
    let tailles = morceaux(largeur, hauteur, &occupees);

    let biomasse = simulation.biomasse();
    let nourriture_initiale = simulation.releves()[0].substrat;
    let nourriture_restante = substrat.masse_totale();
    let nourriture_consommee = reglages.cout * biomasse;

    MesuresPiece {
        cases_libres,
        cases_occupees,
        remplissage: if cases_libres == 0 { 0.0 } else { cases_occupees as f64 / cases_libres as f64 },
        densite_moyenne: if cases_libres == 0 { 0.0 } else { carte.somme() / cases_libres as f64 },
        densite_maximale,
        biomasse,
        nourriture_initiale,
        nourriture_restante,
        nourriture_consommee,
        ecart_du_bilan: (nourriture_restante + nourriture_consommee - nourriture_initiale).abs(),
        plus_grand_morceau: tailles.first().copied().unwrap_or(0),
        nombre_de_morceaux: tailles.len(),
        traversee_gauche_droite: traversee(largeur, hauteur, &occupees, Bord::Gauche, Bord::Droite),
        traversee_haut_bas: traversee(largeur, hauteur, &occupees, Bord::Haut, Bord::Bas),
    }
}
