//! Directions en volume : un vecteur unitaire, qu'on fait tourner d'angles entiers, sans fonction trigonométrique.
//!
//! En 2D, une direction est un degré entier et ses cosinus et sinus viennent des tables [`COS`] et [`SIN`] de
//! [`crate::croissance`]. En 3D, une direction est un vecteur `(x, y, z)` de norme 1. On la fait tourner de `theta`
//! degrés (l'écart à la direction de départ) dans le plan choisi par `phi` degrés (le tour autour de la direction de
//! départ), par la formule de rotation de Rodrigues écrite avec ces mêmes tables :
//!
//! `normer(COS[theta] · d + SIN[theta] · (COS[phi] · u + SIN[phi] · v))`,
//!
//! où `(u, v)` est la base fixée par [`Direction3d::base`]. Le calcul n'emploie que `+`, `-`, `*`, `/` et `sqrt` :
//! le même résultat sur toutes les machines et dans la page.
//!
//! C'est un choix, pas une référence publiée : le modèle de Meškauskas, Fricker et Moore (2004) est en 3D, mais
//! l'article ne dit pas comment il tire ses angles.

use crate::alea::Alea;
use crate::croissance::{COS, NB_DIRECTIONS, SIN};

/// Pourquoi une [`Direction3d`] est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurDirection3d {
    /// Le vecteur est nul : il n'a pas de direction.
    Nulle,
    /// Une composante n'est pas finie (NaN ou infinie).
    NonFinie,
}

/// Une direction en volume : un vecteur de norme 1, à l'arrondi près.
///
/// Contrat : chaque [`Direction3d`] sort de [`Direction3d::nouvelle`], qui divise le vecteur par sa norme ; sa norme
/// est donc `1` à quelques `f64::EPSILON` près. Tout est déterministe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction3d {
    x: f64,
    y: f64,
    z: f64,
}

impl Direction3d {
    /// La direction du vecteur `(x, y, z)` : chaque composante divisée par `sqrt(x * x + y * y + z * z)`.
    ///
    /// Contrat : refuse avec [`ErreurDirection3d::NonFinie`] si une composante n'est pas finie, puis avec
    /// [`ErreurDirection3d::Nulle`] si la norme est nulle. La norme se calcule dans cet ordre d'opérations :
    /// `(x * x + y * y) + z * z`, puis sa racine ; chaque composante est ensuite divisée par la norme.
    pub fn nouvelle(x: f64, y: f64, z: f64) -> Result<Direction3d, ErreurDirection3d> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(ErreurDirection3d::NonFinie);
        }
        let norme = ((x * x + y * y) + z * z).sqrt();
        if norme == 0.0 {
            return Err(ErreurDirection3d::Nulle);
        }
        Ok(Direction3d { x: x / norme, y: y / norme, z: z / norme })
    }

    /// La composante en `x`.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// La composante en `y`.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// La composante en `z`.
    pub fn z(&self) -> f64 {
        self.z
    }

    /// Le produit scalaire `x * a.x + y * a.y + z * a.z`, dans cet ordre : le cosinus de l'angle entre les deux.
    pub fn scalaire(&self, autre: &Direction3d) -> f64 {
        self.x * autre.x + self.y * autre.y + self.z * autre.z
    }

    /// Deux directions `(u, v)` perpendiculaires à celle-ci et entre elles : la base du tour de [`Self::tourner`].
    ///
    /// Contrat : l'axe d'appui `a` est l'axe de coordonnées où la composante est la plus petite en valeur absolue
    /// (`x` si `|x| ≤ |y|` et `|x| ≤ |z|`, sinon `y` si `|y| ≤ |z|`, sinon `z`) ; `u = nouvelle(d × a)` et
    /// `v = nouvelle(d × u)`, où `×` est le produit vectoriel
    /// `(p.y * q.z - p.z * q.y, p.z * q.x - p.x * q.z, p.x * q.y - p.y * q.x)`. Pour `d = (0, 0, 1)` :
    /// `u = (0, 1, 0)` et `v = (-1, 0, 0)`.
    pub fn base(&self) -> (Direction3d, Direction3d) {
        let (ax, ay, az) = (self.x.abs(), self.y.abs(), self.z.abs());
        let appui = if ax <= ay && ax <= az {
            (1.0, 0.0, 0.0)
        } else if ay <= az {
            (0.0, 1.0, 0.0)
        } else {
            (0.0, 0.0, 1.0)
        };
        let c = produit_vectoriel((self.x, self.y, self.z), appui);
        let u = Self::normer_ou_axe(c);
        let c = produit_vectoriel((self.x, self.y, self.z), (u.x, u.y, u.z));
        let v = Self::normer_ou_axe(c);
        (u, v)
    }

    /// Normalise `c` ; un vecteur nul ou non fini (impossible pour une direction du contrat) rend l'axe `x`.
    fn normer_ou_axe(c: (f64, f64, f64)) -> Direction3d {
        Direction3d::nouvelle(c.0, c.1, c.2).unwrap_or(Direction3d { x: 1.0, y: 0.0, z: 0.0 })
    }

    /// Cette direction tournée de `theta` degrés, dans le plan choisi par `phi` degrés autour d'elle.
    ///
    /// Contrat : avec `t = theta % 360`, `p = phi % 360` et `(u, v) = self.base()`, on forme d'abord l'axe
    /// `a = COS[p] * u + SIN[p] * v` (composante par composante), puis `c = COS[t] * d + SIN[t] * a`, et le résultat
    /// est `nouvelle(c.x, c.y, c.z)`. L'angle entre le résultat et `self` est `t` degrés (pour `t ≤ 180`) ;
    /// `theta = 0` rend `self` à l'arrondi près ; `theta = 180` rend `-self`.
    pub fn tourner(&self, theta: u32, phi: u32) -> Direction3d {
        let t = (theta % NB_DIRECTIONS as u32) as usize;
        let p = (phi % NB_DIRECTIONS as u32) as usize;
        let (u, v) = self.base();
        let ax = COS[p] * u.x + SIN[p] * v.x;
        let ay = COS[p] * u.y + SIN[p] * v.y;
        let az = COS[p] * u.z + SIN[p] * v.z;
        let cx = COS[t] * self.x + SIN[t] * ax;
        let cy = COS[t] * self.y + SIN[t] * ay;
        let cz = COS[t] * self.z + SIN[t] * az;
        Self::normer_ou_axe((cx, cy, cz))
    }

    /// Une direction tirée au hasard, uniforme sur la sphère à un degré près de longitude.
    ///
    /// Contrat : deux tirages, dans cet ordre : `z = 2.0 * alea.suivant_f64() - 1.0`, puis
    /// `p = (alea.suivant() % 360) as usize` ; avec `r = (1.0 - z * z).max(0.0).sqrt()`, le résultat est
    /// `nouvelle(r * COS[p], r * SIN[p], z)`.
    pub fn au_hasard(alea: &mut Alea) -> Direction3d {
        let z = 2.0 * alea.suivant_f64() - 1.0;
        let p = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
        let r = (1.0 - z * z).max(0.0).sqrt();
        Self::normer_ou_axe((r * COS[p], r * SIN[p], z))
    }
}

/// Le produit vectoriel `p × q`, dans l'ordre des composantes du contrat de [`Direction3d::base`].
fn produit_vectoriel(p: (f64, f64, f64), q: (f64, f64, f64)) -> (f64, f64, f64) {
    (p.1 * q.2 - p.2 * q.1, p.2 * q.0 - p.0 * q.2, p.0 * q.1 - p.1 * q.0)
}
