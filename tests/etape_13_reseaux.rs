//! Étape 13 (M3) : le calcul de raideur des barres sur des réseaux au hasard : la matrice, les invariants (équilibre,
//! énergie, superposition, réciprocité, corps rigide, échelle, ordre), les mécanismes et les données invalides.
//!
//! La tolérance des invariants vient de la mesure : sur ces réseaux (des grilles de 2 × 2 à 5 × 5 nœuds, `EA` de 1 à 2),
//! le plus grand écart est de l'ordre de 1e-13 ; une erreur de formule fait des écarts de 1e-3 à 1.

#[path = "etape_13/outils.rs"]
mod outils;

use mycelium_lab::raideur::{Appui, Barre, Charge, ErreurRaideur, Noeud, Solution, matrice_de_raideur, resoudre};
use outils::{Hasard, appuis, ecart, fois, gauss, grille, partie_libre};

const TOLERANCE: f64 = 2e-11;
/// Le nœud 0 ne bouge pas, le nœud 1 ne bouge pas en `y` : ce qu'il faut pour tenir une grille.
const TROIS_APPUIS: [(usize, usize); 3] = [(0, 0), (0, 1), (1, 1)];

/// Un réseau au hasard avec ses trois appuis et une à trois charges ; `EA` de 1 à 2, fois `10^k`, `k` de 0 à `decades`.
fn cas(h: &mut Hasard, decades: usize) -> (Vec<Noeud>, Vec<Barre>, Vec<Appui>, Vec<Charge>) {
    let (largeur, hauteur) = (2 + h.jusqu_a(4), 2 + h.jusqu_a(4));
    let (noeuds, segments) = grille(h, largeur, hauteur);
    let barres = segments.iter().map(|&(a, b)| Barre { a, b, ea: h.raideur(decades) }).collect();
    let n = noeuds.len();
    let charges =
        (0..1 + h.jusqu_a(3)).map(|_| Charge { noeud: h.jusqu_a(n), axe: h.jusqu_a(2), valeur: h.pm() }).collect();
    (noeuds, barres, appuis(&TROIS_APPUIS), charges)
}

fn resolu(noeuds: &[Noeud], barres: &[Barre], appuis: &[Appui], charges: &[Charge]) -> Solution {
    resoudre(noeuds, barres, appuis, charges).expect("un réseau au hasard avec ses appuis se résout")
}

/// La longueur d'une barre et sa direction, de `a` vers `b`.
fn geometrie(noeuds: &[Noeud], b: &Barre) -> (f64, f64, f64) {
    let (dx, dy) = (noeuds[b.b].x - noeuds[b.a].x, noeuds[b.b].y - noeuds[b.a].y);
    let longueur = (dx * dx + dy * dy).sqrt();
    (longueur, dx / longueur, dy / longueur)
}

/// Les charges rangées comme les déplacements.
fn forces(charges: &[Charge], n: usize) -> Vec<f64> {
    let mut f = vec![0.0; n];
    for c in charges {
        f[2 * c.noeud + c.axe] += c.valeur;
    }
    f
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn la_matrice_est_celle_des_barres_symetrique_et_sans_energie_de_corps_rigide() {
    // Une barre de (0, 0) à (3, 4), EA = 10 : EA/L = 2, direction (0,6 ; 0,8).
    let k =
        matrice_de_raideur(&[Noeud { x: 0.0, y: 0.0 }, Noeud { x: 3.0, y: 4.0 }], &[Barre { a: 0, b: 1, ea: 10.0 }])
            .unwrap();
    let bloc = [0.72, 0.96, 0.96, 1.28];
    for (i, j) in (0..4).flat_map(|i| (0..4).map(move |j| (i, j))) {
        let signe = if (i < 2) == (j < 2) { 1.0 } else { -1.0 };
        assert!((k[i * 4 + j] - signe * bloc[(i % 2) * 2 + j % 2]).abs() < 1e-15, "K[{i}][{j}] = {}", k[i * 4 + j]);
    }
    let mut h = Hasard(131);
    for essai in 0..100 {
        let (noeuds, barres, _, _) = cas(&mut h, 0);
        let (n, k) = (2 * noeuds.len(), matrice_de_raideur(&noeuds, &barres).unwrap());
        assert!(
            (0..n * n).all(|q| k[q].to_bits() == k[(q % n) * n + q / n].to_bits()),
            "essai {essai} : symétrie au bit"
        );
        // Assemblée à part : pour chaque barre, (EA/L) g gᵀ avec g = (−c, −s, c, s).
        let mut attendue = vec![0.0; n * n];
        for b in &barres {
            let (longueur, c, s) = geometrie(&noeuds, b);
            let (g, ddl) = ([-c, -s, c, s], [2 * b.a, 2 * b.a + 1, 2 * b.b, 2 * b.b + 1]);
            for (i, j) in (0..4).flat_map(|i| (0..4).map(move |j| (i, j))) {
                attendue[ddl[i] * n + ddl[j]] += b.ea / longueur * g[i] * g[j];
            }
        }
        assert!(ecart(&k, &attendue) < 1e-13, "essai {essai} : assemblage");
        // Une translation et une petite rotation d'ensemble ne coûtent aucune force.
        let translation: Vec<f64> = (0..n).map(|i| [0.3, -0.7][i % 2]).collect();
        let rotation: Vec<f64> = noeuds.iter().flat_map(|p| [-p.y, p.x]).collect();
        let zero = vec![0.0; n];
        assert!(
            ecart(&fois(&k, &translation), &zero) < 1e-13 && ecart(&fois(&k, &rotation), &zero) < 1e-13,
            "essai {essai}"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn la_solution_resout_le_systeme_et_tient_l_equilibre_et_l_energie() {
    let mut h = Hasard(132);
    for essai in 0..100 {
        let (noeuds, barres, appuis, charges) = cas(&mut h, 0);
        let (n, s) = (2 * noeuds.len(), resolu(&noeuds, &barres, &appuis, &charges));
        let (k, f) = (matrice_de_raideur(&noeuds, &barres).unwrap(), forces(&charges, n));
        // K u = f + R ; les réactions ne sont qu'aux appuis ; la partie libre est celle de l'oracle de Gauss.
        let totales: Vec<f64> = f.iter().zip(&s.reactions).map(|(f, r)| f + r).collect();
        assert!(ecart(&fois(&k, &s.deplacements), &totales) < TOLERANCE, "essai {essai} : K u = f + R");
        assert!((0..n).all(|i| [0, 1, 3].contains(&i) || s.reactions[i] == 0.0), "essai {essai} : réactions");
        assert!([0, 1, 3].iter().all(|&i| s.deplacements[i] == 0.0), "essai {essai} : appuis");
        let libres: Vec<usize> = (0..n).filter(|i| ![0, 1, 3].contains(i)).collect();
        let (u_libre, f_libre): (Vec<f64>, Vec<f64>) = libres.iter().map(|&i| (s.deplacements[i], f[i])).unzip();
        assert!(
            ecart(&u_libre, &gauss(&partie_libre(&k, n, &[0, 1, 3]), &f_libre)) < TOLERANCE,
            "essai {essai} : Gauss"
        );
        // L'équilibre : la somme des forces (charges et réactions) est nulle, et celle de leurs moments à l'origine.
        let (mut fx, mut fy, mut moment) = (0.0, 0.0, 0.0);
        for (i, p) in noeuds.iter().enumerate() {
            fx += totales[2 * i];
            fy += totales[2 * i + 1];
            moment += p.x * totales[2 * i + 1] - p.y * totales[2 * i];
        }
        assert!(
            fx.abs().max(fy.abs()).max(moment.abs()) < TOLERANCE,
            "essai {essai} : équilibre {fx:e} {fy:e} {moment:e}"
        );
        // L'effort de chaque barre, recalculé : (EA/L) e·(u_b − u_a) ; l'énergie : ½ f·u = Σ ½ N² L/EA.
        let (mut travail, mut deformation) = (0.0, 0.0);
        for (i, f) in f.iter().enumerate() {
            travail += 0.5 * f * s.deplacements[i];
        }
        for (b, effort) in barres.iter().zip(&s.efforts) {
            let (longueur, c, si) = geometrie(&noeuds, b);
            let u = |noeud: usize, axe: usize| s.deplacements[2 * noeud + axe];
            let attendu = b.ea / longueur * (c * (u(b.b, 0) - u(b.a, 0)) + si * (u(b.b, 1) - u(b.a, 1)));
            assert!((effort - attendu).abs() < TOLERANCE, "essai {essai} : effort {effort} et {attendu}");
            deformation += 0.5 * effort * effort * longueur / b.ea;
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
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn la_superposition_la_reciprocite_le_corps_rigide_l_echelle_et_l_ordre() {
    let mut h = Hasard(133);
    for essai in 0..100 {
        let (noeuds, barres, appuis, charges) = cas(&mut h, 0);
        let (n, s) = (noeuds.len(), resolu(&noeuds, &barres, &appuis, &charges));
        // Superposition : la solution de deux jeux de charges est la somme des deux solutions.
        let (_, _, _, autres) = cas(&mut h, 0);
        let autres: Vec<Charge> = autres.iter().map(|c| Charge { noeud: c.noeud % n, ..*c }).collect();
        let (seule, ensemble) =
            (resolu(&noeuds, &barres, &appuis, &autres), [charges.clone(), autres.clone()].concat());
        let ensemble = resolu(&noeuds, &barres, &appuis, &ensemble);
        let somme = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x + y).collect::<Vec<f64>>();
        assert!(
            ecart(&ensemble.deplacements, &somme(&s.deplacements, &seule.deplacements)) < TOLERANCE,
            "essai {essai}"
        );
        assert!(ecart(&ensemble.efforts, &somme(&s.efforts, &seule.efforts)) < TOLERANCE, "essai {essai}");
        assert!(ecart(&ensemble.reactions, &somme(&s.reactions, &seule.reactions)) < TOLERANCE, "essai {essai}");
        // Réciprocité de Maxwell : le déplacement en i dû à une force 1 en j égale celui en j dû à une force 1 en i.
        let (i, j) = (2 + h.jusqu_a(2 * n - 2), 2 + h.jusqu_a(2 * n - 2));
        let unite =
            |ddl: usize| resolu(&noeuds, &barres, &appuis, &[Charge { noeud: ddl / 2, axe: ddl % 2, valeur: 1.0 }]);
        assert!(
            (unite(j).deplacements[i] - unite(i).deplacements[j]).abs() < TOLERANCE,
            "essai {essai} : Maxwell {i} {j}"
        );
        // Corps rigide : des appuis déplacés d'une translation et d'une petite rotation, sans charge : aucun effort.
        let rigide = |p: &Noeud, axe: usize| if axe == 0 { 0.3 - 0.01 * p.y } else { -0.7 + 0.01 * p.x };
        let pousses: Vec<Appui> =
            appuis.iter().map(|a| Appui { valeur: rigide(&noeuds[a.noeud], a.axe), ..*a }).collect();
        let r = resolu(&noeuds, &barres, &pousses, &[]);
        let attendus: Vec<f64> = noeuds.iter().flat_map(|p| [rigide(p, 0), rigide(p, 1)]).collect();
        assert!(ecart(&r.deplacements, &attendus) < TOLERANCE && r.energie.abs() < TOLERANCE, "essai {essai} : rigide");
        assert!(ecart(&r.efforts, &vec![0.0; barres.len()]) < TOLERANCE, "essai {essai} : efforts du corps rigide");
        // Échelle : EA × 2 divise les déplacements par 2 ; des longueurs × 2 les multiplient par 2 ; les efforts restent.
        let raides: Vec<Barre> = barres.iter().map(|b| Barre { ea: 2.0 * b.ea, ..*b }).collect();
        let grands: Vec<Noeud> = noeuds.iter().map(|p| Noeud { x: 2.0 * p.x, y: 2.0 * p.y }).collect();
        for (autre, facteur) in
            [(resolu(&noeuds, &raides, &appuis, &charges), 0.5), (resolu(&grands, &barres, &appuis, &charges), 2.0)]
        {
            let attendus: Vec<f64> = s.deplacements.iter().map(|u| facteur * u).collect();
            assert!(
                ecart(&autre.deplacements, &attendus) < TOLERANCE && ecart(&autre.efforts, &s.efforts) < TOLERANCE,
                "essai {essai}"
            );
            let permise = TOLERANCE * s.energie.max(1.0);
            assert!((autre.energie - facteur * s.energie).abs() < permise, "essai {essai} : énergie à l'échelle");
        }
        // Ordre : les nœuds renumérotés à l'envers, les barres à rebours et retournées : la même solution.
        let envers = |i: usize| n - 1 - i;
        let noeuds_2: Vec<Noeud> = noeuds.iter().rev().copied().collect();
        let barres_2: Vec<Barre> =
            barres.iter().rev().map(|b| Barre { a: envers(b.b), b: envers(b.a), ea: b.ea }).collect();
        let appuis_2: Vec<Appui> = appuis.iter().map(|a| Appui { noeud: envers(a.noeud), ..*a }).collect();
        let charges_2: Vec<Charge> = charges.iter().map(|c| Charge { noeud: envers(c.noeud), ..*c }).collect();
        let s2 = resolu(&noeuds_2, &barres_2, &appuis_2, &charges_2);
        let remis: Vec<f64> = (0..2 * n).map(|d| s2.deplacements[2 * envers(d / 2) + d % 2]).collect();
        let efforts_remis: Vec<f64> = s2.efforts.iter().rev().copied().collect();
        assert!(
            ecart(&remis, &s.deplacements) < TOLERANCE && ecart(&efforts_remis, &s.efforts) < TOLERANCE,
            "essai {essai} : ordre"
        );
        // Déterminisme : deux exécutions égales au bit près.
        assert_eq!(resolu(&noeuds, &barres, &appuis, &charges), s, "essai {essai}");
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn un_mecanisme_ne_rend_jamais_un_nombre_quel_que_soit_le_contraste_des_raideurs() {
    let n = |x: f64, y: f64| Noeud { x, y };
    let b = |a: usize, b: usize, ea: f64| Barre { a, b, ea };
    let carre = [n(0.0, 0.0), n(1.0, 0.0), n(1.0, 1.0), n(0.0, 1.0)];
    let cotes = |ea: f64| vec![b(0, 1, 1.0), b(1, 2, ea), b(2, 3, 1.0), b(3, 0, ea)];
    let pieds = appuis(&[(0, 0), (0, 1), (1, 0), (1, 1)]);
    let charge = [Charge { noeud: 2, axe: 0, valeur: 1.0 }];
    for ea in [1.0, 1e6, 1e9, 1e12] {
        // Le carré sans diagonale se couche, avec ou sans charge, que ses côtés aient ou non la même raideur ; une
        // diagonale le tient, même un milliard de fois plus raide ou plus molle que les côtés.
        assert_eq!(resoudre(&carre, &cotes(ea), &pieds, &charge), Err(ErreurRaideur::Mecanisme), "côtés {ea:e}");
        assert_eq!(
            resoudre(&carre, &cotes(ea), &pieds, &[]),
            Err(ErreurRaideur::Mecanisme),
            "côtés {ea:e}, sans charge"
        );
        for diagonale in [ea, 1.0 / ea].into_iter().filter(|_| ea <= 1e9) {
            let contrevente = [cotes(1.0), vec![b(0, 2, diagonale)]].concat();
            let s =
                resoudre(&carre, &contrevente, &pieds, &charge).unwrap_or_else(|_| panic!("diagonale {diagonale:e}"));
            // La statique : la diagonale porte √2, le côté 1-2 porte −1 (les pieds sont fixes, le côté 3-0 ne porte rien).
            assert!(
                (s.efforts[4] - 2.0_f64.sqrt()).abs() < 1e-6 && (s.efforts[1] + 1.0).abs() < 1e-6,
                "{diagonale:e} : {:?}",
                s.efforts
            );
        }
    }
    // Une barre dont le bout libre n'est pas tenu en travers ; la même, tenue.
    let barre = [n(0.0, 0.0), n(2.0, 0.0)];
    let tire = [Charge { noeud: 1, axe: 0, valeur: 1.0 }];
    assert_eq!(resoudre(&barre, &[b(0, 1, 1.0)], &appuis(&[(0, 0), (0, 1)]), &tire), Err(ErreurRaideur::Mecanisme));
    assert!(resoudre(&barre, &[b(0, 1, 1.0)], &appuis(&[(0, 0), (0, 1), (1, 1)]), &tire).is_ok());
    // Deux barres alignées : le nœud du milieu n'est pas tenu en travers. Un nœud sans barre. Aucun appui.
    let ligne = [n(0.0, 0.0), n(1.0, 0.0), n(2.0, 0.0)];
    let bouts = appuis(&[(0, 0), (0, 1), (2, 0), (2, 1)]);
    assert_eq!(resoudre(&ligne, &[b(0, 1, 1.0), b(1, 2, 1.0)], &bouts, &[]), Err(ErreurRaideur::Mecanisme));
    assert_eq!(resoudre(&ligne, &[b(0, 2, 1.0)], &bouts, &[]), Err(ErreurRaideur::Mecanisme));
    assert_eq!(resoudre(&carre, &[cotes(1.0), vec![b(0, 2, 1.0)]].concat(), &[], &[]), Err(ErreurRaideur::Mecanisme));
    // Des réseaux au hasard, de raideurs toutes proches (contraste 2) ou très différentes (jusqu'à 2e12) : sans un de
    // ses trois appuis, le réseau tourne ; avec un nœud de plus pendu à une seule barre, ce nœud balance. Jamais de
    // nombre. Avec ses trois appuis, le réseau se résout tant que le contraste ne dépasse pas 2e6.
    let mut h = Hasard(134);
    for (essai, decades) in (0..240).map(|essai| (essai, [0, 3, 6, 9, 12, 15][essai % 6])) {
        let (mut noeuds, mut barres, appuis, charges) = cas(&mut h, decades);
        if decades <= 6 {
            assert!(resoudre(&noeuds, &barres, &appuis, &charges).is_ok(), "essai {essai}, contraste 1e{decades}");
        }
        for retire in 0..3 {
            let moins: Vec<Appui> = (0..3).filter(|&i| i != retire).map(|i| appuis[i]).collect();
            assert_eq!(
                resoudre(&noeuds, &barres, &moins, &charges),
                Err(ErreurRaideur::Mecanisme),
                "essai {essai}, 1e{decades}"
            );
        }
        barres.push(b(h.jusqu_a(noeuds.len()), noeuds.len(), h.raideur(decades)));
        noeuds.push(n(-1.0 + 0.3 * h.pm(), -1.0 + 0.3 * h.pm()));
        assert_eq!(
            resoudre(&noeuds, &barres, &appuis, &charges),
            Err(ErreurRaideur::Mecanisme),
            "essai {essai} : nœud pendu"
        );
    }
}

#[test]
#[cfg_attr(not(feature = "etape-13"), ignore)]
fn les_donnees_invalides_sont_refusees_et_les_cas_limites_passent() {
    let noeuds = [Noeud { x: 0.0, y: 0.0 }, Noeud { x: 2.0, y: 0.0 }];
    let barre = [Barre { a: 0, b: 1, ea: 3.0 }];
    let tenu = appuis(&[(0, 0), (0, 1), (1, 1)]);
    let tire = [Charge { noeud: 1, axe: 0, valeur: 6.0 }];
    let invalide = |r: Result<Solution, ErreurRaideur>| matches!(r, Err(ErreurRaideur::DonneeInvalide(_)));
    for mauvaise in [
        Barre { a: 0, b: 2, ea: 1.0 },
        Barre { a: 2, b: 0, ea: 1.0 },
        Barre { a: 1, b: 1, ea: 1.0 }, // de longueur nulle
        Barre { a: 0, b: 1, ea: 0.0 },
        Barre { a: 0, b: 1, ea: -1.0 },
        Barre { a: 0, b: 1, ea: f64::NAN },
        Barre { a: 0, b: 1, ea: f64::INFINITY },
    ] {
        assert!(invalide(resoudre(&noeuds, &[barre[0], mauvaise], &tenu, &tire)), "{mauvaise:?}");
        assert!(matrice_de_raideur(&noeuds, &[mauvaise]).is_err(), "{mauvaise:?}");
    }
    for mauvais in [f64::NAN, f64::INFINITY] {
        assert!(invalide(resoudre(&[noeuds[0], Noeud { x: 2.0, y: mauvais }], &barre, &tenu, &tire)), "nœud {mauvais}");
        assert!(
            invalide(resoudre(&[noeuds[0], noeuds[1], Noeud { x: mauvais, y: 0.0 }], &barre, &tenu, &tire)),
            "nœud seul"
        );
        assert!(invalide(resoudre(
            &noeuds,
            &barre,
            &[tenu.clone(), vec![Appui { noeud: 1, axe: 0, valeur: mauvais }]].concat(),
            &tire
        )));
        assert!(
            invalide(resoudre(&noeuds, &barre, &tenu, &[Charge { noeud: 1, axe: 0, valeur: mauvais }])),
            "charge {mauvais}"
        );
    }
    for (noeud, axe) in [(2, 0), (0, 2), (1, 1)] {
        // Hors limites (nœud, puis axe), puis un degré de liberté imposé deux fois, même à la même valeur.
        assert!(
            invalide(resoudre(&noeuds, &barre, &[tenu.clone(), appuis(&[(noeud, axe)])].concat(), &tire)),
            "appui {noeud} {axe}"
        );
    }
    for (noeud, axe) in [(2, 0), (0, 2)] {
        assert!(
            invalide(resoudre(&noeuds, &barre, &tenu, &[Charge { noeud, axe, valeur: 1.0 }])),
            "charge {noeud} {axe}"
        );
    }
    // Les cas limites : deux charges sur le même degré de liberté s'ajoutent ; une charge sur un appui part dans sa
    // réaction ; un bord poussé sans charge ; aucun nœud ; tout imposé.
    let s = resoudre(&noeuds, &barre, &tenu, &[tire[0], tire[0], Charge { noeud: 0, axe: 1, valeur: 5.0 }]).unwrap();
    assert!(
        ecart(&s.deplacements, &[0.0, 0.0, 8.0, 0.0]) < 1e-15 && ecart(&s.reactions, &[-12.0, -5.0, 0.0, 0.0]) < 1e-15,
        "{s:?}"
    );
    let pousse = [tenu.clone(), vec![Appui { noeud: 1, axe: 0, valeur: 0.5 }]].concat();
    let s = resoudre(&noeuds, &barre, &pousse, &[]).unwrap();
    assert!(ecart(&s.efforts, &[0.75]) < 1e-15 && ecart(&s.reactions, &[-0.75, 0.0, 0.75, 0.0]) < 1e-15, "{s:?}");
    assert!((s.energie - 0.1875).abs() < 1e-15, "{s:?}");
    let vide = resoudre(&[], &[], &[], &[]).unwrap();
    assert!(vide.deplacements.is_empty() && vide.efforts.is_empty() && vide.energie == 0.0);
}
