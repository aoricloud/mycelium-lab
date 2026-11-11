//! Étape 14 (M3) : la matrice de la poutre, les cas d'école des poutres (le tableau de l'exemple `cas_poutres`), le nid
//! d'abeille, les règles du dépôt et la section de `MODELE.md`.

#[path = "../examples/cas_poutres.rs"]
mod exemple;

use exemple::barres::{DERIVE, LU, tableau};
use mycelium_lab::poutres::{Poutre, matrice_de_raideur, matrice_locale};
use mycelium_lab::raideur::Noeud;

/// La tolérance des cas d'école : la plus grande erreur mesurée est de 1,1e-13 (le module du nid d'abeille à `t/l` = 0,02), de
/// 3e-15 ailleurs ; une erreur de formule fait 1e-2 à 1.
const TOLERANCE: f64 = 2e-12;

fn lire(chemin: &str) -> String {
    let complet = format!("{}/{chemin}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&complet).unwrap_or_else(|e| panic!("{complet} : {e}"))
}

/// La matrice 6 × 6 (dans le repère du réseau) d'une seule poutre, de l'origine au point `(x, y)`.
fn matrice_d_une_poutre(x: f64, y: f64, ea: f64, ei: f64) -> Vec<f64> {
    matrice_de_raideur(&[Noeud { x: 0.0, y: 0.0 }, Noeud { x, y }], &[Poutre { a: 0, b: 1, ea, ei }]).unwrap()
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn la_matrice_de_la_poutre_est_celle_des_cours() {
    // EA = 5, EI = 3, L = 2 : a = EA/L = 2,5 ; b = EI/L³ = 0,375 ; 12b = 4,5 ; 6bL = 4,5 ; 4bL² = 6 ; 2bL² = 3.
    let attendue = [
        [2.5, 0.0, 0.0, -2.5, 0.0, 0.0],
        [0.0, 4.5, 4.5, 0.0, -4.5, 4.5],
        [0.0, 4.5, 6.0, 0.0, -4.5, 3.0],
        [-2.5, 0.0, 0.0, 2.5, 0.0, 0.0],
        [0.0, -4.5, -4.5, 0.0, 4.5, -4.5],
        [0.0, 4.5, 3.0, 0.0, -4.5, 6.0],
    ];
    assert_eq!(matrice_locale(5.0, 3.0, 2.0), attendue);
    // Une poutre le long de x : sa matrice dans le repère du réseau est la même.
    assert_eq!(matrice_d_une_poutre(2.0, 0.0, 5.0, 3.0), attendue.concat());
    // Une poutre inclinée, de (0, 0) à (3, 4), EA = 5, EI = 3 : Tᵀ k T en fractions exactes (calcul à part).
    let (a, b, c, d, e) = (1701.0 / 3125.0, 1068.0 / 3125.0, 72.0 / 125.0, 2324.0 / 3125.0, 54.0 / 125.0);
    let inclinee = [
        [a, b, -c, -a, -b, -c],
        [b, d, e, -b, -d, e],
        [-c, e, 2.4, c, -e, 1.2],
        [-a, -b, c, a, b, c],
        [-b, -d, -e, b, d, -e],
        [-c, e, 1.2, c, -e, 2.4],
    ];
    let k = matrice_d_une_poutre(3.0, 4.0, 5.0, 3.0);
    for (q, attendu) in inclinee.concat().iter().enumerate() {
        assert!((k[q] - attendu).abs() < 1e-15, "K[{}][{}] = {} et non {attendu}", q / 6, q % 6, k[q]);
    }
    // Un montant vertical, celui de l'exemple 30.1 du cours de Kharagpur (leçon 30) : E = 200 GPa, I = 1,333e-4 m⁴,
    // A = 0,04 m², L = 6 m (en kN et en m). Le cours imprime, à trois chiffres : 1,48e3 ; 1,333e6 ; 4,44e3 (négatif
    // entre la force selon x et la rotation) ; 17,78e3 ; 8,88e3.
    let k = matrice_d_une_poutre(0.0, 6.0, 200e6 * 0.04, 200e6 * 1.333e-4);
    for (i, j, imprime) in
        [(0, 0, 1.48e3), (1, 1, 1.333e6), (0, 2, -4.44e3), (2, 2, 17.78e3), (2, 5, 8.88e3), (3, 5, 4.44e3)]
    {
        assert!(
            (k[i * 6 + j] / imprime - 1.0).abs() < 2e-3,
            "K[{i}][{j}] = {} et le cours imprime {imprime}",
            k[i * 6 + j]
        );
    }
    // Symétrique au bit près ; une translation et une rotation d'ensemble (u = −ω y, v = ω x, θ = ω) ne coûtent rien.
    for (x, y, ea, ei) in [(3.0, 4.0, 5.0, 3.0), (-0.3, 1.7, 2e3, 1e-2), (1.1, -0.2, 0.7, 40.0), (-2.0, -1.5, 1.0, 1.0)]
    {
        let k = matrice_d_une_poutre(x, y, ea, ei);
        assert!((0..36).all(|q| k[q].to_bits() == k[(q % 6) * 6 + q / 6].to_bits()), "symétrie de ({x}, {y})");
        for mouvement in [[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 1.0, -y, x, 1.0]] {
            for ligne in k.chunks(6) {
                let force: f64 = ligne.iter().zip(&mouvement).map(|(p, q)| p * q).sum();
                assert!(force.abs() < 1e-12 * (ea + ei), "({x}, {y}) : force {force:e} pour {mouvement:?}");
            }
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn les_valeurs_exactes_des_cas_d_ecole_sont_retrouvees() {
    let cas = exemple::cas();
    for c in &cas {
        assert!(c.erreur() <= TOLERANCE, "{} : calculé {:e}, exact {:e}", c.nom, c.calcule, c.exact);
        assert!(c.exact.is_finite() && [LU, DERIVE].contains(&c.statut), "{}", c.nom);
    }
    assert_eq!((cas.len(), cas.iter().filter(|c| c.statut == LU).count()), (61, 11));
    // Quelques valeurs redites ici, pour qu'une valeur changée dans l'exemple ne passe pas avec le code changé de même.
    let valeur = |nom: &str| cas.iter().find(|c| c.nom == nom).unwrap_or_else(|| panic!("pas de cas {nom}")).calcule;
    for (nom, attendue) in [
        ("console : flèche P L³/(3 EI)", 6.222222222222222),
        ("console : rotation du bout P L²/(2 EI)", 4.666666666666667),
        ("console verticale : rotation (anti-horaire)", 4.666666666666667),
        ("poutre appuyée : flèche P L³/(48 EI)", -3.111111111111111),
        ("poutre encastrée aux deux bouts : flèche P L³/(192 EI)", -0.777777777777778),
        ("poutre encastrée aux deux bouts : moment d'encastrement P L/8", 3.5),
        ("portique : moment du premier pied", 8.108235410289326),
        ("toit en poutres : flèche du sommet", -3.659250585480094),
        ("nid d'abeille, t/l = 0,1 : module E*/Es", 0.002242136967726703),
    ] {
        assert!((valeur(nom) / attendue - 1.0).abs() < 1e-11, "{nom} : {} et non {attendue}", valeur(nom));
    }
    let tableau = tableau(&cas);
    assert_eq!(tableau.lines().count(), 62);
    assert!(tableau.lines().all(|ligne| ligne.split(" | ").count() == 5), "{tableau}");
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn le_nid_d_abeille_a_le_module_publie_et_l_etirement_des_parois_le_corrige() {
    use exemple::{module_en_flexion, nid};
    for rapport in [0.001, 0.01, 0.02, 0.05, 0.1, 0.2, 0.3] {
        let ((module, poisson), carre) = (nid(rapport, 1.0), rapport * rapport);
        // Avec l'étirement des parois (formule dérivée, exacte pour le réseau infini) : à 1e-9 près.
        assert!(
            (module * (1.0 + 3.0 * carre) / module_en_flexion(rapport) - 1.0).abs() < 1e-9,
            "t/l = {rapport} : {module:e}"
        );
        assert!((poisson * (1.0 + 3.0 * carre) / (1.0 - carre) - 1.0).abs() < 1e-9, "t/l = {rapport} : ν* = {poisson}");
        // La formule publiée (flexion seule, ν* = 1) néglige l'étirement : le calcul s'en écarte de 3 (t/l)² et de
        // 4 (t/l)², pas plus (0,03 % à t/l = 0,01 ; 2,9 % à 0,1), et pas moins.
        let (ecart_du_module, ecart_du_poisson) = (1.0 - module / module_en_flexion(rapport), 1.0 - poisson);
        assert!((ecart_du_module / (3.0 * carre) - 1.0).abs() < 0.22, "t/l = {rapport} : écart {ecart_du_module:e}");
        assert!((ecart_du_poisson / (4.0 * carre) - 1.0).abs() < 0.22, "t/l = {rapport} : écart {ecart_du_poisson:e}");
        // Des parois dix mille fois plus raides en étirement (presque inextensibles), sauf pour les parois les plus minces,
        // trop mal conditionnées : la formule dérivée avec `3 (t/l)²/10⁴` à la place de `3 (t/l)²`, à 1e-7 près (l'erreur du
        // calcul est d'au plus 3e-9, à t/l = 0,02) ; et, comme borne physique, la formule publiée et ν* = 1 à `3 (t/l)²/10⁴`
        // et `4 (t/l)²/10⁴` près (au plus 4e-5).
        if rapport >= 0.02 {
            let ((module, poisson), petit) = (nid(rapport, 1e4), carre / 1e4);
            assert!(
                (module * (1.0 + 3.0 * petit) / module_en_flexion(rapport) - 1.0).abs() < 1e-7,
                "t/l = {rapport}, inextensible : {module:e}"
            );
            assert!(
                (poisson * (1.0 + 3.0 * petit) / (1.0 - petit) - 1.0).abs() < 1e-7,
                "t/l = {rapport}, inextensible : ν* = {poisson}"
            );
            assert!(
                (module / module_en_flexion(rapport) - 1.0).abs() < 3.1e-4 * carre,
                "t/l = {rapport}, inextensible : {module:e}"
            );
            assert!((poisson - 1.0).abs() < 4.1e-4 * carre, "t/l = {rapport}, inextensible : ν* = {poisson}");
        }
        // Le réseau est isotrope : le même module et le même coefficient pour une traction selon y.
        let energie = |exx: f64, eyy: f64| exemple::energie_du_nid(rapport, 1.0, eyy, exx);
        let (selon_y, poisson_y) = exemple::barres::module_et_poisson(energie, 1.5 * 3.0_f64.sqrt());
        let (module, poisson) = nid(rapport, 1.0);
        assert!(
            (selon_y / module - 1.0).abs() < 1e-9 && (poisson_y / poisson - 1.0).abs() < 1e-9,
            "t/l = {rapport} : isotropie"
        );
    }
    // Le nombre imprimé par le cours (MIT 3.054, leçon 4, page 15) : 2,31 (t/l)³ Es.
    assert!((module_en_flexion(1.0) - 2.31).abs() < 0.005);
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn ni_fonction_interdite_ni_hashmap_et_le_noyau_ne_connait_pas_les_poutres() {
    let interdites = ["sin", "cos", "tan", "atan2", "exp", "ln", "log2", "log10", "powf", "powi", "hypot", "mul_add"];
    for fichier in ["src/poutres.rs", "examples/cas_poutres.rs", "tests/etape_14_cas.rs", "tests/etape_14_reseaux.rs"] {
        // Le code seul : les lignes de commentaire nomment ces fonctions pour les interdire.
        let texte: String =
            lire(fichier).lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
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
        assert!(!lire(&format!("src/{noyau}.rs")).contains("::poutres"), "src/{noyau}.rs");
    }
    assert!(lire("src/lib.rs").contains("pub mod poutres;"));
    assert!(lire("Cargo.toml").contains("[dependencies]\n\n[features]"), "aucune dépendance");
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn la_section_de_modele_md_dit_les_cas_leur_statut_le_nid_d_abeille_et_les_limites() {
    let modele = lire("MODELE.md");
    let debut = modele.find("\n# Les poutres (étape 14, M3)\n").expect("la section de l'étape 14");
    let section = &modele[debut + 1..];
    let section = &section[..section.find("\n# ").unwrap_or(section.len())];
    for mot in [
        "lu à la source",
        "dérivé",
        "Euler-Bernoulli",
        "anti-horaire",
        "Kharagpur",
        "MIT 13.122",
        "MIT 3.054",
        "Roylance",
        "P L³/(3 EI)",
        "P L³/(48 EI)",
        "P L³/(192 EI)",
        "P L/8",
        "(4/√3)",
        "1 + 3 (t/l)²",
        "ν* = 1",
        "27 EI/(400 EA)",
        "cisaillement",
        "réseau infini",
        "ne prédit pas",
        "cas_poutres",
        "Ce que les tests ne vérifient pas",
    ] {
        assert!(section.contains(mot), "la section de l'étape 14 ne dit pas « {mot} »");
    }
}
