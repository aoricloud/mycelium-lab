//! Tests d'acceptation de l'étape 4 (a) : la vitrine est la simulation, côté natif.
//!
//! Les fonctions `mycelium_*` de `vitrine` sont celles que la page appelle à travers le module WebAssembly ; ici on
//! les appelle comme des fonctions Rust et on les compare, à chaque fois, à une `Simulation` construite en face avec
//! le même scénario (`scenarios/couplage.txt`) et la même graine. Le module WebAssembly lui-même est éprouvé dans
//! `etape_4_artefact.rs`.
//!
//! L'état de la vitrine est global au processus et les tests d'un fichier tournent en parallèle : chaque test prend
//! d'abord le verrou `VERROU`, propre à ce fichier, et le garde jusqu'à la fin.
//!
//! Ce que valent les fonctions avant tout `mycelium_demarrer` n'est pas testé : l'état global est déjà posé par
//! les autres tests du processus. Ce contrat est écrit dans la doc de `src/vitrine.rs`.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_bits`, `is_nan` et `is_finite`.

use std::sync::{Mutex, MutexGuard};

use mycelium_lab::simulation::{Simulation, lire_scenario};
use mycelium_lab::vitrine::{
    mycelium_anastomoses, mycelium_avancer, mycelium_biomasse, mycelium_demarrer, mycelium_empreinte_mot,
    mycelium_hauteur, mycelium_largeur, mycelium_nb_segments, mycelium_pointes, mycelium_segment, mycelium_substrat,
    mycelium_temps,
};

/// Le scénario que fait tourner la vitrine.
const SCENARIO: &str = include_str!("../scenarios/couplage.txt");

/// La durée du scénario, et le nombre de pas de l'empreinte de référence.
const PAS: u32 = 80;

/// Une graine qui ne tient pas sur 16 bits, ni sur 31 : pour qu'une graine tronquée ou lue comme un entier signé
/// ne passe pas inaperçue.
const GRAINE_GRANDE: u32 = 0xDEAD_BEEF;

/// Sérialise les tests de ce fichier : l'état de la vitrine est unique.
static VERROU: Mutex<()> = Mutex::new(());

/// Prend le verrou ; un test qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner())
}

/// La simulation native en face : le scénario embarqué, la graine, et `pas` pas de temps.
fn simulation_native(graine: u32, pas: u32) -> Simulation {
    let (reglages, _duree) = lire_scenario(SCENARIO).expect("le scénario est valide");
    let mut simulation = Simulation::nouvelle(reglages, u64::from(graine)).expect("les réglages sont valides");
    simulation.avancer_n(pas);
    simulation
}

/// Les huit mots de l'empreinte de la vitrine, chacun sur 8 chiffres hexadécimaux, dans l'ordre.
fn empreinte_de_la_vitrine() -> String {
    (0..8).map(|i| format!("{:08x}", mycelium_empreinte_mot(i))).collect()
}

/// L'empreinte de la graine 1 à 80 pas, telle que la référence du dépôt la fixe.
fn reference_graine_1() -> String {
    let chemin = concat!(env!("CARGO_MANIFEST_DIR"), "/references/couplage-graine-1.empreinte");
    std::fs::read_to_string(chemin).expect("la référence se lit").trim().to_string()
}

/// Vérifie que la vitrine, dans son état courant, est identique à `native` sur tout ce qu'elle expose : le temps,
/// la boîte, les compteurs, chaque coordonnée de chaque segment et chaque case du substrat (bit à bit), la biomasse
/// et l'empreinte.
fn verifier_identique(native: &Simulation, contexte: &str) {
    assert_eq!(mycelium_temps(), native.temps(), "{contexte} : temps");
    let substrat = native.substrat();
    assert_eq!(mycelium_largeur() as usize, substrat.largeur(), "{contexte} : largeur");
    assert_eq!(mycelium_hauteur() as usize, substrat.hauteur(), "{contexte} : hauteur");
    assert_eq!(mycelium_nb_segments() as usize, native.segments().len(), "{contexte} : nombre de segments");
    assert_eq!(mycelium_pointes() as usize, native.pointes(), "{contexte} : pointes");
    assert_eq!(u64::from(mycelium_anastomoses()), native.anastomoses(), "{contexte} : anastomoses");

    for (i, segment) in native.segments().iter().enumerate() {
        let attendu = [segment.x0, segment.y0, segment.x1, segment.y1];
        for (k, valeur) in attendu.iter().enumerate() {
            let obtenu = mycelium_segment(i as u32, k as u32);
            assert_eq!(obtenu.to_bits(), valeur.to_bits(), "{contexte} : segment {i}, coordonnée {k} : {obtenu}");
        }
    }

    for y in 0..substrat.hauteur() {
        for x in 0..substrat.largeur() {
            let obtenu = mycelium_substrat(x as u32, y as u32);
            let attendu = substrat.valeur(x, y);
            assert_eq!(
                obtenu.to_bits(),
                attendu.to_bits(),
                "{contexte} : case ({x}, {y}) : {obtenu} au lieu de {attendu}"
            );
        }
    }

    assert_eq!(mycelium_biomasse().to_bits(), native.biomasse().to_bits(), "{contexte} : biomasse");
    assert_eq!(empreinte_de_la_vitrine(), native.empreinte(), "{contexte} : empreinte");
}

/// Démarre la vitrine avec `graine`, vérifie qu'elle est au temps 0 comme la simulation native, l'avance de `pas`
/// pas d'un seul appel et vérifie qu'elle redonne la simulation native.
fn verifier_graine(graine: u32, pas: u32) {
    mycelium_demarrer(graine);
    verifier_identique(&simulation_native(graine, 0), &format!("graine {graine}, temps 0"));
    mycelium_avancer(pas);
    let native = simulation_native(graine, pas);
    assert!(!native.segments().is_empty(), "la simulation native doit avoir laissé des segments");
    verifier_identique(&native, &format!("graine {graine}, {pas} pas"));
}

/// 1. Pour la graine 1, 80 pas : tout ce que la vitrine expose égale la simulation native, et l'empreinte est celle
///    du fichier de référence du dépôt. À 80 pas, la colonie a déjà des centaines de segments, des pointes et des
///    anastomoses : aucun compteur n'est nul, donc aucun n'est éprouvé à vide.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_vitrine_est_la_simulation_graine_1() {
    let _verrou = verrou();
    verifier_graine(1, PAS);
    assert!(mycelium_anastomoses() > 0, "il faut au moins une anastomose pour que le compteur soit éprouvé");
    assert_eq!(empreinte_de_la_vitrine(), reference_graine_1());
}

/// 1 (suite). Même chose pour la graine 2.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_vitrine_est_la_simulation_graine_2() {
    let _verrou = verrou();
    verifier_graine(2, PAS);
    assert!(mycelium_anastomoses() > 0, "il faut au moins une anastomose pour que le compteur soit éprouvé");
}

/// 1 (suite). Une graine de 32 bits qui ne tient ni sur 16 bits ni sur 31, et la plus grande graine : la page
/// passe un entier de 32 bits non signé, il ne doit être ni tronqué ni lu comme un entier signé.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_vitrine_est_la_simulation_pour_les_grandes_graines() {
    let _verrou = verrou();
    verifier_graine(GRAINE_GRANDE, PAS);
    verifier_graine(u32::MAX, PAS);
}

/// 1 (suite). Sur 200 pas d'un seul appel, quand la colonie compte des milliers de segments : la vitrine reste la
/// simulation, sans plafond sur le nombre de pas ni sur le nombre de segments.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn la_vitrine_est_la_simulation_sur_une_longue_duree() {
    let _verrou = verrou();
    verifier_graine(3, 200);
    assert!(mycelium_nb_segments() > 5000, "{} segments", mycelium_nb_segments());
}

/// 2. `mycelium_demarrer` remplace l'état : rappelé après des pas, il repart du temps 0 et, après 80 pas, redonne
///    l'empreinte de la graine 1 ; avec une autre graine, il redonne celle de cette graine et non l'ancienne.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn demarrer_repart_de_zero() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(30);
    assert_eq!(mycelium_temps(), 30);
    assert!(mycelium_nb_segments() > 0);

    mycelium_demarrer(1);
    assert_eq!(mycelium_temps(), 0);
    assert_eq!(mycelium_nb_segments(), 0);
    assert_eq!(mycelium_pointes(), 1);
    assert_eq!(mycelium_anastomoses(), 0);
    assert_eq!(mycelium_biomasse().to_bits(), 0.0f64.to_bits());
    mycelium_avancer(PAS);
    assert_eq!(mycelium_temps(), PAS);
    assert_eq!(empreinte_de_la_vitrine(), reference_graine_1());

    mycelium_demarrer(2);
    assert_eq!(mycelium_temps(), 0);
    mycelium_avancer(PAS);
    assert_eq!(empreinte_de_la_vitrine(), simulation_native(2, PAS).empreinte());
}

/// 2 (suite). `mycelium_avancer(80)` en un appel donne la même empreinte que 80 appels de `mycelium_avancer(1)` ;
/// et, appelée par morceaux de tailles quelconques (dont 0), la vitrine suit pas à pas la simulation native : le
/// temps et l'empreinte sont les siens après chaque morceau, jamais une valeur gardée d'avant.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn avancer_en_un_appel_ou_par_morceaux() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(PAS);
    let en_un_appel = empreinte_de_la_vitrine();

    mycelium_demarrer(1);
    for pas in 1..=PAS {
        mycelium_avancer(1);
        assert_eq!(mycelium_temps(), pas);
    }
    assert_eq!(empreinte_de_la_vitrine(), en_un_appel);
    assert_eq!(en_un_appel, reference_graine_1());

    mycelium_demarrer(1);
    let mut native = simulation_native(1, 0);
    for morceau in [1, 0, 6, 13, 0, 60] {
        mycelium_avancer(morceau);
        native.avancer_n(morceau);
        assert_eq!(mycelium_temps(), native.temps());
        assert_eq!(empreinte_de_la_vitrine(), native.empreinte(), "après {} pas", native.temps());
    }
    assert_eq!(native.temps(), PAS);
}

/// 2 (suite). `mycelium_avancer(0)` ne change rien : ni le temps, ni les compteurs, ni l'empreinte.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn avancer_zero_ne_change_rien() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(25);
    let empreinte = empreinte_de_la_vitrine();
    let segments = mycelium_nb_segments();
    let pointes = mycelium_pointes();

    mycelium_avancer(0);
    assert_eq!(mycelium_temps(), 25);
    assert_eq!(mycelium_nb_segments(), segments);
    assert_eq!(mycelium_pointes(), pointes);
    assert_eq!(empreinte_de_la_vitrine(), empreinte);
}

/// 2 (suite). La graine 2 donne, à 80 pas, une empreinte différente de celle de la graine 1 (la graine est donc bien
/// lue), et chacune est celle de sa simulation native.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn deux_graines_deux_empreintes() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(PAS);
    let graine_1 = empreinte_de_la_vitrine();
    mycelium_demarrer(2);
    mycelium_avancer(PAS);
    let graine_2 = empreinte_de_la_vitrine();

    assert_ne!(graine_1, graine_2);
    assert_eq!(graine_1.len(), 64);
    assert_eq!(graine_2.len(), 64);
    assert_eq!(graine_1, simulation_native(1, PAS).empreinte());
    assert_eq!(graine_2, simulation_native(2, PAS).empreinte());
}

/// 3. Hors de portée : `mycelium_segment` rend `NaN` pour un segment qui n'existe pas (`i` égal au nombre de
///    segments, ou très au-delà) et pour une coordonnée qui n'existe pas (`k > 3`) ; le dernier segment, lui, existe.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn segment_hors_de_portee_rend_nan() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(PAS);
    let n = mycelium_nb_segments();
    assert!(n > 0);

    for k in 0..4 {
        assert!(mycelium_segment(0, k).is_finite(), "segment 0, coordonnée {k}");
        assert!(mycelium_segment(n - 1, k).is_finite(), "dernier segment, coordonnée {k}");
        assert!(mycelium_segment(n, k).is_nan(), "segment {n}, coordonnée {k}");
        assert!(mycelium_segment(n + 1, k).is_nan(), "segment {}, coordonnée {k}", n + 1);
        assert!(mycelium_segment(u32::MAX, k).is_nan(), "segment u32::MAX, coordonnée {k}");
    }
    for k in [4, 5, 7, u32::MAX] {
        assert!(mycelium_segment(0, k).is_nan(), "segment 0, coordonnée {k}");
        assert!(mycelium_segment(n - 1, k).is_nan(), "dernier segment, coordonnée {k}");
    }
}

/// 3 (suite). `mycelium_substrat` rend `NaN` hors de la boîte, sur chaque bord (`x == largeur`, `y == hauteur`) et
/// très au-delà ; les quatre coins de la boîte, eux, sont dans la boîte.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn substrat_hors_de_la_boite_rend_nan() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(PAS);
    let (largeur, hauteur) = (mycelium_largeur(), mycelium_hauteur());
    assert!(largeur > 0 && hauteur > 0);

    for (x, y) in [(0, 0), (largeur - 1, 0), (0, hauteur - 1), (largeur - 1, hauteur - 1)] {
        let valeur = mycelium_substrat(x, y);
        assert!(valeur.is_finite() && valeur >= 0.0, "case ({x}, {y}) : {valeur}");
    }
    for (x, y) in [
        (largeur, 0),
        (0, hauteur),
        (largeur, hauteur),
        (largeur, hauteur - 1),
        (largeur - 1, hauteur),
        (u32::MAX, 0),
        (0, u32::MAX),
        (u32::MAX, u32::MAX),
    ] {
        assert!(mycelium_substrat(x, y).is_nan(), "case ({x}, {y}) hors de la boîte");
    }
}

/// 3 (suite). `mycelium_empreinte_mot(i)` rend 0 pour `i >= 8` ; les huit premiers mots ne sont pas tous nuls.
#[test]
#[cfg_attr(not(feature = "etape-4"), ignore)]
fn empreinte_mot_hors_de_portee_rend_zero() {
    let _verrou = verrou();
    mycelium_demarrer(1);
    mycelium_avancer(PAS);

    assert!((0..8).any(|i| mycelium_empreinte_mot(i) != 0));
    for i in [8, 9, 15, 64, u32::MAX] {
        assert_eq!(mycelium_empreinte_mot(i), 0, "mot {i}");
    }
}
