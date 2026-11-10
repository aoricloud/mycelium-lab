//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « les chemins » (T3 : recherche en profondeur et A\*).
//!
//! Ils fixent `plus_court_chemin`, `chemin_a_etoile` et `chemin_en_profondeur` sur des grilles de cases libres (`.` libre,
//! `#` bloquée ; le haut du dessin est la ligne 0) :
//!
//! - **des grilles écrites à la main** : une case seule, un mur, un détour, départ égal à l'arrivée, une arrivée inatteignable,
//!   un départ ou une arrivée bloqués ou **hors de la grille** (dont un `x` égal à la largeur, qui retomberait sur la ligne
//!   suivante si le contrôle ne portait pas sur chaque coordonnée), une diagonale qui ne se traverse pas (quatre voisins, pas
//!   huit), un détour que la distance de Manhattan sous-estime ;
//! - **l'ordre de la profondeur** (haut, droite, bas, gauche) : sur une grille 3 × 3 libre, le premier chemin trouvé de `(0, 0)` à
//!   `(0, 2)` fait 8 pas, et de `(2, 2)` à `(0, 0)` aussi (dérivés à la main, plus longs que les 2 pas du plus court chemin) ;
//! - **A\* égal au plus court chemin** sur des milliers de grilles au hasard, contre un calcul à part par relaxations
//!   successives (`outils.rs`) ; la profondeur est au moins le plus court chemin, et égale à une profondeur **récursive** écrite
//!   à part ; elle est égale au plus court chemin dans un arbre ;
//! - **l'itératif ne déborde pas** : une grille de 200 × 200 en serpentin (un chemin de 20 098 pas), jouée dans un fil
//!   d'exécution à la pile de 128 Ko, que déborderait une récursion de cette profondeur ; et la même grille entièrement libre ;
//! - **les paniques** : une tranche de mauvaise longueur.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_10/outils.rs"]
mod outils;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::moule::{chemin_a_etoile, chemin_en_profondeur, plus_court_chemin};
use outils::{Hasard, delai, grille, grille_au_hasard, plus_court_de_reference, profondeur_de_reference};

/// Les trois réponses d'une grille écrite à la main : `(plus court, A*, profondeur)`.
fn trois(
    texte: &str,
    depart: (usize, usize),
    arrivee: (usize, usize),
) -> (Option<usize>, Option<usize>, Option<usize>) {
    let (l, h, g) = grille(texte);
    (
        plus_court_chemin(l, h, &g, depart, arrivee),
        chemin_a_etoile(l, h, &g, depart, arrivee),
        chemin_en_profondeur(l, h, &g, depart, arrivee),
    )
}

/// Vérifie que les trois fonctions rendent toutes `attendu` sur la grille.
fn tous(texte: &str, depart: (usize, usize), arrivee: (usize, usize), attendu: Option<usize>) {
    assert_eq!(trois(texte, depart, arrivee), (attendu, attendu, attendu), "{depart:?} → {arrivee:?} dans\n{texte}");
}

fn panique<F: FnOnce()>(f: F) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

// ---------------------------------------------------------------------------------------------------------------------
// À la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_case_seule_et_depart_egal_a_l_arrivee() {
    delai();
    tous(".", (0, 0), (0, 0), Some(0));
    tous("#", (0, 0), (0, 0), None);
    tous("...\n...\n...", (1, 1), (1, 1), Some(0));
    tous("...\n.#.\n...", (2, 2), (2, 2), Some(0));
    tous("...\n.#.\n...", (1, 1), (1, 1), None);
    // Une case libre au milieu de murs : atteinte seulement depuis elle-même.
    tous("###\n#.#\n###", (1, 1), (1, 1), Some(0));
    tous("###\n#.#\n#..", (1, 1), (2, 2), Some(2));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_chemin_droit_un_detour_et_un_mur() {
    delai();
    // Une ligne.
    tous(".....", (0, 0), (4, 0), Some(4));
    tous(".....", (4, 0), (0, 0), Some(4));
    tous(".\n.\n.\n.", (0, 3), (0, 0), Some(3));
    // Libre : la distance de Manhattan.
    tous("....\n....\n....", (0, 0), (3, 2), Some(5));
    // À l'envers, le plus court chemin est le même ; la profondeur, elle, fait un détour de 9 pas (haut, haut, gauche, bas, bas,
    // gauche, haut, haut, gauche).
    assert_eq!(trois("....\n....\n....", (3, 2), (0, 0)), (Some(5), Some(5), Some(9)));
    // Un détour : les trois cases du milieu de la ligne 1 sont bloquées.
    let detour = ".....\n.###.\n.....";
    tous(detour, (0, 1), (4, 1), Some(6));
    tous(detour, (4, 1), (0, 1), Some(6));
    tous(detour, (0, 0), (4, 2), Some(6));
    // Un mur de toute la hauteur : inatteignable.
    let mur = "..#..\n..#..\n..#..";
    tous(mur, (0, 0), (4, 0), None);
    tous(mur, (4, 2), (0, 2), None);
    tous(mur, (0, 0), (1, 2), Some(3));
    tous(mur, (3, 0), (4, 2), Some(3));
    // Un mur avec une porte en bas.
    tous("..#..\n..#..\n.....", (0, 0), (4, 0), Some(8));
    // Une spirale : 12 cases de couloir.
    let spirale = ".....\n####.\n.....\n.####\n.....";
    tous(spirale, (0, 0), (4, 4), Some(16));
    tous(spirale, (4, 4), (0, 0), Some(16));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_depart_ou_une_arrivee_bloque_ou_hors_de_la_grille_n_a_pas_de_chemin() {
    delai();
    let g = "...\n.#.\n...";
    // Bloqué.
    tous(g, (1, 1), (0, 0), None);
    tous(g, (0, 0), (1, 1), None);
    // Hors de la grille : `x = largeur` retomberait sur la case (0, y + 1) si le contrôle ne portait que sur l'indice.
    tous(g, (3, 0), (0, 0), None);
    tous(g, (0, 0), (3, 0), None);
    tous(g, (3, 0), (0, 1), None);
    tous(g, (2, 2), (3, 1), None);
    tous(g, (0, 3), (0, 0), None);
    tous(g, (0, 0), (0, 3), None);
    tous(g, (3, 3), (3, 3), None);
    tous(g, (100, 100), (0, 0), None);
    tous(g, (usize::MAX, 0), (0, 0), None);
    tous(g, (0, 0), (0, usize::MAX), None);
    tous(g, (usize::MAX, usize::MAX), (usize::MAX, usize::MAX), None);
    // Une boîte large et plate : `y` hors de la grille alors que `y * largeur + x` y tomberait.
    tous("....\n....", (0, 2), (0, 0), None);
    tous("..\n..\n..", (2, 0), (0, 0), None);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn quatre_voisins_et_non_huit_la_diagonale_ne_se_traverse_pas() {
    delai();
    // Deux cases en diagonale : les quatre voisins ne les relient pas.
    tous(".#\n#.", (0, 0), (1, 1), None);
    tous("#.\n.#", (1, 0), (0, 1), None);
    // Libre : (0, 0) → (1, 1) fait 2 pas par les côtés, pas 1 par la diagonale.
    tous("..\n..", (0, 0), (1, 1), Some(2));
    tous("...\n...\n...", (0, 0), (2, 2), Some(4));
    // Un coin à contourner : la diagonale serait libre, le chemin par les côtés passe par la case du coin.
    tous("..#\n#..\n##.", (0, 0), (2, 2), Some(4));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_profondeur_essaie_haut_droite_bas_gauche_et_garde_le_premier_chemin_trouve() {
    delai();
    let libre = "...\n...\n...";
    // (0,0) → (0,2) : (0,0), (1,0), (2,0), (2,1), (2,2), (1,2), (1,1), (0,1), (0,2) : 8 pas, le plus court en fait 2.
    let (court, etoile, profondeur) = trois(libre, (0, 0), (0, 2));
    assert_eq!((court, etoile, profondeur), (Some(2), Some(2), Some(8)));
    // (2,2) → (0,0) : (2,2), (2,1), (2,0), (1,0), (1,1), (1,2), (0,2), (0,1), (0,0) : 8 pas, le plus court en fait 4.
    let (court, etoile, profondeur) = trois(libre, (2, 2), (0, 0));
    assert_eq!((court, etoile, profondeur), (Some(4), Some(4), Some(8)));
    // Dans l'ordre haut, droite, bas, gauche, (0,0) → (1,0) est trouvé tout de suite : 1 pas.
    assert_eq!(trois(libre, (0, 0), (1, 0)), (Some(1), Some(1), Some(1)));
    // (1,1) → (1,0) : « haut » est essayé le premier, 1 pas.
    assert_eq!(trois(libre, (1, 1), (1, 0)).2, Some(1));
    // (1,1) → (1,2) : haut (1,0), droite (2,0), bas (2,1), bas (2,2), gauche (1,2) : 5 pas (le plus court chemin en fait 1).
    assert_eq!(trois(libre, (1, 1), (1, 2)), (Some(1), Some(1), Some(5)));
    // Un cul-de-sac essayé en premier : la profondeur y entre, en ressort, et ne compte que le chemin de la pile finale. De
    // (1,1) à (2,1) : haut (1,0) → droite (2,0) → bas (2,1) : 3 pas (le chemin direct en fait 1).
    assert_eq!(trois(libre, (1, 1), (2, 1)), (Some(1), Some(1), Some(3)));
    // Un chemin en U : la longueur de la pile finale, pas le nombre de cases vues.
    let impasse = ".#.\n.#.\n...";
    assert_eq!(trois(impasse, (0, 0), (2, 0)), (Some(6), Some(6), Some(6)));
    let impasse2 = "...\n.#.\n.#.";
    // De (2,2) à (0,2) : haut (2,1), haut (2,0), gauche (1,0), gauche (0,0), bas (0,1), bas (0,2) : 6 pas.
    assert_eq!(trois(impasse2, (2, 2), (0, 2)), (Some(6), Some(6), Some(6)));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_distance_de_manhattan_sous_estime_un_long_detour_et_a_etoile_reste_exact() {
    delai();
    // Le chemin direct est barré par un long mur : Manhattan vaut 4, le vrai chemin 18.
    let g = ".#.......\n.#.#####.\n.#.#...#.\n.#.#.#.#.\n.#.#.#.#.\n.#...#.#.\n.#####.#.\n.......#.";
    let (l, h, cases) = grille(g);
    assert_eq!(plus_court_chemin(l, h, &cases, (0, 0), (2, 0)), plus_court_de_reference(l, h, &cases, (0, 0), (2, 0)));
    assert_eq!(chemin_a_etoile(l, h, &cases, (0, 0), (2, 0)), plus_court_de_reference(l, h, &cases, (0, 0), (2, 0)));
    assert!(
        plus_court_chemin(l, h, &cases, (0, 0), (2, 0)).unwrap() > 4,
        "Manhattan vaut 2, le vrai chemin est bien plus long"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// Au hasard
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn sur_des_grilles_au_hasard_a_etoile_est_le_plus_court_chemin_et_la_profondeur_est_plus_longue() {
    delai();
    let mut hasard = Hasard::new(4242);
    let (mut joignables, mut plus_longues, mut impossibles) = (0, 0, 0);
    for _ in 0..4000 {
        let (largeur, hauteur) = (hasard.entre(1, 12), hasard.entre(1, 12));
        let densite = hasard.entre(0, 6);
        let cases = grille_au_hasard(&mut hasard, largeur, hauteur, densite, 10);
        let depart = (hasard.jusqu_a(largeur + 1), hasard.jusqu_a(hauteur + 1));
        let arrivee = (hasard.jusqu_a(largeur + 1), hasard.jusqu_a(hauteur + 1));
        let attendu = plus_court_de_reference(largeur, hauteur, &cases, depart, arrivee);
        let court = plus_court_chemin(largeur, hauteur, &cases, depart, arrivee);
        let etoile = chemin_a_etoile(largeur, hauteur, &cases, depart, arrivee);
        let profondeur = chemin_en_profondeur(largeur, hauteur, &cases, depart, arrivee);
        let contexte =
            format!("{depart:?} → {arrivee:?} dans {largeur} × {hauteur} :\n{}", outils::dessin(largeur, &cases));
        assert_eq!(court, attendu, "plus court chemin, {contexte}");
        assert_eq!(etoile, attendu, "A*, {contexte}");
        assert_eq!(
            profondeur.is_some(),
            attendu.is_some(),
            "la profondeur trouve un chemin ssi il en existe un, {contexte}"
        );
        assert_eq!(
            profondeur,
            profondeur_de_reference(largeur, hauteur, &cases, depart, arrivee),
            "la profondeur, {contexte}"
        );
        match (attendu, profondeur) {
            (Some(a), Some(p)) => {
                joignables += 1;
                assert!(p >= a, "la profondeur ({p}) est au moins le plus court chemin ({a}), {contexte}");
                if p > a {
                    plus_longues += 1;
                }
            }
            _ => impossibles += 1,
        }
    }
    assert!(joignables >= 600, "garde : des chemins existent : {joignables}");
    assert!(impossibles >= 300, "garde : des cas sans chemin : {impossibles}");
    assert!(plus_longues >= 100, "garde : la profondeur est parfois plus longue : {plus_longues} sur {joignables}");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn sur_une_grille_plus_grande_a_etoile_est_encore_le_plus_court_chemin() {
    delai();
    let mut hasard = Hasard::new(8);
    for _ in 0..40 {
        let (largeur, hauteur) = (hasard.entre(30, 45), hasard.entre(30, 45));
        let cases = grille_au_hasard(&mut hasard, largeur, hauteur, 3, 10);
        for _ in 0..5 {
            let depart = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
            let arrivee = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
            let attendu = plus_court_de_reference(largeur, hauteur, &cases, depart, arrivee);
            assert_eq!(plus_court_chemin(largeur, hauteur, &cases, depart, arrivee), attendu);
            assert_eq!(chemin_a_etoile(largeur, hauteur, &cases, depart, arrivee), attendu);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Un arbre, une grande grille
// ---------------------------------------------------------------------------------------------------------------------

/// Une grille de `n × n` en serpentin : les lignes paires sont libres sur toute la largeur, les lignes impaires ne le sont
/// qu'à une extrémité (à droite après la ligne 0, à gauche après la ligne 2, etc.). Le chemin de `(0, 0)` à `(0, n − 2)` est
/// unique et passe par toutes les cases libres : `(n / 2) (n − 1) + (n / 2 − 1) 2` pas pour `n` pair.
fn serpentin(n: usize) -> Vec<bool> {
    let mut cases = vec![false; n * n];
    for y in 0..n {
        for x in 0..n {
            cases[y * n + x] = if y % 2 == 0 {
                true
            } else {
                let k = (y - 1) / 2;
                (k % 2 == 0 && x == n - 1) || (k % 2 == 1 && x == 0)
            };
        }
    }
    // La dernière ligne (impaire) n'a rien à relier : elle reste bloquée.
    for x in 0..n {
        cases[(n - 1) * n + x] = false;
    }
    cases
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_profondeur_est_iterative_et_ne_deborde_pas_sur_200_par_200() {
    delai();
    let n = 200;
    let cases = serpentin(n);
    let attendu = (n / 2) * (n - 1) + (n / 2 - 1) * 2;
    assert_eq!(attendu, 20_098);
    assert_eq!(plus_court_chemin(n, n, &cases, (0, 0), (0, n - 2)), Some(attendu));
    assert_eq!(chemin_a_etoile(n, n, &cases, (0, 0), (0, n - 2)), Some(attendu));
    // Un fil à la pile de 128 Ko : une récursion de 20 000 niveaux la déborderait (le processus s'arrêterait en échec).
    let resultat = std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(move || {
            let serpent = chemin_en_profondeur(n, n, &serpentin(n), (0, 0), (0, n - 2));
            // La grille libre de 200 × 200 : la profondeur longe la ligne 0 jusqu'au bout, puis descend la dernière colonne.
            let libre = chemin_en_profondeur(n, n, &vec![true; n * n], (0, 0), (n - 1, n - 1));
            (serpent, libre)
        })
        .expect("le fil se lance")
        .join()
        .expect("la profondeur d'abord ne déborde pas la pile");
    assert_eq!(resultat.0, Some(attendu));
    assert_eq!(resultat.1, Some(2 * (n - 1)), "la grille libre : 199 pas à droite, puis 199 pas vers le bas");
    // Et sur un labyrinthe de 10 000 cases environ (100 × 100) : tous les chemins égaux, A* compris.
    let (largeur, hauteur) = (101, 101);
    let mut hasard = Hasard::new(5);
    let arbre = outils::libres_de_reference(
        largeur,
        hauteur,
        &mycelium_lab::moule::Forme::Labyrinthe { colonnes: 50, rangees: 50, couloir: 1, graine: hasard.suivant() },
    );
    let fin = (99, 99);
    let court = plus_court_chemin(largeur, hauteur, &arbre, (1, 1), fin);
    assert!(court.is_some());
    assert_eq!(chemin_a_etoile(largeur, hauteur, &arbre, (1, 1), fin), court);
    assert_eq!(chemin_en_profondeur(largeur, hauteur, &arbre, (1, 1), fin), court, "un arbre : un seul chemin simple");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn dans_un_arbre_la_profondeur_egale_le_plus_court_chemin() {
    delai();
    let mut hasard = Hasard::new(77);
    for _ in 0..30 {
        let (colonnes, rangees) = (hasard.entre(2, 9), hasard.entre(2, 9));
        let (largeur, hauteur) = (2 * colonnes + 1, 2 * rangees + 1);
        let arbre = outils::libres_de_reference(
            largeur,
            hauteur,
            &mycelium_lab::moule::Forme::Labyrinthe { colonnes, rangees, couloir: 1, graine: hasard.suivant() },
        );
        for _ in 0..8 {
            let depart = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
            let arrivee = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
            let court = plus_court_chemin(largeur, hauteur, &arbre, depart, arrivee);
            assert_eq!(
                chemin_en_profondeur(largeur, hauteur, &arbre, depart, arrivee),
                court,
                "{depart:?} → {arrivee:?}"
            );
            assert_eq!(chemin_a_etoile(largeur, hauteur, &arbre, depart, arrivee), court);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_grille_sans_case_n_a_pas_de_chemin_et_ne_panique_pas() {
    delai();
    for (largeur, hauteur) in [(0usize, 0usize), (0, 5), (5, 0)] {
        let vide: Vec<bool> = Vec::new();
        for (depart, arrivee) in [((0, 0), (0, 0)), ((1, 2), (3, 4)), ((0, 0), (usize::MAX, usize::MAX))] {
            assert_eq!(plus_court_chemin(largeur, hauteur, &vide, depart, arrivee), None, "{largeur} × {hauteur}");
            assert_eq!(chemin_a_etoile(largeur, hauteur, &vide, depart, arrivee), None, "{largeur} × {hauteur}");
            assert_eq!(chemin_en_profondeur(largeur, hauteur, &vide, depart, arrivee), None, "{largeur} × {hauteur}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_tranche_de_mauvaise_longueur_fait_paniquer() {
    delai();
    for (largeur, hauteur, longueur) in
        [(3usize, 3usize, 8usize), (3, 3, 10), (3, 3, 0), (2, 5, 9), (1, 1, 0), (4, 1, 5)]
    {
        let cases = vec![true; longueur];
        assert!(panique(|| _ = plus_court_chemin(largeur, hauteur, &cases, (0, 0), (0, 0))), "plus court, {longueur}");
        assert!(panique(|| _ = chemin_a_etoile(largeur, hauteur, &cases, (0, 0), (0, 0))), "A*, {longueur}");
        assert!(
            panique(|| _ = chemin_en_profondeur(largeur, hauteur, &cases, (0, 0), (0, 0))),
            "profondeur, {longueur}"
        );
    }
}
