//! Outils partagés par les tests d'acceptation de l'étape 7 (T2, les obstacles durs et le thigmotropisme).
//!
//! Ce fichier n'est pas un test : chaque fichier `etape_7_*.rs` l'inclut par `#[path = "etape_7/outils.rs"] mod outils;`.
//! Tout y est écrit **hors du code testé** :
//!
//! - la géométrie exacte de la règle du plancher (`Grille`) : chaque coordonnée, un `f64`, est convertie en un entier exact
//!   (multiple de 2⁻⁵⁶) et les instants d'entrée dans chaque case sont comparés comme des fractions d'entiers de 128 bits,
//!   case par case, sans parcours de grille ;
//! - un **modèle de référence** (`Modele`) : la spécification de T2 récrite à part, pour des colonies sans ramification
//!   aléatoire, sans anastomose, sur un substrat constant qui ne s'épuise pas (longueur de pas exactement `pas`) ; il tire
//!   ses nombres d'un `Alea` dans l'ordre de la spécification et calcule les positions avec les mêmes additions
//!   flottantes que la simulation, donc il doit lui être égal à l'octet près ;
//! - la lecture de l'écriture canonique (apex et mémoires), la vérification « jamais un segment dans un obstacle »
//!   (méthode des dalles et échantillonnage), la recherche de graines.
//!
//! Il n'utilise, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `sqrt` et `floor`.
#![allow(dead_code)]

use mycelium_lab::alea::Alea;
use mycelium_lab::croissance::{COS, NB_DIRECTIONS, SIN, Segment};
use mycelium_lab::simulation::{Milieu, Obstacle, Reglages, Simulation, lire_scenario_tropismes};
use mycelium_lab::tropisme::{Face, Tropismes, tourner_chimiotropique};

/// Marge de la vérification par dalles, en cases : un segment n'est fautif que s'il entre de plus de `MARGE` dans un
/// obstacle (les arrondis des flottants ne sont pas des fautes ; l'échantillonnage au plancher complète).
pub const MARGE: f64 = 1e-9;

/// Le filet contre l'emballement : au plus ce nombre de ramifications par colonie, vérifié à chaque pas par les tests qui
/// jouent des colonies. Mesuré sur le prototype corrigé (voir le rapport du juge) : au plus 8 220 sur 200 colonies de 100 pas
/// à l'angle 10, 1 513 à angle nul ; un code qui fait naître des pointes en boucle dépasse la borne en quelques secondes.
pub const RAMIFICATIONS_MAXIMALES: u64 = 40_000;

/// Le délai maximal d'un fichier de tests de l'étape, en secondes, compté depuis son premier test : au-delà, le processus de
/// tests s'arrête en échec (un calcul qui ne se termine pas, par exemple une boucle sans fin dans un parcours de grille,
/// doit faire échouer, jamais pendre). Mesuré : le plus long fichier dure moins de 40 s en profil de test.
pub const DELAI_MAXIMAL: u64 = 180;

/// Arme, une fois par processus, la minuterie du délai maximal. Chaque test l'appelle en premier.
pub fn delai() {
    static UNE_FOIS: std::sync::Once = std::sync::Once::new();
    UNE_FOIS.call_once(|| {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(DELAI_MAXIMAL));
            eprintln!("délai de {DELAI_MAXIMAL} s dépassé : un calcul ne se termine pas (boucle sans fin ?)");
            std::process::exit(101);
        });
    });
}

pub fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

/// La direction du premier apex d'une simulation de graine `graine` : le premier tirage du générateur, modulo 360 (le
/// contrat de la V0).
pub fn direction_initiale(graine: u64) -> usize {
    (Alea::new(graine).suivant() % NB_DIRECTIONS as u64) as usize
}

/// Les `n` premières graines (en partant de `debut`) dont la direction initiale vaut `direction`.
pub fn graines_pour_depuis(direction: usize, n: usize, debut: u64) -> Vec<u64> {
    let mut graines = Vec::new();
    let mut graine = debut;
    while graines.len() < n {
        if direction_initiale(graine) == direction {
            graines.push(graine);
        }
        graine += 1;
    }
    graines
}

/// Les `n` premières graines (en partant de 0) dont la direction initiale vaut `direction`.
pub fn graines_pour(direction: usize, n: usize) -> Vec<u64> {
    graines_pour_depuis(direction, n, 0)
}

/// La première graine dont la direction initiale vaut `direction`.
pub fn graine_pour(direction: usize) -> u64 {
    graines_pour(direction, 1)[0]
}

/// Pour chacune des 360 directions, la première graine dont la direction initiale la vaut.
pub fn graines_par_direction() -> Vec<u64> {
    let mut graines: Vec<Option<u64>> = vec![None; NB_DIRECTIONS];
    let mut restant = NB_DIRECTIONS;
    let mut graine = 0u64;
    while restant > 0 {
        let d = direction_initiale(graine);
        if graines[d].is_none() {
            graines[d] = Some(graine);
            restant -= 1;
        }
        graine += 1;
    }
    graines.into_iter().map(|g| g.expect("une graine par direction")).collect()
}

/// L'écart angulaire entre deux directions, de 0 à 180 degrés.
pub fn ecart_circulaire(a: usize, b: usize) -> usize {
    let e = a.abs_diff(b) % NB_DIRECTIONS;
    e.min(NB_DIRECTIONS - e)
}

/// Les deux directions parallèles à une face, dans l'ordre croissant (écrit ici sans le code testé).
pub fn paralleles(face: Face) -> (usize, usize) {
    match face {
        Face::Verticale => (90, 270),
        Face::Horizontale => (0, 180),
    }
}

/// L'angle de contact attendu, par une méthode indépendante de la formule de la spécification : l'écart angulaire entre
/// la direction et la plus proche des deux directions parallèles à la face.
pub fn angle_attendu(d: usize, face: Face) -> usize {
    let (a, b) = paralleles(face);
    ecart_circulaire(d, a).min(ecart_circulaire(d, b))
}

/// La direction de glissement attendue : la plus proche des deux parallèles ; à égalité, la plus petite.
pub fn glissement_attendu(d: usize, face: Face) -> usize {
    let (a, b) = paralleles(face);
    if ecart_circulaire(d, b) < ecart_circulaire(d, a) { b } else { a }
}

/// La direction d'un segment : l'entrée de la table dont le vecteur unité est le plus proche de celui du segment.
pub fn direction_du_segment(s: &Segment) -> usize {
    let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
    let longueur = (dx * dx + dy * dy).sqrt();
    assert!(longueur > 0.0, "segment sans longueur : {s:?}");
    let (ux, uy) = (dx / longueur, dy / longueur);
    let mut meilleure = 0;
    let mut meilleur_ecart = f64::INFINITY;
    for d in 0..NB_DIRECTIONS {
        let (ex, ey) = (ux - COS[d], uy - SIN[d]);
        let ecart = ex * ex + ey * ey;
        if ecart < meilleur_ecart {
            meilleur_ecart = ecart;
            meilleure = d;
        }
    }
    assert!(meilleur_ecart < 1e-18, "le segment {s:?} n'a la direction d'aucune entrée de la table");
    meilleure
}

// ---------------------------------------------------------------------------------------------------------------------
// La vérification « jamais un segment dans un obstacle » (indépendante de toute règle aux limites).
// ---------------------------------------------------------------------------------------------------------------------

/// Vrai si la case `(i, j)` est dans un des obstacles (rectangles de cases `x0..=x1` × `y0..=y1`).
pub fn case_bloquee(obstacles: &[Obstacle], i: i64, j: i64) -> bool {
    obstacles.iter().any(|o| i >= o.x0 as i64 && i <= o.x1 as i64 && j >= o.y0 as i64 && j <= o.y1 as i64)
}

/// Vrai si le segment entre de plus de `MARGE` dans l'intérieur du rectangle plein de l'obstacle (méthode des dalles).
pub fn segment_penetre(s: &Segment, o: &Obstacle) -> bool {
    let bornes = [
        (s.x0, s.x1 - s.x0, o.x0 as f64 + MARGE, o.x1 as f64 + 1.0 - MARGE),
        (s.y0, s.y1 - s.y0, o.y0 as f64 + MARGE, o.y1 as f64 + 1.0 - MARGE),
    ];
    let (mut t_min, mut t_max) = (0.0f64, 1.0f64);
    for (depart, delta, bas, haut) in bornes {
        if delta == 0.0 {
            if depart < bas || depart > haut {
                return false;
            }
        } else {
            let (a, b) = ((bas - depart) / delta, (haut - depart) / delta);
            t_min = t_min.max(a.min(b));
            t_max = t_max.min(a.max(b));
            if t_min > t_max {
                return false;
            }
        }
    }
    true
}

/// La première faute de la colonie contre les obstacles, ou `None` : un segment qui entre dans un obstacle (dalles), ou
/// dont une extrémité, ou un des 16 points intérieurs, tombe dans une case bloquée au sens du plancher.
pub fn premiere_faute(segments: &[Segment], obstacles: &[Obstacle]) -> Option<String> {
    for (numero, s) in segments.iter().enumerate() {
        for o in obstacles {
            if segment_penetre(s, o) {
                return Some(format!("segment {numero} {s:?} entre dans l'obstacle {o:?}"));
            }
        }
        for k in 0..18 {
            let t = if k == 0 {
                0.0
            } else if k == 17 {
                1.0
            } else {
                (k as f64 - 1.0 + 0.37) / 16.0
            };
            let (x, y) = (s.x0 + t * (s.x1 - s.x0), s.y0 + t * (s.y1 - s.y0));
            if case_bloquee(obstacles, x.floor() as i64, y.floor() as i64) {
                return Some(format!("segment {numero} {s:?} : le point t = {t} ({x}, {y}) est dans une case bloquée"));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------------------------------
// La géométrie exacte de la règle du plancher.
// ---------------------------------------------------------------------------------------------------------------------

/// L'échelle des entiers exacts : une coordonnée `x` devient `x · 2¹⁰⁰`.
const ECHELLE: i128 = 1 << 100;

/// Convertit une coordonnée en entier exact. Domaine : `0.0` ou `[2⁻⁴⁸, 128)` (au-delà, les entiers sortiraient des 128
/// bits ; en deçà, la coordonnée pourrait avoir des bits plus fins que 2⁻¹⁰⁰). Hors du domaine, le test refuse ses
/// propres hypothèses (aucun dispositif des tests n'en approche).
fn exact(x: f64) -> i128 {
    assert!(x == 0.0 || (3.6e-15..128.0).contains(&x), "coordonnée {x} hors du domaine de l'oracle exact");
    let v = x * ECHELLE as f64;
    assert!(v == v.floor(), "coordonnée {x} non représentable à 2^-100 près");
    v as i128
}

/// Le produit exact de deux entiers de 128 bits, en (signe, poids fort, poids faible) sur 256 bits.
fn produit(a: i128, b: i128) -> (bool, u128, u128) {
    let negatif = (a < 0) != (b < 0) && a != 0 && b != 0;
    let (x, y) = (a.unsigned_abs(), b.unsigned_abs());
    const M: u128 = (1 << 64) - 1;
    let (x1, x0, y1, y0) = (x >> 64, x & M, y >> 64, y & M);
    let (p00, p01, p10, p11) = (x0 * y0, x0 * y1, x1 * y0, x1 * y1);
    let milieu = (p00 >> 64) + (p01 & M) + (p10 & M);
    let faible = (p00 & M) | (milieu << 64);
    let fort = p11 + (p01 >> 64) + (p10 >> 64) + (milieu >> 64);
    (negatif, fort, faible)
}

/// `a · b` comparé à `c · d`, exactement.
fn comparer_produits(a: i128, b: i128, c: i128, d: i128) -> std::cmp::Ordering {
    let (n1, f1, l1) = produit(a, b);
    let (n2, f2, l2) = produit(c, d);
    let nul1 = f1 == 0 && l1 == 0;
    let nul2 = f2 == 0 && l2 == 0;
    let signe = |n: bool, nul: bool| {
        if nul {
            0
        } else if n {
            -1
        } else {
            1
        }
    };
    let (s1, s2) = (signe(n1, nul1), signe(n2, nul2));
    if s1 != s2 {
        return s1.cmp(&s2);
    }
    let grandeur = (f1, l1).cmp(&(f2, l2));
    if s1 < 0 { grandeur.reverse() } else { grandeur }
}

/// Une borne d'instant `num / den` (`den > 0`), fermée ou ouverte, ou « depuis toujours ».
#[derive(Clone, Copy, Debug)]
struct Borne {
    num: i128,
    den: i128,
    fermee: bool,
    infinie: bool,
}

/// `a` comparé à `b` en valeur (les bornes infinies sont plus petites que tout).
fn comparer(a: &Borne, b: &Borne) -> std::cmp::Ordering {
    match (a.infinie, b.infinie) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => comparer_produits(a.num, b.den, b.num, a.den),
    }
}

/// Une grille de cases bloquées, et la règle du plancher calculée exactement.
#[derive(Clone, Debug)]
pub struct Grille {
    pub largeur: usize,
    pub hauteur: usize,
    bloquees: Vec<bool>,
}

/// Le résultat exact d'un segment : libre, ou la case bloquée touchée en premier et la face d'entrée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Libre,
    Touche { case: (i64, i64), face: Face },
}

impl Grille {
    pub fn nouvelle(largeur: usize, hauteur: usize, murs: &[(usize, usize, usize, usize)]) -> Grille {
        let mut bloquees = vec![false; largeur * hauteur];
        for &(x0, y0, x1, y1) in murs {
            for y in y0..=y1.min(hauteur - 1) {
                for x in x0..=x1.min(largeur - 1) {
                    bloquees[y * largeur + x] = true;
                }
            }
        }
        Grille { largeur, hauteur, bloquees }
    }

    pub fn bloquee(&self, i: i64, j: i64) -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < self.largeur
            && (j as usize) < self.hauteur
            && self.bloquees[j as usize * self.largeur + i as usize]
    }

    /// Les instants (bas, haut) où la coordonnée `p(t) = p0 + t·d` est dans `[c, c + 1)` (à l'échelle exacte), ou
    /// `None` si jamais.
    fn intervalle(p0: i128, d: i128, c: i64) -> Option<(Borne, Borne)> {
        let bas_c = c as i128 * ECHELLE;
        let haut_c = bas_c + ECHELLE;
        let toujours = Borne { num: 0, den: 1, fermee: true, infinie: true };
        if d == 0 {
            return if p0 >= bas_c && p0 < haut_c { Some((toujours, toujours)) } else { None };
        }
        if d > 0 {
            Some((
                Borne { num: bas_c - p0, den: d, fermee: true, infinie: false },
                Borne { num: haut_c - p0, den: d, fermee: false, infinie: false },
            ))
        } else {
            Some((
                Borne { num: p0 - haut_c, den: -d, fermee: false, infinie: false },
                Borne { num: p0 - bas_c, den: -d, fermee: true, infinie: false },
            ))
        }
    }

    /// Pour la case `(i, j)` : `None` si le segment fermé ne la touche pas, sinon `(entrée, bas_x, bas_y)`.
    fn entree(&self, p: [i128; 4], i: i64, j: i64) -> Option<(Borne, Borne, Borne)> {
        let (x0, y0, x1, y1) = (p[0], p[1], p[2], p[3]);
        let (bx, hx) = Self::intervalle(x0, x1 - x0, i)?;
        let (by, hy) = Self::intervalle(y0, y1 - y0, j)?;
        let zero = Borne { num: 0, den: 1, fermee: true, infinie: false };
        let un = Borne { num: 1, den: 1, fermee: true, infinie: false };
        // Le bas de l'intersection : le plus grand des bas ; fermé si tous ceux qui l'atteignent sont fermés.
        let mut bas = zero;
        for b in [bx, by] {
            match comparer(&b, &bas) {
                std::cmp::Ordering::Greater => bas = b,
                std::cmp::Ordering::Equal => bas.fermee = bas.fermee && b.fermee,
                std::cmp::Ordering::Less => {}
            }
        }
        let mut haut = un;
        for h in [hx, hy] {
            let h = if h.infinie { un } else { h };
            match comparer(&h, &haut) {
                std::cmp::Ordering::Less => haut = h,
                std::cmp::Ordering::Equal => haut.fermee = haut.fermee && h.fermee,
                std::cmp::Ordering::Greater => {}
            }
        }
        let vide = match comparer(&bas, &haut) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Equal => !(bas.fermee && haut.fermee),
            std::cmp::Ordering::Less => false,
        };
        if vide { None } else { Some((bas, bx, by)) }
    }

    /// Le contact exact du segment de `(x0, y0)` à `(x1, y1)` selon la règle du plancher. La case de départ doit être
    /// libre (sinon, le test refuse ses propres hypothèses).
    pub fn contact(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Contact {
        assert!(!self.bloquee(x0.floor() as i64, y0.floor() as i64), "oracle : départ ({x0}, {y0}) bloqué");
        let p = [exact(x0), exact(y0), exact(x1), exact(y1)];
        let (imin, imax) = (x0.min(x1).floor() as i64, x0.max(x1).floor() as i64);
        let (jmin, jmax) = (y0.min(y1).floor() as i64, y0.max(y1).floor() as i64);
        let mut meilleur: Option<((i64, i64), Borne, Borne, Borne)> = None;
        for j in jmin..=jmax {
            for i in imin..=imax {
                if !self.bloquee(i, j) {
                    continue;
                }
                if let Some((e, bx, by)) = self.entree(p, i, j) {
                    let plus_tot = match &meilleur {
                        None => true,
                        Some((_, m, _, _)) => match comparer(&e, m) {
                            std::cmp::Ordering::Less => true,
                            std::cmp::Ordering::Equal => e.fermee && !m.fermee,
                            std::cmp::Ordering::Greater => false,
                        },
                    };
                    if plus_tot {
                        meilleur = Some(((i, j), e, bx, by));
                    }
                }
            }
        }
        let Some(((i, j), _, bx, by)) = meilleur else {
            return Contact::Libre;
        };
        // Un axe déjà dans l'intervalle au départ (borne basse négative, ou nulle et atteinte) y est « depuis toujours ».
        let zero = Borne { num: 0, den: 1, fermee: true, infinie: false };
        let normaliser = |b: Borne| match comparer(&b, &zero) {
            std::cmp::Ordering::Less => Borne { infinie: true, ..b },
            std::cmp::Ordering::Equal if b.fermee => Borne { infinie: true, ..b },
            _ => b,
        };
        let (bx, by) = (normaliser(bx), normaliser(by));
        let face = match comparer(&bx, &by) {
            std::cmp::Ordering::Greater => Face::Verticale,
            std::cmp::Ordering::Less => Face::Horizontale,
            std::cmp::Ordering::Equal => {
                // Par un coin : la case d'où il vient en diagonale, et ses deux voisines.
                let sx = if p[2] > p[0] { 1 } else { -1 };
                let sy = if p[3] > p[1] { 1 } else { -1 };
                let (px, py) = (i - sx, j - sy);
                match (self.bloquee(i, py), self.bloquee(px, j)) {
                    (false, true) => Face::Horizontale,
                    _ => Face::Verticale,
                }
            }
        };
        Contact::Touche { case: (i, j), face }
    }

    /// Vrai si le segment est libre : départ libre et aucun contact.
    pub fn libre(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
        !self.bloquee(x0.floor() as i64, y0.floor() as i64) && self.contact(x0, y0, x1, y1) == Contact::Libre
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Scénarios.
// ---------------------------------------------------------------------------------------------------------------------

/// Lit un scénario couplé complet (milieu et tropismes) et le construit.
pub fn construire(texte: &str, graine: u64) -> (Simulation, u32) {
    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(texte).expect("le scénario se lit");
    let simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine).expect("valide");
    (simulation, duree)
}

/// Lit un scénario, le construit et le joue sur toute sa durée, avec le filet contre l'emballement à chaque pas.
pub fn jouer(texte: &str, graine: u64) -> Simulation {
    let (mut simulation, duree) = construire(texte, graine);
    for pas in 1..=duree {
        simulation.avancer();
        assert!(
            simulation.ramifications() <= RAMIFICATIONS_MAXIMALES,
            "graine {graine} : {} ramifications au pas {pas} (emballement)",
            simulation.ramifications()
        );
    }
    simulation
}

/// Lit un scénario et rend ses morceaux, sans construire.
pub fn lire(texte: &str) -> (Reglages, Milieu, Tropismes, u32) {
    lire_scenario_tropismes(texte).expect("le scénario se lit")
}

/// Un dispositif pour le modèle de référence : boîte, spore, murs, longueur de pas, amplitude de la marche, angle.
#[derive(Clone, Debug)]
pub struct Salle {
    pub largeur: usize,
    pub hauteur: usize,
    pub spore: (usize, usize),
    pub murs: Vec<(usize, usize, usize, usize)>,
    pub pas: f64,
    pub affinite: f64,
    pub amplitude: u32,
    /// L'intervalle de ramification aléatoire (`f64::INFINITY` : aucune), angle de ramification 30.
    pub intervalle: f64,
    pub thigmotropisme: f64,
    /// Des sources `(x, y, rayon, quantité)` posées sur le substrat initial 1 (sans diffusion, elles ne bougent pas) : la
    /// longueur du pas varie alors d'une case à l'autre.
    pub sources: Vec<(usize, usize, usize, f64)>,
    /// La force du chimiotropisme (T1), `0.0` pour l'éteindre.
    pub chimiotropisme: f64,
}

impl Salle {
    /// Le texte du scénario : ramification aléatoire selon `intervalle` (angle 30), aucune anastomose, substrat constant (1) sans diffusion, coût
    /// nul : la longueur de chaque pas vaut exactement `(pas · 1) / (affinite + 1)`, dans toute case libre.
    pub fn texte(&self, duree: u32) -> String {
        let mut texte = format!(
            "boite {} {}\nspore {} {}\nsubstrat 1.0 0.0\ncroissance {} {} 30 {}\ncouplage {} 0.0\nduree {duree}\n",
            self.largeur,
            self.hauteur,
            self.spore.0,
            self.spore.1,
            self.pas,
            if self.intervalle.is_infinite() { "infini".to_string() } else { self.intervalle.to_string() },
            self.amplitude,
            self.affinite
        );
        for (x0, y0, x1, y1) in &self.murs {
            texte.push_str(&format!("obstacle {x0} {y0} {x1} {y1}\n"));
        }
        for (x, y, rayon, quantite) in &self.sources {
            texte.push_str(&format!("source {x} {y} {rayon} {quantite}\n"));
        }
        if self.chimiotropisme > 0.0 {
            texte.push_str(&format!("chimiotropisme {}\n", self.chimiotropisme));
        }
        texte.push_str(&format!("thigmotropisme {}\n", self.thigmotropisme));
        texte
    }

    pub fn simulation(&self, graine: u64) -> Simulation {
        construire(&self.texte(1), graine).0
    }

    pub fn modele(&self, graine: u64) -> Modele {
        Modele::nouveau(self, graine)
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le modèle de référence : la spécification de T2 récrite à part.
// ---------------------------------------------------------------------------------------------------------------------

/// Le substrat initial d'une salle : 1 dans chaque case, remplacé par la dernière source qui la contient (disque
/// `di² + dj² ≤ rayon²` autour de la case de la source), 0 dans une case bloquée.
fn substrat_constant(salle: &Salle) -> Vec<f64> {
    let mut v = vec![0.0; salle.largeur * salle.hauteur];
    let grille = Grille::nouvelle(salle.largeur, salle.hauteur, &salle.murs);
    for j in 0..salle.hauteur {
        for i in 0..salle.largeur {
            let mut q = 1.0;
            for &(x, y, rayon, quantite) in &salle.sources {
                let (di, dj) = (i as i64 - x as i64, j as i64 - y as i64);
                if di * di + dj * dj <= (rayon * rayon) as i64 {
                    q = quantite;
                }
            }
            v[j * salle.largeur + i] = if grille.bloquee(i as i64, j as i64) { 0.0 } else { q };
        }
    }
    v
}

/// Une pointe du modèle de référence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointe {
    pub x: f64,
    pub y: f64,
    pub direction: usize,
    pub actif: bool,
    pub memoire: Option<(usize, usize)>,
}

/// Le modèle de référence d'une `Salle`.
#[derive(Clone, Debug)]
pub struct Modele {
    pub grille: Grille,
    pub pas: f64,
    pub affinite: f64,
    pub amplitude: u32,
    pub intervalle: f64,
    pub angle: f64,
    pub chimiotropisme: f64,
    /// Le substrat de chaque case (ligne par ligne), constant : 1, remplacé par la dernière source qui la contient, 0
    /// dans une case bloquée.
    pub substrat: Vec<f64>,
    pub alea: Alea,
    pub pointes: Vec<Pointe>,
    /// Les segments `(apex, temps, x0, y0, x1, y1)`.
    pub segments: Vec<(u32, u32, f64, f64, f64, f64)>,
    pub ramifications: u64,
    pub temps: u32,
}

impl Modele {
    pub fn nouveau(salle: &Salle, graine: u64) -> Modele {
        let mut alea = Alea::new(graine);
        let direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
        let pointe = Pointe {
            x: salle.spore.0 as f64 + 0.5,
            y: salle.spore.1 as f64 + 0.5,
            direction,
            actif: true,
            memoire: None,
        };
        Modele {
            grille: Grille::nouvelle(salle.largeur, salle.hauteur, &salle.murs),
            pas: salle.pas,
            affinite: salle.affinite,
            amplitude: salle.amplitude,
            intervalle: salle.intervalle,
            angle: salle.thigmotropisme,
            chimiotropisme: salle.chimiotropisme,
            substrat: substrat_constant(salle),
            alea,
            pointes: vec![pointe],
            segments: Vec::new(),
            ramifications: 0,
            temps: 0,
        }
    }

    fn dans_la_boite(&self, x: f64, y: f64) -> bool {
        x >= 0.0 && y >= 0.0 && x < self.grille.largeur as f64 && y < self.grille.hauteur as f64
    }

    /// Le substrat de la case `(i, j)`.
    fn valeur(&self, i: usize, j: usize) -> f64 {
        self.substrat[j * self.grille.largeur + i]
    }

    /// Le gradient de T1 : différences centrées, un voisin hors de la grille ou bloqué remplacé par la case.
    fn gradient(&self, i: usize, j: usize) -> (f64, f64) {
        let c = self.valeur(i, j);
        let (l, h) = (self.grille.largeur as i64, self.grille.hauteur as i64);
        let v = |a: i64, b: i64| {
            if a < 0 || b < 0 || a >= l || b >= h || self.grille.bloquee(a, b) {
                c
            } else {
                self.valeur(a as usize, b as usize)
            }
        };
        let (i, j) = (i as i64, j as i64);
        ((v(i + 1, j) - v(i - 1, j)) * 0.5, (v(i, j + 1) - v(i, j - 1)) * 0.5)
    }

    /// Un pas de longueur `l` dans la direction `d` depuis `(x, y)` : arrivée dans la boîte et segment libre.
    fn pas_libre(&self, x: f64, y: f64, l: f64, d: usize) -> bool {
        let (x1, y1) = (x + l * COS[d], y + l * SIN[d]);
        self.dans_la_boite(x1, y1) && self.grille.libre(x, y, x1, y1)
    }

    /// Un pas de temps, selon la spécification.
    pub fn avancer(&mut self) {
        self.temps += 1;
        let n = self.pointes.len();
        for i in 0..n {
            let mut p = self.pointes[i];
            if !p.actif {
                continue;
            }
            let largeur = 2 * self.amplitude as u64 + 1;
            let delta = (self.alea.suivant() % largeur) as i64 - self.amplitude as i64;
            p.direction = (p.direction as i64 + delta).rem_euclid(360) as usize;
            // La longueur de la V0, dans la case de la pointe (coût nul : aucune borne).
            let s = self.valeur(p.x as usize, p.y as usize);
            let l = if s <= 0.0 { 0.0 } else { (self.pas * s) / (self.affinite + s) };
            if let Some((cap, g)) = p.memoire {
                let ecart = (self.alea.suivant() % 41) as i64 - 20;
                let c = (cap as i64 + ecart).rem_euclid(360) as usize;
                if self.pas_libre(p.x, p.y, self.pas, cap) && self.pas_libre(p.x, p.y, l, c) {
                    p.direction = c;
                    p.memoire = None;
                } else {
                    p.direction = g;
                }
            } else if self.chimiotropisme > 0.0 {
                let gradient = self.gradient(p.x as usize, p.y as usize);
                p.direction = tourner_chimiotropique(p.direction, gradient, self.chimiotropisme, &mut self.alea);
            }
            let d = p.direction;
            let (mut x1, mut y1) = (p.x + l * COS[d], p.y + l * SIN[d]);
            if l <= 0.0 || !self.dans_la_boite(x1, y1) {
                p.memoire = None;
                self.pointes[i] = p;
                continue;
            }
            if let Contact::Touche { face, .. } = self.grille.contact(p.x, p.y, x1, y1) {
                if self.angle <= 0.0 {
                    self.pointes[i] = p;
                    continue;
                }
                let mut arret = Some(face);
                if (angle_attendu(d, face) as f64) < self.angle {
                    let g = glissement_attendu(d, face);
                    p.memoire = Some((d, g));
                    p.direction = g;
                    x1 = p.x + l * COS[g];
                    y1 = p.y + l * SIN[g];
                    if !self.dans_la_boite(x1, y1) {
                        p.memoire = None;
                        self.pointes[i] = p;
                        continue;
                    }
                    arret = match self.grille.contact(p.x, p.y, x1, y1) {
                        Contact::Libre => None,
                        Contact::Touche { face, .. } => Some(face),
                    };
                }
                if let Some(face) = arret {
                    p.actif = false;
                    p.memoire = None;
                    self.pointes[i] = p;
                    let (a, b) = paralleles(face);
                    for b in [a, b] {
                        if self.pas_libre(p.x, p.y, self.pas, b) {
                            self.pointes.push(Pointe { x: p.x, y: p.y, direction: b, actif: true, memoire: None });
                            self.ramifications += 1;
                        }
                    }
                    continue;
                }
            }
            let d_pas = p.direction;
            self.segments.push((i as u32, self.temps, p.x, p.y, x1, y1));
            p.x = x1;
            p.y = y1;
            self.pointes[i] = p;
            // La ramification aléatoire de la V0 : un tirage, puis le côté ; la pointe née n'a pas de mémoire et part de
            // la direction du pas (celle de glissement pour une pointe qui longe), à ±30°.
            let probabilite = if self.intervalle.is_infinite() { 0.0 } else { 1.0 / self.intervalle };
            if probabilite > 0.0 && self.alea.suivant_f64() < probabilite {
                let cote = self.alea.suivant() % 2;
                let decalage: i64 = if cote == 0 { 30 } else { -30 };
                let direction = (d_pas as i64 + decalage).rem_euclid(360) as usize;
                self.pointes.push(Pointe { x: x1, y: y1, direction, actif: true, memoire: None });
                self.ramifications += 1;
            }
        }
    }

    pub fn actives(&self) -> usize {
        self.pointes.iter().filter(|p| p.actif).count()
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// La lecture de l'écriture canonique.
// ---------------------------------------------------------------------------------------------------------------------

fn u32_a(o: &[u8], k: usize) -> u32 {
    u32::from_le_bytes(o[k..k + 4].try_into().unwrap())
}
fn u64_a(o: &[u8], k: usize) -> u64 {
    u64::from_le_bytes(o[k..k + 8].try_into().unwrap())
}
fn f64_a(o: &[u8], k: usize) -> f64 {
    f64::from_bits(u64_a(o, k))
}

/// Un apex lu dans l'écriture canonique : position, direction, actif, et sa mémoire `(drapeau, cap, g)` si la queue du
/// thigmotropisme est écrite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApexLu {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub direction: u32,
    pub actif: bool,
    pub memoire: Option<(u8, u32, u32)>,
}

/// Lit les apex et, si la queue du thigmotropisme est là, leurs mémoires. Le format est celui de la spécification :
/// segments, apex (25 octets), trois compteurs, substrat, queue du milieu, chimiotropisme s'il est non nul, puis l'angle
/// et 9 octets par apex.
pub fn apex_lus(simulation: &Simulation) -> Vec<ApexLu> {
    let o = simulation.ecrire_canonique();
    let nb_segments = u64_a(&o, 4) as usize;
    let mut k = 12 + 40 * nb_segments;
    let nb_apex = 1 + simulation.ramifications() as usize;
    let mut apex = Vec::new();
    for _ in 0..nb_apex {
        apex.push(ApexLu {
            id: u32_a(&o, k),
            x: f64_a(&o, k + 4),
            y: f64_a(&o, k + 12),
            direction: u32_a(&o, k + 20),
            actif: o[k + 24] == 1,
            memoire: None,
        });
        k += 25;
    }
    k += 24;
    let (largeur, hauteur) = (u64_a(&o, k) as usize, u64_a(&o, k + 8) as usize);
    k += 16 + 8 * largeur * hauteur;
    if simulation.tropismes().thigmotropisme == 0.0 {
        return apex;
    }
    let nb_sources = u64_a(&o, k) as usize;
    k += 8 + 20 * nb_sources;
    let nb_obstacles = u64_a(&o, k) as usize;
    k += 8 + 16 * nb_obstacles;
    if simulation.tropismes().chimiotropisme != 0.0 {
        k += 8;
    }
    assert_eq!(f64_a(&o, k).to_bits(), simulation.tropismes().thigmotropisme.to_bits(), "l'angle de la queue");
    k += 8;
    assert_eq!(o.len(), k + 9 * nb_apex, "la queue du thigmotropisme : 9 octets par apex");
    for a in apex.iter_mut() {
        a.memoire = Some((o[k], u32_a(&o, k + 1), u32_a(&o, k + 5)));
        k += 9;
    }
    apex
}

/// Compare la simulation au modèle de référence, après le même nombre de pas : segments (à l'octet), compteurs, apex
/// (position, direction, actif) et mémoires. Rend la première différence.
pub fn ecart_au_modele(simulation: &Simulation, modele: &Modele) -> Option<String> {
    ecart_au_modele_depuis(simulation, modele, 0, true)
}

/// Comme `ecart_au_modele`, en ne relisant que les segments à partir du numéro `deja` (les précédents ont été comparés),
/// et les apex seulement si `avec_apex`.
pub fn ecart_au_modele_depuis(
    simulation: &Simulation,
    modele: &Modele,
    deja: usize,
    avec_apex: bool,
) -> Option<String> {
    let segments = simulation.segments();
    if segments.len() != modele.segments.len() {
        return Some(format!(
            "au temps {} : {} segments, le modèle en attend {} (dernier attendu {:?}, dernier obtenu {:?})",
            modele.temps,
            segments.len(),
            modele.segments.len(),
            modele.segments.last(),
            segments.last()
        ));
    }
    for (s, &(apex, temps, x0, y0, x1, y1)) in segments.iter().zip(&modele.segments).skip(deja) {
        if (s.apex, s.temps, s.x0.to_bits(), s.y0.to_bits(), s.x1.to_bits(), s.y1.to_bits())
            != (apex, temps, x0.to_bits(), y0.to_bits(), x1.to_bits(), y1.to_bits())
        {
            return Some(format!("segment {s:?}, le modèle attend {:?}", (apex, temps, x0, y0, x1, y1)));
        }
    }
    if simulation.ramifications() != modele.ramifications {
        return Some(format!(
            "au temps {} : {} ramifications, le modèle en attend {}",
            modele.temps,
            simulation.ramifications(),
            modele.ramifications
        ));
    }
    if simulation.pointes() != modele.actives() {
        return Some(format!("{} pointes actives, le modèle en attend {}", simulation.pointes(), modele.actives()));
    }
    if !avec_apex {
        return None;
    }
    for (a, p) in apex_lus(simulation).iter().zip(&modele.pointes) {
        let memoire = match p.memoire {
            Some((cap, g)) => (1u8, cap as u32, g as u32),
            None => (0, 0, 0),
        };
        if (a.x.to_bits(), a.y.to_bits(), a.direction, a.actif)
            != (p.x.to_bits(), p.y.to_bits(), p.direction as u32, p.actif)
            || a.memoire.is_some_and(|m| m != memoire)
        {
            return Some(format!("au temps {} : apex {a:?}, le modèle attend {p:?}", modele.temps));
        }
    }
    None
}

/// Joue la simulation et le modèle `pas` pas, en les comparant après chaque pas ; panique à la première différence.
pub fn suivre_le_modele(salle: &Salle, graine: u64, pas: u32, contexte: &str) -> (Simulation, Modele) {
    let mut simulation = salle.simulation(graine);
    let mut modele = salle.modele(graine);
    if let Some(e) = ecart_au_modele(&simulation, &modele) {
        panic!("{contexte}, graine {graine}, au départ : {e}");
    }
    for k in 1..=pas {
        let deja = simulation.segments().len().min(modele.segments.len());
        simulation.avancer();
        modele.avancer();
        assert!(simulation.ramifications() <= RAMIFICATIONS_MAXIMALES, "{contexte} : emballement");
        // Les segments et les compteurs à chaque pas ; les apex et leurs mémoires tous les cinq pas et au dernier.
        if let Some(e) = ecart_au_modele_depuis(&simulation, &modele, deja, k % 5 == 0 || k == pas) {
            panic!("{contexte}, graine {graine}, entre les pas {} et {k} : {e}", k.saturating_sub(5));
        }
    }
    (simulation, modele)
}
