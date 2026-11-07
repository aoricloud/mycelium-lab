//! Générateur pseudo-aléatoire déterministe, écrit dans le dépôt : la même graine donne toujours la même suite,
//! sur tous les systèmes.

/// Générateur pseudo-aléatoire déterministe, construit depuis une graine `u64`.
#[derive(Clone, Debug)]
pub struct Alea {
    #[allow(dead_code)]
    etat: u64,
}

impl Alea {
    /// Construit le générateur depuis une graine.
    #[allow(unused_variables)]
    pub fn new(graine: u64) -> Alea {
        todo!()
    }

    /// Tire le `u64` suivant de la suite.
    pub fn suivant(&mut self) -> u64 {
        todo!()
    }

    /// Tire un `f64` dans `[0, 1)`.
    pub fn suivant_f64(&mut self) -> f64 {
        todo!()
    }
}
