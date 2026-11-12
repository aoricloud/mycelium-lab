//! Étape 15 (M3) : les invariants de l'essai de poussée, sur les neuf colonies à la main et sur deux colonies simulées (la
//! colonie de démonstration de l'exemple, et une plus grande à treize contacts) : la poussée est linéaire en `delta`,
//! l'énergie est `½ K δ²` et la somme de celle des poutres, les réactions des mors s'équilibrent, `EA` et `EI` fois `c`
//! donnent `K` fois `c`, la raideur ne dépend ni du bord qu'on tient, ni du miroir, ni de la transposée, ni de l'ordre ou du
//! sens des segments, **retirer une poutre n'élève jamais `K`**, deux exécutions sont égales au bit près.
//!
//! Les tolérances viennent de la mesure (voir les constantes). Une colonie simulée est mal conditionnée (son plus petit mode,
//! voir `systeme`, va de 1e-9 à 1e-5) : calculée dans un autre ordre, sa raideur change jusqu'à 2e-7 (mesuré sur 120 graines).
//! Six à sept chiffres, pas quinze.

#[path = "../examples/colonie_raideur.rs"]
mod exemple;
#[path = "etape_15/outils.rs"]
mod outils;

use mycelium_lab::croissance::Segment;
use mycelium_lab::piece::Bord;
use mycelium_lab::reseau::{Contact, Hyphe, Origine, Poussee, Reseau, extraire, pousser};
use outils::{COLONIES, HYPHE, bits, contacts, egal, jouer, raideur, segments};

/// Colonies à la main : le plus grand écart relatif mesuré est entre 1e-14 et 2e-14 (une marge de 50) ; une faute fait plus de 1e-3.
const A_LA_MAIN: f64 = 1e-12;
/// Colonies simulées : le plus grand écart relatif mesuré est entre 5e-8 et 1e-7 (une marge de 10).
const SIMULEE: f64 = 1e-6;

struct Cas {
    nom: String,
    segments: Vec<Segment>,
    contacts: Vec<Contact>,
    boite: (usize, usize),
    tolerance: f64,
}

fn tous_les_cas() -> Vec<Cas> {
    let mut tous: Vec<Cas> = COLONIES
        .iter()
        .map(|c| Cas {
            nom: c.nom.to_string(),
            segments: segments(c.hyphes),
            contacts: contacts(c.contacts),
            boite: c.boite,
            tolerance: A_LA_MAIN,
        })
        .collect();
    for (graine, pas) in [(42, 60), (8, 100)] {
        let (simulation, liste, boite) = jouer(exemple::DEMONSTRATION, graine, pas);
        let (segments, nom) = (simulation.segments().to_vec(), format!("colonie simulée, graine {graine}, {pas} pas"));
        tous.push(Cas { nom, segments, contacts: liste, boite, tolerance: SIMULEE });
    }
    tous
}

fn reseau(c: &Cas, hyphe: Hyphe) -> Reseau {
    extraire(&c.segments, &c.contacts, hyphe).unwrap_or_else(|e| panic!("{} : {e:?}", c.nom))
}

fn essai(c: &Cas, de: Bord, vers: Bord, delta: f64, hyphe: Hyphe) -> Poussee {
    pousser(&reseau(c, hyphe), c.boite, de, vers, delta).unwrap_or_else(|e| panic!("{} : {e:?}", c.nom))
}

fn k(c: &Cas) -> f64 {
    essai(c, Bord::Gauche, Bord::Droite, 1.0, HYPHE).raideur
}

/// La plus grande valeur absolue.
fn plus_grand(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |m, x| m.max(x.abs()))
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn la_poussee_est_lineaire_l_energie_est_un_demi_k_delta_carre_et_les_mors_s_equilibrent() {
    for c in &tous_les_cas() {
        let (t, unite) = (c.tolerance, essai(c, Bord::Gauche, Bord::Droite, 1.0, HYPHE));
        let ru = &unite.solution.as_ref().unwrap().reactions;
        assert!(unite.raideur > 0.0 && egal(2.0 * unite.energie, unite.raideur, t), "{} : énergie et K", c.nom);
        for delta in [-2.5, 1e-6, 1e6] {
            let p = essai(c, Bord::Gauche, Bord::Droite, delta, HYPHE);
            assert!(egal(p.raideur, unite.raideur, t), "{} δ = {delta:e} : K = {}", c.nom, p.raideur);
            assert!(egal(p.energie, 0.5 * unite.raideur * delta * delta, t), "{} δ = {delta:e} : énergie", c.nom);
            let ecarts: Vec<f64> =
                p.solution.as_ref().unwrap().reactions.iter().zip(ru).map(|(r, u)| r - delta * u).collect();
            assert!(plus_grand(&ecarts) < t * plus_grand(ru) * delta.abs(), "{} δ = {delta:e} : réactions", c.nom);
        }
        // L'énergie est la somme de celle des poutres, toutes positives ou nulles ; les réactions des deux mors s'équilibrent
        // (les forces, puis les moments autour de l'origine) : leur somme est nulle devant la plus grande, fois la boîte (30).
        let somme: f64 = unite.energies.iter().sum();
        assert!(egal(somme, unite.energie, t) && unite.energies.iter().all(|&e| e >= 0.0), "{} : Σ énergies", c.nom);
        let (mut fx, mut fy, mut moment) = (0.0, 0.0, 0.0);
        for (n, r) in reseau(c, HYPHE).noeuds.iter().zip(ru.chunks(3)) {
            (fx, fy, moment) = (fx + r[0], fy + r[1], moment + r[2] + n.x * r[1] - n.y * r[0]);
        }
        assert!(plus_grand(&[fx, fy, moment]) < t * 30.0 * plus_grand(ru), "{} : {fx:e} {fy:e} {moment:e}", c.nom);
        // La force du mors tenu est l'opposée de celle du mors poussé : K se lit aussi sur lui.
        let tenu: f64 =
            reseau(c, HYPHE).noeuds.iter().zip(ru.chunks(3)).filter(|(n, _)| n.x < 1.0).map(|(_, r)| r[0]).sum();
        assert!(egal(-tenu, unite.raideur, t), "{} : le mors tenu porte {tenu}", c.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn multiplier_ea_et_ei_par_c_multiplie_k_par_c_et_laisse_les_deplacements() {
    for c in &tous_les_cas() {
        let (unite, triple) =
            (essai(c, Bord::Gauche, Bord::Droite, 1.0, HYPHE), Hyphe { ea: 3.0 * HYPHE.ea, ei: 3.0 * HYPHE.ei });
        let p = essai(c, Bord::Gauche, Bord::Droite, 1.0, triple);
        assert!(egal(p.raideur, 3.0 * unite.raideur, c.tolerance), "{} : K = {}", c.nom, p.raideur);
        let (u, v) = (&unite.solution.as_ref().unwrap().deplacements, &p.solution.as_ref().unwrap().deplacements);
        let ecarts: Vec<f64> = u.iter().zip(v).map(|(a, b)| a - b).collect();
        assert!(plus_grand(&ecarts) < c.tolerance * plus_grand(u), "{} : les déplacements", c.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn la_raideur_ne_depend_ni_du_bord_tenu_ni_du_miroir_ni_de_la_transposee_ni_de_l_ordre_ni_du_sens() {
    for c in &tous_les_cas() {
        let (k0, l, n) = (k(c), c.boite.0 as f64, c.segments.len());
        let change = |f: &dyn Fn(&Segment) -> Segment| Cas {
            segments: c.segments.iter().map(f).collect(),
            nom: c.nom.clone(),
            contacts: c.contacts.clone(),
            ..*c
        };
        // On tient le bord droit et on pousse le gauche.
        let echange = essai(c, Bord::Droite, Bord::Gauche, 1.0, HYPHE).raideur;
        assert!(k0 > 0.0 && egal(echange, k0, c.tolerance), "{} : bords échangés", c.nom);
        // Le miroir (`x` devient `largeur − x`), puis la transposée (`x` et `y` échangés, la boîte aussi, poussée de haut en bas).
        let miroir = change(&|s| Segment { x0: l - s.x0, x1: l - s.x1, ..*s });
        assert!(egal(k(&miroir), k0, c.tolerance), "{} : miroir", c.nom);
        let transposee = Cas {
            boite: (c.boite.1, c.boite.0),
            ..change(&|s| Segment { x0: s.y0, y0: s.x0, x1: s.y1, y1: s.x1, ..*s })
        };
        let (bas, haut) = (
            essai(&transposee, Bord::Haut, Bord::Bas, 1.0, HYPHE),
            essai(&transposee, Bord::Bas, Bord::Haut, 1.0, HYPHE),
        );
        assert!(egal(bas.raideur, k0, c.tolerance) && egal(haut.raideur, k0, c.tolerance), "{} : transposée", c.nom);
        // L'ordre : les segments à rebours, les contacts renumérotés et à rebours ; et le sens : chaque segment qui n'est pas une
        // pointe est retourné (le bout d'une pointe est son `x1`).
        let pointes: Vec<usize> = c.contacts.iter().map(|c| c.pointe).collect();
        let a_rebours = Cas {
            segments: (0..n)
                .rev()
                .map(|i| {
                    let s = c.segments[i];
                    if pointes.contains(&i) { s } else { Segment { x0: s.x1, y0: s.y1, x1: s.x0, y1: s.y0, ..s } }
                })
                .collect(),
            contacts: c
                .contacts
                .iter()
                .rev()
                .map(|c| Contact { pointe: n - 1 - c.pointe, cible: n - 1 - c.cible })
                .collect(),
            nom: c.nom.clone(),
            ..*c
        };
        assert!(egal(k(&a_rebours), k0, c.tolerance), "{} : ordre et sens", c.nom);
    }
}

#[test]
#[cfg_attr(not(feature = "etape-15"), ignore)]
fn retirer_une_poutre_n_eleve_jamais_k_et_deux_executions_sont_egales_au_bit_pres() {
    let mut ponts_retires = 0;
    for c in &tous_les_cas() {
        // Un théorème : une poutre de moins ne rend jamais un réseau plus raide. On retire les ponts un à un, du dernier au
        // premier : K ne monte jamais (à l'arrondi près), et il baisse au moins une fois si un pont portait.
        let mut r = reseau(c, HYPHE);
        let avec = raideur(&r, c.boite);
        let mut precedente = avec;
        while let Some(i) = r.origines.iter().rposition(|o| matches!(o, Origine::Pont(_))) {
            r.poutres.remove(i);
            r.origines.remove(i);
            let suivante = raideur(&r, c.boite);
            assert!(suivante <= precedente * (1.0 + c.tolerance), "{} : K monte de {precedente} à {suivante}", c.nom);
            (precedente, ponts_retires) = (suivante, ponts_retires + 1);
        }
        // Les contacts ajoutés un à un, dans l'ordre : la raideur ne baisse pas non plus. Ce n'est plus un théorème (un contact
        // peut fondre deux nœuds, et en déplacer un de moins de `COURT`), mais la règle de fonte le tient sur ces colonies.
        let avec_les_premiers = |nombre: usize| {
            k(&Cas { contacts: c.contacts[..nombre].to_vec(), nom: c.nom.clone(), segments: c.segments.clone(), ..*c })
        };
        let mut precedente = avec_les_premiers(0);
        for nombre in 1..=c.contacts.len() {
            let suivante = avec_les_premiers(nombre);
            assert!(suivante >= precedente * (1.0 - c.tolerance), "{} : {nombre} contacts, K baisse", c.nom);
            precedente = suivante;
        }
        assert_eq!(precedente.to_bits(), avec.to_bits(), "{} : tous les contacts", c.nom);
        let (a, b) =
            (essai(c, Bord::Gauche, Bord::Droite, 1.0, HYPHE), essai(c, Bord::Gauche, Bord::Droite, 1.0, HYPHE));
        assert_eq!(bits(&a), bits(&b), "{} : deux exécutions", c.nom);
    }
    assert_eq!(ponts_retires, 22, "les ponts des onze colonies (3 à la main ; mesuré pour les deux simulées)");
}
