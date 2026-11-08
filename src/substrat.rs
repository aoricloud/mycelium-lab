//! Substrat : diffusion explicite à cinq points, avec des bords sans flux.
//!
//! Le substrat est un champ de nutriments `c(x, y)` sur une grille rectangulaire de `largeur × hauteur` cases,
//! de côté `dx`. Il suit l'équation de diffusion `∂c/∂t = D ∇²c`, intégrée par le schéma explicite à cinq points
//! (Euler avant en temps, différences centrées en espace) :
//!
//! `c'(i, j) = c(i, j) + r · (c(i-1, j) + c(i+1, j) + c(i, j-1) + c(i, j+1) - 4 c(i, j))`, avec `r = D dt / dx²`.
//!
//! Aux bords, il n'y a aucun flux : un voisin qui sortirait de la grille est remplacé par la case elle-même
//! (condition de Neumann par réflexion), donc rien ne sort de la grille et la masse totale est conservée.
//! Le schéma est stable si `r ≤ 1/4` ; le module refuse tout paramètre qui ne l'est pas, à la construction.
//!
//! Le calcul n'utilise que `+`, `-`, `*`, `/` : il est déterministe d'une machine à l'autre. Le pas lit l'ancien
//! champ en entier avant d'écrire le nouveau (pas de mise à jour en place), donc l'ordre des cases n'influe pas
//! sur le résultat.

/// Pourquoi une construction de [`Substrat`] est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurSubstrat {
    /// La largeur ou la hauteur est nulle.
    DimensionNulle,
    /// `D`, `dt` ou `dx` n'est pas fini, ou `D < 0`, ou `dt ≤ 0`, ou `dx ≤ 0`.
    ParametreInvalide,
    /// `D dt / dx²` dépasse `1/4` : le schéma explicite serait instable.
    PasInstable,
}

/// Un champ de nutriments sur une grille, et ses paramètres de diffusion.
///
/// Contrat : les valeurs sont des `f64` rangés ligne par ligne (`y` en indice lent, `x` en indice rapide). Un
/// champ neuf est nul partout. Les coordonnées `(x, y)` vont de `0` à `largeur - 1` et de `0` à `hauteur - 1`.
/// Tout est déterministe : mêmes paramètres, mêmes dépôts, même nombre de pas, mêmes octets.
#[derive(Debug, Clone, PartialEq)]
pub struct Substrat {
    largeur: usize,
    hauteur: usize,
    d: f64,
    dt: f64,
    dx: f64,
    cases: Vec<f64>,
    bloquees: Vec<bool>,
}

impl Substrat {
    /// Construit un champ nul de `largeur × hauteur` cases, de coefficient de diffusion `d`, de pas de temps
    /// `dt` et de côté de case `dx` (`d` en longueur²/temps, `dt` en temps, `dx` en longueur).
    ///
    /// Contrat : refuse avec [`ErreurSubstrat::DimensionNulle`] si une dimension est nulle ; avec
    /// [`ErreurSubstrat::ParametreInvalide`] si `d`, `dt` ou `dx` n'est pas fini, si `d < 0`, `dt ≤ 0` ou
    /// `dx ≤ 0` (`d = 0` est permis : le champ ne bouge pas) ; avec [`ErreurSubstrat::PasInstable`] si
    /// `d * dt / (dx * dx) > 0.25`. La limite `0.25` elle-même est acceptée.
    pub fn nouveau(largeur: usize, hauteur: usize, d: f64, dt: f64, dx: f64) -> Result<Substrat, ErreurSubstrat> {
        if largeur == 0 || hauteur == 0 {
            return Err(ErreurSubstrat::DimensionNulle);
        }
        if !d.is_finite() || !dt.is_finite() || !dx.is_finite() || d < 0.0 || dt <= 0.0 || dx <= 0.0 {
            return Err(ErreurSubstrat::ParametreInvalide);
        }
        if d * dt / (dx * dx) > 0.25 {
            return Err(ErreurSubstrat::PasInstable);
        }
        Ok(Substrat {
            largeur,
            hauteur,
            d,
            dt,
            dx,
            cases: vec![0.0; largeur * hauteur],
            bloquees: vec![false; largeur * hauteur],
        })
    }

    /// Nombre de cases en `x`.
    pub fn largeur(&self) -> usize {
        self.largeur
    }

    /// Nombre de cases en `y`.
    pub fn hauteur(&self) -> usize {
        self.hauteur
    }

    /// Le nombre de diffusion `r = D dt / dx²`, toujours dans `[0, 0.25]` pour un substrat construit.
    pub fn nombre_de_diffusion(&self) -> f64 {
        self.d * self.dt / (self.dx * self.dx)
    }

    /// La quantité de nutriments dans la case `(x, y)`.
    ///
    /// Contrat : panique si `x >= largeur()` ou `y >= hauteur()`.
    pub fn valeur(&self, x: usize, y: usize) -> f64 {
        assert!(x < self.largeur && y < self.hauteur, "case ({x}, {y}) hors de la grille");
        let i = y * self.largeur + x;
        if self.bloquees[i] { 0.0 } else { self.cases[i] }
    }

    /// Ajoute `quantite` à la case `(x, y)` (la masse totale croît d'autant).
    ///
    /// Contrat : panique si la case est hors de la grille, ou si `quantite` n'est pas finie ou est négative.
    pub fn deposer(&mut self, x: usize, y: usize, quantite: f64) {
        assert!(x < self.largeur && y < self.hauteur, "case ({x}, {y}) hors de la grille");
        assert!(quantite.is_finite() && quantite >= 0.0, "quantité invalide : {quantite}");
        let i = y * self.largeur + x;
        assert!(!self.bloquees[i], "case ({x}, {y}) bloquée : dépôt refusé");
        self.cases[i] += quantite;
    }

    /// Retire au plus `quantite` de la case `(x, y)` et rend ce qui a été retiré.
    ///
    /// Contrat : le résultat n'excède jamais ce que la case contenait, et la masse totale baisse d'autant.
    /// Panique si la case est hors de la grille, ou si `quantite` n'est pas finie ou est négative.
    pub fn retirer(&mut self, x: usize, y: usize, quantite: f64) -> f64 {
        assert!(x < self.largeur && y < self.hauteur, "case ({x}, {y}) hors de la grille");
        assert!(quantite.is_finite() && quantite >= 0.0, "quantité invalide : {quantite}");
        let i = y * self.largeur + x;
        if self.bloquees[i] {
            return 0.0;
        }
        let retire = quantite.min(self.cases[i]);
        self.cases[i] -= retire;
        retire
    }

    /// Bloque le rectangle de cases `x0..=x1` × `y0..=y1` : ces cases valent `0.0` pour toujours, ne reçoivent
    /// rien et ne donnent rien. Leur quantité du moment est retirée (la masse totale baisse d'autant).
    ///
    /// Contrat : panique si `x0 > x1`, `y0 > y1` ou si une borne sort de la grille. Bloquer deux fois la même
    /// case est permis. Ensuite, [`Substrat::pas`] traite un voisin bloqué comme un bord (aucun flux), une case
    /// bloquée reste à `0.0`, [`Substrat::valeur`] et [`Substrat::retirer`] y rendent `0.0`, et
    /// [`Substrat::deposer`] y panique (le message contient « bloquée »).
    pub fn bloquer(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        assert!(x0 <= x1 && y0 <= y1, "rectangle invalide : ({x0}, {y0}) à ({x1}, {y1})");
        assert!(
            x1 < self.largeur && y1 < self.hauteur,
            "rectangle ({x0}, {y0}) à ({x1}, {y1}) hors de la grille {}×{}",
            self.largeur,
            self.hauteur
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = y * self.largeur + x;
                self.cases[i] = 0.0;
                self.bloquees[i] = true;
            }
        }
    }

    /// Vrai si le segment de `(x0, y0)` à `(x1, y1)` ne touche aucune case bloquée.
    ///
    /// Règle du plancher : chaque point `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)` (la case `(i, j)` est le carré
    /// semi-ouvert `[i, i+1) × [j, j+1)`) ; le segment fermé est libre si aucun de ses points, extrémités comprises,
    /// n'est dans une case bloquée. Finir, longer ou passer par le coin bas-gauche d'une case bloquée, c'est la toucher.
    /// Le calcul est exact (aucune erreur d'arrondi à craindre sur les cases franchies).
    ///
    /// Contrat : panique si une coordonnée n'est pas finie ou sort de `[0, largeur] × [0, hauteur]`.
    pub fn segment_libre(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
        self.verifier_boite(x0, y0, x1, y1);
        let (fx0, fy0) = (x0.floor() as i64, y0.floor() as i64);
        if self.case_bloquee(fx0, fy0) {
            return false;
        }
        self.parcours(x0, y0, x1, y1).is_none()
    }

    /// La face par laquelle le segment de `(x0, y0)` à `(x1, y1)` entre dans la première case bloquée qu'il touche,
    /// ou `None` si le segment est libre (c'est-à-dire si [`Substrat::segment_libre`] est vrai).
    ///
    /// La face est la ligne de la grille franchie pour entrer dans cette case : [`crate::tropisme::Face::Verticale`]
    /// si c'est une ligne `x = entier`, [`crate::tropisme::Face::Horizontale`] si c'est une ligne `y = entier`. Pour
    /// une entrée exacte par un coin, en venant de la case diagonale `(px, py)` vers la case `(cx, cy)` : la face est
    /// horizontale si seule `(px, cy)` est bloquée, verticale dans tous les autres cas.
    ///
    /// Contrat : mêmes paniques que [`Substrat::segment_libre`], et panique si la case `(⌊x0⌋, ⌊y0⌋)` est bloquée.
    pub fn face_de_contact(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Option<crate::tropisme::Face> {
        self.verifier_boite(x0, y0, x1, y1);
        let (fx0, fy0) = (x0.floor() as i64, y0.floor() as i64);
        assert!(!self.case_bloquee(fx0, fy0), "départ dans la case bloquée ({fx0}, {fy0})");
        self.parcours(x0, y0, x1, y1)
    }

    /// Panique si une des quatre coordonnées n'est pas finie ou sort de la boîte fermée.
    fn verifier_boite(&self, x0: f64, y0: f64, x1: f64, y1: f64) {
        assert!(
            x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite(),
            "coordonnée non finie : ({x0}, {y0}) -> ({x1}, {y1})"
        );
        let (l, h) = (self.largeur as f64, self.hauteur as f64);
        assert!(
            (0.0..=l).contains(&x0) && (0.0..=l).contains(&x1) && (0.0..=h).contains(&y0) && (0.0..=h).contains(&y1),
            "coordonnée hors de la boîte [0, {l}] × [0, {h}] : ({x0}, {y0}) -> ({x1}, {y1})"
        );
    }

    /// Vrai si la case `(i, j)` est bloquée ; faux hors de la grille (indices négatifs compris).
    fn case_bloquee(&self, i: i64, j: i64) -> bool {
        if i < 0 || j < 0 {
            return false;
        }
        let (i, j) = (i as usize, j as usize);
        if i >= self.largeur || j >= self.hauteur {
            return false;
        }
        self.bloquees[j * self.largeur + i]
    }

    /// La face par laquelle on entre dans la case `n` depuis la case `p` : seule la colonne change → verticale,
    /// seule la ligne → horizontale, les deux → la règle du coin.
    fn face_entree(&self, p: (i64, i64), n: (i64, i64)) -> crate::tropisme::Face {
        let change_colonne = n.0 != p.0;
        let change_ligne = n.1 != p.1;
        if change_colonne && change_ligne {
            self.face_coin(n, p)
        } else if change_colonne {
            crate::tropisme::Face::Verticale
        } else {
            crate::tropisme::Face::Horizontale
        }
    }

    /// La face d'une entrée exacte par un coin dans la case `c`, en venant de la case diagonale `p` : horizontale si
    /// seule `(c.0, p.1)` est libre et `(p.0, c.1)` bloquée, verticale dans tous les autres cas.
    fn face_coin(&self, c: (i64, i64), p: (i64, i64)) -> crate::tropisme::Face {
        if !self.case_bloquee(c.0, p.1) && self.case_bloquee(p.0, c.1) {
            crate::tropisme::Face::Horizontale
        } else {
            crate::tropisme::Face::Verticale
        }
    }

    /// Le parcours d'Amanatides et Woo du segment fermé : `None` s'il ne touche aucune case bloquée, sinon la face
    /// d'entrée dans la première case bloquée touchée. La case de départ est supposée libre.
    fn parcours(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Option<crate::tropisme::Face> {
        let (fx0, fy0) = (x0.floor() as i64, y0.floor() as i64);
        let (mut i, fin_i, si) = axe(x0, x1);
        let (mut j, fin_j, sj) = axe(y0, y1);
        if (i, j) != (fx0, fy0) && self.case_bloquee(i, j) {
            return Some(self.face_entree((fx0, fy0), (i, j)));
        }
        while fin_i != i || fin_j != j {
            let fi = if si > 0 { i + 1 } else { i };
            let fj = if sj > 0 { j + 1 } else { j };
            let ti = if fin_i != i { (fi as f64 - x0) / (x1 - x0) } else { f64::INFINITY };
            let tj = if fin_j != j { (fj as f64 - y0) / (y1 - y0) } else { f64::INFINITY };
            if ti == tj {
                if (fi, fj) != (i, j) && self.case_bloquee(fi, fj) {
                    return Some(if si > 0 && sj > 0 {
                        self.face_coin((fi, fj), (i, j))
                    } else if si > 0 {
                        crate::tropisme::Face::Verticale
                    } else {
                        crate::tropisme::Face::Horizontale
                    });
                }
                let (pi, pj) = (i, j);
                i += si;
                j += sj;
                if self.case_bloquee(i, j) {
                    return Some(self.face_coin((i, j), (pi, pj)));
                }
            } else if ti < tj {
                i += si;
                if self.case_bloquee(i, j) {
                    return Some(crate::tropisme::Face::Verticale);
                }
            } else {
                j += sj;
                if self.case_bloquee(i, j) {
                    return Some(crate::tropisme::Face::Horizontale);
                }
            }
        }
        let (fx1, fy1) = (x1.floor() as i64, y1.floor() as i64);
        if (fx1, fy1) != (i, j) && self.case_bloquee(fx1, fy1) {
            return Some(self.face_entree((i, j), (fx1, fy1)));
        }
        None
    }

    /// Vrai si la case `(x, y)` est bloquée.
    ///
    /// Contrat : panique si la case est hors de la grille.
    pub fn est_bloquee(&self, x: usize, y: usize) -> bool {
        assert!(x < self.largeur && y < self.hauteur, "case ({x}, {y}) hors de la grille");
        self.bloquees[y * self.largeur + x]
    }

    /// Le gradient du champ dans la case `(x, y)`, par différences centrées sur les quatre cases voisines :
    /// `gx = (droite - gauche) / 2` et `gy = (bas - haut) / 2`, avec `y` qui croît avec l'indice de ligne.
    ///
    /// Comme dans [`Substrat::pas`], un voisin hors de la grille ou bloqué est remplacé par la case elle-même
    /// (aucun flux). Le gradient d'un champ uniforme est exactement `(0.0, 0.0)` ; celui d'une case bloquée aussi.
    ///
    /// Contrat : panique si `x >= largeur()` ou `y >= hauteur()`.
    pub fn gradient(&self, x: usize, y: usize) -> (f64, f64) {
        assert!(x < self.largeur && y < self.hauteur, "case ({x}, {y}) hors de la grille");
        let i = y * self.largeur + x;
        if self.bloquees[i] {
            return (0.0, 0.0);
        }
        let c = self.cases[i];
        // Comme dans `pas` : un voisin hors de la grille ou bloqué est remplacé par la case elle-même (aucun flux).
        let gauche = if x == 0 || self.bloquees[i - 1] { c } else { self.cases[i - 1] };
        let droite = if x + 1 == self.largeur || self.bloquees[i + 1] { c } else { self.cases[i + 1] };
        let haut = if y == 0 || self.bloquees[i - self.largeur] { c } else { self.cases[i - self.largeur] };
        let bas =
            if y + 1 == self.hauteur || self.bloquees[i + self.largeur] { c } else { self.cases[i + self.largeur] };
        ((droite - gauche) * 0.5, (bas - haut) * 0.5)
    }

    /// Avance le champ d'un pas de temps `dt` par le schéma à cinq points, bords sans flux.
    ///
    /// Contrat : la masse totale est conservée (aux erreurs d'arrondi près) ; l'ancien champ est lu en entier
    /// avant d'écrire le nouveau ; l'ordre de calcul est fixe.
    pub fn pas(&mut self) {
        let ancien = self.cases.clone();
        let (largeur, hauteur) = (self.largeur, self.hauteur);
        let r = self.nombre_de_diffusion();
        if !self.bloquees.iter().any(|&b| b) {
            // Aucune case bloquée : le schéma d'origine, à l'identique.
            for y in 0..hauteur {
                for x in 0..largeur {
                    let i = y * largeur + x;
                    let c = ancien[i];
                    let gauche = if x == 0 { c } else { ancien[i - 1] };
                    let droite = if x + 1 == largeur { c } else { ancien[i + 1] };
                    let haut = if y == 0 { c } else { ancien[i - largeur] };
                    let bas = if y + 1 == hauteur { c } else { ancien[i + largeur] };
                    self.cases[i] = c + r * (gauche + droite + haut + bas - 4.0 * c);
                }
            }
            return;
        }
        // Avec des obstacles : un voisin bloqué est traité comme un bord (aucun flux), une case bloquée reste nulle.
        for y in 0..hauteur {
            for x in 0..largeur {
                let i = y * largeur + x;
                if self.bloquees[i] {
                    self.cases[i] = 0.0;
                    continue;
                }
                let c = ancien[i];
                let gauche = if x == 0 || self.bloquees[i - 1] { c } else { ancien[i - 1] };
                let droite = if x + 1 == largeur || self.bloquees[i + 1] { c } else { ancien[i + 1] };
                let haut = if y == 0 || self.bloquees[i - largeur] { c } else { ancien[i - largeur] };
                let bas = if y + 1 == hauteur || self.bloquees[i + largeur] { c } else { ancien[i + largeur] };
                self.cases[i] = c + r * (gauche + droite + haut + bas - 4.0 * c);
            }
        }
    }

    /// Avance le champ de `n` pas : strictement équivalent à appeler [`Substrat::pas`] `n` fois, au même
    /// octet près. `n = 0` ne change rien.
    pub fn pas_n(&mut self, n: usize) {
        for _ in 0..n {
            self.pas();
        }
    }

    /// La somme des valeurs de toutes les cases, additionnées dans l'ordre des lignes puis des colonnes
    /// (`y` lent, `x` rapide).
    pub fn masse_totale(&self) -> f64 {
        let mut somme = 0.0;
        for &v in &self.cases {
            somme += v;
        }
        somme
    }

    /// L'écriture canonique du champ, pour comparer « à l'octet » et pour l'empreinte.
    ///
    /// Format : `largeur` puis `hauteur`, chacun en `u64` petit-boutiste (8 octets), puis la valeur de chaque
    /// case, ligne par ligne (`y` lent, `x` rapide), en `f64::to_bits` petit-boutiste (8 octets). Les paramètres
    /// `d`, `dt` et `dx` n'en font pas partie. Longueur : `16 + 8 * largeur * hauteur` octets.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let mut octets = Vec::with_capacity(16 + 8 * self.cases.len());
        octets.extend_from_slice(&(self.largeur as u64).to_le_bytes());
        octets.extend_from_slice(&(self.hauteur as u64).to_le_bytes());
        for &v in &self.cases {
            octets.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        octets
    }
}

/// L'axe du parcours pour la coordonnée qui va de `a0` à `a1` : la case de départ, la dernière case parcourue et le
/// sens. Le plafond s'écrit `-(-a).floor()`, sans fonction mathématique.
fn axe(a0: f64, a1: f64) -> (i64, i64, i64) {
    if a1 > a0 {
        (a0.floor() as i64, (-(-a1).floor()) as i64 - 1, 1)
    } else if a1 < a0 {
        ((-(-a0).floor()) as i64 - 1, a1.floor() as i64, -1)
    } else {
        let i = a0.floor() as i64;
        (i, i, 0)
    }
}
