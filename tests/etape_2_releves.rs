//! Tests d'acceptation de l'étape 2 : les relevés et leur écriture en CSV.

use mycelium_lab::releves::{ENTETE_CSV, Releve, en_csv};

#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn deux_releves_donnent_le_texte_attendu() {
    let releves = [
        Releve { temps: 0, biomasse: 1.5, pointes: 2, ramifications: 3, anastomoses: 4, rayon: 2.25, substrat: 0.75 },
        Releve { temps: 10, biomasse: 3.0, pointes: 5, ramifications: 6, anastomoses: 7, rayon: 0.5, substrat: 0.125 },
    ];
    let texte = en_csv(&releves);
    let attendu = concat!(
        "temps,biomasse,pointes,ramifications,anastomoses,rayon,substrat\n",
        "0,1.5,2,3,4,2.25,0.75\n",
        "10,3,5,6,7,0.5,0.125\n",
    );
    let obtenus: Vec<char> = texte.chars().collect();
    let attendus: Vec<char> = attendu.chars().collect();
    assert_eq!(obtenus.len(), attendus.len(), "longueur : {} au lieu de {}", obtenus.len(), attendus.len());
    for (rang, (obtenu, attendu)) in obtenus.iter().zip(&attendus).enumerate() {
        assert_eq!(obtenu, attendu, "caractère {rang}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn une_liste_vide_donne_l_entete_et_son_saut_de_ligne() {
    assert_eq!(en_csv(&[]), format!("{ENTETE_CSV}\n"));
}

#[test]
#[cfg_attr(not(feature = "etape-2"), ignore)]
fn chaque_nombre_du_texte_se_relit_sans_perte() {
    let releves = [
        Releve { temps: 0, biomasse: 0.0, pointes: 0, ramifications: 0, anastomoses: 0, rayon: 0.0, substrat: 0.0 },
        Releve {
            temps: 3,
            biomasse: 0.1,
            pointes: 7,
            ramifications: 9,
            anastomoses: 11,
            rayon: 1.0 / 3.0,
            substrat: 123.456,
        },
        Releve {
            temps: 4_000_000_000,
            biomasse: 1e-7,
            pointes: 1,
            ramifications: u64::MAX,
            anastomoses: 42,
            rayon: 1e20,
            substrat: 2.0 / 3.0,
        },
    ];

    let texte = en_csv(&releves);
    let mut lignes = texte.lines();
    assert_eq!(lignes.next(), Some(ENTETE_CSV), "en-tête");
    for releve in &releves {
        let ligne = lignes.next().expect("une ligne par relevé");
        let champs: Vec<&str> = ligne.split(',').collect();
        assert_eq!(champs.len(), 7, "sept champs : {ligne}");
        assert_eq!(champs[0].parse::<u64>().unwrap(), u64::from(releve.temps), "temps");
        assert_eq!(champs[1].parse::<f64>().unwrap(), releve.biomasse, "biomasse");
        assert_eq!(champs[2].parse::<u64>().unwrap(), u64::try_from(releve.pointes).unwrap(), "pointes");
        assert_eq!(champs[3].parse::<u64>().unwrap(), releve.ramifications, "ramifications");
        assert_eq!(champs[4].parse::<u64>().unwrap(), releve.anastomoses, "anastomoses");
        assert_eq!(champs[5].parse::<f64>().unwrap(), releve.rayon, "rayon");
        assert_eq!(champs[6].parse::<f64>().unwrap(), releve.substrat, "substrat");
    }
    assert_eq!(lignes.next(), None, "rien après le dernier relevé");
}
