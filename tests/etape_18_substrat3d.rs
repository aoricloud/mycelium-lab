//! Tests d'acceptation de l'étape 18 : module `substrat3d`.

use mycelium_lab::substrat::Substrat;
use mycelium_lab::substrat3d::{ErreurSubstrat3d, Substrat3d};

/// Variance du champ le long d'un axe, en unités physiques, autour de la case `centre`.
fn variance(s: &Substrat3d, axe: usize, centre: usize, dx: f64) -> f64 {
    let (mut m, mut m2) = (0.0, 0.0);
    for z in 0..s.profondeur() {
        for y in 0..s.hauteur() {
            for x in 0..s.largeur() {
                let c = match axe {
                    0 => x,
                    1 => y,
                    _ => z,
                };
                let pos = (c as f64 - centre as f64) * dx;
                let v = s.valeur(x, y, z);
                m += v;
                m2 += v * pos * pos;
            }
        }
    }
    m2 / m
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn construction_refuse_les_parametres_instables_ou_invalides() {
    assert!(Substrat3d::nouveau(8, 8, 8, 1.0, 1.0 / 6.0, 1.0).is_ok());
    assert!(Substrat3d::nouveau(8, 8, 8, 1.0 / 6.0, 1.0, 1.0).is_ok());
    assert_eq!(Substrat3d::nouveau(8, 8, 8, 1.0, 0.17, 1.0).err(), Some(ErreurSubstrat3d::PasInstable));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, 0.25, 1.0, 1.0).err(), Some(ErreurSubstrat3d::PasInstable));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, 1.0, 0.1, 0.5).err(), Some(ErreurSubstrat3d::PasInstable));
    assert_eq!(Substrat3d::nouveau(0, 8, 8, 0.1, 0.1, 1.0).err(), Some(ErreurSubstrat3d::DimensionNulle));
    assert_eq!(Substrat3d::nouveau(8, 0, 8, 0.1, 0.1, 1.0).err(), Some(ErreurSubstrat3d::DimensionNulle));
    assert_eq!(Substrat3d::nouveau(8, 8, 0, 0.1, 0.1, 1.0).err(), Some(ErreurSubstrat3d::DimensionNulle));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, -1.0, 0.1, 1.0).err(), Some(ErreurSubstrat3d::ParametreInvalide));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, 0.1, 0.0, 1.0).err(), Some(ErreurSubstrat3d::ParametreInvalide));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, 0.1, 0.1, 0.0).err(), Some(ErreurSubstrat3d::ParametreInvalide));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, f64::NAN, 0.1, 1.0).err(), Some(ErreurSubstrat3d::ParametreInvalide));
    assert_eq!(Substrat3d::nouveau(8, 8, 8, 0.1, f64::INFINITY, 1.0).err(), Some(ErreurSubstrat3d::ParametreInvalide));
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn un_pas_donne_les_valeurs_exactes() {
    let mut s = Substrat3d::nouveau(5, 5, 5, 0.125, 1.0, 1.0).unwrap();
    s.deposer(2, 2, 2, 1.0);
    s.pas();
    assert_eq!(s.valeur(2, 2, 2), 0.25);
    let voisines = [(1, 2, 2), (3, 2, 2), (2, 1, 2), (2, 3, 2), (2, 2, 1), (2, 2, 3)];
    for (x, y, z) in voisines {
        assert_eq!(s.valeur(x, y, z), 0.125, "voisine ({x}, {y}, {z})");
    }
    let mut nulles = 0;
    for z in 0..5 {
        for y in 0..5 {
            for x in 0..5 {
                if (x, y, z) == (2, 2, 2) || voisines.contains(&(x, y, z)) {
                    continue;
                }
                assert_eq!(s.valeur(x, y, z), 0.0, "case ({x}, {y}, {z})");
                nulles += 1;
            }
        }
    }
    assert_eq!(nulles, 118);

    s.pas();
    assert_eq!(s.valeur(2, 2, 2), 0.15625);
    assert_eq!(s.valeur(1, 2, 2), 0.0625);
    assert_eq!(s.valeur(1, 1, 2), 0.03125);
    assert_eq!(s.valeur(0, 2, 2), 0.015625);
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn une_tranche_vaut_le_plan_au_bit_pres() {
    for p in [1usize, 4] {
        let mut plan = Substrat::nouveau(23, 17, 0.15, 1.0, 1.0).unwrap();
        let mut volume = Substrat3d::nouveau(23, 17, p, 0.15, 1.0, 1.0).unwrap();
        plan.bloquer(5, 3, 7, 9);
        volume.bloquer(5, 3, 0, 7, 9, p - 1);
        plan.deposer(0, 0, 3.0);
        plan.deposer(11, 8, 7.5);
        plan.deposer(22, 16, 1.25);
        plan.deposer(15, 2, 0.5);
        for z in 0..p {
            volume.deposer(0, 0, z, 3.0);
            volume.deposer(11, 8, z, 7.5);
            volume.deposer(22, 16, z, 1.25);
            volume.deposer(15, 2, z, 0.5);
        }
        for pas in 1..=300 {
            plan.pas();
            volume.pas();
            if pas % 50 == 0 || pas == 300 {
                for z in 0..p {
                    for y in 0..17 {
                        for x in 0..23 {
                            assert_eq!(
                                volume.valeur(x, y, z).to_bits(),
                                plan.valeur(x, y).to_bits(),
                                "profondeur {p}, pas {pas}, case ({x}, {y}, {z})"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn la_variance_d_un_pic_croit_de_deux_r_par_pas_et_par_axe() {
    let (n, centre, pas) = (41, 20, 19);
    let mut s = Substrat3d::nouveau(n, n, n, 0.15, 1.0, 1.0).unwrap();
    s.deposer(centre, centre, centre, 1.0);
    s.pas_n(pas);
    let attendu = 2.0 * 0.15 * pas as f64;
    for axe in 0..3 {
        let v = variance(&s, axe, centre, 1.0);
        assert!(((v - attendu) / attendu).abs() <= 1e-9, "axe {axe} : variance {v}, attendue {attendu}");
    }

    let mut s = Substrat3d::nouveau(n, n, n, 1.0, 0.02, 0.5).unwrap();
    s.deposer(centre, centre, centre, 1.0);
    s.pas_n(pas);
    let attendu = 2.0 * 1.0 * pas as f64 * 0.02;
    for axe in 0..3 {
        let v = variance(&s, axe, centre, 0.5);
        assert!(((v - attendu) / attendu).abs() <= 1e-9, "axe {axe} : variance {v}, attendue {attendu}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn un_pic_au_centre_reste_symetrique() {
    let n = 21;
    let mut s = Substrat3d::nouveau(n, n, n, 0.16, 1.0, 1.0).unwrap();
    s.deposer(10, 10, 10, 1.0);
    s.pas_n(120);
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                let v = s.valeur(x, y, z);
                // Réflexion en x : gauche et droite s'échangent, l'addition est commutative.
                assert_eq!(s.valeur(n - 1 - x, y, z).to_bits(), v.to_bits(), "réflexion x en ({x}, {y}, {z})");
                let images = [(x, n - 1 - y, z), (x, y, n - 1 - z), (y, x, z), (z, y, x), (x, z, y), (y, z, x)];
                for (a, b, c) in images {
                    let w = s.valeur(a, b, c);
                    assert!((w - v).abs() <= 1e-12, "symétrie en ({x}, {y}, {z}) : {v} contre {w} en ({a}, {b}, {c})");
                }
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn la_masse_est_conservee_et_le_mur_est_etanche() {
    let (l, h, p) = (20, 12, 10);
    let mut s = Substrat3d::nouveau(l, h, p, 0.16, 1.0, 1.0).unwrap();
    s.bloquer(9, 0, 0, 10, h - 1, p - 1);
    s.deposer(0, 0, 0, 5.0);
    s.deposer(3, 7, 9, 2.0);
    s.deposer(8, 11, 4, 1.0);
    s.deposer(19, 11, 9, 4.0);
    let avant = s.masse_totale();
    assert!((avant - 12.0).abs() <= 1e-12, "masse initiale {avant}");
    s.pas_n(1000);
    let apres = s.masse_totale();
    assert!(((apres - avant) / avant).abs() <= 1e-9, "masse {avant} puis {apres}");

    for z in 0..p {
        for y in 0..h {
            for x in 9..=10 {
                assert!(s.est_bloquee(x, y, z), "case ({x}, {y}, {z}) devrait être bloquée");
                assert_eq!(s.valeur(x, y, z), 0.0, "case ({x}, {y}, {z}) bloquée");
            }
        }
    }

    let mut gauche = 0.0;
    for z in 0..p {
        for y in 0..h {
            for x in 0..9 {
                gauche += s.valeur(x, y, z);
            }
        }
    }
    assert!(((gauche - 8.0) / 8.0).abs() <= 1e-9, "somme à gauche {gauche}");

    let mut droite = 0.0;
    for z in 0..p {
        for y in 0..h {
            for x in 11..l {
                droite += s.valeur(x, y, z);
            }
        }
    }
    assert!(((droite - 4.0) / 4.0).abs() <= 1e-9, "somme à droite {droite}");
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn les_cases_bloquees_sont_des_murs() {
    let mut s = Substrat3d::nouveau(6, 5, 4, 0.1, 1.0, 1.0).unwrap();
    s.deposer(2, 2, 2, 3.0);
    s.bloquer(1, 1, 1, 3, 3, 2);
    for z in 0..4 {
        for y in 0..5 {
            for x in 0..6 {
                let dedans = (1..=3).contains(&x) && (1..=3).contains(&y) && (1..=2).contains(&z);
                assert_eq!(s.est_bloquee(x, y, z), dedans, "est_bloquee en ({x}, {y}, {z})");
                if dedans {
                    assert_eq!(s.valeur(x, y, z), 0.0, "valeur bloquée en ({x}, {y}, {z})");
                }
            }
        }
    }
    assert_eq!(s.retirer(2, 2, 2, 1.0), 0.0);
}

#[test]
#[should_panic(expected = "bloquée")]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn deposer_sur_une_case_bloquee_panique() {
    let mut s = Substrat3d::nouveau(6, 5, 4, 0.1, 1.0, 1.0).unwrap();
    s.bloquer(1, 1, 1, 3, 3, 2);
    s.deposer(2, 2, 2, 1.0);
}

#[test]
#[should_panic(expected = "hors du volume")]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn valeur_hors_du_volume_panique() {
    let s = Substrat3d::nouveau(6, 5, 4, 0.1, 1.0, 1.0).unwrap();
    let _ = s.valeur(6, 0, 0);
}

#[test]
#[cfg_attr(not(feature = "etape-18"), ignore)]
fn meme_champ_meme_pas_meme_octets() {
    let construire = || {
        let mut s = Substrat3d::nouveau(25, 17, 6, 0.1, 1.2, 1.1).unwrap();
        s.deposer(3, 4, 5, 2.5);
        s.deposer(20, 9, 1, 1.0);
        s
    };
    let (mut a, mut b, mut c) = (construire(), construire(), construire());
    let initial = a.ecrire_canonique();
    assert_eq!(initial, b.ecrire_canonique());
    assert_eq!(initial.len(), 24 + 8 * 25 * 17 * 6);
    a.pas_n(50);
    for _ in 0..50 {
        b.pas();
    }
    let (oa, ob) = (a.ecrire_canonique(), b.ecrire_canonique());
    assert_eq!(oa, ob);
    c.pas_n(0);
    assert_eq!(c.ecrire_canonique(), initial);
    a.pas();
    assert_ne!(a.ecrire_canonique(), oa);
}
