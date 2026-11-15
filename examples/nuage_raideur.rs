//! La raideur selon la densité (étape 17, M4) : le nuage de trois moules et de deux profils.
//!
//! `cargo run --release --example nuage_raideur [-- <dossier>]` calcule le nuage complet (six scénarios, 200 graines
//! chacun : une dizaine de minutes), imprime les comptes et les droites ajustées et, si un dossier est donné, y écrit
//! `nuage.txt` (une ligne par point, lisible plus tard par la page) et `nuage.png` (le nuage en échelle logarithmique et
//! ses droites, sans style). Une erreur va sur la sortie d'erreur et le code de sortie est 2. Les tests
//! de l'étape 17 incluent ce fichier (`#[path]`) et appellent ses fonctions sur de petits nuages ; jamais le binaire, et
//! jamais le nuage complet.
//!
//! # Les scénarios
//!
//! Les moules sont ceux de M1, les profils ceux de M2 : ils ne diffèrent que par l'intervalle de ramification (`dense`
//! 7,5 ; `rapide` 32). Chaque moule est une petite boîte dont la cavité touche la colonne de gauche et celle de droite
//! (les mors) : la **brique** est la référence, une cavité qui couvre toute la boîte (aucun mur) ; l'**anneau** n'a
//! qu'**une case** dans la colonne de chaque mors (un anneau touche le bord de sa boîte en un point) : la spore est dans
//! la première, et une colonie ne traverse que si elle a un nœud dans la seconde ; le **L** a une barre à gauche et une
//! barre en bas. Le plafond de 1 000 nœuds du solveur borne la densité par le haut ([`Raison::TropGrande`]), l'absence
//! de traversée par le bas.
//!
//! # Les nombres et leur statut
//!
//! **C'est un modèle à nous** : aucune pente de colonie n'est publiée. Les puissances de la littérature (1, 2, 3) sont
//! des repères, jamais des cibles ; rien ici n'est réglé pour s'en approcher. Les raideurs d'hyphe sont celles de
//! l'étape 15, et la raideur dépend du choix des mors (une colonne de cases par bord), comme à l'étape 15.

use std::path::Path;
use std::process::ExitCode;

use mycelium_lab::croissance::Segment;
use mycelium_lab::image::ecrire_png;
use mycelium_lab::mesures::logarithme;
use mycelium_lab::moule::developper;
use mycelium_lab::nuage::{Cavite, Ecarte, Nuage, Point, Raison, ajuster};
use mycelium_lab::piece::Bord;
use mycelium_lab::reseau::{Contact, ErreurReseau, Hyphe, contacts, extraire, pousser};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};

const USAGE: &str = "usage : nuage_raideur [<dossier>]";

/// `EA` et `EI` de l'étape 15 (µN et cases) : `EA` = 22,9 µN, `EI` = 18,3 µN·µm² à 7 µm la case.
pub const HYPHE: Hyphe = Hyphe { ea: 22.9, ei: 18.3 / 49.0 };

/// Le déplacement du mors poussé, en cases : l'essai est linéaire, `K` n'en dépend pas (comme à l'étape 15).
pub const DELTA: f64 = 1.0;

/// Lit une colonie (ses `segments` et ses `liens` d'anastomose) dans sa cavité : son point si elle traverse, sinon la
/// raison, jamais un nombre à la place. Le bord gauche est tenu, le droit poussé (`reseau::pousser`, comme à l'étape
/// 15) ; `(graine, pas)` ne sert qu'à étiqueter le point.
///
/// # Errors
///
/// [`Raison::NeTraversePas`] si la colonie n'a pas un nœud dans la colonne de chaque mors (`K = 0`) ;
/// [`Raison::TropGrande`] si elle traverse et dépasse le plafond du solveur ; [`Raison::Refus`] pour tout autre refus de
/// l'extraction ou de la poussée (avec son nom), et pour une densité qui n'existe pas ou qui est nulle.
pub fn lire(
    cavite: &Cavite,
    segments: &[Segment],
    liens: &[Contact],
    (graine, pas): (u64, u32),
    hyphe: Hyphe,
) -> Result<Point, Raison> {
    let refus = |e: ErreurReseau| Raison::Refus(format!("{e:?}"));
    let colonie = extraire(segments, liens, hyphe).map_err(refus)?;
    let boite = (cavite.largeur, cavite.hauteur);
    let poussee = match pousser(&colonie, boite, Bord::Gauche, Bord::Droite, DELTA) {
        Ok(poussee) => poussee,
        Err(ErreurReseau::TropGrand) => return Err(Raison::TropGrande),
        Err(autre) => return Err(refus(autre)),
    };
    if poussee.solution.is_none() {
        return Err(Raison::NeTraversePas);
    }
    match cavite.densite(segments) {
        Some(densite) if densite > 0.0 => {
            Ok(Point { graine, pas, densite, raideur: poussee.raideur, noeuds: colonie.noeuds.len() })
        }
        _ => Err(Raison::Refus("densité : elle n'existe pas dans cette cavité, ou elle est nulle".to_string())),
    }
}

/// [`lire`] sur une simulation, au pas où elle est : ses segments, ses anastomoses (`reseau::contacts`) et sa cavité.
pub fn lire_la_simulation(simulation: &Simulation, graine: u64, hyphe: Hyphe) -> Result<Point, Raison> {
    let liens = contacts(simulation);
    lire(&Cavite::de(simulation), simulation.segments(), &liens, (graine, simulation.temps()), hyphe)
}

/// Pourquoi un nuage n'a pas pu être calculé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurNuage {
    /// Le scénario est refusé (le texte dit pourquoi).
    Scenario(String),
    /// Les pas ne sont pas strictement croissants.
    PasNonCroissants,
}

/// Le nuage d'un scénario (son texte, avec sa ligne `moule` éventuelle) : pour chaque graine de `graines`, dans l'ordre,
/// une simulation neuve avancée jusqu'à chaque pas de `pas` (strictement croissants), une colonie lue à chacun. Ce que
/// donne une graine ne dépend pas des autres graines.
///
/// # Errors
///
/// [`ErreurNuage::PasNonCroissants`] si `pas` ne croît pas strictement, [`ErreurNuage::Scenario`] si le scénario est
/// refusé.
pub fn nuage(texte: &str, graines: &[u64], pas: &[u32], hyphe: Hyphe) -> Result<Nuage, ErreurNuage> {
    if pas.windows(2).any(|p| p[0] >= p[1]) {
        return Err(ErreurNuage::PasNonCroissants);
    }
    let refus = |e: &dyn std::fmt::Debug| ErreurNuage::Scenario(format!("{e:?}"));
    let texte = developper(texte).map_err(|e| refus(&e))?;
    let mut nuage = Nuage::default();
    for &graine in graines {
        let (reglages, milieu, tropismes, _) = lire_scenario_tropismes(&texte).map_err(|e| refus(&e))?;
        let mut simulation =
            Simulation::nouvelle_avec_tropismes(reglages, milieu, tropismes, graine).map_err(|e| refus(&e))?;
        for &jusqu_au_pas in pas {
            simulation.avancer_n(jusqu_au_pas - simulation.temps());
            match lire_la_simulation(&simulation, graine, hyphe) {
                Ok(point) => nuage.points.push(point),
                Err(raison) => nuage.ecartes.push(Ecarte { graine, pas: jusqu_au_pas, raison }),
            }
        }
    }
    Ok(nuage)
}

/// Un moule : son nom, sa boîte (largeur, hauteur), la case de la spore et la fin de sa ligne `moule`.
pub struct Moule {
    pub nom: &'static str,
    pub boite: (usize, usize),
    pub spore: (usize, usize),
    pub ligne: &'static str,
}

/// Les trois moules. La spore de l'anneau est dans la seule case de l'anneau que compte la colonne du mors tenu.
pub const MOULES: [Moule; 3] = [
    Moule { nom: "brique", boite: (30, 20), spore: (2, 10), ligne: "brique 0 0 29 19" },
    Moule { nom: "anneau", boite: (29, 29), spore: (0, 14), ligne: "anneau 14 14 14 8" },
    Moule { nom: "l", boite: (30, 30), spore: (2, 24), ligne: "l 0 0 29 29 10" },
];

/// Un profil : son nom, son intervalle de ramification et la fenêtre de ses pas (du premier au dernier, de 10 en 10).
pub struct Profil {
    pub nom: &'static str,
    pub intervalle: &'static str,
    pub premier_pas: u32,
    pub dernier_pas: u32,
}

/// Les deux profils de M2 et leurs fenêtres de pas : un choix de coût, pas des bornes naturelles (voir `MODELE.md`).
pub const PROFILS: [Profil; 2] = [
    Profil { nom: "dense", intervalle: "7.5", premier_pas: 40, dernier_pas: 100 },
    Profil { nom: "rapide", intervalle: "32.0", premier_pas: 60, dernier_pas: 150 },
];

/// Le texte du scénario d'un moule et d'un profil (les réglages des moules de M1, `thigmotropisme 35` compris).
pub fn scenario(m: &Moule, p: &Profil) -> String {
    format!(
        "boite {} {}\nspore {} {}\nsubstrat 1.0 0.2\ncroissance 1.0 {} 30 20\ncouplage 0.1 0.002\nanastomose 0.5 10\n\
         thigmotropisme 35\nduree {}\nmoule {}\n",
        m.boite.0, m.boite.1, m.spore.0, m.spore.1, p.intervalle, p.dernier_pas, m.ligne
    )
}

/// Les pas d'un profil : de son premier à son dernier, de 10 en 10.
pub fn pas(p: &Profil) -> Vec<u32> {
    (p.premier_pas..=p.dernier_pas).step_by(10).collect()
}

/// Les graines du nuage complet : 1 à 200.
pub fn graines() -> Vec<u64> {
    (1..=200).collect()
}

/// Le nuage d'un moule et d'un profil.
pub struct Calcul {
    pub moule: &'static str,
    pub profil: &'static str,
    pub nuage: Nuage,
}

/// Les six nuages (les trois moules, chacun avec ses deux profils, le dense d'abord) pour ces graines ; les pas sont
/// ceux des profils.
///
/// # Errors
///
/// Le nom du moule et du profil, et l'erreur de [`nuage`].
pub fn calculer(graines: &[u64]) -> Result<Vec<Calcul>, String> {
    let mut calculs = Vec::new();
    for m in &MOULES {
        for p in &PROFILS {
            let nuage =
                nuage(&scenario(m, p), graines, &pas(p), HYPHE).map_err(|e| format!("{} {} : {e:?}", m.nom, p.nom))?;
            calculs.push(Calcul { moule: m.nom, profil: p.nom, nuage });
        }
    }
    Ok(calculs)
}

/// Les paires `(ρ, K)` d'un moule, tous ses profils réunis, dans l'ordre des calculs.
pub fn paires_du_moule(calculs: &[Calcul], moule: &str) -> Vec<(f64, f64)> {
    calculs.iter().filter(|c| c.moule == moule).flat_map(|c| c.nuage.paires()).collect()
}

/// La droite de ces paires, écrite en une ligne (ou pourquoi elle n'existe pas).
fn droite(paires: &[(f64, f64)]) -> String {
    match ajuster(paires) {
        Ok(d) => format!("pente {:e} ordonnee {:e} r2 {:e} points {}", d.pente, d.ordonnee, d.r2, d.points),
        Err(e) => format!("indisponible {e:?}"),
    }
}

/// Le fichier du nuage : trois lignes de commentaire (`#`), puis une ligne par point,
/// `moule profil graine pas densite raideur noeuds` (sept mots séparés par une espace ; les nombres en notation `{:e}`,
/// qui se relit à l'identique), puis, en commentaire, pour chaque calcul les comptes des colonies écartées et la droite
/// ajustée, et pour chaque moule la droite de ses profils réunis.
pub fn fichier(calculs: &[Calcul]) -> String {
    let mut t = String::from(
        "# Nuage de l'étape 17 (M4) : la raideur selon la densité. Un modèle à nous, pas une reproduction.\n\
         # densité en cases^-1 (longueur d'hyphe par case libre), raideur en µN par case ; un point par colonie qui traverse.\n\
         # colonnes : moule profil graine pas densite raideur noeuds\n",
    );
    for c in calculs {
        for p in &c.nuage.points {
            t += &format!(
                "{} {} {} {} {:e} {:e} {}\n",
                c.moule, c.profil, p.graine, p.pas, p.densite, p.raideur, p.noeuds
            );
        }
    }
    for c in calculs {
        let n = c.nuage.comptes();
        t += &format!(
            "# ecartes {} {} : ne_traversent_pas {} trop_grandes {} autres_refus {}\n",
            c.moule, c.profil, n.ne_traversent_pas, n.trop_grandes, n.refus
        );
        t += &format!("# droite {} {} : {}\n", c.moule, c.profil, droite(&c.nuage.paires()));
    }
    for m in &MOULES {
        t += &format!("# droite {} reunis : {}\n", m.nom, droite(&paires_du_moule(calculs, m.nom)));
    }
    t
}

pub const LARGEUR: usize = 640;
pub const HAUTEUR: usize = 480;
/// La marge, en pixels, entre le bord de l'image et le cadre du tracé.
pub const MARGE: usize = 40;
pub const FOND: [u8; 3] = [16, 16, 24];
pub const CADRE: [u8; 3] = [110, 110, 110];
/// Une couleur par moule, dans l'ordre de [`MOULES`] : brique, anneau, L.
pub const COULEURS: [[u8; 3]; 3] = [[90, 150, 255], [90, 220, 120], [255, 170, 60]];

/// L'image du nuage : le logarithme de la raideur (vers le haut) contre celui de la densité (vers la droite), entre les
/// valeurs extrêmes de tous les points, qui tombent sur le cadre. Un carré de 3 pixels par point, une couleur par moule, et
/// la droite ajustée de chaque moule (profils réunis) entre ses densités extrêmes. Largeur, hauteur et pixels, trois
/// octets chacun.
pub fn pixels(calculs: &[Calcul]) -> (usize, usize, Vec<u8>) {
    let mut image = vec![0u8; LARGEUR * HAUTEUR * 3];
    for p in image.chunks_mut(3) {
        p.copy_from_slice(&FOND);
    }
    let tous: Vec<(f64, f64)> = MOULES.iter().flat_map(|m| paires_du_moule(calculs, m.nom)).collect();
    let (zone_x, zone_y) = ((LARGEUR - 2 * MARGE) as f64, (HAUTEUR - 2 * MARGE) as f64);
    let borne = |f: fn(&(f64, f64)) -> f64| {
        let v: Vec<f64> = tous.iter().map(|p| logarithme(f(p))).collect();
        (v.iter().fold(f64::INFINITY, |a, &b| a.min(b)), v.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)))
    };
    let ((x_min, x_max), (y_min, y_max)) = (borne(|p| p.0), borne(|p| p.1));
    // Une seule valeur (ou aucun point) : une étendue de 1, pour ne pas diviser par zéro.
    let etendue = |a: f64, b: f64| if b > a { b - a } else { 1.0 };
    let place = |lx: f64, ly: f64| {
        let x = MARGE as f64 + (lx - x_min) / etendue(x_min, x_max) * zone_x;
        let y = (HAUTEUR - MARGE) as f64 - (ly - y_min) / etendue(y_min, y_max) * zone_y;
        (x, y)
    };
    // Un carré de `cote` pixels centré sur `(x, y)` ; ce qui sort de l'image n'est pas tracé.
    let mut poser = |x: f64, y: f64, couleur: [u8; 3], cote: usize| {
        for (dx, dy) in (0..cote).flat_map(|i| (0..cote).map(move |j| (i, j))) {
            let (px, py) =
                (x as isize + dx as isize - (cote / 2) as isize, y as isize + dy as isize - (cote / 2) as isize);
            if px >= 0 && py >= 0 && (px as usize) < LARGEUR && (py as usize) < HAUTEUR {
                let i = (py as usize * LARGEUR + px as usize) * 3;
                image[i..i + 3].copy_from_slice(&couleur);
            }
        }
    };
    for t in 0..=(LARGEUR - 2 * MARGE) {
        for y in [MARGE, HAUTEUR - MARGE] {
            poser((MARGE + t) as f64, y as f64, CADRE, 1);
        }
    }
    for t in 0..=(HAUTEUR - 2 * MARGE) {
        for x in [MARGE, LARGEUR - MARGE] {
            poser(x as f64, (MARGE + t) as f64, CADRE, 1);
        }
    }
    for (m, couleur) in MOULES.iter().zip(COULEURS) {
        let paires = paires_du_moule(calculs, m.nom);
        for &(rho, k) in &paires {
            let (x, y) = place(logarithme(rho), logarithme(k));
            poser(x, y, couleur, 3);
        }
        if let (Ok(d), Some(bas), Some(haut)) =
            (ajuster(&paires), paires.iter().map(|p| p.0).reduce(f64::min), paires.iter().map(|p| p.0).reduce(f64::max))
        {
            let (a, b) = (logarithme(bas), logarithme(haut));
            for j in 0..=200 {
                let lx = a + (b - a) * j as f64 / 200.0;
                let (x, y) = place(lx, d.ordonnee + d.pente * lx);
                poser(x, y, couleur, 1);
            }
        }
    }
    (LARGEUR, HAUTEUR, image)
}

/// Le PNG de [`pixels`].
pub fn image(calculs: &[Calcul]) -> Vec<u8> {
    let (largeur, hauteur, pixels) = pixels(calculs);
    ecrire_png(largeur as u32, hauteur as u32, &pixels)
}

/// Écrit `nuage.txt` ([`fichier`]) et `nuage.png` ([`image`]) dans `dossier`, créé s'il le faut.
///
/// # Errors
///
/// Le nom de ce qui n'a pas pu être écrit, et pourquoi.
pub fn ecrire(dossier: &Path, calculs: &[Calcul]) -> Result<(), String> {
    std::fs::create_dir_all(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    for (nom, octets) in [("nuage.txt", fichier(calculs).into_bytes()), ("nuage.png", image(calculs))] {
        std::fs::write(dossier.join(nom), octets).map_err(|e| format!("{nom} : {e}"))?;
    }
    Ok(())
}

/// Ce que fait le binaire : le nuage complet, écrit dans le dossier s'il est donné, puis ses comptes et ses droites
/// imprimés. Plus d'un argument est refusé avant tout calcul.
pub fn lancer(args: &[String]) -> Result<(), String> {
    let dossier = match args {
        [] => None,
        [dossier] => Some(Path::new(dossier.as_str())),
        _ => return Err("zéro ou un argument attendu".to_string()),
    };
    let calculs = calculer(&graines())?;
    if let Some(dossier) = dossier {
        ecrire(dossier, &calculs)?;
    }
    // Les comptes et les droites : les lignes de commentaire de la fin du fichier.
    for ligne in fichier(&calculs).lines().filter(|l| l.starts_with("# ecartes") || l.starts_with("# droite")) {
        println!("{ligne}");
    }
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
