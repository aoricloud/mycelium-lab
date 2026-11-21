//! Tests d'acceptation de l'étape 23 : les obstacles et le thigmotropisme en volume.
//!
//! Ce qui est vérifié :
//! - les empreintes de `colonie-mur`, graine 1 : au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis à la
//!   fin de sa durée (l'empreinte de son fichier de `references/`), et les comptes de la fin ;
//! - sans obstacle ni thigmotropisme, rien ne change : `colonie-cube`, `colonie-goutte` et `colonie-chimio` gardent
//!   l'empreinte de leur fichier, et une ligne `bloc` reste refusée (`BlocRefuse`) ;
//! - le mur n'est jamais traversé, graines 1 à 3 : chaque segment est échantillonné en 64 points et parcouru par
//!   `traverser_3d` (qui doit rendre `Libre`), aucun apex n'est dans une case bloquée, un apex inactif n'a pas de
//!   mémoire, et le nombre d'apex vaut 1 + ramifications ;
//! - les pointes passent par la fenêtre du mur, et tout bout de segment dans l'épaisseur du mur est dans la fenêtre ;
//! - le déterminisme, la différence des graines et le bilan de la nourriture, sur `colonie-mur` ;
//! - les cas faits à la main : le longement d'un mur, la reprise du cap au bout d'un mur étroit, l'arrêt contre une
//!   face avec deux naissances, et l'immobilité à `thigmotropisme 0` ;
//! - `traverser_3d`, sur des cas vérifiés à la main : les deux sens de chaque axe, une arrivée sur une face, un départ
//!   sur une frontière en descendant, les coins (2D, voisine, triple), `DepartBloque`, `Invalide` et l'absence de
//!   lecture hors du cube ;
//! - les erreurs de lecture des lignes `obstacle` et `thigmotropisme`, avec leur numéro de ligne ;
//! - l'écriture canonique de `colonie-mur` : la longueur du contrat, queue de l'étape 23 comprise ;
//! - le gradient près d'un mur : une case bloquée voisine vaut la case elle-même, et une case bloquée rend zéro.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`, `min`,
//! `max`, `clamp` et `as` seulement (les cosinus et sinus viennent des tables `COS` et `SIN`).

use std::path::Path;

use mycelium_lab::colonie3d::{Colonie3d, ErreurColonie3d, ScenarioColonie3d, gradient_3d, lire_scenario_colonie3d};
use mycelium_lab::contact3d::{Axe3d, Contact3d, Traversee3d, traverser_3d};
use mycelium_lab::croissance::{COS, SIN};
use mycelium_lab::cube::Bloc;
use mycelium_lab::direction3d::Direction3d;
use mycelium_lab::substrat3d::Substrat3d;
use mycelium_lab::tropisme::MEMOIRE_DE_DIRECTION;

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

/// Le mur plein qui coupe le cube en deux et fait longe l'apex : `thigmotropisme 90`.
const MUR_LONGE: &str = "\
cube 24 24 24
substrat 1.0 0.0
spore 4 4 4
croissance 1.0 infini 30 0
couplage 0.1 0.5
duree 40
obstacle 5 0 0 5 23 23
thigmotropisme 90
";

/// Le mur étroit d'une seule case : l'apex le longe, puis retrouve son cap en le dépassant.
const MUR_ETROIT: &str = "\
cube 16 16 16
substrat 1.0 0.0
spore 4 4 4
croissance 1.0 infini 30 0
couplage 0.1 0.5
duree 40
obstacle 6 4 4 6 4 4
thigmotropisme 90
";

/// Un mur de face, sous un angle plus grand que le seuil : l'apex s'arrête et se ramifie.
const MUR_DE_FACE: &str = "\
cube 16 16 16
substrat 1.0 0.0
spore 4 4 4
croissance 1.0 infini 30 0
couplage 0.1 0.5
duree 20
obstacle 5 0 0 5 15 15
thigmotropisme 1
";

/// La cellule de la spore entourée de six cases bloquées : au premier pas, tout cap touche un mur.
const CELLULE_ENTOUREE: &str = "\
cube 9 9 9
substrat 1.0 0.0
spore 4 4 4
croissance 2.0 infini 30 0
couplage 0.1 0.5
duree 20
obstacle 4 4 3 4 4 3
obstacle 4 4 5 4 4 5
obstacle 4 3 4 4 3 4
obstacle 4 5 4 4 5 4
obstacle 3 4 4 3 4 4
obstacle 5 4 4 5 4 4
";

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

/// La direction du vecteur `(x, y, z)`, supposé non nul et fini.
fn direction(x: f64, y: f64, z: f64) -> Direction3d {
    Direction3d::nouvelle(x, y, z).expect("un vecteur non nul et fini")
}

/// La projection d'une direction sur le plan de la face d'axe `x` : sa composante en `x` mise à zéro, puis normée.
fn projection_sur_x(d: Direction3d) -> Direction3d {
    direction(0.0, d.y(), d.z())
}

/// Le petit scénario valide, la ligne `numero` (à partir de 1) remplacée.
fn avec_ligne(numero: usize, ligne: &str) -> String {
    assert!((1..=LIGNES.len()).contains(&numero), "la ligne {numero} existe dans le petit scénario");
    let mut lignes = LIGNES.to_vec();
    lignes[numero - 1] = ligne;
    lignes.join("\n")
}

/// Le petit scénario valide suivi de lignes de plus : la première est la huitième.
fn avec_lignes_en_plus(lignes: &[&str]) -> String {
    format!("{}\n{}\n", LIGNES.join("\n"), lignes.join("\n"))
}

/// La case d'un point `(x, y, z)` dans `substrat` (règle du plancher), en refusant un point hors du cube.
fn case_du_point(point: (f64, f64, f64), substrat: &Substrat3d) -> (usize, usize, usize) {
    let (x, y, z) = point;
    let (largeur, hauteur, profondeur) =
        (substrat.largeur() as f64, substrat.hauteur() as f64, substrat.profondeur() as f64);
    assert!(
        (0.0..largeur).contains(&x) && (0.0..hauteur).contains(&y) && (0.0..profondeur).contains(&z),
        "point hors du cube : ({x}, {y}, {z})"
    );
    (x as usize, y as usize, z as usize)
}

/// Un substrat de `4 × 4 × 4` sans diffusion, où chaque case de `bloquees` est bloquée.
fn substrat_de_blocage(bloquees: &[(usize, usize, usize)]) -> Substrat3d {
    let mut substrat = Substrat3d::nouveau(4, 4, 4, 0.0, 1.0, 1.0).expect("un substrat sans diffusion se construit");
    for &(x, y, z) in bloquees {
        substrat.bloquer(x, y, z, x, y, z);
    }
    substrat
}

/// Un contact attendu de `traverser_3d`.
fn contact(case: (usize, usize, usize), axe: Axe3d, sens: i8) -> Traversee3d {
    Traversee3d::Contact(Contact3d { case, axe, sens })
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn colonie_mur_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut colonie = colonie("colonie-mur.txt", 1);
    assert_eq!(
        colonie.empreinte(),
        "6596c2e3076f25fd5c5dc60cace670afb527c33a061ed6db7c8bcec229c25786",
        "colonie-mur : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-mur : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "23db85ea2e85d68087a4048a6b43d8f8b99084dc081cb799cc24a944dbfcdd0b",
        "colonie-mur : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-mur : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "ab6d25aded167382f4fd08964e77e2ba8bd3ec2533be19fd931b6c38b5e58f32",
        "colonie-mur : l'empreinte après 10 pas"
    );
    let duree = colonie.duree();
    colonie.avancer_n(duree - 10);
    assert_eq!(colonie.temps(), duree, "colonie-mur : le temps est la durée");
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-mur.empreinte"),
        "colonie-mur : l'empreinte de son fichier de références"
    );
    assert_eq!(colonie.segments().len(), 3227, "colonie-mur : les segments de la fin");
    assert_eq!(colonie.nb_ramifications(), 319, "colonie-mur : les ramifications de la fin");
    assert_eq!(colonie.pointes(), 298, "colonie-mur : les pointes actives de la fin");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn sans_obstacle_ni_thigmotropisme_rien_ne_change() {
    for (nom, fichier) in [
        ("colonie-cube.txt", "colonie-cube.empreinte"),
        ("colonie-goutte.txt", "colonie-goutte.empreinte"),
        ("colonie-chimio.txt", "colonie-chimio.empreinte"),
    ] {
        let colonie = colonie_finie(nom, 1);
        assert_eq!(colonie.temps(), colonie.duree(), "{nom} : le temps est la durée");
        assert!(colonie.scenario().obstacles.is_empty(), "{nom} : pas d'obstacle");
        assert_eq!(colonie.scenario().thigmotropisme, 0, "{nom} : pas de thigmotropisme");
        assert_eq!(colonie.empreinte(), empreinte_de_reference(fichier), "{nom} : l'empreinte de son fichier");
    }

    // Une ligne `bloc`, quelle que soit sa forme, reste refusée.
    let texte = texte_du_fichier("colonie-cube.txt");
    let sans_fin = texte.trim_end();
    let numero = sans_fin.lines().count() + 1;
    for forme in ["bloc 0 0 0 1 1 1", "bloc", "bloc 1 2 3"] {
        let texte = format!("{sans_fin}\n{forme}\n");
        assert_eq!(
            erreur(&texte),
            ErreurColonie3d::BlocRefuse(numero),
            "`{forme}` à la ligne {numero} : les obstacles s'écrivent `obstacle`"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn le_mur_n_est_jamais_traverse_et_les_apex_restent_dans_le_cube() {
    let scenario = scenario("colonie-mur.txt");
    assert_eq!(scenario.obstacles.len(), 4, "colonie-mur : quatre pavés bloqués");
    assert_eq!(scenario.thigmotropisme, 35, "colonie-mur : le seuil du thigmotropisme");

    for graine in 1..=3u64 {
        let mut colonie = Colonie3d::nouvelle(&scenario, graine).expect("le scénario se construit");
        colonie.avancer_n(colonie.duree());
        assert_eq!(colonie.temps(), colonie.duree(), "graine {graine} : le temps est la durée");

        let substrat = colonie.substrat();
        let segments = colonie.segments();
        assert!(!segments.is_empty(), "graine {graine} : des segments à vérifier");

        let mut points_vus = 0usize;
        for (numero, segment) in segments.iter().enumerate() {
            for i in 0..64 {
                let t = i as f64 / 63.0;
                let point = (
                    segment.x0 + (segment.x1 - segment.x0) * t,
                    segment.y0 + (segment.y1 - segment.y0) * t,
                    segment.z0 + (segment.z1 - segment.z0) * t,
                );
                let (x, y, z) = case_du_point(point, substrat);
                assert!(
                    !substrat.est_bloquee(x, y, z),
                    "graine {graine}, segment {numero}, point {i} : la case ({x}, {y}, {z}) est bloquée"
                );
                points_vus += 1;
            }
            assert_eq!(
                traverser_3d(substrat, (segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)),
                Traversee3d::Libre,
                "graine {graine}, segment {numero} : un segment de la colonie est libre dans le cube"
            );
        }
        assert_eq!(points_vus, 64 * segments.len(), "graine {graine} : les 64 points de chaque segment sont vus");

        let mut apex_vus = 0usize;
        for apex in colonie.apex() {
            let (x, y, z) = case_du_point((apex.x, apex.y, apex.z), substrat);
            assert!(
                !substrat.est_bloquee(x, y, z),
                "graine {graine}, apex {} : la case ({x}, {y}, {z}) est bloquée",
                apex.id
            );
            if !apex.actif {
                assert!(apex.memoire.is_none(), "graine {graine}, apex {} inactif : il n'a pas de mémoire", apex.id);
            }
            apex_vus += 1;
        }
        assert_eq!(apex_vus, colonie.apex().len(), "graine {graine} : tous les apex sont vus");
        assert_eq!(
            colonie.apex().len() as u64,
            1 + colonie.nb_ramifications(),
            "graine {graine} : un apex pour la spore, plus un par ramification"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn les_pointes_passent_par_la_fenetre_du_mur() {
    let colonie = colonie_finie("colonie-mur.txt", 1);
    let segments = colonie.segments();
    assert!(!segments.is_empty(), "des segments à mesurer");

    let mut au_dela = 0usize;
    let mut dans_l_epaisseur = 0usize;
    let mut bouts_vus = 0usize;
    for segment in segments {
        for point in [(segment.x0, segment.y0, segment.z0), (segment.x1, segment.y1, segment.z1)] {
            let (x, y, z) = point;
            if x >= 18.0 {
                au_dela += 1;
            }
            if (16.0..18.0).contains(&x) {
                dans_l_epaisseur += 1;
                assert!(
                    (14.0..18.0).contains(&y) && (14.0..18.0).contains(&z),
                    "un bout de segment dans l'épaisseur du mur hors de la fenêtre : ({x}, {y}, {z})"
                );
            }
            bouts_vus += 1;
        }
    }
    assert_eq!(bouts_vus, 2 * segments.len(), "les deux bouts de chaque segment sont vus");
    assert!(au_dela > 0, "au moins un bout de segment est au-delà du mur (x ≥ 18)");
    assert!(dans_l_epaisseur > 0, "au moins un bout de segment est dans l'épaisseur du mur (16 ≤ x < 18)");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn colonie_mur_est_deterministe_et_perd_la_masse_de_ses_segments() {
    let scenario = scenario("colonie-mur.txt");
    let duree = scenario.duree;

    let mut d_un_coup = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    let masse_initiale = d_un_coup.substrat().masse_totale();
    let mut encore = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    let mut un_a_un = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");

    d_un_coup.avancer_n(duree);
    encore.avancer_n(duree);
    for _ in 0..duree {
        un_a_un.avancer();
    }
    assert_eq!(un_a_un.temps(), d_un_coup.temps(), "le même temps");
    assert_eq!(
        un_a_un.ecrire_canonique(),
        d_un_coup.ecrire_canonique(),
        "avancer_n(n) et n appels à avancer, les mêmes octets"
    );
    assert_eq!(encore.ecrire_canonique(), d_un_coup.ecrire_canonique(), "même graine, mêmes octets");

    let mut autre_graine = Colonie3d::nouvelle(&scenario, 2).expect("le scénario se construit");
    autre_graine.avancer_n(duree);
    assert_ne!(autre_graine.empreinte(), d_un_coup.empreinte(), "les graines 1 et 2 donnent deux empreintes");

    // La nourriture : la masse du départ moins le coût des segments est la masse de la fin.
    let mut consomme = 0.0;
    for segment in d_un_coup.segments() {
        consomme += scenario.cout * segment.longueur();
    }
    let masse_finale = d_un_coup.substrat().masse_totale();
    assert!(!d_un_coup.segments().is_empty(), "des segments consommés");
    assert!(consomme > 0.0, "de la nourriture consommée : {consomme}");
    assert!(
        egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
        "{masse_initiale} - {consomme} contre la masse finale {masse_finale}"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn un_mur_plein_fait_longer_l_apex_sans_l_arreter() {
    let mut colonie = colonie_du_texte(MUR_LONGE, 15);
    assert_eq!(colonie.scenario().thigmotropisme, 90, "le seuil est le plus grand permis");

    // Le premier contact : on le trouve pas à pas, et on note l'indice du premier segment du longement.
    let mut premier_segment = None;
    for _ in 0..20 {
        let avant = colonie.segments().len();
        colonie.avancer();
        assert!(colonie.apex()[0].actif, "l'apex ne s'arrête pas contre un mur longé");
        if colonie.apex()[0].memoire.is_some() {
            premier_segment = Some(avant);
            break;
        }
    }
    let premier_segment = premier_segment.expect("l'apex touche le mur et le longe");

    let (cap, g) = colonie.apex()[0].memoire.expect("une mémoire de longement");
    assert!(cap.x() > 0.0, "le cap de contact pointe vers le mur : x = {}", cap.x());
    assert_eq!(g, projection_sur_x(cap), "la mémoire est (d, g), g la projection de d sur la face");
    assert_eq!(g.x(), 0.0, "la direction de glissement est parallèle à la face x = 5");
    assert_eq!(colonie.apex()[0].direction, g, "l'apex prend la direction de glissement");

    // Le pas du contact est déjà le long de la face, et les dix suivants aussi.
    colonie.avancer_n(10);
    assert_eq!(colonie.apex()[0].memoire, Some((cap, g)), "la mémoire dure tant que le mur est là");
    assert!(colonie.apex()[0].actif, "l'apex reste actif");
    assert_eq!(colonie.nb_ramifications(), 0, "un longement ne se ramifie pas");
    assert_eq!(colonie.segments().len(), premier_segment + 11, "un segment par pas, contact compris");
    for (numero, segment) in colonie.segments().iter().enumerate().skip(premier_segment) {
        assert_eq!(
            segment.x1, segment.x0,
            "segment {numero} du longement : il doit rester parallèle à la face, x0 = {}, x1 = {}",
            segment.x0, segment.x1
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn un_mur_etroit_laisse_l_apex_retrouver_son_cap_et_le_depasser() {
    let mut colonie = colonie_du_texte(MUR_ETROIT, 15);

    let mut contact = None;
    for _ in 0..20 {
        let avant = colonie.segments().len();
        colonie.avancer();
        if colonie.apex()[0].memoire.is_some() {
            contact = Some(avant);
            break;
        }
    }
    let contact = contact.expect("l'apex touche le mur étroit");
    let (cap, g) = colonie.apex()[0].memoire.expect("une mémoire de longement");
    assert_eq!(g, projection_sur_x(cap), "la mémoire est (d, g), g la projection de d sur la face");
    assert_eq!(g.x(), 0.0, "la direction de glissement est parallèle à la face x = 6");
    let segment = &colonie.segments()[contact];
    assert_eq!(segment.x1, segment.x0, "le pas du contact longe la face");

    // On avance jusqu'à la reprise du cap ; les segments d'avant restent parallèles à la face.
    let mut reprise = None;
    for _ in 0..=20 {
        let avant = colonie.segments().len();
        colonie.avancer();
        if colonie.apex()[0].memoire.is_none() {
            reprise = Some((avant, colonie.segments().len()));
            break;
        }
        for (numero, segment) in colonie.segments().iter().enumerate().skip(avant) {
            assert_eq!(
                segment.x1, segment.x0,
                "segment {numero} du longement : x0 = {}, x1 = {}",
                segment.x0, segment.x1
            );
        }
    }
    let (debut, fin) = reprise.expect("l'apex retrouve son cap au bout du mur");
    assert_eq!(fin, debut + 1, "la reprise se fait en un pas");

    let rendue = colonie.apex()[0].direction;
    assert!(
        rendue.scalaire(&cap) >= COS[MEMOIRE_DE_DIRECTION as usize] - 1e-12,
        "la direction reprise est à {} degrés du cap au plus, produit scalaire {}",
        MEMOIRE_DE_DIRECTION,
        rendue.scalaire(&cap)
    );
    assert!(rendue.x() > 0.0, "l'apex repart vers l'avant, x = {}", rendue.x());

    // Il dépasse le mur : son corps et un bout de segment au-delà de la case bloquée (x ∈ [6, 7)).
    let mut depasse = false;
    for _ in 0..20 {
        colonie.avancer();
        if colonie.apex()[0].x > 7.0 {
            depasse = true;
            break;
        }
    }
    assert!(depasse, "l'apex dépasse le mur : x = {}", colonie.apex()[0].x);
    assert!(colonie.segments().iter().any(|segment| segment.x1 > 7.0), "un bout de segment est au-delà du mur");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn un_mur_de_face_arrete_l_apex_et_fait_naitre_deux_pointes() {
    let mut colonie = colonie_du_texte(MUR_DE_FACE, 15);
    assert_eq!(colonie.scenario().thigmotropisme, 1, "un seuil d'un degré");

    colonie.avancer_n(1);
    let apex = colonie.apex();
    assert_eq!(apex.len(), 3, "l'apex arrêté et deux naissances");
    assert_eq!(colonie.nb_ramifications(), 2, "deux ramifications");
    assert_eq!(colonie.pointes(), 2, "deux pointes actives");
    assert!(!apex[0].actif, "l'apex qui touche de face s'arrête");
    assert!(apex[0].memoire.is_none(), "un apex arrêté n'a pas de mémoire");
    assert!(colonie.segments().is_empty(), "un apex arrêté ne laisse pas de segment");

    let d = apex[0].direction;
    assert!(d.x().abs() >= SIN[1], "l'angle de contact dépasse le seuil : |d_n| = {}", d.x().abs());
    let plus = projection_sur_x(d);
    let moins = direction(0.0, -d.y(), -d.z());
    for (neuf, attendu, nom) in [(apex[1], plus, "+p"), (apex[2], moins, "−p")] {
        assert_eq!(neuf.direction, attendu, "la naissance dans la direction {nom}");
        assert!(neuf.actif, "la naissance {nom} est active");
        assert!(neuf.memoire.is_none(), "la naissance {nom} n'a pas de mémoire");
        assert_eq!(
            (neuf.x, neuf.y, neuf.z),
            (apex[0].x, apex[0].y, apex[0].z),
            "la naissance {nom} se fait à la position de l'apex arrêté"
        );
    }
    assert_eq!((apex[1].id, apex[2].id), (1, 2), "les identifiants suivent, +p puis −p");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn a_thigmotropisme_zero_l_apex_reste_sur_place() {
    let mut colonie = colonie_du_texte(CELLULE_ENTOUREE, 15);
    assert_eq!(colonie.scenario().thigmotropisme, 0, "le thigmotropisme est éteint");
    assert_eq!(colonie.scenario().obstacles.len(), 6, "les six cases autour de la spore sont bloquées");
    let (x, y, z) = (colonie.apex()[0].x, colonie.apex()[0].y, colonie.apex()[0].z);
    assert_eq!((x, y, z), (4.5, 4.5, 4.5), "l'apex part du centre de la case de la spore");

    colonie.avancer_n(10);
    assert!(colonie.segments().is_empty(), "aucun segment nouveau : la pointe reste sur place");
    assert_eq!(colonie.apex().len(), 1, "aucune naissance");
    assert_eq!(colonie.nb_ramifications(), 0, "aucune ramification");
    assert!(colonie.apex()[0].actif, "la pointe reste active");
    assert!(colonie.apex()[0].memoire.is_none(), "elle ne garde pas de mémoire");
    assert_eq!((colonie.apex()[0].x, colonie.apex()[0].y, colonie.apex()[0].z), (x, y, z), "la position n'a pas bougé");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn traverser_3d_suit_les_cas_verifies_a_la_main() {
    // Un seul axe, dans les deux sens, pour x, y et z : la case bloquée est au milieu du trajet.
    let substrat = substrat_de_blocage(&[(2, 1, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.5, 1.5, 1.5)),
        contact((2, 1, 1), Axe3d::X, 1),
        "x croissant"
    );
    assert_eq!(
        traverser_3d(&substrat, (3.5, 1.5, 1.5), (1.5, 1.5, 1.5)),
        contact((2, 1, 1), Axe3d::X, -1),
        "x décroissant"
    );
    let substrat = substrat_de_blocage(&[(1, 2, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (1.5, 2.5, 1.5)),
        contact((1, 2, 1), Axe3d::Y, 1),
        "y croissant"
    );
    assert_eq!(
        traverser_3d(&substrat, (1.5, 3.5, 1.5), (1.5, 1.5, 1.5)),
        contact((1, 2, 1), Axe3d::Y, -1),
        "y décroissant"
    );
    let substrat = substrat_de_blocage(&[(1, 1, 2)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (1.5, 1.5, 2.5)),
        contact((1, 1, 2), Axe3d::Z, 1),
        "z croissant"
    );
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 3.5), (1.5, 1.5, 1.5)),
        contact((1, 1, 2), Axe3d::Z, -1),
        "z décroissant"
    );

    // Une arrivée exactement sur une face : le point de la face est dans la case bloquée.
    let substrat = substrat_de_blocage(&[(2, 1, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.0, 1.5, 1.5)),
        contact((2, 1, 1), Axe3d::X, 1),
        "arrivée sur la face x = 2"
    );

    // Un départ sur une frontière en descendant : la case du plancher est au-dessus, le parcours commence en
    // dessous, et la case bloquée est atteinte depuis cette case du plancher.
    let substrat = substrat_de_blocage(&[(1, 1, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (2.0, 1.5, 1.5), (1.0, 1.5, 1.5)),
        contact((1, 1, 1), Axe3d::X, -1),
        "départ sur x = 2 en descendant vers la case bloquée (1, 1, 1)"
    );

    // Un coin exact en 2D : la diagonale passe par le coin des quatre cases ; la case diagonale bloquée est touchée,
    // et la face est le premier axe dont la case voisine du départ est bloquée, sinon le premier axe.
    let substrat = substrat_de_blocage(&[(2, 2, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.5, 2.5, 1.5)),
        contact((2, 2, 1), Axe3d::X, 1),
        "coin 2D, seule la diagonale est bloquée : le premier axe"
    );
    let substrat = substrat_de_blocage(&[(2, 2, 1), (1, 2, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.5, 2.5, 1.5)),
        contact((2, 2, 1), Axe3d::Y, 1),
        "coin 2D, la voisine (1, 2, 1) bloquée donne la face y"
    );

    // Le coin d'une voisine ne compte pas : le segment ne passe pas dans la case (2, 1, 1).
    let substrat = substrat_de_blocage(&[(2, 1, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.5, 2.5, 1.5)),
        Traversee3d::Libre,
        "le coin de la voisine bloquée n'est pas touché"
    );

    // Un coin triple : les trois coordonnées franchissent un entier au même point.
    let substrat = substrat_de_blocage(&[(2, 2, 2)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.5, 2.5, 2.5)),
        contact((2, 2, 2), Axe3d::X, 1),
        "coin triple, seule la diagonale est bloquée : le premier axe"
    );

    // La case du départ est bloquée.
    let substrat = substrat_de_blocage(&[(1, 1, 1)]);
    assert_eq!(
        traverser_3d(&substrat, (1.5, 1.5, 1.5), (2.5, 1.5, 1.5)),
        Traversee3d::DepartBloque,
        "départ dans la case bloquée (1, 1, 1)"
    );

    // Une coordonnée non finie ou hors du cube fermé est invalide.
    let substrat = substrat_de_blocage(&[]);
    assert_eq!(traverser_3d(&substrat, (f64::NAN, 0.5, 0.5), (2.5, 0.5, 0.5)), Traversee3d::Invalide, "départ NaN");
    assert_eq!(
        traverser_3d(&substrat, (0.5, 0.5, 0.5), (f64::INFINITY, 0.5, 0.5)),
        Traversee3d::Invalide,
        "arrivée infinie"
    );
    assert_eq!(traverser_3d(&substrat, (-0.001, 0.5, 0.5), (2.5, 0.5, 0.5)), Traversee3d::Invalide, "départ négatif");
    assert_eq!(
        traverser_3d(&substrat, (0.5, 0.5, 0.5), (4.001, 0.5, 0.5)),
        Traversee3d::Invalide,
        "arrivée au-delà de la largeur"
    );
    assert_eq!(
        traverser_3d(&substrat, (0.5, 0.5, 0.5), (0.5, 0.5, 4.001)),
        Traversee3d::Invalide,
        "arrivée au-delà de la profondeur"
    );

    // Aucune lecture hors du cube : finir exactement sur la face haute du cube est permis et libre.
    assert_eq!(
        traverser_3d(&substrat, (3.5, 0.5, 0.5), (4.0, 0.5, 0.5)),
        Traversee3d::Libre,
        "arrivée exactement sur x = 4, la face haute du cube"
    );
    assert_eq!(
        traverser_3d(&substrat, (0.5, 0.5, 0.5), (4.0, 4.0, 4.0)),
        Traversee3d::Libre,
        "arrivée exactement au coin haut du cube"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn les_lignes_obstacle_et_thigmotropisme_mal_formees_sont_refusees_avec_leur_numero() {
    let refusees = [
        ("une boîte inversée", "obstacle 5 0 0 3 1 1"),
        ("un mot manquant", "obstacle 0 0 0 1 1"),
        ("un mot en trop", "obstacle 0 0 0 1 1 1 1"),
        ("un nombre illisible", "obstacle 0 0 0 1 1 six"),
        ("un nombre négatif", "obstacle 0 0 0 1 1 -1"),
    ];
    let mut vues = 0;
    for (nom, ligne) in refusees {
        assert_eq!(
            erreur(&avec_lignes_en_plus(&[ligne])),
            ErreurColonie3d::LigneInvalide(8),
            "{nom}, à la ligne 8 : `{ligne}`"
        );
        vues += 1;
    }
    assert_eq!(vues, 5, "les cinq refus d'obstacle ont été joués");

    assert_eq!(
        erreur(&avec_ligne(2, "obstacle 5 0 0 3 1 1")),
        ErreurColonie3d::LigneInvalide(2),
        "une boîte inversée à sa propre ligne"
    );

    // Un obstacle qui déborde du cube, par sa borne haute.
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["obstacle 6 0 0 8 1 1"])),
        ErreurColonie3d::HorsDuCube(8),
        "un obstacle qui déborde en x"
    );
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["obstacle 0 0 6 1 1 8"])),
        ErreurColonie3d::HorsDuCube(8),
        "un obstacle qui déborde en z"
    );

    // Le thigmotropisme : un entier de 0 à 90 seulement.
    for ligne in ["thigmotropisme 91", "thigmotropisme 35.5", "thigmotropisme -1"] {
        assert_eq!(erreur(&avec_lignes_en_plus(&[ligne])), ErreurColonie3d::LigneInvalide(8), "`{ligne}`");
    }
    for (ligne, angle) in [("thigmotropisme 0", 0), ("thigmotropisme 35", 35), ("thigmotropisme 90", 90)] {
        let lu =
            lire_scenario_colonie3d(&avec_lignes_en_plus(&[ligne])).unwrap_or_else(|e| panic!("`{ligne}` : {e:?}"));
        assert_eq!(lu.thigmotropisme, angle, "`{ligne}` : l'angle lu");
    }

    // Deux lignes `thigmotropisme` : la seconde est en double.
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["thigmotropisme 10", "thigmotropisme 5"])),
        ErreurColonie3d::EnDouble("thigmotropisme"),
        "deux lignes `thigmotropisme`"
    );
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["thigmotropisme 0", "thigmotropisme 0"])),
        ErreurColonie3d::EnDouble("thigmotropisme"),
        "deux lignes `thigmotropisme 0` : une valeur nulle ne dispense pas du refus"
    );

    // Une spore puis une nourriture dans un obstacle, chacune avec sa ligne.
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["obstacle 0 0 0 7 7 7"])),
        ErreurColonie3d::DansUnObstacle(4),
        "la spore (4, 4, 4) est dans un obstacle qui couvre le cube"
    );
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["obstacle 0 0 0 0 0 0"])),
        ErreurColonie3d::DansUnObstacle(3),
        "la nourriture (0, 0, 0) est dans un obstacle"
    );
    let texte = format!("{}\nnourriture 2 2 2 5.0\nobstacle 2 2 2 2 2 2\n", LIGNES.join("\n"));
    assert_eq!(
        erreur(&texte),
        ErreurColonie3d::DansUnObstacle(8),
        "une nourriture ajoutée dans un obstacle, à sa ligne"
    );

    // Les lignes valides sont lues, dans l'ordre du texte.
    let lu = lire_scenario_colonie3d(&avec_lignes_en_plus(&["obstacle 5 5 5 6 6 6", "obstacle 0 1 2 3 3 3"]))
        .expect("deux obstacles dans le cube se lisent");
    assert_eq!(
        lu.obstacles,
        vec![Bloc { x0: 5, y0: 5, z0: 5, x1: 6, y1: 6, z1: 6 }, Bloc { x0: 0, y0: 1, z0: 2, x1: 3, y1: 3, z1: 3 },],
        "les obstacles gardent l'ordre du texte"
    );
    assert_eq!(lu.thigmotropisme, 0, "sans ligne, le thigmotropisme est éteint");
    assert_eq!(lu.chimiotropisme, 0.0, "sans ligne, le chimiotropisme est éteint");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn l_ecriture_canonique_de_colonie_mur_a_la_longueur_du_contrat() {
    let mut colonie = colonie("colonie-mur.txt", 1);
    assert_eq!(colonie.scenario().obstacles.len(), 4, "colonie-mur : quatre obstacles");
    assert_eq!(colonie.scenario().thigmotropisme, 35, "colonie-mur : le seuil du thigmotropisme");
    assert_ne!(colonie.scenario().chimiotropisme, 0.0, "colonie-mur : le chimiotropisme est allumé");

    let cases = 32 * 32 * 32;
    let longueur_attendue = |colonie: &Colonie3d| {
        let apex = colonie.apex().len();
        let obstacles = colonie.scenario().obstacles.len();
        // 4 (temps) + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases, plus le chimiotropisme, plus la queue.
        4 + 8
            + 56 * colonie.segments().len()
            + 8
            + 53 * apex
            + 8
            + 24
            + 8 * cases
            + 8
            + (8 + 24 * obstacles + 4 + 49 * apex)
    };

    let octets = colonie.ecrire_canonique();
    assert_eq!(octets.len(), longueur_attendue(&colonie), "au temps 0 : la longueur du contrat");

    // La queue de l'étape 23 est bien la dernière chose écrite : comptes, obstacles, seuil, mémoires.
    let apex = colonie.apex().len();
    let obstacles = colonie.scenario().obstacles.len();
    assert_eq!(
        &octets[octets.len() - 49 * apex - 4..octets.len() - 49 * apex],
        &35u32.to_le_bytes()[..],
        "les quatre octets du seuil précèdent les mémoires des apex"
    );
    let debut_obstacles = octets.len() - 49 * apex - 4 - 24 * obstacles - 8;
    assert_eq!(
        &octets[debut_obstacles..debut_obstacles + 8],
        &(obstacles as u64).to_le_bytes()[..],
        "le nombre d'obstacles précède leurs pavés"
    );

    // Après des pas, la même formule vaut, avec les segments et les apex du moment.
    colonie.avancer_n(10);
    assert_eq!(colonie.temps(), 10, "dix pas joués");
    assert_eq!(colonie.ecrire_canonique().len(), longueur_attendue(&colonie), "après 10 pas : la longueur du contrat");
}

#[test]
#[cfg_attr(not(feature = "etape-23"), ignore)]
fn le_gradient_voit_un_mur_comme_un_bord_sans_flux() {
    let (largeur, hauteur, profondeur) = (5usize, 5usize, 5usize);
    let mut substrat =
        Substrat3d::nouveau(largeur, hauteur, profondeur, 0.0, 1.0, 1.0).expect("un substrat sans diffusion");
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                // Un champ affine, exact en binaire : 1 + 0.25 x + 0.5 y + 0.125 z.
                substrat.deposer(x, y, z, 1.0 + 0.25 * x as f64 + 0.5 * y as f64 + 0.125 * z as f64);
            }
        }
    }
    substrat.bloquer(2, 2, 2, 2, 2, 2);

    // La case bloquée rend zéro, comme en 2D.
    assert_eq!(gradient_3d(&substrat, 2, 2, 2), Some((0.0, 0.0, 0.0)), "le gradient d'une case bloquée");

    // Chacune des six voisines voit la case bloquée comme elle-même (aucun flux) :
    // (x−1, y, z), (x+1, y, z), (x, y−1, z), (x, y+1, z), (x, y, z−1), (x, y, z+1).
    let attendus = [
        ((1, 2, 2), (0.125, 0.5, 0.125)),
        ((3, 2, 2), (0.125, 0.5, 0.125)),
        ((2, 1, 2), (0.25, 0.25, 0.125)),
        ((2, 3, 2), (0.25, 0.25, 0.125)),
        ((2, 2, 1), (0.25, 0.5, 0.0625)),
        ((2, 2, 3), (0.25, 0.5, 0.0625)),
    ];
    let mut vues = 0;
    for ((x, y, z), attendu) in attendus {
        assert_eq!(gradient_3d(&substrat, x, y, z), Some(attendu), "la voisine ({x}, {y}, {z}) de la case bloquée");
        vues += 1;
    }
    assert_eq!(vues, 6, "les six voisines ont été vues");
}
