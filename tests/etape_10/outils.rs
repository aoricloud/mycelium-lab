//! Outils partagés par les tests d'acceptation de l'étape 10 (M1, le moule).
//!
//! Ce fichier n'est pas un test : chaque fichier `etape_10_*.rs` l'inclut par `#[path = "etape_10/outils.rs"] mod outils;`.
//! Tout y est écrit **hors du code testé** (rien n'est repris de `moule.rs`, sauf les types `Forme` et `ErreurMoule` qu'on
//! donne en entrée) :
//!
//! - un petit générateur pseudo-aléatoire propre aux tests (`Hasard`), qui n'est pas l'`Alea` du dépôt ;
//! - les **formes**, une case à la fois, en `i64` (`dans_la_forme`) : la brique, le L (comme l'union de ses deux barres), le
//!   disque et l'anneau (carrés des distances) ;
//! - le **labyrinthe de référence** : la génération par une recherche en profondeur **récursive** (le code testé a une pile
//!   explicite), puis un rendu case par case qui classe chaque case par son reste modulo le pas (`rendre_le_labyrinthe`), sans
//!   rien de commun avec le dessin chambre par chambre du code testé ;
//! - des dessins en texte (`dessin`, `grille` : `.` une case libre, `#` une case bloquée) ;
//! - la **décomposition de référence** des obstacles (par une table `BTreeMap` des rectangles ouverts) et le comptage de la
//!   couverture d'une liste de rectangles ;
//! - trois **chemins de référence** : par relaxations successives (sans file), par une profondeur récursive, et le test de
//!   « 4 voisins » ;
//! - la lecture des scénarios `scenarios/moules/*.txt`, les formes qu'ils doivent décrire, et la lecture d'un PNG ;
//! - l'arme contre les boucles sans fin : `delai()`.
//!
//! Il n'utilise, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `sqrt`, `floor`, `min` et `max`.
#![allow(dead_code)]

use std::collections::BTreeMap;

use mycelium_lab::alea::Alea;
use mycelium_lab::moule::Forme;
use mycelium_lab::simulation::{Obstacle, Reglages, Simulation, lire_scenario_tropismes};

/// Le délai maximal d'un fichier de tests, en secondes, compté depuis son premier test : au-delà, le processus s'arrête en
/// échec (une boucle sans fin dans une recherche de chemin ou un labyrinthe doit faire échouer, jamais pendre).
pub const DELAI_MAXIMAL: u64 = 900;

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

/// Un générateur pseudo-aléatoire (SplitMix64) : déterministe, sans dépendance, propre aux tests.
pub struct Hasard(u64);

impl Hasard {
    pub fn new(graine: u64) -> Hasard {
        Hasard(graine)
    }

    pub fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Un entier de `0` à `n - 1`.
    pub fn jusqu_a(&mut self, n: usize) -> usize {
        (self.suivant() % n as u64) as usize
    }

    /// Un entier de `a` à `b` inclus.
    pub fn entre(&mut self, a: usize, b: usize) -> usize {
        a + self.jusqu_a(b - a + 1)
    }

    /// Vrai avec la probabilité `num / den`.
    pub fn avec_proba(&mut self, num: usize, den: usize) -> bool {
        self.jusqu_a(den) < num
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Les dessins
// ---------------------------------------------------------------------------------------------------------------------

/// Dessine une grille de cases libres : une ligne de texte par ligne de cases, `.` pour une case libre (la cavité) et `#` pour
/// une case bloquée, séparées par `\n` (pas de `\n` final). Le haut du dessin est la ligne 0.
pub fn dessin(largeur: usize, libres: &[bool]) -> String {
    let lignes: Vec<String> = libres
        .chunks(largeur)
        .map(|ligne| ligne.iter().map(|&libre| if libre { '.' } else { '#' }).collect())
        .collect();
    lignes.join("\n")
}

/// Lit un dessin (voir `dessin`) : les lignes sont rognées, les lignes vides ignorées. Rend `(largeur, hauteur, libres)`.
pub fn grille(texte: &str) -> (usize, usize, Vec<bool>) {
    let lignes: Vec<&str> = texte.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let largeur = lignes[0].len();
    let mut libres = Vec::new();
    for ligne in &lignes {
        assert_eq!(ligne.len(), largeur, "toutes les lignes du dessin ont la même largeur : {ligne}");
        for c in ligne.chars() {
            assert!(c == '#' || c == '.', "caractère de dessin inconnu : {c}");
            libres.push(c == '.');
        }
    }
    (largeur, lignes.len(), libres)
}

/// Le nombre de cases libres.
pub fn compter(libres: &[bool]) -> usize {
    libres.iter().filter(|&&libre| libre).count()
}

// ---------------------------------------------------------------------------------------------------------------------
// Les formes
// ---------------------------------------------------------------------------------------------------------------------

/// Vrai si la case `(x, y)` est dans la forme, calculé en `i64` d'après la spécification. Le labyrinthe n'a pas de prédicat
/// de case : voir `libres_de_reference`.
pub fn dans_la_forme(forme: &Forme, x: i64, y: i64) -> bool {
    let n = |v: usize| v as i64;
    match *forme {
        Forme::Brique { x0, y0, x1, y1 } => n(x0) <= x && x <= n(x1) && n(y0) <= y && y <= n(y1),
        Forme::L { x0, y0, x1, y1, epaisseur } => {
            // La barre de gauche : les `epaisseur` premières colonnes, sur toute la hauteur ; la barre du bas : les
            // `epaisseur` dernières lignes, sur toute la largeur.
            let gauche = n(x0) <= x && x < n(x0) + n(epaisseur) && n(y0) <= y && y <= n(y1);
            let bas = n(x0) <= x && x <= n(x1) && n(y1) - n(epaisseur) < y && y <= n(y1);
            gauche || bas
        }
        Forme::Disque { cx, cy, rayon } => (x - n(cx)) * (x - n(cx)) + (y - n(cy)) * (y - n(cy)) <= n(rayon) * n(rayon),
        Forme::Anneau { cx, cy, rayon_exterieur, rayon_interieur } => {
            let d = (x - n(cx)) * (x - n(cx)) + (y - n(cy)) * (y - n(cy));
            n(rayon_interieur) * n(rayon_interieur) < d && d <= n(rayon_exterieur) * n(rayon_exterieur)
        }
        Forme::Labyrinthe { .. } => panic!("le labyrinthe n'a pas de prédicat de case"),
    }
}

/// Les cases libres d'une forme dans une boîte, comme les attend la spécification (la forme est supposée valide et dans la
/// boîte) : par le prédicat pour les formes simples, par le labyrinthe de référence pour le labyrinthe.
pub fn libres_de_reference(largeur: usize, hauteur: usize, forme: &Forme) -> Vec<bool> {
    if let &Forme::Labyrinthe { colonnes, rangees, couloir, graine } = forme {
        let passages = passages_de_reference(colonnes, rangees, graine);
        return rendre_le_labyrinthe(largeur, hauteur, colonnes, rangees, couloir, &passages);
    }
    let mut libres = Vec::with_capacity(largeur * hauteur);
    for y in 0..hauteur {
        for x in 0..largeur {
            libres.push(dans_la_forme(forme, x as i64, y as i64));
        }
    }
    libres
}

/// Le nombre de cases du disque de rayon `r` (le comptage de Gauss), en comptant les cases une à une.
pub fn gauss(r: usize) -> usize {
    let r = r as i64;
    let mut compte = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                compte += 1;
            }
        }
    }
    compte
}

// ---------------------------------------------------------------------------------------------------------------------
// Le labyrinthe de référence
// ---------------------------------------------------------------------------------------------------------------------

/// Un passage : les deux chambres qu'il relie, `(colonne, rangée)` puis `(colonne, rangée)`, dans l'ordre où la génération
/// l'a ouvert (de la chambre courante vers la nouvelle).
pub type Passage = (usize, usize, usize, usize);

/// Les passages du labyrinthe parfait de la graine donnée, par une recherche en profondeur **récursive** : de la chambre
/// `(c, r)`, tant qu'elle a des voisines non visitées (dans l'ordre haut, droite, bas, gauche), on en tire une avec
/// `Alea::suivant() % n`, on ouvre le passage, on la marque et on la visite à fond ; le tirage a lieu même pour `n = 1`.
pub fn passages_de_reference(colonnes: usize, rangees: usize, graine: u64) -> Vec<Passage> {
    fn visiter(
        (c, r): (usize, usize),
        colonnes: usize,
        rangees: usize,
        alea: &mut Alea,
        visitee: &mut [bool],
        passages: &mut Vec<Passage>,
    ) {
        loop {
            let mut voisines = Vec::new();
            for (dc, dr) in [(0i64, -1i64), (1, 0), (0, 1), (-1, 0)] {
                let (nc, nr) = (c as i64 + dc, r as i64 + dr);
                if nc >= 0 && nr >= 0 && (nc as usize) < colonnes && (nr as usize) < rangees {
                    let (nc, nr) = (nc as usize, nr as usize);
                    if !visitee[nr * colonnes + nc] {
                        voisines.push((nc, nr));
                    }
                }
            }
            if voisines.is_empty() {
                return;
            }
            let i = (alea.suivant() % voisines.len() as u64) as usize;
            let (nc, nr) = voisines[i];
            passages.push((c, r, nc, nr));
            visitee[nr * colonnes + nc] = true;
            visiter((nc, nr), colonnes, rangees, alea, visitee, passages);
        }
    }
    let mut alea = Alea::new(graine);
    let mut visitee = vec![false; colonnes * rangees];
    visitee[0] = true;
    let mut passages = Vec::new();
    visiter((0, 0), colonnes, rangees, &mut alea, &mut visitee, &mut passages);
    passages
}

/// Le rendu d'un labyrinthe, **case par case** : chaque case `(x, y)` de la boîte est classée par ses restes `(x mod p,
/// y mod p)`, `p = couloir + 1` :
///
/// - hors de `[0, colonnes p] × [0, rangees p]` : bloquée ;
/// - les deux restes nuls : un pilier de mur, bloqué ;
/// - reste de `x` nul seulement : un morceau de mur vertical `x = k p`, libre si `1 ≤ k < colonnes` et si le passage entre
///   les chambres `(k − 1, y / p)` et `(k, y / p)` est ouvert ;
/// - reste de `y` nul seulement : un morceau de mur horizontal, de même ;
/// - aucun reste nul : une case de chambre, libre.
pub fn rendre_le_labyrinthe(
    largeur: usize,
    hauteur: usize,
    colonnes: usize,
    rangees: usize,
    couloir: usize,
    passages: &[Passage],
) -> Vec<bool> {
    let p = couloir + 1;
    let ouvert = |a: (usize, usize), b: (usize, usize)| {
        passages.iter().any(|&(c1, r1, c2, r2)| ((c1, r1) == a && (c2, r2) == b) || ((c1, r1) == b && (c2, r2) == a))
    };
    let mut libres = Vec::with_capacity(largeur * hauteur);
    for y in 0..hauteur {
        for x in 0..largeur {
            let libre = if x > colonnes * p || y > rangees * p {
                false
            } else {
                match (x % p, y % p) {
                    (0, 0) => false,
                    (0, _) => {
                        let (k, r) = (x / p, y / p);
                        k >= 1 && k < colonnes && ouvert((k - 1, r), (k, r))
                    }
                    (_, 0) => {
                        let (c, k) = (x / p, y / p);
                        k >= 1 && k < rangees && ouvert((c, k - 1), (c, k))
                    }
                    _ => true,
                }
            };
            libres.push(libre);
        }
    }
    libres
}

// ---------------------------------------------------------------------------------------------------------------------
// Les obstacles
// ---------------------------------------------------------------------------------------------------------------------

/// Pour chaque case de la boîte, le nombre de rectangles qui la couvrent. Panique si un rectangle est à l'envers ou sort de
/// la boîte.
pub fn couverture(largeur: usize, hauteur: usize, obstacles: &[Obstacle]) -> Vec<usize> {
    let mut couvertes = vec![0usize; largeur * hauteur];
    for o in obstacles {
        assert!(o.x0 <= o.x1 && o.y0 <= o.y1, "rectangle à l'envers : {o:?}");
        assert!(o.x1 < largeur && o.y1 < hauteur, "rectangle hors de la boîte {largeur} × {hauteur} : {o:?}");
        for y in o.y0..=o.y1 {
            for x in o.x0..=o.x1 {
                couvertes[y * largeur + x] += 1;
            }
        }
    }
    couvertes
}

/// La décomposition de la spécification, écrite autrement : une table des rectangles ouverts, clé `(xa, xb)`, valeur `y0` ;
/// à chaque ligne, les suites maximales de cases bloquées ; une suite dont la clé est dans la table la prolonge ; les clés de
/// la table que la ligne n'a pas retrouvées sont fermées. Rend les rectangles triés par `(y0, x0)`.
pub fn decomposition_de_reference(largeur: usize, hauteur: usize, libres: &[bool]) -> Vec<Obstacle> {
    let mut ouverts: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut fermes = Vec::new();
    for y in 0..hauteur {
        // Les suites maximales de cases bloquées de la ligne.
        let mut suites = Vec::new();
        let mut debut: Option<usize> = None;
        for x in 0..=largeur {
            let bloquee = x < largeur && !libres[y * largeur + x];
            match (bloquee, debut) {
                (true, None) => debut = Some(x),
                (false, Some(xa)) => {
                    suites.push((xa, x - 1));
                    debut = None;
                }
                _ => {}
            }
        }
        let mut suivants = BTreeMap::new();
        for cle in suites {
            let y0 = ouverts.remove(&cle).unwrap_or(y);
            suivants.insert(cle, y0);
        }
        for ((xa, xb), y0) in std::mem::take(&mut ouverts) {
            fermes.push(Obstacle { x0: xa, y0, x1: xb, y1: y - 1 });
        }
        ouverts = suivants;
    }
    for ((xa, xb), y0) in ouverts {
        fermes.push(Obstacle { x0: xa, y0, x1: xb, y1: hauteur - 1 });
    }
    fermes.sort_by_key(|o| (o.y0, o.x0));
    fermes
}

/// Un obstacle.
pub fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

// ---------------------------------------------------------------------------------------------------------------------
// Les chemins de référence
// ---------------------------------------------------------------------------------------------------------------------

/// Vrai si la case `(x, y)` est dans la grille et libre.
pub fn est_libre(largeur: usize, hauteur: usize, libres: &[bool], (x, y): (usize, usize)) -> bool {
    x < largeur && y < hauteur && libres[y * largeur + x]
}

/// Le plus court chemin à quatre voisins, par **relaxations successives** : la distance du départ vaut 0, puis on baisse la
/// distance de chaque case libre par celle d'une voisine plus un, tant que quelque chose baisse. Aucune file.
pub fn plus_court_de_reference(
    largeur: usize,
    hauteur: usize,
    libres: &[bool],
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> Option<usize> {
    if !est_libre(largeur, hauteur, libres, depart) || !est_libre(largeur, hauteur, libres, arrivee) {
        return None;
    }
    const INFINI: usize = usize::MAX;
    let mut distance = vec![INFINI; largeur * hauteur];
    distance[depart.1 * largeur + depart.0] = 0;
    loop {
        let mut a_baisse = false;
        for y in 0..hauteur {
            for x in 0..largeur {
                if !libres[y * largeur + x] {
                    continue;
                }
                let voisines = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
                for (nx, ny) in voisines {
                    if nx < largeur && ny < hauteur {
                        let voisine = distance[ny * largeur + nx];
                        if voisine != INFINI && voisine + 1 < distance[y * largeur + x] {
                            distance[y * largeur + x] = voisine + 1;
                            a_baisse = true;
                        }
                    }
                }
            }
        }
        if !a_baisse {
            break;
        }
    }
    let d = distance[arrivee.1 * largeur + arrivee.0];
    (d != INFINI).then_some(d)
}

/// La longueur du premier chemin que trouve une recherche en profondeur **récursive** (voisines dans l'ordre haut, droite,
/// bas, gauche ; une case visitée l'est pour toujours). Pour de petites grilles seulement : la récursion est profonde.
pub fn profondeur_de_reference(
    largeur: usize,
    hauteur: usize,
    libres: &[bool],
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> Option<usize> {
    fn visiter(
        largeur: usize,
        hauteur: usize,
        libres: &[bool],
        visitee: &mut [bool],
        (x, y): (usize, usize),
        arrivee: (usize, usize),
        profondeur: usize,
    ) -> Option<usize> {
        if (x, y) == arrivee {
            return Some(profondeur);
        }
        visitee[y * largeur + x] = true;
        let voisines = [(x, y.wrapping_sub(1)), (x + 1, y), (x, y + 1), (x.wrapping_sub(1), y)];
        for (nx, ny) in voisines {
            if nx < largeur
                && ny < hauteur
                && libres[ny * largeur + nx]
                && !visitee[ny * largeur + nx]
                && let Some(longueur) = visiter(largeur, hauteur, libres, visitee, (nx, ny), arrivee, profondeur + 1)
            {
                return Some(longueur);
            }
        }
        None
    }
    if !est_libre(largeur, hauteur, libres, depart) || !est_libre(largeur, hauteur, libres, arrivee) {
        return None;
    }
    let mut visitee = vec![false; largeur * hauteur];
    visiter(largeur, hauteur, libres, &mut visitee, depart, arrivee, 0)
}

/// Les cases libres atteignables depuis `depart` (4 voisins), par un balayage répété (sans file ni pile).
pub fn atteignables(largeur: usize, hauteur: usize, libres: &[bool], depart: (usize, usize)) -> usize {
    let mut vues = vec![false; largeur * hauteur];
    vues[depart.1 * largeur + depart.0] = true;
    loop {
        let mut a_change = false;
        for y in 0..hauteur {
            for x in 0..largeur {
                if !libres[y * largeur + x] || vues[y * largeur + x] {
                    continue;
                }
                let voisines = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
                if voisines.iter().any(|&(nx, ny)| nx < largeur && ny < hauteur && vues[ny * largeur + nx]) {
                    vues[y * largeur + x] = true;
                    a_change = true;
                }
            }
        }
        if !a_change {
            break;
        }
    }
    vues.iter().filter(|&&v| v).count()
}

/// Une grille au hasard : chaque case est bloquée avec la probabilité `num / den`.
pub fn grille_au_hasard(hasard: &mut Hasard, largeur: usize, hauteur: usize, num: usize, den: usize) -> Vec<bool> {
    (0..largeur * hauteur).map(|_| !hasard.avec_proba(num, den)).collect()
}

// ---------------------------------------------------------------------------------------------------------------------
// Les scénarios de moules
// ---------------------------------------------------------------------------------------------------------------------

/// Un des cinq scénarios de `scenarios/moules/` : son fichier, la forme que sa ligne `moule` doit décrire, sa boîte, la taille
/// de sa cavité (comptée ici d'après la forme), sa spore et sa durée, écrites ici d'après les fichiers.
pub struct ScenarioMoule {
    pub nom: &'static str,
    pub forme: Forme,
    pub boite: (usize, usize),
    pub cases_libres: usize,
    pub spore: (usize, usize),
    pub duree: u32,
}

/// Les cinq scénarios de moules.
pub fn scenarios_de_moule() -> Vec<ScenarioMoule> {
    vec![
        ScenarioMoule {
            nom: "brique",
            forme: Forme::Brique { x0: 30, y0: 35, x1: 69, y1: 64 },
            boite: (100, 100),
            cases_libres: 40 * 30,
            spore: (50, 50),
            duree: 250,
        },
        ScenarioMoule {
            nom: "l",
            forme: Forme::L { x0: 20, y0: 20, x1: 79, y1: 79, epaisseur: 15 },
            boite: (100, 100),
            cases_libres: 60 * 60 - 45 * 45,
            spore: (25, 70),
            duree: 250,
        },
        ScenarioMoule {
            nom: "disque",
            forme: Forme::Disque { cx: 50, cy: 50, rayon: 25 },
            boite: (100, 100),
            cases_libres: gauss(25),
            spore: (50, 50),
            duree: 250,
        },
        ScenarioMoule {
            nom: "anneau",
            forme: Forme::Anneau { cx: 50, cy: 50, rayon_exterieur: 35, rayon_interieur: 22 },
            boite: (100, 100),
            cases_libres: gauss(35) - gauss(22),
            spore: (50, 20),
            duree: 250,
        },
        ScenarioMoule {
            nom: "labyrinthe",
            forme: Forme::Labyrinthe { colonnes: 6, rangees: 6, couloir: 3, graine: 5 },
            boite: (25, 25),
            cases_libres: 6 * 6 * 9 + 35 * 3,
            spore: (2, 2),
            duree: 300,
        },
    ]
}

/// Le texte d'un fichier de `scenarios/moules/` (`nom` sans `.txt`).
pub fn texte_du_scenario(nom: &str) -> String {
    let chemin = format!("{}/scenarios/moules/{}.txt", env!("CARGO_MANIFEST_DIR"), nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{chemin} se lit : {erreur}"))
}

/// Le texte d'un fichier de `scenarios/` (de la vitrine).
pub fn texte_du_fichier(fichier: &str) -> String {
    let chemin = format!("{}/scenarios/{}", env!("CARGO_MANIFEST_DIR"), fichier);
    std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{chemin} se lit : {erreur}"))
}

/// Une simulation construite depuis un texte de scénario **sans ligne `moule`** (ou déjà développé), sans passer par le code
/// testé : rend aussi les réglages lus et la durée du texte.
pub fn construire(texte: &str, graine: u64) -> (Simulation, Reglages, u32) {
    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(texte).expect("le scénario est valide");
    let simulation = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu, tropismes, graine)
        .expect("les réglages sont valides");
    (simulation, reglages, duree)
}

/// Le texte d'un scénario de moule dont la ligne `moule` est remplacée par les lignes `obstacle` de la forme, calculées
/// **ici** (cases libres de référence, décomposition de référence), sans `developper` : le texte attendu après le
/// développement.
pub fn developpe_de_reference(texte: &str, forme: &Forme) -> String {
    let (largeur, hauteur) = boite_du_texte(texte);
    let libres = libres_de_reference(largeur, hauteur, forme);
    let mut obstacles = String::new();
    for o in decomposition_de_reference(largeur, hauteur, &libres) {
        obstacles.push_str(&format!("obstacle {} {} {} {}\n", o.x0, o.y0, o.x1, o.y1));
    }
    let mut rendu = String::new();
    for ligne in texte.split_inclusive('\n') {
        if ligne.trim_start().starts_with("moule ") {
            rendu.push_str(&obstacles);
        } else {
            rendu.push_str(ligne);
        }
    }
    rendu
}

/// La boîte `(largeur, hauteur)` de la ligne `boite` d'un texte.
pub fn boite_du_texte(texte: &str) -> (usize, usize) {
    for ligne in texte.lines() {
        let mots: Vec<&str> = ligne.split_whitespace().collect();
        if let ["boite", l, h] = mots.as_slice() {
            return (l.parse().unwrap(), h.parse().unwrap());
        }
    }
    panic!("le texte n'a pas de ligne boite");
}

/// Les cases bloquées d'une simulation, ligne par ligne.
pub fn cases_bloquees(simulation: &Simulation) -> Vec<bool> {
    let substrat = simulation.substrat();
    let mut bloquees = Vec::new();
    for y in 0..substrat.hauteur() {
        for x in 0..substrat.largeur() {
            bloquees.push(substrat.est_bloquee(x, y));
        }
    }
    bloquees
}

/// Les cases libres d'une simulation (le contraire des cases bloquées), ligne par ligne.
pub fn cases_libres_de(simulation: &Simulation) -> Vec<bool> {
    cases_bloquees(simulation).into_iter().map(|bloquee| !bloquee).collect()
}

/// Une cible de `temps_de_sortie` : le rectangle de cases `(x0, y0, x1, y1)`, bornes incluses.
pub type Cible = (usize, usize, usize, usize);

/// La cible d'une seule case.
pub fn case(x: usize, y: usize) -> Cible {
    (x, y, x, y)
}

/// Vrai si la case `(x, y)` est dans le rectangle `cible`.
pub fn dans_la_cible(cible: Cible, x: usize, y: usize) -> bool {
    let (x0, y0, x1, y1) = cible;
    x0 <= x && x <= x1 && y0 <= y && y <= y1
}

/// Le rectangle des cases de la chambre `(colonne, rangee)` d'un labyrinthe de couloir `couloir`, écrit d'après la
/// spécification : `x ∈ [c p + 1, c p + couloir]`, `y ∈ [r p + 1, r p + couloir]`, `p = couloir + 1`.
pub fn chambre_de_reference(couloir: usize, colonne: usize, rangee: usize) -> Cible {
    let p = couloir + 1;
    (colonne * p + 1, rangee * p + 1, colonne * p + couloir, rangee * p + couloir)
}

/// Le plus petit `temps` d'un segment de `simulation` posé au plus tard au pas `pas_max` et dont la fin est dans la `cible`
/// (règle du plancher), en regardant **tous** les segments.
pub fn premier_segment_dans(simulation: &Simulation, cible: Cible, pas_max: u32) -> Option<u32> {
    simulation
        .segments()
        .iter()
        .filter(|s| s.temps <= pas_max && dans_la_cible(cible, s.x1.floor() as usize, s.y1.floor() as usize))
        .map(|s| s.temps)
        .min()
}

/// Le temps de sortie recalculé à la main, d'après la règle exacte de la spécification : (1) la spore dans la cible :
/// `Some(0)` ; (2) le plus petit temps, au plus `pas_max`, des segments déjà posés qui finissent dans la cible ; (3) avancer
/// d'un pas et regarder tous les segments à nouveau, tant que `temps() < pas_max` ; (4) `None`. Une cible qui sort de la
/// boîte rend `None` sans avancer.
pub fn sortie_de_reference(simulation: &mut Simulation, cible: Cible, pas_max: u32) -> Option<u32> {
    let (largeur, hauteur) = (simulation.reglages().largeur, simulation.reglages().hauteur);
    if cible.2 >= largeur || cible.3 >= hauteur {
        return None;
    }
    let spore = simulation.reglages().spore;
    if dans_la_cible(cible, spore.0, spore.1) {
        return Some(0);
    }
    if let Some(m) = premier_segment_dans(simulation, cible, pas_max) {
        return Some(m);
    }
    while simulation.temps() < pas_max {
        simulation.avancer();
        if let Some(m) = premier_segment_dans(simulation, cible, pas_max) {
            return Some(m);
        }
    }
    None
}

/// Le carré de la distance de `(sx + 1/2, sy + 1/2)`, le centre de la case de la spore, au point le plus proche du
/// rectangle `[x0, x1 + 1] × [y0, y1 + 1]` de la cible.
pub fn carre_de_la_distance_minimale(spore: (usize, usize), cible: Cible) -> f64 {
    let (sx, sy) = (spore.0 as f64 + 0.5, spore.1 as f64 + 0.5);
    let (x0, y0, x1, y1) = (cible.0 as f64, cible.1 as f64, cible.2 as f64 + 1.0, cible.3 as f64 + 1.0);
    let dx = (x0 - sx).max(0.0).max(sx - x1);
    let dy = (y0 - sy).max(0.0).max(sy - y1);
    dx * dx + dy * dy
}

/// La somme de `valeurs` dans l'ordre, un par un.
pub fn somme_dans_l_ordre(valeurs: &[f64]) -> f64 {
    let mut somme = 0.0;
    for &v in valeurs {
        somme += v;
    }
    somme
}

/// Vrai si `a` et `b` sont égaux à `relative` près, relativement au plus grand des deux (avec un plancher `1e-300`).
pub fn proche_relatif(a: f64, b: f64, relative: f64) -> bool {
    (a - b).abs() <= relative * a.abs().max(b.abs()).max(1e-300)
}

/// Les six empreintes de référence de la vitrine (V0, terrain, chimiotropisme avec et sans, mur avec et sans) : scénario
/// (fichier), graine, pas, empreinte, écrites d'après la spécification.
pub const REFERENCES: [(&str, u64, u32, &str); 6] = [
    ("terrain.txt", 1, 80, "9f4f39a25d39be7edf523cb6b7646fb6fe87d4fbd00a2fe117fc61b23f49ddc0"),
    ("mur.txt", 31, 120, "ddeb843cd259e73cf51dd52809c983545eee667bc07a28c3b16ff8572435bc71"),
    ("mur-sans.txt", 31, 120, "7f9e2eb264ecb4845df45f2cf2382fbfb8fd2acd830dc8c11223ca175889ef93"),
    ("couplage.txt", 1, 80, "2223a1f3bcbc8627866aba084052cac06e93b05842bacc0ba94778bc23ed1083"),
    ("chimiotropisme.txt", 7, 100, "5d9680f934d2cdbfe54d845a4d33d5d952ee3fc643eba19f7caf506d4d623ed7"),
    ("chimiotropisme-sans.txt", 7, 100, "7d3a681284a24a913fd4582e3f93720dec463d5ede3b8caa19ff172d2651cea1"),
];

// ---------------------------------------------------------------------------------------------------------------------
// Lire un PNG
// ---------------------------------------------------------------------------------------------------------------------

/// Une image relue : largeur, hauteur et pixels RGB, ligne par ligne.
pub struct Png {
    pub largeur: usize,
    pub hauteur: usize,
    pub pixels: Vec<u8>,
}

impl Png {
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.largeur + x) * 3;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]]
    }
}

/// Relit un PNG écrit par `ecrire_png` : la signature, les blocs et leurs sommes de contrôle, l'en-tête (8 bits, RGB), le
/// flux zlib en blocs stockés, le filtre 0 de chaque ligne.
pub fn relire(octets: &[u8]) -> Png {
    use mycelium_lab::image::crc32;
    assert_eq!(&octets[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A], "la signature PNG");
    let (mut i, mut idat) = (8usize, Vec::new());
    let (mut largeur, mut hauteur) = (0usize, 0usize);
    loop {
        let longueur = u32::from_be_bytes(octets[i..i + 4].try_into().unwrap()) as usize;
        let type_ = &octets[i + 4..i + 8];
        let donnees = &octets[i + 8..i + 8 + longueur];
        let somme = u32::from_be_bytes(octets[i + 8 + longueur..i + 12 + longueur].try_into().unwrap());
        assert_eq!(crc32(&octets[i + 4..i + 8 + longueur]), somme, "la somme de contrôle du bloc {type_:?}");
        match type_ {
            b"IHDR" => {
                largeur = u32::from_be_bytes(donnees[0..4].try_into().unwrap()) as usize;
                hauteur = u32::from_be_bytes(donnees[4..8].try_into().unwrap()) as usize;
                assert_eq!(&donnees[8..13], &[8, 2, 0, 0, 0], "8 bits, RGB, sans entrelacement");
            }
            b"IDAT" => idat.extend_from_slice(donnees),
            b"IEND" => break,
            _ => {}
        }
        i += 12 + longueur;
    }
    let mut brut = Vec::new();
    let mut j = 2;
    loop {
        let drapeau = idat[j];
        assert_eq!(drapeau & 0b110, 0, "le flux zlib est en blocs stockés");
        let n = u16::from_le_bytes([idat[j + 1], idat[j + 2]]) as usize;
        j += 5;
        brut.extend_from_slice(&idat[j..j + n]);
        j += n;
        if drapeau & 1 == 1 {
            break;
        }
    }
    let ligne = 1 + 3 * largeur;
    assert_eq!(brut.len(), ligne * hauteur, "les octets bruts : une ligne de filtre plus des pixels, par ligne");
    let mut pixels = Vec::new();
    for rang in brut.chunks(ligne) {
        assert_eq!(rang[0], 0, "filtre 0");
        pixels.extend_from_slice(&rang[1..]);
    }
    Png { largeur, hauteur, pixels }
}

pub const GRIS_BLOQUE: [u8; 3] = [48, 48, 48];
pub const FOND: [u8; 3] = [16, 16, 24];
const SOMBRE: [i32; 3] = [70, 20, 110];
const CLAIR: [i32; 3] = [255, 240, 90];

/// La couleur d'une case occupée de densité `d` (`d > 0`), d'après la rampe de la spécification de M0 : `niveau =
/// (sqrt(min(d / dmax, 1)) × 255) as i32`, puis `sombre + (clair − sombre) × niveau / 255` en entiers, canal par canal.
pub fn couleur_attendue(d: f64, dmax: f64) -> [u8; 3] {
    let niveau = ((d / dmax).min(1.0).sqrt() * 255.0) as i32;
    [0, 1, 2].map(|c| (SOMBRE[c] + (CLAIR[c] - SOMBRE[c]) * niveau / 255) as u8)
}

/// Une simulation d'un scénario de `scenarios/moules/`, après `developper` : rend aussi les réglages lus et la durée.
pub fn construire_moule(nom: &str, graine: u64) -> (Simulation, Reglages, u32) {
    let developpe = mycelium_lab::moule::developper(&texte_du_scenario(nom)).expect("le scénario se développe");
    construire(&developpe, graine)
}

/// Un résumé d'une simulation, pour les messages.
pub fn resume(simulation: &Simulation) -> String {
    format!("temps {}, {} segments, {} pointes", simulation.temps(), simulation.segments().len(), simulation.pointes())
}
