//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « le labyrinthe » (T3, repliée dans M1).
//!
//! Ils fixent `Forme::Labyrinthe` (par `libres`) et `centre_de_la_chambre` :
//!
//! - **deux labyrinthes dessinés à la main** d'après les quatre premières sorties de `Alea::new(0)` (SplitMix64, valeurs
//!   connues) : un 2 × 3 et un 3 × 2, de couloir 1. Le second ne se dessine bien que si le tirage a lieu **même quand une
//!   seule voisine est libre** (`n = 1`) : sans lui, le tirage suivant est décalé et le labyrinthe change ;
//! - **le labyrinthe de référence** de `outils.rs` (une recherche en profondeur récursive et un rendu case par case, écrits
//!   autrement que le code testé), pour des centaines de tailles, de couloirs (1, 2, 3, 4) et de graines : les mêmes cases, dans
//!   des boîtes de la taille juste et dans de plus grandes et non carrées (les cases au-delà de `W` et de `H` sont bloquées) ;
//! - **le déterminisme** (la même graine, les mêmes cases ; des graines différentes, des labyrinthes différents) ;
//! - **un labyrinthe parfait** : le nombre de cases libres `colonnes rangees couloir² + (colonnes rangees − 1) couloir` ; les
//!   piliers `(x mod p, y mod p) = (0, 0)` toujours bloqués ; chaque passage entièrement ouvert ou entièrement fermé ;
//!   `colonnes rangees − 1` passages ; toutes les cases libres reliées ; l'arbre des chambres sans cycle ; **un seul chemin simple** entre deux chambres pour un couloir de 1 case : la
//!   recherche en profondeur, le plus court chemin et A\* donnent la même longueur pour toutes les paires de centres de chambres ;
//! - **les largeurs de couloir 1, 2, 3 et 4**, les labyrinthes `1 × 1`, `1 × n` et `n × 1`, le labyrinthe du scénario
//!   (6 × 6, couloir 3, graine 5 : 429 cases libres et un plus court chemin de 52 pas) ;
//! - **`centre_de_la_chambre`** : la formule `(colonne p + 1 + couloir / 2, rangee p + 1 + couloir / 2)`, la case est libre et
//!   dans sa chambre.
//!
//! Les contrats (`FormeInvalide`, `HorsDeLaBoite`) sont dans `etape_10_formes.rs`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_10/outils.rs"]
mod outils;

use mycelium_lab::alea::Alea;
use mycelium_lab::moule::{
    Forme, centre_de_la_chambre, chemin_a_etoile, chemin_en_profondeur, libres, plus_court_chemin,
};
use outils::{
    Hasard, atteignables, compter, delai, dessin, libres_de_reference, passages_de_reference, plus_court_de_reference,
    profondeur_de_reference, rendre_le_labyrinthe,
};

fn labyrinthe(colonnes: usize, rangees: usize, couloir: usize, graine: u64) -> Forme {
    Forme::Labyrinthe { colonnes, rangees, couloir, graine }
}

/// Les dimensions `(W, H)` du labyrinthe : `colonnes p + 1` et `rangees p + 1`, avec `p = couloir + 1`.
fn dimensions(colonnes: usize, rangees: usize, couloir: usize) -> (usize, usize) {
    (colonnes * (couloir + 1) + 1, rangees * (couloir + 1) + 1)
}

/// Le labyrinthe dans une boîte de la taille juste.
fn juste(colonnes: usize, rangees: usize, couloir: usize, graine: u64) -> (usize, usize, Vec<bool>) {
    let (w, h) = dimensions(colonnes, rangees, couloir);
    let cases =
        libres(w, h, &labyrinthe(colonnes, rangees, couloir, graine)).expect("le labyrinthe tient dans sa boîte");
    assert_eq!(cases.len(), w * h);
    (w, h, cases)
}

// ---------------------------------------------------------------------------------------------------------------------
// À la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn deux_labyrinthes_derives_a_la_main_des_sorties_de_alea_0() {
    delai();
    // Garde : les premières sorties de SplitMix64 pour la graine 0 (valeurs de référence connues). Leur parité décide des tirages
    // `% 2` : impair, pair, impair, pair.
    let mut alea = Alea::new(0);
    let sorties: Vec<u64> = (0..5).map(|_| alea.suivant()).collect();
    assert_eq!(
        sorties[..4],
        [0xE220_A839_7B1D_CDAF, 0x6E78_9E6A_A1B9_65F4, 0x06C4_5D18_8009_454F, 0xF88B_B8A8_724C_81EC],
        "les sorties de Alea::new(0) : {sorties:x?}"
    );

    // 2 colonnes × 3 rangées, couloir 1, graine 0 (boîte 5 × 7). Depuis (0,0) : voisines [droite, bas], tirage impair → bas
    // (0,1). Depuis (0,1) : [droite, bas], tirage pair → droite (1,1). Depuis (1,1) : [haut, bas], tirage impair → bas (1,2).
    // Depuis (1,2) : [gauche] seule, tirage fait → (0,2). Puis on dépile jusqu'à (1,1), qui a encore (1,0) : passage ouvert.
    // Passages : (0,0)-(0,1), (0,1)-(1,1), (1,1)-(1,2), (1,2)-(0,2), (1,1)-(1,0).
    assert_eq!(
        dessin(5, &libres(5, 7, &labyrinthe(2, 3, 1, 0)).unwrap()),
        "#####\n#.#.#\n#.#.#\n#...#\n###.#\n#...#\n#####"
    );
    assert_eq!(
        passages_de_reference(2, 3, 0),
        vec![(0, 0, 0, 1), (0, 1, 1, 1), (1, 1, 1, 2), (1, 2, 0, 2), (1, 1, 1, 0)],
        "le labyrinthe de référence est celui de la dérivation à la main"
    );

    // 3 colonnes × 2 rangées, couloir 1, graine 0 (boîte 7 × 5). Depuis (0,0) : [droite, bas], impair → bas (0,1). Depuis (0,1) :
    // [droite] seule, **tirage fait** (pair, sans effet) → (1,1). Depuis (1,1) : [haut, droite], tirage suivant impair → droite
    // (2,1). Si le tirage de la voisine unique était sauté, ce choix prendrait la sortie paire et irait en haut, en (1,0).
    // Depuis (2,1) : [haut] seule → (2,0). Depuis (2,0) : [gauche] seule → (1,0).
    // Passages : (0,0)-(0,1), (0,1)-(1,1), (1,1)-(2,1), (2,1)-(2,0), (2,0)-(1,0).
    assert_eq!(
        dessin(7, &libres(7, 5, &labyrinthe(3, 2, 1, 0)).unwrap()),
        "#######\n#.#...#\n#.###.#\n#.....#\n#######"
    );
    assert_eq!(
        passages_de_reference(3, 2, 0),
        vec![(0, 0, 0, 1), (0, 1, 1, 1), (1, 1, 2, 1), (2, 1, 2, 0), (2, 0, 1, 0)]
    );

    // Un labyrinthe de 2 × 2 : la parité du premier tirage décide (impair : bas puis un U ; pair : droite puis un S).
    assert_eq!(dessin(5, &libres(5, 5, &labyrinthe(2, 2, 1, 0)).unwrap()), "#####\n#.#.#\n#.#.#\n#...#\n#####");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_plus_petits_labyrinthes() {
    delai();
    // 1 × 1 : une chambre au milieu de son mur, pour les couloirs 1, 2 et 3 (aucune voisine : aucun tirage).
    assert_eq!(dessin(3, &libres(3, 3, &labyrinthe(1, 1, 1, 0)).unwrap()), "###\n#.#\n###");
    assert_eq!(dessin(4, &libres(4, 4, &labyrinthe(1, 1, 2, 9)).unwrap()), "####\n#..#\n#..#\n####");
    assert_eq!(dessin(5, &libres(5, 5, &labyrinthe(1, 1, 3, 123)).unwrap()), "#####\n#...#\n#...#\n#...#\n#####");
    // Deux chambres côte à côte : le seul passage possible.
    assert_eq!(dessin(5, &libres(5, 3, &labyrinthe(2, 1, 1, 77)).unwrap()), "#####\n#...#\n#####");
    assert_eq!(dessin(7, &libres(7, 4, &labyrinthe(2, 1, 2, 5)).unwrap()), "#######\n#.....#\n#.....#\n#######");
    // Deux chambres l'une sur l'autre.
    assert_eq!(dessin(3, &libres(3, 5, &labyrinthe(1, 2, 1, 4)).unwrap()), "###\n#.#\n#.#\n#.#\n###");
    assert_eq!(dessin(4, &libres(4, 7, &labyrinthe(1, 2, 2, 4)).unwrap()), "####\n#..#\n#..#\n#..#\n#..#\n#..#\n####");
    // Une rangée de 4 chambres : un seul labyrinthe parfait possible, quelle que soit la graine (couloir 1).
    for graine in 0..5 {
        assert_eq!(dessin(9, &libres(9, 3, &labyrinthe(4, 1, 1, graine)).unwrap()), "#########\n#.......#\n#########");
    }
    // Une colonne de 4 chambres.
    for graine in 0..5 {
        assert_eq!(
            dessin(3, &libres(3, 9, &labyrinthe(1, 4, 1, graine)).unwrap()),
            "###\n#.#\n#.#\n#.#\n#.#\n#.#\n#.#\n#.#\n###"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le labyrinthe de référence
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_labyrinthe_est_celui_de_la_reference_pour_toute_taille_couloir_et_graine() {
    delai();
    let mut hasard = Hasard::new(1234);
    for couloir in 1..=4 {
        for _ in 0..60 {
            let (colonnes, rangees) = (hasard.entre(1, 9), hasard.entre(1, 9));
            let graine = hasard.suivant();
            let (w, h) = dimensions(colonnes, rangees, couloir);
            // Dans la boîte juste, et dans une plus grande, non carrée : le labyrinthe est en haut à gauche.
            let (largeur, hauteur) = (w + hasard.jusqu_a(5), h + hasard.jusqu_a(5));
            let forme = labyrinthe(colonnes, rangees, couloir, graine);
            let passages = passages_de_reference(colonnes, rangees, graine);
            let attendu = rendre_le_labyrinthe(largeur, hauteur, colonnes, rangees, couloir, &passages);
            let obtenu = libres(largeur, hauteur, &forme).unwrap();
            assert_eq!(
                obtenu,
                attendu,
                "{colonnes} × {rangees}, couloir {couloir}, graine {graine}, boîte {largeur} × {hauteur}\n{}",
                dessin(largeur, &obtenu)
            );
            assert_eq!(obtenu, libres_de_reference(largeur, hauteur, &forme));
        }
    }
    // Des graines qui se suivent, et des graines extrêmes.
    for graine in (0..40).chain([u64::MAX, u64::MAX - 1, 1 << 63, 1 << 32]) {
        let (w, h, cases) = juste(7, 5, 2, graine);
        let attendu = rendre_le_labyrinthe(w, h, 7, 5, 2, &passages_de_reference(7, 5, graine));
        assert_eq!(cases, attendu, "graine {graine}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_labyrinthe_est_deterministe_et_depend_de_la_graine() {
    delai();
    let (_, _, a) = juste(6, 6, 3, 5);
    let (_, _, b) = juste(6, 6, 3, 5);
    assert_eq!(a, b, "la même graine, les mêmes cases");
    // Deux graines données donnent deux labyrinthes différents.
    for (g1, g2) in [(1u64, 2u64), (5, 6), (0, 1), (100, 101), (7, 7_000_000)] {
        assert_ne!(juste(6, 6, 3, g1).2, juste(6, 6, 3, g2).2, "graines {g1} et {g2}");
        assert_ne!(juste(4, 4, 1, g1).2, juste(4, 4, 1, g2).2, "graines {g1} et {g2}, couloir 1");
    }
    // Sur 60 graines consécutives d'un 5 × 5, presque tous les labyrinthes sont différents.
    let mut vus: Vec<Vec<bool>> = (0..60).map(|g| juste(5, 5, 1, g).2).collect();
    vus.sort();
    vus.dedup();
    assert!(vus.len() >= 55, "{} labyrinthes différents sur 60 graines", vus.len());
    // La boîte plus grande ne change pas le labyrinthe : le haut à gauche est le même.
    let (w, h, juste_cases) = juste(6, 6, 3, 5);
    let grande = libres(w + 7, h + 3, &labyrinthe(6, 6, 3, 5)).unwrap();
    for y in 0..h + 3 {
        for x in 0..w + 7 {
            let attendu = x < w && y < h && juste_cases[y * w + x];
            assert_eq!(grande[y * (w + 7) + x], attendu, "case ({x}, {y})");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Un labyrinthe parfait
// ---------------------------------------------------------------------------------------------------------------------

/// Les contrôles de structure d'un labyrinthe dans sa boîte juste. Rend le nombre de passages ouverts.
fn verifier_un_labyrinthe_parfait(colonnes: usize, rangees: usize, couloir: usize, graine: u64) -> usize {
    let pas = couloir + 1;
    let (w, h, cases) = juste(colonnes, rangees, couloir, graine);
    let contexte = format!("{colonnes} × {rangees}, couloir {couloir}, graine {graine}");
    // Le nombre de cases libres.
    assert_eq!(
        compter(&cases),
        colonnes * rangees * couloir * couloir + (colonnes * rangees - 1) * couloir,
        "cases libres, {contexte}"
    );
    let libre = |x: usize, y: usize| cases[y * w + x];
    // Les chambres sont libres, les piliers bloqués, le contour bloqué.
    for r in 0..rangees {
        for c in 0..colonnes {
            for dy in 1..=couloir {
                for dx in 1..=couloir {
                    assert!(libre(c * pas + dx, r * pas + dy), "la chambre ({c}, {r}) est libre, {contexte}");
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            if x % pas == 0 && y % pas == 0 {
                assert!(!libre(x, y), "le pilier ({x}, {y}) est bloqué, {contexte}");
            }
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                assert!(!libre(x, y), "le contour ({x}, {y}) est bloqué, {contexte}");
            }
        }
    }
    // Chaque passage est entièrement ouvert ou entièrement fermé : les `couloir` cases du côté de la chambre.
    let mut passages = 0;
    let mut reliees = vec![vec![]; colonnes * rangees];
    for r in 0..rangees {
        for k in 1..colonnes {
            // Le mur vertical x = k p, entre (k − 1, r) et (k, r).
            let ouvertes = (1..=couloir).filter(|&dy| libre(k * pas, r * pas + dy)).count();
            assert!(ouvertes == 0 || ouvertes == couloir, "passage partiel, {contexte}");
            if ouvertes == couloir {
                passages += 1;
                reliees[r * colonnes + k - 1].push(r * colonnes + k);
                reliees[r * colonnes + k].push(r * colonnes + k - 1);
            }
        }
    }
    for c in 0..colonnes {
        for k in 1..rangees {
            let ouvertes = (1..=couloir).filter(|&dx| libre(c * pas + dx, k * pas)).count();
            assert!(ouvertes == 0 || ouvertes == couloir, "passage partiel, {contexte}");
            if ouvertes == couloir {
                passages += 1;
                reliees[(k - 1) * colonnes + c].push(k * colonnes + c);
                reliees[k * colonnes + c].push((k - 1) * colonnes + c);
            }
        }
    }
    // Un arbre couvrant : `colonnes rangees − 1` passages, et toutes les chambres reliées entre elles.
    assert_eq!(passages, colonnes * rangees - 1, "passages, {contexte}");
    let mut vues = vec![false; colonnes * rangees];
    let mut a_voir = vec![0usize];
    vues[0] = true;
    while let Some(i) = a_voir.pop() {
        for &j in &reliees[i] {
            if !vues[j] {
                vues[j] = true;
                a_voir.push(j);
            }
        }
    }
    assert!(vues.iter().all(|&v| v), "toutes les chambres sont reliées, {contexte}");
    // Toutes les cases libres sont atteignables depuis le centre de la chambre (0, 0).
    let depart = centre_de_la_chambre(couloir, 0, 0);
    assert_eq!(atteignables(w, h, &cases, depart), compter(&cases), "cases atteignables, {contexte}");
    passages
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_labyrinthe_est_parfait_pour_toute_taille_et_tout_couloir() {
    delai();
    let mut hasard = Hasard::new(99);
    for couloir in 1..=4 {
        for colonnes in 1..=5 {
            for rangees in 1..=5 {
                for _ in 0..2 {
                    verifier_un_labyrinthe_parfait(colonnes, rangees, couloir, hasard.suivant());
                }
            }
        }
    }
    // Des labyrinthes plus grands et plus plats.
    for (colonnes, rangees) in [(12, 3), (3, 12), (1, 15), (15, 1), (10, 10)] {
        verifier_un_labyrinthe_parfait(colonnes, rangees, 2, hasard.suivant());
    }
    // Le labyrinthe du scénario.
    assert_eq!(verifier_un_labyrinthe_parfait(6, 6, 3, 5), 35);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_seul_chemin_simple_entre_deux_chambres_quelconques() {
    delai();
    for (colonnes, rangees, couloir, graine) in [
        (4usize, 4usize, 1usize, 3u64),
        (4, 3, 2, 8),
        (3, 5, 3, 17),
        (5, 2, 1, 21),
        (1, 6, 2, 4),
        (6, 1, 3, 4),
        (3, 3, 4, 30),
    ] {
        let (w, h, cases) = juste(colonnes, rangees, couloir, graine);
        let centres: Vec<(usize, usize)> =
            (0..rangees).flat_map(|r| (0..colonnes).map(move |c| centre_de_la_chambre(couloir, c, r))).collect();
        for &depart in &centres {
            for &arrivee in &centres {
                let court = plus_court_chemin(w, h, &cases, depart, arrivee);
                let profondeur = chemin_en_profondeur(w, h, &cases, depart, arrivee);
                let etoile = chemin_a_etoile(w, h, &cases, depart, arrivee);
                assert!(court.is_some(), "les chambres sont reliées : {depart:?} → {arrivee:?}");
                assert_eq!(etoile, court, "A* égale le plus court chemin : {depart:?} → {arrivee:?}");
                assert_eq!(
                    profondeur,
                    profondeur_de_reference(w, h, &cases, depart, arrivee),
                    "la profondeur égale la profondeur récursive de outils.rs : {depart:?} → {arrivee:?}"
                );
                if couloir == 1 {
                    // Couloir de 1 case : les cases libres forment un arbre, un seul chemin simple existe.
                    assert_eq!(
                        profondeur, court,
                        "la profondeur égale le plus court chemin : {depart:?} → {arrivee:?}"
                    );
                } else {
                    // Couloir large : une chambre est un carré de `couloir²` cases, où il y a bien des chemins simples (la
                    // profondeur d'abord peut y tourner en rond) ; seul l'arbre des chambres est sans cycle.
                    assert!(
                        profondeur >= court,
                        "la profondeur est au moins le plus court chemin : {depart:?} → {arrivee:?}"
                    );
                }
            }
        }
        // Un contrôle par un calcul à part, sur quelques paires.
        let (premier, dernier) = (centres[0], centres[centres.len() - 1]);
        assert_eq!(
            plus_court_chemin(w, h, &cases, premier, dernier),
            plus_court_de_reference(w, h, &cases, premier, dernier)
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_labyrinthe_du_scenario_a_429_cases_et_un_chemin_de_52_pas() {
    delai();
    let (w, h, cases) = juste(6, 6, 3, 5);
    assert_eq!((w, h), (25, 25));
    assert_eq!(compter(&cases), 36 * 9 + 35 * 3);
    assert_eq!(compter(&cases), 429);
    let entree = centre_de_la_chambre(3, 0, 0);
    let sortie = centre_de_la_chambre(3, 5, 5);
    assert_eq!((entree, sortie), ((2, 2), (22, 22)));
    assert_eq!(plus_court_chemin(w, h, &cases, entree, sortie), Some(52));
    assert_eq!(chemin_a_etoile(w, h, &cases, entree, sortie), Some(52));
    // La profondeur d'abord est plus longue (92 pas) : à couloir 3 une chambre est un carré 3 × 3, où elle tourne en rond avant
    // de trouver la porte. Le « un seul chemin simple » ne vaut que pour l'arbre des chambres, ou pour un couloir de 1 case.
    let profondeur = chemin_en_profondeur(w, h, &cases, entree, sortie);
    assert_eq!(profondeur, profondeur_de_reference(w, h, &cases, entree, sortie));
    assert!(profondeur.unwrap() >= 52);
    assert_eq!(plus_court_de_reference(w, h, &cases, entree, sortie), Some(52));
}

// ---------------------------------------------------------------------------------------------------------------------
// centre_de_la_chambre
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_centre_d_une_chambre_suit_la_formule() {
    delai();
    // couloir 1 (p = 2) : colonne 2 c + 1. couloir 2 (p = 3) : 3 c + 2. couloir 3 (p = 4) : 4 c + 2. couloir 4 (p = 5) : 5 c + 3.
    for (couloir, pas, decalage) in [(1usize, 2usize, 1usize), (2, 3, 2), (3, 4, 2), (4, 5, 3), (5, 6, 3), (6, 7, 4)] {
        for c in 0..6 {
            for r in 0..6 {
                assert_eq!(
                    centre_de_la_chambre(couloir, c, r),
                    (c * pas + decalage, r * pas + decalage),
                    "couloir {couloir}, chambre ({c}, {r})"
                );
            }
        }
    }
    // La colonne et la rangée ne s'échangent pas.
    assert_eq!(centre_de_la_chambre(3, 1, 0), (6, 2));
    assert_eq!(centre_de_la_chambre(3, 0, 1), (2, 6));
    assert_eq!(centre_de_la_chambre(3, 5, 5), (22, 22));
    assert_eq!(centre_de_la_chambre(1, 0, 0), (1, 1));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_centre_est_une_case_libre_de_sa_chambre() {
    delai();
    for couloir in 1..=5 {
        let pas = couloir + 1;
        let (colonnes, rangees) = (4, 3);
        let (w, _, cases) = juste(colonnes, rangees, couloir, 11);
        for r in 0..rangees {
            for c in 0..colonnes {
                let (x, y) = centre_de_la_chambre(couloir, c, r);
                assert!(cases[y * w + x], "le centre ({x}, {y}) de la chambre ({c}, {r}) est libre, couloir {couloir}");
                assert!((c * pas + 1..=c * pas + couloir).contains(&x), "x dans la chambre, couloir {couloir}");
                assert!((r * pas + 1..=r * pas + couloir).contains(&y), "y dans la chambre, couloir {couloir}");
                // Le centre : autant de cases de chaque côté, à une case près (la division entière).
                let (a_gauche, a_droite) = (x - (c * pas + 1), (c * pas + couloir) - x);
                assert!(a_gauche == a_droite || a_gauche == a_droite + 1, "centré en x, couloir {couloir}");
            }
        }
    }
}
