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

    // Garde contre un test vide : une simulation qui ne fait rien conserve aussi la masse. Biomasse mesurée : 550,8.
    assert!(simulation.biomasse() >= 100.0, "la colonie n'a presque pas poussé : biomasse {}", simulation.biomasse());
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

    let (x_spore, y_spore) = (reglages.spore.0 as f64 + 0.5, reglages.spore.1 as f64 + 0.5);
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

    let mut segments = 0usize;
    for graine in 1..=3u64 {
        let mut simulation = simuler(reglages.clone(), graine);
        simulation.avancer_n(60);
        segments += simulation.segments().len();
        for segment in simulation.segments() {
            for (x, y) in [(segment.x0, segment.y0), (segment.x1, segment.y1)] {
                assert!((0.0..21.0).contains(&x), "graine {graine} : x = {x}");
                assert!((0.0..21.0).contains(&y), "graine {graine} : y = {y}");
            }
        }
    }
    assert!(segments >= 20, "seulement {segments} segments sur les trois graines");
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
    assert!((150..=190).contains(&total), "{total} anastomoses sur dix graines, hors de [150, 190]");

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
    assert!(matches!(lire_scenario("boite 64 48\n"), Err(ErreurLecture::Manquante(_))));
}

/// Les lignes du scénario couplé de référence, sans les sauts de ligne.
const LIGNES_COUPLAGE: [&str; 7] = [
    "boite 161 161",
    "spore 80 80",
    "substrat 1.0 0.2",
    "croissance 1.0 20.0 30 20",
    "couplage 0.1 0.002",
    "anastomose 0.5 10",
    "duree 80",
];

/// Le texte d'un scénario : chaque ligne suivie d'un saut de ligne.
fn texte_de(lignes: &[&str]) -> String {
    let mut texte = String::new();
    for ligne in lignes {
        texte.push_str(ligne);
        texte.push('\n');
    }
    texte
}

/// B. Le texte de référence est exactement celui du fichier `scenarios/couplage.txt`, et il se lit en les
///    réglages attendus.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn le_texte_de_reference_est_celui_du_fichier() {
    // Le commentaire d'en-tête que l'issue #14 montre dans ce fichier (une ligne `#`) n'est pas figé ici.
    let fichier: String = include_str!("../scenarios/couplage.txt")
        .lines()
        .filter(|ligne| !ligne.starts_with('#'))
        .map(|ligne| format!("{ligne}\n"))
        .collect();
    assert_eq!(texte_de(&LIGNES_COUPLAGE), fichier);
    let (reglages, duree) = lire_scenario(&texte_de(&LIGNES_COUPLAGE)).unwrap();
    assert_eq!(duree, 80);
    assert_eq!(reglages, reglages_couplage());
}

/// C. Sans sa ligne `anastomose`, le texte de référence se lit avec l'option coupée.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn l_anastomose_absente_vaut_none_dans_le_texte_de_reference() {
    let lignes: Vec<&str> = LIGNES_COUPLAGE.iter().copied().filter(|ligne| !ligne.starts_with("anastomose")).collect();
    let (reglages, duree) = lire_scenario(&texte_de(&lignes)).unwrap();
    assert_eq!(reglages.anastomose, None);
    assert_eq!(duree, 80);
}

/// F. Chaque directive obligatoire absente rend son nom.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chaque_directive_obligatoire_absente_donne_son_nom() {
    let obligatoires = [
        ("boite", "boite"),
        ("spore", "spore"),
        ("substrat", "substrat"),
        ("croissance", "croissance"),
        ("couplage", "couplage"),
        ("duree", "duree"),
    ];
    for (nom, debut) in obligatoires {
        let lignes: Vec<&str> = LIGNES_COUPLAGE.iter().copied().filter(|ligne| !ligne.starts_with(debut)).collect();
        assert_eq!(lire_scenario(&texte_de(&lignes)), Err(ErreurLecture::Manquante(nom)), "sans {nom}");
    }
}

/// F (suite). Les lignes vides et les commentaires sont ignorés, mais comptent dans les numéros de ligne.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn les_lignes_ignorees_comptent_dans_les_numeros_de_ligne() {
    let mut lignes = LIGNES_COUPLAGE.to_vec();
    lignes.insert(0, "# un commentaire");
    lignes.insert(1, "");
    lignes.push("inconnue 1 2");
    assert_eq!(lire_scenario(&texte_de(&lignes)), Err(ErreurLecture::LigneInvalide(LIGNES_COUPLAGE.len() + 3)));
}

/// G. Des lignes vides et des commentaires ajoutés ne changent pas le scénario lu.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn les_lignes_vides_et_les_commentaires_sont_ignores() {
    let mut lignes = LIGNES_COUPLAGE.to_vec();
    lignes.insert(0, "# un commentaire");
    lignes.insert(4, "");
    let (reglages, duree) = lire_scenario(&texte_de(&lignes)).unwrap();
    assert_eq!(duree, 80);
    assert_eq!(reglages, reglages_couplage());
}

/// H. Un nombre mal écrit donne le numéro de sa ligne.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn un_nombre_mal_ecrit_donne_sa_ligne() {
    let mut lignes = LIGNES_COUPLAGE.to_vec();
    let rang = lignes.iter().position(|ligne| ligne.starts_with("duree")).expect("une ligne duree");
    lignes[rang] = "duree quatre-vingts";
    assert_eq!(lire_scenario(&texte_de(&lignes)), Err(ErreurLecture::LigneInvalide(rang + 1)));
}

// ------------------------------------------------------------------------------------------------------------------
// Tests ajoutés par le juge des tests (voir l'issue #14) : ils ferment des trous où une implémentation fausse passait
// tous les tests ci-dessus.
// ------------------------------------------------------------------------------------------------------------------

/// B. Sans rien qui limite l'extension (affinité 0 et substrat 1 : la longueur voulue vaut exactement `pas` ; coût 0 ;
///    pas de diffusion ; pas d'anastomose ; boîte trop grande pour être touchée), la simulation fait exactement ce que
///    fait `croissance::Colonie` : mêmes segments à l'octet près (apex, temps, x0, y0, x1, y1), mêmes nombres de
///    pointes et de ramifications. Cela fixe d'un coup la marche persistante, la ramification (côté, angle, point de
///    naissance, probabilité), l'ordre des apex, le tirage de la direction de départ et la position de départ
///    `(x + 0,5 ; y + 0,5)`. Le pas vaut 1 : le produit `pas × cos` est exact, quel que soit l'ordre des opérations.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn sans_limite_la_simulation_est_la_colonie() {
    let mut reglages = reglages_base();
    reglages.substrat_initial = 1.0;
    reglages.affinite = 0.0;
    reglages.cout = 0.0;
    reglages.diffusion = 0.0;
    reglages.anastomose = None;
    reglages.croissance =
        croissance::Parametres { pas: 1.0, intervalle: 6.0, angle_ramification: 30, amplitude_marche: 20 };

    for graine in 1..=5u64 {
        let mut simulation = simuler(reglages.clone(), graine);
        simulation.avancer_n(40);
        let mut colonie = croissance::Colonie::depuis_spore((100.5, 100.5), reglages.croissance, graine);
        for _ in 0..40 {
            colonie.avancer();
        }

        let obtenus = simulation.segments();
        let attendus = colonie.segments();
        assert!(!attendus.is_empty(), "graine {graine} : la colonie n'a rien fait");
        assert_eq!(obtenus.len(), attendus.len(), "graine {graine} : nombre de segments");
        for (rang, (obtenu, attendu)) in obtenus.iter().zip(attendus).enumerate() {
            let lieu = format!("graine {graine}, segment {rang}");
            assert_eq!(obtenu.apex, attendu.apex, "{lieu} : apex");
            assert_eq!(obtenu.temps, attendu.temps, "{lieu} : temps");
            assert_eq!(
                obtenu.x0.to_bits(),
                attendu.x0.to_bits(),
                "{lieu} : x0 {} au lieu de {}",
                obtenu.x0,
                attendu.x0
            );
            assert_eq!(
                obtenu.y0.to_bits(),
                attendu.y0.to_bits(),
                "{lieu} : y0 {} au lieu de {}",
                obtenu.y0,
                attendu.y0
            );
            assert_eq!(
                obtenu.x1.to_bits(),
                attendu.x1.to_bits(),
                "{lieu} : x1 {} au lieu de {}",
                obtenu.x1,
                attendu.x1
            );
            assert_eq!(
                obtenu.y1.to_bits(),
                attendu.y1.to_bits(),
                "{lieu} : y1 {} au lieu de {}",
                obtenu.y1,
                attendu.y1
            );
        }
        // Aucun apex ne fusionne ici : tous les apex de la colonie sont des pointes.
        assert_eq!(simulation.pointes(), colonie.apex().len(), "graine {graine} : pointes");
        assert_eq!(simulation.ramifications(), colonie.nb_ramifications(), "graine {graine} : ramifications");
        // Garde contre un test vide : 36 à 492 ramifications mesurées sur ces cinq graines.
        assert!(simulation.ramifications() >= 20, "graine {graine} : {} ramifications", simulation.ramifications());

        // La spore posée dans la case (100, 100) place le premier apex au centre de la case.
        assert_eq!((obtenus[0].x0, obtenus[0].y0), (100.5, 100.5), "graine {graine} : point de départ");
    }
}

/// C. La consommation se fait dans la case de DÉPART de l'apex. Un seul pas, un seul apex qui va tout droit, pas 1,
///    affinité 0 (la longueur voulue est 1), coût 0,1 (plafond 10, jamais atteint) : le segment mesure 1, la case de
///    la spore perd `coût × 1 = 0,1`, et toutes les autres cases valent encore 1. Le segment part du centre de la
///    case et avance de 1 dans une direction dont le cosinus ou le sinus vaut au moins 0,707 en valeur absolue
///    (cos² + sin² = 1) : l'arrivée est donc toujours dans une autre case, que l'on trouve depuis le segment. Douze
///    graines couvrent des directions variées. Le test est joué sans diffusion puis avec une diffusion de 0,2 : le
///    substrat uniforme reste exactement uniforme quand il diffuse, et la diffusion du pas se fait AVANT l'extension
///    (ordre de l'issue), donc rien ne diffuse la consommation du pas et la case de départ vaut bien 0,9 à la fin.
///    Un second pas montre ensuite la diffusion à l'œuvre : la case de départ remonte à 0,9 + 0,4 D.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn la_consommation_se_fait_dans_la_case_de_depart() {
    let mut reglages = reglages_base();
    reglages.substrat_initial = 1.0;
    reglages.affinite = 0.0;
    reglages.cout = 0.1;
    reglages.croissance.intervalle = f64::INFINITY;
    reglages.croissance.amplitude_marche = 0;
    let (x, y) = reglages.spore;

    for (graine, diffusion) in (1..=12u64).flat_map(|graine| [(graine, 0.0), (graine, 0.2)]) {
        let mut simulation = simuler(Reglages { diffusion, ..reglages.clone() }, graine);
        let avant = simulation.substrat().masse_totale();
        simulation.avancer();

        assert_eq!(
            simulation.segments().len(),
            1,
            "graine {graine}, diffusion {diffusion} : un seul segment au premier pas"
        );
        let segment = simulation.segments()[0];
        assert!(
            (segment.longueur() - 1.0).abs() <= 1e-9,
            "graine {graine}, diffusion {diffusion} : segment de {}",
            segment.longueur()
        );
        let arrivee = (segment.x1 as usize, segment.y1 as usize);
        assert_ne!(
            arrivee,
            (x, y),
            "graine {graine}, diffusion {diffusion} : l'arrivée devrait être dans une autre case"
        );

        let depart = simulation.substrat().valeur(x, y);
        assert!(
            (depart - 0.9).abs() <= 1e-12,
            "graine {graine}, diffusion {diffusion} : case de départ {depart} au lieu de 0,9"
        );
        let reste = simulation.substrat().valeur(arrivee.0, arrivee.1);
        assert!(
            (reste - 1.0).abs() <= 1e-12,
            "graine {graine}, diffusion {diffusion} : case d'arrivée {reste} au lieu de 1"
        );
        // Tolérance 1e-9 : la masse totale (40 401) s'additionne avec des arrondis de l'ordre de 1e-11.
        let apres = simulation.substrat().masse_totale();
        assert!(
            (avant - apres - 0.1).abs() <= 1e-9,
            "graine {graine}, diffusion {diffusion} : masse {avant} puis {apres}"
        );

        // Un second pas : sa diffusion (avant l'extension) comble en partie le creux de la case de départ, dont les
        // quatre voisines valent encore 1 : 0,9 + D × (4 × 1 − 4 × 0,9) = 0,9 + 0,4 D. Sans diffusion, elle reste à 0,9.
        simulation.avancer();
        let comblee = simulation.substrat().valeur(x, y);
        let attendue = 0.9 + 0.4 * diffusion;
        assert!(
            (comblee - attendue).abs() <= 1e-12,
            "graine {graine}, diffusion {diffusion} : case de départ {comblee} au lieu de {attendue} au pas 2"
        );
    }
}

/// F. Le numéro d'une ligne invalide compte toutes les lignes du texte, y compris les commentaires et les lignes
///    vides (ici la quatrième) ; puis chaque ligne du scénario de référence, remplacée par une version mal formée
///    (valeur hors contrat, mot en trop ou en moins, nombre illisible), donne `LigneInvalide` avec son numéro.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn une_ligne_mal_formee_donne_son_numero() {
    assert_eq!(lire_scenario("# c\n\nboite 10 10\ninconnue\n"), Err(ErreurLecture::LigneInvalide(4)));

    let cas = [
        (0, "boite 0 5"),
        (0, "boite 5 0"),
        (0, "boite 10"),
        (0, "boite 10 abc"),
        (1, "spore -1 2"),
        (1, "spore 1"),
        (2, "substrat abc 0.2"),
        (2, "substrat 1.0"),
        (3, "croissance 1.0 20.0 30"),
        (3, "croissance 1.0 vingt 30 20"),
        (4, "couplage 0.1"),
        (4, "couplage 0.1 0.002 0.5"),
        (5, "anastomose abc 10"),
        (5, "anastomose 0.5"),
        (6, "duree 1.5"),
        (6, "duree"),
    ];
    for (rang, mauvaise) in cas {
        let mut lignes = LIGNES_COUPLAGE;
        lignes[rang] = mauvaise;
        assert_eq!(
            lire_scenario(&texte_de(&lignes)),
            Err(ErreurLecture::LigneInvalide(rang + 1)),
            "la ligne « {mauvaise} » devrait être invalide"
        );
    }
}

/// G. Les paramètres de croissance hors de leur contrat (ceux que `Parametres` fait paniquer) donnent une erreur,
///    pas une panique : `angle_ramification` et `amplitude_marche` d'au moins 360 (la table des directions n'a que 360
///    entrées), `intervalle` inférieur à 1.
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn les_parametres_de_croissance_hors_contrat_levent_une_erreur() {
    for cas in 0..3 {
        let mut reglages = reglages_base();
        let nom = match cas {
            0 => {
                reglages.croissance.angle_ramification = 360;
                "angle_ramification = 360"
            }
            1 => {
                reglages.croissance.amplitude_marche = 360;
                "amplitude_marche = 360"
            }
            _ => {
                reglages.croissance.intervalle = 0.5;
                "intervalle = 0,5"
            }
        };
        assert!(matches!(Simulation::nouvelle(reglages, 1), Err(ErreurSimulation::ParametresInvalides)), "{nom}");
    }

    // Les valeurs voisines, dans le contrat, sont acceptées : le refus ne vient pas d'un `nouvelle` qui refuserait tout.
    for cas in 0..3 {
        let mut reglages = reglages_base();
        match cas {
            0 => reglages.croissance.angle_ramification = 359,
            1 => reglages.croissance.amplitude_marche = 359,
            _ => reglages.croissance.intervalle = 1.0,
        }
        assert!(Simulation::nouvelle(reglages, 1).is_ok(), "réglage voisin n°{cas} accepté");
    }
}

/// H. La borne droite de la boîte est exclue. Un apex qui part vers +x (direction 0, graine cherchée ici) dans la
///    boîte 5 × 5, depuis `(2,5 ; 2,5)` avec un pas de 0,5, passe par 3,0, 3,5, 4,0 et 4,5 ; la cible 5,0 est hors de
///    la boîte `[0, 5)` et refusée. Il reste donc 4 segments, le dernier finit en x = 4,5, et ce nombre ne bouge
///    plus du pas 10 au pas 30 (l'apex ne repart ni en arrière ni ailleurs : amplitude 0).
#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn la_borne_droite_de_la_boite_est_exclue() {
    let parametres =
        croissance::Parametres { pas: 0.5, intervalle: f64::INFINITY, angle_ramification: 30, amplitude_marche: 0 };
    let graine = (0u64..100_000)
        .find(|&graine| croissance::Colonie::depuis_spore((2.5, 2.5), parametres, graine).apex()[0].direction == 0)
        .expect("une graine sur 360 part vers +x");
    let reglages = Reglages {
        largeur: 5,
        hauteur: 5,
        spore: (2, 2),
        substrat_initial: 1.0,
        diffusion: 0.0,
        croissance: parametres,
        affinite: 0.0,
        cout: 0.0,
        anastomose: None,
    };

    let mut simulation = simuler(reglages, graine);
    simulation.avancer_n(10);
    let au_pas_10 = simulation.segments().len();
    simulation.avancer_n(20);
    assert_eq!(simulation.segments().len(), au_pas_10, "le nombre de segments a bougé entre le pas 10 et le pas 30");
    assert_eq!(au_pas_10, 4, "segments d'un apex qui va de 2,5 à 4,5 par pas de 0,5");

    let segments = simulation.segments();
    assert_eq!((segments[0].x0, segments[0].y0, segments[0].x1), (2.5, 2.5, 3.0), "premier segment");
    let dernier = segments[segments.len() - 1];
    assert_eq!((dernier.x1, dernier.y1), (4.5, 2.5), "le dernier segment finit à 0,5 du mur");
}
