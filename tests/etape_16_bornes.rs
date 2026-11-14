//! Tests d'acceptation de l'étape 16 (d) : les trois lectures de la vitrine aux bornes que la page atteint.
//!
//! `etape_16_vitrine.rs` lit les colonies jusqu'à 1100 pas et jusqu'à l'apex 378. La page va plus loin : des pointes
//! vivantes d'identifiant supérieur à 1000, des segments nés après le pas 1023, des états jusqu'au pas 3000 (`PAS_MAX`).
//! Ici, sur les six scénarios de la page et deux graines, aux moments de `etape_16_vitrine.rs` plus les pas 200 et 3000
//! (et `terrain`, graine 6, au pas 1100), on vérifie à chaque moment :
//! - les trois lectures contre une `Simulation` construite sans passer par `vitrine.rs`, au bit près ;
//! - `NaN` au premier indice de trop et bien au-delà ;
//! - après ces lectures, l'empreinte et la durée de la vitrine inchangées, y compris quand plus aucune pointe ne vit ;
//! - l'accesseur `etat_des_pointes` contre le noyau lui-même, tard aussi : `1 + ramifications()` états, et chaque apex
//!   qui a laissé un segment est au bout de son dernier segment.
//!
//! Ce fichier n'a qu'un test : l'état global de la vitrine n'y est partagé avec aucun autre.

use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use mycelium_lab::vitrine::{
    mycelium_avancer, mycelium_choisir_scenario, mycelium_demarrer, mycelium_duree, mycelium_empreinte_mot,
    mycelium_nb_segments, mycelium_pointe, mycelium_pointes, mycelium_segment_apex, mycelium_segment_temps,
};

/// Les six scénarios de la page : numéro dans la vitrine, fichier, durée.
const SCENARIOS: [(u32, &str, u32); 6] = [
    (0, "couplage.txt", 80),
    (1, "terrain.txt", 80),
    (2, "chimiotropisme.txt", 100),
    (3, "chimiotropisme-sans.txt", 100),
    (4, "mur.txt", 120),
    (5, "mur-sans.txt", 120),
];

/// La simulation de référence, construite depuis le texte du scénario lu sur le disque.
fn native(fichier: &str, graine: u32) -> Simulation {
    let chemin = format!("{}/scenarios/{}", env!("CARGO_MANIFEST_DIR"), fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap();
    let (reseau, milieu, tropismes, _) = lire_scenario_tropismes(&texte).unwrap();
    Simulation::nouvelle_avec_tropismes(reseau, milieu, tropismes, u64::from(graine)).unwrap()
}

/// L'empreinte de la vitrine, mot par mot, dans l'écriture de `Simulation::empreinte`.
fn empreinte_de_la_vitrine() -> String {
    (0..8).map(|i| format!("{:08x}", mycelium_empreinte_mot(i))).collect()
}

/// Les trois lectures contre la simulation de référence, au bit près.
fn verifier_les_valeurs(native: &Simulation, cas: &str) {
    assert_eq!(mycelium_nb_segments() as usize, native.segments().len(), "{cas}");
    for (i, segment) in native.segments().iter().enumerate() {
        let i_u32 = i as u32;
        assert_eq!(mycelium_segment_temps(i_u32).to_bits(), f64::from(segment.temps).to_bits(), "{cas} : temps {i}");
        assert_eq!(mycelium_segment_apex(i_u32).to_bits(), f64::from(segment.apex).to_bits(), "{cas} : apex {i}");
    }
    let vivantes: Vec<_> = native.etat_des_pointes().filter(|pointe| pointe.actif).collect();
    assert_eq!(mycelium_pointes() as usize, vivantes.len(), "{cas}");
    for (j, pointe) in vivantes.iter().enumerate() {
        let j_u32 = j as u32;
        assert_eq!(mycelium_pointe(j_u32, 0).to_bits(), pointe.x.to_bits(), "{cas} : x {j}");
        assert_eq!(mycelium_pointe(j_u32, 1).to_bits(), pointe.y.to_bits(), "{cas} : y {j}");
    }
}

/// `NaN` au premier indice de trop, au suivant, et bien au-delà.
fn verifier_les_bords(cas: &str) {
    let (segments, pointes) = (mycelium_nb_segments(), mycelium_pointes());
    for hors in [segments, segments + 1, 1 << 31, u32::MAX] {
        assert!(mycelium_segment_temps(hors).is_nan(), "{cas} : temps {hors}");
        assert!(mycelium_segment_apex(hors).is_nan(), "{cas} : apex {hors}");
    }
    for hors in [pointes, pointes + 1, 1 << 31, u32::MAX] {
        for k in [0, 1] {
            assert!(mycelium_pointe(hors, k).is_nan(), "{cas} : pointe {hors}, {k}");
        }
    }
}

/// L'accesseur contre le noyau : le nombre d'états, et la position de chaque apex qui a laissé un segment.
fn verifier_l_accesseur(native: &Simulation, cas: &str) {
    let etats: Vec<_> = native.etat_des_pointes().collect();
    assert_eq!(etats.len() as u64, 1 + native.ramifications(), "{cas} : nombre d'états");
    let mut bout = vec![None; etats.len()];
    for segment in native.segments() {
        bout[segment.apex as usize] = Some((segment.x1.to_bits(), segment.y1.to_bits()));
    }
    for (etat, bout) in etats.iter().zip(&bout) {
        if let Some(bout) = bout {
            assert_eq!((etat.x.to_bits(), etat.y.to_bits()), *bout, "{cas} : apex {}", etat.id);
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-16"), ignore)]
fn les_trois_lectures_tiennent_aux_bornes_que_la_page_atteint() {
    let mut parcours: Vec<(usize, u32, Vec<u32>)> = Vec::new();
    for (k, &(_, _, duree)) in SCENARIOS.iter().enumerate() {
        for graine in [1, 0xDEAD_BEEF] {
            parcours.push((k, graine, vec![0, 1, 37, duree, duree + 40, 200, 1100, 3000]));
        }
    }
    parcours.push((1, 6, vec![1100]));

    let (mut plus_grand_id_vivant, mut plus_tardive_naissance) = (0u32, 0u32);
    for (k, graine, moments) in parcours {
        let (numero, fichier, duree) = SCENARIOS[k];
        assert_eq!(mycelium_choisir_scenario(numero), 1);
        mycelium_demarrer(graine);
        let mut native = native(fichier, graine);
        let mut fait = 0;
        for moment in moments {
            mycelium_avancer(moment - fait);
            native.avancer_n(moment - fait);
            fait = moment;
            let cas = format!("{fichier}, graine {graine}, pas {moment}");
            verifier_les_valeurs(&native, &cas);
            verifier_les_bords(&cas);
            assert_eq!(empreinte_de_la_vitrine(), native.empreinte(), "{cas} : une lecture a écrit");
            assert_eq!(mycelium_duree(), duree, "{cas} : une lecture a changé la durée");
            verifier_l_accesseur(&native, &cas);
            let vivants = native.etat_des_pointes().filter(|pointe| pointe.actif);
            plus_grand_id_vivant = vivants.map(|pointe| pointe.id).fold(plus_grand_id_vivant, u32::max);
            let naissances = native.segments().iter().map(|segment| segment.temps);
            plus_tardive_naissance = naissances.fold(plus_tardive_naissance, u32::max);
        }
    }
    // Le parcours atteint bien ce qu'il annonce : une pointe vivante d'identifiant au-delà de 1000, un segment né après
    // le pas 1023.
    assert!(plus_grand_id_vivant > 1000, "pointe vivante d'identifiant le plus grand : {plus_grand_id_vivant}");
    assert!(plus_tardive_naissance > 1023, "pas de naissance le plus tardif : {plus_tardive_naissance}");
}
