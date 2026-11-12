//! La raideur d'une colonie qui pousse (étape 15, M3) : on joue la colonie, on en tire le réseau de poutres, on tient son
//! bord gauche et on pousse son bord droit.
//!
//! `cargo run --release --example colonie_raideur [-- <scénario> <graine> <pas> [<dossier>]]` joue le scénario (le chemin d'un
//! fichier de `scenarios/`) `<pas>` pas avec la graine donnée, puis imprime les nombres avec leur statut et, si un dossier
//! est donné, y écrit `etape-15-raideur.png`. Sans argument : la colonie de démonstration (une boîte de 30 × 20 cases, la
//! graine 42, 60 pas), qui traverse la boîte. Une erreur d'argument ou de calcul va sur la sortie d'erreur, le code de sortie
//! est 2 et rien n'est écrit. Les tests de l'étape 15 incluent ce fichier (`#[path]`) et appellent [`calculer`], [`rapport`],
//! [`pixels`] et [`image`] ; jamais le binaire.
//!
//! # Les nombres et leur statut
//!
//! **Aucune raideur de colonie n'est publiée** : `K` est celle du réseau de poutres que ce banc a simulé, avec des raideurs
//! d'hyphe données, et **elle dépend du choix des mors** (une colonne de cases par bord). Les raideurs d'hyphe viennent de
//! Brinkmann et coll. 2024 (*A. niger*, paroi seule) : `EI` = 18,3 µN·µm² est **lu à la source** ; `EA` = 22,9 µN est
//! **calculé** (le module de la flexion pris isotrope, fois l'aire de la paroi : une extrapolation, pas une mesure de
//! l'article) ; **une case = 7 µm** est un choix (M2), comme la règle de contact et les mors.
//!
//! # L'image
//!
//! La case `(x, y)` est un carré de `k × k` pixels (la ligne 0 en haut). Chaque poutre est tracée à sa place, colorée selon
//! l'énergie de déformation qu'elle porte (la racine de son rapport à la plus grande, du violet au jaune) ; le mors tenu
//! est un fond bleu, le mors poussé un fond rouge. Une colonie qui ne traverse pas est tracée en gris. Le style est celui
//! d'un contrôle, pas d'une vitrine.

use std::path::Path;
use std::process::ExitCode;

use mycelium_lab::image::ecrire_png;
use mycelium_lab::piece::Bord;
use mycelium_lab::reseau::{Contact, ErreurReseau, Hyphe, Origine, Poussee, Reseau, contacts, extraire, pousser};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

const USAGE: &str = "usage : colonie_raideur [<scénario> <graine> <pas> [<dossier>]]";

/// Une colonie qui traverse une petite boîte (30 × 20) en une soixantaine de pas, avec six anastomoses.
pub const DEMONSTRATION: &str = "boite 30 20\nspore 2 10\nsubstrat 1.0 0.2\ncroissance 1.0 7.5 30 20\ncouplage 0.1 0.002\n\
                                 anastomose 0.5 10\nduree 60\n";
pub const GRAINE_DEMONSTRATION: u64 = 42;
pub const PAS_DEMONSTRATION: u32 = 60;
/// Une case vaut 7 µm (choix de M2).
pub const MICROMETRES_PAR_CASE: f64 = 7.0;
/// `EI` de Brinkmann et coll. 2024, 18,3 µN·µm² (lu à la source), converti en µN·case² : divisé par 7².
pub const EI: f64 = 18.3 / (MICROMETRES_PAR_CASE * MICROMETRES_PAR_CASE);
/// `EA` en µN (calculé, voir le haut du fichier).
pub const EA: f64 = 22.9;
/// Le déplacement du mors poussé, en cases : l'essai est linéaire, `K` n'en dépend pas.
pub const DELTA: f64 = 1.0;

/// Ce que l'essai rend, avec de quoi dessiner.
pub struct Resultat {
    pub boite: (usize, usize),
    pub segments: usize,
    pub anastomoses: u64,
    pub contacts: Vec<Contact>,
    pub reseau: Reseau,
    pub poussee: Poussee,
}

/// Joue le scénario `texte` pendant `pas` pas avec `graine`, en tire le réseau et pousse le bord droit, le gauche tenu.
pub fn calculer(texte: &str, graine: u64, pas: u32) -> Result<Resultat, String> {
    let (reglages, milieu, tropismes, _) = lire_scenario_tropismes(texte).map_err(|e| format!("scénario : {e:?}"))?;
    let boite = (reglages.largeur, reglages.hauteur);
    let mut simulation = Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine)
        .map_err(|e| format!("scénario : {e:?}"))?;
    simulation.avancer_n(pas);
    let contacts = contacts(&simulation);
    let reseau = extraire(simulation.segments(), &contacts, Hyphe { ea: EA, ei: EI }).map_err(|e| format!("{e:?}"))?;
    let poussee = pousser(&reseau, boite, Bord::Gauche, Bord::Droite, DELTA).map_err(|e| match e {
        ErreurReseau::TropGrand => "colonie trop grande pour le solveur plein (voir INCONNUES_MAX)".to_string(),
        autre => format!("{autre:?}"),
    })?;
    let (segments, anastomoses) = (simulation.segments().len(), simulation.anastomoses());
    Ok(Resultat { boite, segments, anastomoses, contacts, reseau, poussee })
}

/// Les nombres, un par ligne, chacun avec son statut.
pub fn rapport(r: &Resultat) -> String {
    let ponts = r.reseau.origines.iter().filter(|o| matches!(o, Origine::Pont(_))).count();
    let mut texte = format!(
        "réseau : {} segments, {} nœuds, {} poutres dont {ponts} ponts pour {} contacts ; anastomoses du noyau : {}\n",
        r.segments,
        r.reseau.noeuds.len(),
        r.reseau.poutres.len(),
        r.contacts.len(),
        r.anastomoses
    );
    texte += &format!(
        "mors : {} nœuds tenus (colonne de gauche), {} poussés (colonne de droite) [choix : K en dépend]\n",
        r.poussee.tenus, r.poussee.pousses
    );
    if r.poussee.solution.is_none() {
        texte += "raideur K = 0 : la colonie ne touche pas les deux bords, aucun calcul [calculé]\n";
    } else {
        let k = r.poussee.raideur;
        texte += &format!(
            "raideur K = {k} µN par case [calculé : le réseau simulé, avec EA et EI donnés ; aucune raideur de colonie n'est publiée]\n"
        );
        texte +=
            &format!("raideur K = {} µN/µm, soit N/m [calculé ; une case = 7 µm : choix]\n", k / MICROMETRES_PAR_CASE);
        texte += &format!("énergie ½ K δ² = {} µN·case pour δ = {DELTA} case [calculé]\n", r.poussee.energie);
    }
    texte += &format!(
        "EI = {EI} µN·case² [18,3 µN·µm² : lu à la source (Brinkmann et coll. 2024) ; conversion à 7 µm par case : choix]\n"
    );
    texte += &format!(
        "EA = {EA} µN [calculé : le module de la flexion pris isotrope, une extrapolation, pas une mesure de l'article]\n"
    );
    texte += "règle de contact : la pointe est reliée au point le plus proche du segment visé [choix]\n";
    texte
}

pub const FOND: [u8; 3] = [16, 16, 24];
pub const GRIS: [u8; 3] = [110, 110, 110];
pub const MORS_TENU: [u8; 3] = [20, 40, 90];
pub const MORS_POUSSE: [u8; 3] = [100, 30, 30];
/// Les deux bouts de la rampe : une poutre sans énergie est violette, la plus chargée est jaune.
pub const VIOLET: [u8; 3] = [70, 20, 110];
pub const JAUNE: [u8; 3] = [255, 240, 90];

/// L'image de la colonie (voir le haut du fichier) : sa largeur, sa hauteur et ses pixels, trois octets chacun. Le côté
/// d'une case est de 12 pixels au plus, moins si l'image dépassait 490 000 octets (l'écrivain PNG ne compresse pas).
pub fn pixels(r: &Resultat) -> (usize, usize, Vec<u8>) {
    let (largeur, hauteur) = r.boite;
    let k = (1..=12).rev().find(|k| (hauteur * k) * (largeur * k * 3 + 1) <= 490_000).unwrap_or(1);
    let (w, h) = (largeur * k, hauteur * k);
    let mut pixels = Vec::with_capacity(w * h * 3);
    for _ in 0..h {
        for px in 0..w {
            let colonne = px / k;
            pixels.extend_from_slice(&if colonne == 0 {
                MORS_TENU
            } else if colonne == largeur - 1 {
                MORS_POUSSE
            } else {
                FOND
            });
        }
    }
    let plus_grande = r.poussee.energies.iter().fold(0.0_f64, |m, &e| m.max(e));
    for (i, p) in r.reseau.poutres.iter().enumerate() {
        let couleur = match r.poussee.energies.get(i) {
            Some(&e) if plus_grande > 0.0 => {
                let niveau = ((e / plus_grande).max(0.0).sqrt() * 255.0) as i32;
                [0, 1, 2].map(|c| (VIOLET[c] as i32 + (JAUNE[c] as i32 - VIOLET[c] as i32) * niveau / 255) as u8)
            }
            _ => GRIS,
        };
        let (a, b) = (&r.reseau.noeuds[p.a], &r.reseau.noeuds[p.b]);
        let (dx, dy) = ((b.x - a.x) * k as f64, (b.y - a.y) * k as f64);
        let pas = (dx * dx + dy * dy).sqrt().max(1.0) as usize * 2;
        for j in 0..=pas {
            let t = j as f64 / pas as f64;
            let (x, y) = ((a.x * k as f64 + t * dx) as usize, (a.y * k as f64 + t * dy) as usize);
            for (ix, iy) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                if ix < w && iy < h {
                    pixels[(iy * w + ix) * 3..(iy * w + ix) * 3 + 3].copy_from_slice(&couleur);
                }
            }
        }
    }
    (w, h, pixels)
}

/// Le PNG de [`pixels`].
pub fn image(r: &Resultat) -> Vec<u8> {
    let (largeur, hauteur, pixels) = pixels(r);
    ecrire_png(largeur as u32, hauteur as u32, &pixels)
}

fn lancer(args: &[String]) -> Result<(), String> {
    let (texte, graine, pas, dossier) = match args {
        [] => (DEMONSTRATION.to_string(), GRAINE_DEMONSTRATION, PAS_DEMONSTRATION, None),
        [scenario, graine, pas] | [scenario, graine, pas, _] => {
            let texte = std::fs::read_to_string(scenario).map_err(|e| format!("{scenario} : {e}"))?;
            let graine = graine.parse().map_err(|_| format!("graine illisible : {graine}"))?;
            let pas = pas.parse().map_err(|_| format!("pas illisible : {pas}"))?;
            (texte, graine, pas, args.get(3).map(|d| Path::new(d.as_str())))
        }
        _ => return Err("zéro, trois ou quatre arguments attendus".to_string()),
    };
    let resultat = calculer(&texte, graine, pas)?;
    if let Some(dossier) = dossier {
        std::fs::create_dir_all(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
        let fichier = dossier.join("etape-15-raideur.png");
        std::fs::write(&fichier, image(&resultat)).map_err(|e| format!("{} : {e}", fichier.display()))?;
    }
    print!("{}", rapport(&resultat));
    Ok(())
}

#[allow(dead_code)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match lancer(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}
