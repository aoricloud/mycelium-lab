//! Tests d'acceptation de l'étape 10 (M1, le moule), partie « le texte d'un scénario » (`developper`).
//!
//! Ils fixent le préprocesseur `moule::developper` :
//!
//! - **le texte rendu, exact**, sur des cas écrits à la main : la ligne `moule` remplacée à sa place par ses lignes
//!   `obstacle`, les commentaires, les lignes vides et les lignes en retrait conservés tels quels, une ligne `moule` en
//!   commentaire non développée, la dernière ligne sans saut de ligne conservée (ou complétée quand c'est la ligne `moule`), la
//!   cavité qui est toute la boîte (aucune ligne), les fins de ligne `\r\n` ;
//! - **sans ligne `moule`, le texte rendu est identique octet pour octet** : les six scénarios de la vitrine et des textes
//!   étranges (vide, sans saut de ligne, retours chariot, mots qui commencent par `moule`) ;
//! - **chacune des erreurs** : `BoiteAbsente`, `MoulesEnDouble`, `LigneInvalide(numéro)` (forme inconnue, nombre de mots faux
//!   pour chacune des cinq formes, mot qui n'est pas un entier décimal non signé : signe, point, lettre, hexadécimal, exposant,
//!   chiffre non latin, débordement de `usize` et de `u64`), et celles de la forme (`FormeInvalide`, `HorsDeLaBoite`), avec le
//!   numéro de ligne juste derrière des commentaires et des lignes vides ;
//! - **les cinq scénarios de `scenarios/moules/`** : le texte développé est exactement celui que construit `outils.rs`
//!   (formes et décomposition de référence), `lire_scenario_tropismes` l'accepte et en tire les mêmes obstacles ;
//! - **des scénarios au hasard** : le texte développé de formes au hasard est celui de `outils.rs`.
//!
//! Ils n'utilisent aucune fonction de `f64`.

#[path = "etape_10/outils.rs"]
mod outils;

use mycelium_lab::moule::{ErreurMoule, Forme, developper, obstacles};
use mycelium_lab::simulation::lire_scenario_tropismes;
use outils::{Hasard, delai, developpe_de_reference, scenarios_de_moule, texte_du_fichier, texte_du_scenario};

/// Le texte développé, attendu sans erreur.
fn dev(texte: &str) -> String {
    developper(texte).unwrap_or_else(|erreur| panic!("{erreur:?} pour {texte:?}"))
}

/// L'erreur de `developper`, attendue.
fn erreur(texte: &str) -> ErreurMoule {
    match developper(texte) {
        Ok(rendu) => panic!("une erreur était attendue pour {texte:?}, le texte rendu est {rendu:?}"),
        Err(erreur) => erreur,
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Le texte rendu, à la main
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_ligne_moule_devient_ses_lignes_obstacle_a_sa_place() {
    delai();
    assert_eq!(
        dev("boite 5 4\nspore 2 1\nsubstrat 1.0 0.2\nmoule brique 1 1 3 2\nduree 5\n"),
        "boite 5 4\nspore 2 1\nsubstrat 1.0 0.2\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\nduree 5\n"
    );
    // Chacune des cinq formes, dans une boîte de 5 × 5 ou plus.
    assert_eq!(
        dev("boite 5 5\nmoule disque 2 2 1\n"),
        "boite 5 5\nobstacle 0 0 4 0\nobstacle 0 1 1 1\nobstacle 3 1 4 1\nobstacle 0 2 0 2\nobstacle 4 2 4 2\n\
         obstacle 0 3 1 3\nobstacle 3 3 4 3\nobstacle 0 4 4 4\n"
    );
    assert_eq!(dev("boite 4 4\nmoule l 1 0 3 3 1\n"), "boite 4 4\nobstacle 0 0 0 3\nobstacle 2 0 3 2\n");
    assert_eq!(
        dev("boite 5 5\nmoule anneau 2 2 2 1\n"),
        "boite 5 5\nobstacle 0 0 1 0\nobstacle 3 0 4 0\nobstacle 0 1 0 1\nobstacle 2 1 2 1\nobstacle 4 1 4 1\n\
         obstacle 1 2 3 2\nobstacle 0 3 0 3\nobstacle 2 3 2 3\nobstacle 4 3 4 3\nobstacle 0 4 1 4\nobstacle 3 4 4 4\n"
    );
    assert_eq!(
        dev("boite 3 3\nmoule labyrinthe 1 1 1 99\n"),
        "boite 3 3\nobstacle 0 0 2 0\nobstacle 0 1 0 1\nobstacle 2 1 2 1\nobstacle 0 2 2 2\n"
    );
    // Les lignes du texte qui viennent après la ligne `moule` ne bougent pas, ni celles d'avant, ni leur ordre.
    assert_eq!(
        dev("duree 7\nboite 3 3\nspore 1 1\nmoule labyrinthe 1 1 1 0\nsubstrat 1.0 0.2\nobstacle 9 9 9 9\n"),
        "duree 7\nboite 3 3\nspore 1 1\nobstacle 0 0 2 0\nobstacle 0 1 0 1\nobstacle 2 1 2 1\nobstacle 0 2 2 2\n\
         substrat 1.0 0.2\nobstacle 9 9 9 9\n"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_commentaires_les_lignes_vides_et_les_retraits_sont_conserves() {
    delai();
    let texte = "# un commentaire\n\nboite 5 4\n  # un autre, en retrait\n\n   \n# moule disque 9 9 9\nmoule brique 1 1 3 2\n\n\
                 # fin\nduree 5\n";
    assert_eq!(
        dev(texte),
        "# un commentaire\n\nboite 5 4\n  # un autre, en retrait\n\n   \n# moule disque 9 9 9\nobstacle 0 0 4 0\n\
         obstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\n\n# fin\nduree 5\n"
    );
    // Une ligne `moule` en commentaire ne compte ni comme ligne `moule`, ni comme doublon, ni comme erreur.
    assert_eq!(dev("boite 5 4\n#moule brique 1 1 3 2\n"), "boite 5 4\n#moule brique 1 1 3 2\n");
    assert_eq!(
        dev("# boite 99 99\nboite 3 3\nmoule labyrinthe 1 1 1 0\n# moule disque 1 1 1\n"),
        "# boite 99 99\nboite 3 3\nobstacle 0 0 2 0\nobstacle 0 1 0 1\nobstacle 2 1 2 1\nobstacle 0 2 2 2\n\
         # moule disque 1 1 1\n"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_ligne_moule_peut_etre_en_retrait_ou_avoir_des_blancs_en_trop() {
    delai();
    // Les lignes se lisent comme `lire_scenario` les lit : rognées, mots séparés par des blancs.
    let attendu = "boite 5 4\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\n";
    assert_eq!(dev("boite 5 4\n   moule brique 1 1 3 2\n"), attendu);
    assert_eq!(dev("boite 5 4\nmoule brique 1 1 3 2   \n"), attendu);
    assert_eq!(dev("boite 5 4\nmoule\tbrique\t1 1  3 2\n"), attendu);
    assert_eq!(dev("boite 5 4\nmoule brique 1 1 3 2\r\n"), attendu);
    // Une ligne `boite` en retrait compte aussi.
    assert_eq!(
        dev("  boite 5 4  \nmoule brique 1 1 3 2\n"),
        format!("  boite 5 4  \n{}", &attendu["boite 5 4\n".len()..])
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn la_derniere_ligne_sans_saut_de_ligne_est_conservee() {
    delai();
    // La dernière ligne n'est pas la ligne `moule` : elle reste sans saut de ligne.
    assert_eq!(
        dev("boite 5 4\nmoule brique 1 1 3 2\nduree 5"),
        "boite 5 4\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\nduree 5"
    );
    assert_eq!(
        dev("boite 5 4\nmoule brique 1 1 3 2\n# fin"),
        "boite 5 4\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\n# fin"
    );
    // La dernière ligne est la ligne `moule`, sans saut de ligne : chaque ligne `obstacle` se termine par un saut de ligne.
    assert_eq!(
        dev("boite 5 4\nmoule brique 1 1 3 2"),
        "boite 5 4\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\n"
    );
    assert_eq!(dev("boite 3 1\nmoule brique 1 0 1 0"), "boite 3 1\nobstacle 0 0 0 0\nobstacle 2 0 2 0\n");
    // La cavité est toute la boîte : la ligne `moule` disparaît, sans rien laisser.
    assert_eq!(dev("boite 3 3\nmoule brique 0 0 2 2"), "boite 3 3\n");
    assert_eq!(dev("boite 3 3\nmoule brique 0 0 2 2\nduree 1\n"), "boite 3 3\nduree 1\n");
    // Le texte d'une seule ligne `boite` sans saut de ligne reste tel quel.
    assert_eq!(dev("boite 3 3"), "boite 3 3");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_fins_de_ligne_crlf_des_autres_lignes_sont_conservees() {
    delai();
    assert_eq!(
        dev("boite 5 4\r\nspore 2 1\r\nmoule brique 1 1 3 2\r\nduree 5\r\n"),
        "boite 5 4\r\nspore 2 1\r\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\nduree 5\r\n"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn deux_lignes_boite_la_derniere_avant_le_moule_l_emporte() {
    delai();
    assert_eq!(dev("boite 3 3\nboite 6 6\nmoule brique 0 0 5 5\n"), "boite 3 3\nboite 6 6\n");
    assert_eq!(erreur("boite 6 6\nboite 3 3\nmoule brique 0 0 5 5\n"), ErreurMoule::HorsDeLaBoite);
    // Une ligne `boite` après la ligne `moule` ne compte pas : la boîte doit venir avant.
    assert_eq!(erreur("moule brique 0 0 1 1\nboite 5 5\n"), ErreurMoule::BoiteAbsente);
}

// ---------------------------------------------------------------------------------------------------------------------
// Sans ligne moule : octet pour octet
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn sans_ligne_moule_le_texte_est_rendu_octet_pour_octet() {
    delai();
    for (fichier, _, _, _) in outils::REFERENCES {
        let texte = texte_du_fichier(fichier);
        assert_eq!(dev(&texte).as_bytes(), texte.as_bytes(), "{fichier}");
    }
    for texte in [
        "",
        "\n",
        "\n\n\n",
        "boite 3 3",
        "boite 3 3\n\n\n",
        "  boite  3   3  \r\nspore 1 1\r\n",
        "# moule disque 1 1 1\n",
        "#moule\n",
        "moules brique 0 0 1 1\n",
        "Moule disque 1 1 1\n",
        "moulebrique\n",
        "obstacle 1 2 3 4\nobstacle 1 2 3 4",
        "\t\n \r\n\r\n",
        "boîte 3 3\nspore é à ü\n",
        "n'importe quoi\nsans rapport\n",
        "boite 3 3\rspore 1 1\r",
    ] {
        assert_eq!(dev(texte).as_bytes(), texte.as_bytes(), "{texte:?}");
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Les erreurs
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_ligne_moule_sans_boite_avant_elle_est_boite_absente() {
    delai();
    let absente = ErreurMoule::BoiteAbsente;
    assert_eq!(erreur("moule brique 0 0 1 1\n"), absente);
    assert_eq!(erreur("moule brique 0 0 1 1"), absente);
    assert_eq!(erreur("spore 0 0\nmoule disque 1 1 1\n"), absente);
    assert_eq!(erreur("moule brique 0 0 1 1\nboite 5 5\n"), absente, "la boîte vient après");
    // Une ligne `boite` en commentaire ne compte pas.
    assert_eq!(erreur("# boite 5 5\nmoule brique 0 0 1 1\n"), absente);
    assert_eq!(erreur("\n\n# boite 5 5\n  \nmoule labyrinthe 1 1 1 5\n"), absente);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn deux_lignes_moule_sont_moules_en_double() {
    delai();
    let double = ErreurMoule::MoulesEnDouble;
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule brique 0 0 1 1\n"), double);
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule disque 2 2 1\n"), double);
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nspore 2 2\n# c\n\nmoule disque 2 2 1"), double);
    assert_eq!(
        erreur("boite 5 5\nmoule brique 0 0 4 4\nmoule brique 0 0 4 4\n"),
        double,
        "même quand le premier ne laisse rien"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_ligne_moule_mal_formee_est_ligne_invalide_avec_son_numero() {
    delai();
    let boite = "boite 30 30\n";
    // Forme inconnue, ligne `moule` sans rien, sans forme.
    for ligne in [
        "moule",
        "moule cube 1 2 3",
        "moule Brique 0 0 1 1",
        "moule L 0 0 3 3 1",
        "moule 0 0 1 1",
        "moule brique0 0 1 1 1",
    ] {
        assert_eq!(erreur(&format!("{boite}{ligne}\n")), ErreurMoule::LigneInvalide(2), "{ligne}");
    }
    // Un nombre de mots faux, pour chacune des cinq formes : un de moins, un de plus, aucun.
    let valides = [
        "moule brique 1 1 3 2",
        "moule l 1 1 5 5 2",
        "moule disque 10 10 3",
        "moule anneau 10 10 5 2",
        "moule labyrinthe 2 2 2 7",
    ];
    for valide in valides {
        let mots: Vec<&str> = valide.split(' ').collect();
        let sans_dernier = mots[..mots.len() - 1].join(" ");
        let avec_un_de_plus = format!("{valide} 1");
        let sans_rien = mots[..2].join(" ");
        for ligne in [sans_dernier, avec_un_de_plus, sans_rien, mots[..1].join(" ")] {
            assert_eq!(erreur(&format!("{boite}{ligne}\n")), ErreurMoule::LigneInvalide(2), "{ligne}");
        }
        assert!(developper(&format!("{boite}{valide}\n")).is_ok(), "la ligne valide : {valide}");
    }
    // Un mot qui n'est pas un entier décimal non signé, à chaque place d'une brique.
    for mauvais in ["-1", "+1", "1.5", "1,5", "abc", "0x10", "1e1", "٣", "1_000", "--1", "-0", "+0", "¹"] {
        for place in 0..4 {
            let mut mots = ["1", "1", "3", "2"].map(String::from);
            mots[place] = mauvais.to_string();
            let ligne = format!("moule brique {}", mots.join(" "));
            assert_eq!(erreur(&format!("{boite}{ligne}\n")), ErreurMoule::LigneInvalide(2), "{ligne}");
        }
    }
    // Chaque nombre de chaque forme est lu comme un entier : le mauvais mot à chaque place.
    for valide in valides {
        let mots: Vec<&str> = valide.split(' ').collect();
        for place in 2..mots.len() {
            let mut copie = mots.clone();
            copie[place] = "-1";
            let ligne = copie.join(" ");
            assert_eq!(erreur(&format!("{boite}{ligne}\n")), ErreurMoule::LigneInvalide(2), "{ligne}");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn le_numero_de_ligne_compte_toutes_les_lignes_a_partir_de_un() {
    delai();
    assert_eq!(erreur("boite 5 5\nmoule cube\n"), ErreurMoule::LigneInvalide(2));
    assert_eq!(erreur("# a\n\nboite 5 5\n   \n# b\nmoule cube\n"), ErreurMoule::LigneInvalide(6));
    assert_eq!(erreur("boite 5 5\r\nspore 1 1\r\n\r\nmoule brique 1\r\nduree 1\r\n"), ErreurMoule::LigneInvalide(4));
    assert_eq!(
        erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule cube\n"),
        ErreurMoule::LigneInvalide(3),
        "la seconde ligne est inconnue"
    );
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1\nduree 1\n"), ErreurMoule::LigneInvalide(2));
    // Sans saut de ligne à la fin.
    assert_eq!(erreur("boite 5 5\nmoule cube"), ErreurMoule::LigneInvalide(2));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_entiers_trop_grands_sont_des_lignes_invalides_et_la_graine_est_un_u64() {
    delai();
    // La graine d'un labyrinthe tient dans un u64 : de 0 à 2⁶⁴ − 1.
    assert!(developper("boite 3 3\nmoule labyrinthe 1 1 1 18446744073709551615\n").is_ok());
    assert!(developper("boite 3 3\nmoule labyrinthe 1 1 1 0\n").is_ok());
    assert!(developper("boite 3 3\nmoule labyrinthe 1 1 1 0000000000000000000000007\n").is_ok(), "des zéros en tête");
    assert_eq!(
        erreur("boite 3 3\nmoule labyrinthe 1 1 1 18446744073709551616\n"),
        ErreurMoule::LigneInvalide(2),
        "2⁶⁴ ne tient pas dans un u64"
    );
    assert_eq!(erreur("boite 3 3\nmoule labyrinthe 1 1 1 -1\n"), ErreurMoule::LigneInvalide(2));
    assert_eq!(erreur("boite 3 3\nmoule labyrinthe 1 1 1 1.0\n"), ErreurMoule::LigneInvalide(2));
    // Les autres nombres tiennent dans un usize.
    assert_eq!(
        erreur("boite 3 3\nmoule brique 0 0 99999999999999999999999999999 0\n"),
        ErreurMoule::LigneInvalide(2),
        "plus grand que tout usize"
    );
    assert_eq!(erreur("boite 3 3\nmoule disque 99999999999999999999999999999 0 0\n"), ErreurMoule::LigneInvalide(2));
    // Un usize énorme, mais lisible : la forme est hors de la boîte, ce n'est pas une ligne invalide.
    let max = usize::MAX;
    assert_eq!(erreur(&format!("boite 3 3\nmoule brique 0 0 {max} 0\n")), ErreurMoule::HorsDeLaBoite);
    // La boîte elle-même est lue : une boîte énorme ne fait ni paniquer ni allouer.
    assert_eq!(erreur(&format!("boite {max} {max}\nmoule brique 0 0 {max} 0\n")), ErreurMoule::HorsDeLaBoite);
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_erreurs_de_la_forme_traversent_developper() {
    delai();
    assert_eq!(erreur("boite 5 5\nmoule l 0 0 3 3 0\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule l 0 0 3 3 5\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule brique 3 0 2 0\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule anneau 2 2 2 2\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule labyrinthe 0 2 2 1\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule labyrinthe 2 2 0 1\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule disque 1 1 3\n"), ErreurMoule::HorsDeLaBoite);
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 5 0\n"), ErreurMoule::HorsDeLaBoite);
    assert_eq!(erreur("boite 4 5\nmoule labyrinthe 1 1 3 1\n"), ErreurMoule::HorsDeLaBoite, "W = 5 > 4");
    assert_eq!(erreur("boite 5 4\nmoule labyrinthe 1 1 3 1\n"), ErreurMoule::HorsDeLaBoite, "H = 5 > 4");
    assert_eq!(erreur("boite 0 5\nmoule brique 0 0 0 0\n"), ErreurMoule::HorsDeLaBoite, "boîte de largeur nulle");
    assert_eq!(erreur("boite 5 0\nmoule brique 0 0 0 0\n"), ErreurMoule::HorsDeLaBoite, "boîte de hauteur nulle");
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn l_ordre_des_erreurs_est_la_syntaxe_puis_le_doublon_puis_la_boite_puis_la_forme() {
    delai();
    // Une ligne `moule` est lue dans cet ordre, et la première ligne fautive l'emporte sur les suivantes.
    // 1. La syntaxe avant tout : sans boîte, une ligne mal formée est `LigneInvalide`, pas `BoiteAbsente`.
    assert_eq!(erreur("moule cube\n"), ErreurMoule::LigneInvalide(1));
    assert_eq!(erreur("moule brique 0 0 1\n"), ErreurMoule::LigneInvalide(1));
    assert_eq!(erreur("moule brique 0 0 +1 1\n"), ErreurMoule::LigneInvalide(1));
    // La syntaxe avant le doublon.
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule cube\n"), ErreurMoule::LigneInvalide(3));
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule brique 0 0 1\n"), ErreurMoule::LigneInvalide(3));
    // 2. Le doublon avant la boîte et avant la forme : la seconde ligne `moule`, bien formée, est `MoulesEnDouble` même si
    //    sa forme est hors contrat ou hors de la boîte.
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule brique 3 0 2 0\n"), ErreurMoule::MoulesEnDouble);
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 1 1\nmoule brique 0 0 9 9\n"), ErreurMoule::MoulesEnDouble);
    // 3. La boîte avant la forme : sans boîte, une forme hors contrat est `BoiteAbsente`.
    assert_eq!(erreur("moule brique 3 0 2 0\n"), ErreurMoule::BoiteAbsente);
    assert_eq!(erreur("moule l 0 0 3 3 0\n"), ErreurMoule::BoiteAbsente);
    assert_eq!(erreur("moule anneau 2 2 2 2\n"), ErreurMoule::BoiteAbsente);
    // 4. La forme en dernier : `FormeInvalide` avant `HorsDeLaBoite` (voir les formes).
    assert_eq!(erreur("boite 5 5\nmoule brique 40 40 30 30\n"), ErreurMoule::FormeInvalide);
    // La première ligne fautive l'emporte : une première ligne `moule` fautive est signalée, pas la seconde.
    assert_eq!(
        erreur("moule brique 0 0 1 1\nmoule brique 0 0 1 1\n"),
        ErreurMoule::BoiteAbsente,
        "deux moules sans boîte"
    );
    assert_eq!(erreur("moule brique 0 0 1 1\nboite 5 5\nmoule cube\n"), ErreurMoule::BoiteAbsente);
    assert_eq!(erreur("boite 5 5\nmoule brique 0 0 9 9\nmoule brique 0 0 1 1\n"), ErreurMoule::HorsDeLaBoite);
    assert_eq!(erreur("boite 5 5\nmoule brique 3 0 2 0\nmoule cube\n"), ErreurMoule::FormeInvalide);
    assert_eq!(erreur("boite 5 5\nmoule cube\nmoule brique 3 0 2 0\n"), ErreurMoule::LigneInvalide(2));
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn une_ligne_boite_mal_formee_ne_donne_pas_de_boite_et_reste_telle_quelle() {
    delai();
    // Seule une ligne `boite <largeur> <hauteur>` de deux entiers décimaux non signés donne la boîte ; une autre ligne `boite`
    // est laissée telle quelle (c'est `lire_scenario` qui la refusera) et ne compte pas.
    let moule = "moule brique 0 0 1 1\n";
    for boite in [
        "boite 5",
        "boite 5 5 5",
        "boite a b",
        "boite 5 b",
        "boite -5 5",
        "boite +5 5",
        "boite 5.0 5",
        "boite 5,0 5",
        "boite 0x5 5",
        "boite",
        "Boite 5 5",
        "BOITE 5 5",
        "boîte 5 5",
        "boite5 5",
        "boite 5\t",
        "boite  ",
    ] {
        assert_eq!(erreur(&format!("{boite}\n{moule}")), ErreurMoule::BoiteAbsente, "{boite:?}");
        // Sans ligne `moule`, le texte est rendu tel quel, la ligne `boite` fautive comprise.
        let seule = format!("{boite}\nspore 1 1\n");
        assert_eq!(dev(&seule), seule, "{boite:?}");
    }
    // Une ligne `boite` mal formée entre une bonne et la ligne `moule` ne la remplace pas ; elle est recopiée.
    assert_eq!(
        dev("boite 5 4\nboite 9\nmoule brique 1 1 3 2\n"),
        "boite 5 4\nboite 9\nobstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\n"
    );
    assert_eq!(dev("boite 5 4\nboite 9 9 9\nmoule brique 0 0 4 3\n"), "boite 5 4\nboite 9 9 9\n");
    // Une boîte de dimension nulle est bien une boîte (lue), et toute forme y est hors de la boîte.
    assert_eq!(erreur("boite 0 0\nmoule brique 0 0 0 0\n"), ErreurMoule::HorsDeLaBoite);
    // Les blancs : des espaces en trop, des tabulations, des zéros en tête, le retour chariot.
    let attendu = "obstacle 0 0 4 0\nobstacle 0 1 0 2\nobstacle 4 1 4 2\nobstacle 0 3 4 3\n";
    for boite in ["boite 5 4", "boite  5  4", "boite\t5\t4", "  boite 5 4  ", "boite 05 004", "boite 5 4\r"] {
        assert_eq!(dev(&format!("{boite}\nmoule brique 1 1 3 2\n")), format!("{boite}\n{attendu}"), "{boite:?}");
    }
    // Une ligne `moule` avec des blancs en trop, des zéros en tête ; mais pas un nombre coupé en deux ni collé à un mot.
    assert_eq!(dev("boite 5 4\nmoule  brique  01 1 3 02\n"), format!("boite 5 4\n{attendu}"));
    assert_eq!(erreur("boite 5 4\nmoule brique 1 1 3 2 \n\n moule disque 1 1 1\n"), ErreurMoule::MoulesEnDouble);
    assert_eq!(erreur("boite 5 4\nmoule brique 1 1 3 2 1\n"), ErreurMoule::LigneInvalide(2));
    assert_eq!(erreur("boite 5 4\nmoule brique 1 1 3\n2\n"), ErreurMoule::LigneInvalide(2));
}

// ---------------------------------------------------------------------------------------------------------------------
// Les scénarios de moules, des scénarios au hasard
// ---------------------------------------------------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_cinq_scenarios_se_developpent_comme_la_reference() {
    delai();
    for s in scenarios_de_moule() {
        let texte = texte_du_scenario(s.nom);
        let developpe = dev(&texte);
        assert_eq!(developpe, developpe_de_reference(&texte, &s.forme), "{} : le texte développé", s.nom);
        assert!(
            !developpe.lines().any(|l| l.split_whitespace().next() == Some("moule")),
            "{} : plus de ligne moule",
            s.nom
        );
        // `developper` est idempotent : un texte développé n'a plus de ligne `moule`.
        assert_eq!(dev(&developpe), developpe, "{} : développer deux fois", s.nom);
        // Le noyau lit le texte développé, et en tire les obstacles de `obstacles`.
        let (reglages, milieu, _, duree) =
            lire_scenario_tropismes(&developpe).expect("le noyau lit le texte développé");
        assert_eq!((reglages.largeur, reglages.hauteur), s.boite, "{}", s.nom);
        assert_eq!(reglages.spore, s.spore, "{}", s.nom);
        assert_eq!(duree, s.duree, "{}", s.nom);
        assert_eq!(milieu.obstacles, obstacles(s.boite.0, s.boite.1, &s.forme).unwrap(), "{}", s.nom);
        assert!(milieu.sources.is_empty(), "{}", s.nom);
        // Les lignes autres que `moule` sont celles du fichier, dans l'ordre.
        let autres: Vec<&str> = texte.lines().filter(|l| !l.trim_start().starts_with("moule ")).collect();
        let gardees: Vec<&str> = developpe.lines().filter(|l| !l.starts_with("obstacle ")).collect();
        assert_eq!(autres, gardees, "{} : les autres lignes sont conservées", s.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn les_scenarios_de_moules_ont_une_seule_ligne_moule_et_les_reglages_attendus() {
    delai();
    for s in scenarios_de_moule() {
        let texte = texte_du_scenario(s.nom);
        let moules: Vec<&str> = texte.lines().filter(|l| l.split_whitespace().next() == Some("moule")).collect();
        assert_eq!(moules.len(), 1, "{} : une seule ligne moule", s.nom);
        let mot = |forme: &Forme| match forme {
            Forme::Brique { .. } => "brique",
            Forme::L { .. } => "l",
            Forme::Disque { .. } => "disque",
            Forme::Anneau { .. } => "anneau",
            Forme::Labyrinthe { .. } => "labyrinthe",
        };
        assert_eq!(moules[0].split_whitespace().nth(1), Some(mot(&s.forme)), "{}", s.nom);
        // Les réglages des scénarios existants (la ligne `croissance` peut changer l'intervalle de ramification).
        for ligne in ["substrat 1.0 0.2", "couplage 0.1 0.002", "anastomose 0.5 10"] {
            assert!(texte.lines().any(|l| l == ligne), "{} a la ligne `{ligne}`", s.nom);
        }
        assert!(
            texte.lines().any(|l| l.starts_with("croissance 1.0 ") && l.ends_with(" 30 20")),
            "{} : croissance",
            s.nom
        );
        assert!(texte.lines().any(|l| l.starts_with('#')), "{} : les paramètres sont écrits en commentaire", s.nom);
        assert!(texte.lines().any(|l| l.starts_with("duree ")), "{} : une durée", s.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-10"), ignore)]
fn des_scenarios_au_hasard_se_developpent_comme_la_reference() {
    delai();
    let mut hasard = Hasard::new(31);
    for _ in 0..400 {
        let (largeur, hauteur) = (hasard.entre(6, 30), hasard.entre(6, 30));
        let (forme, ligne) = match hasard.jusqu_a(5) {
            0 => {
                let (x0, y0) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let (x1, y1) = (hasard.entre(x0, largeur - 1), hasard.entre(y0, hauteur - 1));
                (Forme::Brique { x0, y0, x1, y1 }, format!("moule brique {x0} {y0} {x1} {y1}"))
            }
            1 => {
                let (x0, y0) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let (x1, y1) = (hasard.entre(x0, largeur - 1), hasard.entre(y0, hauteur - 1));
                let epaisseur = hasard.entre(1, (x1 - x0 + 1).min(y1 - y0 + 1));
                (Forme::L { x0, y0, x1, y1, epaisseur }, format!("moule l {x0} {y0} {x1} {y1} {epaisseur}"))
            }
            2 => {
                let (cx, cy) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let rayon = hasard.entre(0, cx.min(cy).min(largeur - 1 - cx).min(hauteur - 1 - cy));
                (Forme::Disque { cx, cy, rayon }, format!("moule disque {cx} {cy} {rayon}"))
            }
            3 => {
                let (cx, cy) = (hasard.jusqu_a(largeur), hasard.jusqu_a(hauteur));
                let maximum = cx.min(cy).min(largeur - 1 - cx).min(hauteur - 1 - cy);
                if maximum == 0 {
                    continue;
                }
                let exterieur = hasard.entre(1, maximum);
                let interieur = hasard.entre(0, exterieur - 1);
                (
                    Forme::Anneau { cx, cy, rayon_exterieur: exterieur, rayon_interieur: interieur },
                    format!("moule anneau {cx} {cy} {exterieur} {interieur}"),
                )
            }
            _ => {
                let couloir = hasard.entre(1, 2);
                let pas = couloir + 1;
                let (colonnes_max, rangees_max) = ((largeur - 1) / pas, (hauteur - 1) / pas);
                let (colonnes, rangees) = (hasard.entre(1, colonnes_max), hasard.entre(1, rangees_max));
                let graine = hasard.suivant();
                (
                    Forme::Labyrinthe { colonnes, rangees, couloir, graine },
                    format!("moule labyrinthe {colonnes} {rangees} {couloir} {graine}"),
                )
            }
        };
        let fin = if hasard.avec_proba(1, 2) { "\n" } else { "" };
        let texte = format!("# hasard\nboite {largeur} {hauteur}\n\nspore 0 0\n{ligne}\n# fin\nduree 3{fin}");
        assert_eq!(dev(&texte), developpe_de_reference(&texte, &forme), "{texte:?}");
    }
}
