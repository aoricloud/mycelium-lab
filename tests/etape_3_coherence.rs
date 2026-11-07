//! Tests d'acceptation de l'étape 3 (e) : la cohérence de l'état d'une colonie, les paramètres publiés, des cas
//! limites exacts et les contrats de panique. Ils ne dépendent d'aucune empreinte de référence : ce qu'ils vérifient
//! vient de l'issue (le modèle) et de la doc des signatures.
//!
//! Ils n'utilisent, parmi les fonctions de `f64`, que `+`, `-`, `*`, `/` et `sqrt`.

use mycelium_lab::croissance::{COS, SIN};
use mycelium_lab::voisinage::{Colonie, Parametres, ajuster, ajuster_rayon, vitesses_glissantes};

/// Vrai si `valeur` est à `tolerance` de `attendu`, sans `abs` : seul l'écart au carré est comparé.
fn proche(valeur: f64, attendu: f64, tolerance: f64) -> bool {
    let ecart = valeur - attendu;
    ecart * ecart <= tolerance * tolerance
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    (dx * dx + dy * dy).sqrt()
}

fn colonie(centre: (f64, f64), parametres: Parametres, graine: u64, pas_de_temps: u32) -> Colonie {
    let mut colonie = Colonie::depuis_spore(centre, parametres, graine);
    for _ in 0..pas_de_temps {
        colonie.avancer();
        // Garde-fou : une ramification qui s'emballe doit faire échouer le test, pas le faire durer des heures.
        assert!(
            colonie.pointes() <= 1000,
            "graine {graine} : {} pointes, la ramification s'emballe",
            colonie.pointes()
        );
    }
    colonie
}

/// 1. Les paramètres de la figure 3 sont ceux de l'issue, champ par champ.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn figure_3_porte_les_valeurs_publiees() {
    let attendu = Parametres {
        pas: 1.0,
        persistance: 0.1,
        seuil_densite: 0.06,
        probabilite_ramification: 0.4,
        lc: 20.0,
        epsilon: 1.0,
        portee: 60.0,
        angle_max: 180,
        nb_apex_depart: 1,
    };
    assert_eq!(Parametres::figure_3(), attendu);
}

/// Vérifie l'état d'une colonie jouée `nb_pas` pas depuis `centre`, avec `nb_apex_depart` apex au départ : les
/// accesseurs se recoupent, et le rayon est bien la plus grande distance à la spore d'une extrémité de segment.
fn verifier_coherence(colonie: &Colonie, centre: (f64, f64), pas: f64, nb_pas: u32, nb_apex_depart: usize) {
    let n = nb_pas as usize;
    assert_eq!(colonie.temps(), nb_pas, "temps");
    assert_eq!(colonie.rayons().len(), n, "un rayon par pas");
    assert_eq!(colonie.pointes_par_pas().len(), n, "un nombre de pointes par pas");
    assert_eq!(colonie.pointes(), colonie.apex().len(), "pointes() est le nombre d'apex");
    assert_eq!(colonie.pointes() as u32, colonie.pointes_par_pas()[n - 1], "pointes() est la dernière valeur");
    assert_eq!(colonie.ramifications().len(), colonie.pointes() - nb_apex_depart, "une ramification par naissance");

    for (rang, apex) in colonie.apex().iter().enumerate() {
        assert_eq!(apex.id as usize, rang, "les identifiants se suivent");
        assert!(proche(apex.hx * apex.hx + apex.hy * apex.hy, 1.0, 1e-9), "cap unitaire : {apex:?}");
        let loin = distance((apex.x, apex.y), centre);
        assert!(loin <= pas * nb_pas as f64 + 1e-9, "apex {rang} à {loin} de la spore");
    }

    // Un segment par apex et par pas, plus un par branche neuve (qui avance dès son pas de naissance) : au pas `t`, il
    // y a donc autant de nouveaux segments que de pointes.
    let somme: usize = colonie.pointes_par_pas().iter().map(|&p| p as usize).sum();
    assert_eq!(colonie.segments().len(), somme, "nombre de segments");

    let mut rayon_courant = 0.0;
    let mut suivant = 0;
    let segments = colonie.segments();
    let mut temps_precedent = 1;
    for segment in segments {
        assert!((temps_precedent..=nb_pas).contains(&segment.temps), "temps des segments : {segment:?}");
        temps_precedent = segment.temps;
        assert!(proche(segment.longueur(), pas, 1e-9), "longueur du segment {segment:?}");
        assert!((segment.apex as usize) < colonie.pointes(), "apex du segment {segment:?}");
    }
    for t in 1..=nb_pas {
        while suivant < segments.len() && segments[suivant].temps <= t {
            let fin = distance((segments[suivant].x1, segments[suivant].y1), centre);
            if fin > rayon_courant {
                rayon_courant = fin;
            }
            suivant += 1;
        }
        let rayon = colonie.rayons()[(t - 1) as usize];
        assert!(proche(rayon, rayon_courant, 1e-9), "rayon au temps {t} : {rayon} au lieu de {rayon_courant}");
    }
    assert!(proche(colonie.rayon(), rayon_courant, 1e-9), "rayon() : {}", colonie.rayon());
    if nb_apex_depart == 1 {
        assert!(proche(colonie.rayons()[0], pas, 1e-9), "au premier pas, tout est à `pas` de la spore");
    }

    let mut precedent = 0;
    for ramification in colonie.ramifications() {
        assert!((precedent.max(1)..=nb_pas).contains(&ramification.temps), "temps : {ramification:?}");
        precedent = ramification.temps;
        assert!((ramification.apex as usize) < colonie.pointes(), "apex parent : {ramification:?}");
        assert!((0.0..1e12).contains(&ramification.densite), "densité : {ramification:?}");
    }
}

/// 2. L'état d'une colonie de la figure 3 est cohérent, depuis la spore en `(0, 0)` puis depuis une spore ailleurs, avec
///    un autre pas : le rayon se mesure depuis la spore, et le pas est la vitesse d'un apex.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn l_etat_de_la_colonie_est_coherent() {
    let base = Parametres::figure_3();
    verifier_coherence(&colonie((0.0, 0.0), base, 7, 150), (0.0, 0.0), 1.0, 150, 1);
    verifier_coherence(&colonie((500.0, -300.0), base, 3, 100), (500.0, -300.0), 1.0, 100, 1);
    let rapide = Parametres { pas: 2.0, ..base };
    verifier_coherence(&colonie((-40.0, 25.0), rapide, 5, 100), (-40.0, 25.0), 2.0, 100, 1);
    let etoile = Parametres { nb_apex_depart: 3, ..base };
    verifier_coherence(&colonie((0.0, 0.0), etoile, 11, 100), (0.0, 0.0), 1.0, 100, 3);
}

/// 3. Au temps 0 : `nb_apex_depart` apex sur la spore, en étoile (caps espacés de 360 / n degrés), aucun segment.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn au_temps_zero_les_apex_sont_en_etoile_sur_la_spore() {
    let centre = (12.5, -7.0);
    let parametres = Parametres { nb_apex_depart: 3, ..Parametres::figure_3() };
    let colonie = Colonie::depuis_spore(centre, parametres, 4);
    assert_eq!(colonie.temps(), 0);
    assert_eq!(colonie.pointes(), 3);
    assert!(colonie.segments().is_empty() && colonie.rayons().is_empty() && colonie.pointes_par_pas().is_empty());
    assert!(colonie.ramifications().is_empty());
    assert!(proche(colonie.rayon(), 0.0, 1e-12));
    let apex = colonie.apex();
    for (a, b) in [(0, 1), (1, 2), (0, 2)] {
        assert_eq!((apex[a].x, apex[a].y), centre, "apex {a} sur la spore");
        let produit = apex[a].hx * apex[b].hx + apex[a].hy * apex[b].hy;
        assert!(proche(produit, COS[120], 1e-9), "caps {a} et {b} espacés de 120 degrés : {produit}");
    }
}

/// 4. Sans ramification, un apex seul va droit : rien d'autre ne le dévie, et le rayon vaut `pas × t`.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn sans_ramification_un_apex_seul_va_droit() {
    let parametres = Parametres { probabilite_ramification: 0.0, ..Parametres::figure_3() };
    let colonie = colonie((0.0, 0.0), parametres, 9, 60);
    assert_eq!(colonie.rayons().len(), 60);
    assert_eq!(colonie.pointes_par_pas().len(), 60);
    assert!(colonie.ramifications().is_empty());
    assert!(colonie.pointes_par_pas().iter().all(|&p| p == 1));
    for (rang, &rayon) in colonie.rayons().iter().enumerate() {
        assert!(proche(rayon, (rang + 1) as f64, 1e-9), "rayon au temps {} : {rayon}", rang + 1);
    }
}

/// 5. Avec la probabilité 1, sans limite de densité ni autotropisme, et un angle de ramification nul, chaque apex se
///    ramifie à chaque pas : 2, 4, 8, … pointes, toutes dans le cap de départ.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn probabilite_un_les_pointes_doublent_a_chaque_pas() {
    let parametres = Parametres {
        probabilite_ramification: 1.0,
        seuil_densite: f64::INFINITY,
        persistance: 1.0,
        angle_max: 0,
        ..Parametres::figure_3()
    };
    for depart in [1usize, 3] {
        let colonie = colonie((0.0, 0.0), Parametres { nb_apex_depart: depart, ..parametres }, 2, 8);
        for (rang, &pointes) in colonie.pointes_par_pas().iter().enumerate() {
            assert_eq!(pointes as usize, depart << (rang + 1), "pointes au temps {}", rang + 1);
        }
        assert_eq!(colonie.ramifications().len(), (depart << 8) - depart);
        // L'angle est nul et l'autotropisme absent : chaque apex garde le cap d'un apex de départ.
        let departs: Vec<(f64, f64)> = colonie.apex()[..depart].iter().map(|a| (a.hx, a.hy)).collect();
        for apex in colonie.apex() {
            assert!(
                departs.iter().any(|&(hx, hy)| proche(apex.hx, hx, 1e-9) && proche(apex.hy, hy, 1e-9)),
                "cap de l'apex {} : ({}, {})",
                apex.id,
                apex.hx,
                apex.hy
            );
        }
    }
}

/// 6. L'angle de ramification : une branche neuve part à au plus `angle_max` du cap de son parent (ici 90 degrés),
///    et sur 200 graines, on en voit de presque alignées et de très écartées.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn l_angle_de_ramification_est_borne_et_tire_dans_tout_l_intervalle() {
    let parametres = Parametres {
        probabilite_ramification: 1.0,
        seuil_densite: f64::INFINITY,
        persistance: 1.0,
        angle_max: 90,
        ..Parametres::figure_3()
    };
    let mut plus_petit = 1.0;
    let mut plus_grand = -1.0;
    for graine in 1..=200u64 {
        let colonie = colonie((0.0, 0.0), parametres, graine, 1);
        let (parent, fille) = (colonie.apex()[0], colonie.apex()[1]);
        let produit = parent.hx * fille.hx + parent.hy * fille.hy;
        assert!(produit >= -1e-9, "graine {graine} : branche à plus de 90 degrés (produit scalaire {produit})");
        if produit < plus_petit {
            plus_petit = produit;
        }
        if produit > plus_grand {
            plus_grand = produit;
        }
    }
    assert!(plus_petit < COS[72], "aucune branche à plus de 72 degrés : {plus_petit}");
    assert!(plus_grand > COS[18], "aucune branche à moins de 18 degrés : {plus_grand}");
}

/// 7. Le nombre de pointes de la colonie de la figure 3 à 400 pas : entre 150 et 260 pour chacune des graines 1 à 10
///    (observé : 181 à 213 sur le jeu A), et il croît comme `t^β` avec β dans `[1.6 ; 2.2]` en moyenne (observé : 1,91) : les
///    pointes suivent la surface de la colonie.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn le_nombre_de_pointes_a_400_pas() {
    let mut somme_beta = 0.0;
    for graine in 1..=10u64 {
        let colonie = colonie((0.0, 0.0), Parametres::figure_3(), graine, 400);
        let pointes = colonie.pointes_par_pas();
        let fin = pointes[399];
        assert!((150..=260).contains(&fin), "graine {graine} : {fin} pointes à 400 pas");
        let mut temps_log = Vec::new();
        let mut pointes_log = Vec::new();
        for t in 201..=400u32 {
            temps_log.push(mycelium_lab::mesures::logarithme(t as f64));
            pointes_log.push(mycelium_lab::mesures::logarithme(pointes[(t - 1) as usize] as f64));
        }
        somme_beta += ajuster(&temps_log, &pointes_log).pente;
    }
    let beta = somme_beta / 10.0;
    assert!((1.6..=2.2).contains(&beta), "exposant moyen des pointes : {beta}");
}

/// 8. Les mesures sur une loi qui n'est pas une droite : la régression de `r(t) = t²` sur `]200, 400]` a la pente 601
///    (deux fois la moyenne des temps) et les fenêtres de 50 pas de `]200, 400]` ont les pentes `451 + 20 k` ; un
///    nuage bruité a la pente, l'ordonnée et le R² attendus.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn les_mesures_ont_les_bonnes_fenetres_et_les_bonnes_valeurs() {
    let mut carres = Vec::new();
    for t in 1..=400u32 {
        carres.push((t as f64) * (t as f64));
    }
    let pente = ajuster_rayon(&carres, 200, 400).pente;
    assert!(proche(pente, 601.0, 1e-8), "pente de t² sur ]200, 400] : {pente}");
    let vitesses = vitesses_glissantes(&carres, 200, 400, 50, 10);
    assert_eq!(vitesses.len(), 16);
    for (k, &v) in vitesses.iter().enumerate() {
        assert!(proche(v, 451.0 + 20.0 * k as f64, 1e-8), "fenêtre {k} : {v}");
    }

    let bruite = ajuster(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0], &[1.0, 3.0, 2.0, 5.0, 4.0, 7.0]);
    assert!(proche(bruite.pente, 36.0 / 35.0, 1e-12), "pente : {}", bruite.pente);
    assert!(proche(bruite.ordonnee, 23.0 / 21.0, 1e-12), "ordonnée : {}", bruite.ordonnee);
    assert!(proche(bruite.r2, 972.0 / 1225.0, 1e-12), "R² : {}", bruite.r2);
}

/// Vrai si `f` panique.
fn panique<F: FnOnce()>(f: F) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

/// 9. Les paramètres hors contrat font paniquer `depuis_spore` (doc de `Parametres`), les autres non.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn les_parametres_hors_contrat_font_paniquer() {
    let base = Parametres::figure_3();
    let interdits = [
        Parametres { pas: 0.0, ..base },
        Parametres { pas: f64::NAN, ..base },
        Parametres { persistance: 1.5, ..base },
        Parametres { persistance: -0.1, ..base },
        Parametres { probabilite_ramification: 1.5, ..base },
        Parametres { lc: 0.0, ..base },
        Parametres { epsilon: 0.0, ..base },
        Parametres { portee: f64::INFINITY, ..base },
        Parametres { angle_max: 181, ..base },
        Parametres { nb_apex_depart: 0, ..base },
    ];
    for (rang, parametres) in interdits.into_iter().enumerate() {
        assert!(
            panique(|| {
                let _ = Colonie::depuis_spore((0.0, 0.0), parametres, 1);
            }),
            "hors contrat n° {rang} : {parametres:?}"
        );
    }
    for permis in [
        Parametres { seuil_densite: f64::INFINITY, ..base },
        Parametres { persistance: 0.0, ..base },
        Parametres { persistance: 1.0, ..base },
        Parametres { angle_max: 180, probabilite_ramification: 1.0, ..base },
    ] {
        assert!(
            !panique(|| {
                let _ = Colonie::depuis_spore((0.0, 0.0), permis, 1);
            }),
            "dans le contrat : {permis:?}"
        );
    }
}

/// 10. Les fonctions de mesure paniquent hors contrat (doc de chaque fonction).
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn les_mesures_hors_contrat_font_paniquer() {
    let r: Vec<f64> = (1..=300).map(|t| t as f64).collect();
    assert!(
        panique(|| {
            let _ = ajuster(&[0.0, 1.0], &[1.0]);
        }),
        "ajuster : longueurs différentes"
    );
    assert!(
        panique(|| {
            let _ = ajuster(&[0.0], &[1.0]);
        }),
        "ajuster : un seul point"
    );
    assert!(
        panique(|| {
            let _ = ajuster(&[2.0, 2.0, 2.0], &[1.0, 2.0, 3.0]);
        }),
        "ajuster : x constants"
    );
    assert!(
        panique(|| {
            let _ = ajuster_rayon(&r, 10, 11);
        }),
        "ajuster_rayon : un seul point"
    );
    assert!(
        panique(|| {
            let _ = ajuster_rayon(&r, 10, 301);
        }),
        "ajuster_rayon : hors des bornes"
    );
    assert!(
        panique(|| {
            let _ = vitesses_glissantes(&r, 0, 300, 0, 10);
        }),
        "vitesses_glissantes : largeur nulle"
    );
    assert!(
        panique(|| {
            let _ = vitesses_glissantes(&r, 0, 300, 50, 0);
        }),
        "vitesses_glissantes : pas nul"
    );
}

/// 11. La mise à jour est synchrone : trois apex de départ espacés de 120 degrés, sans ramification, gardent la
///     symétrie d'ordre 3 autour de la spore. Si un apex lisait l'état déjà mis à jour de son prédécesseur, la symétrie
///     se casserait dès le deuxième pas.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn les_apex_lisent_tous_le_meme_etat() {
    let centre = (10.0, -5.0);
    let parametres = Parametres { nb_apex_depart: 3, probabilite_ramification: 0.0, ..Parametres::figure_3() };
    let colonie = colonie(centre, parametres, 3, 40);
    assert!(colonie.rayon() > 30.0, "les apex doivent avoir avancé : {}", colonie.rayon());
    let apex = colonie.apex();
    let (c, s) = (COS[120], SIN[120]);
    for i in 0..3 {
        let (a, b) = (apex[i], apex[(i + 1) % 3]);
        let (x, y) = (a.x - centre.0, a.y - centre.1);
        let (tourne_x, tourne_y) = (x * c - y * s, x * s + y * c);
        assert!(
            proche(tourne_x, b.x - centre.0, 1e-6),
            "apex {i} : abscisse tournée {tourne_x} contre {}",
            b.x - centre.0
        );
        assert!(
            proche(tourne_y, b.y - centre.1, 1e-6),
            "apex {i} : ordonnée tournée {tourne_y} contre {}",
            b.y - centre.1
        );
        let (tourne_hx, tourne_hy) = (a.hx * c - a.hy * s, a.hx * s + a.hy * c);
        assert!(proche(tourne_hx, b.hx, 1e-6) && proche(tourne_hy, b.hy, 1e-6), "cap de l'apex {i}");
    }
}

/// 12. Les paramètres du champ agissent. Avec un poids de section `lc` presque nul, un adoucissement `epsilon` énorme ou
///     une `portee` minuscule, le champ de densité est (presque) nul et les apex se ramifient sans frein : au moins 20
///     pointes à 20 pas (observé : 44 à 693), contre 2 pour le modèle de base. Avec un `lc` énorme, le champ interdit
///     toute ramification après la première : 4 pointes au plus à 100 pas.
#[test]
#[cfg_attr(not(feature = "etape-3"), ignore)]
fn les_parametres_du_champ_agissent() {
    let base = Parametres::figure_3();
    let sans_frein = [
        ("lc minuscule", Parametres { lc: 1e-9, ..base }),
        ("epsilon énorme", Parametres { epsilon: 1e6, ..base }),
        ("portée minuscule", Parametres { portee: 0.1, ..base }),
    ];
    for (nom, parametres) in sans_frein {
        for graine in 1..=5u64 {
            let colonie = colonie((0.0, 0.0), parametres, graine, 20);
            let pointes = colonie.pointes_par_pas()[19];
            assert!(pointes >= 20, "{nom}, graine {graine} : {pointes} pointes à 20 pas");
        }
    }
    let bloque = Parametres { lc: 1e6, ..base };
    for graine in 1..=5u64 {
        let colonie = colonie((0.0, 0.0), bloque, graine, 100);
        assert!(colonie.pointes() <= 4, "lc énorme, graine {graine} : {} pointes à 100 pas", colonie.pointes());
    }
}
