//! Tests d'acceptation de l'étape 2 (a) : le couplage entre la colonie et le substrat.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt`.

use mycelium_lab::anastomose;
use mycelium_lab::croissance;
use mycelium_lab::simulation::{ErreurLecture, ErreurSimulation, Reglages, Simulation, lire_scenario};
use mycelium_lab::substrat::Substrat;

/// Les paramètres de croissance de base : un pas de 1, une ramification tous les 20 pas, un angle de 30 degrés et
/// une marche persistante d'amplitude 20 degrés.
fn parametres_base() -> croissance::Parametres {
    croissance::Parametres { pas: 1.0, intervalle: 20.0, angle_ramification: 30, amplitude_marche: 20 }
}

/// Les réglages de base des tests : boîte 201 × 201, spore au centre, substrat uniforme, sans anastomose.
fn reglages_base() -> Reglages {
    Reglages {
        largeur: 201,
        hauteur: 201,
        spore: (100, 100),
        substrat_initial: 1.0,
        diffusion: 0.2,
        croissance: parametres_base(),
        affinite: 0.1,
        cout: 0.002,
        anastomose: None,
    }
}

fn simuler(reglages: Reglages, graine: u64) -> Simulation {
    Simulation::nouvelle(reglages, graine).expect("réglages valides")
}

fn verifier_cases_non_negatives(substrat: &Substrat) {
    for y in 0..substrat.hauteur() {
        for x in 0..substrat.largeur() {
            let valeur = substrat.valeur(x, y);
            assert!(valeur >= 0.0, "case ({x}, {y}) négative : {valeur}");
        }
    }
}

fn substrat_de_test() -> Substrat {
    let mut substrat = Substrat::nouveau(4, 3, 0.0, 1.0, 1.0).unwrap();
    substrat.deposer(1, 1, 2.0);
    substrat
}

/// 1. `retirer` prend exactement la quantité demandée quand la case la contient, et la masse totale baisse
///    d'autant.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_prend_la_quantite_demandee() {
    let mut substrat = substrat_de_test();
    let avant = substrat.masse_totale();

    let retire = substrat.retirer(1, 1, 0.5);
    assert!((retire - 0.5).abs() <= 1e-12, "retiré : {retire}");
    assert!((substrat.valeur(1, 1) - 1.5).abs() <= 1e-12, "reste : {}", substrat.valeur(1, 1));
    let apres = substrat.masse_totale();
    assert!((avant - apres - 0.5).abs() <= 1e-12, "masse : {avant} puis {apres}");
}

/// 1 (suite). Demander plus que le contenu vide la case et rend seulement ce qu'elle contenait.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_plus_que_le_contenu_vide_la_case() {
    let mut substrat = substrat_de_test();
    let avant = substrat.masse_totale();

    let retire = substrat.retirer(1, 1, 10.0);
    assert!((retire - 2.0).abs() <= 1e-12, "retiré : {retire}");
    assert!(substrat.valeur(1, 1).abs() <= 1e-12, "reste : {}", substrat.valeur(1, 1));
    let apres = substrat.masse_totale();
    assert!((avant - apres - 2.0).abs() <= 1e-12, "masse : {avant} puis {apres}");

    let vide = substrat.retirer(1, 1, 1.0);
    assert!(vide.abs() <= 1e-12, "case vide : {vide}");
}

/// 1 (suite). `retirer` panique hors de la grille, en x comme en y.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_panique_hors_grille_en_x() {
    let mut substrat = substrat_de_test();
    substrat.retirer(4, 1, 0.0);
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_panique_hors_grille_en_y() {
    let mut substrat = substrat_de_test();
    substrat.retirer(1, 3, 0.0);
}

/// 1 (suite). `retirer` panique pour une quantité négative.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_panique_quantite_negative() {
    let mut substrat = substrat_de_test();
    substrat.retirer(1, 1, -0.5);
}

/// 1 (suite). `retirer` panique pour une quantité non finie.
#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_panique_quantite_non_finie() {
    let mut substrat = substrat_de_test();
    substrat.retirer(1, 1, f64::NAN);
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn retirer_panique_quantite_infinie() {
    let mut substrat = substrat_de_test();
    substrat.retirer(1, 1, f64::INFINITY);
}

/// 2. La masse totale du substrat plus le coût de la biomasse reste constante, pas après pas, et aucune case ne
///    devient négative.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn la_masse_totale_se_conserve() {
    let cout = 0.01;
    let reglages = Reglages {
        largeur: 61,
        hauteur: 61,
        spore: (30, 30),
        substrat_initial: 0.05,
        diffusion: 0.2,
        croissance: croissance::Parametres { pas: 1.0, intervalle: 10.0, angle_ramification: 30, amplitude_marche: 20 },
        affinite: 0.1,
        cout,
        anastomose: Some(anastomose::Reglage { rayon: 0.5, delai: 10 }),
    };
    let mut simulation = simuler(reglages, 2);
    let initiale = simulation.substrat().masse_totale();
    verifier_cases_non_negatives(simulation.substrat());

    for pas in 0..60 {
        simulation.avancer();
        let totale = simulation.substrat().masse_totale() + cout * simulation.biomasse();
        assert!(
            ((totale - initiale) / initiale).abs() <= 1e-9,
            "pas {pas} : masse totale {totale} au lieu de {initiale}"
        );
        verifier_cases_non_negatives(simulation.substrat());
    }
}

/// 3. Sans substrat, la colonie ne pousse pas : aucun segment, biomasse et rayon nuls, une seule pointe.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn sans_substrat_pas_de_croissance() {
    let mut reglages = reglages_base();
    reglages.substrat_initial = 0.0;
    let mut simulation = simuler(reglages, 4);
    simulation.avancer_n(20);

    assert!(simulation.segments().is_empty(), "des segments ont poussé sans substrat");
    assert_eq!(simulation.biomasse(), 0.0);
    assert_eq!(simulation.rayon(), 0.0);
    assert_eq!(simulation.pointes(), 1);
}

/// 4. L'extension dépend du substrat, exactement : la biomasse vaut `substrat / (substrat + affinité)` fois la
///    longueur d'un pas par le nombre de pas, et chaque pas laisse un segment.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_extension_depend_du_substrat() {
    fn reglages_sans_cout() -> Reglages {
        let mut reglages = reglages_base();
        reglages.croissance.intervalle = f64::INFINITY;
        reglages.croissance.amplitude_marche = 0;
        reglages.cout = 0.0;
        reglages.diffusion = 0.0;
        reglages
    }

    let cas = [(0.5, 0.5, 0.5), (1.5, 0.5, 0.75), (1.0, 1e-12, 1.0)];
    for (substrat_initial, affinite, facteur) in cas {
        let mut reglages = reglages_sans_cout();
        reglages.substrat_initial = substrat_initial;
        reglages.affinite = affinite;
        let pas = reglages.croissance.pas;
        let mut simulation = simuler(reglages, 5);
        simulation.avancer_n(40);

        let attendue = facteur * pas * 40.0;
        let obtenue = simulation.biomasse();
        assert!(
            ((obtenue - attendue) / attendue).abs() <= 1e-9,
            "substrat {substrat_initial}, affinité {affinite} : biomasse {obtenue} au lieu de {attendue}"
        );
        assert_eq!(simulation.segments().len(), 40, "substrat {substrat_initial}, affinité {affinite}");
    }
}

/// 5. Le substrat plafonne l'extension : le premier segment mesure 0,5, aucun ne le dépasse, et la case de la
///    spore est vide après le premier pas.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn le_substrat_plafonne_l_extension() {
    let mut reglages = reglages_base();
    reglages.substrat_initial = 0.05;
    reglages.affinite = 0.0;
    reglages.cout = 0.1;
    reglages.diffusion = 0.0;
    reglages.croissance.intervalle = f64::INFINITY;
    reglages.croissance.amplitude_marche = 0;
    let (x, y) = reglages.spore;

    let mut simulation = simuler(reglages, 6);
    simulation.avancer();

    let segments = simulation.segments();
    assert!(!segments.is_empty(), "aucun segment au premier pas");
    assert!((segments[0].longueur() - 0.5).abs() <= 1e-9, "premier segment : {}", segments[0].longueur());
    let reste = simulation.substrat().valeur(x, y);
    assert!(reste.abs() <= 1e-12, "case de la spore : {reste}");

    simulation.avancer_n(9);
    for segment in simulation.segments() {
        assert!(segment.longueur() <= 0.5 + 1e-12, "segment de longueur {}", segment.longueur());
    }
    verifier_cases_non_negatives(simulation.substrat());
}

/// 6. Le rayon croît linéairement après une latence : la régression de la moyenne sur les graines 1 à 10, entre
///    les temps 40 et 80, a un R² d'au moins 0,995 et une pente entre 0,5 et 1,0 fois le pas.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn le_rayon_croit_lineairement() {
    let reglages = reglages_base();
    let pas = reglages.croissance.pas;
    let mut moyennes = vec![0.0f64; 81];

    for graine in 1..=10u64 {
        let mut simulation = simuler(reglages.clone(), graine);
        simulation.avancer_n(80);
        let releves = simulation.releves();
        assert_eq!(releves.len(), 81, "graine {graine}");
        for (moyenne, releve) in moyennes.iter_mut().zip(releves.iter()) {
            *moyenne += releve.rayon;
        }
    }
    for moyenne in &mut moyennes {
        *moyenne /= 10.0;
    }

    let (mut n, mut sx, mut sy, mut sxx, mut sxy, mut syy) = (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (temps, &rayon) in moyennes.iter().enumerate().skip(40) {
        let x = temps as f64;
        n += 1.0;
        sx += x;
        sy += rayon;
        sxx += x * x;
        sxy += x * rayon;
        syy += rayon * rayon;
    }

    let covariance = n * sxy - sx * sy;
    let pente = covariance / (n * sxx - sx * sx);
    let r2 = covariance * covariance / ((n * sxx - sx * sx) * (n * syy - sy * sy));
    assert!(r2 >= 0.995, "R² = {r2}");
    assert!(pente >= 0.5 * pas && pente <= 1.0 * pas, "pente = {pente}, pas = {pas}");
}

/// 7. Le déterminisme : même graine, même écriture et même empreinte ; deux graines, deux empreintes ;
///    `avancer_n` équivaut à des appels répétés ; les relevés sont complets et cohérents avec l'état final.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn determinisme_et_releves() {
    let reglages = reglages_base();

    let mut premiere = simuler(reglages.clone(), 7);
    let mut seconde = simuler(reglages.clone(), 7);
    premiere.avancer_n(50);
    seconde.avancer_n(50);
    assert_eq!(premiere.ecrire_canonique(), seconde.ecrire_canonique(), "même graine, même écriture");
    assert_eq!(premiere.empreinte(), seconde.empreinte(), "même graine, même empreinte");

    let empreinte = premiere.empreinte();
    assert_eq!(empreinte.len(), 64, "empreinte : {empreinte}");
    assert!(
        empreinte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "empreinte non hexadécimale minuscule : {empreinte}"
    );

    let mut autre_graine = simuler(reglages.clone(), 8);
    autre_graine.avancer_n(50);
    assert_ne!(premiere.empreinte(), autre_graine.empreinte(), "deux graines, deux empreintes");

    let mut pas_a_pas = simuler(reglages.clone(), 11);
    for _ in 0..40 {
        pas_a_pas.avancer();
    }
    let mut en_bloc = simuler(reglages.clone(), 11);
    en_bloc.avancer_n(40);
    assert_eq!(pas_a_pas.ecrire_canonique(), en_bloc.ecrire_canonique(), "avancer_n(n) doit égaler n appels à avancer");

    let releves = premiere.releves();
    assert_eq!(releves.len(), premiere.temps() as usize + 1, "un relevé par pas depuis le temps 0");
    for (temps, releve) in releves.iter().enumerate() {
        assert_eq!(releve.temps, temps as u32, "relevé {temps}");
    }
    let premier = releves[0];
    assert_eq!(premier.temps, 0);
    assert_eq!(premier.biomasse, 0.0);
    assert_eq!(premier.pointes, 1);
    assert_eq!(releves.last(), Some(&premiere.releve()));

    let final_ = premiere.releve();
    let biomasse: f64 = premiere.segments().iter().map(|segment| segment.longueur()).sum();
    assert!(
        (final_.biomasse - biomasse).abs() <= 1e-9 * biomasse.abs().max(1.0),
        "biomasse {} au lieu de {biomasse}",
        final_.biomasse
    );
    let masse = premiere.substrat().masse_totale();
    assert!(
        (final_.substrat - masse).abs() <= 1e-9 * masse.abs().max(1.0),
        "substrat {} au lieu de {masse}",
        final_.substrat
    );

    let (x_spore, y_spore) = (reglages.spore.0 as f64, reglages.spore.1 as f64);
    let mut rayon = 0.0f64;
    for segment in premiere.segments() {
        for (x, y) in [(segment.x0, segment.y0), (segment.x1, segment.y1)] {
            let dx = x - x_spore;
            let dy = y - y_spore;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > rayon {
                rayon = distance;
            }
        }
    }
    assert!((final_.rayon - rayon).abs() <= 1e-9 * rayon.abs().max(1.0), "rayon {} au lieu de {rayon}", final_.rayon);
}

/// 8. Les murs : toutes les extrémités de segments restent dans `[0, 21) × [0, 21)`.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn les_murs_arretent_les_segments() {
    let mut reglages = reglages_base();
    reglages.largeur = 21;
    reglages.hauteur = 21;
    reglages.spore = (10, 10);
    reglages.substrat_initial = 1.0;
    reglages.diffusion = 0.2;
    reglages.croissance.intervalle = 5.0;
    reglages.cout = 0.05;
    reglages.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 10 });

    for graine in 1..=3u64 {
        let mut simulation = simuler(reglages.clone(), graine);
        simulation.avancer_n(60);
        for segment in simulation.segments() {
            for (x, y) in [(segment.x0, segment.y0), (segment.x1, segment.y1)] {
                assert!((0.0..21.0).contains(&x), "graine {graine} : x = {x}");
                assert!((0.0..21.0).contains(&y), "graine {graine} : y = {y}");
            }
        }
    }
}

/// 9. L'anastomose : nulle quand l'option est coupée, au moins cinq sur dix graines quand elle est allumée,
///    reproductible, et chaque anastomose retire une pointe vivante.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_anastomose_est_optionnelle_et_reproductible() {
    let mut base = reglages_base();
    base.largeur = 101;
    base.hauteur = 101;
    base.spore = (50, 50);
    base.substrat_initial = 1.0;
    base.diffusion = 0.2;
    base.croissance.intervalle = 4.0;
    base.affinite = 0.1;
    base.cout = 0.002;

    let mut coupee = base.clone();
    coupee.anastomose = None;
    for graine in 1..=10u64 {
        let mut simulation = simuler(coupee.clone(), graine);
        simulation.avancer_n(30);
        assert_eq!(simulation.anastomoses(), 0, "graine {graine}, anastomose coupée");
    }

    let mut allumee = base;
    allumee.anastomose = Some(anastomose::Reglage { rayon: 0.5, delai: 10 });
    let mut total = 0u64;
    for graine in 1..=10u64 {
        let mut simulation = simuler(allumee.clone(), graine);
        simulation.avancer_n(30);
        total += simulation.anastomoses();
        assert_eq!(
            simulation.pointes() as u64 + simulation.anastomoses(),
            1 + simulation.ramifications(),
            "graine {graine}"
        );
    }
    assert!(total >= 5, "seulement {total} anastomoses sur dix graines");

    let mut premiere = simuler(allumee.clone(), 3);
    premiere.avancer_n(30);
    let mut seconde = simuler(allumee, 3);
    seconde.avancer_n(30);
    assert_eq!(premiere.anastomoses(), seconde.anastomoses(), "même graine, même nombre");
    assert_eq!(premiere.ecrire_canonique(), seconde.ecrire_canonique(), "même graine, même écriture");
}

/// 10. Les réglages refusés lèvent une erreur, sans paniquer ; un pas de diffusion instable est une erreur de
///     substrat.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn les_reglages_refuses_levent_une_erreur() {
    let mut hors_boite = reglages_base();
    hors_boite.spore = (hors_boite.largeur, 0);
    assert!(
        matches!(Simulation::nouvelle(hors_boite, 1), Err(ErreurSimulation::ParametresInvalides)),
        "spore hors de la boîte"
    );

    let mut cout_negatif = reglages_base();
    cout_negatif.cout = -0.001;
    assert!(
        matches!(Simulation::nouvelle(cout_negatif, 1), Err(ErreurSimulation::ParametresInvalides)),
        "coût négatif"
    );

    let mut substrat_negatif = reglages_base();
    substrat_negatif.substrat_initial = -0.1;
    assert!(
        matches!(Simulation::nouvelle(substrat_negatif, 1), Err(ErreurSimulation::ParametresInvalides)),
        "substrat initial négatif"
    );

    let mut affinite_negative = reglages_base();
    affinite_negative.affinite = -0.1;
    assert!(
        matches!(Simulation::nouvelle(affinite_negative, 1), Err(ErreurSimulation::ParametresInvalides)),
        "affinité négative"
    );

    let mut pas_nul = reglages_base();
    pas_nul.croissance.pas = 0.0;
    assert!(
        matches!(Simulation::nouvelle(pas_nul, 1), Err(ErreurSimulation::ParametresInvalides)),
        "pas de croissance nul"
    );

    let mut diffusion_instable = reglages_base();
    diffusion_instable.diffusion = 0.3;
    assert!(
        matches!(Simulation::nouvelle(diffusion_instable, 1), Err(ErreurSimulation::Substrat(..))),
        "diffusion instable"
    );
}

/// Le scénario couplé de référence, écrit à la main.
fn reglages_couplage() -> Reglages {
    Reglages {
        largeur: 161,
        hauteur: 161,
        spore: (80, 80),
        substrat_initial: 1.0,
        diffusion: 0.2,
        croissance: croissance::Parametres { pas: 1.0, intervalle: 20.0, angle_ramification: 30, amplitude_marche: 20 },
        affinite: 0.1,
        cout: 0.002,
        anastomose: Some(anastomose::Reglage { rayon: 0.5, delai: 10 }),
    }
}

/// 11. Le scénario couplé se lit depuis son texte, avec la durée et tous les réglages.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn le_scenario_couple_se_lit() {
    let (reglages, duree) = lire_scenario(include_str!("../scenarios/couplage.txt")).unwrap();
    assert_eq!(duree, 80);
    assert_eq!(reglages, reglages_couplage());
}

/// 11 (suite). Sans ligne `anastomose`, l'option vaut `None`.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_anastomose_absente_vaut_none() {
    let texte = concat!(
        "boite 161 161\n",
        "spore 80 80\n",
        "substrat 1.0 0.2\n",
        "croissance 1.0 20.0 30 20\n",
        "couplage 0.1 0.002\n",
        "duree 80\n",
    );
    let (reglages, duree) = lire_scenario(texte).unwrap();
    assert_eq!(reglages.anastomose, None);
    assert_eq!(duree, 80);
}

/// 11 (suite). Une ligne inconnue rend son numéro de ligne, à partir de 1.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn une_ligne_inconnue_donne_son_numero() {
    assert_eq!(lire_scenario("boite 10 10\ninconnue 1 2\n"), Err(ErreurLecture::LigneInvalide(2)));
}

/// 11 (suite). Une ligne obligatoire absente rend son nom.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn une_ligne_obligatoire_absente_donne_son_nom() {
    let texte = concat!(
        "boite 161 161\n",
        "substrat 1.0 0.2\n",
        "croissance 1.0 20.0 30 20\n",
        "couplage 0.1 0.002\n",
        "duree 80\n",
    );
    assert_eq!(lire_scenario(texte), Err(ErreurLecture::Manquante("spore")));
}

/// 11 (suite). L'intervalle de ramification peut s'écrire `infini`.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_intervalle_peut_s_ecrire_infini() {
    let texte = concat!(
        "boite 161 161\n",
        "spore 80 80\n",
        "substrat 1.0 0.2\n",
        "croissance 1.0 infini 30 20\n",
        "couplage 0.1 0.002\n",
        "duree 80\n",
    );
    let (reglages, _) = lire_scenario(texte).unwrap();
    assert_eq!(reglages.croissance.intervalle, f64::INFINITY);
}

/// 11 (suite). Le scénario vide de l'étape 0 n'est pas un scénario couplé.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn un_scenario_vide_n_est_pas_couple() {
    assert!(lire_scenario("boite 64 48\n").is_err());
}
