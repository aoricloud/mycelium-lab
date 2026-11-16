//! Substrat en volume : diffusion explicite à sept points, avec des bords sans flux.
//!
//! Le substrat est un champ de nutriments `c(x, y, z)` sur un pavé de `largeur × hauteur × profondeur` cases,
//! de côté `dx`. Il suit l'équation de diffusion `∂c/∂t = D ∇²c`, intégrée par le schéma explicite à sept points
//! (Euler avant en temps, différences centrées en espace). La case `(x, y, z)` est à l'indice
//! `(z * hauteur + y) * largeur + x` (`z` lent, `x` rapide).
//!
//! Un pas lit l'ancien champ en entier, puis calcule, pour chaque case libre, dans cet ordre d'opérations :
//!
//! `c + r · (gauche + droite + haut + bas - 4.0 · c + (devant - c) + (derriere - c))`,
//!
//! avec `r = nombre_de_diffusion() = D dt / dx²`, `gauche` et `droite` en `x - 1` et `x + 1`, `haut` et `bas` en
//! `y - 1` et `y + 1`, `devant` et `derriere` en `z - 1` et `z + 1`. Un voisin hors du volume ou bloqué est
//! remplacé par `c` (aucun flux) ; une case bloquée reste à `0.0`.
//!
//! Cet ordre d'opérations donne, au bit près, le schéma à cinq points de [`crate::substrat`] quand `devant` et
//! `derriere` valent `c` : les deux termes `(devant - c)` et `(derriere - c)` sont alors nuls.
//!
//! Le schéma est stable si `6 r ≤ 1` ; le module refuse tout paramètre qui ne l'est pas, à la construction.
//! Le calcul n'utilise que `+`, `-`, `*`, `/` : il est déterministe d'une machine à l'autre. La masse totale se
//! somme dans l'ordre des indices, et l'écriture canonique suit ce même ordre.

/// Pourquoi une construction de [`Substrat3d`] est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurSubstrat3d {
    /// La largeur, la hauteur ou la profondeur est nulle.
    DimensionNulle,
    /// `D`, `dt` ou `dx` n'est pas fini, ou `D < 0`, ou `dt ≤ 0`, ou `dx ≤ 0`.
    ParametreInvalide,
    /// `6 D dt / dx²` dépasse `1` : le schéma explicite serait instable.
    PasInstable,
}

/// Un champ de nutriments sur un pavé, et ses paramètres de diffusion.
///
/// Contrat : les valeurs sont des `f64` rangés par plans (`z` en indice lent, puis `y`, `x` en indice rapide), à
/// l'indice `(z * hauteur + y) * largeur + x`. Un champ neuf est nul partout. Les coordonnées `(x, y, z)` vont de
/// `0` à `largeur - 1`, `0` à `hauteur - 1` et `0` à `profondeur - 1`. Tout est déterministe : mêmes paramètres,
/// mêmes dépôts, même nombre de pas, mêmes octets.
#[derive(Debug, Clone, PartialEq)]
pub struct Substrat3d {
    largeur: usize,
    hauteur: usize,
    profondeur: usize,
    d: f64,
    dt: f64,
    dx: f64,
    cases: Vec<f64>,
    bloquees: Vec<bool>,
}

impl Substrat3d {
    /// Construit un champ nul de `largeur × hauteur × profondeur` cases, de coefficient de diffusion `d`, de pas
    /// de temps `dt` et de côté de case `dx` (`d` en longueur²/temps, `dt` en temps, `dx` en longueur).
    ///
    /// Contrat : refuse avec [`ErreurSubstrat3d::DimensionNulle`] si une dimension est nulle ; avec
    /// [`ErreurSubstrat3d::ParametreInvalide`] si `d`, `dt` ou `dx` n'est pas fini, si `d < 0`, `dt ≤ 0` ou
    /// `dx ≤ 0` (`d = 0` est permis : le champ ne bouge pas) ; avec [`ErreurSubstrat3d::PasInstable`] si
    /// `6 * (d * dt / (dx * dx)) > 1.0`. La limite elle-même est acceptée.
    pub fn nouveau(
        largeur: usize,
        hauteur: usize,
        profondeur: usize,
        d: f64,
        dt: f64,
        dx: f64,
    ) -> Result<Substrat3d, ErreurSubstrat3d> {
        if largeur == 0 || hauteur == 0 || profondeur == 0 {
            return Err(ErreurSubstrat3d::DimensionNulle);
        }
        if !d.is_finite() || !dt.is_finite() || !dx.is_finite() || d < 0.0 || dt <= 0.0 || dx <= 0.0 {
            return Err(ErreurSubstrat3d::ParametreInvalide);
        }
        if 6.0 * (d * dt / (dx * dx)) > 1.0 {
            return Err(ErreurSubstrat3d::PasInstable);
        }
        Ok(Substrat3d {
            largeur,
            hauteur,
            profondeur,
            d,
            dt,
            dx,
            cases: vec![0.0; largeur * hauteur * profondeur],
            bloquees: vec![false; largeur * hauteur * profondeur],
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

    /// Nombre de cases en `z`.
    pub fn profondeur(&self) -> usize {
        self.profondeur
    }

    /// Le nombre de diffusion `r = D dt / dx²`, toujours dans `[0, 1/6]` pour un substrat construit.
    pub fn nombre_de_diffusion(&self) -> f64 {
        self.d * self.dt / (self.dx * self.dx)
    }

    /// La quantité de nutriments dans la case `(x, y, z)`.
    ///
    /// Contrat : panique si `x >= largeur()`, `y >= hauteur()` ou `z >= profondeur()`.
    pub fn valeur(&self, x: usize, y: usize, z: usize) -> f64 {
        assert!(x < self.largeur && y < self.hauteur && z < self.profondeur, "case ({x}, {y}, {z}) hors du volume");
        let i = (z * self.hauteur + y) * self.largeur + x;
        if self.bloquees[i] { 0.0 } else { self.cases[i] }
    }

    /// Ajoute `quantite` à la case `(x, y, z)` (la masse totale croît d'autant).
    ///
    /// Contrat : panique si la case est hors du volume, ou si `quantite` n'est pas finie ou est négative, ou si la
    /// case est bloquée (le message contient « bloquée »).
    pub fn deposer(&mut self, x: usize, y: usize, z: usize, quantite: f64) {
        assert!(x < self.largeur && y < self.hauteur && z < self.profondeur, "case ({x}, {y}, {z}) hors du volume");
        assert!(quantite.is_finite() && quantite >= 0.0, "quantité invalide : {quantite}");
        let i = (z * self.hauteur + y) * self.largeur + x;
        assert!(!self.bloquees[i], "case ({x}, {y}, {z}) bloquée : dépôt refusé");
        self.cases[i] += quantite;
    }

    /// Retire au plus `quantite` de la case `(x, y, z)` et rend ce qui a été retiré.
    ///
    /// Contrat : le résultat n'excède jamais ce que la case contenait, et la masse totale baisse d'autant. Une
    /// case bloquée rend `0.0`. Panique si la case est hors du volume, ou si `quantite` n'est pas finie ou est
    /// négative.
    pub fn retirer(&mut self, x: usize, y: usize, z: usize, quantite: f64) -> f64 {
        assert!(x < self.largeur && y < self.hauteur && z < self.profondeur, "case ({x}, {y}, {z}) hors du volume");
        assert!(quantite.is_finite() && quantite >= 0.0, "quantité invalide : {quantite}");
        let i = (z * self.hauteur + y) * self.largeur + x;
        if self.bloquees[i] {
            return 0.0;
        }
        let retire = quantite.min(self.cases[i]);
        self.cases[i] -= retire;
        retire
    }

    /// Bloque le pavé de cases `x0..=x1` × `y0..=y1` × `z0..=z1` : ces cases valent `0.0` pour toujours, ne
    /// reçoivent rien et ne donnent rien. Leur quantité du moment est retirée (la masse totale baisse d'autant).
    ///
    /// Contrat : panique si `x0 > x1`, `y0 > y1`, `z0 > z1` ou si une borne sort du volume. Bloquer deux fois la
    /// même case est permis. Ensuite, [`Substrat3d::pas`] traite un voisin bloqué comme un bord (aucun flux), une
    /// case bloquée reste à `0.0`, [`Substrat3d::valeur`] et [`Substrat3d::retirer`] y rendent `0.0`, et
    /// [`Substrat3d::deposer`] y panique (le message contient « bloquée »).
    pub fn bloquer(&mut self, x0: usize, y0: usize, z0: usize, x1: usize, y1: usize, z1: usize) {
        assert!(x0 <= x1 && y0 <= y1 && z0 <= z1, "pavé invalide : ({x0}, {y0}, {z0}) à ({x1}, {y1}, {z1})");
        assert!(
            x1 < self.largeur && y1 < self.hauteur && z1 < self.profondeur,
            "pavé ({x0}, {y0}, {z0}) à ({x1}, {y1}, {z1}) hors du volume {}×{}×{}",
            self.largeur,
            self.hauteur,
            self.profondeur
        );
        for z in z0..=z1 {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let i = (z * self.hauteur + y) * self.largeur + x;
                    self.cases[i] = 0.0;
                    self.bloquees[i] = true;
                }
            }
        }
    }

    /// Vrai si la case `(x, y, z)` est bloquée.
    ///
    /// Contrat : panique si la case est hors du volume.
    pub fn est_bloquee(&self, x: usize, y: usize, z: usize) -> bool {
        assert!(x < self.largeur && y < self.hauteur && z < self.profondeur, "case ({x}, {y}, {z}) hors du volume");
        self.bloquees[(z * self.hauteur + y) * self.largeur + x]
    }

    /// Avance le champ d'un pas de temps `dt` par le schéma à sept points, bords sans flux.
    ///
    /// Contrat : la masse totale est conservée (aux erreurs d'arrondi près) ; l'ancien champ est lu en entier
    /// avant d'écrire le nouveau ; l'ordre de calcul est fixe.
    pub fn pas(&mut self) {
        let ancien = self.cases.clone();
        let (largeur, hauteur, profondeur) = (self.largeur, self.hauteur, self.profondeur);
        let plan = largeur * hauteur;
        let r = self.nombre_de_diffusion();
        for z in 0..profondeur {
            for y in 0..hauteur {
                for x in 0..largeur {
                    let i = (z * hauteur + y) * largeur + x;
                    if self.bloquees[i] {
                        self.cases[i] = 0.0;
                        continue;
                    }
                    let c = ancien[i];
                    let gauche = if x == 0 || self.bloquees[i - 1] { c } else { ancien[i - 1] };
                    let droite = if x + 1 == largeur || self.bloquees[i + 1] { c } else { ancien[i + 1] };
                    let haut = if y == 0 || self.bloquees[i - largeur] { c } else { ancien[i - largeur] };
                    let bas = if y + 1 == hauteur || self.bloquees[i + largeur] { c } else { ancien[i + largeur] };
                    let devant = if z == 0 || self.bloquees[i - plan] { c } else { ancien[i - plan] };
                    let derriere = if z + 1 == profondeur || self.bloquees[i + plan] { c } else { ancien[i + plan] };
                    self.cases[i] = c + r * (gauche + droite + haut + bas - 4.0 * c + (devant - c) + (derriere - c));
                }
            }
        }
    }

    /// Avance le champ de `n` pas : strictement équivalent à appeler [`Substrat3d::pas`] `n` fois, au même
    /// octet près. `n = 0` ne change rien.
    pub fn pas_n(&mut self, n: usize) {
        for _ in 0..n {
            self.pas();
        }
    }

    /// La somme des valeurs de toutes les cases, additionnée dans l'ordre des indices (`z` lent, puis `y`, puis
    /// `x`).
    pub fn masse_totale(&self) -> f64 {
        let mut somme = 0.0;
        for &v in &self.cases {
            somme += v;
        }
        somme
    }

    /// L'écriture canonique du champ, pour comparer « à l'octet » et pour l'empreinte.
    ///
    /// Format : `largeur`, `hauteur` puis `profondeur`, chacun en `u64` petit-boutiste (8 octets), puis la valeur
    /// de chaque case, dans l'ordre des indices (`z` lent, puis `y`, puis `x`), en `f64::to_bits`
    /// petit-boutiste (8 octets). Les paramètres `d`, `dt` et `dx` n'en font pas partie. Longueur :
    /// `24 + 8 * largeur * hauteur * profondeur` octets.
    pub fn ecrire_canonique(&self) -> Vec<u8> {
        let mut octets = Vec::with_capacity(24 + 8 * self.cases.len());
        octets.extend_from_slice(&(self.largeur as u64).to_le_bytes());
        octets.extend_from_slice(&(self.hauteur as u64).to_le_bytes());
        octets.extend_from_slice(&(self.profondeur as u64).to_le_bytes());
        for &v in &self.cases {
            octets.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        octets
    }
}
