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

/// Les segments longs des tests D : trois horizontales d'un bord à l'autre, chacune à un étage différent.
fn segments_longs() -> Vec<Segment> {
    vec![
        Segment { apex: 0, temps: 0, x0: 0.0, y0: 1.0, x1: 10.0, y1: 1.0 },
        Segment { apex: 1, temps: 0, x0: 0.0, y0: 5.0, x1: 10.0, y1: 5.0 },
        Segment { apex: 2, temps: 0, x0: 0.0, y0: 9.0, x1: 10.0, y1: 9.0 },
    ]
}

/// D. Un segment long se trouve depuis chaque case qu'il traverse, pas seulement celle de son départ.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn un_segment_long_se_trouve_dans_chaque_case_qu_il_traverse() {
    let mut index = Index::new(10, 10);
    for segment in segments_longs() {
        index.ajouter(segment);
    }

    let reglage = Reglage { rayon: 0.75, delai: 0 };
    // Le segment (0, 1)-(10, 1) coupe les dix cases de la ligne y = 1. Le point (x + 0,5, 1,5) est à 0,5 de ce
    // segment et à plus de 3 des deux autres : c'est toujours le numéro 0 qui gagne, quelle que soit la case.
    for i in 0..10usize {
        let x = i as f64 + 0.5;
        assert_eq!(index.chercher(x, 1.5, 3, 0, &reglage), Some(0), "case {i}");
    }
}

/// D (suite). Avec des segments longs, `chercher` rend exactement le parcours exhaustif, départage compris.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_index_ne_ment_pas_avec_des_segments_longs() {
    let mut index = Index::new(10, 10);
    let mut segments = segments_longs();
    // Un point proche d'une extrémité et un autre au milieu : le plus proche gagne, puis le plus petit numéro.
    segments.push(Segment { apex: 3, temps: 0, x0: 9.5, y0: 1.0, x1: 9.5, y1: 2.0 });
    segments.push(Segment { apex: 4, temps: 0, x0: 4.5, y0: 5.0, x1: 4.5, y1: 6.0 });
    for segment in &segments {
        index.ajouter(*segment);
    }

    let reglage = Reglage { rayon: 1.0, delai: 0 };
    for i in 0..100usize {
        let x = (i % 10) as f64 + 0.5;
        let y = (i / 10) as f64 + 0.5;
        let apex = (i % 6) as u32;
        let attendu = chercher_exhaustif(&segments, x, y, apex, 0, &reglage);
        let trouve = index.chercher(x, y, apex, 0, &reglage);
        assert_eq!(trouve, attendu, "requête {i} : ({x}, {y}), apex {apex}");
    }
}

// ------------------------------------------------------------------------------------------------------------------
// Tests ajoutés par le juge des tests (voir l'issue #14) : ils ferment des trous où un index faux passait tous les
// tests ci-dessus (rayon plus petit que les segments sur les deux axes et les diagonales, résultats `None`, rayon
// lu loin du point).
// ------------------------------------------------------------------------------------------------------------------

/// 400 segments longs (2,4 à 3), orientés de quatre façons (horizontal, vertical, deux diagonales de pente 4/3 et
/// -4/3), sur une grille de pas 4,8 décalée d'un rang à l'autre, dans la boîte 100 × 100. Le milieu d'un segment est à
/// plus de 2,1 de tout autre segment, et à au moins 1,2 de ses propres extrémités. Les apex (9 valeurs) et les temps
/// (13 valeurs) varient.
fn segments_longs_en_grille() -> Vec<Segment> {
    let mut segments = Vec::new();
    for j in 0..20usize {
        for i in 0..20usize {
            let k = j * 20 + i;
            let longueur = 2.4 + 0.2 * ((i + 2 * j) % 4) as f64;
            let (dx, dy) = match (i + j) % 4 {
                0 => (longueur, 0.0),
                1 => (0.0, longueur),
                2 => (0.6 * longueur, 0.8 * longueur),
                _ => (0.6 * longueur, -0.8 * longueur),
            };
            let cx = 4.8 * i as f64 + 3.0 + 0.37 * (i % 3) as f64;
            let cy = 4.8 * j as f64 + 3.0 + 0.29 * (j % 5) as f64;
            segments.push(Segment {
                apex: (k % 9) as u32,
                temps: (k % 13) as u32,
                x0: cx - dx / 2.0,
                y0: cy - dy / 2.0,
                x1: cx + dx / 2.0,
                y1: cy + dy / 2.0,
            });
        }
    }
    segments
}

/// D. L'index ne ment pas non plus quand le rayon est plus petit que les segments, comme dans la simulation (rayon
///    0,5, segments de 1). Les requêtes sont posées au MILIEU des segments, hors de portée des deux extrémités (à 1,2
///    ou plus, pour un rayon de 0,5 ou de 1,0) : seul le corps du segment est à portée, donc un index qui ne range un
///    segment que près de son début ou de sa fin le manque. Quatre sortes de requêtes, à parts égales : un apex
///    étranger et un segment assez ancien (le segment est trouvé), le même apex (rien : zone dense, mais le seul segment
///    à portée est ignoré), un segment trop jeune d'un pas (rien), un segment d'exactement l'âge du délai (trouvé). Le
///    résultat est contrôlé deux fois : contre la valeur attendue ci-dessus, et contre le parcours exhaustif.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_index_ne_ment_pas_a_petit_rayon() {
    let segments = segments_longs_en_grille();
    let mut index = Index::new(100, 100);
    for (numero, segment) in segments.iter().enumerate() {
        assert_eq!(index.ajouter(*segment), numero);
    }

    let delai = 5;
    for rayon in [0.5, 1.0] {
        let reglage = Reglage { rayon, delai };
        let (mut trouves, mut rien) = (0, 0);
        for (k, segment) in segments.iter().enumerate() {
            let (x, y) = ((segment.x0 + segment.x1) / 2.0, (segment.y0 + segment.y1) / 2.0);
            let (apex, temps, attendu) = match k % 4 {
                0 => (segment.apex + 1, 60, Some(k)),
                1 => (segment.apex, 60, None),
                2 => (segment.apex + 1, segment.temps + delai - 1, None),
                _ => (segment.apex + 1, segment.temps + delai, Some(k)),
            };
            let trouve = index.chercher(x, y, apex, temps, &reglage);
            let exhaustif = chercher_exhaustif(&segments, x, y, apex, temps, &reglage);
            assert_eq!(trouve, attendu, "rayon {rayon}, requête {k} : ({x}, {y}), apex {apex}, temps {temps}");
            assert_eq!(trouve, exhaustif, "rayon {rayon}, requête {k} : l'index diffère du parcours exhaustif");
            if trouve.is_some() {
                trouves += 1;
            } else {
                rien += 1;
            }
        }
        assert_eq!((trouves, rien), (200, 200), "rayon {rayon}");
    }
}

/// D (suite). Le rayon est vraiment lu : un segment à 9 (le bout `(41, 50)` depuis `(50, 50)`) est trouvé à partir du
///    rayon 9 et pas avant, loin de la case du point ; le plus proche de deux segments gagne même quand le plus
///    lointain est aussi à portée ; un rayon trop court pour les deux ne rend rien.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chercher_lit_le_rayon_loin_du_point() {
    let mut index = Index::new(100, 100);
    index.ajouter(Segment { apex: 1, temps: 0, x0: 40.0, y0: 50.0, x1: 41.0, y1: 50.0 });
    index.ajouter(Segment { apex: 1, temps: 0, x0: 10.0, y0: 10.0, x1: 11.0, y1: 10.0 });
    let avec = |rayon: f64| Reglage { rayon, delai: 0 };

    assert_eq!(index.chercher(50.0, 50.0, 0, 0, &avec(9.0)), Some(0));
    assert_eq!(index.chercher(50.0, 50.0, 0, 0, &avec(8.999)), None);
    assert_eq!(index.chercher(50.0, 50.0, 0, 0, &avec(1000.0)), Some(0));

    // Depuis le coin (100, 100) : le premier segment est à sqrt(59² + 50²) ≈ 77,3, le second à plus de 126.
    assert_eq!(index.chercher(100.0, 100.0, 0, 0, &avec(77.0)), None);
    assert_eq!(index.chercher(100.0, 100.0, 0, 0, &avec(78.0)), Some(0));
    assert_eq!(index.chercher(100.0, 100.0, 0, 0, &avec(150.0)), Some(0));
    // Depuis le coin (0, 0) : le second segment, à sqrt(10² + 10²) ≈ 14,1, est le plus proche.
    assert_eq!(index.chercher(0.0, 0.0, 0, 0, &avec(14.0)), None);
    assert_eq!(index.chercher(0.0, 0.0, 0, 0, &avec(14.2)), Some(1));
    assert_eq!(index.chercher(0.0, 0.0, 0, 0, &avec(150.0)), Some(1));
}
