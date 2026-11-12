//! Étape 15 (M3) : des colonies faites à la main. L'extraction (nœuds, coupes, ponts, poutres fondues), l'essai de poussée et
//! ses mors, et la raideur exacte de neuf colonies (`tests/etape_15/outils.rs`).

#[path = "etape_15/outils.rs"]
mod outils;

use mycelium_lab::piece::Bord;
use mycelium_lab::raideur::Noeud;
use mycelium_lab::reseau::{COURT, ErreurReseau, Origine, Reseau, extraire, pousser};
use outils::{COLONIES, HYPHE, Ligne, a_la_main, chaine, egal, invalide, pousse, raideur, reseau, segments};

/// La plus grande erreur relative mesurée avec cette tolérance est de 4e-15 (l'échelle coupée) : une marge de 24. Une règle
/// fausse (une coupe au milieu du segment, un pont oublié) fait plus de 1e-3.
const TOLERANCE: f64 = 1e-13;

/// Les poutres de cette origine.
fn de(r: &Reseau, origine: Origine) -> Vec<usize> {
    (0..r.poutres.len()).filter(|&i| r.origines[i] == origine).collect()
}

/// Vrai si la poutre `i` va du point `p` au point `q`, dans ce sens.
fn va(r: &Reseau, i: usize, p: (f64, f64), q: (f64, f64)) -> bool {
    let (a, b) = (r.noeuds[r.poutres[i].a], r.noeuds[r.poutres[i].b]);
    (a.x, a.y) == p && (b.x, b.y) == q
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn les_colonies_a_la_main_ont_le_reseau_et_la_raideur_exacte_que_le_dessin_donne() {
    for c in &COLONIES {
        let r = reseau(c.hyphes, c.contacts);
        let ponts = r.origines.iter().filter(|o| matches!(o, Origine::Pont(_))).count();
        assert_eq!((r.noeuds.len(), r.poutres.len(), ponts), (c.noeuds, c.poutres, c.ponts), "{}", c.nom);
        assert_eq!(r.origines.len(), r.poutres.len(), "{}", c.nom);
        // Chaque segment donne au moins une poutre ; chaque pont est celui d'un contact, une fois.
        for k in 0..segments(c.hyphes).len() {
            assert!(!de(&r, Origine::Segment(k)).is_empty(), "{} : le segment {k}", c.nom);
        }
        assert_eq!((0..c.contacts.len()).map(|n| de(&r, Origine::Pont(n)).len()).sum::<usize>(), c.ponts, "{}", c.nom);
        let (k, sans) = (raideur(&r, c.boite), raideur(&reseau(c.hyphes, &[]), c.boite));
        assert!(egal(k, c.raideur, TOLERANCE), "{} : K = {k} et non {}", c.nom, c.raideur);
        assert!(egal(sans, c.sans_contacts, TOLERANCE), "{} sans contacts : K = {sans}", c.nom);
    }
    // Quelques valeurs redites ici, pour qu'une valeur changée dans la table ne passe pas avec le code changé de même.
    let k = |i: usize| raideur(&reseau(COLONIES[i].hyphes, COLONIES[i].contacts), COLONIES[i].boite);
    for (i, attendue) in [(0, 1.25), (1, 2.5), (3, 0.54432), (4, 0.9006433166547534), (5, 2.5), (7, 1.5607026172383767)]
    {
        assert!(egal(k(i), attendue, TOLERANCE), "{} : {}", COLONIES[i].nom, k(i));
    }
    // La poutre inclinée (3-4-5), en entier : K est la réaction du mors poussé selon `x` ; les deux mors se répondent ; les
    // efforts sont ceux de la poutre dans son repère (`N` = 3/5 en traction, `V`, `M`) ; l'énergie est ½ K.
    let p = pousse(&reseau(COLONIES[3].hyphes, &[]), (4, 5)).unwrap();
    let s = p.solution.as_ref().unwrap();
    let proche = |calcule: &[f64], exact: &[f64]| calcule.iter().zip(exact).all(|(a, b)| (a - b).abs() < TOLERANCE);
    let (fx, fy, m) = (1701.0 / 3125.0, 1068.0 / 3125.0, 72.0 / 125.0);
    assert!(proche(&s.deplacements, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]), "{:?}", s.deplacements);
    assert!(proche(&s.reactions, &[-fx, -fy, m, fx, fy, m]), "{:?}", s.reactions);
    assert!(proche(&s.efforts[0], &[-0.6, 144.0 / 625.0, m, 0.6, -144.0 / 625.0, m]), "{:?}", s.efforts[0]);
    assert!(proche(&[p.energie, p.energies[0]], &[fx / 2.0, fx / 2.0]) && (p.tenus, p.pousses) == (1, 1));
    // La chaîne et sa console : les deux poutres de la chaîne (1,5 et 2,5 de long) portent `N` = EA/4, d'énergie N² L/(2 EA) ;
    // les deux de la console ne portent rien. Les énergies sont dans l'ordre des poutres.
    let p = pousse(&reseau(COLONIES[2].hyphes, &[]), (5, 5)).unwrap();
    assert!(proche(&p.energies, &[0.234375, 0.390625, 0.0, 0.0]) && p.energies.len() == 4, "{:?}", p.energies);
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn un_segment_vise_est_coupe_au_point_le_plus_proche_et_le_pont_va_de_ce_point_a_la_pointe() {
    // L'échelle : le segment 1, de (2 ; 0,5) à (4,5 ; 0,5), est coupé en (3 ; 0,5) ; le pont va de ce point à la pointe (3 ; 1).
    let r = reseau(COLONIES[6].hyphes, COLONIES[6].contacts);
    let (morceaux, pont) = (de(&r, Origine::Segment(1)), de(&r, Origine::Pont(0)));
    assert_eq!((morceaux.len(), pont.len()), (2, 1));
    assert!(va(&r, morceaux[0], (2.0, 0.5), (3.0, 0.5)) && va(&r, morceaux[1], (3.0, 0.5), (4.5, 0.5)));
    assert!(va(&r, pont[0], (3.0, 0.5), (3.0, 1.0)));
    // Le point de coupe est le dernier nœud : les nœuds des segments d'abord, dans l'ordre, la spore en tête.
    assert_eq!((r.noeuds[0], r.noeuds[8]), (Noeud { x: 0.5, y: 0.5 }, Noeud { x: 3.0, y: 0.5 }));
    // La boucle : la pointe (2,75 ; 2,5) est au-delà de la **fin** (2,5 ; 2,5) du segment visé : le pont part de ce bout, sans
    // coupe. Et une pointe en deçà du **début** du segment visé est reliée à ce début : deux hyphes bout à bout, K = EA/4.
    let r = reseau(COLONIES[5].hyphes, COLONIES[5].contacts);
    assert!(va(&r, de(&r, Origine::Pont(0))[0], (2.5, 2.5), (2.75, 2.5)) && de(&r, Origine::Segment(3)).len() == 1);
    let bout_a_bout: [Ligne; 2] = [&[(0.5, 0.5), (2.5, 0.5)], &[(3.0, 0.5), (4.5, 0.5)]];
    let r = reseau(&bout_a_bout, &[(0, 1)]);
    assert!(va(&r, 2, (3.0, 0.5), (2.5, 0.5)) && (r.noeuds.len(), r.poutres.len()) == (4, 3));
    assert!(egal(raideur(&r, (5, 1)), 1.25, TOLERANCE));
    // Deux contacts sur un même segment, donnés dans le désordre : les morceaux sont rangés le long du segment (la raideur
    // 2 EA/L = 2 le dit : dans le désordre, la file serait plus longue), et chaque pont porte le rang de son contact. Les deux
    // barreaux ne portent rien, à l'arrondi près : l'erreur mesurée est de 4e-14, d'où 1e-12 (une marge de 24).
    let deux_tiges: [Ligne; 4] = [
        &[(0.5, 0.5), (5.5, 0.5)],
        &[(0.5, 0.5), (0.5, 2.5), (2.0, 2.5), (4.0, 2.5), (5.5, 2.5)],
        &[(2.0, 2.5), (2.0, 1.0)],
        &[(4.0, 2.5), (4.0, 1.0)],
    ];
    let r = reseau(&deux_tiges, &[(6, 0), (5, 0)]);
    let morceaux = de(&r, Origine::Segment(0));
    assert_eq!(morceaux.len(), 3);
    for (i, x) in morceaux.iter().zip([(0.5, 2.0), (2.0, 4.0), (4.0, 5.5)]) {
        assert!(va(&r, *i, (x.0, 0.5), (x.1, 0.5)), "morceau {i}");
    }
    let pont = |rang: usize| de(&r, Origine::Pont(rang))[0];
    assert!(va(&r, pont(0), (4.0, 0.5), (4.0, 1.0)) && va(&r, pont(1), (2.0, 0.5), (2.0, 1.0)));
    assert!(egal(raideur(&r, (6, 4)), 2.0, 1e-12));
    // Une ramification naît au bout du segment de sa mère : le nœud est partagé (3 nœuds pour 2 segments), et `-0.0` est le
    // même point que `0.0`.
    let ramification: [Ligne; 2] = [&[(1.5, 0.5), (-0.0, 0.5)], &[(0.0, 0.5), (0.0, 1.5)]];
    let r = reseau(&ramification, &[]);
    assert_eq!((r.noeuds.len(), r.poutres.len(), r.poutres[0].b, r.poutres[1].a), (3, 2, 1, 1));
    let ramification: [Ligne; 2] = [&[(0.5, 1.5), (0.5, -0.0)], &[(0.5, 0.0), (1.5, 0.0)]];
    assert_eq!(reseau(&ramification, &[]).noeuds.len(), 3);
    // À un bit près, ce n'est plus le même nœud : deux hyphes qui se frôlent sans se toucher restent deux morceaux.
    let frole: [Ligne; 2] = [&[(0.5, 0.5), (1.5, 0.5)], &[(1.5000000000000002, 0.5), (2.5, 0.5)]];
    assert_eq!(reseau(&frole, &[]).noeuds.len(), 4);
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn une_poutre_plus_courte_que_court_est_fondue_et_aucune_ne_le_reste() {
    assert_eq!(COURT, 0.01);
    // « Plus courte que » : une poutre de longueur `COURT` tout juste reste.
    assert_eq!(reseau(&[&[(0.5, 0.0), (0.5, COURT), (0.5, 1.0)]], &[]).noeuds.len(), 3);
    let sans_poutre_courte = |r: &Reseau| {
        r.poutres.iter().all(|p| {
            let (a, b) = (r.noeuds[p.a], r.noeuds[p.b]);
            (b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y) >= COURT * COURT
        })
    };
    // L'échelle dont la tige finit à 1/256 au-dessus de la première hyphe : le pont est trop court, **la pointe rejoint le point
    // de coupe** (3 ; 0,5), l'hyphe visée reste droite. C'est le réseau de « la pointe sur le segment » : 8 nœuds, 8 poutres,
    // aucun pont, et la même raideur exacte. À 1/32 le pont reste (9 nœuds, 9 poutres) et ne porte rien (2e-12 mesuré : une marge de 56).
    for (ecart, noeuds, poutres, tolerance) in
        [(0.0, 8, 8, TOLERANCE), (1.0 / 256.0, 8, 8, TOLERANCE), (1.0 / 32.0, 9, 9, 1e-10)]
    {
        let tige: Ligne = if ecart == 0.0 {
            &[(3.0, 2.5), (3.0, 0.5)]
        } else if ecart < 0.01 {
            &[(3.0, 2.5), (3.0, 0.5 + 1.0 / 256.0)]
        } else {
            &[(3.0, 2.5), (3.0, 0.5 + 1.0 / 32.0)]
        };
        let r = reseau(&[COLONIES[6].hyphes[0], COLONIES[6].hyphes[1], tige], &[(6, 1)]);
        assert_eq!((r.noeuds.len(), r.poutres.len()), (noeuds, poutres), "écart {ecart}");
        assert!(r.noeuds.contains(&Noeud { x: 3.0, y: 0.5 }) && sans_poutre_courte(&r), "écart {ecart}");
        assert_eq!(r.noeuds.iter().filter(|n| n.x == 3.0).count(), if poutres == 9 { 3 } else { 2 }, "écart {ecart}");
        let k = raideur(&r, (5, 5));
        assert!(egal(k, 2.5, tolerance), "écart {ecart} : K = {k}");
    }
    // Une pointe qui vise un segment à 1/256 de son début : le point de coupe (le plus récent) rejoint ce début (2 ; 0,5), et le
    // pont en part.
    let pres_du_bout: [Ligne; 3] =
        [COLONIES[6].hyphes[0], COLONIES[6].hyphes[1], &[(3.0, 2.5), (2.0 + 1.0 / 256.0, 1.0)]];
    let r = reseau(&pres_du_bout, &[(6, 1)]);
    assert_eq!((r.noeuds.len(), r.poutres.len(), de(&r, Origine::Segment(1)).len()), (8, 8, 1));
    assert!(va(&r, de(&r, Origine::Pont(0))[0], (2.0, 0.5), (2.0 + 1.0 / 256.0, 1.0)) && sans_poutre_courte(&r));
    // Un segment de 1/256 au milieu d'une chaîne disparaît, et le nœud gardé est le plus ancien (x = 1,5) : K = EA/4.
    let r = reseau(&[&[(0.5, 1.5), (1.5, 1.5), (1.5 + 1.0 / 256.0, 1.5), (4.5, 1.5)]], &[]);
    assert_eq!(r.noeuds.iter().map(|n| n.x).collect::<Vec<_>>(), [0.5, 1.5, 4.5]);
    assert!(r.poutres.len() == 2 && egal(raideur(&r, (5, 3)), 1.25, TOLERANCE));
    // La fonte recommence tant qu'une poutre est trop courte. Ici `p` = 1,5, puis `r` = `p` + 1/256 (nés dans cet ordre), puis `q`
    // = `r` + 1/128 : la poutre `p`-`q` (0,0117) n'est pas trop courte, `q`-`r` (0,0078) l'est : `q` rejoint `r`, et la poutre
    // `p`-`q` devenue `p`-`r` (0,0039) est trop courte à son tour : il reste 0,5, 1,5 et 4,5, et K = EA/4.
    let (p, q, rr) = (1.5, 1.5 + 3.0 / 256.0, 1.5 + 1.0 / 256.0);
    let en_cascade: [&[(f64, f64)]; 3] =
        [&[(0.5, 1.5), (p, 1.5)], &[(4.5, 1.5), (rr, 1.5)], &[(p, 1.5), (q, 1.5), (rr, 1.5)]];
    let r = reseau(&en_cascade, &[]);
    assert_eq!(r.noeuds.iter().map(|n| n.x).collect::<Vec<_>>(), [0.5, 1.5, 4.5]);
    assert!(r.poutres.len() == 2 && sans_poutre_courte(&r) && egal(raideur(&r, (5, 3)), 1.25, TOLERANCE));
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn un_mors_est_la_colonne_ou_la_ligne_de_cases_du_bord() {
    // Dans une boîte de 5 cases : x = 0,25 et 0,75 sont dans la colonne 0 (tenus), 1 dans la colonne 1, 4 et 4,5 dans la colonne 4
    // (poussés) ; 3,75 n'y est pas, ni −0,5 et 5,5, hors de la boîte. Les deux nœuds de chaque mors ne se déforment pas, et ce
    // qui dépasse ne porte rien : L = 4 − 0,75 et K = 5/3,25.
    let x = [-0.5, 0.25, 0.75, 1.0, 2.5, 3.75, 4.0, 4.5, 5.5];
    let colonne: Vec<(f64, f64)> = x.iter().map(|&x| (x, 1.5)).collect();
    let ligne: Vec<(f64, f64)> = x.iter().map(|&y| (1.5, y)).collect();
    for (points, bords, boite) in
        [(&colonne, (Bord::Gauche, Bord::Droite), (5, 9)), (&ligne, (Bord::Haut, Bord::Bas), (9, 5))]
    {
        for (de, vers) in [bords, (bords.1, bords.0)] {
            let p = pousser(&chaine(points), boite, de, vers, 1.0).unwrap();
            assert_eq!((p.tenus, p.pousses), (2, 2), "{de:?} vers {vers:?}");
            assert!(egal(p.raideur, 5.0 / 3.25, TOLERANCE), "{de:?} vers {vers:?} : K = {}", p.raideur);
        }
    }
    // Un nœud juste avant la colonne suivante est encore dans la colonne : 0,9999999999999999 est dans la colonne 0, 1 n'y est pas.
    let p = pousse(&chaine(&[(0.9999999999999999, 1.5), (4.0, 1.5)]), (5, 3)).unwrap();
    assert_eq!((p.tenus, p.pousses), (1, 1));
    let p = pousse(&chaine(&[(1.0, 1.5), (4.0, 1.5)]), (5, 3)).unwrap();
    assert_eq!((p.tenus, p.pousses, p.raideur), (0, 1, 0.0));
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn une_colonie_qui_ne_traverse_pas_a_une_raideur_nulle_et_ce_n_est_pas_une_erreur() {
    let vide = Reseau { noeuds: Vec::new(), poutres: Vec::new(), origines: Vec::new() };
    assert_eq!(extraire(&[], &[], HYPHE).unwrap(), vide);
    // Aucun nœud ; un seul mors touché ; ni l'un ni l'autre (la colonie est au milieu).
    for (nom, reseau) in [
        ("colonie vide", vide),
        ("seulement à gauche", chaine(&[(0.5, 1.5), (2.5, 1.5)])),
        ("seulement à droite", chaine(&[(2.5, 1.5), (4.5, 1.5)])),
        ("au milieu", chaine(&[(1.5, 1.5), (3.5, 1.5)])),
    ] {
        let p = pousse(&reseau, (5, 3)).unwrap();
        assert_eq!((p.raideur, p.energie, p.solution.is_none(), p.energies.len()), (0.0, 0.0, true, 0), "{nom}");
    }
    // Une seule poutre d'un mors à l'autre : K = EA/L = 5, et sa solution est rendue.
    let p = pousse(&chaine(&[(0.5, 0.5), (1.5, 0.5)]), (2, 1)).unwrap();
    assert!(egal(p.raideur, 5.0, TOLERANCE) && p.solution.is_some() && p.energies.len() == 1, "K = {}", p.raideur);
    // Un nœud sans poutre (un réseau mal formé) est un mécanisme, pas un nombre.
    let mut seul = chaine(&[(0.5, 0.5), (2.5, 0.5)]);
    seul.noeuds.push(Noeud { x: 1.5, y: 0.5 });
    assert_eq!(pousse(&seul, (3, 1)).err(), Some(ErreurReseau::Mecanisme));
    // Une hyphe que rien ne tient (elle flotte entre les deux mors, à côté d'une chaîne qui traverse) est un mécanisme aussi.
    let flotte: [Ligne; 2] = [&[(0.5, 0.5), (4.5, 0.5)], &[(1.5, 1.25), (2.25, 2.5), (3.5, 1.75)]];
    assert_eq!(pousse(&reseau(&flotte, &[]), (5, 3)).err(), Some(ErreurReseau::Mecanisme));
    // Deux morceaux, un par mors, que rien ne relie : la force est nulle, et le calcul le refuse (un zéro ne se distingue pas
    // d'un résultat qui s'efface). Une colonie simulée est d'un seul morceau.
    assert!(invalide(pousse(&a_la_main(&[(0.5, 0.5), (1.5, 0.5), (2.5, 0.5), (3.5, 0.5)], &[(0, 1), (2, 3)]), (4, 1))));
}
