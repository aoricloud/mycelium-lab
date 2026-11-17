//! Tests d'acceptation de l'étape 22 : le chimiotropisme en volume.
//!
//! Ce qui est vérifié :
//! - les empreintes de `colonie-chimio`, graine 1 : au temps 0, après 1 et après 10 pas (valeurs écrites ici), puis à
//!   la fin de sa durée (l'empreinte de son fichier de `references/`), et les comptes de la fin ;
//! - sans chimiotropisme, rien ne change : `colonie-cube` et `colonie-goutte` gardent leur empreinte, une ligne
//!   `chimiotropisme 0` donne les mêmes octets qu'aucune ligne, `colonie-chimio` à force nulle est `colonie-goutte` ;
//! - l'écriture canonique : les 8 octets de la force à la fin, 8 octets de plus que sans chimiotropisme ;
//! - les tirages de `tourner_chimiotropique_3d` : aucun si la force ou le gradient est nul, un seul si le gradient
//!   est parallèle à la direction (la direction est alors rendue telle quelle) ;
//! - le virage, refait à la main avec un aléa témoin de même graine : l'angle, le sens, le plan ;
//! - `gradient_3d` : différences centrées à l'intérieur, demi-différences au bord, `None` hors du cube ;
//! - la colonie va vers la goutte : plus de bouts de segments du côté de la goutte, plus près de son centre, que la
//!   colonie sans chimiotropisme, pour les graines 1 à 5 ;
//! - le déterminisme et la nourriture : mêmes octets pour une même graine, `avancer_n(n)` comme `n` appels à
//!   `avancer`, la masse perdue égale au coût des segments ;
//! - les erreurs de lecture du mot `chimiotropisme`, avec leur numéro de ligne.
//!
//! Ils n'emploient aucune fonction de `f64` interdite : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor` et `as` seulement
//! (les cosinus et sinus viennent des tables `COS` et `SIN`).

use std::path::Path;

use mycelium_lab::alea::Alea;
use mycelium_lab::colonie3d::{
    Colonie3d, ErreurColonie3d, ScenarioColonie3d, gradient_3d, lire_scenario_colonie3d, tourner_chimiotropique_3d,
};
use mycelium_lab::croissance::{COS, SIN};
use mycelium_lab::direction3d::Direction3d;
use mycelium_lab::substrat3d::Substrat3d;
use mycelium_lab::tropisme::GRADIENT_DE_REFERENCE;

/// Le centre de la goutte de `colonie-chimio` et de `colonie-goutte` : la case `(20, 16, 16)`, en son centre.
const CENTRE_DE_LA_GOUTTE: (f64, f64, f64) = (20.5, 16.5, 16.5);

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

/// L'erreur exacte d'un texte que la lecture doit refuser.
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

/// La direction du vecteur `(x, y, z)`.
fn direction(x: f64, y: f64, z: f64) -> Direction3d {
    Direction3d::nouvelle(x, y, z).expect("un vecteur non nul et fini")
}

/// Le virage entier, en degrés, que le contrat tire pour un virage réel `force · s · sinus` : l'arrondi au sort de
/// la 2D, avec un tirage de l'aléa `temoin` (toujours un), puis le bornage à 180.
fn virage_entier(force: f64, norme: f64, sinus: f64, temoin: &mut Alea) -> usize {
    let s = norme / (norme + GRADIENT_DE_REFERENCE);
    let virage = (force * s) * sinus;
    let plancher = virage.floor();
    let fraction = virage - plancher;
    let mut t = plancher as usize;
    if temoin.suivant_f64() < fraction {
        t += 1;
    }
    t.min(180)
}

/// Les bouts des segments d'une colonie : la part de ceux du côté de la goutte (`x1 > 10.5`) et leur distance
/// moyenne au centre de la goutte.
fn bouts_et_goutte(colonie: &Colonie3d) -> (f64, f64) {
    let segments = colonie.segments();
    assert!(!segments.is_empty(), "des segments à mesurer");
    let (mut du_cote, mut distances) = (0usize, 0.0);
    for segment in segments {
        if segment.x1 > 10.5 {
            du_cote += 1;
        }
        let dx = segment.x1 - CENTRE_DE_LA_GOUTTE.0;
        let dy = segment.y1 - CENTRE_DE_LA_GOUTTE.1;
        let dz = segment.z1 - CENTRE_DE_LA_GOUTTE.2;
        distances += (dx * dx + dy * dy + dz * dz).sqrt();
    }
    let n = segments.len() as f64;
    (du_cote as f64 / n, distances / n)
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

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn colonie_chimio_a_les_empreintes_ecrites_et_celle_de_son_fichier() {
    let mut colonie = colonie("colonie-chimio.txt", 1);
    assert_eq!(
        colonie.empreinte(),
        "f3eacfb025f475e3b5b3684a9fcf13a6e54874beb3ad0a876dbe5d95e8860dc6",
        "colonie-chimio : l'empreinte au temps 0"
    );
    colonie.avancer_n(1);
    assert_eq!(colonie.temps(), 1, "colonie-chimio : un pas joué");
    assert_eq!(
        colonie.empreinte(),
        "8af9b4b556ebd5e7ba3cd0f0df38eaedfdc1da8445d9ce8f6876b7973cc8ec8b",
        "colonie-chimio : l'empreinte après 1 pas"
    );
    colonie.avancer_n(9);
    assert_eq!(colonie.temps(), 10, "colonie-chimio : dix pas joués");
    assert_eq!(
        colonie.empreinte(),
        "8a1d06c44b6d1e2fe750c295df7a33eac115383795dc6d8a92c51baea6029130",
        "colonie-chimio : l'empreinte après 10 pas"
    );
    let duree = colonie.duree();
    colonie.avancer_n(duree - 10);
    assert_eq!(colonie.temps(), duree, "colonie-chimio : le temps est la durée");
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-chimio.empreinte"),
        "colonie-chimio : l'empreinte de son fichier de références"
    );
    assert_eq!(colonie.segments().len(), 14215, "colonie-chimio : les segments de la fin");
    assert_eq!(colonie.nb_ramifications(), 2321, "colonie-chimio : les ramifications de la fin");
    assert_eq!(colonie.apex().len(), 1 + 2321, "colonie-chimio : un apex de plus que de ramifications");
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn sans_chimiotropisme_rien_ne_change() {
    for (nom, fichier) in
        [("colonie-cube.txt", "colonie-cube.empreinte"), ("colonie-goutte.txt", "colonie-goutte.empreinte")]
    {
        let colonie = colonie_finie(nom, 1);
        assert_eq!(colonie.temps(), colonie.duree(), "{nom} : le temps est la durée");
        assert_eq!(colonie.scenario().chimiotropisme, 0.0, "{nom} : pas de chimiotropisme");
        assert_eq!(colonie.empreinte(), empreinte_de_reference(fichier), "{nom} : l'empreinte de son fichier");
    }

    // Une ligne `chimiotropisme 0` donne les mêmes octets canoniques qu'aucune ligne.
    let sans_ligne = texte_du_fichier("colonie-goutte.txt");
    let avec_zero = format!("{sans_ligne}\nchimiotropisme 0\n");
    let sans = colonie_du_texte(&sans_ligne, 1);
    let zero = colonie_du_texte(&avec_zero, 1);
    assert_eq!(zero.scenario().chimiotropisme, 0.0, "la force lue est nulle");
    assert_eq!(zero.ecrire_canonique(), sans.ecrire_canonique(), "au temps 0 : mêmes octets avec `chimiotropisme 0`");
    let (mut sans, mut zero) = (sans, zero);
    sans.avancer_n(sans.duree());
    zero.avancer_n(zero.duree());
    assert_eq!(zero.ecrire_canonique(), sans.ecrire_canonique(), "à la fin : mêmes octets avec `chimiotropisme 0`");

    // colonie-chimio à force nulle est colonie-goutte.
    let chimio = texte_du_fichier("colonie-chimio.txt");
    assert!(chimio.contains("chimiotropisme 10"), "la ligne de force de colonie-chimio est là");
    let eteinte = chimio.replace("chimiotropisme 10", "chimiotropisme 0");
    assert_ne!(eteinte, chimio, "la force de colonie-chimio a été mise à 0");
    let mut colonie = colonie_du_texte(&eteinte, 1);
    colonie.avancer_n(colonie.duree());
    assert_eq!(
        colonie.empreinte(),
        empreinte_de_reference("colonie-goutte.empreinte"),
        "colonie-chimio à force 0 : l'empreinte de colonie-goutte"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn l_ecriture_canonique_finit_par_la_force_et_gagne_huit_octets() {
    let force = 10.0f64;
    let attendu = force.to_bits().to_le_bytes();
    let chimio = colonie("colonie-chimio.txt", 1);
    let goutte = colonie("colonie-goutte.txt", 1);
    assert_eq!(chimio.scenario().chimiotropisme, force, "colonie-chimio : la force lue");

    let avec = chimio.ecrire_canonique();
    let sans = goutte.ecrire_canonique();
    assert_eq!(avec.len(), sans.len() + 8, "au temps 0 : 8 octets de plus que sans chimiotropisme");
    assert_eq!(avec[avec.len() - 8..], attendu, "au temps 0 : les 8 octets petit-boutistes de 10.0 à la fin");
    assert_eq!(avec[..sans.len()], sans[..], "au temps 0 : le reste de l'écriture est celui d'avant");
    // La longueur du contrat : 4 + 8 + 56 · segments + 8 + 53 · apex + 8 + 24 + 8 · cases, plus 8.
    let cases = 32 * 32 * 32;
    assert_eq!(avec.len(), 4 + 8 + 8 + 53 + 8 + 24 + 8 * cases + 8, "au temps 0 : la longueur du contrat");

    // Après des pas, la force est toujours la dernière chose écrite.
    let mut chimio = chimio;
    chimio.avancer_n(10);
    let octets = chimio.ecrire_canonique();
    assert!(!chimio.segments().is_empty(), "des segments laissés en 10 pas");
    assert_eq!(octets[octets.len() - 8..], attendu, "après 10 pas : les 8 octets de 10.0 à la fin");
    assert_eq!(
        octets.len(),
        4 + 8 + 56 * chimio.segments().len() + 8 + 53 * chimio.apex().len() + 8 + 24 + 8 * cases + 8,
        "après 10 pas : la longueur du contrat"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn le_virage_ne_tire_rien_si_la_force_ou_le_gradient_est_nul() {
    let d = direction(0.0, 1.0, 0.0);

    // Force nulle, gradient fort : la direction telle quelle, aucun tirage.
    let mut alea = Alea::new(7);
    let mut temoin = Alea::new(7);
    let rendue = tourner_chimiotropique_3d(d, (0.25, 0.5, -0.75), 0.0, &mut alea);
    assert_eq!(rendue, d, "force nulle : la direction rendue telle quelle");
    assert_eq!(alea.suivant(), temoin.suivant(), "force nulle : aucun tirage");

    // Gradient nul, force forte : la direction telle quelle, aucun tirage.
    let mut alea = Alea::new(7);
    let mut temoin = Alea::new(7);
    let rendue = tourner_chimiotropique_3d(d, (0.0, 0.0, 0.0), 10.0, &mut alea);
    assert_eq!(rendue, d, "gradient nul : la direction rendue telle quelle");
    assert_eq!(alea.suivant(), temoin.suivant(), "gradient nul : aucun tirage");

    // Un gradient non nul et une force non nulle tirent bel et bien : le témoin le voit.
    let mut alea = Alea::new(7);
    let mut temoin = Alea::new(7);
    let _ = tourner_chimiotropique_3d(d, (0.25, 0.0, 0.0), 10.0, &mut alea);
    temoin.suivant_f64();
    assert_eq!(alea.suivant(), temoin.suivant(), "gradient non nul : exactement un tirage");
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn un_gradient_parallele_fait_un_tirage_et_rend_la_direction() {
    let d = direction(0.0, 1.0, 0.0);
    // Parallèle, puis antiparallèle : la part perpendiculaire est nulle, le sinus aussi.
    for (nom, g) in [("parallèle", (0.0, 0.3, 0.0)), ("antiparallèle", (0.0, -0.3, 0.0))] {
        for graine in [1u64, 2, 3] {
            let mut alea = Alea::new(graine);
            let mut temoin = Alea::new(graine);
            let rendue = tourner_chimiotropique_3d(d, g, 10.0, &mut alea);
            assert_eq!(rendue, d, "gradient {nom}, graine {graine} : la direction rendue telle quelle");
            temoin.suivant_f64();
            assert_eq!(alea.suivant(), temoin.suivant(), "gradient {nom}, graine {graine} : exactement un tirage");
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn le_virage_a_la_main_va_vers_le_gradient_de_l_angle_tire() {
    let d = direction(0.0, 1.0, 0.0);
    let g = (0.25, 0.0, 0.0);
    let force = 10.0;

    // Le virage réel vaut 10 · 0.25 / (0.25 + 0.01), soit environ 9,615 degrés : 9, ou 10 au sort.
    let virage_a_la_main: f64 = 10.0 * 0.25 / (0.25 + 0.01);
    assert!((virage_a_la_main - 9.615_384_615_384_615).abs() <= 1e-12, "le virage à la main : {virage_a_la_main}");

    let mut au_moins_un_de_chaque = [false; 2];
    for graine in 1..=40u64 {
        let mut alea = Alea::new(graine);
        let mut temoin = Alea::new(graine);
        let t = virage_entier(force, 0.25, 1.0, &mut temoin);
        assert!((9..=10).contains(&t), "graine {graine} : le virage entier {t} est 9 ou 10");
        au_moins_un_de_chaque[t - 9] = true;

        let rendue = tourner_chimiotropique_3d(d, g, force, &mut alea);
        assert_eq!(alea.suivant(), temoin.suivant(), "graine {graine} : un seul tirage, comme le témoin");
        let scalaire = rendue.scalaire(&d);
        assert!(
            (scalaire - COS[t]).abs() <= 1e-15,
            "graine {graine} : produit scalaire {scalaire} contre COS[{t}] = {}",
            COS[t]
        );
        assert!(rendue.x() > 0.0, "graine {graine} : le virage va vers le gradient, x = {}", rendue.x());
        assert!(
            (rendue.x() - SIN[t]).abs() <= 1e-15,
            "graine {graine} : x = {} contre SIN[{t}] = {}",
            rendue.x(),
            SIN[t]
        );
        assert!(rendue.z().abs() <= 1e-15, "graine {graine} : la pointe reste dans le plan (d, g), z = {}", rendue.z());
    }
    assert!(au_moins_un_de_chaque[0] && au_moins_un_de_chaque[1], "sur 40 graines, 9 et 10 degrés sortent tous deux");

    // Dans les autres sens : c'est le gradient qui décide du côté, quelle que soit la pointe.
    let cas: [(&str, Direction3d, (f64, f64, f64)); 3] = [
        ("gradient en -x", direction(0.0, 1.0, 0.0), (-0.25, 0.0, 0.0)),
        ("gradient en -z", direction(1.0, 0.0, 0.0), (0.0, 0.0, -0.5)),
        ("gradient en y, pointe en z", direction(0.0, 0.0, 1.0), (0.0, 2.0, 0.0)),
    ];
    for (nom, d, g) in cas {
        let norme = (g.0 * g.0 + g.1 * g.1 + g.2 * g.2).sqrt();
        let mut alea = Alea::new(5);
        let mut temoin = Alea::new(5);
        let t = virage_entier(force, norme, 1.0, &mut temoin);
        assert!(t > 0, "{nom} : un virage non nul");
        let rendue = tourner_chimiotropique_3d(d, g, force, &mut alea);
        assert_eq!(alea.suivant(), temoin.suivant(), "{nom} : un seul tirage, comme le témoin");
        let scalaire = rendue.scalaire(&d);
        assert!((scalaire - COS[t]).abs() <= 1e-15, "{nom} : produit scalaire {scalaire} contre COS[{t}]");
        // Le virage se fait vers le gradient : le produit scalaire avec lui croît.
        let direction_du_gradient = direction(g.0, g.1, g.2);
        assert!(
            rendue.scalaire(&direction_du_gradient) > d.scalaire(&direction_du_gradient),
            "{nom} : la pointe s'est rapprochée du gradient"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn le_gradient_est_centre_a_l_interieur_et_demi_au_bord() {
    let (largeur, hauteur, profondeur) = (5usize, 4usize, 3usize);
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
    // La pente par axe : un quart, une demie, un huitième ; la moitié de cette pente sur une case de bord.
    let pente = |i: usize, n: usize, p: f64| if i == 0 || i + 1 == n { p * 0.5 } else { p };
    let mut vues = 0;
    for z in 0..profondeur {
        for y in 0..hauteur {
            for x in 0..largeur {
                let attendu = (pente(x, largeur, 0.25), pente(y, hauteur, 0.5), pente(z, profondeur, 0.125));
                assert_eq!(gradient_3d(&substrat, x, y, z), Some(attendu), "gradient de la case ({x}, {y}, {z})");
                vues += 1;
            }
        }
    }
    assert_eq!(vues, largeur * hauteur * profondeur, "toutes les cases du cube ont été vues");

    // Une case intérieure, écrite en toutes lettres : (0.25, 0.5, 0.125) ; au bord en x : la moitié en x seulement.
    assert_eq!(gradient_3d(&substrat, 2, 1, 1), Some((0.25, 0.5, 0.125)), "une case intérieure");
    assert_eq!(
        gradient_3d(&substrat, 0, 1, 1),
        Some((0.125, 0.5, 0.125)),
        "bord x = 0 : la voisine absente vaut la case"
    );
    assert_eq!(
        gradient_3d(&substrat, 4, 1, 1),
        Some((0.125, 0.5, 0.125)),
        "bord x = 4 : la voisine absente vaut la case"
    );
    assert_eq!(gradient_3d(&substrat, 0, 0, 0), Some((0.125, 0.25, 0.0625)), "le coin : la moitié sur chaque axe");

    // Hors du cube : None, sans paniquer.
    assert_eq!(gradient_3d(&substrat, largeur, 0, 0), None, "x hors du cube");
    assert_eq!(gradient_3d(&substrat, 0, hauteur, 0), None, "y hors du cube");
    assert_eq!(gradient_3d(&substrat, 0, 0, profondeur), None, "z hors du cube");
    assert_eq!(gradient_3d(&substrat, usize::MAX, usize::MAX, usize::MAX), None, "très loin du cube");
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn le_gradient_d_un_champ_a_un_quart_par_case_est_un_quart() {
    // Le cas du sujet, par un scénario : 1 + 0.25 x, sans diffusion, par des lignes `nourriture`.
    let mut texte =
        String::from("cube 6 3 3\nsubstrat 1.0 0.0\nspore 3 1 1\ncroissance 0.4 6 35 20\ncouplage 0.1 0.5\nduree 5\n");
    for z in 0..3 {
        for y in 0..3 {
            for x in 1..6 {
                texte.push_str(&format!("nourriture {x} {y} {z} {}\n", 0.25 * x as f64));
            }
        }
    }
    let colonie = colonie_du_texte(&texte, 1);
    let substrat = colonie.substrat();
    assert_eq!(substrat.valeur(4, 1, 1), 2.0, "le champ vaut 1 + 0.25 x : 2 en x = 4");
    assert_eq!(gradient_3d(substrat, 2, 1, 1), Some((0.25, 0.0, 0.0)), "case intérieure : (0.25, 0, 0)");
    assert_eq!(gradient_3d(substrat, 0, 1, 1), Some((0.125, 0.0, 0.0)), "bord x = 0 : la moitié");
    assert_eq!(gradient_3d(substrat, 5, 1, 1), Some((0.125, 0.0, 0.0)), "bord x = 5 : la moitié");
    assert_eq!(gradient_3d(substrat, 6, 1, 1), None, "hors du cube : None");
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn la_colonie_chimio_va_vers_la_goutte_plus_que_la_colonie_goutte() {
    for graine in 1..=5u64 {
        let chimio = colonie_finie("colonie-chimio.txt", graine);
        let goutte = colonie_finie("colonie-goutte.txt", graine);
        let (part_chimio, distance_chimio) = bouts_et_goutte(&chimio);
        let (part_goutte, distance_goutte) = bouts_et_goutte(&goutte);
        assert!(
            part_chimio > part_goutte,
            "graine {graine} : la part du côté de la goutte, {part_chimio} avec chimiotropisme contre {part_goutte}"
        );
        assert!(
            distance_chimio < distance_goutte,
            "graine {graine} : la distance moyenne à la goutte, {distance_chimio} avec chimiotropisme contre {distance_goutte}"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn colonie_chimio_est_deterministe_et_perd_la_masse_de_ses_segments() {
    let scenario = scenario("colonie-chimio.txt");
    let duree = scenario.duree;

    let mut d_un_coup = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    let mut un_a_un = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    d_un_coup.avancer_n(duree);
    for _ in 0..duree {
        un_a_un.avancer();
    }
    assert_eq!(un_a_un.temps(), d_un_coup.temps(), "le même temps");
    assert_eq!(
        un_a_un.ecrire_canonique(),
        d_un_coup.ecrire_canonique(),
        "avancer_n(n) et n appels à avancer, les mêmes octets"
    );

    let mut encore = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    encore.avancer_n(duree);
    assert_eq!(encore.ecrire_canonique(), d_un_coup.ecrire_canonique(), "même graine, mêmes octets");

    let mut autre_graine = Colonie3d::nouvelle(&scenario, 2).expect("le scénario se construit");
    autre_graine.avancer_n(duree);
    assert_ne!(autre_graine.empreinte(), d_un_coup.empreinte(), "les graines 1 et 2 donnent deux empreintes");

    // La nourriture : la masse du départ moins le coût des segments est la masse de la fin.
    let mut colonie = Colonie3d::nouvelle(&scenario, 1).expect("le scénario se construit");
    let masse_initiale = colonie.substrat().masse_totale();
    colonie.avancer_n(duree);
    let mut consomme = 0.0;
    for segment in colonie.segments() {
        consomme += scenario.cout * segment.longueur();
    }
    let masse_finale = colonie.substrat().masse_totale();
    assert!(!colonie.segments().is_empty(), "des segments consommés");
    assert!(consomme > 0.0, "de la nourriture consommée : {consomme}");
    assert!(
        egal_relatif(masse_initiale - consomme, masse_finale, 1e-9),
        "{masse_initiale} - {consomme} contre la masse finale {masse_finale}"
    );
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn les_lignes_chimiotropisme_mal_formees_sont_refusees_avec_leur_numero() {
    let refusees = [
        ("une force de 46", "chimiotropisme 46"),
        ("une force négative", "chimiotropisme -1"),
        ("une force `nan`", "chimiotropisme nan"),
        ("une force `inf`", "chimiotropisme inf"),
        ("une force illisible", "chimiotropisme dix"),
        ("un mot manquant", "chimiotropisme"),
        ("un mot en trop", "chimiotropisme 10 20"),
    ];
    let mut vues = 0;
    for (nom, ligne) in refusees {
        assert_eq!(
            erreur(&avec_lignes_en_plus(&[ligne])),
            ErreurColonie3d::LigneInvalide(8),
            "{nom}, à la ligne 8 : `{ligne}`"
        );
        // Au milieu du texte, le numéro est celui de sa ligne.
        assert_eq!(
            erreur(&avec_ligne(2, &format!("{ligne}\nsubstrat 1.0 0.1"))),
            ErreurColonie3d::LigneInvalide(2),
            "{nom}, à la ligne 2 : `{ligne}`"
        );
        vues += 1;
    }
    assert_eq!(vues, 7, "les sept refus ont été joués");

    // Les bornes sont permises : 0 et 45 passent, et la force lue est celle du texte.
    for (ligne, force) in [("chimiotropisme 0", 0.0), ("chimiotropisme 45", 45.0), ("chimiotropisme 2.5", 2.5)] {
        let lu =
            lire_scenario_colonie3d(&avec_lignes_en_plus(&[ligne])).unwrap_or_else(|e| panic!("`{ligne}` : {e:?}"));
        assert_eq!(lu.chimiotropisme, force, "`{ligne}` : la force lue");
    }
    let sans = lire_scenario_colonie3d(&LIGNES.join("\n")).expect("le petit scénario se lit");
    assert_eq!(sans.chimiotropisme, 0.0, "sans la ligne, la force est nulle");
}

#[test]
#[cfg_attr(not(feature = "etape-22"), ignore)]
fn une_seconde_ligne_chimiotropisme_est_en_double() {
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["chimiotropisme 10", "chimiotropisme 5"])),
        ErreurColonie3d::EnDouble("chimiotropisme"),
        "deux lignes `chimiotropisme`"
    );
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["chimiotropisme 10", "chimiotropisme 10"])),
        ErreurColonie3d::EnDouble("chimiotropisme"),
        "deux lignes `chimiotropisme` identiques"
    );
    assert_eq!(
        erreur(&avec_lignes_en_plus(&["chimiotropisme 0", "chimiotropisme 0"])),
        ErreurColonie3d::EnDouble("chimiotropisme"),
        "deux lignes `chimiotropisme 0` : une force nulle ne dispense pas du refus"
    );
}
