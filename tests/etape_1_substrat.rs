//! Tests d'acceptation de l'étape 1 (b) : module `substrat`.

use mycelium_lab::substrat::{ErreurSubstrat, Substrat};

/// Variance du champ le long d'un axe, en unités physiques, autour de la case `centre`.
fn variance(s: &Substrat, dx: f64, centre: usize, selon_x: bool) -> f64 {
    let (mut m, mut m2) = (0.0, 0.0);
    for y in 0..s.hauteur() {
        for x in 0..s.largeur() {
            let c = if selon_x { x } else { y };
            let pos = (c as f64 - centre as f64) * dx;
            let v = s.valeur(x, y);
            m += v;
            m2 += v * pos * pos;
        }
    }
    m2 / m
}

fn champ_pique(n: usize, d: f64, dt: f64, dx: f64) -> Substrat {
    let mut s = Substrat::nouveau(n, n, d, dt, dx).unwrap();
    s.deposer(n / 2, n / 2, 1.0);
    s
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn construction_refuse_les_parametres_instables_ou_invalides() {
    assert!(Substrat::nouveau(8, 8, 1.0, 0.25, 1.0).is_ok());
    assert_eq!(Substrat::nouveau(8, 8, 1.0, 0.26, 1.0).err(), Some(ErreurSubstrat::PasInstable));
    assert_eq!(Substrat::nouveau(8, 8, 1.0, 0.1, 0.5).err(), Some(ErreurSubstrat::PasInstable));
    assert_eq!(Substrat::nouveau(0, 8, 1.0, 0.1, 1.0).err(), Some(ErreurSubstrat::DimensionNulle));
    assert_eq!(Substrat::nouveau(8, 0, 1.0, 0.1, 1.0).err(), Some(ErreurSubstrat::DimensionNulle));
    assert_eq!(Substrat::nouveau(8, 8, -1.0, 0.1, 1.0).err(), Some(ErreurSubstrat::ParametreInvalide));
    assert_eq!(Substrat::nouveau(8, 8, 1.0, 0.0, 1.0).err(), Some(ErreurSubstrat::ParametreInvalide));
    assert_eq!(Substrat::nouveau(8, 8, 1.0, 0.1, 0.0).err(), Some(ErreurSubstrat::ParametreInvalide));
    assert_eq!(Substrat::nouveau(8, 8, f64::NAN, 0.1, 1.0).err(), Some(ErreurSubstrat::ParametreInvalide));
    assert_eq!(Substrat::nouveau(8, 8, 1.0, f64::INFINITY, 1.0).err(), Some(ErreurSubstrat::ParametreInvalide));
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn masse_conservee_apres_mille_pas() {
    let (l, h) = (30, 20);
    let mut s = Substrat::nouveau(l, h, 1.0, 0.2, 1.0).unwrap();
    // Dépôts reproductibles, y compris dans les coins et sur les bords.
    let mut etat: u64 = 12345;
    for _ in 0..40 {
        etat = etat.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = ((etat >> 33) as usize) % l;
        let y = ((etat >> 20) as usize) % h;
        s.deposer(x, y, 1.0 + ((etat >> 50) % 7) as f64);
    }
    s.deposer(0, 0, 5.0);
    s.deposer(l - 1, h - 1, 5.0);
    let avant = s.masse_totale();
    assert!(avant > 0.0);
    s.pas_n(1000);
    let apres = s.masse_totale();
    assert!(((apres - avant) / avant).abs() <= 1e-9, "avant {avant}, après {apres}");
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn la_variance_d_un_pic_croit_de_2dt_par_axe() {
    let (n, d, dt, dx, pas) = (101, 1.0, 0.05, 0.5, 100);
    let mut s = champ_pique(n, d, dt, dx);
    s.pas_n(pas);
    let t = pas as f64 * dt;
    let attendu = 2.0 * d * t;
    for selon_x in [true, false] {
        let v = variance(&s, dx, n / 2, selon_x);
        assert!(((v - attendu) / attendu).abs() <= 0.01, "axe x={selon_x} : variance {v}, attendue {attendu}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn un_pic_au_centre_reste_symetrique() {
    let n = 41;
    let mut s = champ_pique(n, 1.0, 0.2, 1.0);
    s.pas_n(200);
    let mut max = 0.0f64;
    for y in 0..n {
        for x in 0..n {
            let v = s.valeur(x, y);
            max = max.max(v);
            let autres = [
                s.valeur(n - 1 - x, y),         // réflexion en x
                s.valeur(x, n - 1 - y),         // réflexion en y
                s.valeur(n - 1 - y, x),         // quart de tour
                s.valeur(n - 1 - x, n - 1 - y), // demi-tour
                s.valeur(y, n - 1 - x),         // quart de tour inverse
                s.valeur(y, x),                 // transposition
            ];
            for a in autres {
                assert!((a - v).abs() <= 1e-12, "asymétrie en ({x}, {y}) : {v} contre {a}");
            }
        }
    }
    assert!(max > 0.0);
}

#[test]
#[cfg_attr(not(feature = "etape-1"), ignore)]
fn meme_champ_meme_pas_meme_octets() {
    let construire = || {
        let mut s = Substrat::nouveau(25, 17, 0.7, 0.3, 1.1).unwrap();
        s.deposer(3, 4, 2.5);
        s.deposer(20, 9, 1.0);
        s
    };
    let mut a = construire();
    let mut b = construire();
    let mut c = construire();
    assert_eq!(a.ecrire_canonique(), b.ecrire_canonique());
    a.pas_n(50);
    b.pas_n(50);
    for _ in 0..50 {
        c.pas();
    }
    let (oa, ob, oc) = (a.ecrire_canonique(), b.ecrire_canonique(), c.ecrire_canonique());
    assert_eq!(oa.len(), 16 + 8 * 25 * 17);
    assert_eq!(oa, ob);
    assert_eq!(oa, oc);
    b.pas();
    assert_ne!(oa, b.ecrire_canonique());
}
