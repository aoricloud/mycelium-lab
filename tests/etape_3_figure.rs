//! Tests d'acceptation de l'étape 3 (a) : la figure 3 de Meškauskas, Fricker et Moore (2004), en nombres.
//!
//! Chaque test ne s'exécute que si la fonctionnalité `etape-3` est active. Les deux jeux de graines sont simulés
//! une seule fois, paresseusement, et partagés entre les tests.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt` ; le logarithme vient de
//! [`mycelium_lab::mesures::logarithme`].

use std::ops::RangeInclusive;
use std::sync::OnceLock;

use mycelium_lab::mesures;
use mycelium_lab::voisinage::{Colonie, Parametres, ajuster, ajuster_rayon, exposant};

/// Nombre de pas de temps simulés.
const PAS_TEMPS: u32 = 400;
/// Début (exclu) de la fenêtre tardive.
const T_DEBUT: u32 = 200;
/// Fin (incluse) de la fenêtre tardive.
const T_FIN: u32 = 400;

/// Simule chaque graine du jeu pendant [`PAS_TEMPS`] pas et rend ses rayons, un par pas.
fn simuler(graines: RangeInclusive<u64>) -> Vec<Vec<f64>> {
    let parametres = Parametres::figure_3();
    graines
        .map(|graine| {
            let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
            for _ in 0..PAS_TEMPS {
                colonie.avancer();
                assert!(
                    colonie.pointes() <= 500,
                    "graine {graine} : {} pointes, la ramification s'emballe",
                    colonie.pointes()
                );
            }
            colonie.rayons().to_vec()
        })
        .collect()
}

/// Jeu A : les graines 1 à 30, simulées une seule fois pour toute la durée du processus.
fn jeu_a() -> &'static [Vec<f64>] {
    static JEU: OnceLock<Vec<Vec<f64>>> = OnceLock::new();
    JEU.get_or_init(|| simuler(1..=30))
}

/// Jeu B : les graines 1001 à 1030, simulées une seule fois pour toute la durée du processus.
fn jeu_b() -> &'static [Vec<f64>] {
    static JEU: OnceLock<Vec<Vec<f64>>> = OnceLock::new();
    JEU.get_or_init(|| simuler(1001..=1030))
}

/// Coefficient de détermination de l'ajustement de `ln(rayon)` sur le temps, sur la fenêtre `]200, 400]`.
fn r2_logarithme(rayons: &[f64]) -> f64 {
    let nombre = (T_FIN - T_DEBUT) as usize;
    let mut temps = Vec::with_capacity(nombre);
    let mut logs = Vec::with_capacity(nombre);
    for t in (T_DEBUT + 1)..=T_FIN {
        temps.push(t as f64);
        logs.push(mesures::logarithme(rayons[(t - 1) as usize]));
    }
    ajuster(&temps, &logs).r2
}

/// Les grandeurs moyennes d'un jeu de graines.
struct Statistiques {
    /// Moyenne des pentes du rayon sur le temps.
    pente_moyenne: f64,
    /// Moyenne des R² de l'ajustement linéaire du rayon sur le temps.
    r2_lineaire_moyen: f64,
    /// Moyenne des R² de l'ajustement de `ln(rayon)` sur le temps.
    r2_log_moyen: f64,
    /// Écart-type (population) des pentes.
    ecart_type_pente: f64,
    /// Écart entre la plus grande et la plus petite pente.
    etendue_pente: f64,
}

/// Calcule les statistiques d'un jeu de graines sur la fenêtre tardive.
fn statistiques(jeu: &[Vec<f64>]) -> Statistiques {
    let mut pentes = Vec::with_capacity(jeu.len());
    let mut somme_r2_lineaire = 0.0;
    let mut somme_r2_log = 0.0;
    for rayons in jeu {
        let ajustement = ajuster_rayon(rayons, T_DEBUT, T_FIN);
        pentes.push(ajustement.pente);
        somme_r2_lineaire += ajustement.r2;
        somme_r2_log += r2_logarithme(rayons);
    }

    let n = jeu.len() as f64;
    let mut somme_pentes = 0.0;
    for &pente in &pentes {
        somme_pentes += pente;
    }
    let pente_moyenne = somme_pentes / n;

    let mut somme_ecarts = 0.0;
    for &pente in &pentes {
        let ecart = pente - pente_moyenne;
        somme_ecarts += ecart * ecart;
    }
    let ecart_type_pente = (somme_ecarts / n).sqrt();

    let mut pente_min = pentes[0];
    let mut pente_max = pentes[0];
    for &pente in &pentes[1..] {
        if pente < pente_min {
            pente_min = pente;
        }
        if pente > pente_max {
            pente_max = pente;
        }
    }

    Statistiques {
        pente_moyenne,
        r2_lineaire_moyen: somme_r2_lineaire / n,
        r2_log_moyen: somme_r2_log / n,
        ecart_type_pente,
        etendue_pente: pente_max - pente_min,
    }
}

/// Vérifie, pour chaque graine d'un jeu, les assertions de la figure 3.
fn verifier_graines(jeu: &[Vec<f64>], premiere_graine: u64, pas: f64) {
    for (rang, rayons) in jeu.iter().enumerate() {
        let graine = premiere_graine + rang as u64;
        assert_eq!(rayons.len(), PAS_TEMPS as usize, "graine {graine} : nombre de rayons");

        let ajustement = ajuster_rayon(rayons, T_DEBUT, T_FIN);
        assert!(ajustement.r2 >= 0.995, "graine {graine} : R² {:.6}", ajustement.r2);
        assert!(ajustement.pente >= 0.80, "graine {graine} : pente {:.6}", ajustement.pente);
        assert!(ajustement.pente <= 1.0, "graine {graine} : pente {:.6}", ajustement.pente);

        let exposant = exposant(rayons, T_DEBUT, T_FIN);
        assert!(exposant >= 0.85, "graine {graine} : exposant {exposant:.6}");
        assert!(exposant <= 1.15, "graine {graine} : exposant {exposant:.6}");

        for (rang_temps, &rayon) in rayons.iter().enumerate() {
            let t = (rang_temps + 1) as f64;
            let borne = pas * t + 1e-9;
            assert!(
                rayon <= borne,
                "graine {graine} : rayon {rayon:.6} au temps {} dépasse {borne:.6}",
                rang_temps + 1
            );
        }
    }
}

/// Vérifie toutes les assertions du test 1 (par graine puis en moyenne) sur un jeu.
fn verifier_jeu(jeu: &[Vec<f64>], premiere_graine: u64, pas: f64) {
    verifier_graines(jeu, premiere_graine, pas);

    let statistiques = statistiques(jeu);
    assert!(statistiques.pente_moyenne >= 0.80, "pente moyenne {:.6}", statistiques.pente_moyenne);
    assert!(statistiques.pente_moyenne <= 0.97, "pente moyenne {:.6}", statistiques.pente_moyenne);

    let exces = statistiques.r2_lineaire_moyen - statistiques.r2_log_moyen;
    assert!(
        exces >= 0.004,
        "R² linéaire moyen {:.6}, R² logarithmique moyen {:.6}, excès {exces:.6}",
        statistiques.r2_lineaire_moyen,
        statistiques.r2_log_moyen
    );
}

/// 1. La figure 3 sur le jeu A : croissance linéaire du rayon, pente sous 1 et dans `[0.80 ; 0.97]` en moyenne
///    (le prototype donne 0,915 ; la valeur publiée, 0,86, est dans l'intervalle mais n'est pas la cible), et
///    meilleur ajustement linéaire que logarithmique.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn figure_3_croissance_lineaire_du_rayon() {
    let pas = Parametres::figure_3().pas;
    verifier_jeu(jeu_a(), 1, pas);
}

/// 2. L'indépendance à la graine : les assertions du test 1 tiennent sur le jeu B, et les deux jeux donnent des
///    pentes proches, peu dispersées, sans valeur extrême.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn figure_3_independante_de_la_graine() {
    let pas = Parametres::figure_3().pas;
    verifier_jeu(jeu_b(), 1001, pas);

    let stats_a = statistiques(jeu_a());
    let stats_b = statistiques(jeu_b());

    let difference = stats_a.pente_moyenne - stats_b.pente_moyenne;
    assert!(difference <= 0.03, "différence des pentes moyennes {difference:.6}");
    assert!(difference >= -0.03, "différence des pentes moyennes {difference:.6}");

    assert!(stats_a.ecart_type_pente <= 0.06, "écart-type du jeu A {:.6}", stats_a.ecart_type_pente);
    assert!(stats_b.ecart_type_pente <= 0.06, "écart-type du jeu B {:.6}", stats_b.ecart_type_pente);

    assert!(stats_a.etendue_pente <= 0.25, "étendue du jeu A {:.6}", stats_a.etendue_pente);
    assert!(stats_b.etendue_pente <= 0.25, "étendue du jeu B {:.6}", stats_b.etendue_pente);
}
