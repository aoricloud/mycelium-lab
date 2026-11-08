//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie substrat : `Substrat::gradient`.
//!
//! Ils fixent : le gradient d'une case est fait de différences centrées, `gx = (droite - gauche) * 0.5` et
//! `gy = (bas - haut) * 0.5`, avec `y` qui croît avec l'indice de ligne ; sur un champ affine la pente est exacte à
//! l'intérieur, et vaut la moitié de la pente sur un bord ou un coin (le voisin absent est remplacé par la case
//! elle-même, comme dans `pas()`) ; un voisin bloqué est remplacé de la même façon ; un champ uniforme a exactement
//! `(0.0, 0.0)` pour gradient ; une case bloquée aussi ; la formule vaut sur toute la grille, obstacles et pas de
//! diffusion compris ; une case hors de la grille fait paniquer.
//!
//! Ils laissent ouverts : le texte du message de panique ; la valeur du gradient sur une grille de taille 1 dans le
//! sens qui n'a pas de voisin, hors ce que la règle du voisin remplacé impose (elle est testée : `(0.0, 0.0)`).
//!
//! Les nombres déposés sont des multiples de 1/64 : les différences et le facteur 0,5 sont exacts, donc les
//! comparaisons sont des égalités, sans tolérance.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`.

use mycelium_lab::substrat::Substrat;

/// Un substrat de `l × h` cases dont la case `(x, y)` vaut `f(x, y)` (diffusion 0,2, pour les tests qui font des pas).
fn champ(l: usize, h: usize, f: impl Fn(usize, usize) -> f64) -> Substrat {
    let mut s = Substrat::nouveau(l, h, 0.2, 1.0, 1.0).expect("substrat valide");
    for y in 0..h {
        for x in 0..l {
            s.deposer(x, y, f(x, y));
        }
    }
    s
}

/// Le gradient attendu, écrit ici d'après la spécification, sans passer par le code testé : différences centrées,
/// un voisin hors de la grille ou bloqué remplacé par la case elle-même, `(0, 0)` sur une case bloquée.
fn gradient_attendu(s: &Substrat, x: usize, y: usize) -> (f64, f64) {
    if s.est_bloquee(x, y) {
        return (0.0, 0.0);
    }
    let c = s.valeur(x, y);
    let voisin = |dx: i64, dy: i64| {
        let (i, j) = (x as i64 + dx, y as i64 + dy);
        if i < 0 || j < 0 || i >= s.largeur() as i64 || j >= s.hauteur() as i64 || s.est_bloquee(i as usize, j as usize)
        {
            c
        } else {
            s.valeur(i as usize, j as usize)
        }
    };
    ((voisin(1, 0) - voisin(-1, 0)) * 0.5, (voisin(0, 1) - voisin(0, -1)) * 0.5)
}

/// Un générateur reproductible, propre aux tests (congruentiel linéaire).
struct Hasard(u64);

impl Hasard {
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 24
    }
}

/// Exige que `f` panique.
fn exiger_la_panique(nom: &str, f: impl FnOnce() -> (f64, f64)) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok() {
        panic!("{nom} : l'appel aurait dû paniquer");
    }
}

/// 2. Sur un champ affine `a + bx·x + by·y`, le gradient d'une case intérieure est exactement `(bx, by)` : trois
///    champs, de pentes de signes différents (et une pente nulle dans un sens).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_champ_affine_a_sa_pente_exacte_a_l_interieur() {
    for (a, bx, by) in [(40.0, 0.5, -0.25), (2.0, -0.125, 0.75), (1.0, 0.0, 0.0625), (3.0, 1.5, 0.0)] {
        let (l, h) = (9, 7);
        let s = champ(l, h, |x, y| a + bx * x as f64 + by * y as f64);
        for y in 1..h - 1 {
            for x in 1..l - 1 {
                assert_eq!(s.gradient(x, y), (bx, by), "champ ({a}, {bx}, {by}), case ({x}, {y})");
            }
        }
    }
}

/// 2. Aux bords et aux coins, le voisin absent est remplacé par la case elle-même : la pente d'un champ affine y
///    est divisée par deux dans le sens du bord (et seulement dans ce sens), coins compris. Un voisin pris à zéro,
///    ou une différence non centrée, donnerait une autre valeur.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn aux_bords_et_aux_coins_le_voisin_absent_est_la_case_elle_meme() {
    for (a, bx, by) in [(40.0, 0.5, -0.25), (2.0, -0.125, 0.75)] {
        let (l, h) = (9, 7);
        let s = champ(l, h, |x, y| a + bx * x as f64 + by * y as f64);
        for y in 0..h {
            for x in 0..l {
                let gx = if x == 0 || x == l - 1 { bx * 0.5 } else { bx };
                let gy = if y == 0 || y == h - 1 { by * 0.5 } else { by };
                assert_eq!(s.gradient(x, y), (gx, gy), "champ ({a}, {bx}, {by}), case ({x}, {y})");
            }
        }
    }
}

/// 2. Les petites grilles : 1 × 1 (aucun voisin), une colonne et une ligne (un seul sens a des voisins), 2 × 2 (un
///    seul voisin dans chaque sens), valeurs calculées à la main.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn les_petites_grilles_se_calculent_a_la_main() {
    let seule = champ(1, 1, |_, _| 5.0);
    assert_eq!(seule.gradient(0, 0), (0.0, 0.0));

    // Une colonne de 5 cases (largeur 1) valant 1, 2, 4, 8, 16 : pas de voisin à gauche ni à droite.
    let colonne = champ(1, 5, |_, y| [1.0, 2.0, 4.0, 8.0, 16.0][y]);
    let attendu_gy = [0.5, 1.5, 3.0, 6.0, 4.0];
    for (y, gy) in attendu_gy.into_iter().enumerate() {
        assert_eq!(colonne.gradient(0, y), (0.0, gy), "colonne, ligne {y}");
    }
    // Une ligne de 5 cases (hauteur 1) : le même champ posé en travers.
    let ligne = champ(5, 1, |x, _| [1.0, 2.0, 4.0, 8.0, 16.0][x]);
    for (x, gx) in attendu_gy.into_iter().enumerate() {
        assert_eq!(ligne.gradient(x, 0), (gx, 0.0), "ligne, colonne {x}");
    }

    // 2 × 2 : [[1, 3], [5, 11]] (ligne 0 puis ligne 1).
    let deux = champ(2, 2, |x, y| [[1.0, 3.0], [5.0, 11.0]][y][x]);
    assert_eq!(deux.gradient(0, 0), (1.0, 2.0));
    assert_eq!(deux.gradient(1, 0), (1.0, 4.0));
    assert_eq!(deux.gradient(0, 1), (3.0, 2.0));
    assert_eq!(deux.gradient(1, 1), (3.0, 4.0));
}

/// 2. Le gradient d'un champ uniforme est exactement `(0.0, 0.0)`, partout : plusieurs valeurs (dont une qui n'est pas
///    un multiple d'une puissance de deux), avec et sans obstacle (un voisin bloqué est remplacé par la case, donc ne
///    crée aucun gradient), et après des pas de diffusion (un champ uniforme reste uniforme pour une valeur
///    dyadique).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_champ_uniforme_a_un_gradient_nul_exact() {
    for valeur in [0.0, 0.3, 1.0, 7.77, 0.001, 123456.789] {
        let s = champ(8, 6, |_, _| valeur);
        for y in 0..6 {
            for x in 0..8 {
                assert_eq!(s.gradient(x, y), (0.0, 0.0), "valeur {valeur}, case ({x}, {y})");
            }
        }
    }
    // Un champ uniforme avec des obstacles : toutes les cases libres, même contre un obstacle, ont un gradient nul.
    let mut avec_obstacles = champ(8, 6, |_, _| 2.5);
    avec_obstacles.bloquer(2, 1, 3, 2);
    avec_obstacles.bloquer(7, 5, 7, 5);
    avec_obstacles.bloquer(0, 3, 0, 5);
    for y in 0..6 {
        for x in 0..8 {
            assert_eq!(avec_obstacles.gradient(x, y), (0.0, 0.0), "obstacles, case ({x}, {y})");
        }
    }
    // Après des pas de diffusion (valeur dyadique : `pas()` laisse le champ exactement uniforme).
    let mut s = champ(8, 6, |_, _| 0.5);
    s.pas_n(25);
    for y in 0..6 {
        for x in 0..8 {
            assert_eq!(s.gradient(x, y), (0.0, 0.0), "après 25 pas, case ({x}, {y})");
        }
    }
}

/// 2. Le gradient pointe vers où le champ croît : `gx > 0` quand le champ croît avec la colonne, `gy > 0` quand il
///    croît avec l'indice de ligne (`y` croît vers le bas) ; autour d'un pic isolé, chaque case voisine a un gradient
///    qui pointe vers le pic.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_gradient_pointe_vers_ou_le_champ_croit() {
    let croit_en_x = champ(7, 7, |x, _| 10.0 + 2.0 * x as f64);
    assert_eq!(croit_en_x.gradient(3, 3), (2.0, 0.0));
    let decroit_en_x = champ(7, 7, |x, _| 20.0 - 2.0 * x as f64);
    assert_eq!(decroit_en_x.gradient(3, 3), (-2.0, 0.0));
    let croit_en_y = champ(7, 7, |_, y| 10.0 + 3.0 * y as f64);
    assert_eq!(croit_en_y.gradient(3, 3), (0.0, 3.0));
    let decroit_en_y = champ(7, 7, |_, y| 30.0 - 3.0 * y as f64);
    assert_eq!(decroit_en_y.gradient(3, 3), (0.0, -3.0));

    // Un pic de 9 en (4, 3) sur un fond de 1 : la case à gauche voit le pic à sa droite (gx > 0), celle à droite à
    // sa gauche (gx < 0), celle du dessus (ligne 2) le voit en dessous (gy > 0), celle du dessous au-dessus (gy < 0).
    let pic = champ(9, 7, |x, y| if (x, y) == (4, 3) { 9.0 } else { 1.0 });
    assert_eq!(pic.gradient(3, 3), (4.0, 0.0));
    assert_eq!(pic.gradient(5, 3), (-4.0, 0.0));
    assert_eq!(pic.gradient(4, 2), (0.0, 4.0));
    assert_eq!(pic.gradient(4, 4), (0.0, -4.0));
    assert_eq!(pic.gradient(4, 3), (0.0, 0.0), "le pic lui-même : ses deux voisins de chaque sens sont égaux");
    assert_eq!(pic.gradient(0, 0), (0.0, 0.0), "loin du pic");
}

/// 2. Un voisin bloqué est remplacé par la case elle-même (aucun flux), valeurs calculées à la main. Le champ est
///    `1 + x + 4y` sur 5 × 5 ; la case `(3, 2)` est bloquée. Sans blocage, le gradient de ces cases serait `(1, 4)`.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_voisin_bloque_est_remplace_par_la_case_elle_meme() {
    let mut s = champ(5, 5, |x, y| 1.0 + x as f64 + 4.0 * y as f64);
    assert_eq!(s.gradient(2, 2), (1.0, 4.0), "avant le blocage");
    s.bloquer(3, 2, 3, 2);
    // À gauche du bloc : la droite est bloquée, donc vaut la case (11) ; la gauche vaut 10.
    assert_eq!(s.gradient(2, 2), (0.5, 4.0));
    // Au-dessus du bloc : le bas est bloqué, donc vaut la case (8) ; le haut vaut 4.
    assert_eq!(s.gradient(3, 1), (1.0, 2.0));
    // À droite du bloc, sur la dernière colonne : la gauche est bloquée et la droite est hors de la grille.
    assert_eq!(s.gradient(4, 2), (0.0, 4.0));
    // Au-dessous du bloc : le haut est bloqué (la case vaut 16), le bas vaut 20.
    assert_eq!(s.gradient(3, 3), (1.0, 2.0));
    // Une case libre loin du bloc ne change pas.
    assert_eq!(s.gradient(1, 1), (1.0, 4.0));
}

/// 2. Une case bloquée a pour gradient `(0.0, 0.0)`, même quand ses voisins en ont un : au milieu, sur un bord, au
///    coin, dans un rectangle de plusieurs cases.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn une_case_bloquee_a_un_gradient_nul() {
    let mut s = champ(7, 6, |x, y| 1.0 + 3.0 * x as f64 + 5.0 * y as f64);
    assert_ne!(s.gradient(2, 2), (0.0, 0.0), "avant le blocage, la case a un gradient");
    s.bloquer(2, 2, 2, 2);
    s.bloquer(0, 0, 0, 0);
    s.bloquer(6, 3, 6, 3);
    s.bloquer(4, 4, 5, 5);
    for (x, y) in [(2, 2), (0, 0), (6, 3), (4, 4), (5, 4), (4, 5), (5, 5)] {
        assert!(s.est_bloquee(x, y), "garde : la case ({x}, {y}) devrait être bloquée");
        assert_eq!(s.gradient(x, y), (0.0, 0.0), "case bloquée ({x}, {y})");
    }
    // Garde contre un test vide : une case libre voisine a bien un gradient.
    assert_ne!(s.gradient(3, 2), (0.0, 0.0));
}

/// 2. La formule vaut sur toute la grille, bords, coins, obstacles et cases bloquées compris, sur un champ quelconque
///    (multiples de 1/64 tirés au hasard) et après des pas de diffusion : le gradient de chaque case est celui que la
///    spécification écrit, bit à bit.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_gradient_suit_la_formule_sur_toute_la_grille_apres_des_pas() {
    let mut hasard = Hasard(7);
    let (l, h) = (13, 9);
    let valeurs: Vec<f64> = (0..l * h).map(|_| (hasard.suivant() % 640) as f64 / 64.0).collect();
    let mut s = champ(l, h, |x, y| valeurs[y * l + x]);
    s.bloquer(3, 2, 5, 3);
    s.bloquer(0, 8, 0, 8);
    s.bloquer(12, 0, 12, 4);
    s.bloquer(7, 6, 9, 6);
    let mut cases_a_gradient = 0;
    let mut cases_bloquees = 0;
    for pas in 0..=30 {
        for y in 0..h {
            for x in 0..l {
                let obtenu = s.gradient(x, y);
                assert_eq!(obtenu, gradient_attendu(&s, x, y), "après {pas} pas, case ({x}, {y})");
                if pas == 0 {
                    if s.est_bloquee(x, y) {
                        cases_bloquees += 1;
                    } else if obtenu.0 * obtenu.0 + obtenu.1 * obtenu.1 > 0.01 {
                        cases_a_gradient += 1;
                    }
                }
            }
        }
        s.pas();
    }
    // Gardes contre un test vide : des cases bloquées, et beaucoup de cases à gradient net.
    assert_eq!(cases_bloquees, 6 + 1 + 5 + 3);
    assert!(cases_a_gradient >= 60, "seulement {cases_a_gradient} cases à gradient net");
}

/// 2. Contrat de panique : une case hors de la grille (colonne `largeur`, ligne `hauteur`, les deux, très loin)
///    fait paniquer, comme `valeur`. Le témoin (un appel valide, sur un coin) ne panique pas.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn gradient_panique_hors_de_la_grille() {
    let s = champ(8, 6, |x, y| 1.0 + x as f64 + y as f64);
    assert_eq!(s.gradient(7, 5), (0.5, 0.5), "le témoin : la dernière case, un coin");
    assert_eq!(s.gradient(0, 0), (0.5, 0.5), "le témoin : la première case");
    exiger_la_panique("gradient(8, 0)", || s.gradient(8, 0));
    exiger_la_panique("gradient(0, 6)", || s.gradient(0, 6));
    exiger_la_panique("gradient(8, 6)", || s.gradient(8, 6));
    exiger_la_panique("gradient(usize::MAX, 0)", || s.gradient(usize::MAX, 0));
    exiger_la_panique("gradient(3, usize::MAX)", || s.gradient(3, usize::MAX));
    // Une case bloquée hors de la grille ne change rien : la panique vaut aussi pour une grille avec obstacle.
    let mut avec_obstacle = champ(8, 6, |_, _| 1.0);
    avec_obstacle.bloquer(2, 2, 3, 3);
    assert_eq!(avec_obstacle.gradient(2, 2), (0.0, 0.0));
    exiger_la_panique("gradient(8, 3) avec obstacle", || avec_obstacle.gradient(8, 3));
}
