//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « les empreintes » : M1 ne change rien au noyau.
//!
//! - **Les six empreintes de référence de la vitrine** (V0 = `couplage.txt`, le terrain, le chimiotropisme avec et sans, le mur
//!   avec et sans) et `references/couplage-graine-1.empreinte` sont celles du dépôt, en jouant les scénarios **après**
//!   `developper` (qui ne les touche pas) ;
//! - **un scénario sans ligne `moule`, développé, est le même scénario non développé** : le texte rendu est identique octet pour
//!   octet, et l'état simulé aussi (même écriture canonique, même empreinte) ;
//! - **un scénario de moule est un scénario ordinaire** : la simulation construite depuis le texte développé est, octet pour
//!   octet, celle que l'on construit avec le même milieu donné directement par `obstacles` (les réglages lus du texte, les obstacles de
//!   l'interface), et celle du texte que construit `outils.rs` ; elle est déterministe (même graine, même état) et dépend de la
//!   graine.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_10/outils.rs"]
mod outils;

use std::path::Path;

use mycelium_lab::moule::{developper, obstacles};
use mycelium_lab::simulation::{Milieu, Simulation, lire_scenario_tropismes};
use outils::{
    REFERENCES, construire, construire_moule, delai, developpe_de_reference, scenarios_de_moule, texte_du_fichier,
    texte_du_scenario,
};

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_six_empreintes_de_reference_de_la_vitrine_sont_inchangees() {
    delai();
    for (fichier, graine, pas, attendue) in REFERENCES {
        let texte = texte_du_fichier(fichier);
        let developpe = developper(&texte).expect("un scénario sans ligne moule se développe");
        assert_eq!(developpe, texte, "{fichier} : le texte développé est le texte");
        let (mut simulation, _, duree) = construire(&developpe, graine);
        assert_eq!(pas, duree, "{fichier} : le pas de référence est la durée du fichier");
        simulation.avancer_n(pas);
        assert_eq!(
            simulation.empreinte(),
            attendue,
            "{fichier}, graine {graine}, pas {pas} : l'empreinte de référence"
        );
        // Et l'état entier est celui du texte non développé.
        let (mut brute, _, _) = construire(&texte, graine);
        brute.avancer_n(pas);
        assert_eq!(
            simulation.ecrire_canonique(),
            brute.ecrire_canonique(),
            "{fichier} : le même état, octet pour octet"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_empreinte_de_couplage_graine_1_est_celle_du_fichier_de_reference() {
    delai();
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join("couplage-graine-1.empreinte");
    let reference = std::fs::read_to_string(&chemin).expect("le fichier de référence se lit");
    let texte = developper(&texte_du_fichier("couplage.txt")).unwrap();
    let (mut simulation, _, duree) = construire(&texte, 1);
    simulation.avancer_n(duree);
    assert_eq!(simulation.empreinte(), reference.trim(), "couplage.txt, graine 1, après developper");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_scenario_sans_ligne_moule_developpe_est_le_meme_que_non_developpe() {
    delai();
    // Les six scénarios de la vitrine, plus des scénarios à nous sans ligne `moule` (obstacles, sources, tropismes).
    let mut textes: Vec<String> = REFERENCES.iter().map(|(fichier, ..)| texte_du_fichier(fichier)).collect();
    textes.push(
        "boite 40 30\nspore 5 5\nsubstrat 1.0 0.2\ncroissance 1.0 8.0 35 20\ncouplage 0.1 0.002\nanastomose 0.5 8\nduree 60\n\
         source 30 20 4 5.0\nobstacle 15 0 17 20\nthigmotropisme 35\nchimiotropisme 10\n"
            .to_string(),
    );
    textes.push("# un commentaire\nboite 20 20\n\nspore 3 3\nsubstrat 1.0 0.2\ncroissance 1.0 5.0 40 25\ncouplage 0.1 0.002\nduree 30".to_string());
    for texte in &textes {
        let developpe = developper(texte).unwrap();
        assert_eq!(developpe.as_bytes(), texte.as_bytes());
        for graine in [1u64, 2] {
            let (mut a, _, duree) = construire(texte, graine);
            let (mut b, _, _) = construire(&developpe, graine);
            let duree = duree.min(50);
            a.avancer_n(duree);
            b.avancer_n(duree);
            assert_eq!(a.empreinte(), b.empreinte(), "graine {graine}");
            assert_eq!(a.ecrire_canonique(), b.ecrire_canonique(), "graine {graine}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_scenario_de_moule_est_un_scenario_ordinaire_pour_le_noyau() {
    delai();
    for s in scenarios_de_moule() {
        let texte = texte_du_scenario(s.nom);
        let developpe = developper(&texte).unwrap();
        for graine in [1u64, 7] {
            // 1. Depuis le texte développé par `developper`.
            let (mut par_le_texte, reglages, duree) = construire(&developpe, graine);
            // 2. Depuis le texte que construit `outils.rs` (formes et décomposition de référence).
            let (mut par_la_reference, _, _) = construire(&developpe_de_reference(&texte, &s.forme), graine);
            // 3. Depuis les réglages du texte et les obstacles de l'interface, sans passer par le texte `obstacle`.
            let (reglages_lus, _, tropismes, _) = lire_scenario_tropismes(&developpe).unwrap();
            assert_eq!(reglages_lus, reglages);
            let milieu = Milieu {
                sources: vec![],
                obstacles: obstacles(s.boite.0, s.boite.1, &s.forme).expect("la forme est valide"),
            };
            let mut par_l_interface =
                Simulation::nouvelle_avec_tropismes(reglages_lus, milieu, tropismes, graine).expect("réglages valides");
            let pas = duree.min(60);
            par_le_texte.avancer_n(pas);
            par_la_reference.avancer_n(pas);
            par_l_interface.avancer_n(pas);
            let attendu = par_le_texte.ecrire_canonique();
            assert_eq!(par_la_reference.ecrire_canonique(), attendu, "{}, graine {graine} : texte de référence", s.nom);
            assert_eq!(par_l_interface.ecrire_canonique(), attendu, "{}, graine {graine} : interface", s.nom);
            assert_eq!(par_le_texte.empreinte().len(), 64);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn un_moule_est_deterministe_et_depend_de_la_graine() {
    delai();
    for s in scenarios_de_moule() {
        let empreinte = |graine: u64| {
            let (mut simulation, _, duree) = construire_moule(s.nom, graine);
            simulation.avancer_n(duree.min(80));
            simulation.empreinte()
        };
        assert_eq!(empreinte(3), empreinte(3), "{} : la même graine, le même état", s.nom);
        assert_ne!(empreinte(3), empreinte(4), "{} : une autre graine, un autre état", s.nom);
    }
}
