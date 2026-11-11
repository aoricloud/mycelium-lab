//! Calage : mesurer la colonie en unités qu'une publication donne (étape 12, M2).
//!
//! Le noyau compte en cases et en pas de temps. Ce module n'y change rien : il lit les [`Releve`]s d'une simulation
//! déjà jouée et en tire les nombres que Trinci (1974) publie pour de vrais champignons, pour les mettre à côté :
//!
//! - **la longueur d'hyphe par pointe** (l'« unité de croissance hyphale » de Trinci) : la biomasse (la longueur
//!   totale d'hyphe, en cases) divisée par le nombre de pointes vivantes, puis en micromètres à
//!   [`MICROMETRES_PAR_CASE`] ;
//! - **le taux de croissance** d'une suite de relevés : la pente de la droite des moindres carrés de `ln(valeur)` en
//!   fonction du pas de temps ; son temps de doublement est `ln 2 / taux` ;
//! - **l'écart relatif des deux taux**, celui de la biomasse et celui des pointes : Trinci écrit qu'ils sont « à peu
//!   près » égaux ; le plus grand écart de son tableau I est 0,04 / 0,34 = 0,118.
//!
//! Ces mesures ne valent que dans la **fenêtre de croissance** ([`fenetre_de_croissance`]) : une jeune colonie, ni au
//! tout début (quelques pointes, un compte qui saute), ni après qu'elle a quitté sa jeunesse (plus de
//! [`POINTES_MAXIMALES`] pointes).
//!
//! Le calcul n'emploie que `+ - * /` et le logarithme de [`crate::mesures::logarithme`] (aucune fonction de `f64`
//! qui passe par la bibliothèque mathématique du système) : le même résultat, bit à bit, sur toutes les machines.

use std::f64::consts::LN_2;

use crate::mesures::logarithme;
use crate::releves::Releve;

/// L'échelle : une case vaut 7 micromètres. C'est un **choix écrit** (le haut de la fourchette « 5 à 7 µm » que Held et
/// coll. 2019 donnent pour le diamètre d'une hyphe de *Neurospora crassa*), pas une mesure publiée avec son incertitude.
pub const MICROMETRES_PAR_CASE: f64 = 7.0;

/// La fenêtre de croissance s'ouvre au premier relevé qui a au moins ce nombre de pointes.
pub const POINTES_MINIMALES: usize = 40;

/// La fenêtre de croissance se referme avant le premier relevé qui a plus que ce nombre de pointes.
pub const POINTES_MAXIMALES: usize = 200;

/// Le bas de la fourchette de longueurs par pointe de 21 espèces (35 µm) : **de seconde main** (le tableau de la page de
/// David Moore, donné pour des données de Bull et Trinci 1977, chapitre non lu). Un garde-fou large, jamais une cible.
pub const FOURCHETTE_MINIMALE_UM: f64 = 35.0;

/// Le haut de la même fourchette (682 µm), de seconde main aussi.
pub const FOURCHETTE_MAXIMALE_UM: f64 = 682.0;

/// La longueur d'hyphe par pointe, en cases : `biomasse / pointes`. Rend `None` quand il n'y a aucune pointe.
pub fn longueur_par_pointe_cases(biomasse: f64, pointes: usize) -> Option<f64> {
    if pointes == 0 { None } else { Some(biomasse / pointes as f64) }
}

/// Convertit une longueur de cases en micromètres : `cases × MICROMETRES_PAR_CASE`.
pub fn en_micrometres(cases: f64) -> f64 {
    cases * MICROMETRES_PAR_CASE
}

/// Le taux de croissance de `valeurs` entre les indices `t_debut` et `t_fin` inclus : la pente de la droite des
/// moindres carrés de `logarithme(valeur)` en fonction de l'indice `t`. Sur une suite géométrique `v0 × r^t`, c'est
/// `ln r`, à l'arrondi près.
///
/// # Panics
///
/// Panique si `t_debut >= t_fin` (il faut au moins deux points), si `t_fin` sort de `valeurs`, ou si une valeur de la
/// plage n'est pas finie et strictement positive.
pub fn taux_de_croissance(valeurs: &[f64], t_debut: usize, t_fin: usize) -> f64 {
    assert!(t_debut < t_fin, "il faut au moins deux points : {t_debut}..={t_fin}");
    assert!(t_fin < valeurs.len(), "t_fin = {t_fin} hors de 0..{}", valeurs.len());
    let mut points = Vec::with_capacity(t_fin - t_debut + 1);
    for (t, &valeur) in valeurs.iter().enumerate().take(t_fin + 1).skip(t_debut) {
        assert!(valeur.is_finite() && valeur > 0.0, "pas de logarithme pour la valeur {valeur} (t = {t})");
        points.push((t as f64, logarithme(valeur)));
    }

    // La pente des moindres carrés, calculée autour des moyennes (plus juste que `n Σxy − Σx Σy` quand t est grand).
    let n = points.len() as f64;
    let moyenne_t = points.iter().map(|p| p.0).sum::<f64>() / n;
    let moyenne_y = points.iter().map(|p| p.1).sum::<f64>() / n;
    let (mut covariance, mut variance) = (0.0, 0.0);
    for (t, y) in points {
        covariance += (t - moyenne_t) * (y - moyenne_y);
        variance += (t - moyenne_t) * (t - moyenne_t);
    }
    covariance / variance
}

/// Le temps de doublement d'un taux de croissance : `ln 2 / taux`, dans l'unité de temps du taux.
///
/// # Panics
///
/// Panique si `taux` n'est pas fini et strictement positif (ce qui ne croît pas ne double pas).
pub fn temps_de_doublement(taux: f64) -> f64 {
    assert!(taux.is_finite() && taux > 0.0, "un temps de doublement demande un taux > 0 : {taux}");
    LN_2 / taux
}

/// L'écart relatif de deux taux : `|taux_pointes − taux_biomasse| / taux_biomasse`. Le dénominateur est le taux de la
/// **biomasse** (la longueur d'hyphe, α_h de Trinci) : c'est ainsi que 0,04 / 0,34 = 0,118, le plus grand écart de son
/// tableau I (*Neurospora crassa*, 25 °C).
///
/// # Panics
///
/// Panique si `taux_biomasse` n'est pas fini et strictement positif, ou si `taux_pointes` n'est pas fini.
pub fn ecart_relatif(taux_biomasse: f64, taux_pointes: f64) -> f64 {
    assert!(taux_biomasse.is_finite() && taux_biomasse > 0.0, "taux de la biomasse pas > 0 : {taux_biomasse}");
    assert!(taux_pointes.is_finite(), "taux des pointes pas fini : {taux_pointes}");
    (taux_pointes - taux_biomasse).abs() / taux_biomasse
}

/// La fenêtre de croissance : les indices `(premier, dernier)`, inclus, des relevés où la colonie est « jeune ».
///
/// - `premier` est le premier relevé qui a au moins [`POINTES_MINIMALES`] pointes ;
/// - `dernier` est le relevé juste avant le premier relevé qui suit `premier` et qui a plus de [`POINTES_MAXIMALES`]
///   pointes.
///
/// Entre les deux, le nombre de pointes peut repasser sous [`POINTES_MINIMALES`] : il monte par à-coups et perd une
/// pointe à chaque anastomose, et la fenêtre ne se coupe pas pour autant. Il ne tombe jamais à zéro (une colonie sans
/// pointe ne repousse pas, elle ne dépasserait donc jamais [`POINTES_MAXIMALES`]).
///
/// Rend `None` si aucun relevé n'a [`POINTES_MINIMALES`] pointes, si le premier qui les a en a déjà plus de
/// [`POINTES_MAXIMALES`], ou si la fenêtre ne s'est pas refermée avant le dernier relevé : une colonie qui n'a pas
/// quitté sa jeunesse pendant la durée jouée, ou qui s'est éteinte, n'a pas de fenêtre.
pub fn fenetre_de_croissance(releves: &[Releve]) -> Option<(usize, usize)> {
    let premier = releves.iter().position(|releve| releve.pointes >= POINTES_MINIMALES)?;
    let trop = premier + releves[premier..].iter().position(|releve| releve.pointes > POINTES_MAXIMALES)?;
    if trop == premier { None } else { Some((premier, trop - 1)) }
}

/// La longueur d'hyphe par pointe moyenne, en cases, sur la fenêtre `(premier, dernier)` (indices inclus de
/// `releves`) : la moyenne des `biomasse / pointes` de ses relevés (pas le rapport des moyennes).
///
/// # Panics
///
/// Panique si la fenêtre est à l'envers ou sort de `releves`, ou si un relevé de la fenêtre n'a aucune pointe.
pub fn longueur_par_pointe_moyenne(releves: &[Releve], fenetre: (usize, usize)) -> f64 {
    let (premier, dernier) = fenetre;
    assert!(premier <= dernier && dernier < releves.len(), "fenêtre {premier}..={dernier} impossible");
    let mut somme = 0.0;
    for releve in &releves[premier..=dernier] {
        somme += longueur_par_pointe_cases(releve.biomasse, releve.pointes)
            .unwrap_or_else(|| panic!("aucune pointe au pas {} : pas de longueur par pointe", releve.temps));
    }
    somme / (dernier - premier + 1) as f64
}
