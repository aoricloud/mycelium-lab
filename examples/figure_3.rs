//! La figure 3 de Meškauskas, Fricker et Moore (2004) en nombres.
//!
//! `cargo run --example figure_3` joue les graines 1 à 30 avec [`Parametres::figure_3()`], depuis une spore en
//! `(0, 0)`, pendant 400 pas. Pour chaque graine il imprime la pente du rayon sur `]200, 400]`, le R² de son
//! ajustement linéaire, l'exposant de la loi de puissance et le nombre de pointes à 400 pas ; puis les statistiques
//! du jeu. Les nombres imprimés sont ceux recopiés dans `MODELE.md`.
//!
//! Comme le modèle, l'exemple n'appelle aucune fonction de `f64` de la bibliothèque mathématique du système :
//! `+`, `-`, `*`, `/` et `sqrt` suffisent.

use mycelium_lab::voisinage::{Colonie, Parametres, ajuster_rayon, exposant};

/// Nombre de graines du jeu A.
const GRAINES: u64 = 30;
/// Nombre de pas de temps simulés.
const PAS_TEMPS: u32 = 400;
/// Début (exclu) de la fenêtre de mesure.
const T_DEBUT: u32 = 200;
/// Fin (incluse) de la fenêtre de mesure.
const T_FIN: u32 = 400;
/// Pente publiée de la figure 3, en fois la vitesse d'un apex.
const PENTE_PUBLIEE: f64 = 0.86;

/// Moyenne arithmétique.
fn moyenne(valeurs: &[f64]) -> f64 {
    let mut somme = 0.0;
    for &valeur in valeurs {
        somme += valeur;
    }
    somme / valeurs.len() as f64
}

/// Écart-type de population, autour de la moyenne.
fn ecart_type(valeurs: &[f64]) -> f64 {
    let m = moyenne(valeurs);
    let mut carres = 0.0;
    for &valeur in valeurs {
        let ecart = valeur - m;
        carres += ecart * ecart;
    }
    (carres / valeurs.len() as f64).sqrt()
}

fn main() {
    let parametres = Parametres::figure_3();
    let mut pentes = Vec::with_capacity(GRAINES as usize);
    let mut r2 = Vec::with_capacity(GRAINES as usize);
    let mut exposants = Vec::with_capacity(GRAINES as usize);
    let mut pointes = Vec::with_capacity(GRAINES as usize);

    println!("figure 3 : Parametres::figure_3(), spore (0, 0), {PAS_TEMPS} pas, fenetre ]{T_DEBUT}, {T_FIN}]");
    println!("graine\tpente\t\tr2\t\texposant\tpointes");

    for graine in 1..=GRAINES {
        let mut colonie = Colonie::depuis_spore((0.0, 0.0), parametres, graine);
        colonie.avancer_n(PAS_TEMPS);
        let rayons = colonie.rayons();
        let ajustement = ajuster_rayon(rayons, T_DEBUT, T_FIN);
        let pente = ajustement.pente;
        let r2_graine = ajustement.r2;
        let exposant_graine = exposant(rayons, T_DEBUT, T_FIN);
        let pointes_graine = colonie.pointes();
        println!("{graine}\t{pente:.9}\t{r2_graine:.9}\t{exposant_graine:.9}\t{pointes_graine}");
        pentes.push(pente);
        r2.push(r2_graine);
        exposants.push(exposant_graine);
        pointes.push(pointes_graine as f64);
    }

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
    let mut pointes_min = pointes[0];
    let mut pointes_max = pointes[0];
    for &nombre in &pointes[1..] {
        if nombre < pointes_min {
            pointes_min = nombre;
        }
        if nombre > pointes_max {
            pointes_max = nombre;
        }
    }

    let pente_moyenne = moyenne(&pentes);
    println!();
    println!("pente moyenne : {pente_moyenne:.9}");
    println!("ecart-type de la pente : {:.9}", ecart_type(&pentes));
    println!("pente minimale : {pente_min:.9}");
    println!("pente maximale : {pente_max:.9}");
    println!("R2 moyen de l'ajustement lineaire : {:.9}", moyenne(&r2));
    println!("exposant moyen : {:.9}", moyenne(&exposants));
    println!(
        "pointes a {PAS_TEMPS} pas : moyenne {:.3}, minimum {}, maximum {}",
        moyenne(&pointes),
        pointes_min as u64,
        pointes_max as u64
    );
    println!(
        "valeur publiee : {PENTE_PUBLIEE:.2} ; ecart mesure (pente moyenne - publiee) : {:.9}",
        pente_moyenne - PENTE_PUBLIEE
    );
}
