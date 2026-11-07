//! Tests d'acceptation de l'étape 2 (b) : le module `anastomose`.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt`.

use mycelium_lab::anastomose::{Index, Reglage, distance_au_segment};
use mycelium_lab::croissance::Segment;

/// Un segment réduit à un point, pratique pour des distances connues d'avance.
fn point(apex: u32, temps: u32, x: f64, y: f64) -> Segment {
    Segment { apex, temps, x0: x, y0: y, x1: x, y1: y }
}

/// Le parcours exhaustif de référence : le contrat de `Index::chercher`, réécrit à la main.
fn chercher_exhaustif(segments: &[Segment], x: f64, y: f64, apex: u32, temps: u32, reglage: &Reglage) -> Option<usize> {
    let mut meilleur: Option<(usize, f64)> = None;
    for (numero, segment) in segments.iter().enumerate() {
        if segment.apex == apex {
            continue;
        }
        if u64::from(temps) < u64::from(segment.temps) + u64::from(reglage.delai) {
            continue;
        }
        let distance = distance_au_segment(x, y, segment);
        if distance > reglage.rayon {
            continue;
        }
        match meilleur {
            None => meilleur = Some((numero, distance)),
            Some((_, d)) if distance < d => meilleur = Some((numero, distance)),
            _ => {}
        }
    }
    meilleur.map(|(numero, _)| numero)
}

/// 1. `distance_au_segment` mesure au segment, pas à ses extrémités, avec une précision de 1e-12.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn distance_au_segment_au_milieu_et_au_bout() {
    let segment = Segment { apex: 0, temps: 1, x0: 0.0, y0: 0.0, x1: 4.0, y1: 0.0 };
    let precision = 1e-12;

    // (2, 3) est à la verticale du milieu : la distance est 3, pas celle au bout (sqrt(4 + 9) ≈ 3,606).
    let au_milieu = distance_au_segment(2.0, 3.0, &segment);
    assert!((au_milieu - 3.0).abs() < precision, "au milieu : {au_milieu}");

    // (7, 4) est au-delà du bout (4, 0) : la distance est celle au bout, sqrt(3² + 4²) = 5.
    let au_bout = distance_au_segment(7.0, 4.0, &segment);
    assert!((au_bout - 5.0).abs() < precision, "au bout : {au_bout}");

    // Un point du segment est à distance nulle ; les extrémités aussi.
    for (x, y) in [(1.5, 0.0), (0.0, 0.0), (4.0, 0.0)] {
        let distance = distance_au_segment(x, y, &segment);
        assert!(distance.abs() < precision, "({x}, {y}) sur le segment : {distance}");
    }
}

/// 1 (suite). Un segment réduit à un point donne la distance à ce point, sans division par zéro.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn distance_au_segment_reduit_a_un_point() {
    let segment = point(3, 2, 2.0, 2.0);
    let precision = 1e-12;
    assert!((distance_au_segment(2.0, 3.0, &segment) - 1.0).abs() < precision);
    assert!((distance_au_segment(5.0, 6.0, &segment) - 5.0).abs() < precision);
    assert!(distance_au_segment(2.0, 2.0, &segment).abs() < precision);
}

/// 1 (suite). La précision de 1e-12 tient aussi sur une projection oblique.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn distance_au_segment_precision_oblique() {
    let segment = Segment { apex: 0, temps: 0, x0: 0.0, y0: 0.0, x1: 3.0, y1: 4.0 };
    let precision = 1e-12;

    // Projection de (1, 1) sur le segment : (0,84, 1,12), donc une distance de 0,2.
    let distance = distance_au_segment(1.0, 1.0, &segment);
    assert!((distance - 0.2).abs() < precision, "oblique : {distance}");

    // Depuis (-3, -4), le point le plus proche est l'extrémité (0, 0), à 5.
    let derriere = distance_au_segment(-3.0, -4.0, &segment);
    assert!((derriere - 5.0).abs() < precision, "derrière : {derriere}");

    // Sur le segment diagonal (0,0)-(1,1), (0, 1) est à sqrt(0,5) de la droite.
    let diagonal = Segment { apex: 0, temps: 0, x0: 0.0, y0: 0.0, x1: 1.0, y1: 1.0 };
    let milieu = distance_au_segment(0.0, 1.0, &diagonal);
    assert!((milieu - 0.5_f64.sqrt()).abs() < precision, "milieu oblique : {milieu}");
}

/// 2. `chercher` trouve un segment étranger, assez ancien et assez proche.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chercher_trouve_un_segment_etranger_assez_ancien_et_proche() {
    let mut index = Index::new(10, 10);
    assert_eq!(index.ajouter(point(7, 3, 5.0, 6.0)), 0);

    // À l'instant 6, le segment déposé à 3 a exactement l'âge du délai 3 : il est pris.
    let reglage = Reglage { rayon: 2.0, delai: 3 };
    assert_eq!(index.chercher(5.0, 5.0, 0, 6, &reglage), Some(0));
}

/// 2 (suite). Rien au-delà du rayon ; la distance égale au rayon compte comme dedans.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chercher_respecte_le_rayon_bornes_comprises() {
    let mut index = Index::new(10, 10);
    index.ajouter(point(1, 0, 5.0, 7.0)); // distance 2 du point (5, 5)

    let exact = Reglage { rayon: 2.0, delai: 0 };
    assert_eq!(index.chercher(5.0, 5.0, 0, 0, &exact), Some(0));

    let juste_trop_petit = Reglage { rayon: 1.999_999, delai: 0 };
    assert_eq!(index.chercher(5.0, 5.0, 0, 0, &juste_trop_petit), None);

    let mut loin = Index::new(10, 10);
    loin.ajouter(point(1, 0, 5.0, 7.001)); // distance 2,001 > 2
    assert_eq!(loin.chercher(5.0, 5.0, 0, 0, &exact), None);
}

/// 2 (suite). Les segments du même apex sont ignorés, même les plus proches.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chercher_ignore_les_segments_du_meme_apex() {
    let mut index = Index::new(10, 10);
    index.ajouter(point(4, 0, 5.0, 5.5)); // distance 0,5, même apex que la requête
    index.ajouter(point(5, 0, 5.0, 7.0)); // distance 2, autre apex

    let reglage = Reglage { rayon: 5.0, delai: 0 };
    assert_eq!(index.chercher(5.0, 5.0, 4, 0, &reglage), Some(1));
    assert_eq!(index.chercher(5.0, 5.0, 6, 0, &reglage), Some(0));
}

/// 2 (suite). Les segments trop jeunes sont ignorés : au temps `t`, il faut `temps courant >= t + délai`.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chercher_ignore_les_segments_trop_jeunes() {
    let mut index = Index::new(10, 10);
    index.ajouter(point(1, 5, 5.0, 5.5)); // proche mais déposé au temps 5
    index.ajouter(point(2, 0, 5.0, 7.0)); // plus loin mais déposé au temps 0

    let reglage = Reglage { rayon: 5.0, delai: 5 };
    // À l'instant 9, le premier n'a que 4 pas : rejeté ; le second en a 9 : candidat.
    assert_eq!(index.chercher(5.0, 5.0, 0, 9, &reglage), Some(1));
    // À l'instant 10, le premier atteint l'âge 5 : candidat, et plus proche.
    assert_eq!(index.chercher(5.0, 5.0, 0, 10, &reglage), Some(0));
}

/// 2 (suite). Le plus proche gagne, puis à distance égale le plus petit numéro.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chercher_choisit_le_plus_proche_puis_le_plus_petit_numero() {
    let mut index = Index::new(10, 10);
    index.ajouter(point(1, 0, 5.0, 8.0)); // distance 3, numéro 0
    index.ajouter(point(1, 0, 8.0, 5.0)); // distance 3, numéro 1
    index.ajouter(point(1, 0, 5.0, 6.0)); // distance 1, numéro 2

    let reglage = Reglage { rayon: 5.0, delai: 0 };
    assert_eq!(index.chercher(5.0, 5.0, 0, 0, &reglage), Some(2));

    let mut egalite = Index::new(10, 10);
    egalite.ajouter(point(1, 0, 5.0, 8.0)); // distance 3, numéro 0
    egalite.ajouter(point(1, 0, 5.0, 6.0)); // distance 1, numéro 1
    egalite.ajouter(point(1, 0, 6.0, 5.0)); // distance 1, numéro 2
    assert_eq!(egalite.chercher(5.0, 5.0, 0, 0, &reglage), Some(1));
}

/// 3. L'index ne ment pas : sur 2000 segments en grille et 200 requêtes, `chercher` rend exactement ce que rend le
///    parcours exhaustif écrit ici, avec la même éligibilité et le même départage.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_index_ne_ment_pas() {
    const LARGEUR: usize = 100;
    const HAUTEUR: usize = 100;

    let mut index = Index::new(LARGEUR, HAUTEUR);
    let mut segments = Vec::new();

    // 50 × 40 = 2000 segments. Les lignes impaires sont décalées d'une unité : la grille n'est pas alignée.
    for j in 0..40usize {
        for i in 0..50usize {
            // x0 va de 0 à 99, x1 de 1 à 100 ; y0 va de 0 à 97,5, y1 de 2,5 à 100 : les quatre bords sont touchés.
            let x0 = i as f64 * 2.0 + (j % 2) as f64;
            let y0 = j as f64 * 2.5;
            let segment = Segment {
                apex: ((i * 7 + j * 3) % 17) as u32,
                temps: ((i + 3 * j) % 40) as u32,
                x0,
                y0,
                x1: x0 + 1.0,
                y1: y0 + 2.5,
            };
            assert_eq!(index.ajouter(segment), segments.len());
            segments.push(segment);
        }
    }
    assert_eq!(segments.len(), 2000);
    assert_eq!(index.segments(), segments.as_slice());

    let reglage = Reglage { rayon: 12.0, delai: 3 };
    let mut trouves = 0usize;
    // 20 × 10 = 200 points de requête, dont les quatre coins et les quatre bords.
    for k in 0..200usize {
        let x = (k % 20) as f64 * (100.0 / 19.0);
        let y = (k / 20) as f64 * (100.0 / 9.0);
        let apex = ((k * 5) % 17) as u32;
        let temps = 40 + (k % 8) as u32;
        let attendu = chercher_exhaustif(&segments, x, y, apex, temps, &reglage);
        let trouve = index.chercher(x, y, apex, temps, &reglage);
        assert_eq!(trouve, attendu, "requête {k} : ({x}, {y}), apex {apex}, temps {temps}");
        if trouve.is_some() {
            trouves += 1;
        }
    }
    assert!(trouves > 0, "aucune requête n'a trouvé de segment : le test ne prouverait rien");
}

/// 4. `ajouter` rend 0, 1, 2, ... et `segments()` restitue les segments dans cet ordre, bords compris.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn ajouter_numerote_et_segments_restitue() {
    let mut index = Index::new(10, 10);
    let segments = [
        Segment { apex: 2, temps: 1, x0: 0.0, y0: 0.0, x1: 1.0, y1: 2.0 },
        Segment { apex: 0, temps: 3, x0: 4.0, y0: 5.0, x1: 10.0, y1: 10.0 },
        Segment { apex: 9, temps: 7, x0: 2.5, y0: 2.5, x1: 2.5, y1: 2.5 },
    ];
    for (numero, segment) in segments.iter().enumerate() {
        assert_eq!(index.ajouter(*segment), numero);
    }
    assert_eq!(index.segments(), &segments[..]);
}

/// 4 (suite). `ajouter` panique si une extrémité sort par la gauche.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn ajouter_panique_si_x_sort_par_la_gauche() {
    let mut index = Index::new(10, 10);
    index.ajouter(Segment { apex: 0, temps: 0, x0: -0.001, y0: 5.0, x1: 1.0, y1: 5.0 });
}

/// 4 (suite). `ajouter` panique si une extrémité sort par la droite.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn ajouter_panique_si_x_sort_par_la_droite() {
    let mut index = Index::new(10, 10);
    index.ajouter(Segment { apex: 0, temps: 0, x0: 1.0, y0: 5.0, x1: 10.001, y1: 5.0 });
}

/// 4 (suite). `ajouter` panique si une extrémité sort par le bas.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn ajouter_panique_si_y_sort_par_le_bas() {
    let mut index = Index::new(10, 10);
    index.ajouter(Segment { apex: 0, temps: 0, x0: 5.0, y0: -0.001, x1: 5.0, y1: 1.0 });
}

/// 4 (suite). `ajouter` panique si une extrémité sort par le haut.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn ajouter_panique_si_y_sort_par_le_haut() {
    let mut index = Index::new(10, 10);
    index.ajouter(Segment { apex: 0, temps: 0, x0: 5.0, y0: 1.0, x1: 5.0, y1: 10.001 });
}
