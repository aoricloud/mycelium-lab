//! Tests d'acceptation de l'étape 24 : l'anastomose en volume.
//!
//! Ce qui est vérifié :
//! - les empreintes de `colonie-anastomose`, graine 1 : au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis
//!   à la fin de sa durée (l'empreinte de son fichier de `references/`), et les comptes de la fin ;
//! - sans la ligne `anastomose`, rien ne change : `colonie-cube`, `colonie-goutte`, `colonie-chimio` et
//!   `colonie-mur` gardent l'empreinte de leur fichier ; et le témoin de `colonie-anastomose` sans sa ligne diffère
//!   seulement de la queue de 20 octets, tant que la scène ne s'est pas anastomosée ;
//! - l'écriture canonique de `colonie-anastomose` : la longueur du contrat et la queue du contrat ;
//! - le rejeu sans index, sur `colonie-anastomose` graines 1 à 3, puis sur une forêt de piliers graines 1 et 2 : les
//!   anastomoses observées pas à pas sont celles que redonne un parcours de tous les segments, distance point-segment
//!   écrite ici, et un obstacle écarte un candidat sans arrêter la recherche ;
//! - les invariants et le déterminisme sur `colonie-anastomose`, graines 1 à 3 ;
//! - les cas faits à la main : le retour d'une branche sur le premier segment de sa mère, la pointe seule qui ne
//!   s'anastomose jamais avec elle-même ;
//! - les erreurs de lecture de la ligne `anastomose`, l'ordre des refus de `Colonie3d::nouvelle` et les réglages
//!   limites ;
//! - `Index3d` seul : la distance et le point le plus proche en 3D, les bornes du cube fermé et un cube plat, les
//!   paniques de `ajouter`, les trois conditions du candidat, les bornes du rayon et les points non finis, l'index vide,
//!   l'égalité de distance, un point hors du cube et un long segment ;
//! - `chercher_si` : un candidat écarté laisse trouver le suivant, `retenu` ne voit que des candidats, et une plaque
//!   bloquée écarte le plus proche derrière elle.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement.

use std::cell::RefCell;
use std::path::Path;

use mycelium_lab::anastomose::Reglage;
use mycelium_lab::anastomose3d::{Index3d, distance_au_segment_3d, point_le_plus_proche_3d};
use mycelium_lab::colonie3d::{Colonie3d, ErreurColonie3d, ScenarioColonie3d, Segment3d, lire_scenario_colonie3d};
use mycelium_lab::contact3d::{Traversee3d, traverser_3d};
use mycelium_lab::cube::Bloc;
use mycelium_lab::substrat3d::Substrat3d;

/// Le petit scénario valide des erreurs de lecture, une directive par ligne, dans un cube de 8 cases.
const LIGNES: [&str; 7] = [
    "cube 8 8 8",
    "substrat 1.0 0.1",
    "nourriture 0 0 0 1.0",
    "spore 4 4 4",
    "croissance 0.5 6 30 20",
    "couplage 0.1 0.5",
    "duree 10",
];

/// Le scénario du retour d'une branche sur sa mère, sans la ligne `anastomose` : une pointe droite d'un pas de 0,5
/// qui se ramifie à chaque pas d'un demi-tour, donc vers l'arrière.
const RETOUR_SANS_ANASTOMOSE: &str = "\
cube 16 16 16
substrat 1.0 0.0
spore 8 8 8
croissance 1.0 1 180 0
couplage 1.0 0.0
duree 10
";

/// Le scénario de la pointe seule, sans la ligne `anastomose` : une pointe qui ne se ramifie jamais.
const SEULE_SANS_ANASTOMOSE: &str = "\
cube 16 16 16
substrat 1.0 0.0
spore 8 8 8
croissance 1.0 infini 30 0
couplage 1.0 0.0
duree 10
";

/// Les coordonnées des piliers de la forêt : 2, 5, 8, ..., 29.
const PILIERS: [u32; 10] = [2, 5, 8, 11, 14, 17, 20, 23, 26, 29];

/// Le texte d'un fichier de `scenarios/`.
fn texte_du_fichier(nom: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
}

/// Le scénario d'un fichier de `scenarios/`.
fn scenario(nom: &str) -> ScenarioColonie3d {
    lire_scenario_colonie3d(&texte_du_fichier(nom)).unwrap_or_else(|e| panic!("{nom} : refus inattendu {e:?}"))
}

/// L'empreinte de référence d'un fichier de `references/` : 64 chiffres hexadécimaux minuscules, une ligne.
fn empreinte_de_reference(fichier: &str) -> String {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("references").join(fichier);
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
    assert_eq!(texte.lines().count(), 1, "{fichier} : une seule ligne");
    let empreinte = texte.trim();
    assert_eq!(empreinte.len(), 64, "{fichier} : 64 chiffres hexadécimaux");
    assert!(
        empreinte.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "{fichier} : des chiffres hexadécimaux minuscules"
    );
    empreinte.to_string()
}

/// Une erreur de lecture attendue d'un texte.
fn erreur(texte: &str) -> ErreurColonie3d {
    match lire_scenario_colonie3d(texte) {
        Ok(scenario) => panic!("ce texte devait être refusé, il donne {scenario:?}"),
        Err(erreur) => erreur,
    }
}

/// Une colonie construite depuis un texte de scénario, à la graine `graine`, au temps 0.
fn colonie_du_texte(texte: &str, graine: u64) -> Colonie3d {
    let scenario = lire_scenario_colonie3d(texte).unwrap_or_else(|e| panic!("refus inattendu {e:?}"));
    Colonie3d::nouvelle(&scenario, graine).unwrap_or_else(|e| panic!("construction refusée {e:?}"))
}

/// Une colonie de la scène `nom`, à la graine `graine`, au temps 0.
fn colonie(nom: &str, graine: u64) -> Colonie3d {
    colonie_du_texte(&texte_du_fichier(nom), graine)
}

/// Une colonie de la scène `nom`, à la graine `graine`, avancée jusqu'à sa durée.
fn colonie_finie(nom: &str, graine: u64) -> Colonie3d {
    let mut colonie = colonie(nom, graine);
    colonie.avancer_n(colonie.duree());
    colonie
}

/// Vrai si `a` et `b` sont égaux à `tolerance` près, en valeur relative.
fn egal_relatif(a: f64, b: f64, tolerance: f64) -> bool {
    (a / b - 1.0).abs() <= tolerance
}

/// Un segment d'apex `apex`, laissé au pas `temps`, du point `debut` au point `fin`.
fn segment3(apex: u32, temps: u32, debut: (f64, f64, f64), fin: (f64, f64, f64)) -> Segment3d {
    Segment3d { apex, temps, x0: debut.0, y0: debut.1, z0: debut.2, x1: fin.0, y1: fin.1, z1: fin.2 }
}

/// Le point du segment `segment` le plus proche du point `point`, extrémités comprises : la règle du contrat, écrite
/// dans le test pour refaire les anastomoses sans l'index.
fn point_du_contrat(point: (f64, f64, f64), segment: &Segment3d) -> (f64, f64, f64) {
    let (x, y, z) = point;
    let dx = segment.x1 - segment.x0;
    let dy = segment.y1 - segment.y0;
    let dz = segment.z1 - segment.z0;
    let carre = (dx * dx + dy * dy) + dz * dz;
    if carre == 0.0 {
        return (segment.x0, segment.y0, segment.z0);
    }
    let t = ((((x - segment.x0) * dx + (y - segment.y0) * dy) + (z - segment.z0) * dz) / carre).clamp(0.0, 1.0);
    (segment.x0 + t * dx, segment.y0 + t * dy, segment.z0 + t * dz)
}

/// La distance du point `point` au segment `segment` : la règle du contrat, écrite dans le test.
fn distance_du_contrat(point: (f64, f64, f64), segment: &Segment3d) -> f64 {
    let (px, py, pz) = point_du_contrat(point, segment);
    let (x, y, z) = point;
    let ex = x - px;
    let ey = y - py;
    let ez = z - pz;
    ((ex * ex + ey * ey) + ez * ez).sqrt()
}

/// Le petit scénario valide suivi de lignes de plus : la première est la huitième.
fn avec_lignes_en_plus(lignes: &[&str]) -> String {
    format!("{}\n{}\n", LIGNES.join("\n"), lignes.join("\n"))
}

/// Le texte de `colonie-anastomose.txt` sans sa ligne `anastomose 0.5 10`.
fn temoin_sans_anastomose() -> String {
    texte_du_fichier("colonie-anastomose.txt")
        .lines()
        .filter(|ligne| ligne.trim() != "anastomose 0.5 10")
        .collect::<Vec<_>>()
        .join("\n")
}

/// Le texte de la forêt de piliers : `colonie-anastomose.txt` à `anastomose 1.5 10`, plus mille cases bloquées
/// (une par point de la grille 2, 5, ..., 29 sur les trois axes) et `thigmotropisme 35`.
fn foret_de_piliers() -> String {
    let mut lignes: Vec<String> = texte_du_fichier("colonie-anastomose.txt")
        .lines()
        .map(
            |ligne| {
                if ligne.trim() == "anastomose 0.5 10" { "anastomose 1.5 10".to_string() } else { ligne.to_string() }
            },
        )
        .collect();
    let mut ajoutees = 0;
    for x in PILIERS {
        for y in PILIERS {
            for z in PILIERS {
                lignes.push(format!("obstacle {x} {y} {z} {x} {y} {z}"));
                ajoutees += 1;
            }
        }
    }
    assert_eq!(ajoutees, 1000, "la forêt compte mille cases bloquées");
    lignes.push("thigmotropisme 35".to_string());
    lignes.join("\n")
}

/// Le scénario du retour, avec la ligne `anastomose 0.25 <delai>` ou sans elle.
fn retour(delai: Option<u32>) -> String {
    match delai {
        Some(delai) => format!("{RETOUR_SANS_ANASTOMOSE}anastomose 0.25 {delai}\n"),
        None => RETOUR_SANS_ANASTOMOSE.to_string(),
    }
}

/// Une anastomose vue par le rejeu sans index.
struct Rejeu {
    /// Les couples `(apex, pas)` des anastomoses, dans l'ordre des segments.
    liste: Vec<(u32, u32)>,
    /// Le nombre de segments dont un candidat géométrique a été écarté par un obstacle.
    prevenus: usize,
    /// Le nombre d'anastomoses faites avec un candidat plus loin qu'un candidat écarté.
    plus_loin: usize,
}

/// Refait la règle du contrat sans index : pour chaque segment `n`, dans l'ordre, cherche à son arrivée parmi les
/// segments `m < n` un candidat (autre apex, déposé au moins `delai` pas plus tôt, à distance `rayon` au plus), du
/// plus proche au plus petit numéro à distance égale. Avec `substrat`, un candidat dont le point le plus proche, borné
/// au cube fermé, est séparé de l'arrivée par une case bloquée est écarté et la recherche continue.
fn rejeu_par_le_contrat(
    segments: &[Segment3d],
    reglage: Reglage,
    substrat: Option<&Substrat3d>,
    cube: (f64, f64, f64),
) -> Rejeu {
    let mut rejeu = Rejeu { liste: Vec::new(), prevenus: 0, plus_loin: 0 };
    for (n, segment) in segments.iter().enumerate() {
        let arrivee = (segment.x1, segment.y1, segment.z1);
        let mut plus_proche: Option<f64> = None;
        let mut retenue: Option<f64> = None;
        for autre in &segments[..n] {
            if autre.apex == segment.apex {
                continue;
            }
            if u64::from(segment.temps) < u64::from(autre.temps) + u64::from(reglage.delai) {
                continue;
            }
            let distance = distance_du_contrat(arrivee, autre);
            if distance > reglage.rayon {
                continue;
            }
            if plus_proche.is_none_or(|vue| distance < vue) {
                plus_proche = Some(distance);
            }
            let accepte = match substrat {
                None => true,
                Some(substrat) => {
                    let q = point_du_contrat(arrivee, autre);
                    let q = (q.0.clamp(0.0, cube.0), q.1.clamp(0.0, cube.1), q.2.clamp(0.0, cube.2));
                    traverser_3d(substrat, arrivee, q) == Traversee3d::Libre
                }
            };
            if accepte && retenue.is_none_or(|vue| distance < vue) {
                retenue = Some(distance);
            }
        }
        if let Some(distance) = retenue {
            rejeu.liste.push((segment.apex, segment.temps));
            if plus_proche.is_some_and(|proche| distance > proche) {
                rejeu.plus_loin += 1;
            }
        } else if plus_proche.is_some() {
            rejeu.prevenus += 1;
        }
    }
    rejeu
}

/// Joue la colonie pas par pas et rend les couples `(apex, pas)` des anastomoses : un apex actif au début du pas qui
/// laisse un segment pendant ce pas et finit le pas inactif s'est anastomosé (sans obstacle, il n'a pas d'autre fin
/// possible). Vérifie aussi que `nb_anastomoses` croît d'autant à chaque pas.
fn anastomoses_observees(colonie: &mut Colonie3d) -> Vec<(u32, u32)> {
    let duree = colonie.duree();
    let mut liste = Vec::new();
    for _ in 0..duree {
        let actifs: Vec<u32> = colonie.apex().iter().filter(|apex| apex.actif).map(|apex| apex.id).collect();
        let avant = colonie.segments().len();
        let compteur_avant = colonie.nb_anastomoses();
        colonie.avancer();
        let temps = colonie.temps();
        let nouveaux = &colonie.segments()[avant..];
        let mut vues = 0u64;
        for apex in colonie.apex() {
            let a_laisse = nouveaux.iter().any(|s| s.apex == apex.id && s.temps == temps);
            if actifs.contains(&apex.id) && !apex.actif && a_laisse {
                liste.push((apex.id, temps));
                vues += 1;
            }
        }
        assert_eq!(
            colonie.nb_anastomoses() - compteur_avant,
            vues,
            "pas {temps} : le compteur croît d'autant que d'apex anastomosés"
        );
    }
    liste
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn colonie_anastomose_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut colonie = colonie("colonie-anastomose.txt", 1);
    assert_eq!(
        colonie.empreinte(),
        "d7909ccffcdd495e17d3c6dc88de375f14e55d67e6cf80f20d954d5355be582d",
        "colonie-anastomose : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-anastomose : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "bace4dc69d02c1bb61a3eda15ab10a7a66f9c20b06271dfe65986412a44464c7",
        "colonie-anastomose : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-anastomose : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "8e461b0548a44d13e6386ed2aadf42e75c94d211f817800d3c5a5f1360c875f6",
        "colonie-anastomose : l'empreinte après 10 pas"
    );
    let duree = colonie.duree();
    colonie.avancer_n(duree - 10);
    assert_eq!(colonie.temps(), duree, "colonie-anastomose : le temps est la durée");
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-anastomose.empreinte"),
        "colonie-anastomose : l'empreinte de son fichier de références"
    );
    assert_eq!(colonie.segments().len(), 12510, "colonie-anastomose : les segments de la fin");
    assert_eq!(colonie.nb_ramifications(), 2131, "colonie-anastomose : les ramifications de la fin");
    assert_eq!(colonie.nb_anastomoses(), 86, "colonie-anastomose : les anastomoses de la fin");
    assert_eq!(colonie.pointes(), 2046, "colonie-anastomose : les pointes actives de la fin");
    assert_eq!(colonie.apex().len(), 2132, "colonie-anastomose : les apex de la fin");
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn sans_la_ligne_anastomose_rien_ne_change() {
    for (nom, fichier) in [
        ("colonie-cube.txt", "colonie-cube.empreinte"),
        ("colonie-goutte.txt", "colonie-goutte.empreinte"),
        ("colonie-chimio.txt", "colonie-chimio.empreinte"),
        ("colonie-mur.txt", "colonie-mur.empreinte"),
    ] {
        let colonie = colonie_finie(nom, 1);
        assert_eq!(colonie.temps(), colonie.duree(), "{nom} : le temps est la durée");
        assert!(colonie.scenario().anastomose.is_none(), "{nom} : pas d'anastomose");
        assert_eq!(colonie.empreinte(), empreinte_de_reference(fichier), "{nom} : l'empreinte de son fichier");
    }

    // Le témoin : sans sa ligne, la scène s'écrit comme le témoin, plus la queue de 20 octets, tant qu'aucune
    // anastomose n'a eu lieu. La scène finit par s'anastomoser : le témoin reste alors dix pas au moins.
    let mut avec = colonie("colonie-anastomose.txt", 1);
    let mut temoin = colonie_du_texte(&temoin_sans_anastomose(), 1);
    assert!(temoin.scenario().anastomose.is_none(), "le témoin est sans anastomose");
    let mut pas_verifies = 0usize;
    while avec.nb_anastomoses() == 0 && avec.temps() < avec.duree() {
        avec.avancer();
        temoin.avancer();
        if avec.nb_anastomoses() == 0 {
            let octets = avec.ecrire_canonique();
            let attendu = temoin.ecrire_canonique();
            assert_eq!(
                octets.len(),
                attendu.len() + 20,
                "pas {} : la queue de l'anastomose fait 20 octets",
                avec.temps()
            );
            assert_eq!(&octets[..attendu.len()], &attendu[..], "pas {} : le témoin, puis la queue", avec.temps());
            pas_verifies += 1;
        }
    }
    assert!(pas_verifies > 0, "au moins un pas sans anastomose a été comparé");
    assert!(avec.nb_anastomoses() > 0, "la scène finit par s'anastomoser, sinon le témoin ne dit rien");
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_ecriture_canonique_de_colonie_anastomose_a_la_longueur_du_contrat() {
    let colonie = colonie_finie("colonie-anastomose.txt", 1);
    let reglage = colonie.scenario().anastomose.expect("colonie-anastomose a une ligne anastomose");
    let apex = colonie.apex().len();
    let segments = colonie.segments().len();
    let cases = 32 * 32 * 32;
    // 4 (temps) + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases, plus 20 pour l'anastomose.
    let longueur = 4 + 8 + 56 * segments + 8 + 53 * apex + 8 + 24 + 8 * cases + 20;
    let octets = colonie.ecrire_canonique();
    assert_eq!(octets.len(), longueur, "la longueur du contrat, queue de l'anastomose comprise");

    let queue = &octets[octets.len() - 20..];
    assert_eq!(&queue[..8], &reglage.rayon.to_bits().to_le_bytes()[..], "les 8 octets du rayon, en petit-boutiste");
    assert_eq!(&queue[8..12], &reglage.delai.to_le_bytes()[..], "les 4 octets du délai, en petit-boutiste");
    assert_eq!(
        &queue[12..],
        &colonie.nb_anastomoses().to_le_bytes()[..],
        "les 8 octets du nombre d'anastomoses, en petit-boutiste"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn le_rejeu_sans_index_redonne_les_memes_anastomoses() {
    let scenario = scenario("colonie-anastomose.txt");
    let reglage = scenario.anastomose.expect("colonie-anastomose a une ligne anastomose");
    assert!(scenario.obstacles.is_empty(), "colonie-anastomose n'a pas d'obstacle");

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let observees = anastomoses_observees(&mut colonie);
        assert!(!colonie.segments().is_empty(), "graine {graine} : des segments parcourus");
        let rejeu = rejeu_par_le_contrat(colonie.segments(), reglage, None, (32.0, 32.0, 32.0));
        assert_eq!(
            rejeu.liste, observees,
            "graine {graine} : le parcours de tous les segments redonne les anastomoses"
        );
        if graine == 1 {
            assert!(!observees.is_empty(), "graine 1 : des anastomoses à vérifier");
            assert_eq!(observees.len(), 86, "graine 1 : les 86 anastomoses de la fin");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn le_rejeu_avec_obstacles_ecarte_les_candidats_separes() {
    let scenario = lire_scenario_colonie3d(&foret_de_piliers()).expect("la forêt de piliers se lit");
    let reglage = scenario.anastomose.expect("la forêt a une ligne anastomose");
    assert_eq!(scenario.obstacles.len(), 1000, "la forêt compte mille cases bloquées");
    assert_eq!(scenario.thigmotropisme, 35, "la forêt a thigmotropisme 35");

    for graine in 1..=2u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("la forêt se construit");
        let observees = anastomoses_observees(&mut colonie);
        let rejeu = rejeu_par_le_contrat(colonie.segments(), reglage, Some(colonie.substrat()), (32.0, 32.0, 32.0));
        assert_eq!(
            rejeu.liste, observees,
            "graine {graine} : le parcours de tous les segments redonne les anastomoses"
        );
        assert!(!observees.is_empty(), "graine {graine} : des anastomoses à vérifier");
        if graine == 1 {
            assert!(rejeu.prevenus > 0, "graine 1 : au moins une anastomose empêchée par un obstacle");
            assert!(
                rejeu.plus_loin > 0,
                "graine 1 : au moins une anastomose faite avec un candidat plus loin qu'un candidat écarté"
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn colonie_anastomose_respecte_ses_invariants_et_est_deterministe() {
    let scenario = scenario("colonie-anastomose.txt");
    let duree = scenario.duree;

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        let masse_initiale = colonie.substrat().masse_totale();
        colonie.avancer_n(duree);
        assert_eq!(colonie.temps(), duree, "graine {graine} : le temps est la durée");
        assert_eq!(
            colonie.apex().len() as u64,
            1 + colonie.nb_ramifications(),
            "graine {graine} : un apex pour la spore, plus un par ramification"
        );
        assert_eq!(
            colonie.pointes() as u64 + colonie.nb_anastomoses(),
            colonie.apex().len() as u64,
            "graine {graine} : sans obstacle, pointes + anastomoses = apex"
        );
        for apex in colonie.apex() {
            if !apex.actif {
                assert!(apex.memoire.is_none(), "graine {graine}, apex {} inactif : il n'a pas de mémoire", apex.id);
            }
        }

        let mut consomme = 0.0;
        for segment in colonie.segments() {
            consomme += scenario.cout * segment.longueur();
        }
        let masse_finale = colonie.substrat().masse_totale();
        assert!(!colonie.segments().is_empty(), "graine {graine} : des segments consommés");
        assert!(consomme > 0.0, "graine {graine} : de la nourriture consommée : {consomme}");
        assert!(
            egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
            "graine {graine} : {masse_initiale} - {consomme} contre la masse finale {masse_finale}"
        );
    }

    // Même graine, mêmes octets ; avancer_n(n) et n appels à avancer, les mêmes octets.
    let mut un_a_un = colonie("colonie-anastomose.txt", 1);
    for _ in 0..duree {
        un_a_un.avancer();
    }
    let mut d_un_coup = colonie("colonie-anastomose.txt", 1);
    d_un_coup.avancer_n(duree);
    let mut encore = colonie("colonie-anastomose.txt", 1);
    encore.avancer_n(duree);
    assert_eq!(un_a_un.temps(), d_un_coup.temps(), "le même temps");
    assert_eq!(un_a_un.ecrire_canonique(), d_un_coup.ecrire_canonique(), "avancer_n(n) et n appels à avancer");
    assert_eq!(encore.ecrire_canonique(), d_un_coup.ecrire_canonique(), "même graine, mêmes octets");

    let mut autre_graine = colonie("colonie-anastomose.txt", 2);
    autre_graine.avancer_n(duree);
    assert_ne!(autre_graine.empreinte(), d_un_coup.empreinte(), "les graines 1 et 2 donnent deux empreintes");
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn une_branche_en_retour_s_anastomose_sur_le_premier_segment_de_sa_mere() {
    for graine in 1..=5u64 {
        for delai in [0u32, 1u32] {
            let mut colonie = colonie_du_texte(&retour(Some(delai)), graine);

            // Après un pas : la mère a laissé un segment et s'est ramifiée d'un demi-tour, vers l'arrière.
            colonie.avancer();
            assert_eq!(colonie.apex().len(), 2, "graine {graine}, délai {delai} : la mère et sa branche");
            assert_eq!(colonie.nb_ramifications(), 1, "graine {graine}, délai {delai} : une ramification");
            assert_eq!(colonie.segments().len(), 1, "graine {graine}, délai {delai} : un segment");
            let mere = colonie.apex()[0];
            let branche = colonie.apex()[1];
            assert!(mere.actif && branche.actif, "graine {graine}, délai {delai} : les deux pointes sont actives");
            assert!(branche.memoire.is_none(), "graine {graine}, délai {delai} : la branche n'a pas de mémoire");
            assert!(
                branche.direction.scalaire(&mere.direction) <= -1.0 + 1e-12,
                "graine {graine}, délai {delai} : la branche part à l'opposé de sa mère, produit scalaire {}",
                branche.direction.scalaire(&mere.direction)
            );
            let premier = colonie.segments()[0];
            assert_eq!(
                (branche.x, branche.y, branche.z),
                (premier.x1, premier.y1, premier.z1),
                "graine {graine}, délai {delai} : la branche naît à l'arrivée de sa mère"
            );

            // Au pas 2 : la branche s'anastomose sur le premier segment de sa mère, sans se ramifier ; la mère
            // continue et s'est ramifiée une seconde fois.
            colonie.avancer();
            assert_eq!(colonie.apex().len(), 3, "graine {graine}, délai {delai} : trois apex au pas 2");
            assert_eq!(colonie.nb_ramifications(), 2, "graine {graine}, délai {delai} : deux ramifications");
            assert_eq!(colonie.pointes(), 2, "graine {graine}, délai {delai} : deux pointes au pas 2");
            assert_eq!(colonie.segments().len(), 3, "graine {graine}, délai {delai} : trois segments au pas 2");
            assert_eq!(colonie.nb_anastomoses(), 1, "graine {graine}, délai {delai} : une anastomose au pas 2");
            let anastomosee = colonie.apex()[1];
            assert!(!anastomosee.actif, "graine {graine}, délai {delai} : la branche est inactive");
            assert!(anastomosee.memoire.is_none(), "graine {graine}, délai {delai} : elle perd sa mémoire");
            assert!(colonie.apex()[0].actif, "graine {graine}, délai {delai} : la mère continue");
            let troisieme = colonie.segments()[2];
            assert_eq!(
                troisieme.apex, 1,
                "graine {graine}, délai {delai} : le troisième segment est celui de la branche"
            );
            assert_eq!(troisieme.temps, 2, "graine {graine}, délai {delai} : il est laissé au pas 2");
            assert!(
                distance_du_contrat((troisieme.x1, troisieme.y1, troisieme.z1), &colonie.segments()[0]) <= 1e-9,
                "graine {graine}, délai {delai} : l'arrivée de la branche est sur le premier segment de sa mère"
            );
        }
    }

    // Avec un délai de 2 pas, le premier segment de la mère est encore trop récent au pas 2 : aucune anastomose.
    for graine in 1..=5u64 {
        let mut colonie = colonie_du_texte(&retour(Some(2)), graine);
        colonie.avancer_n(2);
        assert_eq!(colonie.nb_anastomoses(), 0, "graine {graine}, délai 2 : aucune anastomose au pas 2");
        assert_eq!(colonie.apex().len(), 4, "graine {graine}, délai 2 : quatre apex au pas 2");
        assert_eq!(colonie.nb_ramifications(), 3, "graine {graine}, délai 2 : trois ramifications au pas 2");
    }

    // Sans la ligne, aucune anastomose en six pas.
    for graine in 1..=5u64 {
        let mut colonie = colonie_du_texte(&retour(None), graine);
        colonie.avancer_n(6);
        assert_eq!(colonie.nb_anastomoses(), 0, "graine {graine}, sans ligne : aucune anastomose en six pas");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn une_pointe_seule_ne_s_anastomose_jamais_avec_elle_meme() {
    let texte = format!("{SEULE_SANS_ANASTOMOSE}anastomose 1.0 0\n");
    for graine in 1..=5u64 {
        let mut colonie = colonie_du_texte(&texte, graine);
        colonie.avancer_n(10);
        assert_eq!(colonie.segments().len(), 10, "graine {graine} : dix segments, un par pas");
        assert_eq!(colonie.nb_anastomoses(), 0, "graine {graine} : aucune anastomose");
        assert_eq!(colonie.apex().len(), 1, "graine {graine} : un seul apex");
        assert_eq!(colonie.pointes(), 1, "graine {graine} : la pointe reste active");
        let pointe = colonie.apex()[0];
        assert!(pointe.actif, "graine {graine} : la pointe est active");
        let dernier = colonie.segments()[9];
        assert!(
            distance_du_contrat((pointe.x, pointe.y, pointe.z), &dernier) <= 1e-9,
            "graine {graine} : le dernier segment touche la pointe, distance {}",
            distance_du_contrat((pointe.x, pointe.y, pointe.z), &dernier)
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn les_lignes_anastomose_mal_formees_sont_refusees_avec_leur_numero() {
    let refusees = [
        "anastomose 0.5",
        "anastomose 0.5 10 3",
        "anastomose -0.5 10",
        "anastomose inf 10",
        "anastomose NaN 10",
        "anastomose demi 10",
        "anastomose 0.5 -1",
        "anastomose 0.5 1.5",
        "anastomose 0.5 +3",
        "anastomose 0.5 4294967296",
    ];
    let mut vues = 0usize;
    for ligne in refusees {
        assert_eq!(
            erreur(&avec_lignes_en_plus(&[ligne])),
            ErreurColonie3d::LigneInvalide(8),
            "à la ligne 8 : `{ligne}`"
        );
        vues += 1;
    }
    assert_eq!(vues, 10, "les dix formes refusées ont été jouées");

    assert_eq!(
        erreur(&avec_lignes_en_plus(&["anastomose 0.5 10", "anastomose 0.5 10"])),
        ErreurColonie3d::EnDouble("anastomose"),
        "deux lignes `anastomose`"
    );
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["anastomose 0.5 10", "anastomose 0.5"])),
        ErreurColonie3d::LigneInvalide(9),
        "une seconde ligne mal formée, à sa ligne"
    );

    for (ligne, rayon, delai) in
        [("anastomose 0 0", 0.0, 0u32), ("anastomose 0.5 10", 0.5, 10), ("anastomose 1e-3 4294967295", 1e-3, u32::MAX)]
    {
        let lu = lire_scenario_colonie3d(&avec_lignes_en_plus(&[ligne]))
            .unwrap_or_else(|e| panic!("`{ligne}` : refus inattendu {e:?}"));
        assert_eq!(lu.anastomose, Some(Reglage { rayon, delai }), "`{ligne}` : le réglage lu");
    }

    let sans = lire_scenario_colonie3d(&LIGNES.join("\n")).expect("le petit scénario se lit");
    assert_eq!(sans.anastomose, None, "sans la ligne, l'anastomose vaut None");
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn un_rayon_d_anastomose_hors_bornes_est_refuse_par_la_construction() {
    let base = lire_scenario_colonie3d(&LIGNES.join("\n")).expect("le petit scénario se lit");

    for rayon in [-0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut scenario = base.clone();
        scenario.anastomose = Some(Reglage { rayon, delai: 5 });
        assert_eq!(
            Colonie3d::nouvelle(&scenario, 1).unwrap_err(),
            ErreurColonie3d::ReglageInvalide("anastomose"),
            "rayon {rayon}"
        );
    }

    // L'ordre du contrat : une spore dans un obstacle passe avant le rayon.
    let mut scenario = base.clone();
    scenario.obstacles = vec![Bloc { x0: 4, y0: 4, z0: 4, x1: 4, y1: 4, z1: 4 }];
    scenario.anastomose = Some(Reglage { rayon: -0.5, delai: 10 });
    assert_eq!(
        Colonie3d::nouvelle(&scenario, 1).unwrap_err(),
        ErreurColonie3d::DansUnObstacle(0),
        "la spore dans un obstacle passe avant le rayon"
    );

    // Et le rayon passe avant un substrat instable : `diffusion 0.5` ferait refuser `Substrat3d::nouveau`.
    let mut scenario = base.clone();
    scenario.diffusion = 0.5;
    scenario.anastomose = Some(Reglage { rayon: f64::NAN, delai: 10 });
    assert_eq!(
        Colonie3d::nouvelle(&scenario, 1).unwrap_err(),
        ErreurColonie3d::ReglageInvalide("anastomose"),
        "le rayon passe avant le substrat instable"
    );

    // Un rayon nul et un délai de u32::MAX se construisent.
    let mut scenario = base.clone();
    scenario.anastomose = Some(Reglage { rayon: 0.0, delai: u32::MAX });
    assert!(Colonie3d::nouvelle(&scenario, 1).is_ok(), "un rayon nul et le plus grand délai se construisent");
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn la_distance_et_le_point_le_plus_proche_suivent_les_cas_verifies_a_la_main() {
    // Un segment horizontal : au milieu, au-delà de l'arrivée, avant le départ, sur le segment.
    let horizontal = segment3(0, 0, (0.0, 0.0, 0.0), (10.0, 0.0, 0.0));
    assert_eq!(point_le_plus_proche_3d((5.0, 5.0, 0.0), &horizontal), (5.0, 0.0, 0.0), "au milieu");
    assert_eq!(distance_au_segment_3d((5.0, 5.0, 0.0), &horizontal), 5.0, "la distance au milieu");
    assert_eq!(point_le_plus_proche_3d((15.0, 0.0, 0.0), &horizontal), (10.0, 0.0, 0.0), "au-delà de l'arrivée");
    assert_eq!(distance_au_segment_3d((15.0, 0.0, 0.0), &horizontal), 5.0, "la distance au-delà de l'arrivée");
    assert_eq!(point_le_plus_proche_3d((-5.0, 0.0, 0.0), &horizontal), (0.0, 0.0, 0.0), "avant le départ");
    assert_eq!(distance_au_segment_3d((-5.0, 0.0, 0.0), &horizontal), 5.0, "la distance avant le départ");
    assert_eq!(point_le_plus_proche_3d((3.0, 0.0, 0.0), &horizontal), (3.0, 0.0, 0.0), "sur le segment");
    assert_eq!(distance_au_segment_3d((3.0, 0.0, 0.0), &horizontal), 0.0, "la distance nulle");

    // Un segment diagonal où `t` vaut exactement 0,5.
    let diagonal = segment3(0, 0, (0.0, 0.0, 0.0), (2.0, 2.0, 2.0));
    assert_eq!(point_le_plus_proche_3d((3.0, 0.0, 0.0), &diagonal), (1.0, 1.0, 1.0), "t = 0,5 sur la diagonale");
    assert_eq!(distance_au_segment_3d((3.0, 0.0, 0.0), &diagonal), 6.0_f64.sqrt(), "la distance à la diagonale");

    // Un segment réduit à un point.
    let point = segment3(0, 0, (1.0, 2.0, 3.0), (1.0, 2.0, 3.0));
    assert_eq!(point_le_plus_proche_3d((1.0, 2.0, 5.0), &point), (1.0, 2.0, 3.0), "un segment réduit à un point");
    assert_eq!(distance_au_segment_3d((1.0, 2.0, 5.0), &point), 2.0, "la distance à ce point");
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_admet_les_bornes_du_cube_ferme_et_un_cube_plat() {
    let mut index = Index3d::nouveau(2, 2, 2);
    assert_eq!(index.ajouter(segment3(0, 0, (0.0, 0.0, 0.0), (2.0, 2.0, 2.0))), 0, "le premier numéro est 0");
    assert_eq!(index.ajouter(segment3(1, 0, (2.0, 1.0, 1.0), (2.0, 2.0, 1.0))), 1, "le second numéro est 1");
    assert_eq!(index.segments().len(), 2, "l'index garde ses segments");
    assert_eq!(index.segments()[0], segment3(0, 0, (0.0, 0.0, 0.0), (2.0, 2.0, 2.0)), "le segment 0 rendu tel quel");
    assert_eq!(index.segments()[1], segment3(1, 0, (2.0, 1.0, 1.0), (2.0, 2.0, 1.0)), "le segment 1 rendu tel quel");

    // Un cube plat sur `y` : la seule coordonnée admise en `y` est 0,0.
    let mut plat = Index3d::nouveau(4, 0, 3);
    assert_eq!(plat.ajouter(segment3(0, 0, (0.0, 0.0, 0.0), (4.0, 0.0, 3.0))), 0, "un segment dans un cube plat");
    assert_eq!(
        plat.chercher((2.0, 0.0, 1.5), 1, 0, &Reglage { rayon: 0.1, delai: 0 }),
        Some(0),
        "un point sur le segment du cube plat"
    );
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_refuse_une_extremite_au_dela_du_cube() {
    let mut index = Index3d::nouveau(2, 2, 2);
    index.ajouter(segment3(0, 0, (0.0, 0.0, 0.0), (3.0, 0.0, 0.0)));
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_refuse_une_extremite_negative() {
    let mut index = Index3d::nouveau(2, 2, 2);
    index.ajouter(segment3(0, 0, (-0.1, 0.0, 0.0), (1.0, 0.0, 0.0)));
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_refuse_une_extremite_non_finie() {
    let mut index = Index3d::nouveau(2, 2, 2);
    index.ajouter(segment3(0, 0, (0.0, 0.0, 0.0), (f64::NAN, 0.0, 0.0)));
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_ne_rend_que_les_candidats_des_trois_conditions() {
    let mut index = Index3d::nouveau(10, 10, 10);
    assert_eq!(index.ajouter(segment3(3, 5, (2.0, 2.0, 2.0), (2.0, 2.0, 2.0))), 0, "un segment au temps 5");
    let sans_delai = Reglage { rayon: 5.0, delai: 0 };

    // Un segment du même apex n'est jamais candidat.
    assert_eq!(index.chercher((2.0, 2.0, 2.0), 3, 5, &sans_delai), None, "son propre segment est écarté");
    assert_eq!(index.chercher((2.0, 2.0, 2.0), 4, 5, &sans_delai), Some(0), "un autre apex, le même pas");

    // Un segment né après le temps courant n'est pas candidat.
    assert_eq!(index.chercher((2.0, 2.0, 2.0), 4, 4, &sans_delai), None, "le segment naît après le temps courant");

    // Le délai, à un pas près.
    let avec_delai = Reglage { rayon: 5.0, delai: 2 };
    assert_eq!(index.chercher((2.0, 2.0, 2.0), 4, 6, &avec_delai), None, "6 < 5 + 2, un pas trop tôt");
    assert_eq!(index.chercher((2.0, 2.0, 2.0), 4, 7, &avec_delai), Some(0), "7 = 5 + 2, le premier pas admis");

    // La somme du délai est faite en u64 : les plus grandes valeurs ne débordent ni ne paniquent.
    let mut extremes = Index3d::nouveau(4, 4, 4);
    extremes.ajouter(segment3(0, u32::MAX - 1, (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
    assert_eq!(
        extremes.chercher((1.0, 1.0, 1.0), 1, u32::MAX, &Reglage { rayon: 1.0, delai: 1 }),
        Some(0),
        "(u32::MAX − 1) + 1 = u32::MAX, le premier pas admis"
    );
    assert_eq!(
        extremes.chercher((1.0, 1.0, 1.0), 1, u32::MAX, &Reglage { rayon: 1.0, delai: 2 }),
        None,
        "un pas de plus et le segment est trop récent"
    );
    let mut au_maximum = Index3d::nouveau(4, 4, 4);
    au_maximum.ajouter(segment3(0, u32::MAX, (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
    assert_eq!(
        au_maximum.chercher((1.0, 1.0, 1.0), 1, u32::MAX, &Reglage { rayon: 1.0, delai: u32::MAX }),
        None,
        "u32::MAX + u32::MAX, en u64, dépasse u32::MAX"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_applique_les_bornes_du_rayon_et_refuse_les_points_non_finis() {
    let mut index = Index3d::nouveau(6, 6, 6);
    index.ajouter(segment3(0, 0, (1.0, 1.0, 1.0), (2.0, 1.0, 1.0)));

    // Le point (1,5 ; 2 ; 1) est à distance 1 du segment.
    assert_eq!(
        index.chercher((1.5, 2.0, 1.0), 9, 0, &Reglage { rayon: 1.0, delai: 0 }),
        Some(0),
        "l'égalité au rayon compte comme dedans"
    );
    assert_eq!(
        index.chercher((1.5, 2.0, 1.0), 9, 0, &Reglage { rayon: 0.999, delai: 0 }),
        None,
        "juste au-delà du rayon"
    );
    assert_eq!(
        index.chercher((1.5, 1.0, 1.0), 9, 0, &Reglage { rayon: 0.0, delai: 0 }),
        Some(0),
        "un rayon nul trouve un point sur le segment"
    );
    assert_eq!(
        index.chercher((1.5, 2.0, 1.0), 9, 0, &Reglage { rayon: 0.0, delai: 0 }),
        None,
        "un rayon nul ne trouve pas un point hors du segment"
    );
    assert_eq!(
        index.chercher((1.5, 1.0, 1.0), 9, 0, &Reglage { rayon: -0.5, delai: 0 }),
        None,
        "un rayon négatif ne trouve rien"
    );
    assert_eq!(
        index.chercher((1.5, 1.0, 1.0), 9, 0, &Reglage { rayon: f64::NAN, delai: 0 }),
        None,
        "un rayon NaN ne trouve rien"
    );
    assert_eq!(
        index.chercher((30.0, 30.0, 30.0), 9, 0, &Reglage { rayon: f64::INFINITY, delai: 0 }),
        Some(0),
        "un rayon infini atteint un point hors du cube"
    );
    assert_eq!(
        index.chercher((f64::NAN, 1.0, 1.0), 9, 0, &Reglage { rayon: 1.0, delai: 0 }),
        None,
        "un point NaN ne trouve rien"
    );
    assert_eq!(
        index.chercher((1.0, f64::INFINITY, 1.0), 9, 0, &Reglage { rayon: 1.0, delai: 0 }),
        None,
        "un point infini ne trouve rien"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn l_index_cherche_par_distance_puis_numero_et_voit_un_long_segment() {
    // L'index vide.
    let vide = Index3d::nouveau(4, 4, 4);
    assert!(vide.segments().is_empty(), "un index neuf n'a pas de segment");
    assert_eq!(
        vide.chercher((2.0, 2.0, 2.0), 0, 0, &Reglage { rayon: 1.0, delai: 0 }),
        None,
        "un index vide ne trouve rien"
    );

    // À égale distance, le plus petit numéro ; sinon le plus proche.
    let mut index = Index3d::nouveau(4, 4, 4);
    index.ajouter(segment3(0, 0, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));
    index.ajouter(segment3(1, 0, (2.0, 0.0, 0.0), (2.0, 0.0, 0.0)));
    let reglage = Reglage { rayon: 3.0, delai: 0 };
    assert_eq!(index.chercher((1.0, 0.0, 0.0), 9, 0, &reglage), Some(0), "à égale distance, le plus petit numéro");
    assert_eq!(index.chercher((1.5, 0.0, 0.0), 9, 0, &reglage), Some(1), "sinon le plus proche");

    // Un point hors du cube : la fenêtre est bornée au cube, et le candidat est trouvé.
    assert_eq!(
        index.chercher((-1.0, 0.0, 0.0), 9, 0, &Reglage { rayon: 2.0, delai: 0 }),
        Some(0),
        "un point hors du cube est permis, la fenêtre se borne au cube"
    );

    // Un long segment vu près de son départ, loin du milieu où il est rangé.
    let mut long = Index3d::nouveau(20, 2, 2);
    long.ajouter(segment3(0, 0, (0.0, 0.5, 0.5), (20.0, 0.5, 0.5)));
    assert_eq!(
        long.chercher((0.5, 0.5, 0.5), 9, 0, &Reglage { rayon: 0.0, delai: 0 }),
        Some(0),
        "un long segment est vu près de son départ, loin de son milieu"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn chercher_si_ecarte_un_candidat_et_trouve_le_suivant() {
    let mut index = Index3d::nouveau(8, 3, 3);
    index.ajouter(segment3(0, 0, (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)));
    index.ajouter(segment3(0, 0, (3.0, 1.0, 1.0), (3.0, 1.0, 1.0)));
    index.ajouter(segment3(0, 0, (5.0, 1.0, 1.0), (5.0, 1.0, 1.0)));
    let reglage = Reglage { rayon: 10.0, delai: 0 };
    let point = (1.0, 1.0, 1.0);

    assert_eq!(
        index.chercher_si(point, 9, 10, &reglage, |segment, _| segment.x1 != 1.0),
        Some(1),
        "un candidat écarté laisse trouver le suivant"
    );
    assert_eq!(
        index.chercher_si(point, 9, 10, &reglage, |segment, _| segment.x1 != 1.0 && segment.x1 != 3.0),
        Some(2),
        "deux candidats écartés laissent le plus loin"
    );
    assert_eq!(index.chercher_si(point, 9, 10, &reglage, |_, _| false), None, "tous écartés, rien");
}

/// Un appel à `retenu` retenu par le test : la fin du segment vue et le point le plus proche passé.
type Appel = (f64, f64, f64, (f64, f64, f64));

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn retenu_ne_recoit_que_des_candidats_avec_leur_point_le_plus_proche() {
    let mut index = Index3d::nouveau(12, 3, 3);
    let segments = [
        segment3(5, 0, (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)),
        segment3(6, 8, (1.5, 1.0, 1.0), (1.5, 1.0, 1.0)),
        segment3(6, 0, (10.0, 1.0, 1.0), (10.0, 1.0, 1.0)),
        segment3(6, 0, (2.5, 1.0, 1.0), (2.5, 1.0, 1.0)),
    ];
    for segment in segments {
        index.ajouter(segment);
    }
    let reglage = Reglage { rayon: 2.0, delai: 3 };
    let appels: RefCell<Vec<Appel>> = RefCell::new(Vec::new());
    let resultat = index.chercher_si((1.0, 1.0, 1.0), 5, 10, &reglage, |segment, point| {
        appels.borrow_mut().push((segment.x1, segment.y1, segment.z1, point));
        true
    });
    assert_eq!(resultat, Some(3), "seul le quatrième segment est candidat");
    let appels = appels.into_inner();
    assert!(!appels.is_empty(), "le candidat a été vu par retenu");
    for (x1, y1, z1, point) in appels {
        assert_eq!((x1, y1, z1), (2.5, 1.0, 1.0), "retenu n'est appelé que sur le candidat");
        assert_eq!(point, (2.5, 1.0, 1.0), "retenu reçoit le point le plus proche du candidat");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-24"), ignore)]
fn chercher_si_ecarte_le_plus_proche_derriere_une_plaque_bloquee() {
    let mut substrat = Substrat3d::nouveau(6, 5, 5, 0.0, 1.0, 1.0).expect("un substrat sans diffusion se construit");
    // Une plaque de cases bloquées en x = 3.
    substrat.bloquer(3, 0, 0, 3, 4, 4);

    let mut index = Index3d::nouveau(6, 5, 5);
    index.ajouter(segment3(0, 0, (3.5, 2.5, 2.5), (3.5, 2.5, 2.5)));
    index.ajouter(segment3(0, 0, (1.0, 2.5, 2.5), (1.0, 2.5, 2.5)));

    let arrivee = (2.5, 2.5, 2.5);
    let reglage = Reglage { rayon: 2.0, delai: 0 };
    let resultat = index.chercher_si(arrivee, 1, 1, &reglage, |_, (qx, qy, qz)| {
        let q = (qx.clamp(0.0, 6.0), qy.clamp(0.0, 5.0), qz.clamp(0.0, 5.0));
        traverser_3d(&substrat, arrivee, q) == Traversee3d::Libre
    });
    assert_eq!(resultat, Some(1), "le plus proche, derrière la plaque, est écarté ; celui du même côté est rendu");
}
