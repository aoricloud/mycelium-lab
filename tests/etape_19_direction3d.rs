//! Tests d'acceptation de l'étape 19 : module `direction3d`.
//!
//! Ce qui est vérifié :
//! - `nouvelle` refuse le vecteur nul et les composantes non finies (la non-finitude passe avant la nullité), et
//!   rend les composantes divisées par la norme, au bit près ;
//! - `scalaire` : une direction contre elle-même vaut 1, deux directions perpendiculaires valent 0 ;
//! - `base` : exacte pour l'axe vertical, puis orthonormée sur 1000 directions tirées au hasard ;
//! - `tourner` : des cas exacts au bit près, la norme, l'angle (par les tables `COS`), le retour à la direction de
//!   départ pour `theta = 0`, et les angles pris modulo 360 ;
//! - `au_hasard` : les trois premières directions de la graine 7 au bit près, le nombre exact de tirages, puis
//!   l'uniformité sur la sphère.

use mycelium_lab::alea::Alea;
use mycelium_lab::croissance::COS;
use mycelium_lab::direction3d::{Direction3d, ErreurDirection3d};

/// La direction du vecteur `(x, y, z)`, qui doit être acceptée.
fn dir(x: f64, y: f64, z: f64) -> Direction3d {
    Direction3d::nouvelle(x, y, z).expect("vecteur accepté")
}

/// Les bits des trois composantes.
fn bits(d: &Direction3d) -> [u64; 3] {
    [d.x().to_bits(), d.y().to_bits(), d.z().to_bits()]
}

/// Les bits de trois flottants.
fn bits_de(x: f64, y: f64, z: f64) -> [u64; 3] {
    [x.to_bits(), y.to_bits(), z.to_bits()]
}

/// `d` a exactement ces trois composantes, au bit près.
fn assert_bits(d: &Direction3d, attendu: [u64; 3], contexte: &str) {
    assert_eq!(bits(d), attendu, "{contexte} : composantes (x, y, z) {:?}", (d.x(), d.y(), d.z()));
}

/// La norme de `d`, sans fonction de f64 autre que `sqrt`.
fn norme(d: &Direction3d) -> f64 {
    (d.x() * d.x() + d.y() * d.y() + d.z() * d.z()).sqrt()
}

/// `n` directions tirées de `Alea::new(graine)`.
fn directions(graine: u64, n: usize) -> Vec<Direction3d> {
    let mut alea = Alea::new(graine);
    let liste: Vec<Direction3d> = (0..n).map(|_| Direction3d::au_hasard(&mut alea)).collect();
    assert_eq!(liste.len(), n, "{n} directions tirées");
    liste
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn nouvelle_refuse_le_nul_et_le_non_fini() {
    assert_eq!(Direction3d::nouvelle(0.0, 0.0, 0.0).err(), Some(ErreurDirection3d::Nulle), "vecteur nul");
    assert_eq!(Direction3d::nouvelle(f64::NAN, 0.0, 0.0).err(), Some(ErreurDirection3d::NonFinie), "NaN en x");
    assert_eq!(Direction3d::nouvelle(0.0, f64::INFINITY, 0.0).err(), Some(ErreurDirection3d::NonFinie), "infini en y");
    assert_eq!(
        Direction3d::nouvelle(0.0, 0.0, f64::NEG_INFINITY).err(),
        Some(ErreurDirection3d::NonFinie),
        "moins l'infini en z"
    );
    let nan = Direction3d::nouvelle(f64::NAN, 0.0, 0.0).err();
    assert_ne!(nan, Some(ErreurDirection3d::Nulle), "(NaN, 0, 0) est non fini, pas nul");
    assert_eq!(nan, Some(ErreurDirection3d::NonFinie), "(NaN, 0, 0) est non fini");
    assert!(Direction3d::nouvelle(0.0, 0.0, 1.0).is_ok(), "(0, 0, 1) est accepté");
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn nouvelle_divise_par_la_norme_au_bit_pres() {
    let d = dir(3.0, 0.0, 4.0);
    assert_bits(&d, bits_de(0.6, 0.0, 0.8), "nouvelle(3, 0, 4)");
    assert_eq!(d.x(), 0.6, "x vaut 0.6");
    assert_eq!(d.y(), 0.0, "y vaut 0");
    assert_eq!(d.z(), 0.8, "z vaut 0.8");

    let d123 = dir(1.0, 2.0, 3.0);
    assert_bits(&d123, [0x3fd11acee560242a, 0x3fe11acee560242a, 0x3fe9a8365810363f], "nouvelle(1, 2, 3)");
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn scalaire_vaut_un_contre_soi_et_zero_en_perpendiculaire() {
    let d123 = dir(1.0, 2.0, 3.0);
    let haut = dir(0.0, 0.0, 1.0);
    let s = d123.scalaire(&d123);
    assert!((s - 1.0).abs() <= 4.5e-16, "d123 · d123 = {s}, attendu 1 à 4.5e-16 près");
    let p = haut.scalaire(&dir(1.0, 0.0, 0.0));
    assert_eq!(p.to_bits(), 0.0f64.to_bits(), "haut · (1, 0, 0) = {p}, attendu 0.0 exactement");
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn base_de_l_axe_vertical_est_exacte() {
    let haut = dir(0.0, 0.0, 1.0);
    let (u, v) = haut.base();
    assert_bits(&u, bits_de(0.0, 1.0, 0.0), "u de la base de haut");
    assert_bits(&v, bits_de(-1.0, 0.0, 0.0), "v de la base de haut");
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn base_est_orthonormee_sur_des_directions_au_hasard() {
    let liste = directions(11, 1000);
    for (i, d) in liste.iter().enumerate() {
        let (u, v) = d.base();
        let (nu, nv) = (norme(&u), norme(&v));
        assert!((nu - 1.0).abs() <= 4.5e-16, "direction {i} : norme de u = {nu}");
        assert!((nv - 1.0).abs() <= 4.5e-16, "direction {i} : norme de v = {nv}");
        let (du, dv, uv) = (d.scalaire(&u), d.scalaire(&v), u.scalaire(&v));
        assert!(du.abs() <= 3e-16, "direction {i} : d · u = {du}");
        assert!(dv.abs() <= 3e-16, "direction {i} : d · v = {dv}");
        assert!(uv.abs() <= 3e-16, "direction {i} : u · v = {uv}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn tourner_donne_les_valeurs_exactes() {
    let haut = dir(0.0, 0.0, 1.0);
    let d123 = dir(1.0, 2.0, 3.0);
    assert_bits(&haut.tourner(90, 0), bits_de(0.0, 1.0, 0.0), "haut.tourner(90, 0)");
    assert_bits(&haut.tourner(90, 90), bits_de(-1.0, 0.0, 0.0), "haut.tourner(90, 90)");
    assert_bits(
        &d123.tourner(37, 211),
        [0x3fe063582122570a, 0xbfa8b843370c72d8, 0x3feb711042514e4c],
        "d123.tourner(37, 211)",
    );
    assert_bits(
        &d123.tourner(180, 0),
        [0xbfd11acee560242a, 0xbfe11acee560242a, 0xbfe9a8365810363f],
        "d123.tourner(180, 0)",
    );
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn tourner_garde_la_norme_et_fait_l_angle_demande() {
    let liste = directions(11, 2000);
    let mut nb_cas = 0usize;
    for (i, d) in liste.iter().enumerate() {
        for theta in (0u32..=180).step_by(7) {
            for phi in (0u32..360).step_by(23) {
                let r = d.tourner(theta, phi);
                let n = norme(&r);
                assert!((n - 1.0).abs() <= 4.5e-16, "direction {i}, theta {theta}, phi {phi} : norme {n}");
                let s = r.scalaire(d);
                let attendu = COS[theta as usize];
                assert!(
                    (s - attendu).abs() <= 1e-15,
                    "direction {i}, theta {theta}, phi {phi} : r · d = {s}, attendu COS[{theta}] = {attendu}"
                );
                nb_cas += 1;
            }
        }
    }
    assert_eq!(nb_cas, 2000 * 26 * 16, "nombre de cas parcourus");
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn tourner_de_zero_rend_la_direction() {
    let liste = directions(11, 2000);
    for (i, d) in liste.iter().enumerate() {
        let r = d.tourner(0, 123);
        assert!((r.x() - d.x()).abs() <= 4.5e-16, "direction {i} : x {} contre {}", r.x(), d.x());
        assert!((r.y() - d.y()).abs() <= 4.5e-16, "direction {i} : y {} contre {}", r.y(), d.y());
        assert!((r.z() - d.z()).abs() <= 4.5e-16, "direction {i} : z {} contre {}", r.z(), d.z());
    }
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn tourner_prend_les_angles_modulo_360() {
    let liste = directions(11, 20);
    for (i, d) in liste.iter().enumerate() {
        let t = (i as u32 * 37) % 181;
        let p = (i as u32 * 71 + 5) % 360;
        let base = bits(&d.tourner(t, p));
        assert_eq!(bits(&d.tourner(t + 360, p)), base, "direction {i} : theta {t} + 360, phi {p}");
        assert_eq!(bits(&d.tourner(t, p + 720)), base, "direction {i} : theta {t}, phi {p} + 720");
        assert_eq!(bits(&d.tourner(t + 720, p + 360)), base, "direction {i} : theta {t} + 720, phi {p} + 360");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn au_hasard_donne_les_premieres_directions_de_la_graine_7() {
    let mut alea = Alea::new(7);
    let attendues = [
        [0xbfec83d606c79041, 0xbfd964315bcbfaf7, 0xbfcc341e1ba6cdf8],
        [0xbfd4d7dee7272a51, 0x3fe00c429b50fda7, 0x3fe9a610202eac4a],
        [0x3feec4fd240336de, 0xbfd07d43fdd60924, 0xbfb85989332bc3c0],
    ];
    for (i, attendue) in attendues.iter().enumerate() {
        let d = Direction3d::au_hasard(&mut alea);
        assert_bits(&d, *attendue, &format!("direction {i} de la graine 7"));
    }
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn au_hasard_fait_exactement_deux_tirages() {
    let mut alea = Alea::new(5);
    let _ = Direction3d::au_hasard(&mut alea);
    let apres = alea.suivant();

    let mut neuf = Alea::new(5);
    let _ = neuf.suivant_f64();
    let _ = neuf.suivant();
    let troisieme = neuf.suivant();
    assert_eq!(apres, troisieme, "après un au_hasard, le tirage suivant est le troisième de la graine 5");
}

#[test]
#[cfg_attr(not(feature = "etape-19"), ignore)]
fn au_hasard_est_uniforme_sur_la_sphere() {
    let n = 100_000;
    let liste = directions(3, n);
    let (mut sx, mut sy, mut sz, mut sz2) = (0.0, 0.0, 0.0, 0.0);
    for d in &liste {
        sx += d.x();
        sy += d.y();
        sz += d.z();
        sz2 += d.z() * d.z();
    }
    let n = n as f64;
    let (mx, my, mz, mz2) = (sx / n, sy / n, sz / n, sz2 / n);
    assert!(mx.abs() <= 0.01, "moyenne de x = {mx}, attendu 0 à 0.01 près");
    assert!(my.abs() <= 0.01, "moyenne de y = {my}, attendu 0 à 0.01 près");
    assert!(mz.abs() <= 0.01, "moyenne de z = {mz}, attendu 0 à 0.01 près");
    assert!((mz2 - 1.0 / 3.0).abs() <= 0.005, "moyenne de z * z = {mz2}, attendu 1/3 à 0.005 près");
}
