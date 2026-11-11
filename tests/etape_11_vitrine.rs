//! Tests d'acceptation de l'étape 11 (a) : la carte de densité de la vitrine, côté natif.
//!
//! `mycelium_densite(x, y)` et `mycelium_densite_max()` de `vitrine` sont ce que la page lit, à travers le module
//! WebAssembly, pour dessiner la colonie comme la carte de densité de l'étape M0. Ici on les appelle comme des
//! fonctions Rust et on les compare, case par case et bit à bit, à `piece::carte_de_densite` (et à `piece::mesurer`)
//! appelées sur une `Simulation` construite **sans passer par `vitrine.rs`** : le texte du scénario est lu sur le
//! disque, jamais par `include_str!`, et la table des six scénarios ci-dessous est écrite ici, d'après la spécification.
//!
//! L'état de la vitrine est global au processus et les tests d'un fichier tournent en parallèle : chaque test prend
//! d'abord le verrou `VERROU`, propre à ce fichier, et le garde jusqu'à la fin. **Le verrou remet le choix à 0 en se
//! relâchant**, même quand le test a paniqué : un test n'hérite jamais du choix d'un autre.
//!
//! Ce que valent les deux fonctions avant tout `mycelium_demarrer` (`NaN`) n'est pas testé ici : l'état global est déjà
//! posé par les autres tests du processus. Le module WebAssembly fraîchement instancié, lui, n'a rien démarré :
//! `etape_11_artefact.rs` le vérifie sur lui.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `abs`, `max`, `to_bits`, `is_nan` et `is_finite`.

use std::sync::{Mutex, MutexGuard};

use mycelium_lab::piece::{carte_de_densite, mesurer};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use mycelium_lab::vitrine::{
    mycelium_avancer, mycelium_biomasse, mycelium_choisir_scenario, mycelium_demarrer, mycelium_densite,
    mycelium_densite_max, mycelium_hauteur, mycelium_largeur, mycelium_temps,
};

/// Un des six scénarios de la vitrine, tel que la spécification de l'étape 8 le fixe.
struct Scenario {
    /// Le numéro que prend `mycelium_choisir_scenario`.
    numero: u32,
    /// Le nom dans l'adresse de la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// Le côté de la boîte, en cases (elle est carrée).
    boite: u32,
    /// Le pas de référence (`duree` du fichier).
    duree: u32,
}

/// Les six scénarios, dans l'ordre de leurs numéros. Cette table est la spécification : elle ne vient pas de
/// `vitrine.rs`.
const SCENARIOS: [Scenario; 6] = [
    Scenario { numero: 0, nom: "libre", fichier: "couplage.txt", boite: 161, duree: 80 },
    Scenario { numero: 1, nom: "terrain", fichier: "terrain.txt", boite: 100, duree: 80 },
    Scenario { numero: 2, nom: "tache", fichier: "chimiotropisme.txt", boite: 100, duree: 100 },
    Scenario { numero: 3, nom: "tache-sans", fichier: "chimiotropisme-sans.txt", boite: 100, duree: 100 },
    Scenario { numero: 4, nom: "mur", fichier: "mur.txt", boite: 100, duree: 120 },
    Scenario { numero: 5, nom: "mur-sans", fichier: "mur-sans.txt", boite: 100, duree: 120 },
];

/// Les graines de la comparaison complète : la plus simple, une autre, celle des aperçus du mur, une qui ne tient ni sur
/// 16 bits ni sur 31, et la plus grande.
const GRAINES: [u32; 5] = [1, 7, 31, 0xDEAD_BEEF, u32::MAX];

/// Le pas moyen, entre le début et le pas de référence.
const PAS_MOYEN: u32 = 37;

/// Combien de pas on joue au-delà du pas de référence : la vitrine ne s'arrête pas à `duree`.
const PAS_AU_DELA: u32 = 40;

/// Sérialise les tests de ce fichier : l'état de la vitrine est unique.
static VERROU: Mutex<()> = Mutex::new(());

/// Le verrou, tenu pendant tout un test. En se relâchant, même si le test a paniqué, il remet le choix à 0.
struct Garde {
    /// Le verrou lui-même : tenu pour sérialiser les tests, jamais lu.
    _verrou: MutexGuard<'static, ()>,
}

impl Drop for Garde {
    fn drop(&mut self) {
        mycelium_choisir_scenario(0);
    }
}

/// Prend le verrou ; un test qui a paniqué en le tenant ne bloque pas les suivants.
fn verrou() -> Garde {
    Garde { _verrou: VERROU.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner()) }
}

/// Le texte d'un scénario, lu sur le disque.
fn texte(scenario: &Scenario) -> String {
    let chemin = format!("{}/scenarios/{}", env!("CARGO_MANIFEST_DIR"), scenario.fichier);
    std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{chemin} se lit : {erreur}"))
}

/// La simulation native en face : le scénario lu sur le disque, la graine, et `pas` pas de temps.
fn simulation_native(scenario: &Scenario, graine: u32, pas: u32) -> Simulation {
    let (reglages, milieu, tropismes, _duree) =
        lire_scenario_tropismes(&texte(scenario)).expect("le scénario du disque est valide");
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages sont valides");
    simulation.avancer_n(pas);
    simulation
}

/// Fixe le choix de la vitrine et vérifie qu'elle l'accepte.
fn choisir(scenario: &Scenario) {
    assert_eq!(mycelium_choisir_scenario(scenario.numero), 1, "le scénario {} est refusé", scenario.nom);
}

/// Toutes les cases de la carte de la vitrine, ligne par ligne, en bits : de quoi comparer deux cartes sans les confondre
/// avec `NaN` ni `-0.0`.
fn toute_la_carte() -> Vec<u64> {
    let (largeur, hauteur) = (mycelium_largeur(), mycelium_hauteur());
    let mut bits = Vec::new();
    for y in 0..hauteur {
        for x in 0..largeur {
            bits.push(mycelium_densite(x, y).to_bits());
        }
    }
    bits
}

/// Vérifie que la carte de la vitrine est, dans son état courant, celle de `piece` sur `native` : chaque case, bit à bit,
/// et le maximum (celui de `mesurer`). Avec `maximum_d_abord`, le maximum est lu avant toute case : la carte ne doit
/// pas dépendre de ce qu'on lit en premier.
fn verifier_la_carte(native: &Simulation, contexte: &str, maximum_d_abord: bool) {
    let reglages = native.reglages();
    let attendue = carte_de_densite(reglages.largeur, reglages.hauteur, native.segments());
    let maximum_attendu = mesurer(native).densite_maximale;
    assert_eq!(mycelium_largeur() as usize, reglages.largeur, "{contexte} : largeur");
    assert_eq!(mycelium_hauteur() as usize, reglages.hauteur, "{contexte} : hauteur");
    if maximum_d_abord {
        assert_eq!(mycelium_densite_max().to_bits(), maximum_attendu.to_bits(), "{contexte} : maximum, lu d'abord");
    }
    for y in 0..reglages.hauteur {
        for x in 0..reglages.largeur {
            let obtenu = mycelium_densite(x as u32, y as u32);
            let voulu = attendue.valeur(x, y);
            assert_eq!(obtenu.to_bits(), voulu.to_bits(), "{contexte} : case ({x}, {y}) : {obtenu} au lieu de {voulu}");
        }
    }
    assert_eq!(mycelium_densite_max().to_bits(), maximum_attendu.to_bits(), "{contexte} : maximum");
}

/// Choisit ce scénario, démarre la vitrine avec cette graine et compare sa carte à celle de `piece` à cinq moments : au
/// démarrage (0 pas), après 1 pas, à un pas moyen, au pas de référence et plus loin. À chaque moment la carte est lue
/// en entier : la lecture suivante, après un changement de la simulation, ne doit pas rendre l'ancienne carte.
fn verifier_scenario_et_graine(scenario: &Scenario, graine: u32) {
    choisir(scenario);
    mycelium_demarrer(graine);
    let mut native = simulation_native(scenario, graine, 0);
    let mut temps = 0;
    for (rang, cible) in [0, 1, PAS_MOYEN, scenario.duree, scenario.duree + PAS_AU_DELA].into_iter().enumerate() {
        mycelium_avancer(cible - temps);
        native.avancer_n(cible - temps);
        temps = cible;
        verifier_la_carte(&native, &format!("scénario {}, graine {graine}, {cible} pas", scenario.nom), rang % 2 == 1);
    }
    assert!(!native.segments().is_empty(), "la simulation native doit avoir laissé des segments");
}

// ---------------------------------------------------------------------------------------------------------------
// 1. La carte est celle de M0.
// ---------------------------------------------------------------------------------------------------------------

/// 1. Pour chacun des six scénarios et cinq graines, à cinq moments (0, 1, un pas moyen, le pas de référence, plus
///    loin), chaque case de la vitrine est, bit à bit, la case de `piece::carte_de_densite` sur les segments de la
///    même simulation, et son maximum est `densite_maximale` de `piece::mesurer`. La carte de la vitrine est lue en
///    entier à chaque moment : une carte gardée trop longtemps (après un pas de temps, un redémarrage, un changement de
///    scénario) serait vue ici.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_carte_de_la_vitrine_est_celle_de_piece_pour_les_six_scenarios() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        for graine in GRAINES {
            verifier_scenario_et_graine(scenario, graine);
        }
    }
}

/// 1 (suite). Une colonie suivie jusqu'à la fin de sa croissance (le mur sans tropisme, graine 1 : plus aucune pointe au
/// pas 700), la plus dense et la plus longue que la page montre, la carte lue en entier tous les 50 pas comme le ferait
/// la page : à chaque fois celle de `piece`, y compris aux pas où les dernières pointes meurent (le pas qui tue la
/// dernière pointe allonge encore la colonie : la carte doit changer avec lui).
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_carte_d_une_colonie_finie_est_celle_de_piece() {
    let _verrou = verrou();
    let scenario = &SCENARIOS[5];
    choisir(scenario);
    mycelium_demarrer(1);
    let mut native = simulation_native(scenario, 1, 0);
    for pas in (50..=700).step_by(50) {
        mycelium_avancer(50);
        native.avancer_n(50);
        verifier_la_carte(&native, &format!("mur-sans, graine 1, {pas} pas"), false);
    }
    assert_eq!(native.pointes(), 0, "la colonie devrait avoir fini de pousser au pas 700");
}

/// 1 (suite). Le bilan de M0 retrouvé par la vitrine, au pas de référence de chaque scénario : le nombre de cases où
/// passe de l'hyphe (`cases_occupees` de `mesurer`, au seuil nul) est le nombre de cases de densité strictement
/// positive, la plus grande case est `densite_maximale`, et la somme des cases est la biomasse, à l'arrondi près. La
/// carte n'est jamais négative, et la colonie occupe au moins une case.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_somme_des_cases_est_la_biomasse_et_les_cases_occupees_sont_celles_de_m0() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        mycelium_avancer(scenario.duree);
        let mesures = mesurer(&simulation_native(scenario, 1, scenario.duree));

        let (mut somme, mut maximum, mut occupees) = (0.0f64, 0.0f64, 0);
        for y in 0..scenario.boite {
            for x in 0..scenario.boite {
                let densite = mycelium_densite(x, y);
                assert!(densite >= 0.0, "{} : la case ({x}, {y}) est négative : {densite}", scenario.nom);
                somme += densite;
                maximum = maximum.max(densite);
                occupees += usize::from(densite > 0.0);
            }
        }
        let biomasse = mycelium_biomasse();
        assert!(biomasse > 0.0, "{} : la colonie n'a rien laissé", scenario.nom);
        assert!(
            (somme - biomasse).abs() <= 1e-9 * biomasse,
            "{} : la somme des cases ({somme}) n'est pas la biomasse ({biomasse})",
            scenario.nom
        );
        assert_eq!(occupees, mesures.cases_occupees, "{} : cases occupées", scenario.nom);
        assert!(occupees > 1, "{} : la colonie n'occupe qu'une case", scenario.nom);
        assert_eq!(maximum.to_bits(), mesures.densite_maximale.to_bits(), "{} : la plus grande case", scenario.nom);
        assert_eq!(mycelium_densite_max().to_bits(), maximum.to_bits(), "{} : le maximum de la vitrine", scenario.nom);
        assert!(maximum > 0.0, "{} : une colonie qui a poussé a une densité maximale positive", scenario.nom);
    }
}

/// 1 (suite). Les cases d'un obstacle n'ont aucune hyphe : leur densité est exactement 0.0 (c'est ce que la page
/// suppose quand elle peint ces cases comme le fond et dessine l'obstacle par-dessus), pour les trois scénarios qui
/// en ont, et la colonie a bien laissé de l'hyphe ailleurs.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn les_cases_d_un_obstacle_n_ont_aucune_hyphe() {
    let _verrou = verrou();
    let mut cases_vues = 0;
    for scenario in [&SCENARIOS[1], &SCENARIOS[4], &SCENARIOS[5]] {
        choisir(scenario);
        mycelium_demarrer(1);
        mycelium_avancer(scenario.duree + PAS_AU_DELA);
        let native = simulation_native(scenario, 1, scenario.duree + PAS_AU_DELA);
        assert!(!native.milieu().obstacles.is_empty(), "{} devrait avoir des obstacles", scenario.nom);
        for obstacle in &native.milieu().obstacles {
            for y in obstacle.y0..=obstacle.y1 {
                for x in obstacle.x0..=obstacle.x1 {
                    let densite = mycelium_densite(x as u32, y as u32);
                    assert_eq!(densite.to_bits(), 0.0f64.to_bits(), "{} : case bloquée ({x}, {y})", scenario.nom);
                    cases_vues += 1;
                }
            }
        }
        assert!(mycelium_densite_max() > 0.0, "{} : aucune hyphe", scenario.nom);
    }
    assert!(cases_vues > 400, "{cases_vues} cases bloquées seulement : les obstacles ont changé ?");
}

// ---------------------------------------------------------------------------------------------------------------
// 2. Hors de la boîte.
// ---------------------------------------------------------------------------------------------------------------

/// 2. Hors de la boîte, `mycelium_densite` rend `NaN` : la colonne `largeur` (qui, sur une ligne de `largeur` cases,
///    glisserait sur la ligne suivante si seul l'indice était contrôlé), la ligne `hauteur`, le coin hors boîte, un peu
///    au-delà, les très grands indices (`u32::MAX`) et la première ligne dont l'indice dépasse 2^32 (qui reviendrait dans
///    la carte en WebAssembly 32 bits si la ligne n'était pas contrôlée). Les quatre coins de la boîte, eux, sont des nombres finis et
///    non négatifs. Pour les six scénarios, dont deux tailles de boîte (161 et 100) : une carte gardée d'une autre boîte
///    ne passe pas.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn hors_de_la_boite_la_densite_est_nan() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        mycelium_avancer(PAS_MOYEN);
        let b = scenario.boite;
        assert_eq!(mycelium_largeur(), b);
        // La première ligne dont l'indice `y * largeur` dépasse 2^32 : en WebAssembly 32 bits il reviendrait dans la carte.
        let repli = u32::MAX / b + 1;
        for (x, y) in [
            (0, repli),
            (b - 1, repli),
            (b, 0),
            (b, b - 1),
            (0, b),
            (b - 1, b),
            (b, b),
            (b + 1, 0),
            (0, b + 1),
            (b + 1, b + 1),
            (1000, 3),
            (3, 1000),
            (u32::MAX, 0),
            (0, u32::MAX),
            (u32::MAX, u32::MAX),
            (u32::MAX, 1),
            (1, u32::MAX),
        ] {
            assert!(mycelium_densite(x, y).is_nan(), "{} : la case ({x}, {y}) devrait être NaN", scenario.nom);
        }
        for (x, y) in [(0, 0), (b - 1, 0), (0, b - 1), (b - 1, b - 1)] {
            let densite = mycelium_densite(x, y);
            assert!(densite.is_finite() && densite >= 0.0, "{} : la case ({x}, {y}) vaut {densite}", scenario.nom);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 3. Quand la carte change.
// ---------------------------------------------------------------------------------------------------------------

/// 3. La carte change après `mycelium_avancer(n)` pour `n >= 1` (ici 1, 2 puis 5 pas : chaque fois une carte
///    différente, qui est celle de `piece` au nouveau temps) et ne change pas après `mycelium_avancer(0)` (la même carte,
///    bit à bit, le même maximum et le même temps). La carte lue avant le pas ne reste pas.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_carte_change_apres_avancer_et_ne_change_pas_apres_avancer_0() {
    let _verrou = verrou();
    let scenario = &SCENARIOS[0];
    choisir(scenario);
    mycelium_demarrer(1);
    mycelium_avancer(30);
    let mut avant = toute_la_carte();
    let maximum = mycelium_densite_max();

    mycelium_avancer(0);
    assert_eq!(mycelium_temps(), 30, "avancer(0) a fait avancer le temps");
    assert_eq!(toute_la_carte(), avant, "avancer(0) a changé la carte");
    assert_eq!(mycelium_densite_max().to_bits(), maximum.to_bits(), "avancer(0) a changé le maximum");

    let mut temps = 30;
    for pas in [1, 2, 5] {
        mycelium_avancer(pas);
        temps += pas;
        assert_eq!(mycelium_temps(), temps);
        let apres = toute_la_carte();
        assert_ne!(apres, avant, "avancer({pas}) n'a pas changé la carte");
        verifier_la_carte(&simulation_native(scenario, 1, temps), &format!("après {temps} pas"), false);
        avant = apres;
        // Rien n'a bougé entre deux lectures : relire ne change rien non plus.
        assert_eq!(toute_la_carte(), avant, "relire la carte l'a changée");
    }
}

/// 3 (suite). `mycelium_demarrer` remet la carte à zéro : après des pas et une lecture de la carte, relancer (même
/// scénario, même graine) donne une carte vide, dont chaque case vaut exactement 0.0 et dont le maximum vaut
/// exactement 0.0 (la valeur d'une carte vide : ni `NaN`, ni un reste de l'ancienne carte). Rejouer les mêmes pas
/// redonne la même carte qu'avant, bit à bit. Pour les six scénarios.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn relancer_remet_la_carte_a_zero() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(2);
        mycelium_avancer(scenario.duree);
        let jouee = toute_la_carte();
        assert!(mycelium_densite_max() > 0.0, "{} : la carte jouée est vide", scenario.nom);

        mycelium_demarrer(2);
        assert_eq!(mycelium_densite_max().to_bits(), 0.0f64.to_bits(), "{} : maximum d'une carte vide", scenario.nom);
        let vide = toute_la_carte();
        assert_eq!(vide.len(), (scenario.boite * scenario.boite) as usize);
        assert!(vide.iter().all(|&bits| bits == 0.0f64.to_bits()), "{} : la carte n'est pas vide", scenario.nom);

        mycelium_avancer(scenario.duree);
        assert_eq!(toute_la_carte(), jouee, "{} : rejouer ne redonne pas la même carte", scenario.nom);
    }
}

/// 3 (suite). Changer de scénario puis redémarrer change aussi la boîte, donc la carte : de la colonie libre (161 × 161)
/// au mur (100 × 100) puis retour, la carte lue est celle du nouveau scénario (vide, de la bonne taille), la colonne 100
/// n'existe plus dans la petite boîte (`NaN`) et la case de la ligne 160 non plus ; et une graine différente donne une
/// carte différente de la même taille.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn changer_de_scenario_et_de_graine_change_la_carte() {
    let _verrou = verrou();
    choisir(&SCENARIOS[0]);
    mycelium_demarrer(1);
    mycelium_avancer(60);
    let grande = toute_la_carte();
    assert_eq!(grande.len(), 161 * 161);
    assert!(mycelium_densite(160, 160).is_finite());

    choisir(&SCENARIOS[4]);
    mycelium_demarrer(1);
    assert_eq!(toute_la_carte().len(), 100 * 100, "la carte n'est pas celle de la petite boîte");
    assert!(mycelium_densite(100, 0).is_nan(), "la colonne 100 existe encore");
    assert!(mycelium_densite(0, 160).is_nan(), "la ligne 160 existe encore");
    assert_eq!(mycelium_densite_max().to_bits(), 0.0f64.to_bits());

    choisir(&SCENARIOS[0]);
    mycelium_demarrer(2);
    mycelium_avancer(60);
    let autre = toute_la_carte();
    assert_eq!(autre.len(), grande.len());
    assert_ne!(autre, grande, "les graines 1 et 2 donnent la même carte");
    verifier_la_carte(&simulation_native(&SCENARIOS[0], 2, 60), "libre, graine 2, 60 pas", false);
}

/// 3 (suite). Choisir un scénario n'agit pas sur la simulation en cours, donc pas sur sa carte : après `choisir`, la
/// carte, son maximum et sa taille sont ceux d'avant ; seul le démarrage suivant prend le nouveau scénario.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn choisir_un_scenario_ne_change_pas_la_carte_en_cours() {
    let _verrou = verrou();
    choisir(&SCENARIOS[0]);
    mycelium_demarrer(7);
    mycelium_avancer(30);
    let carte = toute_la_carte();
    let maximum = mycelium_densite_max();

    choisir(&SCENARIOS[4]);
    assert_eq!(toute_la_carte(), carte, "choisir le mur a changé la carte de la colonie libre");
    assert_eq!(mycelium_densite_max().to_bits(), maximum.to_bits());
    mycelium_avancer(10);
    verifier_la_carte(&simulation_native(&SCENARIOS[0], 7, 40), "la colonie libre continue", false);

    mycelium_demarrer(7);
    verifier_la_carte(&simulation_native(&SCENARIOS[4], 7, 0), "le mur au démarrage suivant", false);
}

/// 3 (suite). La carte ne dépend ni de ce qui a été lu avant ni de ce que la page a fait entre deux lectures : avancer
/// par morceaux (1, 0, 6, 13, 0, 60 pas) en lisant tantôt le maximum, tantôt une seule case, tantôt rien, donne au bout
/// la carte de `piece` au pas 80, comme avancer d'un seul coup.
#[test]
#[cfg_attr(not(feature = "etape-11"), ignore)]
fn la_carte_ne_depend_pas_de_la_facon_d_avancer_ni_de_lire() {
    let _verrou = verrou();
    let scenario = &SCENARIOS[1];
    choisir(scenario);
    mycelium_demarrer(31);
    for (rang, morceau) in [1, 0, 6, 13, 0, 60].into_iter().enumerate() {
        mycelium_avancer(morceau);
        match rang % 3 {
            0 => {
                let _ = mycelium_densite_max();
            }
            1 => {
                let _ = mycelium_densite(50, 50);
            }
            _ => {}
        }
    }
    assert_eq!(mycelium_temps(), 80);
    verifier_la_carte(&simulation_native(scenario, 31, 80), "terrain, graine 31, par morceaux", true);
}
