//! Tests d'acceptation de l'étape 7 (T2, les obstacles durs), partie anastomose : une anastomose ne se fait pas à travers un
//! obstacle.
//!
//! Le dispositif, entièrement déterministe (pas 1, ni marche au hasard ni ramification aléatoire, substrat constant sans
//! diffusion, ni affinité ni coût, anastomose de rayon 6 et de délai 10, `thigmotropisme 35`, une graine qui donne la
//! direction de départ 0°) : la pointe 0 part de (20,5 ; 50,5) vers l'est le long de `y = 50,5` et frappe de face le mur
//! `W1` (colonne 60) : elle s'arrête, et deux pointes naissent ; la pointe 1 (direction 90°, vers les `y` croissants) longe
//! `W1` jusqu'au plafond `W2` (ligne 56, colonnes 40 à 59), qu'elle frappe de face : elle s'arrête, et une seule pointe naît
//! (la direction 0° est contre `W1`) : la pointe 3, qui repart vers l'ouest le long de `y = 55,5`, à 5 cases au-dessus des
//! segments de la pointe 0. Entre les deux, un mur mince `H` (ligne 52, colonnes 30 à 58).
//!
//! Ils fixent : avec `H`, la pointe 3 ne s'anastomose pas tant qu'elle est au-dessus de `H` (24 pas), alors que les segments
//! de la pointe 0 sont à sa portée, et elle s'anastomose une fois `H` passé (le segment qui la joint au plus proche ne le
//! traverse plus) ; sans `H` (le témoin), elle s'anastomose dès son premier pas.
//!
//! Ils laissent ouverts : si un candidat séparé par un obstacle est écarté au profit d'un autre plus lointain ou si la
//! recherche s'arrête (la spécification écarte le candidat et continue, mais ce dispositif ne les distingue pas) ; le pas
//! exact de l'anastomose après `H`.
//!
//! Ils n'utilisent aucune fonction mathématique de `f64`.

#[path = "etape_7/outils.rs"]
mod outils;

use mycelium_lab::simulation::Simulation;
use outils::{construire, graine_pour};

/// Le texte du dispositif, avec ou sans le mur mince `H`.
fn texte(avec_h: bool) -> String {
    let mut t = String::from(
        "boite 100 100\nspore 20 50\nsubstrat 1.0 0.0\ncroissance 1.0 infini 30 0\ncouplage 0.0 0.0\n\
         anastomose 6.0 10\nduree 200\nthigmotropisme 35\nobstacle 60 2 60 97\nobstacle 40 56 59 56\n",
    );
    if avec_h {
        t.push_str("obstacle 30 52 58 52\n");
    }
    t
}

/// Joue jusqu'à la naissance de la pointe 3, puis `n` pas de plus ; rend la simulation et le nombre de segments de la
/// pointe 3 après chaque pas.
fn jouer(avec_h: bool, n: usize) -> (Simulation, Vec<usize>) {
    let (mut simulation, _) = construire(&texte(avec_h), graine_pour(0));
    let mut pas = 0;
    while simulation.ramifications() < 3 {
        simulation.avancer();
        pas += 1;
        assert!(pas < 200, "la pointe 3 ne naît pas");
        assert_eq!(
            simulation.anastomoses(),
            0,
            "avec H = {avec_h} : aucune anastomose avant la naissance de la pointe 3"
        );
    }
    assert_eq!(simulation.ramifications(), 3, "avec H = {avec_h} : deux naissances au premier arrêt, une au second");
    let mut suivis = Vec::new();
    for _ in 0..n {
        simulation.avancer();
        suivis.push(simulation.segments().iter().filter(|s| s.apex == 3).count());
    }
    (simulation, suivis)
}

/// Avec le mur mince `H` entre la pointe 3 et les segments de la pointe 0 : aucune anastomose pendant les 24 pas où elle
/// est au-dessus de `H` (elle y avance d'un pas à chaque fois), puis une anastomose une fois `H` passé (au plus tard quand
/// elle est arrivée au-dessus du début de la pointe 0).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn une_anastomose_ne_se_fait_pas_a_travers_un_mur() {
    outils::delai();
    let (simulation, suivis) = jouer(true, 24);
    assert_eq!(suivis, (1..=24).collect::<Vec<_>>(), "la pointe 3 avance d'un pas à chaque pas au-dessus de H");
    assert_eq!(simulation.anastomoses(), 0, "une anastomose à travers le mur H");
    let (simulation, _) = jouer(true, 40);
    assert_eq!(simulation.anastomoses(), 1, "après H, la pointe 3 s'anastomose avec la pointe 0");
}

/// Le témoin : sans `H`, la même pointe 3 s'anastomose dès son premier pas (les segments de la pointe 0 sont à moins de 6
/// cases et assez anciens).
#[test]
#[cfg_attr(not(feature = "etape-7"), ignore)]
fn sans_le_mur_la_meme_pointe_s_anastomose_aussitot() {
    outils::delai();
    let (simulation, suivis) = jouer(false, 5);
    assert_eq!(simulation.anastomoses(), 1, "sans H, une anastomose");
    assert_eq!(suivis, vec![1; 5], "la pointe 3 s'arrête après son premier segment");
}
