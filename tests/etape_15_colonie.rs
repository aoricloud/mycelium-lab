//! Étape 15 (M3) : des colonies simulées. Le réseau retrouve exactement les anastomoses du noyau (avec ou sans obstacle), « la
//! colonie traverse » s'accorde avec `piece::traversee`, la raideur d'une colonie qui pousse ne baisse pas, et l'exemple
//! `colonie_raideur` écrit ses nombres avec leur statut et une image.

#[path = "../examples/colonie_raideur.rs"]
mod exemple;
#[path = "etape_15/outils.rs"]
mod outils;

use mycelium_lab::anastomose::{Index, distance_au_segment};
use mycelium_lab::moule::developper;
use mycelium_lab::piece::{Bord, mesurer};
use mycelium_lab::reseau::{Contact, Hyphe, Origine, contacts, extraire, pousser};
use mycelium_lab::simulation::{Simulation, lire_scenario_tropismes};
use outils::{bits, egal, jouer};

const HYPHE: Hyphe = Hyphe { ea: exemple::EA, ei: exemple::EI };

fn lire(chemin: &str) -> String {
    let complet = format!("{}/{chemin}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&complet).unwrap_or_else(|e| panic!("{complet} : {e}"))
}

/// La règle d'anastomose du noyau, refaite ici sans index, par une recherche sur tous les segments : le segment que la pointe
/// du segment `pointe` rejoint. Un candidat est d'un autre apex, assez ancien, à `rayon` au plus, et aucun obstacle ne le
/// sépare de la pointe ; le plus proche l'emporte, et à distance égale le plus ancien.
fn cible_attendue(simulation: &Simulation, pointe: usize) -> Option<usize> {
    let (segments, reglages) = (simulation.segments(), simulation.reglages());
    let (reglage, p) = (reglages.anastomose?, &segments[pointe]);
    let mut meilleure: Option<(f64, usize)> = None;
    for (i, s) in segments[..pointe].iter().enumerate() {
        let distance = distance_au_segment(p.x1, p.y1, s);
        if s.apex == p.apex || p.temps < s.temps + reglage.delai || distance > reglage.rayon {
            continue;
        }
        // Le point de `s` le plus proche de la pointe, puis la ligne libre de la pointe à ce point.
        let (dx, dy) = (s.x1 - s.x0, s.y1 - s.y0);
        let t = (((p.x1 - s.x0) * dx + (p.y1 - s.y0) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        let (l, h) = (reglages.largeur as f64, reglages.hauteur as f64);
        let libre = simulation.substrat().segment_libre(
            p.x1,
            p.y1,
            (s.x0 + t * dx).clamp(0.0, l),
            (s.y0 + t * dy).clamp(0.0, h),
        );
        if libre && meilleure.is_none_or(|(d, _)| distance < d) {
            meilleure = Some((distance, i));
        }
    }
    meilleure.map(|(_, i)| i)
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn les_contacts_rejoues_sont_exactement_les_anastomoses_du_noyau_avec_ou_sans_obstacle() {
    // Les six scénarios de la page (dix graines chacun ; trois ont un mur ou des murs), la brique (trois graines) et le
    // labyrinthe (deux : des murs partout, le thigmotropisme, 15 000 anastomoses par colonie) : autant de contacts que le
    // compteur du noyau. Pour les six premiers, chaque segment est repris par une recherche sans index : les contacts sont
    // exactement les segments qui trouvent une cible, et la cible est la même.
    let page = ["couplage", "terrain", "chimiotropisme", "chimiotropisme-sans", "mur", "mur-sans"]
        .map(|n| format!("scenarios/{n}.txt"));
    let moules = ["brique", "labyrinthe"].map(|n| format!("scenarios/moules/{n}.txt"));
    let (mut total, mut repris) = (0, 0);
    for (fichier, graines) in page.iter().map(|f| (f, 1..=10)).chain([(&moules[0], 1..=3), (&moules[1], 1..=2)]) {
        let texte = developper(&lire(fichier)).expect("le scénario se développe");
        let duree = lire_scenario_tropismes(&texte).unwrap().3;
        for graine in graines {
            let (simulation, liste, _) = jouer(&texte, graine, duree);
            assert_eq!(liste.len() as u64, simulation.anastomoses(), "{fichier}, graine {graine}");
            total += liste.len();
            if page.contains(fichier) {
                let attendus: Vec<Contact> = (0..simulation.segments().len())
                    .filter_map(|pointe| cible_attendue(&simulation, pointe).map(|cible| Contact { pointe, cible }))
                    .collect();
                assert_eq!(liste, attendus, "{fichier}, graine {graine}");
                repris += liste.len();
            }
        }
    }
    assert_eq!((total, repris), (39_780, 543), "les contacts de ces 65 colonies, dont ceux repris un à un (mesuré)");
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn le_rejeu_suit_la_regle_de_l_obstacle_quand_un_mur_separe_une_pointe_d_une_hyphe() {
    // Aux réglages de M2 (rayon 0,5), un mur épais d'au moins une case ne sépare jamais une pointe d'une hyphe : la règle de
    // l'obstacle ne change rien. Avec un rayon de 1,5 et un mur d'une case (la colonne 20), elle écarte des candidats : le
    // noyau compte 237, 359 et 418 anastomoses (graines 2, 7 et 8), le rejeu aussi, avec les mêmes cibles que la recherche sans
    // index ; sans la règle (la recherche du noyau sans obstacle, refaite ici) on en compterait 368, 424 et 728.
    let texte = [
        "boite 40 30",
        "spore 5 15",
        "substrat 1.0 0.2",
        "croissance 1.0 7.5 30 20",
        "couplage 0.1 0.002",
        "anastomose 1.5 10",
        "duree 100",
        "obstacle 20 5 20 24",
        "thigmotropisme 35",
    ]
    .join("\n");
    for (graine, anastomoses, sans_la_regle) in [(2, 237, 368), (7, 359, 424), (8, 418, 728)] {
        let (simulation, liste, _) = jouer(&texte, graine, 100);
        assert_eq!((liste.len(), simulation.anastomoses()), (anastomoses, anastomoses as u64), "graine {graine}");
        let attendus: Vec<Contact> = (0..simulation.segments().len())
            .filter_map(|pointe| cible_attendue(&simulation, pointe).map(|cible| Contact { pointe, cible }))
            .collect();
        assert_eq!(liste, attendus, "graine {graine}");
        let (reglage, mut index, mut naif) = (simulation.reglages().anastomose.unwrap(), Index::new(40, 30), 0);
        for s in simulation.segments() {
            index.ajouter(*s);
            naif += usize::from(index.chercher(s.x1, s.y1, s.apex, s.temps, &reglage).is_some());
        }
        assert_eq!(naif, sans_la_regle, "graine {graine} : la recherche sans la règle de l'obstacle (mesuré)");
    }
    // Sans réglage d'anastomose, le noyau n'en fait aucune et le rejeu ne trouve aucun contact.
    let sans = texte.replace("anastomose 1.5 10\n", "");
    let (simulation, liste, _) = jouer(&sans, 2, 100);
    assert!(simulation.reglages().anastomose.is_none() && liste.is_empty() && simulation.anastomoses() == 0);
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn la_colonie_traverse_au_sens_des_noeuds_exactement_quand_piece_traversee_trouve_un_chemin() {
    // La petite boîte de l'exemple (30 × 20, la spore près du bord gauche), quinze graines, jouées de 10 en 10 pas jusqu'à
    // 100 : de la gauche vers la droite et du haut vers le bas, les deux mesures donnent la même réponse, et K > 0 exactement
    // quand la colonie traverse.
    let (mut oui, mut non) = (0, 0);
    for graine in [2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 37, 38, 42, 59, 60] {
        let (mut simulation, _, boite) = jouer(exemple::DEMONSTRATION, graine, 0);
        for pas in (10..=100).step_by(10) {
            simulation.avancer_n(10);
            let reseau = extraire(simulation.segments(), &contacts(&simulation), HYPHE).unwrap();
            let piece = mesurer(&simulation);
            for (de, vers, cases) in [
                (Bord::Gauche, Bord::Droite, piece.traversee_gauche_droite),
                (Bord::Haut, Bord::Bas, piece.traversee_haut_bas),
            ] {
                let p = pousser(&reseau, boite, de, vers, 1.0).unwrap();
                let traverse = p.tenus > 0 && p.pousses > 0;
                assert_eq!(traverse, cases.is_some(), "graine {graine}, {pas} pas, de {de:?} à {vers:?}");
                assert_eq!((p.raideur > 0.0, p.solution.is_some()), (traverse, traverse), "graine {graine}, {pas} pas");
                if traverse { oui += 1 } else { non += 1 }
            }
        }
    }
    assert_eq!((oui, non), (46, 254), "300 comparaisons : 46 traversées, 254 non (mesuré)");
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn la_raideur_d_une_colonie_qui_pousse_ne_baisse_pas_et_les_ponts_la_relevent() {
    // La graine 8 de la petite boîte : elle touche le bord droit vers le pas 34. Un pas sur dix, avec et sans les ponts. Ni
    // l'une ni l'autre raideur ne baisse, et les ponts ne l'abaissent pas : à 1e-6 près, la précision d'une colonie simulée
    // (la plus forte baisse mesurée sur cette courbe est sous 1e-10 : une marge de 10 000).
    let (mut simulation, _, boite) = jouer(exemple::DEMONSTRATION, 8, 0);
    let (mut avec, mut sans, mut courbe) = (0.0, 0.0, Vec::new());
    for _ in 0..10 {
        simulation.avancer_n(10);
        let k = |liste: &[Contact]| {
            let reseau = extraire(simulation.segments(), liste, HYPHE).unwrap();
            pousser(&reseau, boite, Bord::Gauche, Bord::Droite, 1.0).unwrap().raideur
        };
        let (nouvelle_avec, nouvelle_sans) = (k(&contacts(&simulation)), k(&[]));
        let pas = simulation.temps();
        assert!(nouvelle_avec >= avec * (1.0 - 1e-6) && nouvelle_sans >= sans * (1.0 - 1e-6), "{pas} pas : K baisse");
        assert!(nouvelle_avec >= nouvelle_sans * (1.0 - 1e-6), "{pas} pas : les ponts abaissent K");
        (avec, sans) = (nouvelle_avec, nouvelle_sans);
        courbe.push(nouvelle_avec);
    }
    // Zéro avant la traversée (jusqu'au pas 30), puis croissante : les valeurs mesurées aux pas 40, 60 et 100, à 1e-6 près.
    assert_eq!(courbe[..3], [0.0, 0.0, 0.0]);
    for (rang, attendue) in [(3, 4.812531776398288e-3), (5, 5.034065376434249e-3), (9, 5.6856035444105765e-3)] {
        assert!(egal(courbe[rang], attendue, 1e-6), "{} pas : K = {:e}", 10 * (rang + 1), courbe[rang]);
    }
    assert!(avec > 1.6 * sans, "les ponts relèvent la raideur de 68 % : {avec} contre {sans}");
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn l_exemple_donne_la_colonie_de_demonstration_ses_nombres_avec_leur_statut_et_son_image() {
    let calcule = || {
        exemple::calculer(exemple::DEMONSTRATION, exemple::GRAINE_DEMONSTRATION, exemple::PAS_DEMONSTRATION).unwrap()
    };
    let (r, encore) = (calcule(), calcule());
    let ponts = r.reseau.origines.iter().filter(|o| matches!(o, Origine::Pont(_))).count();
    assert_eq!((r.segments, r.reseau.noeuds.len(), r.reseau.poutres.len(), ponts), (79, 85, 90, 6));
    assert_eq!((r.anastomoses, r.contacts.len(), r.poussee.tenus, r.poussee.pousses), (6, 6, 6, 2));
    // Le nombre : mesuré (sept chiffres : le plus petit mode de ce réseau est de 1e-7), et refait au bit près.
    assert!(egal(r.poussee.raideur, 0.07519340022316712, 1e-6), "K = {}", r.poussee.raideur);
    assert_eq!(bits(&r.poussee), bits(&encore.poussee));
    // Le rapport : chaque nombre avec son statut, la conversion à 7 µm par case, et ce que le banc ne dit pas.
    let texte = exemple::rapport(&r);
    let (k, par_micrometre) = (r.poussee.raideur, r.poussee.raideur / 7.0);
    for ligne in [
        "réseau : 79 segments, 85 nœuds, 90 poutres dont 6 ponts pour 6 contacts ; anastomoses du noyau : 6\n".to_string(),
        "mors : 6 nœuds tenus (colonne de gauche), 2 poussés (colonne de droite) [choix : K en dépend]\n".to_string(),
        format!("raideur K = {k} µN par case [calculé : le réseau simulé, avec EA et EI donnés ; aucune raideur de colonie n'est publiée]\n"),
        format!("raideur K = {par_micrometre} µN/µm, soit N/m [calculé ; une case = 7 µm : choix]\n"),
        format!("énergie ½ K δ² = {} µN·case pour δ = 1 case [calculé]\n", r.poussee.energie),
        "EI = 0.3734693877551021 µN·case² [18,3 µN·µm² : lu à la source (Brinkmann et coll. 2024) ; conversion à 7 µm par case : choix]\n".to_string(),
        "EA = 22.9 µN [calculé : le module de la flexion pris isotrope, une extrapolation, pas une mesure de l'article]\n".to_string(),
        "règle de contact : la pointe est reliée au point le plus proche du segment visé [choix]\n".to_string(),
    ] {
        assert!(texte.contains(&ligne), "le rapport ne dit pas « {ligne} » :\n{texte}");
    }
    assert_eq!((exemple::EA, exemple::EI, exemple::DELTA), (22.9, 18.3 / 49.0, 1.0));
    // L'image : 30 × 20 cases de 12 pixels ; le mors tenu est bleu, le mors poussé rouge, le fond sombre ; les poutres vont du
    // violet (sans énergie) au jaune (la plus chargée) ; le PNG est celui de ces pixels.
    let (largeur, hauteur, pixels) = exemple::pixels(&r);
    assert_eq!((largeur, hauteur, pixels.len()), (360, 240, 3 * 360 * 240));
    let pixel = |x: usize, y: usize| pixels[3 * (y * largeur + x)..3 * (y * largeur + x) + 3].to_vec();
    assert_eq!(
        (pixel(2, 237), pixel(357, 237), pixel(180, 237), pixel(11, 237), pixel(348, 237)),
        (
            exemple::MORS_TENU.to_vec(),
            exemple::MORS_POUSSE.to_vec(),
            exemple::FOND.to_vec(),
            exemple::MORS_TENU.to_vec(),
            exemple::MORS_POUSSE.to_vec()
        )
    );
    assert_eq!((pixel(12, 237), pixel(347, 237)), (exemple::FOND.to_vec(), exemple::FOND.to_vec()));
    let couleurs: std::collections::BTreeSet<&[u8]> = pixels.chunks(3).collect();
    assert!(couleurs.contains(&exemple::JAUNE[..]) && couleurs.contains(&exemple::VIOLET[..]) && couleurs.len() > 30);
    assert!(!couleurs.contains(&exemple::GRIS[..]), "{} couleurs", couleurs.len());
    // Une poutre a la couleur de son énergie : la racine de son rapport à la plus grande, du violet au jaune (la seconde plus
    // chargée n'est ni l'un ni l'autre) ; et la spore (2,5 ; 10,5) est dessinée à sa place.
    let mut energies = r.poussee.energies.clone();
    energies.sort_by(|a, b| b.total_cmp(a));
    let niveau = ((energies[1] / energies[0]).sqrt() * 255.0) as i32;
    let seconde = [0, 1, 2].map(|c| {
        (exemple::VIOLET[c] as i32 + (exemple::JAUNE[c] as i32 - exemple::VIOLET[c] as i32) * niveau / 255) as u8
    });
    assert!(niveau > 0 && niveau < 255 && couleurs.contains(&seconde[..]), "niveau {niveau}");
    assert!(pixel(30, 126) != exemple::FOND && pixel(126, 30) == exemple::FOND, "la spore");
    // La poutre la plus chargée est jaune en son milieu : les couleurs suivent l'ordre des poutres.
    let chargee = (0..r.reseau.poutres.len()).find(|&i| r.poussee.energies[i] == energies[0]).unwrap();
    let (a, b) = (r.reseau.noeuds[r.reseau.poutres[chargee].a], r.reseau.noeuds[r.reseau.poutres[chargee].b]);
    assert_eq!(pixel((6.0 * (a.x + b.x)) as usize, (6.0 * (a.y + b.y)) as usize), exemple::JAUNE, "poutre {chargee}");
    assert_eq!(exemple::image(&r), mycelium_lab::image::ecrire_png(360, 240, &pixels));
    assert_eq!(pixels, exemple::pixels(&encore).2, "la même image");
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn l_exemple_dit_quand_la_colonie_ne_traverse_pas_ou_est_trop_grande_et_refuse_un_scenario_faux() {
    // Dix-neuf pas (43 segments ; un pas de plus en ferait 44) : la colonie n'atteint pas le bord droit, K = 0 sans erreur,
    // l'image est grise.
    let r = exemple::calculer(exemple::DEMONSTRATION, 42, 19).unwrap();
    assert_eq!((r.poussee.raideur, r.poussee.pousses > 0, r.poussee.tenus > 0, r.segments), (0.0, false, true, 43));
    let texte = exemple::rapport(&r);
    assert!(texte.contains("raideur K = 0 : la colonie ne touche pas les deux bords, aucun calcul [calculé]\n"));
    assert!(!texte.contains("µN par case") && !texte.contains("énergie"), "{texte}");
    let image = exemple::pixels(&r).2;
    let couleurs: std::collections::BTreeSet<&[u8]> = image.chunks(3).collect();
    assert!(couleurs.contains(&exemple::GRIS[..]) && couleurs.len() == 4, "le gris, le fond et les deux mors");
    // Un contact dont la pointe est à moins de `COURT` de l'hyphe visée ne laisse pas de pont : le rapport compte les deux.
    let r = exemple::calculer(exemple::DEMONSTRATION, 8, 60).unwrap();
    assert!(exemple::rapport(&r).contains(
        "188 nœuds, 195 poutres dont 7 ponts pour 8 contacts ; anastomoses du noyau : 8
"
    ));
    // Une colonie qui traverse mais compte plus de 1 000 nœuds (l'intervalle de ramification 4 : 2 749 segments à 55 pas) :
    // refusée, avec son motif ; un scénario illisible aussi.
    let dense = exemple::DEMONSTRATION.replace("7.5", "4.0");
    let erreur = exemple::calculer(&dense, 1, 55).err().expect("trop grande");
    assert!(erreur.contains("trop grande"), "{erreur}");
    assert!(exemple::calculer("pas un scénario", 1, 10).is_err());
}
