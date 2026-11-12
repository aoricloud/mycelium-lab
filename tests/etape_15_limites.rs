//! Étape 15 (M3) : les refus et les limites. Les données invalides, les grandeurs hors de la plage normale de `f64`, le
//! plafond de taille, les règles du dépôt et la section de `MODELE.md`.

#[path = "etape_15/outils.rs"]
mod outils;

use mycelium_lab::piece::Bord;
use mycelium_lab::poutres::Poutre;
use mycelium_lab::reseau::{ErreurReseau, Hyphe, INCONNUES_MAX, extraire, pousser};
use outils::{HYPHE, Ligne, a_la_main, chaine, egal, invalide, pousse, segments};

/// Les cas exacts de ce fichier (des chaînes en traction) : l'erreur relative mesurée est sous 5e-16 (une marge de 200).
const TOLERANCE: f64 = 1e-13;

fn lire(chemin: &str) -> String {
    let complet = format!("{}/{chemin}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&complet).unwrap_or_else(|e| panic!("{complet} : {e}"))
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn les_donnees_invalides_et_les_limites_sont_refusees_sans_rendre_de_nombre() {
    let un: [Ligne; 1] = [&[(0.5, 0.5), (1.5, 0.5)]];
    for mauvaise in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-310] {
        assert!(invalide(extraire(&segments(&un), &[], Hyphe { ea: mauvaise, ei: 3.0 })), "ea = {mauvaise:e}");
        assert!(invalide(extraire(&segments(&un), &[], Hyphe { ea: 5.0, ei: mauvaise })), "ei = {mauvaise:e}");
    }
    // Une coordonnée non finie ou de plus de 1e50, à chacune des quatre places ; un segment nul, ou de carré sous-normal.
    for mauvais in [f64::NAN, f64::INFINITY, 1e51, -1e51] {
        for place in 0..4 {
            let mut s = segments(&un);
            match place {
                0 => s[0].x0 = mauvais,
                1 => s[0].y0 = mauvais,
                2 => s[0].x1 = mauvais,
                _ => s[0].y1 = mauvais,
            }
            assert!(invalide(extraire(&s, &[], HYPHE)), "coordonnée {mauvais:e} à la place {place}");
        }
    }
    let (court, nul): ([Ligne; 1], [Ligne; 1]) = ([&[(0.0, 0.0), (0.0, 1e-160)]], [&[(0.5, 0.5), (0.5, 0.5)]]);
    assert!(invalide(extraire(&segments(&court), &[], HYPHE)) && invalide(extraire(&segments(&nul), &[], HYPHE)));
    assert!(extraire(&segments(&[&[(0.0, 0.0), (0.0, 1e-150)]]), &[], HYPHE).is_ok(), "1e-150 : son carré est normal");
    // Un contact dont un indice sort de la liste, ou dont la pointe est la cible.
    let deux: [Ligne; 2] = [&[(0.5, 0.5), (1.5, 0.5)], &[(1.5, 0.5), (2.5, 0.5)]];
    for (pointe, cible) in [(2, 0), (0, 2), (1, 1)] {
        assert!(invalide(extraire(&segments(&deux), &outils::contacts(&[(pointe, cible)]), HYPHE)), "{pointe} {cible}");
    }
    assert!(extraire(&segments(&deux), &outils::contacts(&[(1, 0)]), HYPHE).is_ok());
    // L'essai : delta nul, non fini, sous-normal ; deux bords qui ne sont pas opposés ; une boîte d'une case entre les bords.
    let reseau = chaine(&[(0.5, 0.5), (1.5, 0.5)]);
    for mauvais in [0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e-310] {
        assert!(invalide(pousser(&reseau, (2, 1), Bord::Gauche, Bord::Droite, mauvais)), "delta = {mauvais:e}");
    }
    for (de, vers) in
        [(Bord::Gauche, Bord::Gauche), (Bord::Gauche, Bord::Haut), (Bord::Haut, Bord::Droite), (Bord::Bas, Bord::Bas)]
    {
        assert!(invalide(pousser(&reseau, (2, 2), de, vers, 1.0)), "{de:?} {vers:?}");
    }
    let ailleurs = chaine(&[(1.5, 1.5), (2.5, 1.5)]);
    assert!(invalide(pousse(&ailleurs, (1, 5))), "une boîte d'une case de large, que la colonie y soit ou non");
    assert!(invalide(pousser(&ailleurs, (5, 1), Bord::Haut, Bord::Bas, 1.0)), "une boîte d'une case de haut");
    assert!(invalide(pousse(&reseau, (1, 5))) && invalide(pousser(&reseau, (5, 1), Bord::Haut, Bord::Bas, 1.0)));
    let k = pousser(&reseau, (2, 1), Bord::Gauche, Bord::Droite, -2.5).expect("un delta négatif est permis").raideur;
    assert!(egal(k, 5.0, TOLERANCE), "K = {k}");
    // Un réseau fait à la main : une poutre dont un nœud n'existe pas ; une poutre dont le cube de la longueur n'est pas un
    // nombre normal (1e-105 entre deux nœuds tenus ; 1e110), que `poutres` calculerait de travers ou prendrait pour un mécanisme.
    assert!(invalide(pousse(&a_la_main(&[(0.5, 0.5), (1.5, 0.5)], &[(0, 1), (1, 2)]), (2, 1))));
    assert!(invalide(pousse(&chaine(&[(0.5, 0.0), (0.5, 1e-105), (2.5, 0.0)]), (3, 1))));
    assert!(invalide(pousse(&chaine(&[(0.5, 0.5), (1.5, 1e110), (2.5, 0.5)]), (3, 1))));
    // Les grandeurs extrêmes : un résultat qui déborde (delta = 1e300 avec EA = 1e10 : `NaN`) ou s'efface (delta = 1e-300 : la force
    // ou l'énergie s'arrondit à zéro ; EA = 1e-308 avec delta = 1e10 : la force et l'énergie tiennent, la raideur est sous-normale)
    // est refusé ; de EA = 1e-250 à 1e300, avec delta = 1, le calcul est juste (à 5e-16 près, mesuré).
    for (ea, delta, refuse) in [
        (1e10, 1e300, true),
        (1e-100, 1e-300, true),
        (1e10, 1e-300, true),
        (1e-308, 1e10, true),
        (1e300, 1.0, false),
        (1e-250, 1.0, false),
    ] {
        let mut r = chaine(&[(0.5, 0.5), (1.5, 0.5)]);
        r.poutres[0] = Poutre { ea, ei: 0.6 * ea, ..r.poutres[0] };
        match pousser(&r, (2, 1), Bord::Gauche, Bord::Droite, delta) {
            Err(e) => assert!(
                refuse && matches!(e, ErreurReseau::DonneeInvalide(_)),
                "EA = {ea:e}, delta = {delta:e} : {e:?}"
            ),
            Ok(p) => {
                assert!(!refuse && egal(p.raideur, ea, 1e-13), "EA = {ea:e}, delta = {delta:e} : K = {}", p.raideur)
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn le_plafond_est_de_mille_noeuds_mille_passent_mille_et_un_sont_refuses() {
    assert_eq!(INCONNUES_MAX, 3000);
    // Un cadre de `largeur × hauteur` nœuds aux centres des cases, relié par ses côtés horizontaux et verticaux.
    let cadre = |largeur: usize, hauteur: usize| {
        let points: Vec<(f64, f64)> =
            (0..hauteur).flat_map(|j| (0..largeur).map(move |i| (i as f64 + 0.5, j as f64 + 0.5))).collect();
        let mut liens = Vec::new();
        for n in 0..largeur * hauteur {
            if n % largeur + 1 < largeur {
                liens.push((n, n + 1));
            }
            if n + largeur < largeur * hauteur {
                liens.push((n, n + largeur));
            }
        }
        a_la_main(&points, &liens)
    };
    // 1 000 nœuds (40 × 25), 3 000 degrés de liberté : le plus grand réseau accepté (4 s). Ses 25 rangées de 40 nœuds portent
    // chacune EA/39 et les montants ne portent rien : K = 125/39 (l'écart mesuré est de 2e-12 : mille nœuds, un plus petit mode
    // de 4e-4). 1 001 nœuds (77 × 13) sont refusés avant tout calcul, comme une chaîne de 1 002 nœuds.
    let k = pousse(&cadre(40, 25), (40, 25)).expect("1 000 nœuds, 3 000 degrés de liberté").raideur;
    assert!(egal(k, 125.0 / 39.0, 2e-11), "K = {k}");
    assert_eq!(pousse(&cadre(77, 13), (77, 13)).err(), Some(ErreurReseau::TropGrand));
    let points: Vec<(f64, f64)> = (0..1001).map(|i| (0.5 + i as f64 * 0.001, 0.5)).chain([(1.5, 0.5)]).collect();
    assert_eq!(pousse(&chaine(&points), (2, 1)).err(), Some(ErreurReseau::TropGrand));
    // Le plafond ne compte que si la colonie traverse : la même chaîne dans une boîte plus large ne traverse pas, K = 0.
    assert_eq!(pousse(&chaine(&points), (3, 1)).map(|p| p.raideur), Ok(0.0));
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn ni_fonction_interdite_ni_hashmap_le_noyau_ne_connait_pas_le_reseau_et_etape_15_est_la() {
    let interdites = [
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "exp2", "exp_m1", "ln",
        "ln_1p", "log", "log2", "log10", "powf", "powi", "hypot", "mul_add", "cbrt",
    ];
    for fichier in [
        "src/reseau.rs",
        "examples/colonie_raideur.rs",
        "tests/etape_15/outils.rs",
        "tests/etape_15_cas.rs",
        "tests/etape_15_limites.rs",
        "tests/etape_15_invariants.rs",
        "tests/etape_15_colonie.rs",
    ] {
        // Le code seul : les lignes de commentaire nomment ces fonctions pour les interdire.
        let texte: String =
            lire(fichier).lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for nom in interdites {
            assert!(!texte.contains(&format!(".{nom}(")) && !texte.contains(&format!("::{nom}(")), "{fichier} : {nom}");
        }
        assert!(!texte.contains(concat!("Hash", "Map")) && !texte.contains(concat!("Hash", "Set")), "{fichier}");
        assert!(!texte.contains(concat!("thr", "ead")) && !texte.contains(concat!("Inst", "ant")), "{fichier}");
    }
    // Le noyau, et les étapes 13 et 14, ne connaissent pas le réseau ; le noyau n'a aucun accesseur de plus.
    for fichier in std::fs::read_dir(format!("{}/src", env!("CARGO_MANIFEST_DIR"))).unwrap() {
        let nom = fichier.unwrap().file_name().into_string().unwrap();
        if nom != "reseau.rs" && nom != "lib.rs" {
            assert!(!lire(&format!("src/{nom}")).contains("reseau"), "src/{nom} parle du réseau");
        }
    }
    assert!(
        lire("src/lib.rs").contains("\npub mod reseau;\n") && !lire("src/simulation.rs").contains("etat_des_pointes")
    );
    // Aucune dépendance, d'aucune sorte ; la fonctionnalité est déclarée.
    let cargo = lire("Cargo.toml");
    assert!(cargo.contains("\n[dependencies]\n\n[features]\n") && cargo.matches("dependencies]").count() == 1);
    assert!(cargo.contains("\netape-14 = []\netape-15 = []\n"), "etape-15 se déclare après etape-14");
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn la_section_de_modele_md_dit_la_regle_les_statuts_les_cas_et_les_limites() {
    let modele = lire("MODELE.md");
    let debut = modele.find("\n# La raideur d'une colonie (étape 15, M3)\n").expect("la section de l'étape 15");
    let section = &modele[debut + 1..];
    let section = &section[..section.find("\n# ").unwrap_or(section.len())];
    for mot in [
        "lu à la source",
        "calculé",
        "choix",
        "Brinkmann",
        "aucune raideur de colonie n'est publiée",
        "dépend du choix des mors",
        "ne prédit pas",
        "arbre",
        "anastomose",
        "pont",
        "règle de contact",
        "ne traverse pas",
        "1701/3125",
        "1260/1399",
        "8459601460/5420380133",
        "Euler-Bernoulli",
        "trapu",
        "uniformes",
        "petites colonies",
        "INCONNUES_MAX",
        "COURT",
        "plage normale",
        "colonie_raideur",
        "Ce que les tests ne vérifient pas",
    ] {
        assert!(section.contains(mot), "la section de l'étape 15 ne dit pas « {mot} »");
    }
}
