//! Tests d'acceptation de l'étape 3, tests 4 et 5 : le déterminisme de la colonie de la figure 3 et les fonctions
//! de mesure de sa croissance.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt`, et ne parcourent aucune
//! `HashMap` ni aucun `HashSet`.

use mycelium_lab::voisinage::{Colonie, Parametres, ajuster, ajuster_rayon, exposant, vitesses_glissantes};
use std::sync::OnceLock;

/// Nombre de pas joués par les colonies de référence.
const PAS: u32 = 150;

/// La graine 7, avancée en bloc : la colonie dont l'empreinte est publiée.
static GRAINE_7: OnceLock<Colonie> = OnceLock::new();
/// La graine 7, avancée en bloc une seconde fois : mêmes octets et même empreinte.
static GRAINE_7_BIS: OnceLock<Colonie> = OnceLock::new();
/// La graine 8, avancée en bloc : une autre empreinte.
static GRAINE_8: OnceLock<Colonie> = OnceLock::new();
/// La graine 7, avancée pas à pas : le même état que `avancer_n(PAS)`.
static GRAINE_7_PAS_A_PAS: OnceLock<Colonie> = OnceLock::new();

fn colonie_en_bloc(graine: u64) -> Colonie {
    let mut colonie = Colonie::depuis_spore((0.0, 0.0), Parametres::figure_3(), graine);
    colonie.avancer_n(PAS);
    colonie
}

fn colonie_pas_a_pas(graine: u64) -> Colonie {
    let mut colonie = Colonie::depuis_spore((0.0, 0.0), Parametres::figure_3(), graine);
    for _ in 0..PAS {
        colonie.avancer();
    }
    colonie
}

fn graine_7() -> &'static Colonie {
    GRAINE_7.get_or_init(|| colonie_en_bloc(7))
}

fn graine_7_bis() -> &'static Colonie {
    GRAINE_7_BIS.get_or_init(|| colonie_en_bloc(7))
}

fn graine_8() -> &'static Colonie {
    GRAINE_8.get_or_init(|| colonie_en_bloc(8))
}

fn graine_7_pas_a_pas() -> &'static Colonie {
    GRAINE_7_PAS_A_PAS.get_or_init(|| colonie_pas_a_pas(7))
}

/// Vrai si `valeur` est à `tolerance` de `attendu`, sans `abs` ni `max` : seul l'écart au carré est comparé.
fn proche(valeur: f64, attendu: f64, tolerance: f64) -> bool {
    let ecart = valeur - attendu;
    ecart * ecart <= tolerance * tolerance
}

/// 4. Le déterminisme : même graine, mêmes octets et même empreinte ; deux graines, deux empreintes ;
///    `avancer_n(PAS)` équivaut à `PAS` appels à `avancer` ; l'empreinte de la graine 7 est celle de référence.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn determinisme_de_la_colonie() {
    let premiere = graine_7();
    let seconde = graine_7_bis();
    let empreinte = premiere.empreinte();

    assert_eq!(premiere.octets(), seconde.octets(), "même graine, même écriture");
    assert_eq!(empreinte, seconde.empreinte(), "même graine, même empreinte");
    assert_eq!(empreinte.len(), 64, "empreinte : {empreinte}");
    assert!(
        empreinte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "empreinte non hexadécimale minuscule : {empreinte}"
    );

    assert_ne!(empreinte, graine_8().empreinte(), "deux graines, deux empreintes");

    let pas_a_pas = graine_7_pas_a_pas();
    assert_eq!(premiere.octets(), pas_a_pas.octets(), "avancer_n(150) doit égaler 150 appels à avancer");
    assert_eq!(empreinte, pas_a_pas.empreinte(), "avancer_n(150) : même empreinte");

    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("references/voisinage-graine-7.empreinte");
    let reference = std::fs::read_to_string(&chemin).expect("le fichier de référence doit exister");
    assert_eq!(reference.len(), 65, "la référence fait 64 caractères et un saut de ligne : {reference:?}");
    assert_eq!(reference, format!("{empreinte}\n"), "empreinte de référence de la graine 7");
}

/// 5. Les fonctions de mesure : régression exacte et bruitée, vitesses glissantes d'une droite, exposant d'une loi
///    de puissance et pente du rayon.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn mesures_de_la_croissance() {
    // Une droite parfaite : pente 2, ordonnée 1, R² 1.
    let exact = ajuster(&[0.0, 1.0, 2.0, 3.0], &[1.0, 3.0, 5.0, 7.0]);
    assert!(proche(exact.pente, 2.0, 1e-12), "pente : {}", exact.pente);
    assert!(proche(exact.ordonnee, 1.0, 1e-12), "ordonnée : {}", exact.ordonnee);
    assert!(proche(exact.r2, 1.0, 1e-12), "R² : {}", exact.r2);

    // Un nuage bruité : le R², strictement entre 0 et 1, se recalcule ici à partir des données et de la droite
    // ajustée, sans appeler la fonction de régression.
    let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.0, 3.0, 2.0, 5.0, 4.0, 7.0];
    let bruite = ajuster(&x, &y);
    let somme: f64 = y.iter().sum();
    let moyenne = somme / y.len() as f64;
    let mut sct = 0.0;
    let mut scr = 0.0;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        let ecart = yi - moyenne;
        sct += ecart * ecart;
        let residu = yi - (bruite.pente * xi + bruite.ordonnee);
        scr += residu * residu;
    }
    let r2_attendu = 1.0 - scr / sct;
    assert!(proche(bruite.r2, r2_attendu, 1e-12), "R² : {} au lieu de {r2_attendu}", bruite.r2);
    assert!(bruite.r2 > 0.0 && bruite.r2 < 1.0, "R² strictement entre 0 et 1 : {}", bruite.r2);
    // La droite et le R² de ce nuage sont connus exactement : 36/35, 23/21 et 972/1225.
    assert!(proche(bruite.pente, 36.0 / 35.0, 1e-12), "pente du nuage : {}", bruite.pente);
    assert!(proche(bruite.ordonnee, 23.0 / 21.0, 1e-12), "ordonnée du nuage : {}", bruite.ordonnee);
    assert!(proche(bruite.r2, 972.0 / 1225.0, 1e-12), "R² du nuage : {}", bruite.r2);

    // `r(t) = 0,9 t` : les seize fenêtres de `vitesses_glissantes` ont toutes la pente 0,9.
    let mut rayons = Vec::new();
    for t in 1..=400u32 {
        rayons.push(0.9 * t as f64);
    }
    let vitesses = vitesses_glissantes(&rayons, 200, 400, 50, 10);
    assert_eq!(vitesses.len(), 16, "seize fenêtres");
    for (rang, pente) in vitesses.iter().enumerate() {
        assert!(proche(*pente, 0.9, 1e-9), "fenêtre {rang} : pente {pente}");
    }

    // `r(t) = 3 t` a pour exposant 1 ; `r(t) = t * t` a pour exposant 2.
    let mut lineaire = Vec::new();
    let mut quadratique = Vec::new();
    for t in 1..=100u32 {
        let temps = t as f64;
        lineaire.push(3.0 * temps);
        quadratique.push(temps * temps);
    }
    let exposant_lineaire = exposant(&lineaire, 1, 100);
    assert!(proche(exposant_lineaire, 1.0, 1e-9), "exposant linéaire : {exposant_lineaire}");
    let exposant_quadratique = exposant(&quadratique, 1, 100);
    assert!(proche(exposant_quadratique, 2.0, 1e-9), "exposant quadratique : {exposant_quadratique}");

    // La régression du rayon sur le temps retrouve la pente 0,9 de `r(t) = 0,9 t`.
    let pente = ajuster_rayon(&rayons, 200, 400).pente;
    assert!(proche(pente, 0.9, 1e-9), "pente du rayon : {pente}");
    let ordonnee = ajuster_rayon(&rayons, 200, 400).ordonnee;
    assert!(proche(ordonnee, 0.0, 1e-8), "ordonnée du rayon : {ordonnee}");

    // `r(t) = t²` n'est pas une droite : la pente sur `]200, 400]` est 601 (deux fois la moyenne des temps) et celle de
    // la `k`-ième fenêtre de 50 pas est `451 + 20 k`. Une fenêtre décalée d'un pas ou un temps mal compté se voit.
    let mut carres = Vec::new();
    for t in 1..=400u32 {
        carres.push((t as f64) * (t as f64));
    }
    let pente_carres = ajuster_rayon(&carres, 200, 400).pente;
    assert!(proche(pente_carres, 601.0, 1e-8), "pente de t² sur ]200, 400] : {pente_carres}");
    for (k, pente) in vitesses_glissantes(&carres, 200, 400, 50, 10).iter().enumerate() {
        assert!(proche(*pente, 451.0 + 20.0 * k as f64, 1e-8), "fenêtre {k} de t² : pente {pente}");
    }
}
