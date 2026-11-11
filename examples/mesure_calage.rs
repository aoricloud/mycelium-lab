//! Mesure du calage (étape 12, M2) : la longueur d'hyphe par pointe et les taux de croissance des deux profils, à côté
//! des chiffres publiés.
//!
//! `cargo run --release --example mesure_calage [-- <première graine> <dernière graine> [rapide|dense]]`
//!
//! Sans argument : les graines 1 à 20, les deux profils (`rapide` puis `dense`, séparés par une ligne vide). Pour
//! chaque graine, l'exemple joue le scénario du profil jusqu'à ce que la fenêtre de croissance se referme (ou jusqu'à
//! la `duree` du scénario), puis écrit sur la sortie standard, en mots séparés par une espace :
//!
//! - `profil <nom> <scénario>`, puis l'en-tête `graine premier dernier longueur_um taux_biomasse taux_pointes ecart` ;
//! - une ligne par graine : les indices de la fenêtre, la longueur par pointe moyenne en micromètres (1 décimale), les
//!   taux par pas de temps de la biomasse et des pointes (4 décimales) et leur écart relatif (3 décimales) ; `<graine>
//!   aucune` si la fenêtre n'existe pas ou n'a qu'un relevé ;
//! - le résumé, une ligne `<nom> <valeur>` chacune, sur les graines mesurées : `graines`, `sans_fenetre`,
//!   `longueur_um_moyenne`, `longueur_um_ecart_type` (diviseur `n − 1`), `longueur_um_minimum`, `longueur_um_maximum`,
//!   `dans_la_fourchette_35_682` (le garde-fou de seconde main), `taux_biomasse_moyen`, `taux_pointes_moyen`,
//!   `ecart_des_taux_moyens` (Trinci publie des taux moyens), `ecart_moyen`, `ecart_maximal`, `au_dessus_de_0_12`
//!   (le nombre de graines dont l'écart dépasse la tolérance) ; `aucun` à la place d'un nombre qui n'existe pas.
//!
//! Sur une erreur, le message et l'usage vont sur la sortie d'erreur, rien n'est imprimé et le code de sortie est 2.
//! L'exemple n'écrit aucun fichier. Les tests de l'étape 12 l'incluent comme un module (`#[path]`) et appellent ses
//! fonctions : ils testent donc toujours ce code-ci, jamais un binaire périmé.

use std::fmt::Write as _;
use std::process::ExitCode;

use mycelium_lab::calage::{
    FOURCHETTE_MAXIMALE_UM, FOURCHETTE_MINIMALE_UM, ecart_relatif, en_micrometres, fenetre_de_croissance,
    longueur_par_pointe_moyenne, taux_de_croissance,
};
use mycelium_lab::releves::Releve;
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

const USAGE: &str = "usage : mesure_calage [<première graine> <dernière graine> [rapide|dense]]";

/// Les deux profils : leur nom et leur scénario, sous la racine du dépôt.
pub const PROFILS: [(&str, &str); 2] =
    [("rapide", "scenarios/calage-rapide.txt"), ("dense", "scenarios/calage-dense.txt")];

/// La tolérance de Trinci : le plus grand écart de son tableau I (0,118), arrondi.
pub const TOLERANCE: f64 = 0.12;

/// Ce qui se mesure dans la fenêtre de croissance d'une graine.
#[derive(Debug, PartialEq)]
pub struct Mesure {
    pub fenetre: (usize, usize),
    pub longueur_um: f64,
    pub taux_biomasse: f64,
    pub taux_pointes: f64,
    pub ecart: f64,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match lancer(&args) {
        Ok(texte) => {
            print!("{texte}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Les graines demandées (bornes incluses) et le profil choisi (`None` : les deux).
pub fn lire_les_arguments(args: &[String]) -> Result<(u64, u64, Option<&str>), String> {
    let (premiere, derniere, profil) = match args {
        [] => return Ok((1, 20, None)),
        [premiere, derniere] => (premiere, derniere, None),
        [premiere, derniere, profil] => (premiere, derniere, Some(profil.as_str())),
        _ => return Err("zéro, deux ou trois arguments attendus".to_string()),
    };
    if let Some(nom) = profil
        && !PROFILS.iter().any(|(connu, _)| *connu == nom)
    {
        return Err(format!("profil inconnu : {nom}"));
    }
    let premiere: u64 = premiere.parse().map_err(|_| format!("graine illisible : {premiere}"))?;
    let derniere: u64 = derniere.parse().map_err(|_| format!("graine illisible : {derniere}"))?;
    if derniere < premiere {
        return Err(format!("la dernière graine ({derniere}) précède la première ({premiere})"));
    }
    Ok((premiere, derniere, profil))
}

/// Tout le texte à écrire, ou la raison d'un refus.
pub fn lancer(args: &[String]) -> Result<String, String> {
    let (premiere, derniere, choisi) = lire_les_arguments(args)?;
    let mut texte = String::new();
    for (nom, fichier) in PROFILS.iter().filter(|(nom, _)| choisi.is_none_or(|c| c == *nom)) {
        let chemin = format!("{}/{fichier}", env!("CARGO_MANIFEST_DIR"));
        let scenario = std::fs::read_to_string(&chemin).map_err(|e| format!("{chemin} : {e}"))?;
        let mut mesures = Vec::new();
        for graine in premiere..=derniere {
            mesures.push((graine, mesurer(&jouer(&scenario, graine)?)));
        }
        if !texte.is_empty() {
            texte.push('\n');
        }
        ecrire_le_profil(&mut texte, nom, fichier, &mesures);
    }
    Ok(texte)
}

/// Joue le scénario avec la graine, pas à pas, jusqu'à ce que la fenêtre de croissance se referme ou jusqu'à la
/// `duree` du scénario, et rend les relevés (un par pas, du pas 0 au dernier joué). La fenêtre et ses mesures sont les
/// mêmes que si toute la durée avait été jouée : la suite ne les change pas.
pub fn jouer(scenario: &str, graine: u64) -> Result<Vec<Releve>, String> {
    let (reglages, milieu, tropismes, duree) = lire_scenario_tropismes(scenario).map_err(|e| format!("{e:?}"))?;
    let mut simulation =
        Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine).map_err(|e| format!("{e:?}"))?;
    while simulation.temps() < duree && fenetre_de_croissance(simulation.releves()).is_none() {
        simulation.avancer();
    }
    Ok(simulation.releves().to_vec())
}

/// Les mesures de la fenêtre de croissance des relevés ; `None` s'il n'y a pas de fenêtre, si elle n'a qu'un relevé
/// (pas de pente) ou si la biomasse n'y croît pas.
pub fn mesurer(releves: &[Releve]) -> Option<Mesure> {
    let (premier, dernier) = fenetre_de_croissance(releves).filter(|(p, d)| p < d)?;
    let biomasse: Vec<f64> = releves.iter().map(|r| r.biomasse).collect();
    let pointes: Vec<f64> = releves.iter().map(|r| r.pointes as f64).collect();
    let taux_biomasse = taux_de_croissance(&biomasse, premier, dernier);
    let taux_pointes = taux_de_croissance(&pointes, premier, dernier);
    (taux_biomasse > 0.0).then(|| Mesure {
        fenetre: (premier, dernier),
        longueur_um: en_micrometres(longueur_par_pointe_moyenne(releves, (premier, dernier))),
        taux_biomasse,
        taux_pointes,
        ecart: ecart_relatif(taux_biomasse, taux_pointes),
    })
}

/// La moyenne, additionnée dans l'ordre ; `None` pour une liste vide.
pub fn moyenne(valeurs: &[f64]) -> Option<f64> {
    if valeurs.is_empty() {
        return None;
    }
    let mut somme = 0.0;
    for v in valeurs {
        somme += v;
    }
    Some(somme / valeurs.len() as f64)
}

/// L'écart-type de l'échantillon (diviseur `n − 1`) ; `None` sous deux valeurs.
pub fn ecart_type(valeurs: &[f64]) -> Option<f64> {
    let m = moyenne(valeurs).filter(|_| valeurs.len() >= 2)?;
    let mut somme = 0.0;
    for v in valeurs {
        somme += (v - m) * (v - m);
    }
    Some((somme / (valeurs.len() - 1) as f64).sqrt())
}

/// Écrit le bloc d'un profil : son nom, l'en-tête, une ligne par graine, le résumé.
pub fn ecrire_le_profil(texte: &mut String, nom: &str, fichier: &str, mesures: &[(u64, Option<Mesure>)]) {
    let nombre = |v: Option<f64>, decimales: usize| v.map_or("aucun".to_string(), |v| format!("{v:.decimales$}"));
    writeln!(texte, "profil {nom} {fichier}").unwrap();
    writeln!(texte, "graine premier dernier longueur_um taux_biomasse taux_pointes ecart").unwrap();
    let mesurees: Vec<&Mesure> = mesures.iter().filter_map(|(_, m)| m.as_ref()).collect();
    for (graine, mesure) in mesures {
        match mesure {
            None => writeln!(texte, "{graine} aucune").unwrap(),
            Some(m) => writeln!(
                texte,
                "{graine} {} {} {:.1} {:.4} {:.4} {:.3}",
                m.fenetre.0, m.fenetre.1, m.longueur_um, m.taux_biomasse, m.taux_pointes, m.ecart
            )
            .unwrap(),
        }
    }
    let colonne = |f: fn(&Mesure) -> f64| -> Vec<f64> { mesurees.iter().map(|m| f(m)).collect() };
    let (um, biomasse, pointes, ecarts) =
        (colonne(|m| m.longueur_um), colonne(|m| m.taux_biomasse), colonne(|m| m.taux_pointes), colonne(|m| m.ecart));
    let minimum = um.iter().copied().reduce(f64::min);
    let maximum = um.iter().copied().reduce(f64::max);
    let dans = um.iter().filter(|&&v| (FOURCHETTE_MINIMALE_UM..=FOURCHETTE_MAXIMALE_UM).contains(&v)).count();
    let ecart_des_moyennes = moyenne(&biomasse).zip(moyenne(&pointes)).map(|(b, p)| ecart_relatif(b, p));
    let resume = [
        ("graines", mesures.len().to_string()),
        ("sans_fenetre", (mesures.len() - mesurees.len()).to_string()),
        ("longueur_um_moyenne", nombre(moyenne(&um), 1)),
        ("longueur_um_ecart_type", nombre(ecart_type(&um), 1)),
        ("longueur_um_minimum", nombre(minimum, 1)),
        ("longueur_um_maximum", nombre(maximum, 1)),
        ("dans_la_fourchette_35_682", dans.to_string()),
        ("taux_biomasse_moyen", nombre(moyenne(&biomasse), 4)),
        ("taux_pointes_moyen", nombre(moyenne(&pointes), 4)),
        ("ecart_des_taux_moyens", nombre(ecart_des_moyennes, 3)),
        ("ecart_moyen", nombre(moyenne(&ecarts), 3)),
        ("ecart_maximal", nombre(ecarts.iter().copied().reduce(f64::max), 3)),
        ("au_dessus_de_0_12", ecarts.iter().filter(|&&e| e > TOLERANCE).count().to_string()),
    ];
    for (nom, valeur) in resume {
        writeln!(texte, "{nom} {valeur}").unwrap();
    }
}
