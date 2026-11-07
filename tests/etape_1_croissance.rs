//! Tests d'acceptation de l'étape 1 (a) : le module `croissance`. Aucune fonction mathématique du système ici non
//! plus : seulement `+`, `-`, `*`, `/`, `sqrt` et les tables du module.

use mycelium_lab::croissance::{COS, Colonie, NB_DIRECTIONS, OCTETS_PAR_SEGMENT, Parametres, SIN};

/// Paramètres sans ramification : un seul apex, pour toujours.
fn sans_ramification() -> Parametres {
    Parametres { pas: 0.7, intervalle: f64::INFINITY, angle_ramification: 30, amplitude_marche: 15 }
}

/// Paramètres avec ramification fréquente.
fn avec_ramification(intervalle: f64) -> Parametres {
    Parametres { pas: 1.0, intervalle, angle_ramification: 30, amplitude_marche: 20 }
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn tables_de_directions_coherentes() {
    assert_eq!(NB_DIRECTIONS, 360);
    assert_eq!((COS[0], SIN[0]), (1.0, 0.0));
    assert_eq!((COS[90], SIN[90]), (0.0, 1.0));
    assert_eq!((COS[180], SIN[180]), (-1.0, 0.0));
    assert_eq!((COS[270], SIN[270]), (0.0, -1.0));
    for d in 0..NB_DIRECTIONS {
        let norme = (COS[d] * COS[d] + SIN[d] * SIN[d]).sqrt();
        assert!((norme - 1.0).abs() < 1e-15, "direction {d} : norme {norme}");
    }
}

/// 1. Sans ramification, la longueur d'hyphe d'un apex vaut `pas × nombre de pas`, à 1e-9 près en relatif.
#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn longueur_d_hyphe_egale_pas_fois_nombre_de_pas() {
    let parametres = sans_ramification();
    for graine in [1u64, 2, 3, 42, 1000] {
        let n = 1000u32;
        let mut colonie = Colonie::depuis_spore((5.0, -3.0), parametres, graine);
        colonie.avancer_n(n);

        assert_eq!(colonie.apex().len(), 1, "graine {graine} : un seul apex sans ramification");
        assert_eq!(colonie.nb_ramifications(), 0);
        assert_eq!(colonie.temps(), n);
        assert_eq!(colonie.segments().len(), n as usize);

        // Longueur recalculée dans le test, segment par segment, depuis les extrémités.
        let mut longueur = 0.0;
        for s in colonie.segments() {
            let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
            longueur += (dx * dx + dy * dy).sqrt();
        }
        let attendu = parametres.pas * f64::from(n);
        assert!(
            ((longueur - attendu) / attendu).abs() < 1e-9,
            "graine {graine} : longueur {longueur}, attendu {attendu}"
        );
        // Les segments se suivent : la fin de l'un est le début du suivant, et le dernier finit sur l'apex.
        for deux in colonie.segments().windows(2) {
            assert_eq!((deux[0].x1, deux[0].y1), (deux[1].x0, deux[1].y0));
        }
        let apex = colonie.apex()[0];
        let dernier = colonie.segments().last().unwrap();
        assert_eq!((dernier.x1, dernier.y1), (apex.x, apex.y));
    }
}

/// 2. L'intervalle moyen entre deux ramifications, mesuré sur assez de pas et de graines, est à 5 % de `intervalle`.
///
/// Mesure : chaque apex vivant tente une ramification à chaque pas ; l'intervalle moyen est donc le nombre de
/// « apex-pas » divisé par le nombre de ramifications. Avec 300 graines de 50 pas à l'intervalle 10, on compte de
/// l'ordre de 4×10⁵ apex-pas et 4×10⁴ ramifications : l'écart-type relatif de la mesure est de l'ordre de
/// `1/sqrt(4×10⁴)`, soit 0,5 %, dix fois sous la tolérance de 5 %.
#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn intervalle_moyen_de_ramification() {
    let intervalle = 10.0;
    let parametres = avec_ramification(intervalle);
    let mut apex_pas: u64 = 0;
    let mut ramifications: u64 = 0;
    for graine in 0..300u64 {
        let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
        for _ in 0..50 {
            apex_pas += colonie.apex().len() as u64;
            colonie.avancer();
        }
        assert_eq!(colonie.apex().len() as u64, colonie.nb_ramifications() + 1);
        ramifications += colonie.nb_ramifications();
    }
    assert!(ramifications > 10_000, "trop peu de ramifications pour mesurer : {ramifications}");
    let mesure = apex_pas as f64 / ramifications as f64;
    assert!((mesure - intervalle).abs() <= 0.05 * intervalle, "intervalle mesuré {mesure}, attendu {intervalle}");
}

/// 3. Isotropie : sur des colonies parties d'une spore au centre, la direction moyenne des apex est proche de zéro.
///
/// Parmi 200 colonies, on garde les ramifiées. Pour chacune, on prend le vecteur moyen des directions unitaires de
/// ses apex, puis la moyenne de ces vecteurs sur les graines. La direction initiale de chaque colonie est uniforme, et
/// le modèle est invariant par rotation : le vecteur moyen d'une colonie est d'espérance nulle, de norme au plus 1,
/// donc de variance totale `E|m|² <= 1`. Les colonies sont indépendantes (les apex d'une même colonie, eux, sont corrélés : on ne compte pas
/// sur eux). La moyenne `M` de `K` vecteurs indépendants a donc `E|M|² <= 1/K`. On attend `K ≈ 190` colonies ramifiées
/// sur 200, soit une norme de l'ordre de 0,07 ; pour une loi proche de la gaussienne, `P(|M| > t) ≈ exp(-t²·K)`. Le
/// seuil `t = 0,25` donne `exp(-11,9) ≈ 7×10⁻⁶` : un modèle isotrope ne le dépasse pas, un biais sensible (une
/// direction préférée) oui.
///
/// Une colonie peut ne pas se ramifier du tout : à l'intervalle 10, un apex se ramifie à chaque pas avec la
/// probabilité 0,1, donc 30 pas passent sans ramification avec la probabilité `0,9³⁰ ≈ 4,2 %` (environ 8 colonies sur
/// 200). Ces colonies, réduites à leur apex initial, ne comptent pas dans la moyenne : la ramification ne dépend pas
/// de la direction, donc les écarter ne biaise rien. Leur part est mesurée à part, par le test suivant.
#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn isotropie_de_la_direction_moyenne() {
    let parametres = avec_ramification(10.0);
    let k = 200usize;
    let (mut somme_x, mut somme_y) = (0.0, 0.0);
    let mut ramifiees = 0usize;
    for graine in 0..k as u64 {
        let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
        colonie.avancer_n(30);
        if colonie.apex().len() == 1 {
            continue;
        }
        ramifiees += 1;
        let (mut mx, mut my) = (0.0, 0.0);
        for a in colonie.apex() {
            assert!(a.direction < NB_DIRECTIONS);
            mx += COS[a.direction];
            my += SIN[a.direction];
        }
        let n = colonie.apex().len() as f64;
        somme_x += mx / n;
        somme_y += my / n;
    }
    assert!(ramifiees >= 170, "trop peu de colonies ramifiées pour mesurer : {ramifiees} sur {k}");
    let (mx, my) = (somme_x / ramifiees as f64, somme_y / ramifiees as f64);
    let norme = (mx * mx + my * my).sqrt();
    assert!(norme < 0.25, "direction moyenne ({mx}, {my}), norme {norme} ({ramifiees} colonies ramifiées)");
}

/// 3 bis. La part des colonies sans ramification : après 30 pas à l'intervalle 10, une colonie reste réduite à son
/// apex initial avec la probabilité `0,9³⁰ ≈ 4,2 %` (chaque pas ramifie l'apex avec la probabilité `1/10`).
///
/// Mesure sur 2000 graines : l'écart-type de la part mesurée est `sqrt(0,042 × 0,958 / 2000) ≈ 0,0045`, soit une
/// tolérance de 0,02 à plus de quatre écarts-types. Le modèle ne force pas la première ramification : ni plus tôt, ni à
/// une date butoir.
#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn part_des_colonies_sans_ramification() {
    let intervalle = 10.0;
    let parametres = avec_ramification(intervalle);
    let (graines, pas) = (2000u64, 30u32);
    let mut sans_ramification = 0u64;
    for graine in 0..graines {
        let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
        colonie.avancer_n(pas);
        if colonie.apex().len() == 1 {
            sans_ramification += 1;
        }
    }
    let mut attendu = 1.0;
    for _ in 0..pas {
        attendu *= 1.0 - 1.0 / intervalle;
    }
    let mesure = sans_ramification as f64 / graines as f64;
    assert!(
        (mesure - attendu).abs() <= 0.02,
        "part sans ramification {mesure} ({sans_ramification} sur {graines}), attendue {attendu}"
    );
}

/// 4. Même graine, même colonie à l'octet ; deux graines, deux colonies.
#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn meme_graine_meme_colonie_deux_graines_deux_colonies() {
    let parametres = avec_ramification(8.0);
    let jouer = |graine: u64| {
        let mut colonie = Colonie::depuis_spore((1.5, -2.5), parametres, graine);
        colonie.avancer_n(60);
        colonie
    };
    let a = jouer(7);
    let b = jouer(7);
    let c = jouer(8);
    assert!(a.segments().len() > 60, "la colonie doit s'être ramifiée");
    assert_eq!(a.octets().len(), OCTETS_PAR_SEGMENT * a.segments().len());
    assert_eq!(a.octets(), b.octets());
    assert_ne!(a.octets(), c.octets());
    // L'écriture d'un segment est celle qu'annonce la doc.
    let s = a.segments()[0];
    let mut attendu = Vec::new();
    attendu.extend_from_slice(&s.apex.to_le_bytes());
    attendu.extend_from_slice(&s.temps.to_le_bytes());
    for v in [s.x0, s.y0, s.x1, s.y1] {
        attendu.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(s.octets().to_vec(), attendu);
    assert_eq!(&a.octets()[..OCTETS_PAR_SEGMENT], attendu.as_slice());
}

/// `avancer_n(n)` donne le même état que `n` appels à `avancer`.
#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn avancer_n_equivaut_a_n_avancer() {
    let parametres = avec_ramification(6.0);
    let mut a = Colonie::depuis_spore((0.0, 0.0), parametres, 99);
    let mut b = Colonie::depuis_spore((0.0, 0.0), parametres, 99);
    a.avancer_n(40);
    for _ in 0..40 {
        b.avancer();
    }
    assert_eq!(a.octets(), b.octets());
    assert_eq!(a.apex(), b.apex());
    assert_eq!(a.temps(), 40);
}
