//! Outils partagés par les tests d'acceptation de l'étape 9 (M0, mesurer la pièce que la colonie remplit).
//!
//! Ce fichier n'est pas un test : chaque fichier `etape_9_*.rs` l'inclut par `#[path = "etape_9/outils.rs"] mod outils;`.
//! Tout y est écrit **hors du code testé** (rien n'est repris de `piece.rs`, sauf l'énumération `Bord`) :
//!
//! - un petit générateur pseudo-aléatoire propre aux tests (`Hasard`), qui n'est pas l'`Alea` du dépôt ;
//! - trois **oracles de la carte de densité**, par des méthodes qui ne coupent pas les segments aux lignes de grille :
//!   - `densite_exacte` : des coordonnées en huitièmes de case, et, case par case, l'intervalle exact du paramètre `t`
//!     (des fractions d'entiers de 128 bits) pendant lequel le segment est dans la case (méthode des dalles) ;
//!   - `densite_par_dalles` : la même méthode en `f64`, pour des segments quelconques (les segments de la simulation) ;
//!   - `densite_par_echantillons` : N points très fins sur le segment, chacun compté pour `L / N` dans sa case ;
//! - deux **calculs indépendants sur les grilles de cases** : `morceaux_de_reference` (une union de classes, sans
//!   parcours) et `traversee_de_reference` (des relaxations successives, sans file) ;
//! - la lecture des scénarios sur le disque, les six scénarios de la vitrine et quelques scénarios à nous (boîtes qui ne
//!   sont pas carrées : un échange de la largeur et de la hauteur ne se voit pas sur une boîte carrée) ;
//! - l'arme contre les boucles sans fin : `delai()`.
//!
//! Il n'utilise, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `sqrt`, `floor`, `min` et `max`.
#![allow(dead_code)]

use std::cmp::Ordering;

use mycelium_lab::croissance::Segment;
use mycelium_lab::piece::{Bord, Carte, carte_de_densite};
use mycelium_lab::simulation::{Reglages, Simulation, lire_scenario_tropismes};

/// Le délai maximal d'un fichier de tests, en secondes, compté depuis son premier test : au-delà, le processus s'arrête en
/// échec (une boucle sans fin dans une coupe de segment ou dans un parcours de grille doit faire échouer, jamais pendre).
pub const DELAI_MAXIMAL: u64 = 240;

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

    /// Un `f64` de `0` (inclus) à `1` (exclu).
    pub fn unite(&mut self) -> f64 {
        (self.suivant() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Vrai avec la probabilité `num / den`.
    pub fn avec_proba(&mut self, num: usize, den: usize) -> bool {
        self.jusqu_a(den) < num
    }
}

/// Un segment de l'apex 0, au pas 1.
pub fn seg(x0: f64, y0: f64, x1: f64, y1: f64) -> Segment {
    Segment { apex: 0, temps: 1, x0, y0, x1, y1 }
}

/// La longueur d'un segment, `sqrt(dx² + dy²)`, écrite ici.
pub fn longueur(s: &Segment) -> f64 {
    let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
    (dx * dx + dy * dy).sqrt()
}

/// La somme des longueurs, dans l'ordre des segments.
pub fn somme_des_longueurs(segments: &[Segment]) -> f64 {
    let mut somme = 0.0;
    for s in segments {
        somme += longueur(s);
    }
    somme
}

/// La somme de `valeurs` dans l'ordre, un par un (c'est l'ordre que la spécification fixe pour `Carte::somme`).
pub fn somme_dans_l_ordre(valeurs: &[f64]) -> f64 {
    let mut somme = 0.0;
    for &v in valeurs {
        somme += v;
    }
    somme
}

/// La somme de `valeurs` à l'envers (pour montrer qu'un test d'ordre peut échouer).
pub fn somme_a_l_envers(valeurs: &[f64]) -> f64 {
    let mut somme = 0.0;
    for &v in valeurs.iter().rev() {
        somme += v;
    }
    somme
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en absolu.
pub fn proche(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// Vrai si `a` et `b` sont égaux à `relative` près, relativement au plus grand des deux (avec un plancher `1e-300`).
pub fn proche_relatif(a: f64, b: f64, relative: f64) -> bool {
    (a - b).abs() <= relative * a.abs().max(b.abs()).max(1e-300)
}

// ---------------------------------------------------------------------------------------------------------------------
// L'oracle exact : des huitièmes de case, des fractions.
// ---------------------------------------------------------------------------------------------------------------------

/// Une fraction `n / d`, `d > 0`, en entiers de 128 bits.
#[derive(Clone, Copy, Debug)]
struct Frac {
    n: i128,
    d: i128,
}

impl Frac {
    fn entier(n: i128) -> Frac {
        Frac { n, d: 1 }
    }

    /// `n / d` avec un dénominateur positif.
    fn normalisee(n: i128, d: i128) -> Frac {
        if d < 0 { Frac { n: -n, d: -d } } else { Frac { n, d } }
    }

    fn comparer(self, autre: Frac) -> Ordering {
        (self.n * autre.d).cmp(&(autre.n * self.d))
    }

    fn plus_grande(self, autre: Frac) -> Frac {
        if self.comparer(autre) == Ordering::Less { autre } else { self }
    }

    fn plus_petite(self, autre: Frac) -> Frac {
        if self.comparer(autre) == Ordering::Greater { autre } else { self }
    }

    fn moins(self, autre: Frac) -> Frac {
        Frac { n: self.n * autre.d - autre.n * self.d, d: self.d * autre.d }
    }

    fn en_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

/// L'intervalle de `t ∈ [0, 1]` pendant lequel la coordonnée `c0 + t d` (en huitièmes) est dans la case `case` (de `8 case`
/// à `8 case + 8` huitièmes, semi-ouverte). `limite` est le nombre de cases de l'axe.
///
/// Si la coordonnée ne bouge pas (`d == 0`), c'est tout `[0, 1]` quand la case est celle du plancher, rien sinon ; une
/// coordonnée sur le bord haut de la boîte (`c0 == 8 limite`) est dans la dernière case.
fn intervalle_dans_la_case(c0: i128, d: i128, case: usize, limite: usize) -> Option<(Frac, Frac)> {
    if d == 0 {
        let plancher = ((c0 / 8) as usize).min(limite - 1);
        return (plancher == case).then(|| (Frac::entier(0), Frac::entier(1)));
    }
    let bas = Frac::normalisee(8 * case as i128 - c0, d);
    let haut = Frac::normalisee(8 * case as i128 + 8 - c0, d);
    Some(if bas.comparer(haut) == Ordering::Greater { (haut, bas) } else { (bas, haut) })
}

/// La carte de densité exacte (une valeur par case, ligne par ligne) d'un ensemble de segments dont les coordonnées sont
/// des **huitièmes de case** entiers `[x0, y0, x1, y1]`, dans `[0, 8 largeur] × [0, 8 hauteur]`.
///
/// Méthode des dalles : pour chaque case, l'intervalle de `t` où le segment est dans la bande des `x` de la case et dans
/// la bande des `y`, leur intersection avec `[0, 1]`, sa longueur exacte (une fraction), multipliée par `L`.
pub fn densite_exacte(largeur: usize, hauteur: usize, segments: &[[i64; 4]]) -> Vec<f64> {
    let mut carte = vec![0.0; largeur * hauteur];
    for s in segments {
        let [x0, y0, x1, y1] = s.map(i128::from);
        let (dx, dy) = (x1 - x0, y1 - y0);
        if dx == 0 && dy == 0 {
            continue;
        }
        let l = ((dx * dx + dy * dy) as f64).sqrt() / 8.0;
        for j in 0..hauteur {
            for i in 0..largeur {
                let (Some((ax, bx)), Some((ay, by))) =
                    (intervalle_dans_la_case(x0, dx, i, largeur), intervalle_dans_la_case(y0, dy, j, hauteur))
                else {
                    continue;
                };
                let debut = Frac::entier(0).plus_grande(ax).plus_grande(ay);
                let fin = Frac::entier(1).plus_petite(bx).plus_petite(by);
                if fin.comparer(debut) == Ordering::Greater {
                    carte[j * largeur + i] += fin.moins(debut).en_f64() * l;
                }
            }
        }
    }
    carte
}

/// Les mêmes segments, en `f64` (des huitièmes, donc exacts), pour les donner à `carte_de_densite`.
pub fn segments_en_huitiemes(segments: &[[i64; 4]]) -> Vec<Segment> {
    segments.iter().map(|s| seg(s[0] as f64 / 8.0, s[1] as f64 / 8.0, s[2] as f64 / 8.0, s[3] as f64 / 8.0)).collect()
}

// ---------------------------------------------------------------------------------------------------------------------
// Les oracles en f64.
// ---------------------------------------------------------------------------------------------------------------------

/// L'intervalle de `t` (en `f64`) où `c0 + t d` est dans la case `case` d'un axe de `limite` cases ; même règle que
/// `intervalle_dans_la_case`.
fn dalle_f64(c0: f64, d: f64, case: usize, limite: usize) -> Option<(f64, f64)> {
    if d == 0.0 {
        let plancher = (c0.floor() as usize).min(limite - 1);
        return (plancher == case).then_some((0.0, 1.0));
    }
    let a = (case as f64 - c0) / d;
    let b = (case as f64 + 1.0 - c0) / d;
    Some(if a < b { (a, b) } else { (b, a) })
}

/// La carte de densité par la méthode des dalles en `f64`, pour des segments quelconques : seules les cases de la boîte
/// englobante du segment sont regardées. Exacte à quelques ulps près.
pub fn densite_par_dalles(largeur: usize, hauteur: usize, segments: &[Segment]) -> Vec<f64> {
    let mut carte = vec![0.0; largeur * hauteur];
    for s in segments {
        let l = longueur(s);
        if l == 0.0 {
            continue;
        }
        let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
        let bas_x = (s.x0.min(s.x1).floor() as usize).min(largeur - 1);
        let haut_x = (s.x0.max(s.x1).floor() as usize).min(largeur - 1);
        let bas_y = (s.y0.min(s.y1).floor() as usize).min(hauteur - 1);
        let haut_y = (s.y0.max(s.y1).floor() as usize).min(hauteur - 1);
        let (i0, i1, j0, j1) = (bas_x, haut_x, bas_y, haut_y);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let (Some((ax, bx)), Some((ay, by))) =
                    (dalle_f64(s.x0, dx, i, largeur), dalle_f64(s.y0, dy, j, hauteur))
                else {
                    continue;
                };
                let debut = 0.0f64.max(ax).max(ay);
                let fin = 1.0f64.min(bx).min(by);
                if fin > debut {
                    carte[j * largeur + i] += (fin - debut) * l;
                }
            }
        }
    }
    carte
}

/// La carte de densité par échantillonnage : chaque segment est découpé en `n` parties égales, le milieu de chaque partie
/// compte pour `L / n` dans la case de son plancher. L'erreur sur une case est au plus de l'ordre de `2 L / n`.
pub fn densite_par_echantillons(largeur: usize, hauteur: usize, segments: &[Segment], n: usize) -> Vec<f64> {
    let mut carte = vec![0.0; largeur * hauteur];
    for s in segments {
        let l = longueur(s);
        for k in 0..n {
            let t = (k as f64 + 0.5) / n as f64;
            let x = s.x0 + t * (s.x1 - s.x0);
            let y = s.y0 + t * (s.y1 - s.y0);
            let i = (x.floor() as usize).min(largeur - 1);
            let j = (y.floor() as usize).min(hauteur - 1);
            carte[j * largeur + i] += l / n as f64;
        }
    }
    carte
}

// ---------------------------------------------------------------------------------------------------------------------
// Les grilles de cases.
// ---------------------------------------------------------------------------------------------------------------------

/// Lit une grille écrite à la main : une ligne de texte par ligne de cases, `#` pour une case occupée, `.` pour une case
/// vide. Rend `(largeur, hauteur, cases)`, `y` lent, `x` rapide.
pub fn grille(texte: &str) -> (usize, usize, Vec<bool>) {
    let lignes: Vec<&str> = texte.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let largeur = lignes[0].len();
    let mut cases = Vec::new();
    for ligne in &lignes {
        assert_eq!(ligne.len(), largeur, "toutes les lignes de la grille ont la même largeur : {ligne}");
        for c in ligne.chars() {
            assert!(c == '#' || c == '.', "caractère de grille inconnu : {c}");
            cases.push(c == '#');
        }
    }
    (largeur, lignes.len(), cases)
}

/// Les quatre bords, par leur indice : 0 gauche, 1 droite, 2 haut, 3 bas (les noms servent aux messages d'erreur).
pub const NOMS_BORDS: [&str; 4] = ["gauche", "droite", "haut", "bas"];

/// Le bord d'indice `i` (voir `NOMS_BORDS`).
pub fn bord(i: usize) -> Bord {
    match i {
        0 => Bord::Gauche,
        1 => Bord::Droite,
        2 => Bord::Haut,
        3 => Bord::Bas,
        _ => panic!("il n'y a que quatre bords"),
    }
}

/// Vrai si la case `(x, y)` est sur le bord d'indice `bord` : écrit ici, indépendamment du code testé. Le haut est la ligne
/// 0 (l'axe `y` descend avec l'indice de ligne), le bas la dernière.
pub fn est_sur_le_bord(bord: usize, x: usize, y: usize, largeur: usize, hauteur: usize) -> bool {
    match bord {
        0 => x == 0,
        1 => x + 1 == largeur,
        2 => y == 0,
        3 => y + 1 == hauteur,
        _ => panic!("il n'y a que quatre bords"),
    }
}

/// Les tailles des morceaux (huit voisins), de la plus grande à la plus petite, par **union de classes** : chaque case
/// occupée est réunie à ses voisines occupées déjà vues (gauche, haut-gauche, haut, haut-droite), puis on compte les cases
/// de chaque classe. Ni parcours, ni pile, ni récursion : la méthode n'a rien de commun avec un parcours de graphe, et
/// elle tient sur une boîte de 161 × 161.
pub fn morceaux_de_reference(largeur: usize, hauteur: usize, occupees: &[bool]) -> Vec<usize> {
    fn racine(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut parent: Vec<usize> = (0..largeur * hauteur).collect();
    for y in 0..hauteur {
        for x in 0..largeur {
            if !occupees[y * largeur + x] {
                continue;
            }
            // Les voisines déjà vues dans l'ordre de lecture : (x − 1, y), (x − 1, y − 1), (x, y − 1), (x + 1, y − 1).
            let mut deja_vues = Vec::new();
            if x > 0 {
                deja_vues.push((x - 1, y));
            }
            if y > 0 {
                if x > 0 {
                    deja_vues.push((x - 1, y - 1));
                }
                deja_vues.push((x, y - 1));
                if x + 1 < largeur {
                    deja_vues.push((x + 1, y - 1));
                }
            }
            for (vx, vy) in deja_vues {
                if occupees[vy * largeur + vx] {
                    let a = racine(&mut parent, y * largeur + x);
                    let b = racine(&mut parent, vy * largeur + vx);
                    if a != b {
                        parent[a.max(b)] = a.min(b);
                    }
                }
            }
        }
    }
    let mut compte = vec![0usize; largeur * hauteur];
    for (i, &occupee) in occupees.iter().enumerate() {
        if occupee {
            let r = racine(&mut parent, i);
            compte[r] += 1;
        }
    }
    let mut tailles: Vec<usize> = compte.into_iter().filter(|&c| c > 0).collect();
    tailles.sort_by(|a, b| b.cmp(a));
    tailles
}

/// Le plus grand `f64` strictement plus petit que `v` (`v` fini et strictement positif) : un seuil posé un ulp sous la
/// valeur d'une case laisse cette case au-dessus du seuil.
pub fn juste_sous(v: f64) -> f64 {
    assert!(v > 0.0 && v.is_finite());
    f64::from_bits(v.to_bits() - 1)
}

/// Le nombre de cases du plus court chemin occupé (huit voisins) du bord d'indice `de` au bord d'indice `vers`, par des **relaxations
/// successives** : la distance de chaque case occupée vaut 1 sur le bord `de`, puis on baisse chaque distance par celle d'une
/// voisine plus un, tant que quelque chose baisse. Aucune file, aucun parcours en largeur.
pub fn traversee_de_reference(
    largeur: usize,
    hauteur: usize,
    occupees: &[bool],
    de: usize,
    vers: usize,
) -> Option<usize> {
    const INFINI: usize = usize::MAX;
    let mut distance = vec![INFINI; largeur * hauteur];
    for y in 0..hauteur {
        for x in 0..largeur {
            if occupees[y * largeur + x] && est_sur_le_bord(de, x, y, largeur, hauteur) {
                distance[y * largeur + x] = 1;
            }
        }
    }
    loop {
        let mut a_baisse = false;
        for y in 0..hauteur {
            for x in 0..largeur {
                if !occupees[y * largeur + x] {
                    continue;
                }
                for ny in y.saturating_sub(1)..=(y + 1).min(hauteur - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(largeur - 1) {
                        let voisine = distance[ny * largeur + nx];
                        if (nx, ny) != (x, y) && voisine != INFINI && voisine + 1 < distance[y * largeur + x] {
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
    let mut meilleure = INFINI;
    for y in 0..hauteur {
        for x in 0..largeur {
            if occupees[y * largeur + x] && est_sur_le_bord(vers, x, y, largeur, hauteur) {
                meilleure = meilleure.min(distance[y * largeur + x]);
            }
        }
    }
    (meilleure != INFINI).then_some(meilleure)
}

// ---------------------------------------------------------------------------------------------------------------------
// Les scénarios.
// ---------------------------------------------------------------------------------------------------------------------

/// Un des six scénarios de la vitrine : son fichier, sa boîte (carrée) et sa durée, écrits ici d'après la spécification.
pub struct Scenario {
    pub nom: &'static str,
    pub fichier: &'static str,
    pub boite: usize,
    pub duree: u32,
}

/// Les six scénarios de la vitrine.
pub const SCENARIOS: [Scenario; 6] = [
    Scenario { nom: "libre", fichier: "couplage.txt", boite: 161, duree: 80 },
    Scenario { nom: "terrain", fichier: "terrain.txt", boite: 100, duree: 80 },
    Scenario { nom: "tache", fichier: "chimiotropisme.txt", boite: 100, duree: 100 },
    Scenario { nom: "tache-sans", fichier: "chimiotropisme-sans.txt", boite: 100, duree: 100 },
    Scenario { nom: "mur", fichier: "mur.txt", boite: 100, duree: 120 },
    Scenario { nom: "mur-sans", fichier: "mur-sans.txt", boite: 100, duree: 120 },
];

/// Le texte d'un fichier de `scenarios/`.
pub fn texte_du_fichier(fichier: &str) -> String {
    let chemin = format!("{}/scenarios/{}", env!("CARGO_MANIFEST_DIR"), fichier);
    std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{chemin} se lit : {erreur}"))
}

/// Les scénarios à nous, dont la boîte n'est pas carrée (un échange de la largeur et de la hauteur doit se voir) : le nom
/// et le texte. `lateral` : une boîte large avec un mur et une tache ; `haut` : une boîte haute traversée par un mur
/// horizontal à porte ; `petit` : une petite boîte large que la colonie remplit de part en part (des traversées existent) ;
/// `etroit` : une boîte étroite et haute, un couloir, que la colonie remplit ; `coin_bas_droit` et `coin_haut_gauche` : une
/// spore dans la dernière case, ou dans la première, d'une boîte non carrée (la case la plus dense est alors celle d'un coin).
pub const AUTRES_SCENARIOS: [(&str, &str); 6] = [
    (
        "lateral",
        "boite 70 40\nspore 10 20\nsubstrat 1.0 0.2\ncroissance 1.0 8.0 35 20\ncouplage 0.1 0.002\nanastomose 0.5 8\n\
         duree 100\nsource 55 12 5 6.0\nobstacle 30 5 33 30\nthigmotropisme 35\nchimiotropisme 10\n",
    ),
    (
        "haut",
        "boite 30 80\nspore 15 10\nsubstrat 1.0 0.2\ncroissance 1.0 12.0 35 20\ncouplage 0.1 0.002\nanastomose 0.5 8\n\
         duree 120\nobstacle 0 40 20 43\nthigmotropisme 35\n",
    ),
    (
        "petit",
        "boite 24 12\nspore 12 6\nsubstrat 1.0 0.2\ncroissance 1.0 5.0 40 25\ncouplage 0.1 0.002\nanastomose 0.5 6\n\
         duree 150\n",
    ),
    (
        "etroit",
        "boite 9 30\nspore 4 15\nsubstrat 1.0 0.2\ncroissance 1.0 5.0 40 25\ncouplage 0.1 0.002\nanastomose 0.5 6\n\
         duree 150\nobstacle 3 5 5 7\n",
    ),
    (
        "coin_bas_droit",
        "boite 14 9\nspore 13 8\nsubstrat 1.0 0.2\ncroissance 1.0 6.0 40 25\ncouplage 0.1 0.002\nanastomose 0.5 6\n\
         duree 60\n",
    ),
    (
        "coin_haut_gauche",
        "boite 9 14\nspore 0 0\nsubstrat 1.0 0.2\ncroissance 1.0 6.0 40 25\ncouplage 0.1 0.002\nanastomose 0.5 6\n\
         duree 60\n",
    ),
];

/// Une simulation construite depuis le texte d'un scénario, sans passer par le code testé : rend aussi les réglages lus et
/// la durée du texte.
pub fn construire(texte: &str, graine: u64) -> (Simulation, Reglages, u32) {
    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(texte).expect("le scénario est valide");
    let simulation = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu, tropismes, graine)
        .expect("les réglages sont valides");
    (simulation, reglages, duree)
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

/// Le nom du bord d'indice `i`, pour les messages d'erreur.
pub fn nom_du_bord(i: usize) -> &'static str {
    NOMS_BORDS[i]
}

/// Vérifie qu'une carte a la forme demandée : sa largeur, sa hauteur, et `largeur × hauteur` valeurs. Sans ce contrôle, une
/// carte vide passerait tous les contrôles qui parcourent « les cases de la carte ».
pub fn verifier_la_forme(carte: &Carte, largeur: usize, hauteur: usize) {
    assert_eq!((carte.largeur(), carte.hauteur()), (largeur, hauteur), "la forme de la carte");
    assert_eq!(carte.valeurs().len(), largeur * hauteur, "le nombre de valeurs de la carte");
}

/// `carte_de_densite`, suivie de la vérification de la forme de la carte rendue.
pub fn faire_la_carte(largeur: usize, hauteur: usize, segments: &[Segment]) -> Carte {
    let carte = carte_de_densite(largeur, hauteur, segments);
    verifier_la_forme(&carte, largeur, hauteur);
    carte
}
