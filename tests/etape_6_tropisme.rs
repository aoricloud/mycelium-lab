//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie virage : `GRADIENT_DE_REFERENCE`,
//! `virage_chimiotropique` et `tourner_chimiotropique`.
//!
//! Ils fixent : la constante de saturation vaut 0,01 ; le virage réel vaut `force · s · sin(angle du gradient −
//! direction)` avec `s = n / (n + 0,01)`, pour tous les angles entiers (la valeur attendue est lue dans la table `SIN` à
//! l'indice de l'écart) ; son signe (un gradient dans le sens trigonométrique de la pointe donne un virage positif) ;
//! son maximum, `force · s`, à angle droit, et son zéro pour un gradient aligné, opposé ou nul ; la saturation de la
//! réponse (un gradient 100 fois plus faible tourne environ 100 fois moins) ; la linéarité en `force` ; l'invariance
//! par rotation de 90° et l'antisymétrie par symétrie ; la plus grande force permise, `FORCE_MAXIMALE = 45` ; la
//! panique hors contrat de `virage_chimiotropique` (direction `>= 360`, même pour un gradient nul ; force hors de
//! `0..=45`, `NaN`, infinie) et celle de `tourner_chimiotropique` quand la force est positive. Pour
//! `tourner_chimiotropique` : aucun tirage pour une force nulle et pour un gradient nul, un seul tirage sinon (même
//! pour un virage nul) ; le résultat est la direction plus le virage arrondi au sort (`< frac`, tirage rejoué par le
//! test), toujours dans `0..360` ; le virage appliqué est `⌊v⌋` ou `⌊v⌋ + 1`, jamais à un degré ou plus de `v`, et vaut
//! `v` en moyenne (tolérance mesurée, voir `TOLERANCE_MOYENNE`) ; un virage négatif minuscule, dont la fraction
//! `v − ⌊v⌋` s'arrondit à `1.0` en flottant, laisse la direction inchangée (la règle appliquée telle quelle).
//!
//! Ils laissent ouverts : le texte des messages de panique ; ce que `virage_chimiotropique` fait d'un gradient non
//! fini (contrat de l'appelant, sans panique exigée) ; ce que `tourner_chimiotropique` fait d'une direction hors de la
//! table quand la force est nulle ou négative (la spécification la rend sans la regarder ; non testé) ; la valeur d'un
//! gradient dont le carré de la norme sous-passe (`n == 0.0` sans que le gradient soit nul).
//!
//! L'orientation moyenne à l'équilibre (preuve 5) est dans `etape_6_equilibre.rs`.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `sqrt` et `floor`.

use mycelium_lab::alea::Alea;
use mycelium_lab::croissance::{COS, NB_DIRECTIONS, SIN};
use mycelium_lab::tropisme::{FORCE_MAXIMALE, GRADIENT_DE_REFERENCE, tourner_chimiotropique, virage_chimiotropique};

/// La constante de saturation, réécrite ici : les tests de formule ne dépendent pas de la constante du code.
const G0: f64 = 0.01;

/// Tolérance (absolue) sur le virage réel quand l'attendu est lu dans la table `SIN` (les tables sont arrondies au
/// `f64` le plus proche, le code les combine autrement). Mesurée sur le prototype : l'écart maximal, sur les
/// 777 600 combinaisons du test de formule (force jusqu'à 4), est 4,8e-15 ; sur les contrôles « aligné ou opposé » 2,6e-15 ;
/// sur la rotation et la symétrie 2,7e-15. Une faute de formule (mauvais signe, `s` oublié, angle décalé d'un degré)
/// donne plus de 1e-3.
const TOLERANCE_VIRAGE: f64 = 1e-12;

/// Tolérance (absolue, en degrés) sur la moyenne du virage appliqué par `tourner_chimiotropique` sur `N_TIRAGES`
/// tirages : l'écart-type de la moyenne est `sqrt(frac · (1 − frac) / N)`, au plus `0,5 / sqrt(20000) = 0,0035`.
/// Mesurée sur le prototype : sur 40 graines et les 8 cas du test, les écarts-types observés sont de 0,0025 à 0,0033 et
/// l'écart `|moyenne − v|` maximal est 0,011 ; la tolérance, 0,02, est 5,7 fois l'écart-type maximal de la théorie. Un
/// arrondi vers le bas donne un écart égal à `frac`, un arrondi au plus proche un écart `min(frac, 1 − frac)` : au moins
/// 0,167 dans les huit cas du test, très au-dessus de la tolérance. Rejoué par le juge sur 20 autres graines par cas :
/// écart-type observé au plus 0,0033, écart maximal 0,0070 ; la tolérance est à 6 écarts-types.
const TOLERANCE_MOYENNE: f64 = 0.02;

/// Nombre de tirages du test de moyenne.
const N_TIRAGES: u32 = 20_000;

/// Le gradient de norme `norme` dont l'angle (en degrés entiers) est `angle`.
fn gradient_vers(angle: usize, norme: f64) -> (f64, f64) {
    (norme * COS[angle % NB_DIRECTIONS], norme * SIN[angle % NB_DIRECTIONS])
}

/// La réponse `s = n / (n + G0)`.
fn reponse(n: f64) -> f64 {
    n / (n + G0)
}

/// Exige que `f` panique.
fn exiger_la_panique(nom: &str, f: impl FnOnce() -> f64) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok() {
        panic!("{nom} : l'appel aurait dû paniquer");
    }
}

/// Vrai si `a` et `b` sont à moins de `tolerance` l'un de l'autre.
fn proches(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

// ---------------------------------------------------------------------------------------------------------------
// 3. virage_chimiotropique
// ---------------------------------------------------------------------------------------------------------------

/// 3. La constante de saturation est 0,01, comme la spécification la pose.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_constante_de_reference_vaut_0_01() {
    assert_eq!(GRADIENT_DE_REFERENCE, 0.01, "GRADIENT_DE_REFERENCE : attendu 0,01");
}

/// 3 et 7. La plus grande force permise est 45 (`FORCE_MAXIMALE`) : le virage d'un pas ne dépasse jamais 45°. Choisie
///    par le calcul exact de la loi d'équilibre (rapport du juge) : l'orientation moyenne croît strictement avec
///    `p = force · s` jusqu'à `p = 57,25` pour les amplitudes 1, 2 et 5, plus loin pour les autres, puis décroît (à
///    `p = 360`, amplitude 20, elle est négative : −0,216).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_force_maximale_vaut_45() {
    assert_eq!(FORCE_MAXIMALE, 45.0, "FORCE_MAXIMALE : attendu 45");
}

/// 3. Le virage vaut `force · s · sin(angle du gradient − direction)` pour tous les angles entiers : 360 directions,
///    360 angles de gradient, trois normes (en dessous, au niveau et au-dessus de `G0`), deux forces.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_virage_est_force_fois_reponse_fois_sinus_de_l_ecart() {
    let mut ecart_max: f64 = 0.0;
    let mut plus_grand_virage: f64 = 0.0;
    for force in [1.0, 4.0] {
        for n in [0.001, 0.01, 0.2] {
            for direction in 0..NB_DIRECTIONS {
                for angle in 0..NB_DIRECTIONS {
                    let voulu = force * reponse(n) * SIN[(angle + NB_DIRECTIONS - direction) % NB_DIRECTIONS];
                    let obtenu = virage_chimiotropique(direction, gradient_vers(angle, n), force);
                    let ecart = (obtenu - voulu).abs();
                    if ecart > ecart_max {
                        ecart_max = ecart;
                    }
                    if obtenu.abs() > plus_grand_virage {
                        plus_grand_virage = obtenu.abs();
                    }
                    assert!(
                        ecart <= TOLERANCE_VIRAGE,
                        "force {force}, norme {n}, direction {direction}, angle du gradient {angle} : {obtenu} au lieu de {voulu}"
                    );
                }
            }
        }
    }
    // Garde contre un test vide : le plus grand virage est celui de la force 4 à n = 0,2 (s = 0,952), à angle droit.
    assert!(plus_grand_virage > 3.8, "plus grand virage observé : {plus_grand_virage}");
    assert!(ecart_max <= TOLERANCE_VIRAGE);
}

/// 3. Le signe : un gradient dans le sens trigonométrique de la pointe (l'angle du gradient est plus grand que la
///    direction, de moins de 180°) donne un virage positif ; dans l'autre sens, négatif. Cas à la main, sur les axes
///    (`y` croît avec l'indice de ligne, donc vers le bas de l'écran : le sens trigonométrique de la table).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_signe_du_virage_suit_le_sens_trigonometrique() {
    // (direction, gradient, signe attendu)
    let cas: [(usize, (f64, f64), f64); 12] = [
        (0, (0.0, 1.0), 1.0),
        (0, (0.0, -1.0), -1.0),
        (90, (-1.0, 0.0), 1.0),
        (90, (1.0, 0.0), -1.0),
        (180, (0.0, -1.0), 1.0),
        (180, (0.0, 1.0), -1.0),
        (270, (1.0, 0.0), 1.0),
        (270, (-1.0, 0.0), -1.0),
        (45, (0.0, 1.0), 1.0),
        (350, (1.0, 0.0), 1.0),
        (10, (1.0, 0.0), -1.0),
        (0, (1.0, 0.1), 1.0),
    ];
    for (direction, gradient, signe) in cas {
        let v = virage_chimiotropique(direction, gradient, 2.0);
        assert!(v * signe > 0.0, "direction {direction}, gradient {gradient:?} : virage {v}, signe attendu {signe}");
    }
    // Pour tout écart entre 1° et 179°, le signe est celui de l'écart.
    for direction in (0..NB_DIRECTIONS).step_by(7) {
        for ecart in 1..180 {
            let plus = virage_chimiotropique(direction, gradient_vers(direction + ecart, 0.05), 3.0);
            let moins = virage_chimiotropique(direction, gradient_vers(direction + NB_DIRECTIONS - ecart, 0.05), 3.0);
            assert!(plus > 0.0, "direction {direction}, écart +{ecart} : virage {plus}");
            assert!(moins < 0.0, "direction {direction}, écart -{ecart} : virage {moins}");
        }
    }
}

/// 3. Le virage est maximal à angle droit, où il vaut `±force · s`, et nul (à la tolérance de la table) pour un
///    gradient aligné sur la pointe ou opposé à elle ; il ne dépasse jamais `force · s`.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_virage_est_maximal_a_angle_droit_et_nul_si_aligne_ou_oppose() {
    let force = 3.0;
    for n in [0.003, 0.01, 1.0] {
        let maximum = force * reponse(n);
        for direction in 0..NB_DIRECTIONS {
            let aligne = virage_chimiotropique(direction, gradient_vers(direction, n), force);
            let oppose = virage_chimiotropique(direction, gradient_vers(direction + 180, n), force);
            let gauche = virage_chimiotropique(direction, gradient_vers(direction + 90, n), force);
            let droite = virage_chimiotropique(direction, gradient_vers(direction + 270, n), force);
            assert!(aligne.abs() <= TOLERANCE_VIRAGE, "norme {n}, direction {direction}, aligné : {aligne}");
            assert!(oppose.abs() <= TOLERANCE_VIRAGE, "norme {n}, direction {direction}, opposé : {oppose}");
            assert!(proches(gauche, maximum, TOLERANCE_VIRAGE), "norme {n}, direction {direction} : {gauche}");
            assert!(proches(droite, -maximum, TOLERANCE_VIRAGE), "norme {n}, direction {direction} : {droite}");
            for angle in (0..NB_DIRECTIONS).step_by(11) {
                let v = virage_chimiotropique(direction, gradient_vers(angle, n), force);
                assert!(v.abs() <= maximum + TOLERANCE_VIRAGE, "norme {n}, direction {direction}, angle {angle} : {v}");
            }
        }
    }
}

/// 3. La réponse sature : `s = n / (n + 0,01)`. À angle droit, le virage vaut `force · n / (n + 0,01)` ; il vaut la
///    moitié du maximum pour `n = 0,01`, tend vers `force` pour un gradient très fort, croît avec `n`, et un
///    gradient 100 fois plus faible (loin sous 0,01) tourne environ 100 fois moins.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn la_reponse_sature_comme_n_sur_n_plus_g0() {
    let force = 2.0;
    let normes = [1e-6, 1e-5, 1e-4, 1e-3, 3e-3, 0.01, 0.03, 0.1, 1.0, 1e3, 1e9];
    let mut precedent = 0.0;
    for n in normes {
        // Gradient à angle droit d'une pointe de direction 0 : vers le haut de la table (angle 90°), norme n.
        let v = virage_chimiotropique(0, (0.0, n), force);
        let voulu = force * n / (n + G0);
        assert!(proches(v, voulu, 1e-12 * voulu), "norme {n} : virage {v} au lieu de {voulu}");
        assert!(v > precedent, "norme {n} : le virage {v} ne croît pas (précédent {precedent})");
        precedent = v;
    }
    let moitie = virage_chimiotropique(0, (0.0, 0.01), force);
    assert!(proches(moitie, force * 0.5, 1e-12), "à n = 0,01 le virage devrait valoir la moitié de {force} : {moitie}");
    let sature = virage_chimiotropique(0, (0.0, 1e9), force);
    assert!(sature < force && sature > force * (1.0 - 1e-9), "gradient très fort : {sature}");
    // « Un gradient 100 fois plus faible tourne environ 100 fois moins » : le rapport vaut 0,01 à 2 % près.
    for (faible, fort) in [(1e-6, 1e-4), (1e-7, 1e-5), (2e-8, 2e-6)] {
        let rapport = virage_chimiotropique(0, (0.0, faible), force) / virage_chimiotropique(0, (0.0, fort), force);
        assert!(proches(rapport, 0.01, 0.0002), "gradients {faible} et {fort} : rapport des virages {rapport}");
    }
    // Seule la norme compte, par `s` : changer l'échelle du gradient sans changer sa direction ne change que `s`.
    let direction = 40;
    let (a, b) = (gradient_vers(130, 0.004), gradient_vers(130, 0.04));
    let rapport = virage_chimiotropique(direction, b, force) / virage_chimiotropique(direction, a, force);
    assert!(proches(rapport, reponse(0.04) / reponse(0.004), 1e-9), "rapport des virages : {rapport}");
}

/// 3. Le virage est linéaire en `force`, nul pour la force 0, et nul pour un gradient nul (quelles que soient la
///    direction et la force, `-0.0` compris).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_virage_est_lineaire_en_force_et_nul_pour_force_0_ou_gradient_nul() {
    for direction in (0..NB_DIRECTIONS).step_by(13) {
        for gradient in [(0.02, -0.01), (-0.3, 0.2), (0.001, 0.004), (5.0, 5.0)] {
            let un = virage_chimiotropique(direction, gradient, 1.0);
            for force in [2.0, 3.0, 0.5, 30.0, 45.0] {
                let v = virage_chimiotropique(direction, gradient, force);
                assert!(
                    proches(v, force * un, 1e-12 * (1.0 + force)),
                    "direction {direction}, gradient {gradient:?}, force {force} : {v} au lieu de {}",
                    force * un
                );
            }
            assert_eq!(virage_chimiotropique(direction, gradient, 0.0), 0.0, "force 0 : direction {direction}");
        }
        for force in [0.0, 1.0, 4.0, 45.0] {
            for gradient in [(0.0, 0.0), (-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
                assert_eq!(
                    virage_chimiotropique(direction, gradient, force),
                    0.0,
                    "gradient nul {gradient:?}, direction {direction}, force {force}"
                );
            }
        }
    }
    // Garde : un cas à virage net, pour que les égalités ci-dessus ne soient pas toutes des zéros.
    assert!(virage_chimiotropique(0, (0.0, 0.1), 2.0) > 1.0);
}

/// 3. Invariance par rotation de 90° (et de 180°, 270°) de la pointe et du gradient ensemble, et antisymétrie par
///    symétrie d'axe `x` (direction `−d`, gradient `(gx, −gy)` : virage opposé).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_virage_est_invariant_par_rotation_et_antisymetrique_par_symetrie() {
    let gradients = [(0.003, -0.007), (0.05, 0.02), (-0.2, 0.1), (1e-4, 3e-4), (0.0, 0.6), (-0.9, -0.01)];
    for force in [1.0, 4.0] {
        for direction in 0..NB_DIRECTIONS {
            for (gx, gy) in gradients {
                let v = virage_chimiotropique(direction, (gx, gy), force);
                let tourne_90 = virage_chimiotropique((direction + 90) % NB_DIRECTIONS, (-gy, gx), force);
                let tourne_180 = virage_chimiotropique((direction + 180) % NB_DIRECTIONS, (-gx, -gy), force);
                let tourne_270 = virage_chimiotropique((direction + 270) % NB_DIRECTIONS, (gy, -gx), force);
                for (nom, autre) in [("90°", tourne_90), ("180°", tourne_180), ("270°", tourne_270)] {
                    assert!(
                        proches(v, autre, 1e-12),
                        "force {force}, direction {direction}, gradient ({gx}, {gy}), rotation de {nom} : {autre} contre {v}"
                    );
                }
                let miroir = virage_chimiotropique((NB_DIRECTIONS - direction) % NB_DIRECTIONS, (gx, -gy), force);
                assert!(
                    proches(miroir, -v, 1e-12),
                    "force {force}, direction {direction}, gradient ({gx}, {gy}), symétrie : {miroir} contre {}",
                    -v
                );
            }
        }
    }
}

/// 7. `virage_chimiotropique` panique hors contrat : direction hors de la table (même quand le gradient est nul), force
///    négative, supérieure à 45 (le `f64` juste au-dessus compris), `NaN` ou infinie. Les témoins (appels valides, bornes
///    comprises : force 0 et force 45, direction 359) ne paniquent pas, et leur virage est fini, dans `±45`.
///    `tourner_chimiotropique`, quand la force est positive, calcule le virage avant tout : elle panique de même pour une
///    direction 360 ou une force supérieure à 45, infinie ou `NaN`, que le gradient soit nul ou non.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn virage_chimiotropique_panique_hors_contrat() {
    let gradient = (0.02, -0.01);
    for (nom, v) in [
        ("force 1", virage_chimiotropique(10, gradient, 1.0)),
        ("force 0", virage_chimiotropique(10, gradient, 0.0)),
        ("force 45", virage_chimiotropique(10, gradient, 45.0)),
        ("direction 359", virage_chimiotropique(359, gradient, 1.0)),
        ("direction 0", virage_chimiotropique(0, gradient, 1.0)),
    ] {
        assert!(v.is_finite() && v.abs() <= 45.0, "témoin {nom} : virage {v}, attendu fini et dans ±45");
    }
    for direction in [360, 361, 720, usize::MAX] {
        exiger_la_panique(&format!("direction {direction}"), || virage_chimiotropique(direction, gradient, 1.0));
    }
    exiger_la_panique("direction 360, force 0", || virage_chimiotropique(360, gradient, 0.0));
    exiger_la_panique("direction 360, gradient nul", || virage_chimiotropique(360, (0.0, 0.0), 1.0));
    exiger_la_panique("direction 400, gradient nul, force 0", || virage_chimiotropique(400, (0.0, 0.0), 0.0));
    let juste_au_dessus = f64::from_bits(45.0f64.to_bits() + 1);
    for force in [
        -1.0,
        -1e-9,
        juste_au_dessus,
        45.000001,
        46.0,
        90.0,
        360.0,
        361.0,
        1e300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        exiger_la_panique(&format!("force {force}"), || virage_chimiotropique(10, gradient, force));
    }
    exiger_la_panique("force négative, gradient nul", || virage_chimiotropique(10, (0.0, 0.0), -1.0));
    exiger_la_panique("force NaN, gradient nul", || virage_chimiotropique(10, (0.0, 0.0), f64::NAN));
    exiger_la_panique("force juste au-dessus de 45, gradient nul", || {
        virage_chimiotropique(10, (0.0, 0.0), juste_au_dessus)
    });
    // `tourner_chimiotropique` à force positive : le même contrat.
    let tourner = |direction: usize, g: (f64, f64), force: f64| {
        move || tourner_chimiotropique(direction, g, force, &mut Alea::new(1)) as f64
    };
    for g in [gradient, (0.0, 0.0)] {
        exiger_la_panique(&format!("tourner, direction 360, gradient {g:?}"), tourner(360, g, 1.0));
        for force in [juste_au_dessus, 46.0, 360.0, f64::INFINITY, f64::NAN] {
            exiger_la_panique(&format!("tourner, force {force}, gradient {g:?}"), tourner(10, g, force));
        }
    }
    // Témoin : la borne 45 et la direction 359 sont permises.
    let d = tourner_chimiotropique(359, gradient, 45.0, &mut Alea::new(1));
    assert!(d < NB_DIRECTIONS, "tourner, force 45, direction 359 : résultat {d}, attendu dans 0..360");
}

// ---------------------------------------------------------------------------------------------------------------
// 4. tourner_chimiotropique
// ---------------------------------------------------------------------------------------------------------------

/// Vrai si, après l'appel, `alea` est dans l'état d'un `Alea` neuf de même graine qui a tiré `tirages` nombres : on le
/// vérifie en comparant leur tirage suivant (`Alea` n'est pas comparable).
fn a_tire_exactement(mut alea: Alea, graine: u64, tirages: usize) -> bool {
    let mut neuf = Alea::new(graine);
    for _ in 0..tirages {
        neuf.suivant();
    }
    (0..3).all(|_| alea.suivant() == neuf.suivant())
}

/// 4. Sans tirage pour une force nulle (`0.0` et `-0.0`), même dans un gradient fort, et pour un gradient nul, même
///    avec une force forte : la direction est rendue telle quelle et le générateur n'a pas bougé.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn tourner_ne_tire_rien_pour_force_nulle_ou_gradient_nul() {
    for graine in [0u64, 1, 42, u64::MAX] {
        for direction in [0usize, 1, 179, 180, 359] {
            for (force, gradient) in [
                (0.0, (0.5, -0.3)),
                (-0.0, (0.5, -0.3)),
                (0.0, (0.0, 0.0)),
                (1.0, (0.0, 0.0)),
                (4.0, (-0.0, 0.0)),
                (45.0, (0.0, -0.0)),
                (2.5, (-0.0, -0.0)),
            ] {
                let mut alea = Alea::new(graine);
                let obtenue = tourner_chimiotropique(direction, gradient, force, &mut alea);
                assert_eq!(obtenue, direction, "force {force}, gradient {gradient:?} : la direction a changé");
                assert!(
                    a_tire_exactement(alea, graine, 0),
                    "graine {graine}, force {force}, gradient {gradient:?} : un tirage a été fait"
                );
            }
        }
    }
}

/// 4. Un seul tirage sinon, même quand le virage est nul (un gradient aligné ou opposé : le tirage est fait quand même,
///    la spécification dit « toujours fait quand force > 0 et gradient non nul »), pour un virage positif, négatif,
///    grand, petit.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn tourner_tire_exactement_un_nombre_sinon() {
    let cas: [(usize, (f64, f64), f64); 8] = [
        (0, (0.05, 0.0), 2.0),    // aligné : virage nul
        (0, (-0.05, 0.0), 2.0),   // opposé : virage nul
        (0, (0.0, 0.05), 2.0),    // virage positif
        (0, (0.0, -0.05), 2.0),   // virage négatif
        (123, (0.3, 0.4), 0.001), // très petit virage
        (300, (-0.2, 0.7), 45.0), // le plus grand virage permis
        (359, (1e-9, 1e-9), 1.0), // gradient minuscule mais non nul
        (17, (1e9, -1e9), 4.0),   // gradient énorme
    ];
    for graine in [0u64, 1, 42, u64::MAX] {
        for (direction, gradient, force) in cas {
            let mut alea = Alea::new(graine);
            let resultat = tourner_chimiotropique(direction, gradient, force, &mut alea);
            assert!(resultat < NB_DIRECTIONS, "résultat hors de la table : {resultat}");
            assert!(
                a_tire_exactement(alea, graine, 1),
                "graine {graine}, direction {direction}, gradient {gradient:?}, force {force} : pas exactement un tirage"
            );
        }
    }
}

/// 4. Le résultat est `direction + ⌊v⌋ + (1 si le tirage < frac(v))` modulo 360 : le test rejoue le tirage sur une copie
///    du générateur. Il fixe l'arrondi au sort (et pas au plus proche ni vers le bas), l'inégalité `tirage < frac`, et le
///    signe de `v` négatif (`⌊v⌋` plonge vers le bas). Plusieurs centaines de combinaisons, dont des virages négatifs,
///    de plus d'un degré, et au passage de 0/359.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn tourner_rend_la_direction_plus_le_virage_arrondi_au_sort() {
    let mut vus_vers_le_haut = 0;
    let mut vus_vers_le_bas = 0;
    let mut negatifs = 0;
    let mut graine = 0u64;
    for direction in (0..NB_DIRECTIONS).step_by(9) {
        for gradient in [(0.02, 0.0), (0.0, 0.05), (-0.01, -0.01), (0.3, -0.4), (0.004, 0.003)] {
            for force in [0.7, 1.0, 2.5, 4.0, 30.0] {
                graine += 1;
                let mut rejeu = Alea::new(graine);
                let tirage = rejeu.suivant_f64();
                let v = virage_chimiotropique(direction, gradient, force);
                let bas = v.floor();
                let frac = v - bas;
                let monte = tirage < frac;
                let voulue = (direction as i64 + bas as i64 + i64::from(monte)).rem_euclid(360) as usize;
                let mut alea = Alea::new(graine);
                let obtenue = tourner_chimiotropique(direction, gradient, force, &mut alea);
                assert_eq!(
                    obtenue, voulue,
                    "graine {graine}, direction {direction}, gradient {gradient:?}, force {force} : v = {v}, tirage {tirage}"
                );
                if monte {
                    vus_vers_le_haut += 1;
                } else {
                    vus_vers_le_bas += 1;
                }
                if v < 0.0 {
                    negatifs += 1;
                }
            }
        }
    }
    // Gardes : les deux issues de l'arrondi et des virages négatifs sont bien dans l'échantillon.
    assert!(vus_vers_le_haut >= 50 && vus_vers_le_bas >= 50, "arrondis : {vus_vers_le_haut} / {vus_vers_le_bas}");
    assert!(negatifs >= 100, "virages négatifs : {negatifs}");
}

/// 4. Le résultat reste dans `0..360` pour toutes les directions, avec de grands virages dans les deux sens (force
///    jusqu'à 45, la borne), et il passe bien de 0 à 359 et de 359 à 0.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn tourner_reste_dans_la_table_et_boucle_autour_de_360() {
    let mut alea = Alea::new(99);
    let (mut a_boucle_vers_le_haut, mut a_boucle_vers_le_bas) = (false, false);
    for direction in 0..NB_DIRECTIONS {
        for gradient in [(0.0, 0.5), (0.0, -0.5), (0.5, 0.5), (-0.1, 0.2)] {
            for force in [1.0, 10.0, 30.0, 45.0] {
                let resultat = tourner_chimiotropique(direction, gradient, force, &mut alea);
                assert!(resultat < NB_DIRECTIONS, "direction {direction}, force {force} : résultat {resultat}");
                let v = virage_chimiotropique(direction, gradient, force);
                // Le résultat est congru à direction + ⌊v⌋ ou direction + ⌊v⌋ + 1 (modulo 360).
                let bas = v.floor() as i64;
                let candidats =
                    [(direction as i64 + bas).rem_euclid(360), (direction as i64 + bas + 1).rem_euclid(360)];
                assert!(
                    candidats.contains(&(resultat as i64)),
                    "direction {direction}, force {force}, v = {v} : résultat {resultat}, attendu {candidats:?}"
                );
                if direction >= 350 && resultat <= 10 {
                    a_boucle_vers_le_haut = true;
                }
                if direction <= 10 && resultat >= 350 {
                    a_boucle_vers_le_bas = true;
                }
            }
        }
    }
    assert!(a_boucle_vers_le_haut && a_boucle_vers_le_bas, "le test n'a jamais traversé la frontière 0/359");
}

/// Le virage appliqué par un appel : `résultat − direction` ramené dans `-180..180`.
fn virage_applique(direction: usize, resultat: usize) -> i64 {
    (resultat as i64 - direction as i64 + 540).rem_euclid(360) - 180
}

/// 4. L'arrondi tiré au sort ne perd pas la partie fractionnaire : sur `N_TIRAGES` tirages, le virage appliqué vaut `⌊v⌋`
///    ou `⌊v⌋ + 1` (donc ne s'écarte jamais de `v` d'un degré ou plus), et sa moyenne vaut `v` à `TOLERANCE_MOYENNE`
///    près. Huit cas : virages positifs et négatifs, fractions de 0,05 à 0,95.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn le_virage_applique_vaut_le_virage_reel_en_moyenne() {
    // (direction, gradient, force)
    let cas: [(usize, (f64, f64), f64); 8] = [
        (0, (0.0, 0.01), 1.0),        // v = 0,5
        (0, (0.0, 0.05), 1.0),        // v ≈ 0,833
        (0, (0.0, 0.002), 1.0),       // v ≈ 0,167
        (0, (0.0, 0.01), 4.5),        // v = 2,25
        (90, (0.01, 0.0), 1.0),       // v = -0,5 : gradient à droite de la pointe
        (200, (-0.0433, 0.025), 2.0), // v ≈ -1,28 : gradient à 150°, pointe à 200°
        (355, (0.3, 0.3), 3.0),       // virage vers le haut, au passage de 360
        (5, (0.3, -0.3), 7.0),        // virage négatif de plusieurs degrés, au passage de 0
    ];
    for (numero, (direction, gradient, force)) in cas.into_iter().enumerate() {
        let v = virage_chimiotropique(direction, gradient, force);
        let bas = v.floor();
        let frac = v - bas;
        // Garde : la partie fractionnaire n'est ni nulle ni voisine de 1, sinon l'arrondi ne serait pas testé.
        assert!(frac > 0.04 && frac < 0.96, "cas {direction}, {gradient:?}, {force} : v = {v}, frac = {frac}");
        let mut alea = Alea::new(1000 + numero as u64);
        let mut somme = 0.0;
        for _ in 0..N_TIRAGES {
            let resultat = tourner_chimiotropique(direction, gradient, force, &mut alea);
            let applique = virage_applique(direction, resultat);
            assert!(
                applique == bas as i64 || applique == bas as i64 + 1,
                "v = {v} : virage appliqué {applique}, attendu {bas} ou {}",
                bas + 1.0
            );
            assert!((applique as f64 - v).abs() < 1.0, "v = {v} : virage appliqué {applique} à un degré ou plus");
            somme += applique as f64;
        }
        let moyenne = somme / f64::from(N_TIRAGES);
        assert!(
            proches(moyenne, v, TOLERANCE_MOYENNE),
            "cas {direction}, {gradient:?}, force {force} : moyenne {moyenne} au lieu de v = {v}"
        );
    }
}

/// 4. Un virage exactement nul (gradient aligné) reste nul : la direction ne change jamais, sur beaucoup de tirages
///    (la partie fractionnaire est nulle, aucun tirage ne peut faire monter).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_virage_nul_ne_change_jamais_la_direction() {
    let mut alea = Alea::new(5);
    for direction in [0usize, 90, 180, 270] {
        // Un gradient exactement aligné sur un axe : le virage vaut exactement 0.
        let gradient = match direction {
            0 => (0.05, 0.0),
            90 => (0.0, 0.05),
            180 => (-0.05, 0.0),
            _ => (0.0, -0.05),
        };
        assert_eq!(virage_chimiotropique(direction, gradient, 3.0), 0.0, "garde : le virage doit être exactement nul");
        for _ in 0..5000 {
            assert_eq!(tourner_chimiotropique(direction, gradient, 3.0, &mut alea), direction);
        }
    }
}

/// 4. Un virage négatif minuscule : `v ≈ −10⁻²⁰`, donc `⌊v⌋ = −1` et `v − ⌊v⌋` s'arrondit à `1.0` en flottant ; la règle
///    appliquée telle quelle (le tirage, dans `[0, 1)`, est toujours `< frac`) rend la direction inchangée, avec un seul
///    tirage. Un code qui oublie le `+ 1` des virages négatifs, ou qui compare `<= frac` à rebours, rend `direction − 1`.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_virage_negatif_minuscule_laisse_la_direction() {
    // Un gradient de norme 1 aligné sur la pointe (un axe), avec une composante de 10⁻²⁰ à sa droite.
    for (direction, gradient) in
        [(0usize, (1.0, -1e-20)), (90, (1e-20, 1.0)), (180, (-1.0, 1e-20)), (270, (-1e-20, -1.0))]
    {
        let v = virage_chimiotropique(direction, gradient, 1.0);
        assert!(v < 0.0 && v > -1e-15, "garde, direction {direction} : virage {v}, attendu négatif minuscule");
        for graine in 0..200u64 {
            let mut alea = Alea::new(graine);
            let obtenue = tourner_chimiotropique(direction, gradient, 1.0, &mut alea);
            assert_eq!(
                obtenue, direction,
                "direction {direction}, graine {graine}, v = {v} : obtenue {obtenue}, attendue {direction}"
            );
            assert!(
                a_tire_exactement(alea, graine, 1),
                "direction {direction}, graine {graine} : pas exactement un tirage"
            );
        }
    }
}
