//! Tests d'acceptation de l'étape 16 (a) : l'état des pointes que le noyau montre, `Simulation::etat_des_pointes`.
//!
//! L'accesseur rend, pour **tous** les apex dans l'ordre de leur identifiant, `(id, x, y, actif)` : un apex anastomosé ou
//! arrêté par un obstacle reste dans la liste, inactif. Il ne fait que lire. Ces tests le comparent à ce que le noyau dit
//! déjà par d'autres chemins (`pointes`, `ramifications`, `anastomoses`, `segments`, l'écriture canonique), sur les six
//! scénarios de la page, cinq graines et chaque pas de 0 à la durée du scénario plus 40.
//!
//! La règle de position (test 2) : un apex ne bouge qu'en laissant un segment, qui part de là où il était et au bout
//! duquel il arrive. Sa position est donc le bout `(x1, y1)` de son dernier segment ou, s'il n'en a laissé aucun, le point
//! où il est né : la spore pour l'apex 0 ; pour un autre, le bout du segment que sa mère vient de laisser (ramification),
//! ou la position de sa mère si elle vient de s'arrêter contre un obstacle (naissance de contact).
//!
//! Les scénarios sont lus sur le disque. Ils n'utilisent, parmi les fonctions de `f64`, que `to_bits`.

use mycelium_lab::simulation::{EtatPointe, Simulation, lire_scenario_tropismes};

/// Un des six scénarios de la page : son nom, son fichier sous `scenarios/` et son pas de référence (`duree`).
struct Scenario {
    nom: &'static str,
    fichier: &'static str,
    duree: u32,
}

/// Les six scénarios de la page, tels que les fixent les étapes 4 et 8 (cette table ne vient pas du code).
const SCENARIOS: [Scenario; 6] = [
    Scenario { nom: "libre", fichier: "couplage.txt", duree: 80 },
    Scenario { nom: "terrain", fichier: "terrain.txt", duree: 80 },
    Scenario { nom: "tache", fichier: "chimiotropisme.txt", duree: 100 },
    Scenario { nom: "tache-sans", fichier: "chimiotropisme-sans.txt", duree: 100 },
    Scenario { nom: "mur", fichier: "mur.txt", duree: 120 },
    Scenario { nom: "mur-sans", fichier: "mur-sans.txt", duree: 120 },
];

/// Les graines : la plus simple, deux autres, une qui ne tient ni sur 16 bits ni sur 31, et la plus grande.
const GRAINES: [u32; 5] = [1, 7, 31, 0xDEAD_BEEF, u32::MAX];

/// Combien de pas on joue au-delà du pas de référence : la page ne s'arrête pas à `duree`.
const PAS_AU_DELA: u32 = 40;

/// La simulation de ce scénario (lu sur le disque) avec cette graine, au temps 0.
fn simulation(scenario: &Scenario, graine: u32) -> Simulation {
    let chemin = format!("{}/scenarios/{}", env!("CARGO_MANIFEST_DIR"), scenario.fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|erreur| panic!("{chemin} se lit : {erreur}"));
    let (reglages, milieu, tropismes, _duree) =
        lire_scenario_tropismes(&texte).expect("le scénario du disque est valide");
    Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, u64::from(graine))
        .expect("les réglages sont valides")
}

/// Les états de tous les apex, rassemblés. La lecture n'exige que `&Simulation` : ce qui ne compilerait plus si
/// l'accesseur prenait `&mut self`.
fn etats(simulation: &Simulation) -> Vec<EtatPointe> {
    simulation.etat_des_pointes().collect()
}

/// Les bits d'une position : de quoi comparer sans passer par `==` sur des flottants.
fn bits(x: f64, y: f64) -> (u64, u64) {
    (x.to_bits(), y.to_bits())
}

/// 1. Au temps 0 il y a une seule pointe, vivante, d'identifiant 0, sur la spore (le centre de sa case).
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn au_depart_il_y_a_une_pointe_vivante_sur_la_spore() {
    for scenario in &SCENARIOS {
        let simulation = simulation(scenario, 1);
        let (sx, sy) = simulation.reglages().spore;
        let attendu = EtatPointe { id: 0, x: sx as f64 + 0.5, y: sy as f64 + 0.5, actif: true };
        assert_eq!(etats(&simulation), vec![attendu], "{}", scenario.nom);
    }
}

/// 2. À chaque pas, l'état d'après comparé à celui d'avant, apex par apex (dans l'ordre des identifiants) :
///    - il y a `1 + ramifications()` états, d'identifiants 0, 1, 2… sans trou, et les vivants sont les `pointes()` ;
///    - un apex vivant laisse au plus un segment, de ce pas ; s'il en laisse un, le segment part de sa position d'avant et
///      l'apex est à son bout, vivant ou anastomosé (`anastomoses()` en compte autant) ; s'il n'en laisse pas, il n'a pas
///      bougé : bloqué s'il reste vivant, arrêté contre un obstacle s'il ne l'est plus ;
///    - un apex inactif ne laisse rien, ne bouge pas et ne revit pas ;
///    - les apex nés pendant le pas sont vivants et chacun est sur sa mère, les mères prises dans l'ordre des
///      identifiants : un au plus au bout du segment d'une mère restée vivante, deux au plus sur une mère qui vient de
///      s'arrêter, aucun sur une autre.
///
///    Le test compte les six cas qu'il a vus, et chacun doit s'être présenté : sans quoi il ne prouverait pas sa part de
///    la règle.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn un_apex_ne_bouge_qu_avec_son_segment_et_nait_sur_sa_mere() {
    // Vus : avancés, anastomosés, bloqués, arrêtés, nés d'une ramification, nés d'un contact.
    let mut vus = [0usize; 6];
    for scenario in &SCENARIOS {
        for graine in GRAINES {
            let mut simulation = simulation(scenario, graine);
            for pas in 1..=scenario.duree + PAS_AU_DELA {
                let contexte = format!("{} graine {graine} pas {pas}", scenario.nom);
                let avant = etats(&simulation);
                let (deja_laisses, anastomoses) = (simulation.segments().len(), simulation.anastomoses());
                simulation.avancer();
                let apres = etats(&simulation);
                let laisses = &simulation.segments()[deja_laisses..];
                assert_eq!(apres.len() as u64, 1 + simulation.ramifications(), "{contexte} : le nombre d'apex");
                assert!(apres.iter().enumerate().all(|(rang, etat)| etat.id as usize == rang), "{contexte} : les id");
                let vivants = apres.iter().filter(|etat| etat.actif).count();
                assert_eq!(vivants, simulation.pointes(), "{contexte} : les pointes vivantes");

                // Les mères possibles, dans l'ordre : où un apex peut naître sur elle, combien, et si c'est un contact.
                let mut meres: Vec<((u64, u64), usize, bool)> = Vec::new();
                let (mut lus, mut anastomosees) = (0, 0);
                for (a, b) in avant.iter().zip(&apres) {
                    let (ici, la) = (bits(a.x, a.y), bits(b.x, b.y));
                    // Les segments du pas sont dans l'ordre des apex, un au plus par apex.
                    if let Some(segment) = laisses.get(lus).filter(|segment| segment.apex == a.id) {
                        assert!(a.actif && segment.temps == pas, "{contexte} : le segment de l'apex {}", a.id);
                        assert_eq!(bits(segment.x0, segment.y0), ici, "{contexte} : l'apex {} part d'ailleurs", a.id);
                        assert_eq!(la, bits(segment.x1, segment.y1), "{contexte} : l'apex {} hors de son bout", a.id);
                        lus += 1;
                        anastomosees += usize::from(!b.actif);
                        vus[usize::from(!b.actif)] += 1;
                        meres.push((la, usize::from(b.actif), false));
                    } else {
                        assert_eq!(la, ici, "{contexte} : l'apex {} a bougé sans segment", a.id);
                        assert!(a.actif || !b.actif, "{contexte} : l'apex {} revit", a.id);
                        let arrete = a.actif && !b.actif;
                        vus[2 + usize::from(arrete)] += usize::from(a.actif);
                        meres.push((la, if arrete { 2 } else { 0 }, arrete));
                    }
                }
                assert_eq!(lus, laisses.len(), "{contexte} : un segment hors d'ordre, en double ou d'un nouveau-né");
                assert_eq!(simulation.anastomoses() - anastomoses, anastomosees as u64, "{contexte} : anastomoses");

                let mut mere = 0;
                for nouveau in &apres[avant.len()..] {
                    let position = bits(nouveau.x, nouveau.y);
                    while mere < meres.len() && (meres[mere].1 == 0 || meres[mere].0 != position) {
                        mere += 1;
                    }
                    assert!(nouveau.actif && mere < meres.len(), "{contexte} : l'apex {} sans mère", nouveau.id);
                    meres[mere].1 -= 1;
                    vus[4 + usize::from(meres[mere].2)] += 1;
                }
            }
        }
    }
    assert!(vus.iter().all(|&n| n > 0), "avancés, anastomosés, bloqués, arrêtés, ramifiés, contacts : {vus:?}");
}

/// 3. Lire ne change rien : à chaque pas, l'écriture canonique de la simulation lue est la même avant et après la lecture
///    (de tous les états, deux fois), et égale à celle d'une simulation jumelle, même scénario et même graine, **jamais
///    lue**. À la fin, les empreintes sont égales aussi : si la lecture avançait l'aléa, la jumelle s'en écarterait.
#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn lire_les_pointes_ne_change_ni_l_ecriture_canonique_ni_l_empreinte() {
    for scenario in &SCENARIOS {
        for graine in [1, 0xDEAD_BEEF] {
            let mut lue = simulation(scenario, graine);
            let mut jumelle = simulation(scenario, graine);
            for pas in 0..=scenario.duree + PAS_AU_DELA {
                let contexte = format!("{} graine {graine} pas {pas}", scenario.nom);
                let avant = lue.ecrire_canonique();
                let premiere = etats(&lue);
                assert_eq!(etats(&lue), premiere, "{contexte} : deux lectures de suite");
                assert_eq!(lue.ecrire_canonique(), avant, "{contexte} : la lecture a changé l'écriture canonique");
                assert_eq!(jumelle.ecrire_canonique(), avant, "{contexte} : la jumelle jamais lue diffère");
                lue.avancer();
                jumelle.avancer();
            }
            assert_eq!(lue.empreinte(), jumelle.empreinte(), "{} graine {graine} : l'empreinte finale", scenario.nom);
        }
    }
}
