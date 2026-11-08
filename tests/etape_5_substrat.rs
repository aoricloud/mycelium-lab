//! Tests d'acceptation de l'étape 5 (T0, le terrain), partie substrat : les cases bloquées de `Substrat`.
//!
//! Ils fixent : `bloquer`, `est_bloquee` et leurs contrats de panique ; une case bloquée vaut `0.0` pour toujours,
//! ne reçoit rien et ne donne rien ; `pas()` n'a pas changé d'un octet sans case bloquée (comparé à une
//! réimplémentation écrite ici) et, avec des cases bloquées, remplace un voisin bloqué ou hors de la grille par la case
//! elle-même ; la masse est conservée, y compris dans chacun des compartiments qu'un mur sépare ; l'écriture
//! canonique garde son format.
//!
//! Les nombres déposés sont des multiples de 1/64 : leurs sommes sont exactes, les contrôles de masse « à
//! l'entier près » n'ont donc pas besoin de tolérance. Les tolérances des autres contrôles ont été mesurées (voir
//! les constantes).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs` et `max`.

use mycelium_lab::substrat::Substrat;

/// Un rectangle de cases `(x0, y0, x1, y1)`, bornes comprises.
type Rect = (usize, usize, usize, usize);

/// Tolérance sur la conservation de la masse (relative), après des centaines de pas avec des obstacles. Mesurée
/// sur le prototype : l'écart reste sous 3e-15 (et sous 7e-16 par compartiment) ; une fuite à travers un mur donne
/// un écart de l'ordre de 1e-2 ou plus.
const TOLERANCE_MASSE: f64 = 1e-11;

/// Tolérance (absolue, sur des valeurs de l'ordre de 10) entre `pas()` avec obstacles et la réimplémentation
/// naïve, pas à pas. Mesurée : 0 pour le même ordre de calcul, 2e-15 avec le flux écrit autrement
/// (`c + r · Σ (voisin - c)`) ; une faute de schéma donne un écart supérieur à 1e-4.
const TOLERANCE_CHAMP: f64 = 1e-11;

/// Un générateur reproductible, propre aux tests (congruentiel linéaire).
struct Hasard(u64);

impl Hasard {
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 24
    }

    /// Un entier de `0` à `n - 1`.
    fn dans(&mut self, n: usize) -> usize {
        (self.suivant() % n as u64) as usize
    }

    /// Un multiple de 1/64 entre 0 et 10.
    fn quantite(&mut self) -> f64 {
        (self.suivant() % 641) as f64 / 64.0
    }
}

fn dans_le_rect(r: Rect, x: usize, y: usize) -> bool {
    r.0 <= x && x <= r.2 && r.1 <= y && y <= r.3
}

/// Pour chaque case, vrai si un des rectangles la couvre.
fn masque(l: usize, h: usize, rects: &[Rect]) -> Vec<bool> {
    let mut m = vec![false; l * h];
    for y in 0..h {
        for x in 0..l {
            m[y * l + x] = rects.iter().any(|&r| dans_le_rect(r, x, y));
        }
    }
    m
}

/// Un champ de `l × h` cases : les valeurs déposées case par case, puis les rectangles bloqués.
fn construire(l: usize, h: usize, (d, dt, dx): (f64, f64, f64), valeurs: &[f64], rects: &[Rect]) -> Substrat {
    let mut s = Substrat::nouveau(l, h, d, dt, dx).expect("paramètres de diffusion valides");
    for y in 0..h {
        for x in 0..l {
            s.deposer(x, y, valeurs[y * l + x]);
        }
    }
    for &(x0, y0, x1, y1) in rects {
        s.bloquer(x0, y0, x1, y1);
    }
    s
}

fn champ(s: &Substrat) -> Vec<f64> {
    let mut v = Vec::with_capacity(s.largeur() * s.hauteur());
    for y in 0..s.hauteur() {
        for x in 0..s.largeur() {
            v.push(s.valeur(x, y));
        }
    }
    v
}

fn valeurs_au_hasard(hasard: &mut Hasard, n: usize) -> Vec<f64> {
    (0..n).map(|_| hasard.quantite()).collect()
}

/// Des rectangles au hasard dans une grille de `l × h`, de côtés au plus `cote_max`.
fn rects_au_hasard(hasard: &mut Hasard, l: usize, h: usize, n: usize, cote_max: usize) -> Vec<Rect> {
    let mut rects = Vec::new();
    for _ in 0..n {
        let x0 = hasard.dans(l);
        let y0 = hasard.dans(h);
        let x1 = x0 + hasard.dans(cote_max.min(l - x0));
        let y1 = y0 + hasard.dans(cote_max.min(h - y0));
        rects.push((x0, y0, x1, y1));
    }
    rects
}

/// Un pas du schéma à cinq points, réécrit ici sans lire le code du dépôt : champ plat, masque de cases bloquées.
/// Un voisin hors de la grille ou bloqué est remplacé par la case elle-même ; une case bloquée vaut `0.0`.
/// L'ordre du calcul est celui de `Substrat::pas` : `c + r * (gauche + droite + haut + bas - 4 c)`.
fn pas_naif(avant: &[f64], bloquee: &[bool], l: usize, h: usize, r: f64) -> Vec<f64> {
    let mut apres = vec![0.0; avant.len()];
    for y in 0..h {
        for x in 0..l {
            let i = y * l + x;
            if bloquee[i] {
                continue;
            }
            let c = avant[i];
            let voisin = |dx: isize, dy: isize| -> f64 {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                if nx < 0 || ny < 0 || nx >= l as isize || ny >= h as isize {
                    return c;
                }
                let j = ny as usize * l + nx as usize;
                if bloquee[j] { c } else { avant[j] }
            };
            apres[i] = c + r * (voisin(-1, 0) + voisin(1, 0) + voisin(0, -1) + voisin(0, 1) - 4.0 * c);
        }
    }
    apres
}

/// La masse des cases `x` de `x_min` à `x_max` (bornes comprises), toutes lignes.
fn masse_colonnes(s: &Substrat, x_min: usize, x_max: usize) -> f64 {
    let mut somme = 0.0;
    for y in 0..s.hauteur() {
        for x in x_min..=x_max {
            somme += s.valeur(x, y);
        }
    }
    somme
}

/// La masse des cases `y` de `y_min` à `y_max` (bornes comprises), toutes colonnes.
fn masse_lignes(s: &Substrat, y_min: usize, y_max: usize) -> f64 {
    let mut somme = 0.0;
    for y in y_min..=y_max {
        for x in 0..s.largeur() {
            somme += s.valeur(x, y);
        }
    }
    somme
}

fn ecart_relatif(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

/// 2. Sans case bloquée, `pas()` rend exactement les mêmes octets qu'avant : comparé, bit à bit et à chaque pas, à
///    la réimplémentation du schéma, sur des grilles de toutes formes (1×1, une ligne, une colonne, 2×2, plus
///    grandes) et plusieurs nombres de diffusion, dont la limite 1/4 et zéro.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn sans_case_bloquee_pas_est_la_reimplementation_du_schema_bit_a_bit() {
    let formes = [(1, 1), (1, 7), (7, 1), (2, 2), (5, 4), (17, 11), (30, 20)];
    let parametres = [(0.2, 1.0, 1.0), (0.7, 0.3, 1.1), (1.0, 0.25, 1.0), (0.0, 1.0, 1.0), (0.05, 2.0, 0.7)];
    let mut hasard = Hasard(17);
    for &(l, h) in &formes {
        for &(d, dt, dx) in &parametres {
            let valeurs = valeurs_au_hasard(&mut hasard, l * h);
            let mut s = construire(l, h, (d, dt, dx), &valeurs, &[]);
            let r = d * dt / (dx * dx);
            let rien = vec![false; l * h];
            let mut attendu = valeurs;
            for k in 1..=30 {
                s.pas();
                attendu = pas_naif(&attendu, &rien, l, h, r);
                for y in 0..h {
                    for x in 0..l {
                        let obtenu = s.valeur(x, y);
                        assert_eq!(
                            obtenu.to_bits(),
                            attendu[y * l + x].to_bits(),
                            "grille {l}×{h}, r = {r}, pas {k}, case ({x}, {y}) : {obtenu} au lieu de {}",
                            attendu[y * l + x]
                        );
                    }
                }
            }
        }
    }
}

/// 3. Avec des obstacles, `pas()` suit le schéma : un voisin bloqué ou hors de la grille est remplacé par la case
///    elle-même, une case bloquée reste à `0.0`. Obstacles au hasard (dont contre les bords, sur les coins,
///    qui se recouvrent), 60 pas, comparés à chaque pas à la réimplémentation. `pas_n(n)` rend les mêmes octets
///    que `n` appels à `pas()`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn avec_des_obstacles_pas_suit_le_schema_voisin_bloque_remplace_par_la_case() {
    let mut hasard = Hasard(99);
    let mut cas: Vec<(usize, usize, Vec<Rect>)> = vec![
        (9, 7, vec![(0, 0, 0, 0), (8, 6, 8, 6), (4, 0, 4, 2)]),
        (12, 5, vec![(3, 0, 3, 3), (7, 1, 7, 4), (0, 4, 11, 4)]),
        (1, 9, vec![(0, 3, 0, 4)]),
        (8, 1, vec![(2, 0, 2, 0), (6, 0, 6, 0)]),
    ];
    for _ in 0..8 {
        let (l, h) = (6 + hasard.dans(15), 6 + hasard.dans(12));
        let nombre = 1 + hasard.dans(6);
        let rects = rects_au_hasard(&mut hasard, l, h, nombre, 5);
        cas.push((l, h, rects));
    }
    let (d, dt, dx) = (0.9, 0.25, 1.0);
    let r = d * dt / (dx * dx);
    for (l, h, rects) in cas {
        let valeurs = valeurs_au_hasard(&mut hasard, l * h);
        let bloquee = masque(l, h, &rects);
        let mut s = construire(l, h, (d, dt, dx), &valeurs, &rects);
        let mut en_bloc = construire(l, h, (d, dt, dx), &valeurs, &rects);
        let mut attendu = champ(&s);
        for k in 1..=60 {
            s.pas();
            attendu = pas_naif(&attendu, &bloquee, l, h, r);
            for y in 0..h {
                for x in 0..l {
                    let obtenu = s.valeur(x, y);
                    if bloquee[y * l + x] {
                        assert_eq!(obtenu, 0.0, "grille {l}×{h}, pas {k} : la case bloquée ({x}, {y}) vaut {obtenu}");
                    }
                    let ecart = (obtenu - attendu[y * l + x]).abs();
                    assert!(
                        ecart <= TOLERANCE_CHAMP,
                        "grille {l}×{h}, obstacles {rects:?}, pas {k}, case ({x}, {y}) : {obtenu} au lieu de {}",
                        attendu[y * l + x]
                    );
                }
            }
            // La référence suit le champ réel : pas de dérive qui s'accumule entre les deux.
            attendu = champ(&s);
        }
        en_bloc.pas_n(60);
        assert_eq!(en_bloc.ecrire_canonique(), s.ecrire_canonique(), "pas_n(60) contre 60 appels à pas()");
    }
}

/// 3 (suite). Une case bloquée vaut `0.0` en permanence : tout de suite après `bloquer`, après chaque pas, même
///    collée à un pic, et même si on bloque en cours de route. `retirer` y rend `0.0` sans toucher à la masse.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_case_bloquee_vaut_zero_en_permanence() {
    let (l, h) = (11, 9);
    let mut s = Substrat::nouveau(l, h, 1.0, 0.2, 1.0).unwrap();
    for y in 0..h {
        for x in 0..l {
            s.deposer(x, y, 4.0);
        }
    }
    s.deposer(5, 4, 100.0);
    // Une case collée au pic, et deux cases dans un coin, dès le début.
    s.bloquer(6, 4, 6, 4);
    s.bloquer(0, 0, 1, 0);
    assert_eq!(s.valeur(6, 4), 0.0, "la case bloquée garde sa quantité");
    let mut bloquees = vec![(6, 4), (0, 0), (1, 0)];
    for k in 1..=200 {
        if k == 50 {
            // On bloque en cours de route une case qui contient du substrat.
            assert!(s.valeur(5, 4) > 4.0);
            s.bloquer(5, 4, 5, 4);
            bloquees.push((5, 4));
            assert_eq!(s.valeur(5, 4), 0.0, "la case bloquée en cours de route garde sa quantité");
        }
        s.pas();
        for &(x, y) in &bloquees {
            assert_eq!(s.valeur(x, y), 0.0, "pas {k} : la case bloquée ({x}, {y}) vaut {}", s.valeur(x, y));
            assert!(s.est_bloquee(x, y));
        }
        let avant = s.masse_totale();
        for &(x, y) in &bloquees {
            let retire = s.retirer(x, y, 1.0);
            assert_eq!(retire, 0.0, "retirer sur la case bloquée ({x}, {y}) rend {retire}");
        }
        assert_eq!(s.masse_totale(), avant, "retirer sur une case bloquée a changé la masse");
    }
}

/// 3 (suite). `bloquer` retire la quantité des cases bloquées : la masse baisse exactement de leur somme, une case
///    bloquée deux fois (rectangles qui se recouvrent) ne compte qu'une fois.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn bloquer_retire_la_quantite_et_la_masse_baisse_d_autant() {
    let (l, h) = (14, 10);
    let mut hasard = Hasard(5);
    let valeurs = valeurs_au_hasard(&mut hasard, l * h);
    let mut s = construire(l, h, (0.2, 1.0, 1.0), &valeurs, &[]);
    let somme = |rects: &[Rect]| -> f64 {
        let m = masque(l, h, rects);
        (0..l * h).filter(|&i| m[i]).map(|i| valeurs[i]).sum()
    };
    let masse0 = s.masse_totale();

    s.bloquer(2, 1, 6, 4);
    let attendu = masse0 - somme(&[(2, 1, 6, 4)]);
    assert_eq!(s.masse_totale(), attendu, "après un premier rectangle");

    // Un second rectangle qui recouvre le premier en partie, puis le premier une seconde fois.
    s.bloquer(5, 3, 9, 7);
    let attendu = masse0 - somme(&[(2, 1, 6, 4), (5, 3, 9, 7)]);
    assert_eq!(s.masse_totale(), attendu, "après un second rectangle qui recouvre le premier");
    s.bloquer(2, 1, 6, 4);
    s.bloquer(2, 1, 6, 4);
    assert_eq!(s.masse_totale(), attendu, "bloquer deux fois la même case change la masse");

    // Le champ hors des rectangles n'a pas bougé.
    for y in 0..h {
        for x in 0..l {
            let dedans = dans_le_rect((2, 1, 6, 4), x, y) || dans_le_rect((5, 3, 9, 7), x, y);
            let attendue = if dedans { 0.0 } else { valeurs[y * l + x] };
            assert_eq!(s.valeur(x, y), attendue, "case ({x}, {y})");
        }
    }
}

/// 3 (suite). `est_bloquee` est vraie exactement sur le rectangle `x0..=x1` × `y0..=y1`, bornes comprises, sur une
///    grille qui n'est pas carrée (un `x` pris pour un `y` se verrait) ; les cases voisines restent libres et
///    reçoivent encore un dépôt.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn est_bloquee_est_exactement_le_rectangle() {
    let (l, h) = (13, 9);
    let rects: [Rect; 8] = [
        (2, 1, 7, 3),
        (5, 5, 5, 5),
        (0, 0, 0, 0),
        (12, 8, 12, 8),
        (0, 4, 12, 4),
        (6, 0, 6, 8),
        (0, 0, 12, 8),
        (10, 2, 12, 6),
    ];
    for rect in rects {
        let mut s = Substrat::nouveau(l, h, 0.2, 1.0, 1.0).unwrap();
        s.bloquer(rect.0, rect.1, rect.2, rect.3);
        for y in 0..h {
            for x in 0..l {
                assert_eq!(
                    s.est_bloquee(x, y),
                    dans_le_rect(rect, x, y),
                    "rectangle {rect:?}, case ({x}, {y}) : est_bloquee est fausse"
                );
                if !dans_le_rect(rect, x, y) {
                    s.deposer(x, y, 1.0); // ne doit pas paniquer
                }
            }
        }
    }
}

/// 3 (suite). Une case libre entourée de quatre cases bloquées ne reçoit rien et ne donne rien : sa valeur ne
///    change pas, ni celle du reste du champ qu'elle ne touche pas.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_case_libre_entouree_d_obstacles_ne_change_pas() {
    let mut s = Substrat::nouveau(9, 9, 1.0, 0.25, 1.0).unwrap();
    s.deposer(4, 4, 3.0);
    s.deposer(0, 8, 5.0);
    s.bloquer(4, 3, 4, 3);
    s.bloquer(4, 5, 4, 5);
    s.bloquer(3, 4, 3, 4);
    s.bloquer(5, 4, 5, 4);
    s.pas_n(300);
    assert!((s.valeur(4, 4) - 3.0).abs() <= 1e-12, "la case enfermée vaut {}", s.valeur(4, 4));
    let reste = s.masse_totale() - s.valeur(4, 4);
    assert!(ecart_relatif(reste, 5.0) <= TOLERANCE_MASSE, "le reste du champ a perdu de la masse : {reste}");
}

/// 3 (suite). La masse est conservée avec des obstacles : 1000 pas, cinq graines, des rectangles au hasard.
///    Mesure : l'écart relatif reste sous 1e-15 ; la tolérance est `TOLERANCE_MASSE`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn la_masse_est_conservee_avec_des_obstacles() {
    let (l, h) = (30, 20);
    for graine in 1..=5u64 {
        let mut hasard = Hasard(graine * 7919);
        let rects = rects_au_hasard(&mut hasard, l, h, 6, 9);
        let valeurs = valeurs_au_hasard(&mut hasard, l * h);
        let mut s = construire(l, h, (0.8, 0.25, 1.0), &valeurs, &rects);
        let avant = s.masse_totale();
        assert!(avant > 1.0, "graine {graine} : presque tout est bloqué");
        // Garde contre un blocage qui ne bloque rien : la masse de départ est celle des seules cases libres (somme
        // exacte, multiples de 1/64), et les coins de chaque rectangle sont bloqués.
        let bloquee = masque(l, h, &rects);
        let libres: f64 = (0..l * h).filter(|&i| !bloquee[i]).map(|i| valeurs[i]).sum();
        assert_eq!(avant, libres, "graine {graine} : la masse de départ n'est pas celle des cases libres");
        for &(x0, y0, x1, y1) in &rects {
            assert!(
                s.est_bloquee(x0, y0) && s.est_bloquee(x1, y1),
                "graine {graine} : le rectangle {x0},{y0},{x1},{y1}"
            );
        }
        for k in 1..=1000 {
            s.pas();
            if k % 250 == 0 {
                let apres = s.masse_totale();
                assert!(
                    ecart_relatif(apres, avant) <= TOLERANCE_MASSE,
                    "graine {graine}, pas {k}, obstacles {rects:?} : masse {apres} au lieu de {avant}"
                );
            }
        }
    }
}

/// 3 (suite). Un mur qui traverse toute la boîte sépare deux compartiments : la masse de chacun est conservée
///    séparément, et un compartiment vide au départ reste exactement à `0.0` après des centaines de pas. Murs
///    verticaux et horizontaux, d'une et de trois cases d'épaisseur, au bord et au milieu, au nombre de diffusion
///    maximal et en dessous.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_mur_qui_traverse_la_boite_separe_deux_compartiments() {
    let (l, h) = (24, 16);
    let mut hasard = Hasard(31);
    // (mur, vertical ?, nombre de diffusion par d avec dt = dx = 1)
    let murs: [(Rect, bool, f64); 6] = [
        ((10, 0, 10, h - 1), true, 0.25),
        ((10, 0, 12, h - 1), true, 0.2),
        ((1, 0, 1, h - 1), true, 0.25),
        ((0, 7, l - 1, 7), false, 0.25),
        ((0, 6, l - 1, 8), false, 0.1),
        ((0, h - 2, l - 1, h - 2), false, 0.25),
    ];
    for (mur, vertical, d) in murs {
        for droite_vide in [true, false] {
            let valeurs: Vec<f64> = (0..l * h)
                .map(|i| {
                    let (x, y) = (i % l, i / l);
                    let apres_le_mur = if vertical { x > mur.2 } else { y > mur.3 };
                    if apres_le_mur && droite_vide { 0.0 } else { hasard.quantite() }
                })
                .collect();
            let mut s = construire(l, h, (d, 1.0, 1.0), &valeurs, &[mur]);
            let masses = |s: &Substrat| {
                if vertical {
                    (masse_colonnes(s, 0, mur.0 - 1), masse_colonnes(s, mur.2 + 1, l - 1))
                } else {
                    (masse_lignes(s, 0, mur.1 - 1), masse_lignes(s, mur.3 + 1, h - 1))
                }
            };
            let (gauche0, droite0) = masses(&s);
            assert!(gauche0 > 1.0);
            s.pas_n(600);
            let (gauche, droite) = masses(&s);
            let nom = format!("mur {mur:?}, d = {d}, côté vide au départ : {droite_vide}");
            assert!(
                ecart_relatif(gauche, gauche0) <= TOLERANCE_MASSE,
                "{nom} : masse du côté 1 {gauche} au lieu de {gauche0}"
            );
            if droite_vide {
                assert_eq!(droite0, 0.0);
                for y in 0..h {
                    for x in 0..l {
                        let apres_le_mur = if vertical { x > mur.2 } else { y > mur.3 };
                        if apres_le_mur {
                            assert_eq!(
                                s.valeur(x, y),
                                0.0,
                                "{nom} : la case ({x}, {y}) de l'autre côté du mur vaut {}",
                                s.valeur(x, y)
                            );
                        }
                    }
                }
            } else {
                assert!(
                    ecart_relatif(droite, droite0) <= TOLERANCE_MASSE,
                    "{nom} : masse du côté 2 {droite} au lieu de {droite0}"
                );
            }
        }
    }
}

/// 3 (suite). La diffusion contourne un mur percé d'une porte : la masse de l'autre côté, nulle au départ, devient
///    positive, et croît ; la masse totale reste conservée ; le mur reste à `0.0`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn la_diffusion_contourne_un_mur_perce_d_une_porte() {
    let (l, h) = (24, 16);
    let mut hasard = Hasard(8);
    let valeurs: Vec<f64> = (0..l * h).map(|i| if i % l < 10 { hasard.quantite() } else { 0.0 }).collect();
    // Le mur est la colonne x = 10, percée à y = 7 et y = 8.
    let mut s = construire(l, h, (0.2, 1.0, 1.0), &valeurs, &[(10, 0, 10, 6), (10, 9, 10, h - 1)]);
    let total = s.masse_totale();
    assert_eq!(masse_colonnes(&s, 11, l - 1), 0.0);
    s.pas_n(100);
    let apres_100 = masse_colonnes(&s, 11, l - 1);
    s.pas_n(200);
    let apres_300 = masse_colonnes(&s, 11, l - 1);
    // Mesuré : 5,6 % du total après 100 pas, 14,4 % après 300.
    assert!(apres_100 > 1e-3 * total, "après 100 pas, la masse passée par la porte est {apres_100} sur {total}");
    assert!(apres_300 > apres_100, "la masse de l'autre côté ne croît pas : {apres_100} puis {apres_300}");
    assert!(
        ecart_relatif(s.masse_totale(), total) <= TOLERANCE_MASSE,
        "masse totale {} au lieu de {total}",
        s.masse_totale()
    );
    for y in [0, 3, 6, 9, 12, 15] {
        assert_eq!(s.valeur(10, y), 0.0, "le mur (10, {y}) vaut {}", s.valeur(10, y));
    }
    // La porte, elle, est une case libre qui laisse passer.
    assert!(s.valeur(10, 7) > 0.0 && s.valeur(10, 8) > 0.0, "la porte reste vide");
}

/// 3 (suite). Un couloir d'une case de large, muré des deux côtés : c'est une diffusion à une dimension, dont la
///    variance croît exactement de `2 r` par pas (le noyau `r, 1 - 2r, r`), et la masse reste entière. Attendu
///    calculé à la main, sans le schéma.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_couloir_mure_diffuse_en_une_dimension() {
    let (l, h) = (201, 3);
    let (d, dt, dx) = (0.8, 0.25, 1.0);
    let r = d * dt / (dx * dx);
    let mut s = Substrat::nouveau(l, h, d, dt, dx).unwrap();
    s.bloquer(0, 0, l - 1, 0);
    s.bloquer(0, 2, l - 1, 2);
    s.deposer(100, 1, 1.0);
    let pas = 400;
    s.pas_n(pas);
    let (mut masse, mut moment2) = (0.0, 0.0);
    for x in 0..l {
        let v = s.valeur(x, 1);
        let position = x as f64 - 100.0;
        masse += v;
        moment2 += v * position * position;
        assert_eq!(s.valeur(x, 0), 0.0, "le mur du haut ({x}, 0) vaut {}", s.valeur(x, 0));
        assert_eq!(s.valeur(x, 2), 0.0, "le mur du bas ({x}, 2) vaut {}", s.valeur(x, 2));
    }
    assert!((masse - 1.0).abs() <= 1e-12, "la masse du couloir est {masse}");
    let attendue = 2.0 * r * pas as f64;
    let variance = moment2 / masse;
    assert!(ecart_relatif(variance, attendue) <= 1e-9, "variance {variance} au lieu de {attendue}");
}

/// 3 (suite). Des obstacles symétriques gardent le champ symétrique : un anneau carré percé de quatre portes au
///    milieu de ses côtés, un pic au centre, 200 pas, et les huit symétries du carré donnent le même champ.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn des_obstacles_symetriques_gardent_le_champ_symetrique() {
    let n = 25;
    let rects: [Rect; 8] = [
        (6, 6, 10, 6),
        (14, 6, 18, 6),
        (6, 18, 10, 18),
        (14, 18, 18, 18),
        (6, 6, 6, 10),
        (6, 14, 6, 18),
        (18, 6, 18, 10),
        (18, 14, 18, 18),
    ];
    let mut s = Substrat::nouveau(n, n, 1.0, 0.2, 1.0).unwrap();
    s.deposer(12, 12, 1.0);
    for r in rects {
        s.bloquer(r.0, r.1, r.2, r.3);
    }
    s.pas_n(200);
    let mut au_dehors = 0.0f64;
    for y in 0..n {
        for x in 0..n {
            let v = s.valeur(x, y);
            if !(5..=19).contains(&x) || !(5..=19).contains(&y) {
                au_dehors = au_dehors.max(v);
            }
            let autres = [
                s.valeur(n - 1 - x, y),
                s.valeur(x, n - 1 - y),
                s.valeur(n - 1 - y, x),
                s.valeur(n - 1 - x, n - 1 - y),
                s.valeur(y, n - 1 - x),
                s.valeur(y, x),
                s.valeur(n - 1 - y, n - 1 - x),
            ];
            for a in autres {
                assert!((a - v).abs() <= 1e-12, "asymétrie en ({x}, {y}) : {v} contre {a}");
            }
        }
    }
    // Garde contre un test vide : le champ a franchi les portes de l'anneau.
    assert!(au_dehors > 1e-6, "rien n'a franchi les portes de l'anneau : {au_dehors}");
}

/// 6. L'écriture canonique garde son format : `16 + 8 · largeur · hauteur` octets, `largeur` et `hauteur` en `u64`
///    petit-boutiste puis chaque case en `f64::to_bits` petit-boutiste ; une case bloquée s'y lit `0.0` ; deux champs
///    qui ne diffèrent que par le blocage ont les mêmes octets si leurs valeurs sont les mêmes.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn l_ecriture_canonique_ne_change_pas_de_format_et_lit_zero_sur_une_case_bloquee() {
    let (l, h) = (9, 6);
    let mut hasard = Hasard(3);
    let valeurs = valeurs_au_hasard(&mut hasard, l * h);
    let rects: [Rect; 2] = [(1, 1, 3, 2), (7, 5, 8, 5)];
    let mut s = construire(l, h, (0.2, 1.0, 1.0), &valeurs, &rects);
    s.pas_n(7);
    let octets = s.ecrire_canonique();
    assert_eq!(octets.len(), 16 + 8 * l * h, "longueur de l'écriture canonique");
    assert_eq!(&octets[0..8], &(l as u64).to_le_bytes(), "largeur");
    assert_eq!(&octets[8..16], &(h as u64).to_le_bytes(), "hauteur");
    let bloquee = masque(l, h, &rects);
    let mut sans_blocage = Substrat::nouveau(l, h, 0.2, 1.0, 1.0).unwrap();
    for y in 0..h {
        for x in 0..l {
            let i = y * l + x;
            let lu = f64::from_bits(u64::from_le_bytes(octets[16 + 8 * i..24 + 8 * i].try_into().unwrap()));
            assert_eq!(lu.to_bits(), s.valeur(x, y).to_bits(), "case ({x}, {y}) : octets et valeur() diffèrent");
            if bloquee[i] {
                assert_eq!(lu, 0.0, "la case bloquée ({x}, {y}) s'écrit {lu}");
            }
            sans_blocage.deposer(x, y, s.valeur(x, y));
        }
    }
    assert_eq!(sans_blocage.ecrire_canonique(), octets, "les octets ne dépendent que des valeurs des cases");
}

/// Exige que `f` panique. Ces contrats ne s'écrivent pas avec `#[should_panic]` : un corps resté en `todo!()` ou en
/// `unimplemented!()` panique toujours, et passerait. Chaque test commence donc par un appel valide (le témoin), hors
/// de `catch_unwind`, qui ne doit pas paniquer, puis exige la panique de l'appel invalide.
fn exiger_la_panique(nom: &str, f: impl FnOnce()) -> Box<dyn std::any::Any + Send> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(()) => panic!("{nom} : l'appel aurait dû paniquer"),
        Err(charge) => charge,
    }
}

/// Le témoin : un blocage valide sur une grille de 8 × 6, qui ne panique pas et qui bloque ce qu'il doit.
fn substrat_avec_un_temoin() -> Substrat {
    let mut s = Substrat::nouveau(8, 6, 0.2, 1.0, 1.0).unwrap();
    s.bloquer(1, 1, 2, 2);
    assert!(s.est_bloquee(1, 1) && s.est_bloquee(2, 2) && s.est_bloquee(1, 2) && s.est_bloquee(2, 1));
    assert!(!s.est_bloquee(0, 0) && !s.est_bloquee(3, 3) && !s.est_bloquee(7, 5));
    s.bloquer(7, 5, 7, 5); // un rectangle d'une case, au coin : permis
    assert!(s.est_bloquee(7, 5));
    s
}

/// 6. Contrat de panique de `bloquer` : `x0 > x1`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn bloquer_panique_si_x0_depasse_x1() {
    let mut s = substrat_avec_un_temoin();
    exiger_la_panique("bloquer(4, 2, 3, 2)", || s.bloquer(4, 2, 3, 2));
}

/// 6. Contrat de panique de `bloquer` : `y0 > y1`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn bloquer_panique_si_y0_depasse_y1() {
    let mut s = substrat_avec_un_temoin();
    exiger_la_panique("bloquer(2, 4, 2, 3)", || s.bloquer(2, 4, 2, 3));
}

/// 6. Contrat de panique de `bloquer` : `x1` sort de la grille (`x1 == largeur`, pile au-delà).
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn bloquer_panique_si_x1_sort_de_la_grille() {
    let mut s = substrat_avec_un_temoin();
    exiger_la_panique("bloquer(2, 1, 8, 2)", || s.bloquer(2, 1, 8, 2));
}

/// 6. Contrat de panique de `bloquer` : `y1` sort de la grille (`y1 == hauteur`).
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn bloquer_panique_si_y1_sort_de_la_grille() {
    let mut s = substrat_avec_un_temoin();
    exiger_la_panique("bloquer(2, 1, 3, 6)", || s.bloquer(2, 1, 3, 6));
}

/// 6. Contrat de panique de `bloquer` : le rectangle entier est hors de la grille.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn bloquer_panique_si_le_rectangle_est_hors_de_la_grille() {
    let mut s = substrat_avec_un_temoin();
    exiger_la_panique("bloquer(9, 7, 10, 8)", || s.bloquer(9, 7, 10, 8));
}

/// 6. Contrat de panique de `est_bloquee` : `x == largeur`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn est_bloquee_panique_hors_de_la_grille_en_x() {
    let s = substrat_avec_un_temoin();
    exiger_la_panique("est_bloquee(8, 0)", || {
        let _ = s.est_bloquee(8, 0);
    });
}

/// 6. Contrat de panique de `est_bloquee` : `y == hauteur`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn est_bloquee_panique_hors_de_la_grille_en_y() {
    let s = substrat_avec_un_temoin();
    exiger_la_panique("est_bloquee(0, 6)", || {
        let _ = s.est_bloquee(0, 6);
    });
}

/// 6. `deposer` sur une case bloquée panique (« case bloquée »), même d'une quantité valide ; sur une case libre, à
///    côté, il ne panique pas. Le message est celui de la spécification, jugé sur sa racine « bloqu » (accent ou
///    genre ne comptent pas) : une panique pour une autre raison ne passe pas.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn deposer_sur_une_case_bloquee_panique() {
    let mut s = substrat_avec_un_temoin();
    s.deposer(3, 3, 1.0); // case libre voisine du témoin : permis
    s.deposer(0, 0, 2.0);
    assert_eq!(s.valeur(3, 3), 1.0);
    let charge = exiger_la_panique("deposer(2, 2, 1.0) sur une case bloquée", || s.deposer(2, 2, 1.0));
    let message = if let Some(m) = charge.downcast_ref::<String>() {
        m.clone()
    } else if let Some(m) = charge.downcast_ref::<&str>() {
        (*m).to_string()
    } else {
        String::new()
    };
    assert!(message.contains("bloqu"), "la panique de deposer sur une case bloquée dit : {message:?}");
}
