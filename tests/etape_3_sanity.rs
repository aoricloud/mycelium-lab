//! Tests de bon sens de l'étape 3 : ils échouent pour un modèle faux.
//!
//! Chaque jeu coûteux (le modèle de base à 400 pas, le modèle sans autotropisme à 400 pas, le modèle sans limite de
//! densité à 24 pas) n'est joué qu'une fois, par un [`std::sync::OnceLock`], et se partage entre les tests.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt` ; le logarithme est celui de
//! [`mesures::logarithme`]. Aucune `HashMap` ni aucun `HashSet` n'est parcourue.

use std::sync::OnceLock;

use mycelium_lab::mesures;
use mycelium_lab::voisinage::{Colonie, Parametres, Ramification};

/// Nombre de graines du jeu A, qui vont de 1 à `GRAINES`.
const GRAINES: u64 = 30;

/// Le résultat d'un jeu : pour chaque graine de A, les séries de sa colonie et ses ramifications.
struct Jeu {
    rayons: Vec<Vec<f64>>,
    pointes_par_pas: Vec<Vec<u32>>,
    ramifications: Vec<Vec<Ramification>>,
}

/// Joue un jeu complet : une colonie par graine de A, depuis la spore `(0, 0)`, pendant `pas` pas.
fn jouer(parametres: Parametres, pas: u32) -> Jeu {
    let mut jeu = Jeu {
        rayons: Vec::with_capacity(GRAINES as usize),
        pointes_par_pas: Vec::with_capacity(GRAINES as usize),
        ramifications: Vec::with_capacity(GRAINES as usize),
    };
    for graine in 1..=GRAINES {
        let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
        colonie.avancer_n(pas);
        jeu.rayons.push(colonie.rayons().to_vec());
        jeu.pointes_par_pas.push(colonie.pointes_par_pas().to_vec());
        jeu.ramifications.push(colonie.ramifications().to_vec());
    }
    jeu
}

static BASE_400: OnceLock<Jeu> = OnceLock::new();
static SANS_AUTOTROPISME_400: OnceLock<Jeu> = OnceLock::new();
static SANS_LIMITE_24: OnceLock<Jeu> = OnceLock::new();

/// Le modèle de base (les paramètres de la figure 3) sur 400 pas.
fn base_400() -> &'static Jeu {
    BASE_400.get_or_init(|| jouer(Parametres::figure_3(), 400))
}

/// Le modèle de la figure 3 privé d'autotropisme (`persistance = 1,0`) sur 400 pas.
fn sans_autotropisme_400() -> &'static Jeu {
    SANS_AUTOTROPISME_400.get_or_init(|| jouer(Parametres { persistance: 1.0, ..Parametres::figure_3() }, 400))
}

/// Le modèle de la figure 3 privé de limite de densité (`seuil_densite = infini`) sur 24 pas.
fn sans_limite_24() -> &'static Jeu {
    SANS_LIMITE_24.get_or_init(|| jouer(Parametres { seuil_densite: f64::INFINITY, ..Parametres::figure_3() }, 24))
}

/// Pente des moindres carrés de `y` sur `x`, sans aucune fonction de `f64` hors `+`, `-`, `*` et `/`.
fn pente(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len(), "x et y n'ont pas la même longueur");
    let mut n = 0.0;
    let mut somme_x = 0.0;
    let mut somme_y = 0.0;
    let mut somme_xx = 0.0;
    let mut somme_xy = 0.0;
    for (xi, yi) in x.iter().zip(y.iter()) {
        n += 1.0;
        somme_x += *xi;
        somme_y += *yi;
        somme_xx += *xi * *xi;
        somme_xy += *xi * *yi;
    }
    (n * somme_xy - somme_x * somme_y) / (n * somme_xx - somme_x * somme_x)
}

/// Pente de `serie` sur `t`, pour les temps `t` de `t_debut` à `t_fin` inclus : `serie[t - 1]` est la valeur au
/// temps `t`.
fn pente_entre(serie: &[f64], t_debut: u32, t_fin: u32) -> f64 {
    let mut temps = Vec::with_capacity((t_fin - t_debut + 1) as usize);
    let mut valeurs = Vec::with_capacity((t_fin - t_debut + 1) as usize);
    for t in t_debut..=t_fin {
        temps.push(t as f64);
        valeurs.push(serie[(t - 1) as usize]);
    }
    pente(&temps, &valeurs)
}

/// Pente de `ln(pointes)` sur `t`, pour les temps `t` de 6 à 24 inclus.
fn pente_log_des_pointes(pointes_par_pas: &[u32]) -> f64 {
    let logarithme: Vec<f64> = pointes_par_pas.iter().map(|&pointes| mesures::logarithme(pointes as f64)).collect();
    pente_entre(&logarithme, 6, 24)
}

/// 1. Sans autotropisme (`persistance = 1,0`), la colonie avance droit : sur la fenêtre `]200, 400]`, la pente du
///    rayon vaut au moins 0,999 pour chaque graine du jeu A, et la moyenne des pentes dépasse celle du modèle de
///    base d'au moins 0,05.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn sans_autotropisme_le_rayon_a_la_pente_du_pas() {
    let sans = sans_autotropisme_400();
    let base = base_400();

    let mut somme = 0.0;
    let mut somme_base = 0.0;
    for (indice, (rayons_sans, rayons_base)) in sans.rayons.iter().zip(base.rayons.iter()).enumerate() {
        let graine = indice + 1;
        let pente_sans = pente_entre(rayons_sans, 201, 400);
        assert!(pente_sans >= 0.999, "graine {graine} : pente {pente_sans}");
        somme += pente_sans;
        somme_base += pente_entre(rayons_base, 201, 400);
    }

    let moyenne = somme / GRAINES as f64;
    let moyenne_base = somme_base / GRAINES as f64;
    assert!(moyenne - moyenne_base >= 0.05, "moyenne {moyenne} contre {moyenne_base} pour le modèle de base");
}

/// 2. Sans limite de densité (`seuil_densite = infini`), les ramifications explosent : à 24 pas, chaque graine du
///    jeu A a au moins 100 pointes et la pente de `ln(pointes)` sur `t` de 6 à 24 vaut au moins 0,25. Le modèle de
///    base, à 24 pas, reste à 10 pointes au plus et à une pente de 0,10 au plus.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn sans_limite_de_densite_les_pointes_explosent() {
    let sans = sans_limite_24();
    let base = base_400();

    for (indice, pointes_par_pas) in sans.pointes_par_pas.iter().enumerate() {
        let graine = indice + 1;
        let pointes = pointes_par_pas[23];
        assert!(pointes >= 100, "graine {graine} : {pointes} pointes à 24 pas");
        let pente = pente_log_des_pointes(pointes_par_pas);
        assert!(pente >= 0.25, "graine {graine} : pente {pente}");
    }

    for (indice, pointes_par_pas) in base.pointes_par_pas.iter().enumerate() {
        let graine = indice + 1;
        let pointes = pointes_par_pas[23];
        assert!(pointes <= 10, "graine {graine} : {pointes} pointes à 24 pas");
        let pente = pente_log_des_pointes(pointes_par_pas);
        assert!(pente <= 0.10, "graine {graine} : pente {pente}");
    }
}

/// 3. La limite de densité tient : sur le modèle de base à 400 pas, chaque `Ramification` a une densité
///    strictement inférieure au seuil (0,06) ; sans limite, la densité maximale à une ramification dépasse 100.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn la_limite_de_densite_tient() {
    let base = base_400();
    let seuil = Parametres::figure_3().seuil_densite;
    for (indice, ramifications) in base.ramifications.iter().enumerate() {
        let graine = indice + 1;
        for ramification in ramifications {
            assert!(
                ramification.densite < seuil,
                "graine {graine}, temps {} : densité {} au seuil {seuil}",
                ramification.temps,
                ramification.densite
            );
        }
    }

    let sans = sans_limite_24();
    let mut maximale = 0.0;
    for ramifications in &sans.ramifications {
        for ramification in ramifications {
            if ramification.densite > maximale {
                maximale = ramification.densite;
            }
        }
    }
    assert!(maximale > 100.0, "densité maximale à une ramification : {maximale}");
}
