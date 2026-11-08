//! Tests d'acceptation de l'étape 8 (a) : la vitrine choisit un scénario, côté natif.
//!
//! Les fonctions `mycelium_*` de `vitrine` sont celles que la page appelle à travers le module WebAssembly ; ici on
//! les appelle comme des fonctions Rust et on les compare à une `Simulation` construite **sans passer par
//! `vitrine.rs`** : le texte du scénario est lu sur le disque (`scenarios/<fichier>.txt`), jamais par `include_str!`,
//! et lu par `lire_scenario_tropismes` puis construit par `Simulation::nouvelle_avec_tropismes`. Ainsi, un numéro
//! rangé en face d'un mauvais fichier dans `vitrine.rs` fait échouer les tests : la table des six scénarios
//! ci-dessous (numéro, nom, fichier, boîte, durée) est écrite ici, d'après la spécification, et non reprise du code.
//!
//! L'état de la vitrine est global au processus et les tests d'un fichier tournent en parallèle : chaque test prend
//! d'abord le verrou `VERROU`, propre à ce fichier, et le garde jusqu'à la fin. **Le verrou remet le choix à 0 en
//! se relâchant**, même quand le test a paniqué : un test n'hérite jamais du choix d'un autre (chacun choisit ce
//! dont il a besoin) et les tests de l'étape 4 restent vrais.
//!
//! Ce que valent les fonctions avant tout `mycelium_demarrer` (0 ou `NaN`, pour `mycelium_duree`,
//! `mycelium_nb_obstacles` et `mycelium_obstacle` comme pour les autres) n'est pas testé ici : comme à l'étape 4,
//! l'état global est déjà posé par les autres tests du processus. Le module WebAssembly fraîchement instancié, lui,
//! n'a rien démarré : `etape_8_artefact.rs` le vérifie sur lui.
//!
//! Le choix initial (0, le couplage) n'est visible qu'avant le premier `mycelium_choisir_scenario` du processus :
//! le premier test qui prend le verrou le regarde (voir `verrou`), et `le_choix_initial_est_le_couplage` rapporte
//! ce qu'il a vu. Les tests de l'étape 4, qui ne choisissent jamais, le vérifient aussi.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `to_bits`, `is_nan` et `is_finite`.

use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard, OnceLock};

use mycelium_lab::simulation::{Simulation, lire_scenario, lire_scenario_tropismes};
use mycelium_lab::vitrine::{
    mycelium_anastomoses, mycelium_avancer, mycelium_biomasse, mycelium_choisir_scenario, mycelium_demarrer,
    mycelium_duree, mycelium_empreinte_mot, mycelium_hauteur, mycelium_largeur, mycelium_nb_obstacles,
    mycelium_nb_segments, mycelium_obstacle, mycelium_pointes, mycelium_segment, mycelium_substrat, mycelium_temps,
};

/// Un des six scénarios de la vitrine, tel que la spécification de l'étape 8 le fixe.
struct Scenario {
    /// Le numéro que prend `mycelium_choisir_scenario`.
    numero: u32,
    /// Le nom dans l'adresse de la page.
    nom: &'static str,
    /// Le fichier sous `scenarios/`.
    fichier: &'static str,
    /// La largeur et la hauteur de la boîte, en cases.
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

/// Les graines de la vérification complète : la plus simple, une autre, la graine des aperçus du mur, et une qui ne
/// tient ni sur 16 bits ni sur 31 (une graine tronquée ou lue comme un entier signé ne passe pas inaperçue).
const GRAINES: [u32; 4] = [1, 7, 31, 0xDEAD_BEEF];

/// Les graines extrêmes, vérifiées au pas de référence de chaque scénario : le bit de poids fort levé, et le maximum.
const GRAINES_EXTREMES: [u32; 3] = [0x8000_0000, 0xFFFF_FFFE, u32::MAX];

/// Le pas moyen, entre le début et le pas de référence.
const PAS_MOYEN: u32 = 37;

/// Combien de pas on joue au-delà du pas de référence : la vitrine ne s'arrête pas à `duree`.
const PAS_AU_DELA: u32 = 40;

/// Ce que le premier test à avoir pris le verrou a vu du choix initial : vrai si la vitrine, démarrée sans rien
/// choisir, a donné l'empreinte de référence du couplage.
static DEFAUT_EST_LE_COUPLAGE: OnceLock<bool> = OnceLock::new();

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
///
/// Le premier test du processus qui passe ici regarde, avant que quiconque ait choisi quoi que ce soit, ce que
/// donne la vitrine démarrée sans choix : c'est le seul moment où le choix initial se voit.
fn verrou() -> Garde {
    let garde = VERROU.lock().unwrap_or_else(|empoisonne| empoisonne.into_inner());
    DEFAUT_EST_LE_COUPLAGE.get_or_init(|| {
        mycelium_demarrer(1);
        mycelium_avancer(80);
        empreinte_de_la_vitrine() == reference_couplage_graine_1()
    });
    Garde { _verrou: garde }
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

/// La `duree` écrite dans le fichier du scénario.
fn duree_du_fichier(scenario: &Scenario) -> u32 {
    lire_scenario_tropismes(&texte(scenario)).expect("le scénario du disque est valide").3
}

/// Fixe le choix de la vitrine et vérifie qu'elle l'accepte.
fn choisir(scenario: &Scenario) {
    assert_eq!(
        mycelium_choisir_scenario(scenario.numero),
        1,
        "le scénario {} ({}) est refusé",
        scenario.numero,
        scenario.nom
    );
}

/// Les huit mots de l'empreinte de la vitrine, chacun sur 8 chiffres hexadécimaux, dans l'ordre.
fn empreinte_de_la_vitrine() -> String {
    (0..8).map(|i| format!("{:08x}", mycelium_empreinte_mot(i))).collect()
}

/// L'empreinte de référence du dépôt : le couplage, graine 1, 80 pas.
fn reference_couplage_graine_1() -> String {
    let chemin = concat!(env!("CARGO_MANIFEST_DIR"), "/references/couplage-graine-1.empreinte");
    std::fs::read_to_string(chemin).expect("la référence se lit").trim().to_string()
}

/// Vérifie que la vitrine, dans son état courant, est identique à `native` sur tout ce qu'elle expose : le temps, la
/// boîte, la durée du scénario, les compteurs, chaque coordonnée de chaque segment, chaque case du substrat (bit à
/// bit), les obstacles, la biomasse et l'empreinte.
///
/// L'empreinte est comparée par son premier mot seulement (`mycelium_empreinte_mot(0)`, 32 bits du SHA-256 de tout
/// l'état, apex compris) : chaque appel à `mycelium_empreinte_mot` hache l'état entier, et lire les huit mots à chaque
/// point de contrôle coûtait la moitié du temps de ce fichier en profil dev. Deux états différents qui auraient le même
/// premier mot, c'est une chance sur quatre milliards. Les huit mots, eux, sont comparés par `verifier_empreinte` au
/// pas de référence de chaque scénario et dans les tests qui comparent des empreintes.
fn verifier_identique(native: &Simulation, scenario: &Scenario, contexte: &str) {
    let contexte = format!("scénario {} ({}), {contexte}", scenario.numero, scenario.nom);
    assert_eq!(mycelium_temps(), native.temps(), "{contexte} : temps");
    let substrat = native.substrat();
    assert_eq!(mycelium_largeur() as usize, substrat.largeur(), "{contexte} : largeur");
    assert_eq!(mycelium_hauteur() as usize, substrat.hauteur(), "{contexte} : hauteur");
    assert_eq!(mycelium_largeur(), scenario.boite, "{contexte} : largeur de la boîte de la spécification");
    assert_eq!(mycelium_hauteur(), scenario.boite, "{contexte} : hauteur de la boîte de la spécification");
    assert_eq!(mycelium_duree(), duree_du_fichier(scenario), "{contexte} : durée du fichier");
    assert_eq!(mycelium_duree(), scenario.duree, "{contexte} : durée de la spécification");
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

    let obstacles = &native.milieu().obstacles;
    assert_eq!(mycelium_nb_obstacles() as usize, obstacles.len(), "{contexte} : nombre d'obstacles");
    for (i, obstacle) in obstacles.iter().enumerate() {
        for (k, valeur) in [obstacle.x0, obstacle.y0, obstacle.x1, obstacle.y1].into_iter().enumerate() {
            let obtenu = mycelium_obstacle(i as u32, k as u32);
            assert_eq!(
                obtenu.to_bits(),
                (valeur as f64).to_bits(),
                "{contexte} : obstacle {i}, coordonnée {k} : {obtenu}"
            );
        }
    }

    assert_eq!(mycelium_biomasse().to_bits(), native.biomasse().to_bits(), "{contexte} : biomasse");
    let attendue = native.empreinte();
    assert_eq!(format!("{:08x}", mycelium_empreinte_mot(0)), attendue[..8], "{contexte} : empreinte (premier mot)");
}

/// Vérifie les huit mots de l'empreinte de la vitrine contre l'empreinte de `native` (64 caractères).
fn verifier_empreinte(native: &Simulation, contexte: &str) {
    assert_eq!(empreinte_de_la_vitrine(), native.empreinte(), "{contexte} : empreinte (huit mots)");
}

/// Choisit ce scénario, démarre la vitrine avec cette graine et la compare à la simulation native à cinq moments :
/// au démarrage (0 pas), après 1 pas, à un pas moyen, au pas de référence et plus loin. La vitrine avance par morceaux,
/// la simulation native de la même façon.
fn verifier_scenario_et_graine(scenario: &Scenario, graine: u32) {
    choisir(scenario);
    mycelium_demarrer(graine);
    let mut native = simulation_native(scenario, graine, 0);
    let mut temps = 0;
    for cible in [0, 1, PAS_MOYEN, scenario.duree, scenario.duree + PAS_AU_DELA] {
        mycelium_avancer(cible - temps);
        native.avancer_n(cible - temps);
        temps = cible;
        let contexte = format!("scénario {}, graine {graine}, {cible} pas", scenario.nom);
        verifier_identique(&native, scenario, &contexte);
        if cible == scenario.duree {
            verifier_empreinte(&native, &contexte);
        }
    }
    assert!(!native.segments().is_empty(), "la simulation native doit avoir laissé des segments");
}

/// Vérifie un scénario sur les quatre graines de `GRAINES`.
fn verifier_scenario(scenario: &Scenario) {
    let _verrou = verrou();
    for graine in GRAINES {
        verifier_scenario_et_graine(scenario, graine);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 1. Le choix.
// ---------------------------------------------------------------------------------------------------------------

/// 1. Avant tout choix, le scénario est le 0 : la vitrine démarrée sans rien choisir donne l'empreinte de référence du
///    couplage. Le test lit ce que le premier test du processus à prendre le verrou a observé (voir `verrou`) ; les
///    tests de l'étape 4 le vérifient eux aussi, dans leur propre processus. Puis le choix explicite du 0 est accepté
///    et redonne la même empreinte : le défaut est bien le numéro 0, et non un scénario qui lui ressemblerait.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn le_choix_initial_est_le_couplage() {
    let _verrou = verrou();
    assert_eq!(DEFAUT_EST_LE_COUPLAGE.get(), Some(&true), "la vitrine démarrée sans choix n'est pas le couplage");
    assert_eq!(mycelium_choisir_scenario(0), 1, "le numéro 0 est refusé");
    mycelium_demarrer(1);
    mycelium_avancer(80);
    assert_eq!(empreinte_de_la_vitrine(), reference_couplage_graine_1(), "le numéro 0 n'est pas le couplage");
}

/// 1 (suite). Les numéros 0 à 5 sont acceptés (la fonction rend 1) et chacun fixe bien le scénario du prochain
/// démarrage : la boîte et la durée de ce qu'on démarre sont celles de la table.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn les_numeros_0_a_5_sont_acceptes() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        assert_eq!(mycelium_choisir_scenario(scenario.numero), 1, "numéro {}", scenario.numero);
        mycelium_demarrer(1);
        assert_eq!(mycelium_largeur(), scenario.boite, "numéro {} : boîte", scenario.numero);
        assert_eq!(mycelium_duree(), scenario.duree, "numéro {} : durée", scenario.numero);
        // Choisir deux fois de suite le même numéro est permis et ne change rien.
        assert_eq!(mycelium_choisir_scenario(scenario.numero), 1, "numéro {}, une seconde fois", scenario.numero);
    }
}

/// 1 (suite). Un numéro qui n'est pas de 0 à 5 rend 0 et **ne change rien** : le scénario choisi avant reste. Les
/// valeurs voisines (6, 7), des puissances de deux, des numéros qui reviendraient dans 0 à 5 si on les tronquait à 8,
/// 16 ou 31 bits (256, 260, 65 540, 2^31 + 4) et `u32::MAX` : toutes refusées. Le test les essaie après un choix de 0
/// puis après un choix de 4, pour qu'une valeur refusée qui ferait revenir au 0 ne passe pas pour un « rien changé ».
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn un_numero_hors_de_0_a_5_est_refuse_sans_rien_changer() {
    let _verrou = verrou();
    let refuses = [6, 7, 8, 16, 255, 256, 260, 65_536 + 4, 0x8000_0004, u32::MAX - 1, u32::MAX];
    for precedent in [&SCENARIOS[0], &SCENARIOS[4], &SCENARIOS[5]] {
        choisir(precedent);
        for numero in refuses {
            assert_eq!(mycelium_choisir_scenario(numero), 0, "le numéro {numero} doit être refusé");
        }
        mycelium_demarrer(1);
        assert_eq!(mycelium_duree(), precedent.duree, "après les refus, le scénario n'est plus {}", precedent.nom);
        assert_eq!(
            mycelium_largeur(),
            precedent.boite,
            "après les refus, la boîte n'est plus celle de {}",
            precedent.nom
        );
        mycelium_avancer(precedent.duree);
        assert_eq!(
            empreinte_de_la_vitrine(),
            simulation_native(precedent, 1, precedent.duree).empreinte(),
            "après les refus, la vitrine n'est plus le scénario {}",
            precedent.nom
        );
    }
}

/// 1 (suite). Choisir n'agit pas sur la simulation en cours : ni redémarrage, ni changement de scénario. Après un
/// choix, le temps, la boîte, la durée, les compteurs, les obstacles et l'empreinte sont ceux d'avant ; et si l'on
/// avance encore, c'est l'ancien scénario qui continue. Seul le démarrage suivant prend le nouveau.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn choisir_ne_redemarre_pas_la_simulation_en_cours() {
    let _verrou = verrou();
    // Des paires qui diffèrent par la boîte (0 et 4), par la durée (0 et 4) et par les obstacles (4 et 0).
    for (en_cours, choisi) in
        [(&SCENARIOS[0], &SCENARIOS[4]), (&SCENARIOS[4], &SCENARIOS[0]), (&SCENARIOS[2], &SCENARIOS[1])]
    {
        choisir(en_cours);
        mycelium_demarrer(7);
        mycelium_avancer(30);
        let empreinte = empreinte_de_la_vitrine();
        let segments = mycelium_nb_segments();
        let pointes = mycelium_pointes();
        let obstacles = mycelium_nb_obstacles();

        choisir(choisi);
        assert_eq!(mycelium_temps(), 30, "choisir {} a redémarré {}", choisi.nom, en_cours.nom);
        assert_eq!(mycelium_nb_segments(), segments);
        assert_eq!(mycelium_pointes(), pointes);
        assert_eq!(
            mycelium_nb_obstacles(),
            obstacles,
            "choisir {} a changé les obstacles de {}",
            choisi.nom,
            en_cours.nom
        );
        assert_eq!(mycelium_largeur(), en_cours.boite, "choisir {} a changé la boîte de {}", choisi.nom, en_cours.nom);
        assert_eq!(mycelium_duree(), en_cours.duree, "choisir {} a changé la durée de {}", choisi.nom, en_cours.nom);
        assert_eq!(empreinte_de_la_vitrine(), empreinte, "choisir {} a changé l'état de {}", choisi.nom, en_cours.nom);

        // La simulation en cours continue : c'est toujours l'ancien scénario.
        mycelium_avancer(10);
        verifier_identique(
            &simulation_native(en_cours, 7, 40),
            en_cours,
            &format!("après avoir choisi {}", choisi.nom),
        );

        // Le démarrage suivant prend le scénario choisi.
        mycelium_demarrer(7);
        verifier_identique(&simulation_native(choisi, 7, 0), choisi, "au démarrage suivant");
    }
}

/// 1 (suite). Le choix tient pour plusieurs `mycelium_demarrer` de suite : il n'est pas consommé par un démarrage.
/// Pour chaque scénario, trois démarrages avec trois graines donnent trois fois ce scénario.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn le_choix_tient_pour_plusieurs_demarrer_de_suite() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        for graine in [1, 2, 3] {
            mycelium_demarrer(graine);
            mycelium_avancer(PAS_MOYEN);
            verifier_identique(
                &simulation_native(scenario, graine, PAS_MOYEN),
                scenario,
                &format!("démarrage de graine {graine}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 2. Chaque scénario est celui de la bibliothèque.
// ---------------------------------------------------------------------------------------------------------------

/// 2. Le scénario 0 (`couplage.txt`, « libre ») : pour chacune des quatre graines et à cinq moments (0, 1, un pas
///    moyen, le pas de référence, plus loin), tout ce que la vitrine expose est ce que donne la bibliothèque. La
///    colonie a, au pas de référence, des anastomoses : le compteur n'est pas éprouvé à vide.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn scenario_0_libre_est_la_bibliotheque() {
    verifier_scenario(&SCENARIOS[0]);
    let _verrou = verrou();
    choisir(&SCENARIOS[0]);
    mycelium_demarrer(1);
    mycelium_avancer(80);
    assert!(mycelium_anastomoses() > 0, "il faut au moins une anastomose pour que le compteur soit éprouvé");
}

/// 2 (suite). Le scénario 1 (`terrain.txt`) : une tache de nutriments et deux obstacles.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn scenario_1_terrain_est_la_bibliotheque() {
    verifier_scenario(&SCENARIOS[1]);
}

/// 2 (suite). Le scénario 2 (`chimiotropisme.txt`, « tache ») : chimiotropisme 10.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn scenario_2_tache_est_la_bibliotheque() {
    verifier_scenario(&SCENARIOS[2]);
}

/// 2 (suite). Le scénario 3 (`chimiotropisme-sans.txt`, « tache-sans ») : le même sans chimiotropisme.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn scenario_3_tache_sans_est_la_bibliotheque() {
    verifier_scenario(&SCENARIOS[3]);
}

/// 2 (suite). Le scénario 4 (`mur.txt`) : un mur, thigmotropisme 35.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn scenario_4_mur_est_la_bibliotheque() {
    verifier_scenario(&SCENARIOS[4]);
}

/// 2 (suite). Le scénario 5 (`mur-sans.txt`) : le même mur sans thigmotropisme.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn scenario_5_mur_sans_est_la_bibliotheque() {
    verifier_scenario(&SCENARIOS[5]);
}

/// 2 (suite). Les graines extrêmes (le bit de poids fort levé, `0xFFFF_FFFE`, `u32::MAX`), sur chacun des six
/// scénarios, au pas de référence : la graine que passe la page est un entier de 32 bits non signé, ni tronqué ni lu
/// comme un entier signé.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn les_graines_extremes_sur_les_six_scenarios() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        for graine in GRAINES_EXTREMES {
            mycelium_demarrer(graine);
            mycelium_avancer(scenario.duree);
            let native = simulation_native(scenario, graine, scenario.duree);
            let contexte = format!("graine {graine}, pas de référence");
            verifier_identique(&native, scenario, &contexte);
            verifier_empreinte(&native, &contexte);
        }
    }
}

/// 2 (suite). Les six scénarios donnent six états différents pour la même graine au pas de référence, et deux
/// scénarios qui ne diffèrent que par un tropisme (2 et 3 pour le chimiotropisme, 4 et 5 pour le thigmotropisme)
/// donnent des états différents pour au moins une des quatre graines : le tropisme du fichier est bien appliqué, il
/// n'est pas remplacé par la valeur par défaut. Une comparaison à la bibliothèque seule ne le verrait pas si les deux
/// côtés ignoraient le tropisme.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn les_scenarios_se_distinguent() {
    let _verrou = verrou();
    let mut empreintes = BTreeSet::new();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        mycelium_avancer(scenario.duree);
        empreintes.insert(empreinte_de_la_vitrine());
    }
    assert_eq!(empreintes.len(), 6, "deux scénarios donnent le même état pour la graine 1");

    for (avec, sans) in [(&SCENARIOS[2], &SCENARIOS[3]), (&SCENARIOS[4], &SCENARIOS[5])] {
        let un_etat_differe = GRAINES.iter().any(|graine| {
            let mut empreintes = Vec::new();
            for scenario in [avec, sans] {
                choisir(scenario);
                mycelium_demarrer(*graine);
                mycelium_avancer(scenario.duree);
                empreintes.push(empreinte_de_la_vitrine());
            }
            empreintes[0] != empreintes[1]
        });
        assert!(
            un_etat_differe,
            "{} et {} donnent le même état pour toutes les graines : tropisme ignoré ?",
            avec.nom, sans.nom
        );
    }
}

/// 2 (suite). Le scénario 0 est, octet pour octet, ce que construisait la vitrine avant les six scénarios : la
/// lecture `lire_scenario` suivie de `Simulation::nouvelle` (T0 le garantit à réglages par défaut). On le prouve en
/// natif, par l'écriture canonique de l'état au temps 0 et à 80 pas, pour plusieurs graines ; puis on prouve que la
/// vitrine, sur le scénario 0, donne cette empreinte. Et le couplage, graine 1, 80 pas, égale toujours la référence du
/// dépôt, y compris quand d'autres scénarios ont été choisis et joués entre-temps.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn le_scenario_0_est_l_ancienne_construction() {
    let _verrou = verrou();
    let couplage = &SCENARIOS[0];
    for graine in [1, 2, 7, 0xDEAD_BEEF, u32::MAX] {
        let (reglages, _duree) = lire_scenario(&texte(couplage)).expect("le scénario est valide");
        let mut ancienne = Simulation::nouvelle(reglages, u64::from(graine)).expect("les réglages sont valides");
        let mut nouvelle = simulation_native(couplage, graine, 0);
        assert_eq!(ancienne.ecrire_canonique(), nouvelle.ecrire_canonique(), "graine {graine}, temps 0");
        ancienne.avancer_n(80);
        nouvelle.avancer_n(80);
        assert_eq!(ancienne.ecrire_canonique(), nouvelle.ecrire_canonique(), "graine {graine}, 80 pas");

        choisir(couplage);
        mycelium_demarrer(graine);
        mycelium_avancer(80);
        assert_eq!(
            empreinte_de_la_vitrine(),
            ancienne.empreinte(),
            "graine {graine} : la vitrine n'est pas l'ancienne"
        );
    }

    // La référence du dépôt, après avoir joué d'autres scénarios.
    for autre in [&SCENARIOS[4], &SCENARIOS[1], &SCENARIOS[5]] {
        choisir(autre);
        mycelium_demarrer(1);
        mycelium_avancer(autre.duree);
    }
    choisir(couplage);
    mycelium_demarrer(1);
    mycelium_avancer(80);
    assert_eq!(empreinte_de_la_vitrine(), reference_couplage_graine_1());
}

// ---------------------------------------------------------------------------------------------------------------
// 3. La durée et les obstacles.
// ---------------------------------------------------------------------------------------------------------------

/// 3. `mycelium_duree` rend la `duree` du fichier, pour la simulation **en cours** : quand on a choisi un autre
///    scénario depuis, elle rend encore celle de la simulation en cours (et non celle du scénario choisi) ; elle ne
///    change qu'au démarrage suivant. Les six durées sont lues sur le disque, et deux scénarios ont la même durée
///    (80 pour 0 et 1, 100 pour 2 et 3, 120 pour 4 et 5) : le test passe aussi par des paires de durées différentes.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn la_duree_est_celle_de_la_simulation_en_cours() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        assert_eq!(mycelium_duree(), duree_du_fichier(scenario), "{} : la durée du fichier", scenario.nom);
        assert_eq!(mycelium_duree(), scenario.duree, "{} : la durée de la spécification", scenario.nom);
    }

    for (en_cours, choisi) in
        [(&SCENARIOS[4], &SCENARIOS[2]), (&SCENARIOS[2], &SCENARIOS[0]), (&SCENARIOS[0], &SCENARIOS[5])]
    {
        assert_ne!(en_cours.duree, choisi.duree, "la paire doit avoir deux durées différentes");
        choisir(en_cours);
        mycelium_demarrer(3);
        mycelium_avancer(5);
        choisir(choisi);
        assert_eq!(
            mycelium_duree(),
            en_cours.duree,
            "{} en cours, {} choisi : la durée est celle du choisi",
            en_cours.nom,
            choisi.nom
        );
        mycelium_avancer(PAS_MOYEN);
        assert_eq!(mycelium_duree(), en_cours.duree, "{} : la durée change quand la simulation avance", en_cours.nom);
        mycelium_demarrer(3);
        assert_eq!(mycelium_duree(), choisi.duree, "{} : la durée n'est pas passée au scénario démarré", choisi.nom);
    }
}

/// Les rectangles `(x0, y0, x1, y1)` du scénario, écrits à la main d'après les fichiers : ce que la page dessine. Les
/// quatre nombres de chaque rectangle sont deux à deux différents, pour que l'échange de deux coordonnées ne passe pas
/// inaperçu. Si un fichier de scénario change, ces nombres changent avec lui.
fn obstacles_ecrits_a_la_main(scenario: &Scenario) -> Vec<[u32; 4]> {
    match scenario.numero {
        1 => vec![[48, 0, 51, 39], [48, 60, 51, 99]],
        4 | 5 => vec![[50, 20, 53, 80]],
        _ => Vec::new(),
    }
}

/// 3 (suite). `mycelium_nb_obstacles` et `mycelium_obstacle` rendent exactement les rectangles du scénario, dans
/// l'ordre du fichier, bords inclus : `terrain` en a deux, `mur` et `mur-sans` un, les trois autres aucun. Chaque
/// coordonnée, une fois l'obstacle lu, vaut le nombre entier du fichier : ni le bord exclu (`x1 + 1`), ni un autre rang.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn les_obstacles_sont_les_rectangles_du_scenario_dans_l_ordre() {
    let _verrou = verrou();
    let mut total = 0;
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        let attendus = obstacles_ecrits_a_la_main(scenario);
        total += attendus.len();
        // Ce que lit la bibliothèque dans le fichier du disque est ce qu'on a écrit à la main.
        let (_, milieu, _, _) = lire_scenario_tropismes(&texte(scenario)).expect("le scénario est valide");
        let du_fichier: Vec<[u32; 4]> =
            milieu.obstacles.iter().map(|o| [o.x0 as u32, o.y0 as u32, o.x1 as u32, o.y1 as u32]).collect();
        assert_eq!(
            du_fichier, attendus,
            "{} : les obstacles écrits à la main ne sont pas ceux du fichier",
            scenario.nom
        );

        assert_eq!(mycelium_nb_obstacles() as usize, attendus.len(), "{} : nombre d'obstacles", scenario.nom);
        for (i, rectangle) in attendus.iter().enumerate() {
            let distincts: BTreeSet<u32> = rectangle.iter().copied().collect();
            assert_eq!(
                distincts.len(),
                4,
                "{} : le rectangle {i} a deux coordonnées égales, le test ne verrait pas un échange",
                scenario.nom
            );
            for (k, valeur) in rectangle.iter().enumerate() {
                let obtenu = mycelium_obstacle(i as u32, k as u32);
                assert_eq!(
                    obtenu.to_bits(),
                    f64::from(*valeur).to_bits(),
                    "{} : obstacle {i}, coordonnée {k} : {obtenu}",
                    scenario.nom
                );
            }
        }
    }
    assert_eq!(
        total, 4,
        "trois scénarios ont des obstacles : un pour le mur, un pour le mur sans tropisme, deux pour le terrain"
    );
}

/// 3 (suite). Les obstacles sont ceux de la simulation **en cours** : quand on a choisi un autre scénario depuis, le
/// nombre et les coordonnées restent ceux de la simulation en cours.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn les_obstacles_sont_ceux_de_la_simulation_en_cours() {
    let _verrou = verrou();
    choisir(&SCENARIOS[1]);
    mycelium_demarrer(1);
    choisir(&SCENARIOS[4]);
    assert_eq!(mycelium_nb_obstacles(), 2, "le terrain a deux obstacles, même quand le mur est choisi");
    assert_eq!(mycelium_obstacle(1, 1).to_bits(), 60.0f64.to_bits());
    choisir(&SCENARIOS[0]);
    assert_eq!(mycelium_nb_obstacles(), 2, "même quand le couplage est choisi");
    mycelium_demarrer(1);
    assert_eq!(mycelium_nb_obstacles(), 0, "le couplage n'a pas d'obstacle");
}

/// 3 (suite). Hors de portée : `mycelium_obstacle` rend `NaN` pour un obstacle qui n'existe pas (`i` égal au nombre
/// d'obstacles, ou très au-delà) et pour une coordonnée qui n'existe pas (`k > 3`), y compris pour un obstacle qui
/// existe ; le dernier obstacle, lui, existe. Sur un scénario sans obstacle, tout est hors de portée. Quant à ce que
/// valent ces fonctions avant tout démarrage, ce test ne le vérifie pas (l'état global est partagé) : le module
/// WebAssembly neuf le vérifie dans `etape_8_artefact.rs`.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn obstacle_hors_de_portee_rend_nan() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        mycelium_avancer(5);
        let n = mycelium_nb_obstacles();
        assert_eq!(n as usize, obstacles_ecrits_a_la_main(scenario).len());

        for k in 0..4 {
            if n > 0 {
                assert!(mycelium_obstacle(0, k).is_finite(), "{} : obstacle 0, coordonnée {k}", scenario.nom);
                assert!(mycelium_obstacle(n - 1, k).is_finite(), "{} : dernier obstacle, coordonnée {k}", scenario.nom);
            }
            for i in [n, n + 1, 7, 100, u32::MAX] {
                assert!(mycelium_obstacle(i, k).is_nan(), "{} : obstacle {i}, coordonnée {k}", scenario.nom);
            }
        }
        for k in [4, 5, 7, 255, 256, 260, u32::MAX] {
            for i in [0, 1, n.saturating_sub(1), n, u32::MAX] {
                assert!(mycelium_obstacle(i, k).is_nan(), "{} : obstacle {i}, coordonnée {k}", scenario.nom);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 4. Démarrer.
// ---------------------------------------------------------------------------------------------------------------

/// 4. `mycelium_demarrer` repart de zéro et dans le bon scénario après un autre : quel que soit le scénario qu'on
///    jouait, le démarrage suivant d'un autre scénario est au temps 0, sans segment, avec la boîte, la durée, les
///    obstacles et l'empreinte de ce scénario, et après sa durée de pas, l'empreinte de la bibliothèque. Les couples
///    changent de boîte (161 contre 100) et d'obstacles (aucun contre un mur).
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn demarrer_repart_de_zero_dans_le_bon_scenario_apres_un_autre() {
    let _verrou = verrou();
    for (avant, apres) in [
        (&SCENARIOS[4], &SCENARIOS[0]),
        (&SCENARIOS[0], &SCENARIOS[4]),
        (&SCENARIOS[1], &SCENARIOS[5]),
        (&SCENARIOS[3], &SCENARIOS[2]),
        (&SCENARIOS[5], &SCENARIOS[1]),
    ] {
        choisir(avant);
        mycelium_demarrer(2);
        mycelium_avancer(60);
        assert_eq!(mycelium_temps(), 60);
        assert!(mycelium_nb_segments() > 0);

        choisir(apres);
        mycelium_demarrer(2);
        assert_eq!(mycelium_temps(), 0, "{} puis {} : le temps ne repart pas de zéro", avant.nom, apres.nom);
        assert_eq!(mycelium_nb_segments(), 0, "{} puis {} : des segments restent", avant.nom, apres.nom);
        assert_eq!(mycelium_anastomoses(), 0);
        assert_eq!(mycelium_pointes(), 1);
        assert_eq!(mycelium_biomasse().to_bits(), 0.0f64.to_bits());
        verifier_identique(
            &simulation_native(apres, 2, 0),
            apres,
            &format!("{} puis {}, temps 0", avant.nom, apres.nom),
        );

        mycelium_avancer(apres.duree);
        let native = simulation_native(apres, 2, apres.duree);
        let contexte = format!("{} puis {}, pas de référence", avant.nom, apres.nom);
        verifier_identique(&native, apres, &contexte);
        verifier_empreinte(&native, &contexte);
    }
}

/// 4 (suite). Redémarrer le même scénario avec la même graine redonne exactement le même état (celui d'une première
/// fois), et avec une autre graine un état différent : la graine est lue et le scénario ne garde rien d'avant. Pour
/// les six scénarios.
#[test]
#[cfg_attr(not(feature = "etape-8"), ignore)]
fn redemarrer_le_meme_scenario_redonne_le_meme_etat() {
    let _verrou = verrou();
    for scenario in &SCENARIOS {
        choisir(scenario);
        mycelium_demarrer(1);
        mycelium_avancer(scenario.duree);
        let premiere = empreinte_de_la_vitrine();
        assert_eq!(premiere, simulation_native(scenario, 1, scenario.duree).empreinte(), "{}", scenario.nom);

        mycelium_demarrer(1);
        mycelium_avancer(scenario.duree);
        assert_eq!(
            empreinte_de_la_vitrine(),
            premiere,
            "{} : un second démarrage ne redonne pas le même état",
            scenario.nom
        );

        mycelium_demarrer(2);
        mycelium_avancer(scenario.duree);
        assert_ne!(empreinte_de_la_vitrine(), premiere, "{} : les graines 1 et 2 donnent le même état", scenario.nom);
    }
}
