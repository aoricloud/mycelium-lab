//! Tests d'acceptation de l'étape 7 (T2, les obstacles durs et le thigmotropisme), partie simulation : la construction avec
//! `Tropismes::thigmotropisme`, l'écriture canonique, la non-régression, le déterminisme.
//!
//! Ils fixent :
//!
//! - **le refus** : `nouvelle_avec_tropismes` refuse par `TropismeInvalide` un `thigmotropisme` négatif, supérieur à 90, `NaN`
//!   ou infini, et accepte `0.0`, `-0.0`, 35, 89,999 et 90 ; les contrôles des réglages et du milieu passent en premier
//!   (`ParametresInvalides`, `MilieuInvalide`) ; `tropismes()` rend l'angle donné ;
//! - **la non-régression** (T0, T1, V0 inchangés) : les empreintes relevées sur le binaire d'avant T2 (09aae5e) sont celles
//!   d'aujourd'hui, pour `couplage.txt`, `chimiotropisme.txt`, deux scénarios sans obstacle, un scénario où les obstacles
//!   ne sont jamais touchés et `terrain.txt` (graines 1 et 2, où la colonie n'en touche aucun), avec `thigmotropisme 0`,
//!   `-0` ou sans la ligne ; avec obstacles et angle nul, l'état change quand une pointe en touche un (`terrain.txt`,
//!   graine 3 : l'empreinte d'avant n'est plus l'état) ;
//! - **l'angle sans effet quand rien ne touche** : sans obstacle, `thigmotropisme 35` laisse segments, apex, compteurs,
//!   substrat et relevés exactement ceux de l'angle nul ; seule la fin de l'écriture canonique diffère ;
//! - **l'écriture canonique** : sans milieu ni tropisme, celle d'avant ; un angle non nul sur un milieu vide écrit deux `u64`
//!   nuls, puis l'angle (`to_bits`, petit-boutiste) puis 9 octets par apex (drapeau, `cap`, `g`) ; avec le
//!   chimiotropisme, ses 8 octets viennent avant ceux de l'angle ; `-0.0` n'écrit rien ; une pointe qui longe a le drapeau 1,
//!   `cap` égal à sa direction d'avant le contact et `g` à sa direction de glissement ; la pointe qui s'arrête et les deux
//!   pointes nées de son arrêt n'ont pas de mémoire ; une
//!   pointe sans mémoire a trois champs nuls ; `ecrire_canonique` et `empreinte` ne dépendent que de l'état ;
//! - **le déterminisme** : même scénario, même graine, même état à l'octet ; une autre graine, un autre état ;
//!   `avancer_n(n)` est `n` fois `avancer`, avec obstacles et angle ; les relevés suivent (un par pas, biomasse = somme
//!   des segments, dans l'ordre).
//!
//! Ils laissent ouverts : le texte des erreurs ; l'ordre des identifiants des deux pointes d'un arrêt.
//!
//! Ils n'utilisent aucune fonction mathématique de `f64`.

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::croissance::OCTETS_PAR_SEGMENT;
use mycelium_lab::simulation::{ErreurSimulation, Milieu, Obstacle, Simulation};
use mycelium_lab::tropisme::Tropismes;
use outils::{construire, jouer, lire, obstacle};

const BASE: &str =
    "boite 40 30\nspore 20 15\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nduree 30\n";

/// Les deux scénarios sans obstacle, relevés sur le binaire d'avant T2.
const SANS_OBSTACLE: &str = "boite 60 50\nspore 10 25\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 40\n";

/// Un scénario avec un obstacle que la colonie n'atteint jamais en 30 pas (pas 1, spore à 40 cases du mur).
const OBSTACLE_LOINTAIN: &str = "boite 60 60\nspore 10 30\nsubstrat 1.0 0.2\ncroissance 1.0 20.0 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\nduree 30\nobstacle 50 0 55 59\n";

fn texte_couplage() -> &'static str {
    include_str!("../scenarios/couplage.txt")
}

fn texte_chimiotropisme() -> &'static str {
    include_str!("../scenarios/chimiotropisme.txt")
}

fn texte_terrain() -> &'static str {
    include_str!("../scenarios/terrain.txt")
}

/// Les tropismes, construits sans littéral complet (une étape à venir pourra ajouter un champ).
#[allow(clippy::needless_update)]
fn tropismes(chimio: f64, thigmo: f64) -> Tropismes {
    Tropismes { chimiotropisme: chimio, thigmotropisme: thigmo, ..Tropismes::default() }
}

// ---------------------------------------------------------------------------------------------------------------
// Le refus
// ---------------------------------------------------------------------------------------------------------------

/// `thigmotropisme` négatif, supérieur à 90, `NaN` ou infini : `TropismeInvalide` ; `0.0`, `-0.0`, 35, 89,999, 90 et un
/// dénormalisé minuscule : acceptés, et `tropismes()` rend l'angle donné.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn un_angle_hors_de_zero_a_90_est_refuse() {
    outils::delai();
    let (reglages, milieu, _, _) = lire(BASE);
    for a in [
        -1.0,
        -0.000_001,
        -90.0,
        90.000_001,
        90.1,
        91.0,
        360.0,
        1e300,
        f64::MAX,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
    ] {
        let resultat = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), tropismes(0.0, a), 1);
        assert_eq!(resultat.err(), Some(ErreurSimulation::TropismeInvalide), "angle {a}");
    }
    for a in [0.0, -0.0, 35.0, 89.999, 90.0, 1e-300, 5e-324, 0.5, 45.0] {
        let simulation = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), tropismes(0.0, a), 1)
            .unwrap_or_else(|e| panic!("angle {a} : refusé à tort : {e:?}"));
        assert_eq!(simulation.tropismes().thigmotropisme.to_bits(), a.to_bits(), "angle {a} : rendu par tropismes()");
    }
    // Les deux tropismes se contrôlent chacun : un chimiotropisme invalide avec un angle valide est refusé, et inversement.
    let resultat = Simulation::nouvelle_avec_tropismes(reglages.clone(), milieu.clone(), tropismes(46.0, 35.0), 1);
    assert_eq!(resultat.err(), Some(ErreurSimulation::TropismeInvalide));
    let resultat = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes(45.0, 90.0), 1);
    assert!(resultat.is_ok(), "les deux bornes ensemble : {:?}", resultat.err());
}

/// Les contrôles des réglages et du milieu passent avant celui du tropisme : une spore hors de la boîte avec un angle
/// invalide donne `ParametresInvalides`, un obstacle à l'envers avec un angle invalide `MilieuInvalide`.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn les_controles_des_reglages_et_du_milieu_passent_en_premier() {
    outils::delai();
    let (reglages, milieu, _, _) = lire(BASE);
    let mut hors_boite = reglages.clone();
    hors_boite.spore = (40, 15);
    assert_eq!(
        Simulation::nouvelle_avec_tropismes(hors_boite, milieu.clone(), tropismes(0.0, -5.0), 1).err(),
        Some(ErreurSimulation::ParametresInvalides)
    );
    let a_l_envers = Milieu { sources: vec![], obstacles: vec![Obstacle { x0: 5, y0: 5, x1: 4, y1: 9 }] };
    assert_eq!(
        Simulation::nouvelle_avec_tropismes(reglages.clone(), a_l_envers, tropismes(0.0, 120.0), 1).err(),
        Some(ErreurSimulation::MilieuInvalide)
    );
    let contient_la_spore = Milieu { sources: vec![], obstacles: vec![obstacle(19, 14, 21, 16)] };
    assert_eq!(
        Simulation::nouvelle_avec_tropismes(reglages, contient_la_spore, tropismes(0.0, 35.0), 1).err(),
        Some(ErreurSimulation::MilieuInvalide),
        "un obstacle qui contient la spore reste refusé à tout angle"
    );
}

// ---------------------------------------------------------------------------------------------------------------
// La non-régression
// ---------------------------------------------------------------------------------------------------------------

/// Les empreintes relevées sur le binaire d'avant T2 (T1, 09aae5e) : `(nom, texte, graine, empreinte)`.
fn empreintes_d_avant() -> Vec<(&'static str, String, u64, &'static str)> {
    vec![
        (
            "couplage.txt, graine 1",
            texte_couplage().to_string(),
            1,
            "2223a1f3bcbc8627866aba084052cac06e93b05842bacc0ba94778bc23ed1083",
        ),
        (
            "chimiotropisme.txt, graine 3",
            texte_chimiotropisme().to_string(),
            3,
            "eba6a297755b36a7aa471bb09a2874e108061f337d280d1e3185863581679035",
        ),
        (
            "sans obstacle, graine 5",
            SANS_OBSTACLE.to_string(),
            5,
            "377caf4c4b866f3874377a97efcbd43e7f0176e5b5821d8b790ebaf774149d20",
        ),
        (
            "sans obstacle, graine 6",
            SANS_OBSTACLE.to_string(),
            6,
            "9da9a0a057623093dc449725a422944cf671f747b00ae00ebfcf70d11ddb15a1",
        ),
        (
            "obstacle lointain, graine 1",
            OBSTACLE_LOINTAIN.to_string(),
            1,
            "ea75426c890a97e695498b92720c46888f48404913cbceffcba43b2c48d356be",
        ),
        (
            "obstacle lointain, graine 2",
            OBSTACLE_LOINTAIN.to_string(),
            2,
            "98a8a4de031c23e8057851837300edead83d6b1d3ef29772bf1ca81d1446bfc9",
        ),
        (
            "terrain.txt, graine 1 (aucun contact)",
            texte_terrain().to_string(),
            1,
            "9f4f39a25d39be7edf523cb6b7646fb6fe87d4fbd00a2fe117fc61b23f49ddc0",
        ),
        (
            "terrain.txt, graine 2 (aucun contact)",
            texte_terrain().to_string(),
            2,
            "9b492b8a86e95e1c66ca6ada2f02e9fb4e79c596a0fe5a63ced1570cdb52767a",
        ),
    ]
}

/// Sans contact avec un obstacle et à angle nul, l'état est celui d'avant T2, octet pour octet : huit empreintes relevées sur
/// le binaire d'avant, avec la ligne `thigmotropisme 0`, `-0` ou sans elle (l'angle nul n'écrit rien de plus).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn a_angle_nul_et_sans_contact_les_empreintes_d_avant_ne_bougent_pas() {
    outils::delai();
    for (nom, texte, graine, empreinte) in empreintes_d_avant() {
        for (variante, texte) in [
            ("sans la ligne", texte.clone()),
            ("thigmotropisme 0", format!("{texte}thigmotropisme 0\n")),
            ("thigmotropisme -0", format!("{texte}thigmotropisme -0\n")),
        ] {
            let simulation = jouer(&texte, graine);
            assert_eq!(simulation.empreinte(), empreinte, "{nom}, {variante}");
        }
    }
}

/// Avec obstacles et angle nul, l'état change quand une pointe en touche un : `terrain.txt`, graine 3, valait
/// `b600c223…0253` avant T2 (une colonie qui traversait les murs) ; aujourd'hui ses segments n'entrent dans aucun obstacle
/// (recoupement exact, écrit dans `outils.rs`) et l'empreinte est autre.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn avec_obstacles_et_angle_nul_l_etat_change_quand_une_pointe_en_touche_un() {
    outils::delai();
    let simulation = jouer(texte_terrain(), 3);
    assert_ne!(
        simulation.empreinte(),
        "b600c223203a6b9051eb49aa958b840636510569c18ff080e4b7b34cc11a0253",
        "terrain.txt graine 3 : l'état d'avant T2 (des segments dans les murs) ne devrait plus être celui-ci"
    );
    assert_eq!(outils::premiere_faute(simulation.segments(), &simulation.milieu().obstacles), None);
}

/// Sans obstacle, `thigmotropisme 35` ne change rien à la colonie : segments, compteurs, relevés, substrat et pointes sont
/// exactement ceux de l'angle nul (aucun contact n'a lieu). Quatre scénarios, deux graines chacun, dont un avec le
/// chimiotropisme. L'écriture canonique, elle, commence par celle de l'angle nul et se prolonge de l'angle et des
/// drapeaux de mémoire (tous nuls, aucune pointe n'a de mémoire).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn sans_obstacle_l_angle_ne_change_pas_la_colonie() {
    outils::delai();
    for (nom, texte) in [
        ("couplage.txt", texte_couplage().to_string()),
        ("chimiotropisme.txt", texte_chimiotropisme().to_string()),
        ("sans obstacle", SANS_OBSTACLE.to_string()),
        ("avec une tache", format!("{BASE}source 25 15 6 4.0\nanastomose 0.5 10\n")),
    ] {
        for graine in [2, 7] {
            let zero = jouer(&texte, graine);
            let trente_cinq = jouer(&format!("{texte}thigmotropisme 35\n"), graine);
            let contexte = format!("{nom}, graine {graine}");
            assert_eq!(zero.segments(), trente_cinq.segments(), "{contexte} : segments");
            assert_eq!(zero.releves(), trente_cinq.releves(), "{contexte} : relevés");
            assert_eq!(zero.substrat(), trente_cinq.substrat(), "{contexte} : substrat");
            assert_eq!(
                (zero.pointes(), zero.ramifications(), zero.anastomoses()),
                (trente_cinq.pointes(), trente_cinq.ramifications(), trente_cinq.anastomoses()),
                "{contexte} : compteurs"
            );
            let (a, b) = (zero.ecrire_canonique(), trente_cinq.ecrire_canonique());
            let napex = 1 + zero.ramifications() as usize;
            // La partie commune : tout, jusqu'à la fin de la queue du milieu et du chimiotropisme.
            let commune = a.len();
            let milieu_vide = zero.milieu().sources.is_empty() && zero.milieu().obstacles.is_empty();
            let ajout_milieu_vide = if milieu_vide && zero.tropismes().chimiotropisme == 0.0 { 16 } else { 0 };
            assert_eq!(
                b.len(),
                commune + ajout_milieu_vide + 8 + 9 * napex,
                "{contexte} : longueur de l'écriture avec l'angle"
            );
            assert_eq!(&b[..commune], &a[..], "{contexte} : début de l'écriture");
            let fin = &b[commune + ajout_milieu_vide..];
            assert_eq!(&fin[..8], &35.0f64.to_bits().to_le_bytes(), "{contexte} : l'angle");
            assert!(fin[8..].iter().all(|&o| o == 0), "{contexte} : aucune mémoire quand rien ne touche");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// L'écriture canonique
// ---------------------------------------------------------------------------------------------------------------

/// Les morceaux de la fin d'une écriture canonique, lus à la main : l'octet où commence la queue du milieu (après le substrat),
/// calculé depuis le nombre de segments, d'apex (`1 + ramifications`) et la taille du substrat.
fn debut_de_la_queue(simulation: &Simulation) -> usize {
    let octets = simulation.ecrire_canonique();
    let segments = u64::from_le_bytes(octets[4..12].try_into().unwrap()) as usize;
    let napex = 1 + simulation.ramifications() as usize;
    let (l, h) = (simulation.substrat().largeur(), simulation.substrat().hauteur());
    4 + 8 + OCTETS_PAR_SEGMENT * segments + 25 * napex + 24 + 16 + 8 * l * h
}

/// Sans milieu ni tropisme, ou à angle nul (`0`, `-0`), l'écriture canonique est celle d'avant : rien après le substrat ;
/// avec un milieu non vide et un angle nul, la queue du milieu seule ; l'angle non nul sur un milieu vide écrit deux
/// `u64` nuls puis l'angle puis les drapeaux ; avec le chimiotropisme, ses 8 octets viennent avant l'angle.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_queue_de_l_ecriture_canonique() {
    outils::delai();
    let sans = jouer(BASE, 3);
    let debut = debut_de_la_queue(&sans);
    assert_eq!(sans.ecrire_canonique().len(), debut, "sans milieu ni tropisme : rien après le substrat");
    for ligne in ["thigmotropisme 0\n", "thigmotropisme -0\n", "chimiotropisme 0\nthigmotropisme -0.0\n"] {
        let s = jouer(&format!("{BASE}{ligne}"), 3);
        assert_eq!(s.ecrire_canonique(), sans.ecrire_canonique(), "{ligne:?} : l'angle nul n'écrit rien");
    }
    // L'angle seul, milieu vide : deux u64 nuls, l'angle, 9 octets par apex.
    let s = jouer(&format!("{BASE}thigmotropisme 35\n"), 3);
    let napex = 1 + s.ramifications() as usize;
    let octets = s.ecrire_canonique();
    let debut = debut_de_la_queue(&s);
    assert_eq!(octets.len(), debut + 16 + 8 + 9 * napex);
    assert!(octets[debut..debut + 16].iter().all(|&o| o == 0), "milieu vide : deux u64 nuls");
    assert_eq!(&octets[debut + 16..debut + 24], &35.0f64.to_bits().to_le_bytes());
    // Chimiotropisme et angle, avec un obstacle et une source : le milieu, puis la force, puis l'angle, puis les drapeaux.
    let texte = format!("{BASE}source 10 12 3 2.5\nobstacle 30 2 31 20\nchimiotropisme 2.5\nthigmotropisme 35\n");
    let s = jouer(&texte, 3);
    let napex = 1 + s.ramifications() as usize;
    let octets = s.ecrire_canonique();
    let debut = debut_de_la_queue(&s);
    let milieu = 8 + (4 + 4 + 4 + 8) + 8 + 16;
    assert_eq!(octets.len(), debut + milieu + 8 + 8 + 9 * napex, "longueur avec milieu, force et angle");
    assert_eq!(&octets[debut + milieu..debut + milieu + 8], &2.5f64.to_bits().to_le_bytes(), "la force vient d'abord");
    assert_eq!(&octets[debut + milieu + 8..debut + milieu + 16], &35.0f64.to_bits().to_le_bytes(), "puis l'angle");
    // Avec un angle nul, le milieu et la force seuls (écriture de T1).
    let sans_angle = jouer(&texte.replace("thigmotropisme 35\n", ""), 3);
    assert_eq!(sans_angle.ecrire_canonique().len(), debut_de_la_queue(&sans_angle) + milieu + 8);
}

/// Les 9 octets de mémoire de chaque apex : drapeau, `cap`, `g` (petit-boutiste), lus ici à la main dans l'écriture
/// canonique d'une pointe seule contre un mur vertical (colonnes 22 et 23, la pointe part de (20,5 ; 40,5)), deux pas après
/// le contact. Pour une pointe qui longe (angle de contact 20 < 35) : un apex, drapeau 1, `cap` = sa direction au contact,
/// `g` = la direction de glissement. Pour une pointe qui s'arrête (angle 70 ≥ 35) : trois apex, la pointe arrêtée et les
/// deux nées, tous sans mémoire (drapeau 0, champs nuls).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_memoire_s_ecrit_cap_et_g_pour_chaque_apex() {
    outils::delai();
    // (direction, longe ?, g) : 20° et 340° touchent la face verticale sous 70° ; 70° et 290° sous 20°.
    for (d, longe, g) in [(70usize, true, 90u32), (290, true, 270), (20, false, 0), (340, false, 0)] {
        let texte = "boite 30 80
spore 20 40
substrat 1.0 0.0
croissance 1.0 infini 30 0
couplage 0.0 0.0
                     duree 30
obstacle 22 0 23 79
thigmotropisme 35
";
        let graine = outils::graine_pour(d);
        let (mut s, _) = construire(texte, graine);
        // Jusqu'au contact (une mémoire ou une naissance), puis deux pas.
        for k in 0.. {
            assert!(k < 60, "direction {d} : pas de contact");
            s.avancer();
            let octets = s.ecrire_canonique();
            let memoire = &octets[octets.len() - 9 * (1 + s.ramifications() as usize)..];
            if memoire[0] == 1 || s.ramifications() > 0 {
                break;
            }
        }
        s.avancer_n(2);
        let octets = s.ecrire_canonique();
        let debut = debut_de_la_queue(&s);
        // La queue : milieu (0 source, 1 obstacle de 16 octets), angle, mémoire.
        let milieu = 8 + 8 + 16;
        assert_eq!(
            &octets[debut + milieu..debut + milieu + 8],
            &35.0f64.to_bits().to_le_bytes(),
            "direction {d} : l'angle"
        );
        let memoire = &octets[debut + milieu + 8..];
        let champs = |i: usize| -> (u8, u32, u32) {
            let m = &memoire[9 * i..9 * i + 9];
            (m[0], u32::from_le_bytes(m[1..5].try_into().unwrap()), u32::from_le_bytes(m[5..9].try_into().unwrap()))
        };
        if longe {
            assert_eq!(memoire.len(), 9, "direction {d} : un apex");
            assert_eq!(champs(0), (1, d as u32, g), "direction {d} : (drapeau, cap, g) de la pointe qui longe");
        } else {
            assert_eq!(memoire.len(), 27, "direction {d} : trois apex");
            for i in 0..3 {
                assert_eq!(champs(i), (0, 0, 0), "direction {d} : l'apex {i} n'a pas de mémoire");
            }
        }
    }
}

/// La mémoire s'efface au retour au cap : à la fin d'une longue simulation sur un mur fini, les pointes qui ont retrouvé leur
/// cap ont un drapeau 0 et des champs nuls ; une pointe inactive (arrêtée ou anastomosée) n'a jamais de mémoire ; les drapeaux ne valent jamais autre chose que 0 ou 1 ; `cap < 360` ; `g` est
/// l'une des quatre directions des axes ; deux lectures donnent les mêmes octets.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn la_memoire_ecrite_est_toujours_bien_formee() {
    outils::delai();
    let texte = include_str!("../scenarios/mur.txt");
    let mut vus_avec_memoire = 0usize;
    let mut vus_sans = 0usize;
    for graine in 1..=12 {
        let s = jouer(texte, graine);
        let octets = s.ecrire_canonique();
        let napex = 1 + s.ramifications() as usize;
        let memoire = &octets[octets.len() - 9 * napex..];
        let actifs = outils::apex_lus(&s);
        for (i, m) in memoire.chunks(9).enumerate() {
            if !actifs[i].actif {
                assert_eq!(m[0], 0, "graine {graine}, apex {i} : une pointe inactive n'a pas de mémoire");
            }
            let (cap, g) =
                (u32::from_le_bytes(m[1..5].try_into().unwrap()), u32::from_le_bytes(m[5..9].try_into().unwrap()));
            match m[0] {
                0 => {
                    assert_eq!((cap, g), (0, 0), "graine {graine}, apex {i} : sans mémoire, champs nuls");
                    vus_sans += 1;
                }
                1 => {
                    assert!(cap < 360, "graine {graine}, apex {i} : cap {cap}");
                    assert!([0, 90, 180, 270].contains(&g), "graine {graine}, apex {i} : g {g}");
                    vus_avec_memoire += 1;
                }
                autre => panic!("graine {graine}, apex {i} : drapeau {autre}"),
            }
        }
        assert_eq!(octets, s.ecrire_canonique());
    }
    assert!(vus_avec_memoire > 0 && vus_sans > 0, "garde : {vus_avec_memoire} apex avec mémoire, {vus_sans} sans");
}

// ---------------------------------------------------------------------------------------------------------------
// Le déterminisme
// ---------------------------------------------------------------------------------------------------------------

/// Même scénario, même graine : même état à l'octet (écriture canonique, empreinte, segments) ; une autre graine : un autre état.
/// Sur `scenarios/mur.txt`, qui touche son mur.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn meme_graine_meme_etat_autre_graine_autre_etat() {
    outils::delai();
    let texte = include_str!("../scenarios/mur.txt");
    let a = jouer(texte, 2);
    let b = jouer(texte, 2);
    assert_eq!(a.ecrire_canonique(), b.ecrire_canonique());
    assert_eq!(a.empreinte(), b.empreinte());
    assert_eq!(a.empreinte().len(), 64);
    let c = jouer(texte, 3);
    assert_ne!(a.empreinte(), c.empreinte());
}

/// `avancer_n(n)` est strictement `n` fois `avancer`, avec obstacles et thigmotropisme : même état à l'octet à chaque pas
/// intermédiaire de `0` à `duree`, et les relevés suivent : un par pas, `temps` croissant de 1, biomasse égale à la somme des
/// longueurs des segments (dans l'ordre où ils sont déposés), compteurs et pointes de `Simulation`.
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn avancer_n_est_n_fois_avancer_et_les_releves_suivent() {
    outils::delai();
    let texte = include_str!("../scenarios/mur.txt");
    let (mut un_par_un, duree) = construire(texte, 2);
    let (mut d_un_coup, _) = construire(texte, 2);
    for t in 0..duree {
        assert_eq!(un_par_un.temps(), t);
        un_par_un.avancer();
    }
    d_un_coup.avancer_n(duree);
    assert_eq!(un_par_un.ecrire_canonique(), d_un_coup.ecrire_canonique());
    // Des étapes intermédiaires : 17 puis 40 puis le reste.
    let (mut par_etapes, _) = construire(texte, 2);
    par_etapes.avancer_n(17);
    par_etapes.avancer_n(40);
    par_etapes.avancer_n(duree - 57);
    assert_eq!(par_etapes.ecrire_canonique(), d_un_coup.ecrire_canonique());
    // Les relevés.
    let releves = d_un_coup.releves();
    assert_eq!(releves.len(), duree as usize + 1);
    for (t, r) in releves.iter().enumerate() {
        assert_eq!(r.temps as usize, t);
    }
    let dernier = d_un_coup.releve();
    assert_eq!(dernier, *releves.last().unwrap());
    let mut somme = 0.0;
    for s in d_un_coup.segments() {
        somme += s.longueur();
    }
    assert_eq!(dernier.biomasse, somme, "biomasse = somme des longueurs des segments, dans l'ordre");
    assert_eq!(dernier.biomasse, d_un_coup.biomasse());
    assert_eq!(dernier.pointes, d_un_coup.pointes());
    assert_eq!(dernier.ramifications, d_un_coup.ramifications());
    assert_eq!(dernier.anastomoses, d_un_coup.anastomoses());
    // La biomasse ne diminue jamais, les ramifications non plus.
    for paire in releves.windows(2) {
        assert!(paire[1].biomasse >= paire[0].biomasse && paire[1].ramifications >= paire[0].ramifications);
    }
}
