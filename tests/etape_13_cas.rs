//! Étape 13 (M3) : les cas d'école des barres (le tableau de l'exemple `cas_barres`), le treillis triangulaire, les
//! règles du dépôt et la section de `MODELE.md`.

#[path = "../examples/cas_barres.rs"]
mod exemple;

use mycelium_lab::raideur::{Appui, Barre, Noeud, resoudre};

/// La tolérance des cas d'école : la plus grande erreur mesurée est de 1,5e-15 ; une erreur de formule fait 1e-2 à 1.
const TOLERANCE: f64 = 1e-12;

fn lire(chemin: &str) -> String {
    let complet = format!("{}/{chemin}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&complet).unwrap_or_else(|e| panic!("{complet} : {e}"))
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn les_valeurs_exactes_des_cas_d_ecole_sont_retrouvees() {
    let cas = exemple::cas();
    for c in &cas {
        assert!(c.erreur() <= TOLERANCE, "{} : calculé {:e}, exact {:e}", c.nom, c.calcule, c.exact);
        assert!(c.exact.is_finite() && [exemple::LU, exemple::DERIVE].contains(&c.statut), "{}", c.nom);
    }
    // Le compte des cas : aucun n'a disparu ; les six lus à la source sont la barre seule et le treillis.
    assert_eq!((cas.len(), cas.iter().filter(|c| c.statut == exemple::LU).count()), (45, 6));
    assert!(
        cas.iter()
            .all(|c| (c.statut == exemple::LU) == (c.nom.starts_with("une barre") || c.nom.starts_with("treillis")))
    );
    // Quelques valeurs redites ici, pour qu'une valeur changée dans l'exemple ne passe pas avec le code changé de même.
    let valeur = |nom: &str| cas.iter().find(|c| c.nom == nom).unwrap_or_else(|| panic!("pas de cas {nom}")).calcule;
    assert!((valeur("deux barres : déplacement vertical du sommet") + 3.90625).abs() < 1e-12);
    assert!((valeur("deux barres : effort de la première") + 0.625).abs() < 1e-12);
    assert!((valeur("triangle : déplacement vertical du sommet") + 0.144337567297406).abs() < 1e-12);
    assert!((valeur("carré à deux diagonales : effort de la diagonale 0-2") - 0.625425056993489).abs() < 1e-12);
    assert!((valeur("treillis triangulaire : module E* l/EA") - 1.154700538379252).abs() < 1e-12);
    // Le nombre imprimé par le cours (MIT 3.054, leçon 4, page 25) : E* = 1,15 Es (t/l).
    assert!((valeur("treillis triangulaire : module E* l/EA") - 1.15).abs() < 0.005);
    // L'erreur d'un cas est relative (absolue quand la valeur exacte est zéro).
    let erreur = |calcule: f64, exact: f64| exemple::Cas { nom: "", calcule, exact, statut: "" }.erreur();
    assert_eq!((erreur(2.5e-7, 2e-7), erreur(-30.0, -20.0), erreur(0.5, 0.0), erreur(7.0, 7.0)), (0.25, 0.5, 0.5, 0.0));
    // Le tableau : un titre et une ligne par cas, chacune avec ses cinq colonnes.
    let tableau = exemple::tableau(&cas);
    assert_eq!(tableau.lines().count(), 46);
    assert!(tableau.lines().all(|ligne| ligne.split(" | ").count() == 5), "{tableau}");
    // Une ligne, ses colonnes (cas | calculé | exact | …) : le calculé se compare à 1e-12 près, le format se fige sur l'exact.
    let ligne =
        tableau.lines().find(|l| l.starts_with("deux barres : effort de la première | ")).expect("la ligne du cas");
    let colonnes: Vec<&str> = ligne.split(" | ").collect();
    assert_eq!(colonnes[2], "-6.250000000000000e-1", "{tableau}");
    assert!((colonnes[1].parse::<f64>().expect("un nombre") + 0.625).abs() < TOLERANCE, "{tableau}");
}

/// Un morceau de treillis triangulaire : les nœuds `(i, j)`, `0 ≤ i, j ≤ cote`, en `(i + j/2, j √3/2)`, et les barres
/// de longueur 1 (`EA = 1`) vers `(i + 1, j)`, `(i, j + 1)` et `(i − 1, j + 1)`.
fn morceau(cote: usize) -> (Vec<Noeud>, Vec<Barre>) {
    let (mut noeuds, mut barres) = (Vec::new(), Vec::new());
    for j in 0..=cote {
        for i in 0..=cote {
            noeuds.push(Noeud { x: i as f64 + 0.5 * j as f64, y: j as f64 * 0.75_f64.sqrt() });
            let a = j * (cote + 1) + i;
            let voisins = [
                (i < cote).then_some(a + 1),
                (j < cote).then_some(a + cote + 1),
                (j < cote && i > 0).then_some(a + cote),
            ];
            barres.extend(voisins.into_iter().flatten().map(|b| Barre { a, b, ea: 1.0 }));
        }
    }
    (noeuds, barres)
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn un_morceau_de_treillis_dont_le_bord_suit_un_champ_affine_le_suit_partout() {
    // Dans un treillis triangulaire, chaque nœud est un centre de symétrie : si les nœuds du bord sont déplacés de
    // `u = G x` (`G` : une matrice 2 × 2 quelconque), les nœuds de l'intérieur le sont aussi, et chaque barre de
    // direction `e` porte `EA eᵀ G e`. De 2 à 722 inconnues : le solveur tient sa précision quand la taille grandit.
    let g = [[0.3, -0.2], [0.5, 0.1]];
    let champ = |p: &Noeud| [g[0][0] * p.x + g[0][1] * p.y, g[1][0] * p.x + g[1][1] * p.y];
    for cote in [2, 3, 5, 9, 14, 20] {
        let (noeuds, barres) = morceau(cote);
        assert_eq!((noeuds.len(), barres.len()), ((cote + 1) * (cote + 1), cote * (3 * cote + 2)));
        let mut appuis = Vec::new();
        for (noeud, p) in noeuds.iter().enumerate() {
            if noeud % (cote + 1) == 0 || noeud % (cote + 1) == cote || noeud <= cote || noeud >= cote * (cote + 1) {
                appuis.extend([0, 1].map(|axe| Appui { noeud, axe, valeur: champ(p)[axe] }));
            }
        }
        assert_eq!(appuis.len(), 8 * cote);
        let s = resoudre(&noeuds, &barres, &appuis, &[]).expect("le morceau se résout");
        let mut pire: f64 = 0.0;
        for (noeud, p) in noeuds.iter().enumerate() {
            pire = pire
                .max((s.deplacements[2 * noeud] - champ(p)[0]).abs())
                .max((s.deplacements[2 * noeud + 1] - champ(p)[1]).abs());
        }
        for (b, effort) in barres.iter().zip(&s.efforts) {
            let (ex, ey) = (noeuds[b.b].x - noeuds[b.a].x, noeuds[b.b].y - noeuds[b.a].y);
            pire = pire.max((effort - (ex * (g[0][0] * ex + g[0][1] * ey) + ey * (g[1][0] * ex + g[1][1] * ey))).abs());
        }
        assert!(pire < 1e-11, "côté {cote} : écart {pire:e}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn ni_fonction_interdite_ni_hashmap_et_le_noyau_ne_connait_pas_la_raideur() {
    let interdites = ["sin", "cos", "tan", "atan2", "exp", "ln", "log2", "log10", "powf", "powi", "hypot", "mul_add"];
    let fichiers = ["src/systeme.rs", "src/raideur.rs", "examples/cas_barres.rs", "tests/etape_13/outils.rs"];
    let tests = ["tests/etape_13_solveur.rs", "tests/etape_13_reseaux.rs", "tests/etape_13_cas.rs"];
    for fichier in fichiers.iter().chain(&tests) {
        // Le code seul : les lignes de commentaire nomment ces fonctions pour les interdire.
        let texte: String =
            lire(fichier).lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join(
                "
",
            );
        for nom in interdites {
            assert!(!texte.contains(&format!(".{nom}(")) && !texte.contains(&format!("::{nom}(")), "{fichier} : {nom}");
        }
        assert!(!texte.contains(concat!("Hash", "Map")) && !texte.contains(concat!("Hash", "Set")), "{fichier}");
        assert!(!texte.contains(concat!("thr", "ead")) && !texte.contains(concat!("Inst", "ant")), "{fichier}");
    }
    for noyau in [
        "croissance",
        "simulation",
        "substrat",
        "voisinage",
        "anastomose",
        "tropisme",
        "piece",
        "moule",
        "calage",
        "main",
    ] {
        let texte = lire(&format!("src/{noyau}.rs"));
        assert!(!texte.contains("::raideur") && !texte.contains("::systeme"), "src/{noyau}.rs");
    }
    let lib = lire("src/lib.rs");
    assert!(lib.contains("pub mod raideur;") && lib.contains("pub mod systeme;"));
    assert!(lire("Cargo.toml").contains("[dependencies]\n\n[features]"), "aucune dépendance");
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn la_section_de_modele_md_dit_les_cas_leur_statut_le_seuil_et_les_limites() {
    let modele = lire("MODELE.md");
    let debut =
        modele.find("\n# Le calcul de raideur : les barres (étape 13, M3)\n").expect("la section de l'étape 13");
    let section = &modele[debut + 1..];
    let section = &section[..section.find("\n# ").unwrap_or(section.len())];
    for mot in [
        "lu à la source",
        "dérivé",
        "Felippa",
        "MIT 3.054",
        "Omidi",
        "−125/32",
        "−5/8",
        "−√3/12",
        "(2/√3)",
        "ν* = 1/3",
        "`SEUIL_SINGULIER`",
        "1e-12",
        "2,5e-16",
        "plus petit mode",
        "plus petit pivot",
        "matrice pleine",
        "petits déplacements",
        "réseau infini",
        "ne prédit pas",
        "cas_barres",
        "Ce que les tests ne vérifient pas",
    ] {
        assert!(section.contains(mot), "la section de l'étape 13 ne dit pas « {mot} »");
    }
}
