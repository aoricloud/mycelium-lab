//! Générateur pseudo-aléatoire déterministe, écrit dans le dépôt : la même graine donne toujours la même suite,
//! sur tous les systèmes.

/// Générateur pseudo-aléatoire déterministe, construit depuis une graine `u64`.
#[derive(Clone, Debug)]
pub struct Alea {
    etat: u64,
}

impl Alea {
    /// Construit le générateur depuis une graine.
    pub fn new(graine: u64) -> Alea {
        Alea { etat: graine }
    }

    /// Tire le `u64` suivant de la suite.
    pub fn suivant(&mut self) -> u64 {
        // splitmix64
        self.etat = self.etat.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.etat;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Tire un `f64` dans `[0, 1)`.
    pub fn suivant_f64(&mut self) -> f64 {
        // 53 bits de poids fort, divisés par 2^53 (exact).
        (self.suivant() >> 11) as f64 / 9_007_199_254_740_992.0
    }
}
