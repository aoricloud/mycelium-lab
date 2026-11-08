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
