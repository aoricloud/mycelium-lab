//! Étape 17 (M4) : des colonies faites à la main, à réponse exacte. `N` hyphes parallèles (`K = N EA/L`,
//! `ρ = N L/surface` : la pente 1, la seule puissance exacte de l'étape), un Z, son miroir et une branche libre (une
//! fraction calculée à part), et le repère du nid d'abeille (une pente mesurée, près de 3 : un repère, pas une cible).

#[path = "../examples/nuage_raideur.rs"]
#[allow(dead_code)]
mod exemple;
#[path = "etape_17/outils.rs"]
mod outils;

use mycelium_lab::mesures::logarithme;
use mycelium_lab::nuage::ajuster;
use mycelium_lab::reseau::Hyphe;
use outils::{HYPHE, cavite, egal, miroir, nid_d_abeilles, paralleles, point, segment};

/// Les raideurs et les densités exactes : l'erreur relative mesurée est de 2e-14 au plus (le miroir du Z et de sa
/// branche), une marge de 50. Une règle fausse fait plus de 1e-3.
const TOLERANCE: f64 = 1e-12;
/// La droite des hyphes parallèles (pente, ordonnée, `R²`) : l'erreur mesurée est de 4e-16 au plus, une marge de 2 500.
const TOLERANCE_DROITE: f64 = 1e-12;

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn n_hyphes_paralleles_ont_k_egal_n_ea_sur_l_rho_egal_n_l_sur_la_surface_et_la_pente_1() {
    for (largeur, hauteur) in [(30, 10), (12, 7)] {
        let (c, l, surface) = (cavite(largeur, hauteur), largeur as f64 - 1.0, (largeur * hauteur) as f64);
        let mut paires = Vec::new();
        for n in 1..=5 {
            let p = point(&c, &paralleles(n, largeur), HYPHE);
            assert!(
                egal(p.raideur, n as f64 * 5.0 / l, TOLERANCE),
                "{largeur} × {hauteur}, N = {n} : K = {}",
                p.raideur
            );
            assert!(
                egal(p.densite, n as f64 * l / surface, TOLERANCE),
                "{largeur} × {hauteur}, N = {n} : ρ = {}",
                p.densite
            );
            assert_eq!(p.noeuds, 2 * n);
            paires.push((p.densite, p.raideur));
        }
        // `K/ρ = EA × surface / L²` pour tout `N` : la droite est de pente 1, et son ordonnée est ce logarithme.
        let droite = ajuster(&paires).unwrap();
        assert!((droite.pente - 1.0).abs() < TOLERANCE_DROITE, "pente {}", droite.pente);
        let ordonnee = logarithme(5.0 * surface / (l * l));
        assert!((droite.ordonnee - ordonnee).abs() < TOLERANCE_DROITE, "ordonnée {}", droite.ordonnee);
        assert!((droite.r2 - 1.0).abs() < TOLERANCE_DROITE && droite.points == 5, "R² {}", droite.r2);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn un_z_son_miroir_et_une_branche_libre_ont_la_raideur_exacte_et_la_densite_de_leur_longueur() {
    // Un Z dans une boîte de 8 × 6 : une traverse de 2,5, un montant de 3 qui fléchit, une traverse de 4,5 (rien de
    // symétrique). Avec `EA = 5` et `EI = 3`, `K = 155920/601511` (calculé à part, en fractions exactes). La branche
    // libre, longue de 1, prolonge le montant vers le haut de la boîte : elle ne touche aucun mors et ne porte rien.
    let z = [segment(0.5, 1.5, 3.0, 1.5), segment(3.0, 1.5, 3.0, 4.5), segment(3.0, 4.5, 7.5, 4.5)];
    let avec_branche = [&z[..], &[segment(3.0, 1.5, 3.0, 0.5)]].concat();
    assert_eq!((miroir(&z, 8)[0].x0, miroir(&z, 8)[0].x1), (7.5, 5.0), "le miroir a sa traverse courte à droite");
    let cas = [
        ("le Z", z.to_vec(), 10.0, 4),
        ("son miroir", miroir(&z, 8), 10.0, 4),
        ("le Z et sa branche", avec_branche.clone(), 11.0, 5),
        ("leur miroir", miroir(&avec_branche, 8), 11.0, 5),
    ];
    for (nom, segments, longueur, noeuds) in cas {
        let p = point(&cavite(8, 6), &segments, HYPHE);
        assert!(egal(p.raideur, 155920.0 / 601511.0, TOLERANCE), "{nom} : K = {}", p.raideur);
        assert!(egal(p.densite, longueur / 48.0, TOLERANCE), "{nom} : ρ = {}", p.densite);
        assert_eq!(p.noeuds, noeuds, "{nom}");
    }
}

/// Des parois minces (`EA` = 1 000, `EI` = 1 : une paroi pleine d'épaisseur `t` a `EI/EA = t²/12`, soit `t` = 0,11 case) :
/// le nid d'abeille est alors dominé par la flexion, l'hypothèse du repère.
const MINCE: Hyphe = Hyphe { ea: 1000.0, ei: 1.0 };
/// Les trois panneaux du repère : (demi-périodes d'un mors à l'autre, rangées), dans une boîte de 41 × 34 cases. Le côté
/// des hexagones vaut 5,8 puis 2,9 puis 1,9 cases.
const PANNEAUX: [(usize, usize); 3] = [(8, 4), (16, 8), (24, 12)];
/// La pente et le `R²` **mesurés** sur ces trois panneaux : une non-régression. Le repère publié est 3 (un nid d'abeille
/// infini aux parois minces, MIT 3.054) ; la mesure est à 1,4 % au-dessus, et rien n'est réglé pour l'en approcher.
const PENTE_DU_NID: f64 = 3.042062298467831;
const R2_DU_NID: f64 = 0.9999928569085136;
/// Les mêmes panneaux construits nœud par nœud, sans l'extraction, rendent ces deux nombres à 1e-13 près : une marge
/// de 10 000.
const TOLERANCE_DU_NID: f64 = 1e-9;

#[test]
#[cfg_attr(not(feature = "etape-17"), ignore)]
fn le_nid_d_abeilles_aux_parois_minces_a_sa_pente_mesuree_pres_du_repere_3_qui_n_est_pas_une_cible() {
    let mut paires = Vec::new();
    for (demi_periodes, rangees) in PANNEAUX {
        let (segments, hauteur) = nid_d_abeilles(demi_periodes, 41, rangees);
        let p = point(&cavite(41, 34), &segments, MINCE);
        assert_eq!(p.noeuds, rangees * (demi_periodes + 1), "{demi_periodes} demi-périodes");
        // Le repère parle d'un matériau : son module contre sa densité. On divise donc la raideur et la densité par la
        // hauteur du panneau, qui change d'un panneau à l'autre (un panneau plus haut, c'est plus d'hyphes en parallèle :
        // la puissance 1, pas la puissance 3).
        paires.push((p.densite / hauteur, p.raideur / hauteur));
    }
    let droite = ajuster(&paires).unwrap();
    assert!((droite.pente - PENTE_DU_NID).abs() < TOLERANCE_DU_NID, "pente {}", droite.pente);
    assert!((droite.r2 - R2_DU_NID).abs() < TOLERANCE_DU_NID, "R² {}", droite.r2);
}
