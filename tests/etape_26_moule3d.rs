//! Tests d'acceptation de l'étape 26 : le moule en volume.
//!
//! Ce qui est vérifié :
//! - les formes (pavé, coquille, boule, tore) recalculées dans le test, case par case, en `i128`, sur de petites
//!   boîtes et sur les trois formes des scènes (`sha` des tableaux de la boule de rayon 14, du tore 10/4 et de la
//!   coquille d'épaisseur 3) ; le tore de petit rayon 0, dont les douze cases sont écrites à la main ;
//! - le contrat des formes : `FormeInvalide`, `HorsDuCube` à chaque bord et sur chaque axe, l'ordre des deux erreurs,
//!   les boîtes de dimension nulle, des paramètres énormes sans panique, et la boule qui touche les faces ;
//! - la découpe en pavés : des pavés dans le cube, bien orientés, deux à deux disjoints, d'union exactement le
//!   complément de la cavité, triés par `(z0, y0, x0)` ; les nombres de pavés des formes des scènes et du labyrinthe,
//!   et le cas exact du pavé 1..=2 dans 4³ ;
//! - le labyrinthe : la formule des cases libres, les chambres entières, les murs communs entiers (passage ou mur
//!   plein), exactement N − 1 passages, le graphe connexe et les cases libres reliées par leurs six voisines, deux
//!   `sha` et trois nombres de pavés, et `chambre_3d` / `centre_de_la_chambre_3d` ;
//! - `developper_3d` : le texte rendu tel quel sans ligne `moule` (les cinq scènes et un texte à fins `\r\n`), le
//!   remplacement en place avec une ligne `moule`, les `sha` des quatre textes développés, l'ordre des refus, et le
//!   scénario complet de la boule lu et construit ;
//! - le remplissage : 828 cases de `colonie-cube` sur 32768, le rejeu des cases atteintes par les segments pour
//!   trois scènes et la boule, la courbe de la boule (101 points, 11513 cases libres, 8705 atteintes et la part
//!   écrite), le cas fait à la main « DROITE », et la courbe d'une colonie déjà à son temps ;
//! - sans moule, rien ne change : les cinq scènes gardent l'empreinte de leur fichier de `references/`.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement.

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ErreurColonie3d, lire_scenario_colonie3d};
use mycelium_lab::cube::Bloc;
use mycelium_lab::empreinte::sha256_hex;
use mycelium_lab::moule3d::{
    ErreurMoule3d, Forme3d, cases_atteintes, centre_de_la_chambre_3d, chambre_3d, courbe_de_remplissage_3d,
    developper_3d, libres_3d as libres, obstacles_3d, part_remplie,
};

/// Le scénario de la boule de rayon 14 dans un cube de 32³, développé par le moule.
const BOULE: &str = "\
cube 32 32 32
substrat 1.0 0.15
moule boule 16 16 16 14
spore 16 16 16
croissance 1.0 12 35 20
couplage 0.1 0.5
thigmotropisme 35
duree 100
";

/// Le scénario du cas fait à la main : une pointe droite de pas 0,5 dans un pavé de 4³, sans ramification.
const DROITE: &str = "\
cube 8 8 8
substrat 1.0 0.0
moule pave 2 2 2 5 5 5
spore 3 3 3
croissance 1.0 infini 30 0
couplage 1.0 0.0
duree 10
";

// ---------------------------------------------------------------------------------------------------------------------
// Les constructeurs et les outils
// ---------------------------------------------------------------------------------------------------------------------

fn pave(x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize) -> Forme3d {
    Forme3d::Pave { x0, y0, z0, x1, y1, z1 }
}

fn coquille(x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize, epaisseur: usize) -> Forme3d {
    Forme3d::Coquille { x0, y0, z0, x1, y1, z1, epaisseur }
}

fn boule(cx: usize, cy: usize, cz: usize, rayon: usize) -> Forme3d {
    Forme3d::Boule { cx, cy, cz, rayon }
}

fn tore(cx: usize, cy: usize, cz: usize, grand_rayon: usize, petit_rayon: usize) -> Forme3d {
    Forme3d::Tore { cx, cy, cz, grand_rayon, petit_rayon }
}

fn labyrinthe(colonnes: usize, rangees: usize, etages: usize, couloir: usize, graine: u64) -> Forme3d {
    Forme3d::Labyrinthe { colonnes, rangees, etages, couloir, graine }
}

fn bloc(x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32) -> Bloc {
    Bloc { x0, y0, z0, x1, y1, z1 }
}

/// Le nombre de cases vraies d'un tableau.
fn compter(cases: &[bool]) -> usize {
    cases.iter().filter(|&&libre| libre).count()
}

/// Le `sha256_hex` des cases, une case par octet, 1 pour vrai et 0 pour faux, dans l'ordre du tableau.
fn sha(cases: &[bool]) -> String {
    let octets: Vec<u8> = cases.iter().map(|&libre| u8::from(libre)).collect();
    sha256_hex(&octets)
}

/// Le prédicat de la cavité, écrit dans le test en entiers `i128` : la formule de la doc de [`libres_3d`].
fn dans_la_forme(forme: &Forme3d, x: usize, y: usize, z: usize) -> bool {
    let (x, y, z) = (x as i128, y as i128, z as i128);
    match forme {
        Forme3d::Pave { x0, y0, z0, x1, y1, z1 } => {
            let (x0, y0, z0, x1, y1, z1) =
                (*x0 as i128, *y0 as i128, *z0 as i128, *x1 as i128, *y1 as i128, *z1 as i128);
            (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (z0..=z1).contains(&z)
        }
        Forme3d::Coquille { x0, y0, z0, x1, y1, z1, epaisseur } => {
            let (x0, y0, z0, x1, y1, z1) =
                (*x0 as i128, *y0 as i128, *z0 as i128, *x1 as i128, *y1 as i128, *z1 as i128);
            let epaisseur = *epaisseur as i128;
            let dedans = (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (z0..=z1).contains(&z);
            dedans
                && (x - x0 < epaisseur
                    || x1 - x < epaisseur
                    || y - y0 < epaisseur
                    || y1 - y < epaisseur
                    || z - z0 < epaisseur
                    || z1 - z < epaisseur)
        }
        Forme3d::Boule { cx, cy, cz, rayon } => {
            let (dx, dy, dz) = (x - *cx as i128, y - *cy as i128, z - *cz as i128);
            let rayon = *rayon as i128;
            dx * dx + dy * dy + dz * dz <= rayon * rayon
        }
        Forme3d::Tore { cx, cy, cz, grand_rayon, petit_rayon } => {
            let (dx, dy, dz) = (x - *cx as i128, y - *cy as i128, z - *cz as i128);
            let grand = *grand_rayon as i128;
            let petit = *petit_rayon as i128;
            let s = dx * dx + dy * dy;
            let d = dz * dz;
            let a = s + grand * grand + d - petit * petit;
            a <= 0 || (a > 0 && a * a <= 4 * grand * grand * s)
        }
        Forme3d::Labyrinthe { .. } => panic!("le labyrinthe se vérifie par ses propriétés, pas par ce prédicat"),
    }
}

/// La cavité recalculée dans le test, case par case, avec le prédicat de la doc.
fn libres_reference(largeur: usize, hauteur: usize, profondeur: usize, forme: &Forme3d) -> Vec<bool> {
    let mut cases = vec![false; largeur * hauteur * profondeur];
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                cases[(z * hauteur + y) * largeur + x] = dans_la_forme(forme, x, y, z);
            }
        }
    }
    cases
}

/// Le résultat de [`libres_3d`] et celui de [`obstacles_3d`] doivent être la même erreur.
fn erreur(largeur: usize, hauteur: usize, profondeur: usize, forme: &Forme3d) -> ErreurMoule3d {
    let a = libres(largeur, hauteur, profondeur, forme).expect_err("une erreur était attendue");
    let b = obstacles_3d(largeur, hauteur, profondeur, forme).expect_err("obstacles : la même erreur était attendue");
    assert_eq!(a, b, "libres_3d et obstacles_3d rendent la même erreur pour {forme:?}");
    a
}

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// L'empreinte de référence d'un fichier de `references/`.
fn empreinte_de_reference(fichier: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join(fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
    assert_eq!(texte.lines().count(), 1, "{fichier} : une seule ligne");
    let empreinte = texte.trim();
    assert_eq!(empreinte.len(), 64, "{fichier} : 64 chiffres hexadécimaux");
    empreinte.to_string()
}

/// Une colonie construite depuis un texte de scénario, déjà développé par le moule, à la graine `graine`.
fn colonie_developpee(texte: &str, graine: u64) -> Colonie3d {
    let developpe = developper_3d(texte).unwrap_or_else(|e| panic!("refus inattendu du moule : {e:?}"));
    let scenario = lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("refus inattendu : {e:?}"));
    Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("construction refusée : {e:?}"))
}

/// Une colonie d'une scène, à la graine `graine`, avancée jusqu'à sa durée.
fn colonie_finie(nom: &str, graine: u64) -> Colonie3d {
    let scenario =
        lire_scenario_colonie3d(&texte_du_fichier(nom)).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"));
    let mut colonie =
        Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("{nom} : construction refusée {e:?}"));
    colonie.avancer_n(colonie.duree());
    colonie
}

/// Les cases atteintes, recalculées dans le test : l'arrivée de chaque segment, au plancher, bornée à la dernière case.
fn rejeu_des_atteintes(colonie: &Colonie3d) -> Vec<bool> {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let mut atteintes = vec![false; largeur * hauteur * profondeur];
    for segment in colonie.segments() {
        let x = (segment.x1.floor() as usize).min(largeur - 1);
        let y = (segment.y1.floor() as usize).min(hauteur - 1);
        let z = (segment.z1.floor() as usize).min(profondeur - 1);
        atteintes[(z * hauteur + y) * largeur + x] = true;
    }
    atteintes
}

/// Le nombre de cases non bloquées du substrat : la cavité.
fn cases_libres_du_substrat(colonie: &Colonie3d) -> usize {
    let scenario = colonie.scenario();
    let (largeur, hauteur, profondeur) =
        (scenario.largeur as usize, scenario.hauteur as usize, scenario.profondeur as usize);
    let mut libres = 0;
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                if !colonie.substrat().est_bloquee(x, y, z) {
                    libres += 1;
                }
            }
        }
    }
    libres
}

// ---------------------------------------------------------------------------------------------------------------------
// 1. Les formes
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn les_formes_sont_leur_cavite_recalculee_dans_le_test() {
    let cas: &[(usize, usize, usize, Forme3d, usize)] = &[
        (5, 5, 5, boule(2, 2, 2, 0), 1),
        (5, 5, 5, boule(2, 2, 2, 1), 7),
        (5, 5, 5, boule(2, 2, 2, 2), 33),
        (7, 7, 3, tore(3, 3, 1, 2, 1), 36),
        (11, 11, 5, tore(5, 5, 2, 3, 2), 216),
        (32, 32, 32, tore(16, 16, 16, 10, 0), 12),
        (6, 6, 6, coquille(1, 1, 1, 4, 4, 4, 1), 56),
        (6, 6, 6, coquille(1, 1, 1, 4, 4, 4, 2), 64),
        (4, 4, 4, pave(1, 1, 1, 2, 2, 2), 8),
        (4, 4, 4, pave(0, 0, 0, 3, 3, 3), 64),
    ];
    for &(largeur, hauteur, profondeur, ref forme, attendues) in cas {
        let attendu = libres_reference(largeur, hauteur, profondeur, forme);
        let vues = libres(largeur, hauteur, profondeur, forme).expect("la forme est valide et dans le cube");
        assert_eq!(vues.len(), largeur * hauteur * profondeur, "{forme:?} : un booléen par case");
        assert_eq!(vues, attendu, "{forme:?} dans {largeur} × {hauteur} × {profondeur} : la cavité du test");
        assert_eq!(compter(&attendu), attendues, "{forme:?} : le nombre de cases écrit");
        assert_eq!(compter(&vues), attendues, "{forme:?} : le nombre de cases de libres_3d");
    }

    // Le tore de petit rayon 0 : exactement les douze cases à distance 10 dans le plan z = 16.
    let vues = libres(32, 32, 32, &tore(16, 16, 16, 10, 0)).expect("le tore de petit rayon 0 est valide");
    let mut vues_exactes = vec![false; 32 * 32 * 32];
    let (cx, cy, cz) = (16usize, 16usize, 16usize);
    for (dx, dy) in [(10usize, 0usize), (0, 10), (6, 8), (8, 6)] {
        for (sx, sy) in [(1isize, 1isize), (1, -1), (-1, 1), (-1, -1)] {
            let x = (cx as isize + sx * dx as isize) as usize;
            let y = (cy as isize + sy * dy as isize) as usize;
            vues_exactes[(cz * 32 + y) * 32 + x] = true;
        }
    }
    assert_eq!(compter(&vues_exactes), 12, "douze cases exactement");
    assert_eq!(vues, vues_exactes, "le tore de petit rayon 0 est les douze cases écrites à la main");

    // Les formes des scènes dans 32³ : le nombre de cases et le sha écrit.
    let boule14 = libres(32, 32, 32, &boule(16, 16, 16, 14)).expect("la boule de rayon 14 tient dans 32³");
    assert_eq!(compter(&boule14), 11513, "la boule de rayon 14 : 11513 cases");
    assert_eq!(
        sha(&boule14),
        "b85c88057f2041315688075779397f927983cb7e15afc030c186011988a56af4",
        "la boule de rayon 14 : le sha de la cavité"
    );
    let tore10_4 = libres(32, 32, 32, &tore(16, 16, 16, 10, 4)).expect("le tore 10/4 tient dans 32³");
    assert_eq!(compter(&tore10_4), 2992, "le tore 10/4 : 2992 cases");
    assert_eq!(
        sha(&tore10_4),
        "5ebcd4e12545a332ab2fe26e98f2c8977d2ae7d696d4931e003310be8589c894",
        "le tore 10/4 : le sha de la cavité"
    );
    let coquille3 = libres(32, 32, 32, &coquille(2, 2, 2, 29, 29, 29, 3)).expect("la coquille d'épaisseur 3 tient");
    assert_eq!(compter(&coquille3), 11304, "la coquille d'épaisseur 3 : 28³ − 22³ cases");
    assert_eq!(
        sha(&coquille3),
        "5b3d213d19c4ea9afa925c73f9f6fc5961524dd3efed352171301dd5d8207435",
        "la coquille d'épaisseur 3 : le sha de la cavité"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// 2. Le contrat des formes
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn les_formes_hors_contrat_sont_refusees() {
    let invalide = ErreurMoule3d::FormeInvalide;
    // Un pavé à l'envers, sur chacun des trois axes.
    assert_eq!(erreur(10, 10, 10, &pave(3, 0, 0, 2, 5, 5)), invalide);
    assert_eq!(erreur(10, 10, 10, &pave(0, 3, 0, 5, 2, 5)), invalide);
    assert_eq!(erreur(10, 10, 10, &pave(0, 0, 3, 5, 5, 2)), invalide);
    assert!(libres(10, 10, 10, &pave(3, 3, 3, 3, 3, 3)).is_ok(), "un pavé d'une case est permis");
    // Une coquille à l'envers sur chaque axe, et d'épaisseur 0.
    assert_eq!(erreur(10, 10, 10, &coquille(3, 0, 0, 2, 5, 5, 1)), invalide);
    assert_eq!(erreur(10, 10, 10, &coquille(0, 3, 0, 5, 2, 5, 1)), invalide);
    assert_eq!(erreur(10, 10, 10, &coquille(0, 0, 3, 5, 5, 2, 1)), invalide);
    assert_eq!(erreur(10, 10, 10, &coquille(0, 0, 0, 5, 5, 5, 0)), invalide, "épaisseur 0");
    assert!(libres(10, 10, 10, &coquille(0, 0, 0, 5, 5, 5, 1)).is_ok());
    // Un tore de petit rayon égal ou supérieur au grand.
    assert_eq!(erreur(10, 10, 10, &tore(5, 5, 5, 3, 3)), invalide);
    assert_eq!(erreur(10, 10, 10, &tore(5, 5, 5, 3, 4)), invalide);
    assert!(libres(11, 11, 11, &tore(5, 5, 5, 3, 2)).is_ok());
    // Un labyrinthe dont un des quatre nombres vaut 0.
    assert_eq!(erreur(30, 30, 30, &labyrinthe(0, 3, 3, 2, 1)), invalide);
    assert_eq!(erreur(30, 30, 30, &labyrinthe(3, 0, 3, 2, 1)), invalide);
    assert_eq!(erreur(30, 30, 30, &labyrinthe(3, 3, 0, 2, 1)), invalide);
    assert_eq!(erreur(30, 30, 30, &labyrinthe(3, 3, 3, 0, 1)), invalide);
    assert!(libres(30, 30, 30, &labyrinthe(1, 1, 1, 1, 0)).is_ok());
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn une_cavite_qui_sort_du_cube_est_refusee_a_chaque_bord() {
    let hors = ErreurMoule3d::HorsDuCube;
    // Un cube de 12 × 9 × 7 : une boule de rayon 3, dépassant d'un côté puis de l'autre, sur chaque axe.
    assert!(libres(12, 9, 7, &boule(4, 4, 3, 3)).is_ok(), "la boule de référence tient");
    assert_eq!(erreur(12, 9, 7, &boule(2, 4, 3, 3)), hors, "cx − rayon négatif");
    assert_eq!(erreur(12, 9, 7, &boule(9, 4, 3, 3)), hors, "cx + rayon = largeur");
    assert_eq!(erreur(12, 9, 7, &boule(4, 2, 3, 3)), hors, "cy − rayon négatif");
    assert_eq!(erreur(12, 9, 7, &boule(4, 6, 3, 3)), hors, "cy + rayon = hauteur");
    assert_eq!(erreur(12, 9, 7, &boule(4, 4, 2, 3)), hors, "cz − rayon négatif");
    assert_eq!(erreur(12, 9, 7, &boule(4, 4, 4, 3)), hors, "cz + rayon = profondeur");
    // Un tore de grand rayon 3 et petit rayon 1, dépassant sur chaque axe, des deux côtés.
    assert!(libres(12, 9, 7, &tore(4, 4, 3, 3, 1)).is_ok(), "le tore de référence tient");
    assert_eq!(erreur(12, 9, 7, &tore(3, 4, 3, 3, 1)), hors, "cx − (R + r) négatif");
    assert_eq!(erreur(12, 9, 7, &tore(8, 4, 3, 3, 1)), hors, "cx + (R + r) = largeur");
    assert_eq!(erreur(12, 9, 7, &tore(4, 3, 3, 3, 1)), hors, "cy − (R + r) négatif");
    assert_eq!(erreur(12, 9, 7, &tore(4, 5, 3, 3, 1)), hors, "cy + (R + r) = hauteur");
    assert_eq!(erreur(12, 9, 7, &tore(4, 4, 0, 3, 1)), hors, "cz − r négatif");
    assert_eq!(erreur(12, 9, 7, &tore(4, 4, 6, 3, 1)), hors, "cz + r = profondeur");
    // Un pavé qui finit au bord.
    assert!(libres(12, 9, 7, &pave(0, 0, 0, 11, 8, 6)).is_ok(), "la dernière case est dedans");
    assert_eq!(erreur(12, 9, 7, &pave(0, 0, 0, 12, 8, 6)), hors, "x1 = largeur");
    assert_eq!(erreur(12, 9, 7, &pave(0, 0, 0, 11, 9, 6)), hors, "y1 = hauteur");
    assert_eq!(erreur(12, 9, 7, &pave(0, 0, 0, 11, 8, 7)), hors, "z1 = profondeur");
    // Un labyrinthe trop grand d'une case.
    assert!(libres(25, 25, 25, &labyrinthe(6, 6, 6, 3, 5)).is_ok(), "W = H = P = 25");
    assert_eq!(erreur(24, 25, 25, &labyrinthe(6, 6, 6, 3, 5)), hors, "W = 25 > largeur 24");
    assert_eq!(erreur(25, 24, 25, &labyrinthe(6, 6, 6, 3, 5)), hors, "H = 25 > hauteur 24");
    assert_eq!(erreur(25, 25, 24, &labyrinthe(6, 6, 6, 3, 5)), hors, "P = 25 > profondeur 24");
    // Une boule qui touche exactement les faces du cube est acceptée.
    assert!(libres(5, 5, 5, &boule(2, 2, 2, 2)).is_ok(), "la boule touche les six faces");
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn une_forme_invalide_l_emporte_sur_hors_du_cube() {
    let invalide = ErreurMoule3d::FormeInvalide;
    // Deux fautes à la fois : la forme est invalide ET elle sort du cube.
    assert_eq!(erreur(5, 5, 5, &pave(40, 40, 40, 30, 30, 30)), invalide);
    assert_eq!(erreur(5, 5, 5, &coquille(0, 0, 0, 40, 40, 40, 0)), invalide);
    assert_eq!(erreur(5, 5, 5, &tore(40, 40, 40, 3, 3)), invalide);
    assert_eq!(erreur(5, 5, 5, &labyrinthe(10, 0, 0, 3, 0)), invalide);
    assert_eq!(erreur(0, 0, 0, &pave(3, 0, 0, 2, 0, 0)), invalide, "même dans une boîte de dimension nulle");
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn une_boite_de_dimension_nulle_ne_contient_aucune_forme_valide() {
    let formes = [
        pave(0, 0, 0, 0, 0, 0),
        coquille(0, 0, 0, 0, 0, 0, 1),
        boule(0, 0, 0, 0),
        tore(1, 1, 1, 1, 0),
        labyrinthe(1, 1, 1, 1, 7),
        pave(0, 0, 0, 3, 3, 3),
        boule(5, 5, 5, 2),
    ];
    assert_eq!(formes.len(), 7, "les sept formes valides sont essayées");
    for forme in &formes {
        for (largeur, hauteur, profondeur) in
            [(0usize, 0usize, 0usize), (0, 7, 7), (7, 0, 7), (7, 7, 0), (0, 1, 1), (1, 0, 1), (1, 1, 0)]
        {
            assert_eq!(
                erreur(largeur, hauteur, profondeur, forme),
                ErreurMoule3d::HorsDuCube,
                "{forme:?} dans {largeur} × {hauteur} × {profondeur}"
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn des_parametres_enormes_donnent_une_erreur_et_non_une_panique() {
    let max = usize::MAX;
    // Un rayon, un centre, un couloir ou un nombre de chambres énormes : une erreur, sans panique ni débordement.
    assert!(matches!(erreur(10, 10, 10, &boule(5, 5, 5, max)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &boule(max, 5, 5, 1)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &boule(max - 1, 5, 5, 1)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &boule(5, 5, max, 1)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &boule(max, max, max, max)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &tore(max, max, max, max, max - 1)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &tore(5, 5, 5, max, 0)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &pave(0, 0, 0, max, 3, 3)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &labyrinthe(max, 1, 1, 1, 0)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &labyrinthe(1, max, 1, 1, 0)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &labyrinthe(1, 1, max, 1, 0)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(erreur(10, 10, 10, &labyrinthe(1, 1, 1, max, 0)), ErreurMoule3d::HorsDuCube));
    assert!(matches!(
        erreur(10, 10, 10, &labyrinthe(max / 2, max / 2, max / 2, max / 2, 0)),
        ErreurMoule3d::HorsDuCube
    ));
}

// ---------------------------------------------------------------------------------------------------------------------
// 3. La découpe en pavés
// ---------------------------------------------------------------------------------------------------------------------

/// Vérifie qu'une découpe est dans le cube, bien orientée, disjointe, d'union le complément, et triée.
fn verifier_la_decoupe(largeur: usize, hauteur: usize, profondeur: usize, forme: &Forme3d) -> Vec<Bloc> {
    let cases = libres(largeur, hauteur, profondeur, forme).expect("la forme est valide et dans le cube");
    let paves = obstacles_3d(largeur, hauteur, profondeur, forme).expect("la même forme, la même découpe");

    // Dans le cube et bien orientés.
    for p in &paves {
        assert!(p.x0 <= p.x1 && p.y0 <= p.y1 && p.z0 <= p.z1, "un pavé est à l'endroit : {p:?}");
        assert!(
            (p.x1 as usize) < largeur && (p.y1 as usize) < hauteur && (p.z1 as usize) < profondeur,
            "un pavé est dans le cube : {p:?}"
        );
    }
    // Triés par (z0, y0, x0).
    for couple in paves.windows(2) {
        let (a, b) = (&couple[0], &couple[1]);
        assert!((a.z0, a.y0, a.x0) <= (b.z0, b.y0, b.x0), "les pavés sont triés par (z0, y0, x0) : {a:?} puis {b:?}");
    }
    // Deux à deux disjoints, et d'union exactement le complément de la cavité.
    let mut couvert = vec![false; largeur * hauteur * profondeur];
    for p in &paves {
        for z in p.z0 as usize..=p.z1 as usize {
            for y in p.y0 as usize..=p.y1 as usize {
                for x in p.x0 as usize..=p.x1 as usize {
                    let i = (z * hauteur + y) * largeur + x;
                    assert!(!couvert[i], "deux pavés sont disjoints : la case ({x}, {y}, {z}) est deux fois");
                    couvert[i] = true;
                }
            }
        }
    }
    for (i, (&couvert, &libre)) in couvert.iter().zip(&cases).enumerate() {
        assert_eq!(couvert, !libre, "l'union des pavés est le complément de la cavité, case {i}");
    }
    paves
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn la_decoupe_est_disjointe_d_union_le_complement_et_triee() {
    let formes: Vec<(usize, usize, usize, Forme3d)> = vec![
        (5, 5, 5, boule(2, 2, 2, 0)),
        (5, 5, 5, boule(2, 2, 2, 1)),
        (5, 5, 5, boule(2, 2, 2, 2)),
        (7, 7, 3, tore(3, 3, 1, 2, 1)),
        (11, 11, 5, tore(5, 5, 2, 3, 2)),
        (32, 32, 32, tore(16, 16, 16, 10, 0)),
        (6, 6, 6, coquille(1, 1, 1, 4, 4, 4, 1)),
        (6, 6, 6, coquille(1, 1, 1, 4, 4, 4, 2)),
        (4, 4, 4, pave(1, 1, 1, 2, 2, 2)),
        (4, 4, 4, pave(0, 0, 0, 3, 3, 3)),
        (32, 32, 32, boule(16, 16, 16, 14)),
        (32, 32, 32, tore(16, 16, 16, 10, 4)),
        (32, 32, 32, coquille(2, 2, 2, 29, 29, 29, 3)),
        (16, 16, 16, labyrinthe(3, 3, 3, 4, 1)),
        (16, 16, 16, labyrinthe(3, 3, 3, 4, 4)),
        (32, 32, 32, labyrinthe(4, 4, 4, 6, 1)),
        (7, 4, 4, labyrinthe(2, 1, 1, 2, 9)),
        (5, 5, 9, labyrinthe(1, 1, 2, 3, 0)),
    ];
    assert_eq!(formes.len(), 18, "toutes les formes des tests 1 et 4 sont découpées");
    for (largeur, hauteur, profondeur, forme) in &formes {
        verifier_la_decoupe(*largeur, *hauteur, *profondeur, forme);
    }

    // Une cavité qui est tout le cube donne une liste vide.
    let tout = verifier_la_decoupe(4, 4, 4, &pave(0, 0, 0, 3, 3, 3));
    assert!(tout.is_empty(), "une cavité qui est tout le cube donne une liste vide");

    // Les nombres de pavés écrits.
    assert_eq!(obstacles_3d(32, 32, 32, &boule(16, 16, 16, 14)).unwrap().len(), 570, "la boule 14 : 570 pavés");
    assert_eq!(obstacles_3d(32, 32, 32, &tore(16, 16, 16, 10, 4)).unwrap().len(), 291, "le tore 10/4 : 291 pavés");
    assert_eq!(
        obstacles_3d(32, 32, 32, &coquille(2, 2, 2, 29, 29, 29, 3)).unwrap().len(),
        7,
        "la coquille d'épaisseur 3 : 7 pavés"
    );
    assert_eq!(
        obstacles_3d(16, 16, 16, &labyrinthe(3, 3, 3, 4, 4)).unwrap().len(),
        37,
        "le labyrinthe 3 3 3 de couloir 4, graine 4 : 37 pavés"
    );

    // Le cas exact : le pavé 1..=2 dans 4³, dans l'ordre.
    assert_eq!(
        obstacles_3d(4, 4, 4, &pave(1, 1, 1, 2, 2, 2)).unwrap(),
        vec![
            bloc(0, 0, 0, 3, 3, 0),
            bloc(0, 0, 1, 3, 0, 2),
            bloc(0, 1, 1, 0, 2, 2),
            bloc(3, 1, 1, 3, 2, 2),
            bloc(0, 3, 1, 3, 3, 2),
            bloc(0, 0, 3, 3, 3, 3),
        ],
        "le pavé 1..=2 dans 4³ : les six pavés du complément, dans l'ordre"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// 4. Le labyrinthe
// ---------------------------------------------------------------------------------------------------------------------

/// Un cas de labyrinthe : ses chambres, son couloir, sa graine, sa cavité et, quand ils sont écrits, son sha et ses pavés.
struct CasLabyrinthe {
    largeur: usize,
    hauteur: usize,
    profondeur: usize,
    colonnes: usize,
    rangees: usize,
    etages: usize,
    couloir: usize,
    graine: u64,
    cases_attendues: usize,
    sha_attendu: Option<&'static str>,
    paves_attendus: Option<usize>,
}

/// Vérifie la géométrie, la perfection et la connexité d'un labyrinthe.
fn verifier_labyrinthe(cas: &CasLabyrinthe) {
    let (w, h, p) = (cas.largeur, cas.hauteur, cas.profondeur);
    let (colonnes, rangees, etages, couloir) = (cas.colonnes, cas.rangees, cas.etages, cas.couloir);
    let q = couloir + 1;
    let forme = labyrinthe(colonnes, rangees, etages, couloir, cas.graine);

    let cases = libres(w, h, p, &forme).expect("le labyrinthe tient dans le cube");
    assert_eq!(cases.len(), w * h * p, "un booléen par case du cube");
    assert_eq!(libres(w, h, p, &forme).unwrap(), cases, "le même appel rend le même tableau");

    // Le nombre de cases libres : la formule, et le nombre écrit.
    let n = colonnes * rangees * etages;
    let formule = n * couloir * couloir * couloir + (n - 1) * couloir * couloir;
    assert_eq!(compter(&cases), cas.cases_attendues, "le nombre de cases libres écrit");
    assert_eq!(compter(&cases), formule, "N couloir³ + (N − 1) couloir², avec N = {n} chambres et couloir {couloir}");

    // Chaque chambre est libre en entier, et `chambre_3d` la donne exactement.
    let mut chambres = vec![false; w * h * p];
    for e in 0..etages {
        for r in 0..rangees {
            for c in 0..colonnes {
                let (x0, y0, z0, x1, y1, z1) = chambre_3d(couloir, c, r, e);
                assert!(x0 <= x1 && y0 <= y1 && z0 <= z1, "une chambre est un pavé à l'endroit");
                for z in z0..=z1 {
                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            let i = (z * h + y) * w + x;
                            assert!(cases[i], "la chambre ({c}, {r}, {e}) est libre en entier, case ({x}, {y}, {z})");
                            chambres[i] = true;
                        }
                    }
                }
            }
        }
    }

    // Les murs communs à deux chambres voisines : un passage entier, ou un mur plein.
    let identifiant = |c: usize, r: usize, e: usize| (e * rangees + r) * colonnes + c;
    let mut passages = vec![false; w * h * p];
    let mut liens: Vec<(usize, usize)> = Vec::new();
    for e in 0..etages {
        for r in 0..rangees {
            for c in 0..colonnes.saturating_sub(1) {
                let mut toutes_libres = true;
                let mut toutes_bloquees = true;
                for z in e * q + 1..=e * q + couloir {
                    for y in r * q + 1..=r * q + couloir {
                        if cases[(z * h + y) * w + (c + 1) * q] {
                            toutes_bloquees = false;
                        } else {
                            toutes_libres = false;
                        }
                    }
                }
                assert!(
                    toutes_libres || toutes_bloquees,
                    "le mur en x entre ({c}, {r}, {e}) et ({}, {r}, {e}) est un passage entier ou un mur plein",
                    c + 1
                );
                if toutes_libres {
                    for z in e * q + 1..=e * q + couloir {
                        for y in r * q + 1..=r * q + couloir {
                            passages[(z * h + y) * w + (c + 1) * q] = true;
                        }
                    }
                    liens.push((identifiant(c, r, e), identifiant(c + 1, r, e)));
                }
            }
        }
    }
    for e in 0..etages {
        for r in 0..rangees.saturating_sub(1) {
            for c in 0..colonnes {
                let mut toutes_libres = true;
                let mut toutes_bloquees = true;
                for z in e * q + 1..=e * q + couloir {
                    for x in c * q + 1..=c * q + couloir {
                        if cases[(z * h + (r + 1) * q) * w + x] {
                            toutes_bloquees = false;
                        } else {
                            toutes_libres = false;
                        }
                    }
                }
                assert!(
                    toutes_libres || toutes_bloquees,
                    "le mur en y entre ({c}, {r}, {e}) et ({c}, {}, {e}) est un passage entier ou un mur plein",
                    r + 1
                );
                if toutes_libres {
                    for z in e * q + 1..=e * q + couloir {
                        for x in c * q + 1..=c * q + couloir {
                            passages[(z * h + (r + 1) * q) * w + x] = true;
                        }
                    }
                    liens.push((identifiant(c, r, e), identifiant(c, r + 1, e)));
                }
            }
        }
    }
    for e in 0..etages.saturating_sub(1) {
        for r in 0..rangees {
            for c in 0..colonnes {
                let mut toutes_libres = true;
                let mut toutes_bloquees = true;
                for y in r * q + 1..=r * q + couloir {
                    for x in c * q + 1..=c * q + couloir {
                        if cases[((e + 1) * q * h + y) * w + x] {
                            toutes_bloquees = false;
                        } else {
                            toutes_libres = false;
                        }
                    }
                }
                assert!(
                    toutes_libres || toutes_bloquees,
                    "le mur en z entre ({c}, {r}, {e}) et ({c}, {r}, {}) est un passage entier ou un mur plein",
                    e + 1
                );
                if toutes_libres {
                    for y in r * q + 1..=r * q + couloir {
                        for x in c * q + 1..=c * q + couloir {
                            passages[((e + 1) * q * h + y) * w + x] = true;
                        }
                    }
                    liens.push((identifiant(c, r, e), identifiant(c, r, e + 1)));
                }
            }
        }
    }
    assert_eq!(liens.len(), n - 1, "il y a exactement N − 1 passages");

    // Hors des chambres et des murs de passage, tout est bloqué ; au-delà de l'emprise aussi.
    for z in 0..p {
        for y in 0..h {
            for x in 0..w {
                let i = (z * h + y) * w + x;
                if chambres[i] || passages[i] {
                    assert!(cases[i], "une case de chambre ou de passage est libre : ({x}, {y}, {z})");
                } else {
                    assert!(!cases[i], "hors des chambres et des passages, tout est bloqué : ({x}, {y}, {z})");
                }
            }
        }
    }
    let (xl, yl, zl) = (colonnes * q + 1, rangees * q + 1, etages * q + 1);
    for z in 0..p {
        for y in 0..h {
            for x in 0..w {
                if x >= xl || y >= yl || z >= zl {
                    assert!(
                        !cases[(z * h + y) * w + x],
                        "au-delà de l'emprise du labyrinthe, tout est bloqué : ({x}, {y}, {z})"
                    );
                }
            }
        }
    }

    // Le graphe des chambres et des passages est connexe : c'est un arbre.
    let mut adjacence = vec![Vec::new(); n];
    for &(a, b) in &liens {
        adjacence[a].push(b);
        adjacence[b].push(a);
    }
    let mut vues_chambres = vec![false; n];
    vues_chambres[0] = true;
    let mut pile = vec![0usize];
    while let Some(sommet) = pile.pop() {
        for &voisine in &adjacence[sommet] {
            if !vues_chambres[voisine] {
                vues_chambres[voisine] = true;
                pile.push(voisine);
            }
        }
    }
    assert!(vues_chambres.iter().all(|&vue| vue), "le graphe des chambres est connexe, donc un arbre");

    // Toutes les cases libres sont reliées par leurs six voisines.
    let premiere = cases.iter().position(|&libre| libre).expect("un labyrinthe a des cases libres");
    let mut vues_cases = vec![false; w * h * p];
    vues_cases[premiere] = true;
    let mut pile = vec![premiere];
    while let Some(i) = pile.pop() {
        let (x, y, z) = (i % w, (i / w) % h, i / (w * h));
        let mut voisines = Vec::with_capacity(6);
        if x > 0 {
            voisines.push(i - 1);
        }
        if x + 1 < w {
            voisines.push(i + 1);
        }
        if y > 0 {
            voisines.push(i - w);
        }
        if y + 1 < h {
            voisines.push(i + w);
        }
        if z > 0 {
            voisines.push(i - w * h);
        }
        if z + 1 < p {
            voisines.push(i + w * h);
        }
        for voisine in voisines {
            if cases[voisine] && !vues_cases[voisine] {
                vues_cases[voisine] = true;
                pile.push(voisine);
            }
        }
    }
    for (i, &libre) in cases.iter().enumerate() {
        if libre {
            assert!(vues_cases[i], "toutes les cases libres sont reliées par leurs six voisines : case {i}");
        }
    }

    // Le sha et le nombre de pavés, quand ils sont écrits.
    if let Some(attendu) = cas.sha_attendu {
        assert_eq!(
            sha(&cases),
            attendu,
            "le labyrinthe {}x{}x{} de couloir {couloir}, graine {} : le sha de la cavité",
            colonnes,
            rangees,
            etages,
            cas.graine
        );
    }
    if let Some(attendu) = cas.paves_attendus {
        let paves = obstacles_3d(w, h, p, &forme).expect("le labyrinthe, la même découpe");
        assert_eq!(
            paves.len(),
            attendu,
            "le labyrinthe {}x{}x{} de couloir {couloir}, graine {} : les pavés",
            colonnes,
            rangees,
            etages,
            cas.graine
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn le_labyrinthe_est_parfait_et_remplit_sa_formule() {
    assert_eq!(chambre_3d(4, 2, 0, 1), (11, 1, 6, 14, 4, 9), "chambre_3d(4, 2, 0, 1)");
    assert_eq!(centre_de_la_chambre_3d(4, 0, 0, 0), (3, 3, 3), "centre_de_la_chambre_3d(4, 0, 0, 0)");
    assert_eq!(centre_de_la_chambre_3d(4, 2, 2, 2), (13, 13, 13), "centre_de_la_chambre_3d(4, 2, 2, 2)");

    let cas = [
        CasLabyrinthe {
            largeur: 16,
            hauteur: 16,
            profondeur: 16,
            colonnes: 3,
            rangees: 3,
            etages: 3,
            couloir: 4,
            graine: 1,
            cases_attendues: 2144,
            sha_attendu: Some("440ce43b18ce895f52d763bb7f86f64e66aad3710334f3758fb7a5fad45b97ed"),
            paves_attendus: Some(36),
        },
        CasLabyrinthe {
            largeur: 16,
            hauteur: 16,
            profondeur: 16,
            colonnes: 3,
            rangees: 3,
            etages: 3,
            couloir: 4,
            graine: 4,
            cases_attendues: 2144,
            sha_attendu: Some("3eb61bd2c7f02961c883dda3d3f319fd337efd66fb6db5b1034d9265bda911b3"),
            paves_attendus: Some(37),
        },
        CasLabyrinthe {
            largeur: 32,
            hauteur: 32,
            profondeur: 32,
            colonnes: 4,
            rangees: 4,
            etages: 4,
            couloir: 6,
            graine: 1,
            cases_attendues: 16092,
            sha_attendu: Some("0f28ae0bcaba8ce5dc0b2410c3a1db82c5803d57f16c724b758a79593183ab0d"),
            paves_attendus: Some(79),
        },
        CasLabyrinthe {
            largeur: 7,
            hauteur: 4,
            profondeur: 4,
            colonnes: 2,
            rangees: 1,
            etages: 1,
            couloir: 2,
            graine: 9,
            cases_attendues: 20,
            sha_attendu: None,
            paves_attendus: None,
        },
        CasLabyrinthe {
            largeur: 5,
            hauteur: 5,
            profondeur: 9,
            colonnes: 1,
            rangees: 1,
            etages: 2,
            couloir: 3,
            graine: 0,
            cases_attendues: 63,
            sha_attendu: None,
            paves_attendus: None,
        },
    ];
    assert_eq!(cas.len(), 5, "les cinq labyrinthes écrits sont vérifiés");
    for cas in &cas {
        verifier_labyrinthe(cas);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// 5. Le texte d'un scénario
// ---------------------------------------------------------------------------------------------------------------------

/// Le texte développé du pavé 1..=2 dans 4³ : le `cube` puis les six lignes `obstacle` du test 3.
const SIX_OBSTACLES: &str = "\
obstacle 0 0 0 3 3 0
obstacle 0 0 1 3 0 2
obstacle 0 1 1 0 2 2
obstacle 3 1 1 3 2 2
obstacle 0 3 1 3 3 2
obstacle 0 0 3 3 3 3
";

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn sans_ligne_moule_le_texte_est_rendu_octet_pour_octet() {
    let scenarios =
        ["colonie-cube.txt", "colonie-goutte.txt", "colonie-chimio.txt", "colonie-mur.txt", "colonie-anastomose.txt"];
    assert_eq!(scenarios.len(), 5, "les cinq scènes sont lues");
    for nom in scenarios {
        let texte = texte_du_fichier(nom);
        assert!(!texte.is_empty(), "{nom} : le texte lu n'est pas vide");
        assert_eq!(developper_3d(&texte).unwrap(), texte, "{nom} : sans ligne moule, le texte est rendu tel quel");
    }

    // Des fins `\r\n`, des commentaires, des lignes vides, et une dernière ligne sans saut de ligne.
    let texte = "cube 8 8 8\r\n# un commentaire\r\n\r\n   \r\nspore 4 4 4\r\ncroissance 1.0 infini 30 0\r\ncouplage 1.0 0.0\r\nduree 5";
    assert_eq!(developper_3d(texte).unwrap(), texte, "aucune ligne moule : le texte est conservé octet pour octet");
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn avec_ligne_moule_elle_est_remplacee_en_place() {
    // Le cas exact : une ligne `moule` seule, remplacée par les six lignes `obstacle`.
    let exact = "cube 4 4 4\nmoule pave 1 1 1 2 2 2\n";
    let attendu = format!("cube 4 4 4\n{SIX_OBSTACLES}");
    assert_eq!(developper_3d(exact).unwrap(), attendu, "le cas exact du pavé 1..=2 dans 4³");

    // Les lignes d'avant et d'après sont gardées à leur place, telles quelles, la dernière sans saut de ligne.
    let texte = "cube 4 4 4\n# avant\nmoule pave 1 1 1 2 2 2\nspore 2 2 2\nduree 1";
    let attendu = format!("cube 4 4 4\n# avant\n{SIX_OBSTACLES}spore 2 2 2\nduree 1");
    assert_eq!(developper_3d(texte).unwrap(), attendu, "l'avant, les obstacles, puis l'après");

    // Les quatre textes des scènes : le nombre de lignes et le sha des octets.
    let scenes = [
        (
            "cube 32 32 32\nmoule boule 16 16 16 14\n",
            571usize,
            "303ee4ecb810e9635fdbeb82cca7813e043e2497880ca3997b5e4eebda6c7d07",
        ),
        (
            "cube 32 32 32\nmoule tore 16 16 16 10 4\n",
            292,
            "f7df0d593b2c10fa93bd9cfaed63acd04ef12a698cfeead43505512c2128e4a5",
        ),
        (
            "cube 32 32 32\nmoule coquille 2 2 2 29 29 29 3\n",
            8,
            "1792991b085ab175823e54063251c7e4106d2577ba608ef0582f7dc712e1ad40",
        ),
        (
            "cube 16 16 16\nmoule labyrinthe 3 3 3 4 4\n",
            38,
            "79a306f78cf3638da38a585515b580e0c67e3e312b060239a8c2a292b7267409",
        ),
    ];
    assert_eq!(scenes.len(), 4, "les quatre textes développés sont vérifiés");
    for (texte, lignes, empreinte) in scenes {
        let developpe = developper_3d(texte).unwrap_or_else(|e| panic!("{texte:?} : refus inattendu {e:?}"));
        assert_eq!(developpe.lines().count(), lignes, "{texte:?} : le nombre de lignes");
        assert_eq!(sha256_hex(developpe.as_bytes()), empreinte, "{texte:?} : le sha des octets");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn les_lignes_moule_mal_formees_sont_refusees_dans_l_ordre() {
    let invalide = |numero| ErreurMoule3d::LigneInvalide(numero);
    let refusees = [
        "moule sphere 4 4 4 2",
        "moule boule 4 4 4",
        "moule boule 4 4 4 2 1",
        "moule boule +4 4 4 2",
        "moule boule 4 4 4 -2",
        "moule boule 4 4 4 2.0",
        "moule labyrinthe 1 1 1 1 18446744073709551616",
    ];
    assert_eq!(refusees.len(), 7, "les sept lignes mal formées sont essayées");
    for ligne in refusees {
        let texte = format!("cube 8 8 8\n{ligne}\n");
        assert_eq!(developper_3d(&texte).unwrap_err(), invalide(2), "à la ligne 2 : `{ligne}`");
    }
    // La plus grande graine tient dans un u64 et se lit.
    assert!(
        developper_3d("cube 8 8 8\nmoule labyrinthe 1 1 1 1 18446744073709551615\n").is_ok(),
        "usize::MAX de la graine se lit"
    );

    // Deux lignes `moule` valides : une double ; la seconde mal formée : invalide à sa ligne.
    assert_eq!(
        developper_3d("cube 8 8 8\nmoule boule 4 4 4 1\nmoule boule 4 4 4 1\n").unwrap_err(),
        ErreurMoule3d::MoulesEnDouble,
        "deux lignes moule"
    );
    assert_eq!(
        developper_3d("cube 8 8 8\nmoule boule 4 4 4 1\nmoule sphere 1 2 3\n").unwrap_err(),
        invalide(3),
        "une seconde ligne mal formée, à sa ligne"
    );

    // Une ligne `moule` sans ligne `cube` bien formée avant elle.
    assert_eq!(developper_3d("moule boule 4 4 4 2\n").unwrap_err(), ErreurMoule3d::CubeAbsent);
    assert_eq!(
        developper_3d("moule boule 4 4 4 2\ncube 8 8 8\n").unwrap_err(),
        ErreurMoule3d::CubeAbsent,
        "un cube après seulement"
    );
    assert_eq!(
        developper_3d("cube 8 8\nmoule boule 4 4 4 2\n").unwrap_err(),
        ErreurMoule3d::CubeAbsent,
        "un cube à deux nombres"
    );
    assert_eq!(
        developper_3d("cube 8 8 x\nmoule boule 4 4 4 2\n").unwrap_err(),
        ErreurMoule3d::CubeAbsent,
        "un cube au troisième mot illisible"
    );

    // Puis les erreurs de `obstacles_3d`.
    assert_eq!(
        developper_3d("cube 8 8 8\nmoule tore 4 4 4 2 2\n").unwrap_err(),
        ErreurMoule3d::FormeInvalide,
        "un tore de petit rayon égal au grand"
    );
    assert_eq!(
        developper_3d("cube 8 8 8\nmoule boule 4 4 4 5\n").unwrap_err(),
        ErreurMoule3d::HorsDuCube,
        "une boule qui sort du cube"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn un_scenario_de_colonie_avec_moule_se_developpe_se_lit_et_se_construit() {
    let texte = "cube 16 16 16\nsubstrat 1.0 0.1\nmoule boule 8 8 8 6\nspore 8 8 8\ncroissance 1.0 6 35 20\ncouplage 0.1 0.5\nduree 10\n";
    let developpe = developper_3d(texte).expect("la boule de rayon 6 tient dans 16³");
    let scenario = lire_scenario_colonie3d(&developpe).expect("le texte développé se lit");
    let colonie = Colonie3d::nouvelle(&scenario, 1).expect("la colonie de la boule se construit");
    assert_eq!(colonie.scenario().spore, (8, 8, 8), "la spore est au centre de la boule");
    assert!(!colonie.substrat().est_bloquee(8, 8, 8), "la case de la spore, au centre, est dans la cavité");

    // La spore hors de la cavité : la lecture du texte développé la refuse.
    let texte = "cube 16 16 16\nsubstrat 1.0 0.1\nmoule boule 8 8 8 6\nspore 1 1 1\ncroissance 1.0 6 35 20\ncouplage 0.1 0.5\nduree 10\n";
    let developpe = developper_3d(texte).expect("le moule est valide, la spore n'est pas son affaire");
    let resultat = lire_scenario_colonie3d(&developpe);
    assert!(
        matches!(&resultat, Err(ErreurColonie3d::DansUnObstacle(_))),
        "la spore en (1, 1, 1), hors de la cavité, est dans un obstacle : {resultat:?}"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// 6. Le remplissage
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn le_remplissage_de_colonie_cube_est_ecrit() {
    let colonie = colonie_finie("colonie-cube.txt", 1);
    assert_eq!(colonie.temps(), colonie.duree(), "colonie-cube : le temps est la durée");
    let atteintes = cases_atteintes(&colonie);
    assert_eq!(atteintes.len(), 32 * 32 * 32, "un booléen par case du cube");
    assert_eq!(compter(&atteintes), 828, "colonie-cube : 828 cases atteintes");
    assert_eq!(part_remplie(&colonie), 828.0 / 32768.0, "colonie-cube : la part, exactement");
    assert_eq!(part_remplie(&colonie), 0.0252685546875, "colonie-cube : la part écrite");
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn le_rejeu_des_segments_redonne_les_cases_atteintes_et_la_part() {
    for (nom, graine) in [("colonie-cube.txt", 1u64), ("colonie-mur.txt", 1), ("colonie-anastomose.txt", 1)] {
        let colonie = colonie_finie(nom, graine);
        let rejeu = rejeu_des_atteintes(&colonie);
        assert_eq!(cases_atteintes(&colonie), rejeu, "{nom} : le rejeu des arrivées redonne les cases atteintes");
        let libres = cases_libres_du_substrat(&colonie);
        assert!(libres > 0, "{nom} : la cavité n'est pas vide");
        assert_eq!(
            part_remplie(&colonie),
            compter(&rejeu) as f64 / libres as f64,
            "{nom} : la part est les cases atteintes sur les cases libres"
        );
    }

    // La scène de la boule, à la fin de sa durée.
    let mut colonie = colonie_developpee(BOULE, 1);
    colonie.avancer_n(colonie.duree());
    let rejeu = rejeu_des_atteintes(&colonie);
    assert_eq!(cases_atteintes(&colonie), rejeu, "la boule : le rejeu des arrivées redonne les cases atteintes");
    assert_eq!(compter(&rejeu), 8705, "la boule : 8705 cases atteintes à la fin");
    let libres = cases_libres_du_substrat(&colonie);
    assert_eq!(libres, 11513, "la boule : 11513 cases libres");
    assert_eq!(
        part_remplie(&colonie),
        compter(&rejeu) as f64 / libres as f64,
        "la boule : la part est les cases atteintes sur les cases libres"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn la_courbe_de_la_boule_ne_decroit_jamais_et_finit_a_la_part_ecrite() {
    let mut colonie = colonie_developpee(BOULE, 1);
    let courbe = courbe_de_remplissage_3d(&mut colonie, 100);
    assert_eq!(courbe.len(), 101, "un point par pas, de 0 à 100");
    for (pas, point) in courbe.iter().enumerate() {
        assert_eq!(point.pas, pas as u32, "le pas du point {pas}");
        assert_eq!(point.libres, 11513, "la boule : 11513 cases libres à chaque point");
        if pas > 0 {
            assert!(
                point.atteintes >= courbe[pas - 1].atteintes,
                "les cases atteintes ne décroissent jamais, au pas {pas}"
            );
        }
    }
    let dernier = courbe.last().expect("la courbe a un dernier point");
    assert_eq!(dernier.pas, 100, "le dernier point est au pas 100");
    assert_eq!(dernier.atteintes, 8705, "la boule : 8705 cases atteintes au pas 100");
    assert_eq!(dernier.part, 0.756101797967515, "la boule : la part écrite");
    assert_eq!(dernier.part, 8705.0 / 11513.0, "la boule : 8705 divisé par 11513");
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn le_cas_droite_suit_les_cases_atteintes_ecrites() {
    let attendues: [&[u64]; 3] =
        [&[0, 1, 2, 2, 3, 3, 3, 3, 3, 3, 3], &[0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2], &[0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2]];
    for (indice, attendu) in attendues.iter().enumerate() {
        let graine = indice as u64 + 1;
        let mut colonie = colonie_developpee(DROITE, graine);
        let courbe = courbe_de_remplissage_3d(&mut colonie, 10);
        assert_eq!(courbe.len(), 11, "graine {graine} : un point par pas, de 0 à 10");
        for point in &courbe {
            assert_eq!(point.libres, 64, "graine {graine} : le pavé 2..=5 dans 8³ a 64 cases libres");
        }
        let compte: Vec<u64> = courbe.iter().map(|point| point.atteintes).collect();
        assert_eq!(compte.as_slice(), *attendu, "graine {graine} : les cases atteintes pas à pas");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn la_courbe_d_une_colonie_deja_a_son_temps_ne_l_avance_pas() {
    let mut colonie = colonie_developpee(DROITE, 1);
    colonie.avancer_n(10);
    assert_eq!(colonie.temps(), 10, "la colonie est au pas 10");

    let courbe = courbe_de_remplissage_3d(&mut colonie, 10);
    assert_eq!(courbe.len(), 1, "au temps demandé : un seul point");
    assert_eq!(courbe[0].pas, 10, "le point est au temps présent");
    assert_eq!(colonie.temps(), 10, "la colonie n'avance pas");

    let courbe = courbe_de_remplissage_3d(&mut colonie, 4);
    assert_eq!(courbe.len(), 1, "au-delà du temps demandé : un seul point");
    assert_eq!(courbe[0].pas, 10, "toujours au temps présent");
    assert_eq!(colonie.temps(), 10, "la colonie n'avance pas");

    let courbe = courbe_de_remplissage_3d(&mut colonie, 13);
    assert_eq!(courbe.len(), 4, "les points des pas 10 à 13");
    assert_eq!(courbe.first().expect("un point").pas, 10, "le premier point est au temps présent");
    assert_eq!(courbe.last().expect("un point").pas, 13, "le dernier point est au pas demandé");
    assert_eq!(colonie.temps(), 13, "la colonie est avancée en place jusqu'au pas demandé");
}

// ---------------------------------------------------------------------------------------------------------------------
// 7. Sans moule, rien ne change
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-26"), ignore)]
fn sans_moule_les_cinq_scenes_gardent_l_empreinte_de_leur_fichier() {
    let fichiers = [
        ("colonie-cube.txt", "colonie-cube.empreinte"),
        ("colonie-goutte.txt", "colonie-goutte.empreinte"),
        ("colonie-chimio.txt", "colonie-chimio.empreinte"),
        ("colonie-mur.txt", "colonie-mur.empreinte"),
        ("colonie-anastomose.txt", "colonie-anastomose.empreinte"),
    ];
    assert_eq!(fichiers.len(), 5, "les cinq scènes sont vérifiées");
    for (nom, reference) in fichiers {
        let texte = texte_du_fichier(nom);
        let developpe = developper_3d(&texte).expect("sans moule, le texte est rendu tel quel");
        assert_eq!(developpe, texte, "{nom} : sans moule, rien ne change dans le texte");
        let scenario = lire_scenario_colonie3d(&developpe).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"));
        let mut colonie =
            Colonie3d::nouvelle(&scenario, 1).unwrap_or_else(|e| panic!("{nom} : construction refusée {e:?}"));
        colonie.avancer_n(colonie.duree());
        assert_eq!(colonie.temps(), colonie.duree(), "{nom} : le temps est la durée");
        assert_eq!(colonie.empreinte(), empreinte_de_reference(reference), "{nom} : l'empreinte de son fichier");
    }
}
