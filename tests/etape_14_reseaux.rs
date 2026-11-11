//! Étape 14 (M3) : le calcul de raideur des poutres sur des réseaux au hasard : les invariants (équilibre des forces
//! et des moments, énergie, superposition, réciprocité, corps rigide, échelle, ordre), les mécanismes, les raideurs très
//! différentes et les données invalides.
//!
//! La tolérance des invariants vient de la mesure : sur ces réseaux (des grilles de 2 × 2 à 4 × 4 nœuds, `EA` et `EI`
//! de 1 à 2), le plus grand écart est de l'ordre de 1e-13 ; une erreur de formule fait des écarts de 1e-3 à 1.

#[path = "etape_13/outils.rs"]
mod outils;

use mycelium_lab::poutres::{Poutre, SolutionPoutres, matrice_de_raideur, resoudre};
use mycelium_lab::raideur::{Appui, Barre, Charge, ErreurRaideur, Noeud};
use mycelium_lab::systeme::plus_petit_mode;
use outils::{Hasard, appuis, ecart, fois, gauss, grille, partie_libre};

const TOLERANCE: f64 = 2e-11;
/// Le nœud 0 ne bouge pas (il peut tourner), le nœud 1 ne bouge pas en `y` : ce qu'il faut pour tenir une grille.
const TROIS_APPUIS: [(usize, usize); 3] = [(0, 0), (0, 1), (1, 1)];
const BLOQUES: [usize; 3] = [0, 1, 4];

/// Un réseau au hasard avec ses trois appuis et une à trois charges (forces ou moments) ; `EA` et `EI` de 1 à 2, fois
/// `10^k`, `k` de 0 à `decades`.
fn cas(h: &mut Hasard, decades: usize) -> (Vec<Noeud>, Vec<Poutre>, Vec<Appui>, Vec<Charge>) {
    let (largeur, hauteur) = (2 + h.jusqu_a(3), 2 + h.jusqu_a(3));
    let (noeuds, segments) = grille(h, largeur, hauteur);
    let poutres =
        segments.iter().map(|&(a, b)| Poutre { a, b, ea: h.raideur(decades), ei: h.raideur(decades) }).collect();
    let n = noeuds.len();
    let charges =
        (0..1 + h.jusqu_a(3)).map(|_| Charge { noeud: h.jusqu_a(n), axe: h.jusqu_a(3), valeur: h.pm() }).collect();
    (noeuds, poutres, appuis(&TROIS_APPUIS), charges)
}

fn resolu(noeuds: &[Noeud], poutres: &[Poutre], appuis: &[Appui], charges: &[Charge]) -> SolutionPoutres {
    resoudre(noeuds, poutres, appuis, charges).expect("un réseau au hasard avec ses appuis se résout")
}

fn longueur(noeuds: &[Noeud], p: &Poutre) -> f64 {
    let (dx, dy) = (noeuds[p.b].x - noeuds[p.a].x, noeuds[p.b].y - noeuds[p.a].y);
    (dx * dx + dy * dy).sqrt()
}

/// Le plus petit mode de la matrice libre d'un réseau : celle qui reste sans les degrés de liberté des appuis.
fn mode(noeuds: &[Noeud], poutres: &[Poutre], appuis: &[Appui]) -> f64 {
    let bloques: Vec<usize> = appuis.iter().map(|a| 3 * a.noeud + a.axe).collect();
    let (n, k) = (3 * noeuds.len(), matrice_de_raideur(noeuds, poutres).unwrap());
    plus_petit_mode(&partie_libre(&k, n, &bloques), n - bloques.len())
}

/// Une console de `n` poutres alignées, de longueur totale 1 (`EA = EI = 1`), tenue au nœud 0 selon les axes `tenus`
/// (les trois pour un encastrement), chargée de 1 en travers au bout : sa solution, et son plus petit mode.
fn console(n: usize, tenus: &[usize]) -> (Result<SolutionPoutres, ErreurRaideur>, f64) {
    let noeuds: Vec<Noeud> = (0..=n).map(|i| Noeud { x: i as f64 / n as f64, y: 0.0 }).collect();
    let poutres: Vec<Poutre> = (0..n).map(|i| Poutre { a: i, b: i + 1, ea: 1.0, ei: 1.0 }).collect();
    let pied: Vec<Appui> = tenus.iter().map(|&axe| Appui { noeud: 0, axe, valeur: 0.0 }).collect();
    let charge = [Charge { noeud: n, axe: 1, valeur: 1.0 }];
    (resoudre(&noeuds, &poutres, &pied, &charge), mode(&noeuds, &poutres, &pied))
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn la_solution_resout_le_systeme_et_tient_l_equilibre_et_l_energie() {
    let mut h = Hasard(141);
    for essai in 0..100 {
        let (noeuds, poutres, appuis, charges) = cas(&mut h, 0);
        let (n, s) = (3 * noeuds.len(), resolu(&noeuds, &poutres, &appuis, &charges));
        let k = matrice_de_raideur(&noeuds, &poutres).unwrap();
        assert!(
            (0..n * n).all(|q| k[q].to_bits() == k[(q % n) * n + q / n].to_bits()),
            "essai {essai} : symétrie au bit"
        );
        let mut f = vec![0.0; n];
        for c in &charges {
            f[3 * c.noeud + c.axe] += c.valeur;
        }
        // K u = f + R ; les réactions ne sont qu'aux appuis ; la partie libre est celle de l'oracle de Gauss.
        let totales: Vec<f64> = f.iter().zip(&s.reactions).map(|(f, r)| f + r).collect();
        assert!(ecart(&fois(&k, &s.deplacements), &totales) < TOLERANCE, "essai {essai} : K u = f + R");
        let nul = |i: usize| if BLOQUES.contains(&i) { s.deplacements[i] } else { s.reactions[i] } == 0.0;
        assert!(
            (0..n).all(nul),
            "essai {essai} : un appui ne bouge pas, un degré de liberté libre n'a pas de réaction"
        );
        let libres: Vec<usize> = (0..n).filter(|i| !BLOQUES.contains(i)).collect();
        let (u_libre, f_libre): (Vec<f64>, Vec<f64>) = libres.iter().map(|&i| (s.deplacements[i], f[i])).unzip();
        assert!(ecart(&u_libre, &gauss(&partie_libre(&k, n, &BLOQUES), &f_libre)) < TOLERANCE, "essai {essai} : Gauss");
        // L'équilibre du réseau : la somme des forces est nulle, et celle des moments à l'origine (les moments aux
        // nœuds comptés).
        let (mut fx, mut fy, mut moment) = (0.0, 0.0, 0.0);
        for (i, p) in noeuds.iter().enumerate() {
            fx += totales[3 * i];
            fy += totales[3 * i + 1];
            moment += totales[3 * i + 2] + p.x * totales[3 * i + 1] - p.y * totales[3 * i];
        }
        assert!(
            fx.abs().max(fy.abs()).max(moment.abs()) < TOLERANCE,
            "essai {essai} : équilibre {fx:e} {fy:e} {moment:e}"
        );
        // Chaque poutre est en équilibre sous ses six efforts d'extrémité [N_a, V_a, M_a, N_b, V_b, M_b] ; son effort
        // normal est (EA/L) fois son allongement ; son énergie, écrite à part, est N² L/(2 EA) plus
        // L (m_a² + m_a m_b + m_b²)/(6 EI), où m_a = −M_a et m_b = M_b sont les moments de flexion aux deux bouts.
        let (mut travail, mut deformation) = (0.0, 0.0);
        for (i, f) in f.iter().enumerate() {
            travail += 0.5 * f * s.deplacements[i];
        }
        for (p, e) in poutres.iter().zip(&s.efforts) {
            let l = longueur(&noeuds, p);
            assert!(
                (e[0] + e[3]).abs().max((e[1] + e[4]).abs()).max((e[2] + e[5] + e[4] * l).abs()) < TOLERANCE,
                "essai {essai} : {e:?}"
            );
            let (ex, ey) = ((noeuds[p.b].x - noeuds[p.a].x) / l, (noeuds[p.b].y - noeuds[p.a].y) / l);
            let u = |noeud: usize, axe: usize| s.deplacements[3 * noeud + axe];
            let allongement = ex * (u(p.b, 0) - u(p.a, 0)) + ey * (u(p.b, 1) - u(p.a, 1));
            assert!((e[3] - p.ea / l * allongement).abs() < TOLERANCE, "essai {essai} : effort normal {}", e[3]);
            deformation +=
                e[3] * e[3] * l / (2.0 * p.ea) + l * (e[2] * e[2] - e[2] * e[5] + e[5] * e[5]) / (6.0 * p.ei);
        }
        assert!(
            (s.energie - travail).abs() < TOLERANCE * travail.max(1.0),
            "essai {essai} : énergie {} et {travail}",
            s.energie
        );
        assert!(
            (s.energie - deformation).abs() < TOLERANCE * travail.max(1.0),
            "essai {essai} : {} et {deformation}",
            s.energie
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn la_superposition_la_reciprocite_le_corps_rigide_l_echelle_et_l_ordre() {
    let mut h = Hasard(142);
    for essai in 0..100 {
        let (noeuds, poutres, appuis, charges) = cas(&mut h, 0);
        let (n, s) = (noeuds.len(), resolu(&noeuds, &poutres, &appuis, &charges));
        let a_plat = |efforts: &[[f64; 6]]| efforts.concat();
        // Superposition : la solution de deux jeux de charges est la somme des deux solutions.
        let (_, _, _, autres) = cas(&mut h, 0);
        let autres: Vec<Charge> = autres.iter().map(|c| Charge { noeud: c.noeud % n, ..*c }).collect();
        let seule = resolu(&noeuds, &poutres, &appuis, &autres);
        let ensemble = resolu(&noeuds, &poutres, &appuis, &[charges.clone(), autres].concat());
        let somme = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x + y).collect::<Vec<f64>>();
        assert!(
            ecart(&ensemble.deplacements, &somme(&s.deplacements, &seule.deplacements)) < TOLERANCE,
            "essai {essai}"
        );
        assert!(
            ecart(&a_plat(&ensemble.efforts), &somme(&a_plat(&s.efforts), &a_plat(&seule.efforts))) < TOLERANCE,
            "essai {essai}"
        );
        assert!(ecart(&ensemble.reactions, &somme(&s.reactions, &seule.reactions)) < TOLERANCE, "essai {essai}");
        // Réciprocité de Maxwell, entre forces et moments : le déplacement (ou la rotation) en i dû à une charge 1 en j
        // égale celui en j dû à une charge 1 en i.
        let (i, j) = (2 + h.jusqu_a(3 * n - 2), 2 + h.jusqu_a(3 * n - 2));
        let unite =
            |ddl: usize| resolu(&noeuds, &poutres, &appuis, &[Charge { noeud: ddl / 3, axe: ddl % 3, valeur: 1.0 }]);
        assert!(
            (unite(j).deplacements[i] - unite(i).deplacements[j]).abs() < TOLERANCE,
            "essai {essai} : Maxwell {i} {j}"
        );
        // Corps rigide : des appuis déplacés d'une translation et d'une petite rotation ω, sans charge : chaque nœud
        // suit (et tourne de ω), aucun effort.
        let rigide = |p: &Noeud, axe: usize| [0.3 - 0.01 * p.y, -0.7 + 0.01 * p.x, 0.01][axe];
        let pousses: Vec<Appui> =
            appuis.iter().map(|a| Appui { valeur: rigide(&noeuds[a.noeud], a.axe), ..*a }).collect();
        let r = resolu(&noeuds, &poutres, &pousses, &[]);
        let attendus: Vec<f64> = noeuds.iter().flat_map(|p| [0, 1, 2].map(|axe| rigide(p, axe))).collect();
        assert!(ecart(&r.deplacements, &attendus) < TOLERANCE && r.energie.abs() < TOLERANCE, "essai {essai} : rigide");
        assert!(
            ecart(&a_plat(&r.efforts), &vec![0.0; 6 * poutres.len()]) < TOLERANCE,
            "essai {essai} : efforts du corps rigide"
        );
        // Échelle : EA et EI × 2 divisent les déplacements par 2, les efforts restent.
        let raides: Vec<Poutre> = poutres.iter().map(|p| Poutre { ea: 2.0 * p.ea, ei: 2.0 * p.ei, ..*p }).collect();
        let raide = resolu(&noeuds, &raides, &appuis, &charges);
        let moitie: Vec<f64> = s.deplacements.iter().map(|u| 0.5 * u).collect();
        assert!(
            ecart(&raide.deplacements, &moitie) < TOLERANCE
                && ecart(&a_plat(&raide.efforts), &a_plat(&s.efforts)) < TOLERANCE
        );
        // Ordre : les nœuds renumérotés à l'envers, les poutres à rebours et retournées : la même solution. Une poutre
        // retournée voit ses efforts échangés : [−N_b, −V_b, M_b, −N_a, −V_a, M_a].
        let envers = |i: usize| n - 1 - i;
        let noeuds_2: Vec<Noeud> = noeuds.iter().rev().copied().collect();
        let poutres_2: Vec<Poutre> =
            poutres.iter().rev().map(|p| Poutre { a: envers(p.b), b: envers(p.a), ..*p }).collect();
        let appuis_2: Vec<Appui> = appuis.iter().map(|a| Appui { noeud: envers(a.noeud), ..*a }).collect();
        let charges_2: Vec<Charge> = charges.iter().map(|c| Charge { noeud: envers(c.noeud), ..*c }).collect();
        let s2 = resolu(&noeuds_2, &poutres_2, &appuis_2, &charges_2);
        let remis: Vec<f64> = (0..3 * n).map(|d| s2.deplacements[3 * envers(d / 3) + d % 3]).collect();
        let efforts_remis: Vec<f64> =
            s2.efforts.iter().rev().flat_map(|e| [-e[3], -e[4], e[5], -e[0], -e[1], e[2]]).collect();
        assert!(
            ecart(&remis, &s.deplacements) < TOLERANCE && ecart(&efforts_remis, &a_plat(&s.efforts)) < TOLERANCE,
            "essai {essai} : ordre"
        );
        // Déterminisme : deux exécutions égales au bit près.
        assert_eq!(resolu(&noeuds, &poutres, &appuis, &charges), s, "essai {essai}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn un_mecanisme_ne_rend_jamais_un_nombre_mais_un_noeud_rigide_tient_ce_qu_une_articulation_lache() {
    let n = |x: f64, y: f64| Noeud { x, y };
    let p = |a: usize, b: usize| Poutre { a, b, ea: 1.0, ei: 1.0 };
    let carre = [n(0.0, 0.0), n(1.0, 0.0), n(1.0, 1.0), n(0.0, 1.0)];
    let cotes = [p(0, 1), p(1, 2), p(2, 3), p(3, 0)];
    let pousse = [Charge { noeud: 2, axe: 0, valeur: 1.0 }];
    // Le carré sans diagonale : en poutres il tient (ses nœuds sont rigides), en barres il se couche.
    let pieds = appuis(&[(0, 0), (0, 1), (1, 0), (1, 1)]);
    assert!(resoudre(&carre, &cotes, &pieds, &pousse).is_ok());
    let barres: Vec<Barre> = cotes.iter().map(|p| Barre { a: p.a, b: p.b, ea: 1.0 }).collect();
    assert_eq!(mycelium_lab::raideur::resoudre(&carre, &barres, &pieds, &pousse), Err(ErreurRaideur::Mecanisme));
    // Trop peu d'appuis : aucun ; un point fixe (le carré tourne autour) ; un nœud sans poutre ; un nœud tenu seulement
    // en rotation.
    for trop_peu in [vec![], vec![(0, 0), (0, 1)], vec![(0, 0), (0, 1), (0, 2), (4, 0)], vec![(0, 2), (1, 2), (2, 2)]] {
        let noeuds = [carre.to_vec(), vec![n(5.0, 5.0)]].concat();
        let noeuds = if trop_peu.contains(&(4, 0)) { &noeuds[..] } else { &carre[..] };
        for charges in [&pousse[..], &[]] {
            assert_eq!(
                resoudre(noeuds, &cotes, &appuis(&trop_peu), charges).err(),
                Some(ErreurRaideur::Mecanisme),
                "{trop_peu:?}"
            );
        }
    }
    // Une console de n poutres alignées : mal conditionnée (son plus petit mode décroît comme 1/n⁴), pas un mécanisme ;
    // sa flèche est 1/3, à 2,2e-15 / mode près (mesuré : 2e-16 / mode au plus). Sans l'encastrement en rotation (une
    // rotule au pied), ou sans rien qui la tienne en travers, c'est un mécanisme, quelle que soit sa longueur.
    for nombre in [1, 3, 10, 30, 100, 300] {
        let (solution, mode) = console(nombre, &[0, 1, 2]);
        let fleche = solution.unwrap_or_else(|_| panic!("une console de {nombre} poutres est valide")).deplacements
            [3 * nombre + 1];
        assert!((3.0 * fleche - 1.0).abs() <= 2.2e-15 / mode, "{nombre} poutres : flèche {fleche}, mode {mode:e}");
        let puissance_4 = (nombre * nombre * nombre * nombre) as f64;
        assert!(0.1 / puissance_4 < mode && mode < 1.0 / puissance_4, "{nombre} poutres : mode {mode:e}");
        for tenus in [&[0, 1][..], &[0, 2], &[0], &[1, 2]].into_iter().filter(|_| nombre <= 100) {
            let (solution, mode) = console(nombre, tenus);
            assert_eq!(solution.err(), Some(ErreurRaideur::Mecanisme), "{nombre} poutres, tenue selon {tenus:?}");
            assert!(mode <= 1e-14, "{nombre} poutres, tenue selon {tenus:?} : mode {mode:e}");
        }
    }
    // Des réseaux au hasard, de raideurs proches ou très différentes (EA et EI jusqu'à 2e12 fois plus grands d'une
    // poutre à l'autre) : sans un de ses trois appuis, le réseau bouge sans effort. Jamais de nombre. Avec ses trois
    // appuis, il se résout tant que le contraste ne dépasse pas 2e3.
    let mut h = Hasard(143);
    for (essai, decades) in (0..200).map(|essai| (essai, [0, 3, 6, 9, 12][essai % 5])) {
        let (noeuds, poutres, appuis, charges) = cas(&mut h, decades);
        if decades <= 3 {
            assert!(resoudre(&noeuds, &poutres, &appuis, &charges).is_ok(), "essai {essai}, contraste 1e{decades}");
        }
        for retire in 0..3 {
            let moins: Vec<Appui> = (0..3).filter(|&i| i != retire).map(|i| appuis[i]).collect();
            assert_eq!(
                resoudre(&noeuds, &poutres, &moins, &charges).err(),
                Some(ErreurRaideur::Mecanisme),
                "essai {essai}, 1e{decades}"
            );
        }
    }
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn une_poutre_tres_souple_en_flexion_rend_les_barres_et_une_poutre_tres_raide_ne_les_rend_pas() {
    // Le toit 3-4-5 de l'étape 13 (pieds articulés, sommet chargé de 1 vers le bas, EA = 1) : en barres, la flèche est
    // −125/32 et l'effort −5/8. En poutres, le nœud rigide du sommet ajoute la flexion : la flèche et l'effort sont
    // divisés par 1 + 27 EI/(400 EA) (dérivé, en fractions exactes). EI → 0 rend les barres ; EI grand ne les rend
    // pas : le toit ne bouge presque plus.
    let toit = [Noeud { x: -3.0, y: 0.0 }, Noeud { x: 3.0, y: 0.0 }, Noeud { x: 0.0, y: 4.0 }];
    let pieds = appuis(&[(0, 0), (0, 1), (1, 0), (1, 1)]);
    let charge = [Charge { noeud: 2, axe: 1, valeur: -1.0 }];
    for ei in [1e-9, 1e-6, 1e-3, 1.0, 1e3, 1e6, 1e9] {
        let poutres = [Poutre { a: 0, b: 2, ea: 1.0, ei }, Poutre { a: 1, b: 2, ea: 1.0, ei }];
        let s = resoudre(&toit, &poutres, &pieds, &charge).unwrap_or_else(|_| panic!("EI = {ei:e} est valide"));
        let (fleche, effort, diviseur) = (s.deplacements[7], s.efforts[0][3], 1.0 + 27.0 * ei / 400.0);
        let permise = 2.2e-15 / mode(&toit, &poutres, &pieds); // le mode est petit quand EI est grand devant EA
        assert!((fleche * diviseur * 32.0 / -125.0 - 1.0).abs() < permise, "EI = {ei:e} : flèche {fleche:e}");
        assert!((effort * diviseur * 8.0 / -5.0 - 1.0).abs() < permise, "EI = {ei:e} : effort {effort:e}");
        if ei <= 1e-6 {
            assert!((fleche * 32.0 / -125.0 - 1.0).abs() < 1e-7, "EI = {ei:e} : les barres, {fleche}");
        }
        if ei >= 1e6 {
            assert!(fleche.abs() < 1e-3, "EI = {ei:e} : presque immobile, {fleche:e}");
        }
    }
    // Une poutre très raide en étirement ne s'étire pas et fléchit comme avant ; très raide en flexion, elle ne
    // fléchit pas et s'étire comme avant (une console de longueur 2, chargée en long et en travers de 1).
    let console = [Noeud { x: 0.0, y: 0.0 }, Noeud { x: 2.0, y: 0.0 }];
    let (encastre, charges) = (
        appuis(&[(0, 0), (0, 1), (0, 2)]),
        [Charge { noeud: 1, axe: 0, valeur: 1.0 }, Charge { noeud: 1, axe: 1, valeur: 1.0 }],
    );
    for raideur in [1.0, 1e6, 1e12, 1e18] {
        let s = resoudre(&console, &[Poutre { a: 0, b: 1, ea: raideur, ei: 3.0 }], &encastre, &charges).unwrap();
        assert!(
            (s.deplacements[3] * raideur / 2.0 - 1.0).abs() < 1e-14
                && (s.deplacements[4] * 9.0 / 8.0 - 1.0).abs() < 1e-14,
            "EA = {raideur:e}"
        );
        let s = resoudre(&console, &[Poutre { a: 0, b: 1, ea: 5.0, ei: raideur }], &encastre, &charges).unwrap();
        assert!(
            (s.deplacements[3] * 5.0 / 2.0 - 1.0).abs() < 1e-14
                && (s.deplacements[4] * 3.0 * raideur / 8.0 - 1.0).abs() < 1e-14,
            "EI = {raideur:e}"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-14"), ignore)]
fn les_donnees_invalides_sont_refusees_et_les_cas_limites_passent() {
    let noeuds = [Noeud { x: 0.0, y: 0.0 }, Noeud { x: 2.0, y: 0.0 }];
    let poutre = [Poutre { a: 0, b: 1, ea: 5.0, ei: 3.0 }];
    let encastre = appuis(&[(0, 0), (0, 1), (0, 2)]);
    let pousse = [Charge { noeud: 1, axe: 1, valeur: 9.0 }];
    let invalide = |r: Result<SolutionPoutres, ErreurRaideur>| matches!(r, Err(ErreurRaideur::DonneeInvalide(_)));
    for (a, b, ea, ei) in [(0, 2, 1.0, 1.0), (2, 0, 1.0, 1.0), (1, 1, 1.0, 1.0)] {
        assert!(
            invalide(resoudre(&noeuds, &[poutre[0], Poutre { a, b, ea, ei }], &encastre, &pousse)),
            "poutre {a}-{b}"
        );
    }
    for mauvaise in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        // Une poutre sans raideur en flexion serait une barre : c'est l'étape 13.
        assert!(
            invalide(resoudre(&noeuds, &[Poutre { ea: mauvaise, ..poutre[0] }], &encastre, &pousse)),
            "ea = {mauvaise}"
        );
        assert!(
            invalide(resoudre(&noeuds, &[Poutre { ei: mauvaise, ..poutre[0] }], &encastre, &pousse)),
            "ei = {mauvaise}"
        );
        assert!(matrice_de_raideur(&noeuds, &[Poutre { ei: mauvaise, ..poutre[0] }]).is_err(), "ei = {mauvaise}");
    }
    for mauvais in [f64::NAN, f64::INFINITY] {
        assert!(
            invalide(resoudre(&[noeuds[0], Noeud { x: mauvais, y: 0.0 }], &poutre, &encastre, &pousse)),
            "nœud {mauvais}"
        );
        let seul = [noeuds[0], noeuds[1], Noeud { x: 0.0, y: mauvais }];
        assert!(invalide(resoudre(&seul, &poutre, &encastre, &pousse)), "nœud sans poutre {mauvais}");
        assert!(
            invalide(resoudre(&noeuds, &poutre, &encastre, &[Charge { noeud: 1, axe: 2, valeur: mauvais }])),
            "charge {mauvais}"
        );
        assert!(
            invalide(resoudre(&noeuds, &poutre, &[Appui { noeud: 0, axe: 2, valeur: mauvais }], &pousse)),
            "appui {mauvais}"
        );
    }
    for (noeud, axe) in [(2, 0), (0, 3), (0, 2)] {
        // Hors limites (nœud, puis axe : la rotation est l'axe 2, il n'y a pas d'axe 3), puis un appui répété.
        assert!(
            invalide(resoudre(&noeuds, &poutre, &[encastre.clone(), appuis(&[(noeud, axe)])].concat(), &pousse)),
            "appui {noeud} {axe}"
        );
    }
    for (noeud, axe) in [(2, 0), (1, 3)] {
        assert!(
            invalide(resoudre(&noeuds, &poutre, &encastre, &[Charge { noeud, axe, valeur: 1.0 }])),
            "charge {noeud} {axe}"
        );
    }
    // Les cas limites : deux charges sur le même degré de liberté s'ajoutent (flèche 2 P L³/(3 EI) = 16) ; une rotation
    // imposée au bout, sans charge (moment 2 EI θ/L à l'encastrement, 4 EI θ/L au bout, pour un bout tenu en travers) ;
    // aucun nœud.
    let s = resoudre(&noeuds, &poutre, &encastre, &[pousse[0], pousse[0]]).unwrap();
    assert!(
        (s.deplacements[4] - 16.0).abs() < 1e-13
            && (s.reactions[1] + 18.0).abs() < 1e-13
            && (s.reactions[2] + 36.0).abs() < 1e-13,
        "{s:?}"
    );
    let tourne =
        [encastre.clone(), vec![Appui { noeud: 1, axe: 1, valeur: 0.0 }, Appui { noeud: 1, axe: 2, valeur: 0.5 }]]
            .concat();
    let s = resoudre(&noeuds, &poutre, &tourne, &[]).unwrap();
    assert!(
        (s.reactions[2] - 1.5).abs() < 1e-14
            && (s.reactions[5] - 3.0).abs() < 1e-14
            && (s.energie - 0.75).abs() < 1e-14,
        "{s:?}"
    );
    assert!((s.reactions[1] - 2.25).abs() < 1e-14 && (s.reactions[4] + 2.25).abs() < 1e-14, "{s:?}");
    let vide = resoudre(&[], &[], &[], &[]).unwrap();
    assert!(vide.deplacements.is_empty() && vide.efforts.is_empty() && vide.energie == 0.0);
}
