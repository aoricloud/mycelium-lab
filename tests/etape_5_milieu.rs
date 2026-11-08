//! Tests d'acceptation de l'étape 5 (T0, le terrain), partie milieu : `Source`, `Obstacle`, `Milieu` et
//! `Simulation::nouvelle_avec_milieu`.
//!
//! Ils fixent : sans milieu, l'état est celui d'avant, octet pour octet (comparé à `nouvelle`, aux empreintes que
//! le code d'avant T0 donnait et à `references/couplage-graine-1.empreinte`) ; une source est le disque de cases
//! `(i - x)² + (j - y)² ≤ rayon²`, compté en entiers ; ses cases remplacent le substrat initial, la dernière source
//! l'emporte ; un obstacle reste à `0.0`, même sous une source ; la masse se conserve, compartiment par
//! compartiment quand un mur sépare la boîte ; les milieux hors contrat sont refusés ; l'écriture canonique garde
//! son format sans milieu et ajoute sa queue avec un milieu ; T0 ne change ni la marche ni l'extension.
//!
//! Une source dont le centre est dans la boîte mais dont le disque en déborde est acceptée et tronquée à la boîte
//! (choix du chef de projet : seul son centre est vérifié). Ce que la spécification laisse ouvert n'est pas exigé :
//! la variante d'erreur d'un milieu refusé (tout ce qu'on demande est une erreur qui n'est pas `Substrat`), et ce qu'une
//! pointe fait en arrivant dans un obstacle (T2 le changera : aucun test ne le fixe).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs` et `sqrt`.

use std::path::Path;

use mycelium_lab::anastomose;
use mycelium_lab::croissance;
use mycelium_lab::empreinte::sha256_hex;
use mycelium_lab::simulation::{
    ErreurSimulation, Milieu, Obstacle, Reglages, Simulation, Source, lire_scenario, lire_scenario_complet,
};

/// Tolérance (relative) sur « substrat + coût de la biomasse = constante » avec obstacles et sources, pas après
/// pas, sur six graines. Mesurée sur le prototype : l'écart reste sous 6e-14 ; une fuite à travers un obstacle donne
/// plus de 1e-3. Celle des tests d'avant T0 est 1e-9.
const TOLERANCE_MASSE: f64 = 1e-10;

fn source(x: usize, y: usize, rayon: usize, quantite: f64) -> Source {
    Source { x, y, rayon, quantite }
}

fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

fn milieu(sources: Vec<Source>, obstacles: Vec<Obstacle>) -> Milieu {
    Milieu { sources, obstacles }
}

/// Les réglages de base : substrat initial 1, diffusion 0,2, une ramification tous les 20 pas, sans anastomose.
fn reglages(largeur: usize, hauteur: usize, spore: (usize, usize)) -> Reglages {
    Reglages {
        largeur,
        hauteur,
        spore,
        substrat_initial: 1.0,
        diffusion: 0.2,
        croissance: croissance::Parametres { pas: 1.0, intervalle: 20.0, angle_ramification: 30, amplitude_marche: 20 },
        affinite: 0.1,
        cout: 0.002,
        anastomose: None,
    }
}

/// Une colonie qui ne se ramifie pas : une seule pointe, qui marche (consomme, mais ne multiplie pas les pointes).
fn reglages_sans_ramification(largeur: usize, hauteur: usize, spore: (usize, usize)) -> Reglages {
    let mut r = reglages(largeur, hauteur, spore);
    r.croissance.intervalle = f64::INFINITY;
    r
}

fn construire(r: Reglages, m: Milieu, graine: u64) -> Simulation {
    Simulation::nouvelle_avec_milieu(r, m, graine).expect("réglages et milieu valides")
}

/// Vrai si la case `(i, j)` est dans le disque de centre `(x, y)` et de rayon `rayon`, compté en entiers signés,
/// sans aucun flottant : réécrit ici, indépendamment du code testé.
fn dans_le_disque(i: usize, j: usize, x: usize, y: usize, rayon: usize) -> bool {
    let (di, dj, r) = (i as i64 - x as i64, j as i64 - y as i64, rayon as i64);
    di * di + dj * dj <= r * r
}

fn dans_le_rectangle(i: usize, j: usize, o: &Obstacle) -> bool {
    o.x0 <= i && i <= o.x1 && o.y0 <= j && j <= o.y1
}

/// La quantité attendue au temps 0 dans la case `(i, j)` : la dernière source qui la contient l'emporte, sinon le
/// substrat initial ; `0.0` sous un obstacle.
fn attendue(r: &Reglages, m: &Milieu, i: usize, j: usize) -> f64 {
    if m.obstacles.iter().any(|o| dans_le_rectangle(i, j, o)) {
        return 0.0;
    }
    let mut quantite = r.substrat_initial;
    for s in &m.sources {
        if dans_le_disque(i, j, s.x, s.y, s.rayon) {
            quantite = s.quantite;
        }
    }
    quantite
}

/// Vérifie, case par case, que le substrat au temps 0 est celui que le milieu décrit, et que `est_bloquee` vaut
/// exactement « sous un obstacle ». Rend le nombre de cases bloquées.
fn verifier_etat_initial(simulation: &Simulation, r: &Reglages, m: &Milieu) -> usize {
    let substrat = simulation.substrat();
    let mut bloquees = 0;
    for j in 0..r.hauteur {
        for i in 0..r.largeur {
            let sous_obstacle = m.obstacles.iter().any(|o| dans_le_rectangle(i, j, o));
            assert_eq!(substrat.est_bloquee(i, j), sous_obstacle, "est_bloquee({i}, {j})");
            let obtenue = substrat.valeur(i, j);
            let voulue = attendue(r, m, i, j);
            assert_eq!(obtenue, voulue, "case ({i}, {j}) : {obtenue} au lieu de {voulue}");
            bloquees += usize::from(sous_obstacle);
        }
    }
    bloquees
}

fn est_hexadecimal_de_64(texte: &str) -> bool {
    texte.len() == 64 && texte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

// ---------------------------------------------------------------------------------------------------------------
// 1. Régression bit à bit : sans milieu, rien ne change.
// ---------------------------------------------------------------------------------------------------------------

/// Les trois réglages dont les empreintes ont été relevées sur le code d'avant T0 (commit 3447859) : une boîte
/// avec anastomose et ramifications serrées, une spore dans un coin sans ramification ni coût, une diffusion nulle.
fn reglages_b() -> Reglages {
    Reglages {
        largeur: 41,
        hauteur: 29,
        spore: (20, 14),
        substrat_initial: 0.5,
        diffusion: 0.15,
        croissance: croissance::Parametres { pas: 1.0, intervalle: 6.0, angle_ramification: 40, amplitude_marche: 15 },
        affinite: 0.2,
        cout: 0.01,
        anastomose: Some(anastomose::Reglage { rayon: 0.5, delai: 5 }),
    }
}

fn reglages_c() -> Reglages {
    Reglages {
        largeur: 33,
        hauteur: 17,
        spore: (0, 16),
        substrat_initial: 2.0,
        diffusion: 0.25,
        croissance: croissance::Parametres {
            pas: 0.7,
            intervalle: f64::INFINITY,
            angle_ramification: 30,
            amplitude_marche: 30,
        },
        affinite: 0.5,
        cout: 0.0,
        anastomose: None,
    }
}

fn reglages_d() -> Reglages {
    Reglages {
        largeur: 25,
        hauteur: 25,
        spore: (12, 12),
        substrat_initial: 0.3,
        diffusion: 0.0,
        croissance: croissance::Parametres { pas: 1.0, intervalle: 3.0, angle_ramification: 60, amplitude_marche: 10 },
        affinite: 0.1,
        cout: 0.02,
        anastomose: Some(anastomose::Reglage { rayon: 0.7, delai: 4 }),
    }
}

/// 1. Sans milieu, `nouvelle_avec_milieu(r, Milieu::default(), g)` et `nouvelle(r, g)` ont la même écriture
///    canonique au temps 0 et après 40 pas, pour des graines variées (zéro, petites, grandes) et trois réglages.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn sans_milieu_l_etat_est_celui_de_nouvelle_pour_toute_graine() {
    let graines = [0u64, 1, 2, 3, 7, 42, (1 << 40) + 5, u64::MAX];
    for (nom, r) in [("b", reglages_b()), ("c", reglages_c()), ("d", reglages_d())] {
        for graine in graines {
            let mut ancienne = Simulation::nouvelle(r.clone(), graine).expect("réglages valides");
            let mut nouvelle = construire(r.clone(), Milieu::default(), graine);
            assert_eq!(
                ancienne.ecrire_canonique(),
                nouvelle.ecrire_canonique(),
                "réglages {nom}, graine {graine}, temps 0 : l'écriture diffère"
            );
            ancienne.avancer_n(40);
            nouvelle.avancer_n(40);
            assert_eq!(
                ancienne.ecrire_canonique(),
                nouvelle.ecrire_canonique(),
                "réglages {nom}, graine {graine}, après 40 pas : l'écriture diffère"
            );
        }
    }
}

/// 1 (suite). Les empreintes que le code d'avant T0 donnait, recopiées : (réglages, graine, empreinte au temps 0,
///    empreinte après 60 pas). Le code de T0 ne doit les changer en rien, par `nouvelle` comme par
///    `nouvelle_avec_milieu` sans milieu.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn sans_milieu_les_empreintes_sont_celles_d_avant_t0() {
    let relevees: [(&str, u64, &str, &str); 6] = [
        (
            "b",
            7,
            "c1bdde55e610ba32e6f6dc7671fb101083a0ae0129c8ecc08db3057e99a3988f",
            "f5d43ec71f78377634dacbc85b4d3aee4a241425ef6673010e945925d0b176e6",
        ),
        (
            "b",
            (1 << 40) + 5,
            "bce1559067d9d1755fbfd9bda833553743dd022c689ba9875610d1f4c4503227",
            "8b0da8cdb6a316215f07699cb5762a6e0ca87d1253f00f4043c6d40c8c1e99a3",
        ),
        (
            "b",
            u64::MAX,
            "942d82207a284f34f802c962838e307c06bf98a78b2f97c4650575a7a8ff05f7",
            "89de4bbebbe9d5b2ad3476a5589c57bf328f414d3d1393adbeada9f8fadd71e5",
        ),
        (
            "c",
            3,
            "1df12b74ab67c84bcb2a503ca9ca58abfc9bcffa3d1aa01c722738962641b683",
            "cb4794aa639e0bf15960915798290762cca49554028929eed263c6db8b38b7e9",
        ),
        (
            "c",
            0,
            "948ed222f68e1be5127e49eb7576d0b63183f6161e42c38903cefe812aab0481",
            "16c76aca90952ee993c11689fc5971ff72e92e7c55544d1be82a2dc135d32212",
        ),
        (
            "d",
            11,
            "9b37e8fefbf69a89096b49f1fdd058c864671ed6e7393f10b80dc3a5225ae98f",
            "a4ec1463e54cc01e66786d63330b7e455a94f9db1f1631337129b33f41b4b0ce",
        ),
    ];
    for (nom, graine, au_depart, apres_60) in relevees {
        let r = match nom {
            "b" => reglages_b(),
            "c" => reglages_c(),
            _ => reglages_d(),
        };
        let mut par_nouvelle = Simulation::nouvelle(r.clone(), graine).expect("réglages valides");
        let mut par_milieu = construire(r, Milieu::default(), graine);
        for (nom_de_la_voie, simulation) in [("nouvelle", &mut par_nouvelle), ("nouvelle_avec_milieu", &mut par_milieu)]
        {
            assert_eq!(
                sha256_hex(&simulation.ecrire_canonique()),
                au_depart,
                "réglages {nom}, graine {graine}, {nom_de_la_voie}, temps 0 : empreinte différente de celle d'avant T0"
            );
            simulation.avancer_n(60);
            assert_eq!(
                sha256_hex(&simulation.ecrire_canonique()),
                apres_60,
                "réglages {nom}, graine {graine}, {nom_de_la_voie}, après 60 pas : empreinte différente de celle d'avant T0"
            );
        }
    }
}

/// 1 (suite). L'empreinte de la graine 1 à 80 pas sur `scenarios/couplage.txt` est
///    `references/couplage-graine-1.empreinte`, par `lire_scenario_complet` + `nouvelle_avec_milieu` comme par
///    `lire_scenario` + `nouvelle`. La référence est lue au moment du test.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn l_empreinte_de_reference_du_couplage_ne_change_pas() {
    let texte = include_str!("../scenarios/couplage.txt");
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join("couplage-graine-1.empreinte");
    let reference =
        std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("lecture de {} : {e}", chemin.display()));
    let reference = reference.trim_end();
    assert!(est_hexadecimal_de_64(reference), "référence inattendue : {reference:?}");

    let (r, m, duree) = lire_scenario_complet(texte).expect("le scénario couplé se lit");
    assert_eq!(m, Milieu::default(), "couplage.txt n'a ni source ni obstacle");
    assert_eq!(duree, 80);
    let mut complet = construire(r, m, 1);
    complet.avancer_n(duree);
    assert_eq!(complet.empreinte(), reference, "par lire_scenario_complet et nouvelle_avec_milieu");

    let (r, duree) = lire_scenario(texte).expect("le scénario couplé se lit");
    let mut ancien = Simulation::nouvelle(r, 1).expect("réglages valides");
    ancien.avancer_n(duree);
    assert_eq!(ancien.empreinte(), reference, "par lire_scenario et nouvelle");
}

// ---------------------------------------------------------------------------------------------------------------
// 4. Source.
// ---------------------------------------------------------------------------------------------------------------

/// 4. Le nombre de cases d'une source de rayon 0, 1, 2, 3 et 5 vaut 1, 5, 13, 29 et 81 : ces cases ont la
///    quantité de la source, les autres le substrat initial, et la masse initiale vaut la somme attendue. Le
///    décompte est aussi refait en entiers dans le test.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_source_de_rayon_0_1_2_3_5_couvre_1_5_13_29_81_cases() {
    let (largeur, hauteur) = (31, 31);
    for (rayon, nombre) in [(0usize, 1usize), (1, 5), (2, 13), (3, 29), (5, 81)] {
        let r = reglages(largeur, hauteur, (2, 2));
        let m = milieu(vec![source(15, 15, rayon, 7.0)], vec![]);
        let simulation = construire(r.clone(), m.clone(), 1);

        let mut dans_la_tache = 0;
        let mut hors_de_la_tache = 0;
        for j in 0..hauteur {
            for i in 0..largeur {
                let v = simulation.substrat().valeur(i, j);
                if v == 7.0 {
                    dans_la_tache += 1;
                    assert!(
                        dans_le_disque(i, j, 15, 15, rayon),
                        "rayon {rayon} : la case ({i}, {j}) vaut 7 hors du disque"
                    );
                } else {
                    hors_de_la_tache += 1;
                    assert_eq!(v, 1.0, "rayon {rayon} : la case ({i}, {j}) vaut {v}");
                    assert!(
                        !dans_le_disque(i, j, 15, 15, rayon),
                        "rayon {rayon} : la case ({i}, {j}) du disque ne vaut pas 7"
                    );
                }
            }
        }
        assert_eq!(dans_la_tache, nombre, "rayon {rayon} : nombre de cases de la source");
        let mut compte_en_entiers = 0;
        for dj in -(rayon as i64)..=(rayon as i64) {
            for di in -(rayon as i64)..=(rayon as i64) {
                if di * di + dj * dj <= (rayon * rayon) as i64 {
                    compte_en_entiers += 1;
                }
            }
        }
        assert_eq!(compte_en_entiers, nombre, "le décompte en entiers du test");
        assert_eq!(dans_la_tache + hors_de_la_tache, largeur * hauteur);

        let masse = simulation.substrat().masse_totale();
        let voulue = 7.0 * nombre as f64 + 1.0 * (largeur * hauteur - nombre) as f64;
        assert!((masse - voulue).abs() <= 1e-9, "rayon {rayon} : masse initiale {masse} au lieu de {voulue}");
        assert!((simulation.releve().substrat - voulue).abs() <= 1e-9, "rayon {rayon} : relevé du temps 0");
    }
}

/// 4 (suite). Le bord du disque : pour le rayon 2, les cases `(±2, 0)` et `(0, ±2)` en font partie (`≤`), pas
///    `(±2, ±1)` ni `(±1, ±2)` (distance² = 5 > 4) ; pour le rayon 5, `(3, 4)` en est (25 ≤ 25) et `(4, 4)` non.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn le_bord_du_disque_est_inclus() {
    let r = reglages(31, 31, (2, 2));
    let rayon_2 = construire(r.clone(), milieu(vec![source(15, 15, 2, 7.0)], vec![]), 1);
    for (di, dj, dedans) in [
        (2i64, 0i64, true),
        (-2, 0, true),
        (0, 2, true),
        (0, -2, true),
        (1, 1, true),
        (2, 1, false),
        (-2, -1, false),
        (1, 2, false),
        (-1, -2, false),
        (2, 2, false),
    ] {
        let v = rayon_2.substrat().valeur((15 + di) as usize, (15 + dj) as usize);
        assert_eq!(v == 7.0, dedans, "rayon 2, décalage ({di}, {dj}) : valeur {v}");
    }
    let rayon_5 = construire(r, milieu(vec![source(15, 15, 5, 7.0)], vec![]), 1);
    for (di, dj, dedans) in
        [(3i64, 4i64, true), (-4, 3, true), (5, 0, true), (4, 4, false), (5, 1, false), (-5, 1, false)]
    {
        let v = rayon_5.substrat().valeur((15 + di) as usize, (15 + dj) as usize);
        assert_eq!(v == 7.0, dedans, "rayon 5, décalage ({di}, {dj}) : valeur {v}");
    }
}

/// 4 (suite). Les cases d'une source **remplacent** le substrat initial (elles ne s'y ajoutent pas, et peuvent valoir
///    moins ou zéro) ; la dernière source du milieu l'emporte sur une case commune, dans un sens comme dans l'autre ;
///    une boîte qui n'est pas carrée et une source décentrée n'y changent rien.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn les_sources_remplacent_et_la_derniere_l_emporte() {
    let r = reglages(40, 30, (35, 3));
    let a = source(10, 12, 4, 5.0);
    let b = source(13, 12, 4, 0.25);
    let c = source(30, 22, 3, 0.0);
    for (nom, sources) in [
        ("a puis b", vec![a.clone(), b.clone(), c.clone()]),
        ("b puis a", vec![b.clone(), a.clone(), c.clone()]),
        ("c, b, a", vec![c.clone(), b.clone(), a.clone()]),
        ("a seule", vec![a.clone()]),
    ] {
        let m = milieu(sources, vec![]);
        let simulation = construire(r.clone(), m.clone(), 3);
        verifier_etat_initial(&simulation, &r, &m);
        // Garde contre un test vide : les trois valeurs sont bien présentes quand les trois sources y sont.
        if m.sources.len() == 3 {
            let valeurs: Vec<f64> = (0..30 * 40).map(|k| simulation.substrat().valeur(k % 40, k / 40)).collect();
            for voulue in [5.0, 0.25, 0.0, 1.0] {
                assert!(valeurs.contains(&voulue), "{nom} : aucune case ne vaut {voulue}");
            }
        }
    }
    // Une case de recouvrement précise : (11, 12) est dans a et dans b.
    let apres_b = construire(r.clone(), milieu(vec![a.clone(), b.clone()], vec![]), 1);
    let apres_a = construire(r.clone(), milieu(vec![b, a], vec![]), 1);
    assert_eq!(apres_b.substrat().valeur(11, 12), 0.25, "a puis b : la dernière (b) l'emporte");
    assert_eq!(apres_a.substrat().valeur(11, 12), 5.0, "b puis a : la dernière (a) l'emporte");
}

/// 4 (suite). Une source qui recouvre un obstacle ne le remplit pas : les cases d'obstacle valent `0.0` et restent
///    bloquées, les autres cases de la source valent la quantité, la masse initiale est la somme attendue ; et
///    l'obstacle reste à `0.0` pendant que la colonie pousse.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_source_qui_recouvre_un_obstacle_ne_le_remplit_pas() {
    let r = reglages(31, 31, (25, 25));
    let m = milieu(vec![source(10, 10, 4, 3.0)], vec![obstacle(8, 8, 10, 10)]);
    let mut simulation = construire(r.clone(), m.clone(), 2);
    let bloquees = verifier_etat_initial(&simulation, &r, &m);
    assert_eq!(bloquees, 9);
    // Les cases de la source qui ne sont pas sous l'obstacle, comptées en entiers.
    let mut source_libre = 0;
    for j in 0..31 {
        for i in 0..31 {
            if dans_le_disque(i, j, 10, 10, 4) && !dans_le_rectangle(i, j, &m.obstacles[0]) {
                source_libre += 1;
            }
        }
    }
    let voulue = 3.0 * source_libre as f64 + 1.0 * (31 * 31 - 9 - source_libre) as f64;
    let masse = simulation.substrat().masse_totale();
    assert!((masse - voulue).abs() <= 1e-9, "masse initiale {masse} au lieu de {voulue}");

    simulation.avancer_n(40);
    for j in 8..=10 {
        for i in 8..=10 {
            assert_eq!(
                simulation.substrat().valeur(i, j),
                0.0,
                "après 40 pas, la case d'obstacle ({i}, {j}) vaut {}",
                simulation.substrat().valeur(i, j)
            );
            assert!(simulation.substrat().est_bloquee(i, j));
        }
    }
}

/// 4 (suite). Une source dont le centre est dans la boîte mais dont le disque déborde est TRONQUÉE à la boîte : elle
///    est acceptée, ses seules cases dans la boîte valent la quantité (comparées une à une au disque recompté en
///    entiers), et rien n'en revient de l'autre côté de la boîte ni ne s'en décale. Les trois premiers décomptes sont
///    faits à la main : un quart de disque de rayon 3 en coin (4 + 3 + 3 + 1 = 11 cases), un quart de rayon 2
///    (3 + 2 + 1 = 6), un demi-disque de rayon 2 sur un bord (5 + 3 + 1 = 9).
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_source_qui_deborde_de_la_boite_est_tronquee() {
    let (largeur, hauteur) = (40usize, 30usize);
    let r = reglages(largeur, hauteur, (20, 15));
    let cas = [
        (source(0, 0, 3, 6.0), Some(11usize)),
        (source(39, 29, 2, 6.0), Some(6)),
        (source(0, 15, 2, 6.0), Some(9)),
        (source(1, 2, 4, 6.0), None),
        (source(38, 3, 4, 6.0), None),
        (source(5, 28, 4, 6.0), None),
        (source(20, 15, 1000, 6.0), Some(largeur * hauteur)),
    ];
    for (s, a_la_main) in cas {
        let m = milieu(vec![s.clone()], vec![]);
        let simulation = construire(r.clone(), m.clone(), 1);
        verifier_etat_initial(&simulation, &r, &m);
        let mut couvertes = 0usize;
        for j in 0..hauteur {
            for i in 0..largeur {
                if simulation.substrat().valeur(i, j) == 6.0 {
                    couvertes += 1;
                }
            }
        }
        if let Some(voulu) = a_la_main {
            assert_eq!(couvertes, voulu, "{s:?} : nombre de cases de la boîte couvertes");
        }
        // Garde contre un test vide : la source déborde vraiment (elle couvre quelque chose, mais moins que son disque
        // entier, compté en entiers sans tenir compte de la boîte).
        if s.rayon < 10 {
            let n = s.rayon as i64;
            let mut disque_entier = 0usize;
            for dj in -n..=n {
                for di in -n..=n {
                    if di * di + dj * dj <= n * n {
                        disque_entier += 1;
                    }
                }
            }
            assert!(couvertes > 0 && couvertes < disque_entier, "{s:?} : {couvertes} cases sur {disque_entier}");
        }
        let voulue = 6.0 * couvertes as f64 + 1.0 * (largeur * hauteur - couvertes) as f64;
        assert!((simulation.substrat().masse_totale() - voulue).abs() <= 1e-9, "{s:?} : masse initiale");
        assert!((simulation.releve().substrat - voulue).abs() <= 1e-9, "{s:?} : relevé du temps 0");
    }
}

/// 4 (suite). Un rayon bien plus grand que la boîte couvre toute la boîte, sans dépassement d'entier : `rayon²` pour
///    cent mille dépasse 2³¹ et pour un milliard dépasse 2⁵³ ; l'arithmétique « en entiers signés » de la spécification
///    doit suivre. (Au-delà de trois milliards, `rayon²` dépasse même 2⁶³ : la spécification n'en dit rien et le test
///    ne s'y aventure pas.)
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_source_de_tres_grand_rayon_couvre_toute_la_boite() {
    let (largeur, hauteur) = (17usize, 11usize);
    let r = reglages(largeur, hauteur, (3, 3));
    for rayon in [100usize, 100_000, 1_000_000, 1_000_000_000] {
        let m = milieu(vec![source(8, 5, rayon, 2.5)], vec![]);
        let simulation = construire(r.clone(), m.clone(), 1);
        verifier_etat_initial(&simulation, &r, &m);
        for j in 0..hauteur {
            for i in 0..largeur {
                assert_eq!(simulation.substrat().valeur(i, j), 2.5, "rayon {rayon}, case ({i}, {j})");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 3. Obstacle.
// ---------------------------------------------------------------------------------------------------------------

/// 3. Les cases bloquées sont exactement l'union des rectangles, bornes comprises, sur des boîtes carrée et non
///    carrée, avec des obstacles d'une case, contre les bords, sur les coins, qui se recouvrent ; elles valent `0.0`
///    au temps 0 ; la masse initiale est celle des seules cases libres.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn les_obstacles_bloquent_exactement_leurs_cases() {
    let cas = [
        (
            31,
            31,
            (16, 16),
            vec![
                obstacle(2, 1, 7, 3),
                obstacle(5, 5, 5, 5),
                obstacle(0, 20, 30, 20),
                obstacle(30, 0, 30, 30),
                obstacle(12, 12, 14, 12),
                obstacle(4, 2, 9, 4),
            ],
        ),
        (
            37,
            19,
            (18, 9),
            vec![
                obstacle(0, 0, 3, 0),
                obstacle(36, 18, 36, 18),
                obstacle(10, 2, 10, 16),
                obstacle(20, 8, 33, 8),
                obstacle(0, 18, 0, 18),
            ],
        ),
    ];
    for (largeur, hauteur, spore, obstacles) in cas {
        let r = reglages(largeur, hauteur, spore);
        let m = milieu(vec![], obstacles);
        let mut simulation = construire(r.clone(), m.clone(), 4);
        let bloquees = verifier_etat_initial(&simulation, &r, &m);
        let libres = largeur * hauteur - bloquees;
        assert!(
            (simulation.substrat().masse_totale() - libres as f64).abs() <= 1e-9,
            "masse initiale des cases libres"
        );
        simulation.avancer_n(25);
        for j in 0..hauteur {
            for i in 0..largeur {
                if m.obstacles.iter().any(|o| dans_le_rectangle(i, j, o)) {
                    assert_eq!(
                        simulation.substrat().valeur(i, j),
                        0.0,
                        "{largeur}×{hauteur}, après 25 pas, case bloquée ({i}, {j})"
                    );
                }
            }
        }
    }
}

/// 3 (suite). La masse se conserve avec des obstacles et des sources, une colonie qui se ramifie et s'anastomose :
///    à chaque pas, substrat + coût × biomasse reste égal à la masse initiale (tolérance `TOLERANCE_MASSE`),
///    aucune case n'est négative, les cases d'obstacle valent `0.0`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn la_masse_se_conserve_avec_des_obstacles_et_des_sources() {
    let mut r = reglages(61, 61, (30, 30));
    r.substrat_initial = 0.05;
    r.cout = 0.01;
    r.croissance.intervalle = 10.0;
    r.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 10 });
    let m = milieu(
        vec![source(30, 30, 3, 0.4), source(45, 20, 4, 0.6)],
        vec![obstacle(36, 10, 38, 40), obstacle(10, 45, 40, 46), obstacle(0, 0, 5, 5)],
    );
    for graine in [2u64, 9] {
        let mut simulation = construire(r.clone(), m.clone(), graine);
        let initiale = simulation.substrat().masse_totale();
        assert!(initiale > 10.0);
        // Le relevé du temps 0 compte la masse du substrat tel qu'il est, obstacles posés.
        assert_eq!(simulation.releves()[0].substrat, initiale, "graine {graine} : relevé du temps 0");
        for pas in 1..=60 {
            simulation.avancer();
            assert_eq!(
                simulation.releve().substrat,
                simulation.substrat().masse_totale(),
                "graine {graine}, pas {pas} : relevé"
            );
            let totale = simulation.substrat().masse_totale() + r.cout * simulation.biomasse();
            assert!(
                ((totale - initiale) / initiale).abs() <= TOLERANCE_MASSE,
                "graine {graine}, pas {pas} : substrat + coût × biomasse = {totale} au lieu de {initiale}"
            );
        }
        for j in 0..61 {
            for i in 0..61 {
                let v = simulation.substrat().valeur(i, j);
                assert!(v >= 0.0, "graine {graine} : case ({i}, {j}) négative : {v}");
                if m.obstacles.iter().any(|o| dans_le_rectangle(i, j, o)) {
                    assert_eq!(v, 0.0, "graine {graine} : la case d'obstacle ({i}, {j}) vaut {v}");
                }
            }
        }
        // Garde contre un test vide : la colonie a poussé.
        assert!(simulation.biomasse() >= 50.0, "graine {graine} : biomasse {}", simulation.biomasse());
    }
}

/// Les réglages des tests de mur : une seule pointe qui marche, du substrat seulement dans les taches.
fn reglages_de_mur() -> Reglages {
    let mut r = reglages_sans_ramification(41, 25, (12, 12));
    r.substrat_initial = 0.0;
    r.cout = 0.01;
    r
}

/// 3 (suite). Un mur qui traverse toute la boîte sépare deux compartiments dans la simulation : la masse de
///    chacun se conserve séparément (à gauche, avec le coût de la biomasse de la colonie), et un compartiment
///    initialement vide reste exactement à `0.0` après des centaines de pas, le mur aussi.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_mur_separe_deux_compartiments_dans_la_simulation() {
    let r = reglages_de_mur();
    let mur = obstacle(20, 0, 21, 24);
    for droite_vide in [true, false] {
        let mut sources = vec![source(12, 12, 5, 1.0)];
        if !droite_vide {
            sources.push(source(32, 12, 5, 2.0));
        }
        let m = milieu(sources, vec![mur.clone()]);
        for graine in [3u64, 4] {
            let mut simulation = construire(r.clone(), m.clone(), graine);
            let masse = |simulation: &Simulation, x_min: usize, x_max: usize| {
                let mut somme = 0.0;
                for j in 0..25 {
                    for i in x_min..=x_max {
                        somme += simulation.substrat().valeur(i, j);
                    }
                }
                somme
            };
            let (gauche0, droite0) = (masse(&simulation, 0, 19), masse(&simulation, 22, 40));
            assert!(gauche0 > 50.0);
            simulation.avancer_n(300);
            let gauche = masse(&simulation, 0, 19) + r.cout * simulation.biomasse();
            let droite = masse(&simulation, 22, 40);
            let nom = format!("graine {graine}, droite vide au départ : {droite_vide}");
            assert!(
                ((gauche - gauche0) / gauche0).abs() <= TOLERANCE_MASSE,
                "{nom} : masse à gauche {gauche} au lieu de {gauche0}"
            );
            if droite_vide {
                assert_eq!(droite0, 0.0);
                for j in 0..25 {
                    for i in 20..41 {
                        assert_eq!(
                            simulation.substrat().valeur(i, j),
                            0.0,
                            "{nom} : la case ({i}, {j}) derrière le mur vaut {}",
                            simulation.substrat().valeur(i, j)
                        );
                    }
                }
            } else {
                assert!(
                    ((droite - droite0) / droite0).abs() <= TOLERANCE_MASSE,
                    "{nom} : masse à droite {droite} au lieu de {droite0}"
                );
            }
            // Garde contre un test vide : la colonie a poussé (mesuré : de 8 à 58 selon la graine, la pointe butant
            // souvent contre le mur), et la diffusion a atteint le mur (mesuré : 0,19 en (19, 12)).
            assert!(simulation.biomasse() >= 3.0, "{nom} : biomasse {}", simulation.biomasse());
            assert!(simulation.substrat().valeur(19, 12) > 1e-6, "{nom} : rien n'a diffusé jusqu'au mur");
        }
    }
}

/// 3 (suite). La diffusion contourne un mur percé d'une porte : la masse de l'autre côté, nulle au départ, devient
///    positive ; la masse totale (avec le coût de la biomasse) reste conservée.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn la_diffusion_contourne_un_mur_perce_dans_la_simulation() {
    let r = reglages_de_mur();
    let m = milieu(vec![source(12, 12, 5, 1.0)], vec![obstacle(20, 0, 21, 9), obstacle(20, 12, 21, 24)]);
    let mut simulation = construire(r.clone(), m, 5);
    let initiale = simulation.substrat().masse_totale();
    let derriere = |simulation: &Simulation| {
        let mut somme = 0.0;
        for j in 0..25 {
            for i in 22..41 {
                somme += simulation.substrat().valeur(i, j);
            }
        }
        somme
    };
    assert_eq!(derriere(&simulation), 0.0);
    simulation.avancer_n(300);
    let passee = derriere(&simulation);
    assert!(passee > 1e-3 * initiale, "la masse passée par la porte est {passee} sur {initiale}");
    let totale = simulation.substrat().masse_totale() + r.cout * simulation.biomasse();
    assert!(((totale - initiale) / initiale).abs() <= TOLERANCE_MASSE, "masse totale {totale} au lieu de {initiale}");
}

// ---------------------------------------------------------------------------------------------------------------
// 6. Refus à la construction.
// ---------------------------------------------------------------------------------------------------------------

/// Exige que `nouvelle_avec_milieu` refuse : une erreur, qui n'est pas une erreur de substrat (la spécification
/// laisse le choix entre `ParametresInvalides` et une variante nouvelle), sans paniquer.
fn exiger_le_refus(nom: &str, r: Reglages, m: Milieu) {
    match Simulation::nouvelle_avec_milieu(r, m, 1).err() {
        None => panic!("{nom} : le milieu aurait dû être refusé"),
        Some(ErreurSimulation::Substrat(e)) => {
            panic!("{nom} : refusé comme une erreur de substrat ({e:?}), pas comme un milieu invalide")
        }
        Some(_) => {}
    }
}

/// 6. Une source hors de la boîte est refusée : `x == largeur`, `y == hauteur`, très loin.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_source_hors_de_la_boite_est_refusee() {
    let r = reglages(30, 20, (5, 5));
    exiger_le_refus("x == largeur", r.clone(), milieu(vec![source(30, 3, 2, 1.0)], vec![]));
    exiger_le_refus("y == hauteur", r.clone(), milieu(vec![source(3, 20, 2, 1.0)], vec![]));
    exiger_le_refus("x et y très grands", r.clone(), milieu(vec![source(1000, 1000, 0, 1.0)], vec![]));
    exiger_le_refus("x == largeur, rayon 0", r.clone(), milieu(vec![source(30, 0, 0, 1.0)], vec![]));
    exiger_le_refus(
        "la deuxième source seulement est hors de la boîte",
        r,
        milieu(vec![source(3, 3, 1, 1.0), source(4, 20, 1, 1.0), source(5, 5, 1, 1.0)], vec![]),
    );
}

/// 6. Une source de quantité négative ou non finie est refusée.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn une_source_de_quantite_negative_ou_non_finie_est_refusee() {
    let r = reglages(30, 20, (5, 5));
    for (nom, quantite) in [
        ("négative", -1.0),
        ("un peu négative", -1e-9),
        ("NaN", f64::NAN),
        ("infinie", f64::INFINITY),
        ("infinie négative", f64::NEG_INFINITY),
    ] {
        exiger_le_refus(&format!("quantité {nom}"), r.clone(), milieu(vec![source(10, 10, 2, quantite)], vec![]));
    }
}

/// 6. Un obstacle hors de la boîte, ou à l'envers, est refusé.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_obstacle_hors_de_la_boite_ou_a_l_envers_est_refuse() {
    let r = reglages(30, 20, (5, 5));
    exiger_le_refus("x1 == largeur", r.clone(), milieu(vec![], vec![obstacle(10, 10, 30, 12)]));
    exiger_le_refus("y1 == hauteur", r.clone(), milieu(vec![], vec![obstacle(10, 10, 12, 20)]));
    exiger_le_refus("x0 et x1 == largeur", r.clone(), milieu(vec![], vec![obstacle(30, 10, 30, 12)]));
    exiger_le_refus("très loin", r.clone(), milieu(vec![], vec![obstacle(100, 100, 200, 200)]));
    exiger_le_refus("x0 > x1", r.clone(), milieu(vec![], vec![obstacle(12, 10, 11, 12)]));
    exiger_le_refus("y0 > y1", r.clone(), milieu(vec![], vec![obstacle(10, 12, 12, 11)]));
    exiger_le_refus(
        "un seul des obstacles est refusé",
        r,
        milieu(vec![], vec![obstacle(10, 10, 12, 12), obstacle(20, 3, 19, 4), obstacle(1, 1, 2, 2)]),
    );
}

/// 6. Un obstacle qui contient la case de la spore est refusé, quelle que soit sa place dans le rectangle (case
///    unique, coin, bord, grand rectangle) ; un obstacle qui la touche seulement (voisin d'une case, en diagonale)
///    est accepté.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_obstacle_sur_la_spore_est_refuse() {
    let r = reglages(30, 20, (12, 8));
    for (nom, o) in [
        ("la case de la spore seule", obstacle(12, 8, 12, 8)),
        ("la spore au coin haut gauche", obstacle(12, 8, 15, 11)),
        ("la spore au coin bas droit", obstacle(9, 5, 12, 8)),
        ("la spore au milieu", obstacle(10, 6, 14, 10)),
        ("la spore sur le bord droit", obstacle(8, 7, 12, 9)),
        ("la spore sur le bord haut", obstacle(10, 8, 14, 8)),
        ("toute la boîte", obstacle(0, 0, 29, 19)),
        ("une colonne entière", obstacle(12, 0, 12, 19)),
        ("une ligne entière", obstacle(0, 8, 29, 8)),
    ] {
        exiger_le_refus(nom, r.clone(), milieu(vec![], vec![o]));
    }
    // Avec une source, un autre obstacle valide devant : la faute est toujours vue.
    exiger_le_refus(
        "obstacle sur la spore après un obstacle valide",
        r.clone(),
        milieu(vec![source(3, 3, 1, 1.0)], vec![obstacle(0, 0, 1, 1), obstacle(11, 7, 13, 9)]),
    );
    for (nom, o) in [
        ("à gauche de la spore", obstacle(8, 6, 11, 10)),
        ("à droite", obstacle(13, 6, 16, 10)),
        ("au-dessus", obstacle(10, 4, 14, 7)),
        ("en dessous", obstacle(10, 9, 14, 12)),
        ("en diagonale", obstacle(13, 9, 16, 12)),
    ] {
        let m = milieu(vec![], vec![o]);
        let simulation = Simulation::nouvelle_avec_milieu(r.clone(), m, 1);
        assert!(simulation.is_ok(), "{nom} : un obstacle qui ne contient pas la spore est refusé");
    }
}

/// 6. Les milieux limites, mais dans le contrat, sont acceptés : source au dernier rang et à la dernière colonne,
///    source sur la spore, quantité zéro, obstacles contre les bords, plusieurs de chaque, milieu vide. Les erreurs de
///    réglages sont inchangées : `nouvelle_avec_milieu` rend la même erreur que `nouvelle`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn les_milieux_limites_sont_acceptes_et_les_erreurs_de_reglages_inchangees() {
    let r = reglages(30, 20, (12, 8));
    let m = milieu(
        vec![
            source(29, 19, 0, 1.0),
            source(0, 0, 0, 2.0),
            source(12, 8, 1, 3.0),
            source(10, 10, 2, 0.0),
            source(5, 5, 3, 0.5),
        ],
        vec![
            obstacle(0, 19, 29, 19),
            obstacle(29, 0, 29, 18),
            obstacle(0, 10, 0, 10),
            obstacle(20, 2, 25, 6),
            obstacle(22, 3, 28, 5),
        ],
    );
    let simulation = construire(r.clone(), m.clone(), 1);
    assert_eq!(simulation.milieu(), &m);
    assert!(Simulation::nouvelle_avec_milieu(r.clone(), Milieu::default(), 1).is_ok(), "milieu vide");
    assert!(
        Simulation::nouvelle_avec_milieu(r, milieu(vec![source(0, 0, 0, 0.0)], vec![]), 1).is_ok(),
        "quantité nulle"
    );

    let mut hors_boite = reglages(30, 20, (5, 5));
    hors_boite.spore = (30, 0);
    let mut cout_negatif = reglages(30, 20, (5, 5));
    cout_negatif.cout = -0.001;
    let mut diffusion_instable = reglages(30, 20, (5, 5));
    diffusion_instable.diffusion = 0.3;
    let mut dimension_nulle = reglages(30, 20, (5, 5));
    dimension_nulle.largeur = 0;
    let mut substrat_nan = reglages(30, 20, (5, 5));
    substrat_nan.substrat_initial = f64::NAN;
    for (nom, mauvais) in [
        ("spore hors de la boîte", hors_boite),
        ("coût négatif", cout_negatif),
        ("diffusion instable", diffusion_instable),
        ("dimension nulle", dimension_nulle),
        ("substrat initial NaN", substrat_nan),
    ] {
        let attendue = Simulation::nouvelle(mauvais.clone(), 1).err();
        assert!(attendue.is_some(), "{nom} : `nouvelle` devrait refuser");
        let obtenue = Simulation::nouvelle_avec_milieu(mauvais, Milieu::default(), 1).err();
        assert_eq!(obtenue, attendue, "{nom} : l'erreur de `nouvelle_avec_milieu` diffère de celle de `nouvelle`");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Accesseur, écriture canonique, déterminisme.
// ---------------------------------------------------------------------------------------------------------------

/// 7 (suite). `milieu()` rend le milieu donné à la construction, vide par défaut, et ne change pas en route.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn milieu_rend_le_milieu_de_la_construction() {
    let r = reglages(30, 20, (5, 5));
    let sans = Simulation::nouvelle(r.clone(), 1).unwrap();
    assert_eq!(sans.milieu(), &Milieu::default());
    let m = milieu(vec![source(20, 10, 3, 2.5), source(21, 11, 0, 0.5)], vec![obstacle(10, 2, 11, 15)]);
    let mut simulation = construire(r, m.clone(), 1);
    assert_eq!(simulation.milieu(), &m);
    simulation.avancer_n(10);
    assert_eq!(simulation.milieu(), &m, "le milieu a changé en route");
}

/// La queue que l'écriture canonique ajoute à un milieu non vide : le nombre de sources en `u64`, chacune (`x`, `y`,
/// `rayon` en `u32`, `quantite` par `f64::to_bits`), le nombre d'obstacles en `u64`, chacun (`x0`, `y0`, `x1`, `y1`
/// en `u32`), tout en petit-boutiste. Écrite ici à la main.
fn queue_attendue(m: &Milieu) -> Vec<u8> {
    let mut octets = Vec::new();
    octets.extend_from_slice(&(m.sources.len() as u64).to_le_bytes());
    for s in &m.sources {
        octets.extend_from_slice(&(s.x as u32).to_le_bytes());
        octets.extend_from_slice(&(s.y as u32).to_le_bytes());
        octets.extend_from_slice(&(s.rayon as u32).to_le_bytes());
        octets.extend_from_slice(&s.quantite.to_bits().to_le_bytes());
    }
    octets.extend_from_slice(&(m.obstacles.len() as u64).to_le_bytes());
    for o in &m.obstacles {
        for v in [o.x0, o.y0, o.x1, o.y1] {
            octets.extend_from_slice(&(v as u32).to_le_bytes());
        }
    }
    octets
}

/// 1 et 7. L'écriture canonique d'un milieu non vide ajoute à la toute fin la queue décrite par la spécification,
///    et rien d'autre. Preuve exacte : avec des sources dont la quantité est le substrat initial, l'état simulé est
///    le même que sans milieu, à tous les pas, donc l'écriture est celle de la simulation sans milieu suivie de la
///    queue. Avec des obstacles, l'état diffère, mais la queue est la même, à la fin, à tous les pas.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn l_ecriture_canonique_ajoute_la_queue_du_milieu_a_la_fin() {
    let r = reglages(12, 9, (5, 4));
    // Sources qui ne changent aucune case : leur quantité est le substrat initial.
    let neutre = milieu(vec![source(2, 6, 0, 1.0), source(9, 2, 2, 1.0)], vec![]);
    for graine in [1u64, 2] {
        let mut sans = construire(r.clone(), Milieu::default(), graine);
        let mut avec = construire(r.clone(), neutre.clone(), graine);
        for pas in [0u32, 30] {
            if pas > 0 {
                sans.avancer_n(pas);
                avec.avancer_n(pas);
            }
            let mut attendu = sans.ecrire_canonique();
            attendu.extend_from_slice(&queue_attendue(&neutre));
            assert_eq!(avec.ecrire_canonique(), attendu, "graine {graine}, temps {pas} : sources neutres");
        }
    }
    // Les trois formes de milieu non vide : des sources seules, des obstacles seuls, les deux.
    let formes = [
        milieu(
            vec![source(2, 6, 1, 1.5), source(9, 2, 0, 0.1), source(3, 3, 2, 0.0), source(5, 1, 0, 12345.678)],
            vec![],
        ),
        milieu(vec![], vec![obstacle(0, 0, 1, 1), obstacle(8, 6, 11, 8)]),
        milieu(vec![source(2, 6, 1, 1.5)], vec![obstacle(8, 6, 11, 8)]),
    ];
    for (k, m) in formes.iter().enumerate() {
        let vide = construire(r.clone(), Milieu::default(), 3);
        let mut simulation = construire(r.clone(), m.clone(), 3);
        let queue = queue_attendue(m);
        let octets = simulation.ecrire_canonique();
        assert!(octets.ends_with(&queue), "forme {k}, temps 0 : la queue du milieu n'est pas à la fin");
        // Au temps 0, tout ce qui précède la queue a la longueur de l'écriture sans milieu (même boîte, même spore).
        assert_eq!(octets.len(), vide.ecrire_canonique().len() + queue.len(), "forme {k}, temps 0 : longueur");
        simulation.avancer_n(20);
        assert!(
            simulation.ecrire_canonique().ends_with(&queue),
            "forme {k}, temps 20 : la queue du milieu n'est pas à la fin"
        );
    }
}

/// 7. Déterminisme : même graine, même milieu, même écriture canonique et même empreinte, au temps 0 et plus tard ;
///    `avancer_n(n)` équivaut à `n` appels à `avancer` ; une autre graine, une autre empreinte.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn meme_graine_meme_milieu_meme_ecriture() {
    let r = reglages_b();
    let m = milieu(
        vec![source(8, 7, 3, 2.0), source(30, 20, 2, 0.1)],
        vec![obstacle(25, 2, 26, 22), obstacle(2, 25, 15, 26)],
    );
    let mut premiere = construire(r.clone(), m.clone(), 6);
    let mut seconde = construire(r.clone(), m.clone(), 6);
    assert_eq!(premiere.ecrire_canonique(), seconde.ecrire_canonique(), "temps 0");
    premiere.avancer_n(40);
    seconde.avancer_n(40);
    assert_eq!(premiere.ecrire_canonique(), seconde.ecrire_canonique(), "après 40 pas");
    assert_eq!(premiere.empreinte(), seconde.empreinte());
    assert!(est_hexadecimal_de_64(&premiere.empreinte()), "empreinte : {}", premiere.empreinte());

    let mut pas_a_pas = construire(r.clone(), m.clone(), 6);
    for _ in 0..40 {
        pas_a_pas.avancer();
    }
    assert_eq!(pas_a_pas.ecrire_canonique(), premiere.ecrire_canonique(), "avancer_n(40) contre 40 appels à avancer");

    let mut autre_graine = construire(r, m, 7);
    autre_graine.avancer_n(40);
    assert_ne!(autre_graine.empreinte(), premiere.empreinte(), "deux graines, deux empreintes");
}

/// 7 (suite). Deux milieux différents, deux empreintes, au temps 0 comme après des pas : sans milieu et avec une
///    source ; une quantité qui diffère du dernier bit ; une source ou un obstacle décalé d'une case ; un obstacle
///    de plus ; deux sources dans l'autre ordre ; une source de plus qui ne change aucune case.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn deux_milieux_differents_deux_empreintes() {
    let r = reglages(40, 30, (20, 15));
    let quantite_voisine = f64::from_bits(2.0f64.to_bits() + 1);
    let milieux = vec![
        Milieu::default(),
        milieu(vec![source(30, 8, 3, 2.0)], vec![]),
        milieu(vec![source(30, 8, 3, quantite_voisine)], vec![]),
        milieu(vec![source(31, 8, 3, 2.0)], vec![]),
        milieu(vec![source(30, 8, 3, 2.0), source(4, 4, 0, 1.0)], vec![]),
        milieu(vec![source(30, 8, 3, 2.0), source(4, 20, 0, 2.0)], vec![]),
        milieu(vec![source(4, 20, 0, 2.0), source(30, 8, 3, 2.0)], vec![]),
        milieu(vec![], vec![obstacle(5, 5, 6, 12)]),
        milieu(vec![], vec![obstacle(5, 6, 6, 13)]),
        milieu(vec![], vec![obstacle(5, 5, 6, 12), obstacle(30, 20, 31, 22)]),
        milieu(vec![source(30, 8, 3, 2.0)], vec![obstacle(5, 5, 6, 12)]),
    ];
    for pas in [0u32, 15] {
        let mut empreintes: Vec<(usize, String)> = Vec::new();
        for (k, m) in milieux.iter().enumerate() {
            let mut simulation = construire(r.clone(), m.clone(), 8);
            simulation.avancer_n(pas);
            let e = simulation.empreinte();
            if let Some((autre, _)) = empreintes.iter().find(|(_, deja)| *deja == e) {
                panic!(
                    "temps {pas} : les milieux {autre} et {k} ont la même empreinte ({:?} et {m:?})",
                    milieux[*autre]
                );
            }
            empreintes.push((k, e));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// T0 ne change ni la marche, ni l'extension, ni l'anastomose.
// ---------------------------------------------------------------------------------------------------------------

/// Un milieu que la colonie n'atteint pas en 30 pas (spore au centre d'une boîte de 101 × 101, pas de 1) : sa seule
/// trace est son propre terrain. La marche, l'extension, la ramification et l'anastomose ne dépendent pas de lui, et il
/// ne tire pas dans le générateur. Deux preuves :
///
/// 1. des taches qui ne changent aucune case (leur quantité est le substrat initial) et aucun obstacle : tout l'état
///    est celui de la simulation sans milieu, bit à bit, substrat compris (sans case bloquée, `pas()` rend les mêmes
///    octets qu'avant), suivi de la queue du milieu ;
/// 2. les mêmes taches et des obstacles lointains : la colonie est la même, au détail près de l'arithmétique (la
///    spécification ne fixe l'ordre des opérations de `pas()` que sans case bloquée, donc les cases libres peuvent
///    différer de quelques unités du dernier bit) : mêmes nombres de segments, de pointes, de ramifications et
///    d'anastomoses, mêmes apex et mêmes temps de segment, coordonnées, biomasse, rayon et substrat à 1e-9 près.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn un_milieu_lointain_ne_change_ni_la_marche_ni_l_extension() {
    let mut r = reglages(101, 101, (50, 50));
    r.croissance.intervalle = 10.0;
    r.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 10 });
    let sources = vec![source(4, 4, 3, 1.0), source(96, 96, 2, 1.0)];
    let neutre = milieu(sources.clone(), vec![]);
    let avec_obstacles =
        milieu(sources, vec![obstacle(88, 88, 100, 100), obstacle(0, 95, 10, 100), obstacle(90, 0, 100, 6)]);
    for graine in [1u64, 5] {
        let mut sans = construire(r.clone(), Milieu::default(), graine);
        sans.avancer_n(30);
        assert!(sans.segments().len() >= 30, "graine {graine} : la colonie n'a pas poussé");

        // 1. Sans obstacle : bit à bit.
        let mut neutre_simule = construire(r.clone(), neutre.clone(), graine);
        neutre_simule.avancer_n(30);
        let octets_sans = sans.ecrire_canonique();
        let octets_neutre = neutre_simule.ecrire_canonique();
        let queue = queue_attendue(&neutre);
        assert_eq!(octets_neutre.len(), octets_sans.len() + queue.len(), "graine {graine} : longueur");
        assert!(
            octets_neutre[..octets_sans.len()] == octets_sans[..],
            "graine {graine} : l'état avec des taches neutres diffère de l'état sans milieu"
        );
        assert!(octets_neutre[octets_sans.len()..] == queue[..], "graine {graine} : queue du milieu");

        // 2. Avec des obstacles lointains : la même colonie, à l'arithmétique près.
        let mut avec = construire(r.clone(), avec_obstacles.clone(), graine);
        avec.avancer_n(30);
        assert_eq!(avec.segments().len(), sans.segments().len(), "graine {graine} : nombre de segments");
        assert_eq!(avec.pointes(), sans.pointes(), "graine {graine} : pointes");
        assert_eq!(avec.ramifications(), sans.ramifications(), "graine {graine} : ramifications");
        assert_eq!(avec.anastomoses(), sans.anastomoses(), "graine {graine} : anastomoses");
        for (k, (a, b)) in avec.segments().iter().zip(sans.segments()).enumerate() {
            assert!(
                a.apex == b.apex && a.temps == b.temps,
                "graine {graine} : le segment {k} n'a pas le même apex ou pas"
            );
            for (nom, u, v) in [("x0", a.x0, b.x0), ("y0", a.y0, b.y0), ("x1", a.x1, b.x1), ("y1", a.y1, b.y1)] {
                assert!((u - v).abs() <= 1e-9, "graine {graine} : segment {k}, {nom} : {u} contre {v}");
            }
        }
        assert!((avec.biomasse() - sans.biomasse()).abs() <= 1e-9, "graine {graine} : biomasse");
        assert!((avec.rayon() - sans.rayon()).abs() <= 1e-9, "graine {graine} : rayon");
        for j in 0..101 {
            for i in 0..101 {
                if avec.substrat().est_bloquee(i, j) {
                    continue;
                }
                let (u, v) = (avec.substrat().valeur(i, j), sans.substrat().valeur(i, j));
                assert!((u - v).abs() <= 1e-9, "graine {graine} : la case libre ({i}, {j}) vaut {u} contre {v}");
            }
        }
        // Garde contre un test vide : les obstacles sont bien là.
        let substrat = avec.substrat();
        assert!(substrat.est_bloquee(100, 100) && substrat.est_bloquee(5, 97) && substrat.est_bloquee(95, 3));
    }
}

/// La pointe lit le substrat de sa case de départ, donc celui de la source : sans diffusion, sans ramification, sans
/// marche, sans coût, le premier segment mesure `pas · q / (affinité + q)` quand la spore est sur une source de
/// quantité `q`, et il n'y a aucun segment quand `q = 0`.
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn la_colonie_lit_le_substrat_de_la_source_sous_la_spore() {
    let mut r = reglages_sans_ramification(21, 21, (10, 10));
    r.diffusion = 0.0;
    r.cout = 0.0;
    r.croissance.amplitude_marche = 0;
    for q in [0.5, 3.0, 40.0] {
        let m = milieu(vec![source(10, 10, 0, q)], vec![]);
        let mut simulation = construire(r.clone(), m, 2);
        simulation.avancer();
        assert_eq!(simulation.segments().len(), 1, "quantité {q} : un segment au premier pas");
        let voulue = (r.croissance.pas * q) / (r.affinite + q);
        let obtenue = simulation.segments()[0].longueur();
        assert!((obtenue - voulue).abs() <= 1e-12, "quantité {q} : premier segment {obtenue} au lieu de {voulue}");
    }
    // Sous la spore, une source vide : pas de substrat, pas de croissance, et la colonie ne triche pas.
    let m = milieu(vec![source(10, 10, 0, 0.0)], vec![]);
    let mut simulation = construire(r, m, 2);
    simulation.avancer_n(20);
    assert!(simulation.segments().is_empty(), "des segments ont poussé sur une case vide");
    assert_eq!(simulation.biomasse(), 0.0);
    assert_eq!(simulation.pointes(), 1);
}

/// Sans tropisme, une pointe peut entrer dans un obstacle, mais elle s'y arrête faute de substrat : aucun segment ne
/// part jamais d'une case bloquée, et la masse reste conservée. Un anneau d'obstacles enferme la spore dans un
/// carré de 3 × 3 cases, soixante graines, cinquante pas : la plupart des pointes butent contre l'anneau. Le test
/// n'exige pas qu'une pointe y entre (un tropisme futur le lui interdira).
#[test]
#[cfg_attr(not(feature = "etape-5"), ignore)]
fn aucun_segment_ne_part_d_une_case_bloquee() {
    let r = reglages_sans_ramification(15, 15, (7, 7));
    let m = milieu(
        vec![],
        vec![
            obstacle(5, 5, 9, 5),
            obstacle(5, 9, 9, 9),
            obstacle(5, 6, 5, 8),
            obstacle(9, 6, 9, 8),
            obstacle(0, 0, 14, 0),
        ],
    );
    let mut segments_en_tout = 0;
    for graine in 1..=60u64 {
        let mut simulation = construire(r.clone(), m.clone(), graine);
        // Garde contre un anneau qui ne bloque rien : le test n'aurait alors rien à vérifier.
        for (i, j) in [(5, 5), (9, 9), (5, 7), (9, 7), (7, 0), (14, 0)] {
            assert!(simulation.substrat().est_bloquee(i, j), "l'anneau ne bloque pas ({i}, {j})");
        }
        assert!(!simulation.substrat().est_bloquee(7, 7) && !simulation.substrat().est_bloquee(6, 6));
        let initiale = simulation.substrat().masse_totale();
        simulation.avancer_n(50);
        segments_en_tout += simulation.segments().len();
        for segment in simulation.segments() {
            let (i, j) = (segment.x0 as usize, segment.y0 as usize);
            assert!(
                !simulation.substrat().est_bloquee(i, j),
                "graine {graine} : un segment part de la case bloquée ({i}, {j})"
            );
        }
        let totale = simulation.substrat().masse_totale() + r.cout * simulation.biomasse();
        assert!(
            ((totale - initiale) / initiale).abs() <= TOLERANCE_MASSE,
            "graine {graine} : masse {totale} au lieu de {initiale}"
        );
    }
    assert!(segments_en_tout >= 60, "les colonies n'ont presque pas poussé : {segments_en_tout} segments en tout");
}
