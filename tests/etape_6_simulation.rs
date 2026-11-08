//! Tests d'acceptation de l'étape 6 (T1, le chimiotropisme), partie simulation : `Tropismes`,
//! `Simulation::nouvelle_avec_tropismes`, `Simulation::tropismes`, `ErreurSimulation::TropismeInvalide` et l'écriture
//! canonique.
//!
//! Ils fixent : à force nulle (`Tropismes::default()`, `chimiotropisme: 0.0` ou `-0.0`), l'état est celui de
//! `nouvelle_avec_milieu`, octet pour octet, avec et sans milieu, pour des graines variées, à 0 et à 80 pas (mêmes
//! segments, donc aucun tirage de plus) ; les empreintes qu'avait le code d'avant T0 et la référence
//! `references/couplage-graine-1.empreinte` ne changent pas ; un `chimiotropisme` non fini, négatif ou supérieur à 45
//! (`FORCE_MAXIMALE`) est refusé par `ErreurSimulation::TropismeInvalide`, après les contrôles des réglages et du milieu ;
//! les bornes 0 et 45 sont acceptées ; `tropismes()` rend ce qui a été donné ; l'écriture canonique est celle de T0 à force nulle, et
//! ajoute sinon la queue du milieu (toujours écrite, deux `u64` à zéro si le milieu est vide) puis
//! `chimiotropisme.to_bits()` en petit-boutiste ; deux forces différentes, deux empreintes ; même graine, même milieu,
//! même force, même empreinte.
//!
//! Ils laissent ouverts : la variante d'erreur des réglages et du milieu invalides (celle d'avant T1, quelle qu'elle soit :
//! on exige seulement que le tropisme n'y change rien) ; que `Tropismes` soit `Clone` ou `Copy` (les tests le reconstruisent à chaque fois) ; le comportement de
//! la colonie (voir `etape_6_comportement.rs`).
//!
//! Ils n'utilisent aucune fonction mathématique de `f64` (seulement `to_bits` et `from_bits`).

use std::path::Path;

use mycelium_lab::anastomose;
use mycelium_lab::croissance;
use mycelium_lab::empreinte::sha256_hex;
use mycelium_lab::simulation::{
    ErreurSimulation, Milieu, Obstacle, Reglages, Simulation, Source, lire_scenario_complet, lire_scenario_tropismes,
};
use mycelium_lab::substrat::ErreurSubstrat;
use mycelium_lab::tropisme::Tropismes;

fn source(x: usize, y: usize, rayon: usize, quantite: f64) -> Source {
    Source { x, y, rayon, quantite }
}

fn obstacle(x0: usize, y0: usize, x1: usize, y1: usize) -> Obstacle {
    Obstacle { x0, y0, x1, y1 }
}

/// Des tropismes de force `f`, sans littéral complet : T2 ajoutera un champ à `Tropismes`, et `..Tropismes::default()` garde
/// les tests valables. Tant qu'il n'y a qu'un champ, clippy trouve la mise à jour inutile (`needless_update`) : l'`allow`
/// tombera avec T2.
#[allow(clippy::needless_update)]
fn tropismes(f: f64) -> Tropismes {
    Tropismes { chimiotropisme: f, ..Tropismes::default() }
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

/// Les trois réglages dont les empreintes ont été relevées sur le code d'avant T0 (ce sont ceux de
/// `etape_5_milieu.rs`) : anastomose et ramifications serrées, spore dans un coin sans ramification ni coût, diffusion
/// nulle.
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

fn est_hexadecimal_de_64(texte: &str) -> bool {
    texte.len() == 64 && texte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// La queue que l'écriture canonique ajoute pour un milieu : le nombre de sources en `u64`, chacune (`x`, `y`, `rayon`
/// en `u32`, `quantite` par `f64::to_bits`), le nombre d'obstacles en `u64`, chacun (`x0`, `y0`, `x1`, `y1` en `u32`),
/// tout en petit-boutiste. Écrite ici à la main (même description que `etape_5_milieu.rs`).
fn queue_du_milieu(m: &Milieu) -> Vec<u8> {
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

fn construire(r: Reglages, m: Milieu, t: Tropismes, graine: u64) -> Simulation {
    Simulation::nouvelle_avec_tropismes(r, m, t, graine).expect("réglages, milieu et tropismes valides")
}

/// Un milieu non vide valide pour chacun des trois réglages de T0, et un milieu vide.
fn milieux_pour(nom: &str) -> Vec<Milieu> {
    let riche = match nom {
        "b" => Milieu {
            sources: vec![source(8, 7, 3, 2.0), source(30, 20, 2, 0.1)],
            obstacles: vec![obstacle(25, 2, 26, 22), obstacle(2, 25, 15, 26)],
        },
        "c" => Milieu { sources: vec![source(10, 5, 3, 3.0)], obstacles: vec![obstacle(15, 0, 16, 10)] },
        _ => Milieu {
            sources: vec![source(5, 5, 2, 1.0), source(20, 20, 3, 0.05)],
            obstacles: vec![obstacle(18, 3, 19, 20)],
        },
    };
    vec![Milieu::default(), riche]
}

// ---------------------------------------------------------------------------------------------------------------
// 1. Régression bit à bit : à force nulle, rien ne change.
// ---------------------------------------------------------------------------------------------------------------

/// 1. À force nulle (`Tropismes::default()`, `0.0` et `-0.0`), `nouvelle_avec_tropismes` et `nouvelle_avec_milieu` ont la
///    même écriture canonique au temps 0 et après 80 pas, pour trois réglages, un milieu vide et un milieu riche, huit
///    graines (zéro, petites, grandes). Les segments, les ramifications et les pointes sont les mêmes : une simulation à
///    force nulle ne consomme pas un seul tirage de plus.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn a_force_nulle_l_etat_est_celui_de_nouvelle_avec_milieu() {
    let graines = [0u64, 1, 2, 3, 7, 42, (1 << 40) + 5, u64::MAX];
    let mut segments_en_tout = 0;
    for (nom, r) in [("b", reglages_b()), ("c", reglages_c()), ("d", reglages_d())] {
        for (k, m) in milieux_pour(nom).into_iter().enumerate() {
            for graine in graines {
                let mut ancienne = Simulation::nouvelle_avec_milieu(r.clone(), m.clone(), graine).expect("valide");
                let mut neuves: Vec<(&str, Simulation)> =
                    [("défaut", Tropismes::default()), ("0.0", tropismes(0.0)), ("-0.0", tropismes(-0.0))]
                        .into_iter()
                        .map(|(etiquette, t)| (etiquette, construire(r.clone(), m.clone(), t, graine)))
                        .collect();
                for (etiquette, neuve) in &neuves {
                    assert_eq!(
                        ancienne.ecrire_canonique(),
                        neuve.ecrire_canonique(),
                        "réglages {nom}, milieu {k}, graine {graine}, force {etiquette}, temps 0 : l'écriture diffère"
                    );
                }
                ancienne.avancer_n(80);
                for (etiquette, neuve) in &mut neuves {
                    neuve.avancer_n(80);
                    assert_eq!(
                        ancienne.ecrire_canonique(),
                        neuve.ecrire_canonique(),
                        "réglages {nom}, milieu {k}, graine {graine}, force {etiquette}, après 80 pas : l'écriture diffère"
                    );
                    assert_eq!(ancienne.segments(), neuve.segments(), "segments, force {etiquette}");
                    assert_eq!(ancienne.ramifications(), neuve.ramifications());
                    assert_eq!(ancienne.pointes(), neuve.pointes());
                    assert_eq!(ancienne.releves(), neuve.releves(), "relevés, force {etiquette}");
                }
                segments_en_tout += ancienne.segments().len();
            }
        }
    }
    // Garde contre un test vide : les colonies ont poussé.
    assert!(segments_en_tout >= 5000, "les colonies n'ont presque pas poussé : {segments_en_tout} segments en tout");
}

/// 1 (suite). Les empreintes que le code d'avant T0 donnait, recopiées de `etape_5_milieu.rs` : (réglages, graine,
///    empreinte au temps 0, empreinte après 60 pas). Elles ne changent pas par `nouvelle_avec_tropismes` à force nulle.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn a_force_nulle_les_empreintes_sont_celles_d_avant_t0() {
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
        for (etiquette, t) in [("défaut", Tropismes::default()), ("0.0", tropismes(0.0)), ("-0.0", tropismes(-0.0))] {
            let mut simulation = construire(r.clone(), Milieu::default(), t, graine);
            assert_eq!(
                sha256_hex(&simulation.ecrire_canonique()),
                au_depart,
                "réglages {nom}, graine {graine}, force {etiquette}, temps 0 : empreinte différente de celle d'avant T0"
            );
            simulation.avancer_n(60);
            assert_eq!(
                sha256_hex(&simulation.ecrire_canonique()),
                apres_60,
                "réglages {nom}, graine {graine}, force {etiquette}, après 60 pas : empreinte différente de celle d'avant T0"
            );
        }
    }
}

/// 1 (suite). L'empreinte de la graine 1 à 80 pas sur `scenarios/couplage.txt` est `references/couplage-graine-1.empreinte`,
///    par `lire_scenario_complet` + `nouvelle_avec_tropismes` (force par défaut, `0.0`, `-0.0`) comme par
///    `lire_scenario_tropismes` (sans la ligne `chimiotropisme`, et avec `chimiotropisme 0`). La référence est lue au
///    moment du test.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn l_empreinte_de_reference_du_couplage_ne_change_pas_a_force_nulle() {
    let texte = include_str!("../scenarios/couplage.txt");
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join("couplage-graine-1.empreinte");
    let reference =
        std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("lecture de {} : {e}", chemin.display()));
    let reference = reference.trim_end();
    assert!(est_hexadecimal_de_64(reference), "référence inattendue : {reference:?}");

    let (r, m, duree) = lire_scenario_complet(texte).expect("le scénario couplé se lit");
    assert_eq!(m, Milieu::default(), "couplage.txt n'a ni source ni obstacle");
    assert_eq!(duree, 80);
    for (etiquette, t) in [("défaut", Tropismes::default()), ("0.0", tropismes(0.0)), ("-0.0", tropismes(-0.0))] {
        let mut simulation = construire(r.clone(), m.clone(), t, 1);
        simulation.avancer_n(duree);
        assert_eq!(simulation.empreinte(), reference, "force {etiquette}, par nouvelle_avec_tropismes");
    }
    for (etiquette, texte) in [
        ("sans la ligne", texte.to_string()),
        ("chimiotropisme 0", format!("{texte}chimiotropisme 0\n")),
        ("chimiotropisme 0.0", format!("{texte}chimiotropisme 0.0\n")),
    ] {
        let (r, m, t, duree) = lire_scenario_tropismes(&texte).expect("le scénario se lit");
        let mut simulation = construire(r, m, t, 1);
        simulation.avancer_n(duree);
        assert_eq!(simulation.empreinte(), reference, "{etiquette}, par lire_scenario_tropismes");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 7. Refus à la construction ; `tropismes()`.
// ---------------------------------------------------------------------------------------------------------------

/// Le résultat d'une construction, ramené à l'erreur éventuelle (`Simulation` n'est pas comparable).
fn erreur_de(r: Reglages, m: Milieu, t: Tropismes) -> Option<ErreurSimulation> {
    Simulation::nouvelle_avec_tropismes(r, m, t, 1).err()
}

/// Le `f64` juste au-dessus de `x` (pour `x` positif fini).
fn au_dessus(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// 7. Un `chimiotropisme` négatif, `NaN`, infini ou supérieur à 45 est refusé par `ErreurSimulation::TropismeInvalide`,
///    sans panique, avec et sans milieu, y compris le `f64` juste au-dessus de 45 et ceux juste en dessous de 0, et
///    l'ancienne borne 360.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn un_chimiotropisme_invalide_est_refuse() {
    let invalides = [
        -1.0,
        -0.001,
        -1e-300,
        -f64::MIN_POSITIVE,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        au_dessus(45.0),
        45.001,
        46.0,
        90.0,
        360.0,
        361.0,
        1000.0,
        1e300,
        f64::MAX,
        f64::MIN,
    ];
    let avec_milieu = Milieu { sources: vec![source(3, 3, 1, 2.0)], obstacles: vec![obstacle(8, 2, 8, 6)] };
    for f in invalides {
        for m in [Milieu::default(), avec_milieu.clone()] {
            assert_eq!(
                erreur_de(reglages(20, 10, (5, 5)), m, tropismes(f)),
                Some(ErreurSimulation::TropismeInvalide),
                "chimiotropisme {f} : aurait dû être refusé"
            );
        }
    }
}

/// 7. Les bornes et l'intérieur sont acceptés : `0.0`, `-0.0`, le plus petit positif, 0,5, 1, 2,5, 30, 44,99, le `f64`
///    juste en dessous de 45, 45 ; et `tropismes()` rend exactement ce qui a été donné (pour une simulation neuve comme
///    après des pas).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn les_forces_valides_sont_acceptees_et_tropismes_les_rend() {
    let juste_en_dessous = f64::from_bits(45.0f64.to_bits() - 1);
    for f in [0.0, -0.0, 5e-324, f64::MIN_POSITIVE, 1e-300, 0.5, 1.0, 2.5, 30.0, 44.99, juste_en_dessous, 45.0] {
        let mut simulation = construire(reglages(20, 10, (5, 5)), Milieu::default(), tropismes(f), 3);
        assert_eq!(simulation.tropismes(), &tropismes(f), "chimiotropisme {f} : tropismes() à la construction");
        assert_eq!(simulation.tropismes().chimiotropisme.to_bits(), f.to_bits(), "bits de {f}");
        simulation.avancer_n(5);
        assert_eq!(simulation.tropismes(), &tropismes(f), "chimiotropisme {f} : tropismes() après 5 pas");
    }
    // Par défaut, par `nouvelle` et par `nouvelle_avec_milieu` : les tropismes sont éteints.
    let r = reglages(20, 10, (5, 5));
    assert_eq!(Tropismes::default().chimiotropisme, 0.0);
    let par_nouvelle = Simulation::nouvelle(r.clone(), 1).expect("valide");
    assert_eq!(par_nouvelle.tropismes(), &Tropismes::default());
    let par_milieu = Simulation::nouvelle_avec_milieu(r.clone(), Milieu::default(), 1).expect("valide");
    assert_eq!(par_milieu.tropismes(), &Tropismes::default());
    let par_defaut = construire(r, Milieu::default(), Tropismes::default(), 1);
    assert_eq!(par_defaut.tropismes(), &Tropismes::default());
}

/// 7. Les contrôles des réglages et du milieu passent en premier : avec un tropisme invalide, la construction rend la
///    **même erreur** que `nouvelle_avec_milieu` sans tropisme (réglage invalide, milieu invalide, substrat instable ou
///    vide, l'un et l'autre ensemble), et jamais `TropismeInvalide`. Le test ne fixe pas quelle variante d'erreur sort :
///    celle d'avant, quelle qu'elle soit (comme `etape_5_milieu.rs`, il n'exige du milieu invalide qu'une erreur qui n'est pas
///    une erreur de substrat).
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn les_controles_des_reglages_et_du_milieu_passent_avant_le_tropisme() {
    let valide = reglages(20, 10, (5, 5));
    let modifie = |f: &dyn Fn(&mut Reglages)| {
        let mut r = valide.clone();
        f(&mut r);
        r
    };
    let reglages_invalides = [
        ("spore hors de la boîte", modifie(&|r| r.spore = (20, 5))),
        ("substrat négatif", modifie(&|r| r.substrat_initial = -1.0)),
        ("substrat NaN", modifie(&|r| r.substrat_initial = f64::NAN)),
        ("coût négatif", modifie(&|r| r.cout = -0.001)),
        ("diffusion instable", modifie(&|r| r.diffusion = 0.5)),
        ("boîte vide", modifie(&|r| r.largeur = 0)),
    ];
    let milieux_invalides = [
        ("source hors de la boîte", Milieu { sources: vec![source(20, 3, 1, 1.0)], obstacles: vec![] }),
        ("source de quantité négative", Milieu { sources: vec![source(3, 3, 1, -1.0)], obstacles: vec![] }),
        ("obstacle sur la spore", Milieu { sources: vec![], obstacles: vec![obstacle(4, 4, 6, 6)] }),
        ("obstacle hors de la boîte", Milieu { sources: vec![], obstacles: vec![obstacle(15, 2, 25, 4)] }),
        ("obstacle à l'envers", Milieu { sources: vec![], obstacles: vec![obstacle(6, 2, 4, 2)] }),
    ];
    let mauvais_tropismes = [-1.0, f64::NAN, 361.0, f64::INFINITY, 1e300];
    let avant = |r: &Reglages, m: &Milieu| Simulation::nouvelle_avec_milieu(r.clone(), m.clone(), 1).err();
    let mut cas: Vec<(String, Reglages, Milieu)> = Vec::new();
    for (nom, r) in &reglages_invalides {
        cas.push((nom.to_string(), r.clone(), Milieu::default()));
    }
    for (nom, m) in &milieux_invalides {
        cas.push((nom.to_string(), valide.clone(), m.clone()));
    }
    // Réglage et milieu invalides ensemble.
    cas.push((
        "réglage et milieu invalides".to_string(),
        reglages_invalides[0].1.clone(),
        milieux_invalides[0].1.clone(),
    ));
    for (nom, r, m) in cas {
        let attendue = avant(&r, &m);
        assert!(attendue.is_some(), "{nom} : garde, `nouvelle_avec_milieu` devrait refuser");
        for f in mauvais_tropismes {
            let obtenue = erreur_de(r.clone(), m.clone(), tropismes(f));
            assert_ne!(
                obtenue,
                Some(ErreurSimulation::TropismeInvalide),
                "{nom}, tropisme {f} : le tropisme a passé avant"
            );
            assert_eq!(obtenue, attendue, "{nom}, tropisme {f} : l'erreur n'est pas celle de `nouvelle_avec_milieu`");
        }
        // Avec un tropisme valide, la même erreur aussi.
        assert_eq!(erreur_de(r.clone(), m.clone(), tropismes(2.0)), attendue, "{nom}, tropisme valide");
    }
    // Un substrat instable ou vide reste une erreur de substrat, avec un tropisme invalide.
    assert!(matches!(
        erreur_de(reglages_invalides[4].1.clone(), Milieu::default(), tropismes(f64::NAN)),
        Some(ErreurSimulation::Substrat(ErreurSubstrat::PasInstable))
    ));
    assert!(matches!(
        erreur_de(reglages_invalides[5].1.clone(), Milieu::default(), tropismes(-1.0)),
        Some(ErreurSimulation::Substrat(ErreurSubstrat::DimensionNulle))
    ));
}

// ---------------------------------------------------------------------------------------------------------------
// 9. Écriture canonique.
// ---------------------------------------------------------------------------------------------------------------

/// 9. À force nulle, l'écriture canonique est celle de T0, octet pour octet, avec et sans milieu, au temps 0 et plus
///    tard ; elle n'a ni la queue d'un milieu vide ni les octets de la force. Avec une force non nulle et un milieu vide,
///    elle a **24 octets de plus** au temps 0 : la queue du milieu (deux `u64` à zéro) puis les 8 octets de la force.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn l_ecriture_canonique_est_celle_de_t0_a_force_nulle_et_ajoute_24_octets_sinon() {
    let r = reglages(12, 9, (5, 4));
    let sans_milieu = construire(r.clone(), Milieu::default(), Tropismes::default(), 3);
    let t0 = sans_milieu.ecrire_canonique();
    for f in [0.0, -0.0] {
        let nulle = construire(r.clone(), Milieu::default(), tropismes(f), 3);
        assert_eq!(nulle.ecrire_canonique(), t0, "force {f}, milieu vide : octets de T0");
    }
    for f in [1.0, 2.5, 45.0, 1e-300] {
        let active = construire(r.clone(), Milieu::default(), tropismes(f), 3);
        let octets = active.ecrire_canonique();
        assert_eq!(octets.len(), t0.len() + 24, "force {f}, milieu vide : 24 octets de plus");
        assert_eq!(octets[..t0.len()], t0[..], "force {f} : le début est l'écriture de T0, au temps 0");
        let mut queue = vec![0u8; 16];
        queue.extend_from_slice(&f.to_bits().to_le_bytes());
        assert_eq!(octets[t0.len()..], queue[..], "force {f} : deux u64 à zéro puis les bits de la force");
    }
    // Avec un milieu non vide : à force nulle, la queue du milieu seule (T0) ; sinon, la même queue puis les 8 octets.
    let m =
        Milieu { sources: vec![source(2, 6, 1, 1.5), source(9, 2, 0, 0.1)], obstacles: vec![obstacle(8, 6, 11, 8)] };
    let avec_milieu = construire(r.clone(), m.clone(), Tropismes::default(), 3).ecrire_canonique();
    assert!(avec_milieu.ends_with(&queue_du_milieu(&m)), "garde : la queue du milieu est à la fin à force nulle");
    for f in [0.0, -0.0] {
        let nulle = construire(r.clone(), m.clone(), tropismes(f), 3);
        assert_eq!(nulle.ecrire_canonique(), avec_milieu, "force {f}, avec milieu : octets de T0");
    }
    for f in [0.5, 4.0, 45.0] {
        let octets = construire(r.clone(), m.clone(), tropismes(f), 3).ecrire_canonique();
        assert_eq!(octets.len(), avec_milieu.len() + 8, "force {f}, avec milieu : 8 octets de plus");
        assert_eq!(octets[..avec_milieu.len()], avec_milieu[..], "force {f}, avec milieu : le début est celui de T0");
        assert_eq!(octets[avec_milieu.len()..], f.to_bits().to_le_bytes()[..], "force {f} : les bits de la force");
    }
}

/// La longueur que l'écriture canonique doit avoir d'après son format : temps (4), nombre de segments (8), 40 octets
/// par segment, 25 par apex (`1 + ramifications`), trois compteurs (24), le substrat (`16 + 8 · cases`), puis la queue.
fn longueur_attendue(s: &Simulation, cases: usize, queue: usize) -> usize {
    4 + 8 + 40 * s.segments().len() + 25 * (1 + s.ramifications() as usize) + 24 + 16 + 8 * cases + queue
}

/// 9 (suite). À force non nulle, après des pas, l'écriture se termine par la queue du milieu puis les bits de la force
///    (vide ou non, 20, 40 puis 60 pas), et sa longueur est exactement celle que le format donne.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn a_force_non_nulle_la_queue_est_a_la_fin_a_tout_instant() {
    let (l, h) = (30, 20);
    let mut r = reglages(l, h, (14, 9));
    r.croissance.intervalle = 5.0;
    let riche = Milieu { sources: vec![source(24, 10, 3, 3.0)], obstacles: vec![obstacle(6, 3, 6, 14)] };
    for m in [Milieu::default(), riche] {
        for f in [0.7, 4.0] {
            let mut simulation = construire(r.clone(), m.clone(), tropismes(f), 5);
            let mut queue = queue_du_milieu(&m);
            queue.extend_from_slice(&f.to_bits().to_le_bytes());
            for pas in [0u32, 20, 20, 20] {
                simulation.avancer_n(pas);
                let octets = simulation.ecrire_canonique();
                assert!(
                    octets.ends_with(&queue),
                    "milieu {:?}, force {f}, temps {} : la queue n'est pas à la fin",
                    m.sources.len(),
                    simulation.temps()
                );
                assert_eq!(
                    octets.len(),
                    longueur_attendue(&simulation, l * h, queue.len()),
                    "milieu {:?}, force {f}, temps {} : longueur",
                    m.sources.len(),
                    simulation.temps()
                );
                assert_eq!(octets[..4], simulation.temps().to_le_bytes(), "le temps est écrit en tête");
            }
            // Garde contre un test vide : la colonie a poussé et s'est ramifiée.
            assert!(simulation.segments().len() >= 40 && simulation.ramifications() >= 1);
        }
    }
}

/// 9 (suite). Deux forces différentes, deux empreintes : au temps 0 (où seule la queue diffère, y compris pour deux forces
///    qui ne diffèrent que du dernier bit) et après 40 pas ; la force nulle diffère de toute force non nulle ; même
///    graine, même milieu, même force : même empreinte et même écriture (déterminisme), `avancer_n(40)` valant 40 appels
///    à `avancer` ; une autre graine, une autre empreinte.
#[test]
#[cfg_attr(not(feature = "etape-6"), ignore)]
fn deux_forces_deux_empreintes_et_une_force_donne_toujours_la_meme() {
    let r = reglages(30, 20, (14, 9));
    let m = Milieu { sources: vec![source(24, 10, 3, 3.0)], obstacles: vec![] };
    let forces = [0.0, 1.0, au_dessus(1.0), 2.0, 2.5, 4.0, 45.0];
    for milieu in [Milieu::default(), m] {
        for pas in [0u32, 40] {
            let mut empreintes: Vec<String> = Vec::new();
            for f in forces {
                let mut simulation = construire(r.clone(), milieu.clone(), tropismes(f), 4);
                simulation.avancer_n(pas);
                let empreinte = simulation.empreinte();
                assert!(est_hexadecimal_de_64(&empreinte), "empreinte : {empreinte}");
                assert_eq!(empreinte, sha256_hex(&simulation.ecrire_canonique()));
                assert!(!empreintes.contains(&empreinte), "force {f}, temps {pas} : même empreinte qu'une autre force");
                empreintes.push(empreinte);
            }
        }
        // Le déterminisme, à force non nulle.
        let mut premiere = construire(r.clone(), milieu.clone(), tropismes(2.5), 6);
        let mut seconde = construire(r.clone(), milieu.clone(), tropismes(2.5), 6);
        assert_eq!(premiere.ecrire_canonique(), seconde.ecrire_canonique(), "temps 0");
        premiere.avancer_n(40);
        seconde.avancer_n(40);
        assert_eq!(premiere.ecrire_canonique(), seconde.ecrire_canonique(), "après 40 pas");
        assert_eq!(premiere.empreinte(), seconde.empreinte());
        let mut pas_a_pas = construire(r.clone(), milieu.clone(), tropismes(2.5), 6);
        for _ in 0..40 {
            pas_a_pas.avancer();
        }
        assert_eq!(
            pas_a_pas.ecrire_canonique(),
            premiere.ecrire_canonique(),
            "avancer_n(40) contre 40 appels à avancer"
        );
        let mut autre_graine = construire(r.clone(), milieu.clone(), tropismes(2.5), 7);
        autre_graine.avancer_n(40);
        assert_ne!(autre_graine.empreinte(), premiere.empreinte(), "deux graines, deux empreintes");
    }
}
