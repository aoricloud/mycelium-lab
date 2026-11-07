//! Tests d'acceptation de l'étape 3 : la vitesse radiale constante prédite par Edelstein (1982).
//!
//! Le jeu A (graines 1 à 30) se calcule une seule fois : les paramètres publiés de la figure 3, une spore en
//! `(0, 0)` et quatre cents pas.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt`, et [`mesures::logarithme`]
//! pour le logarithme.

use std::sync::OnceLock;

use mycelium_lab::mesures;
use mycelium_lab::voisinage;
use mycelium_lab::voisinage::Parametres;

/// Le jeu A : les rayons et les nombres de pointes de chaque graine, un élément par graine.
struct JeuA {
    rayons: Vec<Vec<f64>>,
    pointes: Vec<Vec<u32>>,
}

/// Le jeu A, calculé une seule fois et partagé entre les tests.
static JEU_A: OnceLock<JeuA> = OnceLock::new();

/// Rend le jeu A, en le calculant au premier appel.
fn jeu_a() -> &'static JeuA {
    JEU_A.get_or_init(|| {
        let parametres = Parametres::figure_3();
        let mut rayons = Vec::new();
        let mut pointes = Vec::new();
        for graine in 1..=30u64 {
            let mut colonie = voisinage::Colonie::depuis_spore((0.0, 0.0), parametres, graine);
            colonie.avancer_n(400);
            rayons.push(colonie.rayons().to_vec());
            pointes.push(colonie.pointes_par_pas().to_vec());
        }
        JeuA { rayons, pointes }
    })
}

/// Moyenne des valeurs.
fn moyenne(valeurs: &[f64]) -> f64 {
    let mut somme = 0.0;
    for &valeur in valeurs {
        somme += valeur;
    }
    somme / valeurs.len() as f64
}

/// Écart-type des valeurs, autour de leur moyenne.
fn ecart_type(valeurs: &[f64]) -> f64 {
    let moyenne = moyenne(valeurs);
    let mut carres = 0.0;
    for &valeur in valeurs {
        let ecart = valeur - moyenne;
        carres += ecart * ecart;
    }
    (carres / valeurs.len() as f64).sqrt()
}

/// Coefficient de variation : l'écart-type divisé par la moyenne.
fn coefficient_de_variation(valeurs: &[f64]) -> f64 {
    ecart_type(valeurs) / moyenne(valeurs)
}

/// Rapport de la moyenne de la seconde moitié à celle de la première moitié.
fn rapport_des_moities(valeurs: &[f64]) -> f64 {
    let milieu = valeurs.len() / 2;
    moyenne(&valeurs[milieu..]) / moyenne(&valeurs[..milieu])
}

/// Pente de `ln(pointes)` sur le temps, pour les temps `]debut, fin]`.
fn pente_du_logarithme(pointes: &[u32], debut: u32, fin: u32) -> f64 {
    let mut temps = Vec::new();
    let mut logs = Vec::new();
    for (decalage, &nombre) in pointes[debut as usize..fin as usize].iter().enumerate() {
        temps.push((debut as usize + decalage + 1) as f64);
        logs.push(mesures::logarithme(nombre as f64));
    }
    voisinage::ajuster(&temps, &logs).pente
}

/// 1. La moyenne sur les 30 graines des pentes du rayon sur `]100, 200]`, `]200, 300]` et `]300, 400]` a un
///    rapport plus grand / plus petit au plus 1,06 : la vitesse radiale est constante.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn les_pentes_du_rayon_se_valent_a_six_pour_cent_pres() {
    let jeu = jeu_a();
    let mut pentes = [0.0f64; 3];
    for rayons in &jeu.rayons {
        pentes[0] += voisinage::ajuster_rayon(rayons, 100, 200).pente;
        pentes[1] += voisinage::ajuster_rayon(rayons, 200, 300).pente;
        pentes[2] += voisinage::ajuster_rayon(rayons, 300, 400).pente;
    }
    for pente in &mut pentes {
        *pente /= jeu.rayons.len() as f64;
    }

    let mut plus_grande = pentes[0];
    let mut plus_petite = pentes[0];
    for &pente in &pentes[1..] {
        if pente > plus_grande {
            plus_grande = pente;
        }
        if pente < plus_petite {
            plus_petite = pente;
        }
    }
    let rapport = plus_grande / plus_petite;
    assert!(rapport <= 1.06, "pentes {pentes:?} : rapport {rapport} au-delà de 1,06");
}

/// 2. Pour chaque graine, le coefficient de variation de `vitesses_glissantes(rayons, 200, 400, 50, 10)` est
///    au plus 0,15.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn chaque_vitesse_glissante_a_un_coefficient_de_variation_faible() {
    let jeu = jeu_a();
    for (indice, rayons) in jeu.rayons.iter().enumerate() {
        let vitesses = voisinage::vitesses_glissantes(rayons, 200, 400, 50, 10);
        let variation = coefficient_de_variation(&vitesses);
        assert!(variation <= 0.15, "graine {} : coefficient de variation {variation}", indice + 1);
    }
}

/// 3. Le rapport de la moyenne de la seconde moitié à celle de la première moitié des vitesses glissantes,
///    moyenné sur les graines, est dans `[0.95 ; 1.08]`.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn la_vitesse_radiale_ne_derive_pas_entre_les_moities() {
    let jeu = jeu_a();
    let mut somme = 0.0;
    for rayons in &jeu.rayons {
        let vitesses = voisinage::vitesses_glissantes(rayons, 200, 400, 50, 10);
        somme += rapport_des_moities(&vitesses);
    }
    let rapport = somme / jeu.rayons.len() as f64;
    assert!(rapport >= 0.95, "rapport {rapport} inférieur à 0,95");
    assert!(rapport <= 1.08, "rapport {rapport} supérieur à 1,08");
}

/// 4. La moyenne de `exposant(rayons, 200, 400)` est dans `[0.95 ; 1.05]` : le rayon croît comme le temps.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn l_exposant_du_rayon_vaut_un() {
    let jeu = jeu_a();
    let mut somme = 0.0;
    for rayons in &jeu.rayons {
        somme += voisinage::exposant(rayons, 200, 400);
    }
    let exposant = somme / jeu.rayons.len() as f64;
    assert!(exposant >= 0.95, "exposant {exposant} inférieur à 0,95");
    assert!(exposant <= 1.05, "exposant {exposant} supérieur à 1,05");
}

/// 5. La croissance du nombre de pointes n'est pas exponentielle : pour chaque graine, la pente de
///    `ln(pointes)` sur `]300, 400]` est au plus 0,85 fois celle sur `]200, 300]`.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn le_nombre_de_pointes_ne_croit_pas_exponentiellement() {
    let jeu = jeu_a();
    for (indice, pointes) in jeu.pointes.iter().enumerate() {
        let premiere = pente_du_logarithme(pointes, 200, 300);
        let seconde = pente_du_logarithme(pointes, 300, 400);
        assert!(
            seconde <= 0.85 * premiere,
            "graine {} : pente {premiere} puis {seconde}, au-delà de 0,85 fois la première",
            indice + 1
        );
    }
}
