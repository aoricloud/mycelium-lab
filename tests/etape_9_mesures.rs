//! Tests d'acceptation de l'étape 9 (M0, mesurer la pièce), partie « sur la simulation ».
//!
//! Ils fixent `mesurer`, `mesurer_au_seuil`, `Simulation::reglages` et le branchement de la carte de densité sur de vraies
//! colonies : les six scénarios de la vitrine (`scenarios/couplage.txt`, `terrain.txt`, `chimiotropisme.txt`,
//! `chimiotropisme-sans.txt`, `mur.txt`, `mur-sans.txt`), plusieurs graines, plusieurs pas, et six scénarios à nous dont
//! la boîte **n'est pas carrée** (un échange de la largeur et de la hauteur, ou de `x` et `y`, ne se voit pas sur une
//! boîte carrée).
//!
//! - **La carte** : la somme de la carte est la biomasse (relative 1e-9), la densité est nulle dans toutes les cases
//!   bloquées, la carte suit la méthode des dalles (`outils::densite_par_dalles`, écrite à part) case par case, et elle
//!   est **exactement nulle là où les dalles le sont** (c'est ce que compte une case occupée au seuil nul).
//! - **Le bilan de nourriture** : `|restante + coût × biomasse − initiale|` est de l'ordre de l'arrondi, **à chaque pas**
//!   (relatif 1e-9), et les champs du bilan valent exactement ce que la spécification écrit (la nourriture initiale est le
//!   premier relevé, la restante la masse du substrat, la consommée `cout × biomasse` avec le coût **lu dans le fichier**).
//! - **Les mesures, à un seuil** : chaque champ de `MesuresPiece` est recalculé ici, à part, depuis la carte et le substrat
//!   (cases libres comptées case par case ; cases occupées : densité strictement supérieure au seuil ; morceaux par l'union
//!   de classes de `outils.rs` ; traversées par les fonctions de grille, et par le calcul de référence quand la boîte est
//!   petite), et a ses bornes (`0 ≤ remplissage ≤ 1`, une traversée a au moins une largeur ou une hauteur de cases et au
//!   plus le plus grand morceau, etc.). Les champs qui ne dépendent pas du seuil sont, au bit près, ceux de `mesurer`, et
//!   `mesurer` est `mesurer_au_seuil(…, 0.0)`.
//! - **Les seuils sur les six scénarios** : à des seuils croissants (dont la valeur exacte d'une case et un ulp dessous),
//!   les cases occupées et le plus grand morceau ne croissent jamais, une traversée ne naît jamais ; au seuil 1 (une
//!   longueur d'hyphe d'une case), chaque colonie de la vitrine se découpe en plusieurs morceaux ; au seuil de la densité
//!   maximale, il ne reste rien.
//! - **Le temps 0**, la monotonie au fil des pas (cases occupées, biomasse, densité maximale, plus grand morceau,
//!   nourriture consommée qui ne baissent pas ; nourriture restante qui ne monte pas).
//! - **`mesurer` et `mesurer_au_seuil` ne changent rien** : l'empreinte avant et après est la même, et une simulation
//!   mesurée à chaque pas arrive à la même empreinte qu'une jumelle jamais mesurée. Un seuil négatif ou non fini panique.
//! - **Les empreintes de référence du dépôt** des six scénarios aux pas de référence (la mesure ne change rien à la
//!   simulation : T0 à T4 restent identiques à l'octet).
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/`, `abs`, `sqrt`, `floor`, `min` et `max`.

#[path = "etape_9/outils.rs"]
mod outils;

use std::panic::{AssertUnwindSafe, catch_unwind};

use mycelium_lab::piece::{Bord, Carte, MesuresPiece, carte_de_densite, mesurer, mesurer_au_seuil, traversee};
use mycelium_lab::simulation::{Reglages, Simulation};
use outils::{
    AUTRES_SCENARIOS, SCENARIOS, cases_bloquees, construire, delai, densite_par_dalles, juste_sous,
    morceaux_de_reference, proche_relatif, somme_dans_l_ordre, texte_du_fichier, traversee_de_reference,
};

/// Les graines jouées à chaque pas.
const GRAINES: [u64; 3] = [1, 7, 31];

/// Les empreintes de référence de la spécification : scénario (fichier), graine, pas, empreinte du dépôt.
const REFERENCES: [(&str, u64, u32, &str); 6] = [
    ("terrain.txt", 1, 80, "9f4f39a25d39be7edf523cb6b7646fb6fe87d4fbd00a2fe117fc61b23f49ddc0"),
    ("mur.txt", 31, 120, "ddeb843cd259e73cf51dd52809c983545eee667bc07a28c3b16ff8572435bc71"),
    ("mur-sans.txt", 31, 120, "7f9e2eb264ecb4845df45f2cf2382fbfb8fd2acd830dc8c11223ca175889ef93"),
    ("couplage.txt", 1, 80, "2223a1f3bcbc8627866aba084052cac06e93b05842bacc0ba94778bc23ed1083"),
    ("chimiotropisme.txt", 7, 100, "5d9680f934d2cdbfe54d845a4d33d5d952ee3fc643eba19f7caf506d4d623ed7"),
    ("chimiotropisme-sans.txt", 7, 100, "7d3a681284a24a913fd4582e3f93720dec463d5ede3b8caa19ff172d2651cea1"),
];

/// Le nombre de cases libres de chaque scénario de la vitrine, compté ici d'après les fichiers (boîte moins les rectangles
/// d'obstacles) : le couplage 161 × 161 ; le terrain 100 × 100 moins deux obstacles de 4 × 40 ; les deux taches : rien ;
/// les deux murs : 100 × 100 moins un obstacle de 4 × 61.
const CASES_LIBRES: [usize; 6] = [25_921, 9_680, 10_000, 10_000, 9_756, 9_756];

/// Une simulation d'un scénario de la vitrine, lue sur le disque.
fn simulation_de_la_vitrine(index: usize, graine: u64) -> (Simulation, Reglages, u32) {
    construire(&texte_du_fichier(SCENARIOS[index].fichier), graine)
}

/// Le contexte lisible d'un contrôle.
fn contexte(nom: &str, graine: u64, pas: u32) -> String {
    format!("scénario {nom}, graine {graine}, pas {pas}")
}

/// Vrai si l'appel panique.
fn panique<F: FnOnce()>(f: F) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

/// Le bilan de nourriture, par le chemin le plus court, sans `mesurer` : `|restante + coût × biomasse − initiale|`.
fn ecart_direct(simulation: &Simulation, cout: f64) -> f64 {
    let initiale = simulation.releves()[0].substrat;
    let restante = simulation.substrat().masse_totale();
    (restante + cout * simulation.biomasse() - initiale).abs()
}

/// Les cases occupées d'une carte au seuil `seuil` : densité strictement supérieure, calculée ici.
fn occupees_au_seuil(carte: &Carte, seuil: f64) -> Vec<bool> {
    carte.valeurs().iter().map(|&v| v > seuil).collect()
}

/// Vérifie **tout** ce que `mesurer_au_seuil(simulation, seuil)` doit dire de l'état courant de `simulation`, en
/// recalculant chaque champ à part. `reglages` est ce que le texte du scénario a donné (le coût et la boîte viennent de
/// lui, pas de `Simulation::reglages`). Au seuil nul, vérifie aussi que `mesurer` donne les mêmes mesures. Rend les
/// mesures.
fn verifier_les_mesures(simulation: &Simulation, reglages: &Reglages, seuil: f64, ou: &str) -> MesuresPiece {
    let ou = &format!("{ou}, seuil {seuil}");
    let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
    let m = mesurer_au_seuil(simulation, seuil);
    let carte = carte_de_densite(largeur, hauteur, simulation.segments());
    let bloquees = cases_bloquees(simulation);
    let occupees = occupees_au_seuil(&carte, seuil);
    if seuil == 0.0 {
        assert_eq!(mesurer(simulation), m, "{ou} : mesurer est mesurer_au_seuil(…, 0.0)");
    }

    // La carte : la taille de la boîte, la somme, les cases bloquées, aucune case négative ; ses cases occupées.
    assert_eq!((carte.largeur(), carte.hauteur()), (largeur, hauteur), "{ou} : la forme de la carte");
    assert!(
        proche_relatif(carte.somme(), simulation.biomasse(), 1e-9)
            || (carte.somme() == 0.0 && simulation.biomasse() == 0.0),
        "{ou} : la somme de la carte {} n'est pas la biomasse {}",
        carte.somme(),
        simulation.biomasse()
    );
    for (i, &valeur) in carte.valeurs().iter().enumerate() {
        assert!(valeur >= 0.0 && valeur.is_finite(), "{ou} : la case ({}, {}) vaut {valeur}", i % largeur, i / largeur);
        assert!(
            !bloquees[i] || valeur == 0.0,
            "{ou} : la case bloquée ({}, {}) a une densité {valeur}",
            i % largeur,
            i / largeur
        );
    }
    assert_eq!(carte.occupees(seuil), occupees, "{ou} : Carte::occupees");

    // Les cases : libres, occupées, remplissage.
    let libres = bloquees.iter().filter(|&&b| !b).count();
    let nb_occupees = occupees.iter().filter(|&&o| o).count();
    assert_eq!(m.cases_libres, libres, "{ou} : cases libres");
    assert_eq!(m.cases_occupees, nb_occupees, "{ou} : cases occupées");
    assert!(m.cases_occupees <= m.cases_libres, "{ou} : plus de cases occupées que de cases libres");
    assert!((0.0..=1.0).contains(&m.remplissage), "{ou} : remplissage {}", m.remplissage);
    assert_eq!(
        m.remplissage.to_bits(),
        (nb_occupees as f64 / libres as f64).to_bits(),
        "{ou} : remplissage {} au lieu de {} / {}",
        m.remplissage,
        nb_occupees,
        libres
    );

    // Les densités : moyenne sur les cases libres, maximale (aucune ne dépend du seuil).
    let attendue_moyenne = somme_dans_l_ordre(carte.valeurs()) / libres as f64;
    assert_eq!(m.densite_moyenne.to_bits(), attendue_moyenne.to_bits(), "{ou} : densité moyenne");
    let mut maximale = 0.0f64;
    for &v in carte.valeurs() {
        maximale = maximale.max(v);
    }
    assert_eq!(m.densite_maximale.to_bits(), maximale.to_bits(), "{ou} : densité maximale");
    assert!(m.densite_maximale >= m.densite_moyenne && m.densite_moyenne >= 0.0, "{ou} : moyenne et maximum");
    assert!(m.densite_maximale <= simulation.biomasse() * (1.0 + 1e-12), "{ou} : le maximum dépasse la biomasse");
    // La moyenne est la biomasse répartie sur les cases libres (relative 1e-9) : elle ne compte ni les cases bloquées ni la
    // boîte entière.
    assert!(
        proche_relatif(m.densite_moyenne * libres as f64, simulation.biomasse(), 1e-9)
            || (m.densite_moyenne == 0.0 && simulation.biomasse() == 0.0),
        "{ou} : densité moyenne {} × {} cases libres ≠ biomasse {}",
        m.densite_moyenne,
        libres,
        simulation.biomasse()
    );

    // Le bilan : chaque champ vaut ce que la spécification écrit.
    assert_eq!(m.biomasse.to_bits(), simulation.biomasse().to_bits(), "{ou} : biomasse");
    assert_eq!(
        m.nourriture_initiale.to_bits(),
        simulation.releves()[0].substrat.to_bits(),
        "{ou} : nourriture initiale"
    );
    assert_eq!(
        m.nourriture_restante.to_bits(),
        simulation.substrat().masse_totale().to_bits(),
        "{ou} : nourriture restante"
    );
    assert_eq!(
        m.nourriture_consommee.to_bits(),
        (reglages.cout * simulation.biomasse()).to_bits(),
        "{ou} : nourriture consommée (le coût du fichier fois la biomasse)"
    );
    let ecart = (m.nourriture_restante + m.nourriture_consommee - m.nourriture_initiale).abs();
    assert_eq!(m.ecart_du_bilan.to_bits(), ecart.to_bits(), "{ou} : écart du bilan");
    assert!(
        m.ecart_du_bilan <= 1e-9 * m.nourriture_initiale,
        "{ou} : le bilan de nourriture ne tient pas : écart {} sur {}",
        m.ecart_du_bilan,
        m.nourriture_initiale
    );
    assert!(m.nourriture_consommee >= 0.0 && m.nourriture_restante <= m.nourriture_initiale * (1.0 + 1e-12));

    // Les morceaux : ceux de l'union de classes, sur les cases au-dessus du seuil.
    let tailles = morceaux_de_reference(largeur, hauteur, &occupees);
    assert_eq!(m.nombre_de_morceaux, tailles.len(), "{ou} : nombre de morceaux");
    assert_eq!(m.plus_grand_morceau, tailles.first().copied().unwrap_or(0), "{ou} : plus grand morceau");
    assert!(m.plus_grand_morceau <= m.cases_occupees, "{ou} : un morceau plus grand que les cases occupées");

    // Les traversées : les mêmes que celles des fonctions de grille (puis, quand la boîte est petite, que le calcul de
    // référence), avec leurs bornes.
    let gauche_droite = traversee(largeur, hauteur, &occupees, Bord::Gauche, Bord::Droite);
    let haut_bas = traversee(largeur, hauteur, &occupees, Bord::Haut, Bord::Bas);
    assert_eq!(m.traversee_gauche_droite, gauche_droite, "{ou} : traversée gauche-droite");
    assert_eq!(m.traversee_haut_bas, haut_bas, "{ou} : traversée haut-bas");
    if let Some(n) = m.traversee_gauche_droite {
        assert!(n >= largeur && n <= m.plus_grand_morceau, "{ou} : gauche-droite {n} pour une largeur de {largeur}");
    }
    if let Some(n) = m.traversee_haut_bas {
        assert!(n >= hauteur && n <= m.plus_grand_morceau, "{ou} : haut-bas {n} pour une hauteur de {hauteur}");
    }
    if largeur * hauteur <= 3000 {
        assert_eq!(
            m.traversee_gauche_droite,
            traversee_de_reference(largeur, hauteur, &occupees, 0, 1),
            "{ou} : gauche-droite (référence)"
        );
        assert_eq!(
            m.traversee_haut_bas,
            traversee_de_reference(largeur, hauteur, &occupees, 2, 3),
            "{ou} : haut-bas (référence)"
        );
    }
    m
}

/// Le contrôle de la carte contre la méthode des dalles : case par case, à 1e-9 près, et nulle exactement là où les
/// dalles le sont (les cases bloquées à 0 exactement).
fn verifier_la_carte_contre_les_dalles(simulation: &Simulation, reglages: &Reglages, ou: &str) {
    let (largeur, hauteur) = (reglages.largeur, reglages.hauteur);
    let carte = carte_de_densite(largeur, hauteur, simulation.segments());
    outils::verifier_la_forme(&carte, largeur, hauteur);
    let dalles = densite_par_dalles(largeur, hauteur, simulation.segments());
    let mut occupees = 0;
    for (i, (obtenu, attendu)) in carte.valeurs().iter().zip(&dalles).enumerate() {
        assert!(
            (obtenu - attendu).abs() <= 1e-9,
            "{ou} : la case ({}, {}) vaut {obtenu}, la méthode des dalles dit {attendu}",
            i % largeur,
            i / largeur
        );
        assert_eq!(
            *obtenu == 0.0,
            *attendu == 0.0,
            "{ou} : la case ({}, {}) vaut {obtenu}, la méthode des dalles dit {attendu}",
            i % largeur,
            i / largeur
        );
        occupees += usize::from(*obtenu > 0.0);
    }
    assert!(occupees > 0, "{ou} : garde : une colonie existe");
}

/// Joue un scénario pas à pas. À chaque pas, `mesurer` est appelée et son bilan de nourriture est vérifié (relatif 1e-9) ;
/// aux pas de `complets` et au dernier, tous les champs sont vérifiés à part. Vérifie aussi la monotonie. Rend la
/// simulation au dernier pas et ses dernières mesures.
fn parcourir(texte: &str, nom: &str, graine: u64, duree: u32, complets: &[u32]) -> (Simulation, MesuresPiece) {
    let (mut simulation, reglages, _) = construire(texte, graine);
    let mut precedentes: Option<MesuresPiece> = None;
    for pas in 0..=duree {
        if pas > 0 {
            simulation.avancer();
        }
        let ou = contexte(nom, graine, pas);
        let m = if complets.contains(&pas) || pas == duree {
            verifier_les_mesures(&simulation, &reglages, 0.0, &ou)
        } else {
            mesurer(&simulation)
        };
        // Le bilan à chaque pas, par deux chemins : celui de `mesurer` et le calcul direct.
        let initiale = simulation.releves()[0].substrat;
        assert!(m.ecart_du_bilan <= 1e-9 * initiale, "{ou} : bilan de `mesurer` : écart {}", m.ecart_du_bilan);
        let direct = ecart_direct(&simulation, reglages.cout);
        assert!(direct <= 1e-9 * initiale, "{ou} : bilan direct : écart {direct}");
        // Et l'écart de `mesurer` est, au bit près, `((restante + consommée) − initiale).abs()`, dans cet ordre.
        assert_eq!(m.ecart_du_bilan.to_bits(), direct.to_bits(), "{ou} : l'écart du bilan, au bit près");
        // La monotonie : la colonie ne fait que grandir, la nourriture que baisser.
        if let Some(p) = &precedentes {
            assert!(m.cases_occupees >= p.cases_occupees, "{ou} : les cases occupées ne baissent pas");
            assert!(m.biomasse >= p.biomasse, "{ou} : la biomasse ne baisse pas");
            assert!(m.densite_maximale >= p.densite_maximale, "{ou} : la densité maximale ne baisse pas");
            assert!(m.plus_grand_morceau >= p.plus_grand_morceau, "{ou} : le plus grand morceau ne rapetisse pas");
            assert!(m.nourriture_consommee >= p.nourriture_consommee, "{ou} : la nourriture consommée ne baisse pas");
            assert!(
                m.nourriture_restante <= p.nourriture_restante + 1e-9 * initiale,
                "{ou} : la nourriture restante ne monte pas"
            );
            assert_eq!(
                m.nourriture_initiale.to_bits(),
                p.nourriture_initiale.to_bits(),
                "{ou} : la nourriture initiale ne change pas"
            );
            assert_eq!(m.cases_libres, p.cases_libres, "{ou} : les cases libres ne changent pas");
        }
        precedentes = Some(m);
    }
    (simulation, precedentes.expect("au moins le temps 0"))
}

/// Ce que donnent les mesures à des seuils croissants sur une colonie : chaque seuil est vérifié champ par champ, et la
/// suite des seuils est monotone (cases occupées incluses les unes dans les autres, cases occupées et plus grand morceau
/// qui ne croissent pas, traversées qui ne naissent pas et ne raccourcissent pas). Rend les mesures, seuil par seuil.
fn verifier_les_seuils(
    simulation: &Simulation,
    reglages: &Reglages,
    seuils: &[f64],
    ou: &str,
) -> Vec<(f64, MesuresPiece)> {
    let au_seuil_nul = mesurer(simulation);
    let carte = carte_de_densite(reglages.largeur, reglages.hauteur, simulation.segments());
    let mut resultats: Vec<(f64, MesuresPiece)> = Vec::new();
    for &seuil in seuils {
        let m = verifier_les_mesures(simulation, reglages, seuil, ou);
        // Ce qui ne dépend pas du seuil est, au bit près, ce que donne `mesurer`.
        let independants = |x: &MesuresPiece| {
            [
                x.densite_moyenne,
                x.densite_maximale,
                x.biomasse,
                x.nourriture_initiale,
                x.nourriture_restante,
                x.nourriture_consommee,
                x.ecart_du_bilan,
            ]
            .map(f64::to_bits)
        };
        assert_eq!(independants(&m), independants(&au_seuil_nul), "{ou}, seuil {seuil} : les champs sans seuil");
        assert_eq!(m.cases_libres, au_seuil_nul.cases_libres, "{ou}, seuil {seuil} : cases libres");
        if let Some((seuil_avant, avant)) = resultats.last() {
            assert!(seuil > *seuil_avant, "les seuils du test sont croissants");
            let (bas, haut) = (occupees_au_seuil(&carte, *seuil_avant), occupees_au_seuil(&carte, seuil));
            assert!(bas.iter().zip(&haut).all(|(b, h)| *b || !*h), "{ou}, seuil {seuil} : une case apparaît");
            assert!(m.cases_occupees <= avant.cases_occupees, "{ou}, seuil {seuil} : les cases occupées croissent");
            assert!(
                m.plus_grand_morceau <= avant.plus_grand_morceau,
                "{ou}, seuil {seuil} : le plus grand morceau grandit"
            );
            for (apres, avant) in [
                (m.traversee_gauche_droite, avant.traversee_gauche_droite),
                (m.traversee_haut_bas, avant.traversee_haut_bas),
            ] {
                match (avant, apres) {
                    (None, Some(n)) => panic!("{ou}, seuil {seuil} : une traversée de {n} cases naît"),
                    (Some(a), Some(b)) => assert!(b >= a, "{ou}, seuil {seuil} : une traversée raccourcit"),
                    _ => {}
                }
            }
        }
        resultats.push((seuil, m));
    }
    resultats
}

/// Les seuils essayés sur une colonie : nul, un ulp sous la plus petite densité positive et cette densité exacte, la
/// densité moyenne, 0,5, 1 (une longueur d'hyphe d'une case), un ulp sous la densité maximale, la densité maximale, et
/// au-delà. Triés, sans doublon.
fn seuils_de(simulation: &Simulation, reglages: &Reglages) -> Vec<f64> {
    let carte = carte_de_densite(reglages.largeur, reglages.hauteur, simulation.segments());
    let positives: Vec<f64> = carte.valeurs().iter().copied().filter(|&v| v > 0.0).collect();
    let minimum = positives.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = positives.iter().copied().fold(0.0, f64::max);
    let m = mesurer(simulation);
    let mut seuils = vec![
        0.0,
        juste_sous(minimum),
        minimum,
        m.densite_moyenne,
        0.5,
        1.0,
        juste_sous(maximum),
        maximum,
        2.0 * maximum,
        1e300,
    ];
    seuils.sort_by(f64::total_cmp);
    seuils.dedup();
    seuils
}

// ---------------------------------------------------------------------------------------------------------------------
// Les six scénarios de la vitrine
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_six_scenarios_ont_leur_carte_leur_bilan_et_leurs_mesures() {
    delai();
    for (index, scenario) in SCENARIOS.iter().enumerate() {
        let texte = texte_du_fichier(scenario.fichier);
        for graine in GRAINES {
            let (_, derniere) =
                parcourir(&texte, scenario.nom, graine, scenario.duree, &[0, 1, 2, 5, 17, scenario.duree / 2]);
            assert_eq!(
                derniere.cases_libres, CASES_LIBRES[index],
                "{} : cases libres d'après le fichier",
                scenario.nom
            );
            // Une colonie de plusieurs dizaines de pas a laissé de l'hyphe.
            assert!(derniere.cases_occupees > 10 && derniere.biomasse > 10.0, "{} : une colonie existe", scenario.nom);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_six_scenarios_a_plusieurs_seuils() {
    delai();
    let mut traversee_coupee = false;
    for (index, scenario) in SCENARIOS.iter().enumerate() {
        for graine in GRAINES {
            let (mut simulation, reglages, duree) = simulation_de_la_vitrine(index, graine);
            simulation.avancer_n(duree);
            let ou = contexte(scenario.nom, graine, duree);
            let seuils = seuils_de(&simulation, &reglages);
            let resultats = verifier_les_seuils(&simulation, &reglages, &seuils, &ou);
            let au_seuil = |s: f64| &resultats.iter().find(|(seuil, _)| *seuil == s).expect("le seuil est joué").1;
            let nul = au_seuil(0.0);
            // Au seuil 1 (une longueur d'hyphe d'une case), la colonie se découpe : plusieurs morceaux, et le plus grand
            // est plus petit que toutes les cases occupées au seuil nul.
            let un = au_seuil(1.0);
            assert!(un.nombre_de_morceaux > 1, "{ou} : au seuil 1, {} morceau(x)", un.nombre_de_morceaux);
            assert!(un.plus_grand_morceau < nul.cases_occupees, "{ou} : au seuil 1, le plus grand morceau baisse");
            // Au seuil de la densité maximale, plus rien ; un ulp dessous, au moins la case la plus dense.
            let maximale = nul.densite_maximale;
            let rien = au_seuil(maximale);
            assert_eq!(
                (rien.cases_occupees, rien.plus_grand_morceau, rien.nombre_de_morceaux),
                (0, 0, 0),
                "{ou} : au seuil maximal"
            );
            assert_eq!((rien.traversee_gauche_droite, rien.traversee_haut_bas), (None, None), "{ou}");
            assert_eq!(rien.remplissage, 0.0, "{ou}");
            assert!(au_seuil(juste_sous(maximale)).cases_occupees >= 1, "{ou} : un ulp sous le maximum");
            traversee_coupee |= (nul.traversee_haut_bas.is_some() && un.traversee_haut_bas.is_none())
                || (nul.traversee_gauche_droite.is_some() && un.traversee_gauche_droite.is_none());
        }
    }
    // Garde : au moins une colonie traverse au seuil nul et plus au seuil 1.
    assert!(traversee_coupee, "garde : aucune traversée n'est coupée par le seuil 1");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_carte_des_six_scenarios_suit_la_methode_des_dalles() {
    delai();
    for (index, scenario) in SCENARIOS.iter().enumerate() {
        for graine in [1u64, 7] {
            for pas in [3, scenario.duree] {
                let (mut simulation, reglages, _) = simulation_de_la_vitrine(index, graine);
                simulation.avancer_n(pas);
                verifier_la_carte_contre_les_dalles(&simulation, &reglages, &contexte(scenario.nom, graine, pas));
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn au_temps_zero_la_piece_est_vide_et_le_bilan_exact() {
    delai();
    for (index, scenario) in SCENARIOS.iter().enumerate() {
        let (simulation, reglages, _) = simulation_de_la_vitrine(index, 1);
        let ou = contexte(scenario.nom, 1, 0);
        // `reglages()` rend les réglages donnés, tels quels.
        assert_eq!(*simulation.reglages(), reglages, "{ou} : Simulation::reglages");
        for seuil in [0.0, 1.0] {
            let m = verifier_les_mesures(&simulation, &reglages, seuil, &ou);
            assert_eq!(m.cases_libres, CASES_LIBRES[index], "{ou}");
            assert_eq!(m.cases_occupees, 0, "{ou}");
            assert_eq!(m.remplissage, 0.0, "{ou}");
            assert_eq!(m.densite_moyenne, 0.0, "{ou}");
            assert_eq!(m.densite_maximale, 0.0, "{ou}");
            assert_eq!(m.biomasse, 0.0, "{ou}");
            assert_eq!(m.nourriture_consommee, 0.0, "{ou}");
            assert_eq!(
                m.nourriture_restante.to_bits(),
                m.nourriture_initiale.to_bits(),
                "{ou} : rien n'est encore consommé"
            );
            assert_eq!(m.ecart_du_bilan, 0.0, "{ou}");
            assert_eq!((m.plus_grand_morceau, m.nombre_de_morceaux), (0, 0), "{ou}");
            assert_eq!((m.traversee_gauche_droite, m.traversee_haut_bas), (None, None), "{ou}");
            // La nourriture de départ : au moins une unité par case libre, d'après `substrat 1.0` des fichiers.
            assert!(
                m.nourriture_initiale >= CASES_LIBRES[index] as f64 * 0.999,
                "{ou} : nourriture initiale {}",
                m.nourriture_initiale
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn un_seuil_negatif_ou_non_fini_fait_paniquer_mesurer_au_seuil() {
    delai();
    let (mut simulation, _, _) = simulation_de_la_vitrine(4, 31);
    for pas in [0, 5] {
        simulation.avancer_n(pas);
        for mauvais in [-1.0, -f64::MIN_POSITIVE, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                panique(|| {
                    let _ = mesurer_au_seuil(&simulation, mauvais);
                }),
                "pas {pas} : le seuil {mauvais}"
            );
        }
        assert_eq!(mesurer_au_seuil(&simulation, -0.0), mesurer(&simulation), "pas {pas} : -0 est le seuil nul");
        assert!(
            !panique(|| {
                let _ = mesurer_au_seuil(&simulation, f64::MAX);
            }),
            "pas {pas} : f64::MAX est un seuil"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_empreintes_de_reference_restent_et_mesurer_ne_les_change_pas() {
    delai();
    for (fichier, graine, pas, attendue) in REFERENCES {
        let texte = texte_du_fichier(fichier);
        // Une jumelle jamais mesurée.
        let (mut intacte, _, duree) = construire(&texte, graine);
        intacte.avancer_n(pas);
        assert_eq!(
            intacte.empreinte(),
            attendue,
            "{fichier}, graine {graine}, pas {pas} : l'empreinte de référence du dépôt"
        );
        assert!(pas == duree, "{fichier} : le pas de référence est la durée du fichier");
        // Une simulation mesurée à chaque pas, à deux seuils, et à travers ses accesseurs.
        let (mut mesuree, reglages, _) = construire(&texte, graine);
        for p in 1..=pas {
            mesuree.avancer();
            let avant = mesuree.ecrire_canonique();
            let m = mesurer(&mesuree);
            let m1 = mesurer_au_seuil(&mesuree, 1.0);
            assert!(m.biomasse >= 0.0 && m1.biomasse >= 0.0);
            assert_eq!(*mesuree.reglages(), reglages);
            assert_eq!(
                mesuree.ecrire_canonique(),
                avant,
                "{fichier}, graine {graine}, pas {p} : mesurer a changé l'état"
            );
        }
        assert_eq!(
            mesuree.empreinte(),
            attendue,
            "{fichier}, graine {graine} : mesurée à chaque pas, la même empreinte"
        );
        assert_eq!(mesuree.ecrire_canonique(), intacte.ecrire_canonique());
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn mesurer_a_deux_etats_differents_donne_deux_mesures_et_le_meme_etat_les_memes() {
    delai();
    // Deux colonies de graines différentes n'ont pas les mêmes mesures ; deux colonies identiques, les mêmes, au bit près.
    let texte = texte_du_fichier("mur.txt");
    let (mut a, _, duree) = construire(&texte, 31);
    let (mut b, _, _) = construire(&texte, 31);
    let (mut c, _, _) = construire(&texte, 32);
    a.avancer_n(duree);
    b.avancer_n(duree);
    c.avancer_n(duree);
    let (ma, mb, mc) = (mesurer(&a), mesurer(&b), mesurer(&c));
    assert_eq!(ma, mb, "le même état, les mêmes mesures");
    assert_eq!(ma.biomasse.to_bits(), mb.biomasse.to_bits());
    assert_eq!(ma.densite_moyenne.to_bits(), mb.densite_moyenne.to_bits());
    assert_eq!(mesurer_au_seuil(&a, 1.0), mesurer_au_seuil(&b, 1.0));
    assert_ne!(ma, mc, "deux graines, deux colonies");
    assert!(
        ma.cases_occupees != mc.cases_occupees || ma.biomasse.to_bits() != mc.biomasse.to_bits(),
        "deux graines, deux colonies"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// Des boîtes qui ne sont pas carrées
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn des_boites_non_carrees_ont_leur_carte_leur_bilan_et_leurs_mesures() {
    delai();
    let (mut gauche_droite_oui, mut gauche_droite_non, mut haut_bas_oui, mut haut_bas_non) = (0, 0, 0, 0);
    let mut difference = false;
    let mut decoupees = 0;
    for (nom, texte) in AUTRES_SCENARIOS {
        let (_, reglages, duree) = construire(texte, 1);
        assert_ne!(reglages.largeur, reglages.hauteur, "{nom} : la boîte n'est pas carrée");
        for graine in 1..=6u64 {
            let (simulation, m) = parcourir(texte, nom, graine, duree, &[0, 1, 3, 10, 40, duree / 2]);
            gauche_droite_oui += usize::from(m.traversee_gauche_droite.is_some());
            gauche_droite_non += usize::from(m.traversee_gauche_droite.is_none());
            haut_bas_oui += usize::from(m.traversee_haut_bas.is_some());
            haut_bas_non += usize::from(m.traversee_haut_bas.is_none());
            difference |= m.traversee_gauche_droite != m.traversee_haut_bas;
            // La boîte : largeur × hauteur cases, moins les obstacles.
            assert!(m.cases_libres <= reglages.largeur * reglages.hauteur);
            // Une traversée n'est jamais plus courte que la dimension qu'elle traverse (et jamais l'autre).
            if let Some(n) = m.traversee_gauche_droite {
                assert!(n >= reglages.largeur, "{nom} : gauche-droite {n} < largeur {}", reglages.largeur);
            }
            if let Some(n) = m.traversee_haut_bas {
                assert!(n >= reglages.hauteur, "{nom} : haut-bas {n} < hauteur {}", reglages.hauteur);
            }
            // Les mêmes colonies, à des seuils croissants.
            if graine <= 2 {
                let ou = contexte(nom, graine, duree);
                let resultats = verifier_les_seuils(&simulation, &reglages, &seuils_de(&simulation, &reglages), &ou);
                decoupees += usize::from(resultats.iter().any(|(_, m)| m.nombre_de_morceaux > 1));
            }
        }
        // La carte suit les dalles aussi sur une boîte non carrée.
        let (mut simulation, reglages, duree) = construire(texte, 3);
        simulation.avancer_n(duree);
        verifier_la_carte_contre_les_dalles(&simulation, &reglages, &contexte(nom, 3, duree));
    }
    // Garde contre un test vide : on a bien vu des traversées et des absences, dans les deux sens, et un sens sans l'autre.
    assert!(
        gauche_droite_oui > 3 && gauche_droite_non > 3,
        "gauche-droite : {gauche_droite_oui} oui, {gauche_droite_non} non"
    );
    assert!(haut_bas_oui > 3 && haut_bas_non > 3, "haut-bas : {haut_bas_oui} oui, {haut_bas_non} non");
    assert!(difference, "garde : une colonie traverse dans un sens sans traverser dans l'autre");
    assert!(decoupees >= 6, "garde : seulement {decoupees} colonies se découpent en morceaux à un seuil");
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn la_densite_maximale_peut_etre_dans_la_premiere_ou_la_derniere_case() {
    delai();
    // Une spore dans la dernière case (ou la première) : la case la plus dense est souvent celle du coin. `densite_maximale`
    // la voit, même si c'est la toute dernière (ou la toute première) de `valeurs()`.
    for (nom, derniere) in [("coin_bas_droit", true), ("coin_haut_gauche", false)] {
        let (_, texte) = AUTRES_SCENARIOS.iter().find(|(n, _)| *n == nom).expect("le scénario existe");
        let mut vues = 0;
        for graine in 1..=12u64 {
            let (mut simulation, reglages, duree) = construire(texte, graine);
            simulation.avancer_n(duree);
            let m = mesurer(&simulation);
            let carte = carte_de_densite(reglages.largeur, reglages.hauteur, simulation.segments());
            let coin = if derniere { carte.valeurs().len() - 1 } else { 0 };
            let (mut plus_dense, mut maximum) = (0, 0.0f64);
            for (i, &v) in carte.valeurs().iter().enumerate() {
                if v > maximum {
                    (plus_dense, maximum) = (i, v);
                }
            }
            assert_eq!(m.densite_maximale.to_bits(), maximum.to_bits(), "{nom}, graine {graine} : densité maximale");
            vues += usize::from(plus_dense == coin);
        }
        // Garde contre un test vide : au moins une colonie a sa case la plus dense dans le coin.
        assert!(vues >= 1, "{nom} : aucune colonie n'a sa case la plus dense dans la case du coin");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-9"), ignore)]
fn les_cases_libres_d_une_boite_non_carree_se_comptent_case_par_case() {
    delai();
    // 70 × 40 moins un obstacle de 4 × 26 ; 30 × 80 moins 21 × 4 ; 24 × 12 ; 9 × 30 moins 3 × 3 ; 14 × 9 ; 9 × 14.
    let attendu = [70 * 40 - 4 * 26, 30 * 80 - 21 * 4, 24 * 12, 9 * 30 - 3 * 3, 14 * 9, 9 * 14];
    for ((nom, texte), cases) in AUTRES_SCENARIOS.iter().zip(attendu) {
        let (simulation, reglages, _) = construire(texte, 1);
        assert_eq!(*simulation.reglages(), reglages, "{nom} : Simulation::reglages");
        let m = mesurer(&simulation);
        assert_eq!(m.cases_libres, cases, "{nom} : cases libres");
        assert_eq!(m.nourriture_initiale.to_bits(), simulation.releves()[0].substrat.to_bits());
        let simulation_apres = {
            let (mut s, _, duree) = construire(texte, 2);
            s.avancer_n(duree);
            s
        };
        assert_eq!(mesurer(&simulation_apres).cases_libres, cases, "{nom} : les cases libres ne bougent pas");
    }
}
