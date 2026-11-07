//! Écrivain PNG sans dépendance, et les sommes de contrôle qu'il utilise.
//!
//! Le flux zlib est en blocs stockés (non compressés) pour que les octets des pixels se relisent tels quels.

/// CRC-32 (polynôme PNG/zlib) de `octets`.
#[allow(unused_variables)]
pub fn crc32(octets: &[u8]) -> u32 {
    todo!()
}

/// Somme Adler-32 de `octets`.
#[allow(unused_variables)]
pub fn adler32(octets: &[u8]) -> u32 {
    todo!()
}

/// Écrit un PNG 8 bits en RGB : `pixels_rgb` contient `largeur * hauteur * 3` octets, ligne par ligne.
#[allow(unused_variables)]
pub fn ecrire_png(largeur: u32, hauteur: u32, pixels_rgb: &[u8]) -> Vec<u8> {
    todo!()
}
