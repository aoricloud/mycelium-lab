//! Écrivain PNG sans dépendance, et les sommes de contrôle qu'il utilise.
//!
//! Le flux zlib est en blocs stockés (non compressés) pour que les octets des pixels se relisent tels quels.

/// CRC-32 (polynôme PNG/zlib) de `octets`.
pub fn crc32(octets: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &o in octets {
        crc ^= u32::from(o);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Somme Adler-32 de `octets`.
pub fn adler32(octets: &[u8]) -> u32 {
    const MODULO: u32 = 65_521;
    let (mut a, mut b) = (1u32, 0u32);
    // 5552 octets au plus entre deux réductions : pas de débordement de u32.
    for morceau in octets.chunks(5552) {
        for &o in morceau {
            a += u32::from(o);
            b += a;
        }
        a %= MODULO;
        b %= MODULO;
    }
    (b << 16) | a
}

/// Ajoute un bloc PNG (longueur, type, données, CRC) à `sortie`.
fn ajouter_bloc(sortie: &mut Vec<u8>, type_: &[u8; 4], donnees: &[u8]) {
    sortie.extend_from_slice(&(donnees.len() as u32).to_be_bytes());
    let debut = sortie.len();
    sortie.extend_from_slice(type_);
    sortie.extend_from_slice(donnees);
    let crc = crc32(&sortie[debut..]);
    sortie.extend_from_slice(&crc.to_be_bytes());
}

/// Écrit un PNG 8 bits en RGB : `pixels_rgb` contient `largeur * hauteur * 3` octets, ligne par ligne.
pub fn ecrire_png(largeur: u32, hauteur: u32, pixels_rgb: &[u8]) -> Vec<u8> {
    let taille_ligne = largeur as usize * 3;
    assert_eq!(pixels_rgb.len(), taille_ligne * hauteur as usize, "taille des pixels incohérente");

    // Flux brut : un octet de filtre 0 devant chaque ligne.
    let mut brut = Vec::with_capacity(pixels_rgb.len() + hauteur as usize);
    if taille_ligne > 0 {
        for ligne in pixels_rgb.chunks(taille_ligne) {
            brut.push(0);
            brut.extend_from_slice(ligne);
        }
    } else {
        brut.resize(hauteur as usize, 0);
    }

    // Flux zlib en blocs stockés.
    let mut zlib = vec![0x78, 0x01];
    if brut.is_empty() {
        zlib.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    } else {
        let mut reste = brut.as_slice();
        while !reste.is_empty() {
            let n = reste.len().min(65_535);
            let (bloc, suite) = reste.split_at(n);
            zlib.push(u8::from(suite.is_empty()));
            zlib.extend_from_slice(&(n as u16).to_le_bytes());
            zlib.extend_from_slice(&(!(n as u16)).to_le_bytes());
            zlib.extend_from_slice(bloc);
            reste = suite;
        }
    }
    zlib.extend_from_slice(&adler32(&brut).to_be_bytes());

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&largeur.to_be_bytes());
    ihdr.extend_from_slice(&hauteur.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);

    let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    ajouter_bloc(&mut png, b"IHDR", &ihdr);
    ajouter_bloc(&mut png, b"IDAT", &zlib);
    ajouter_bloc(&mut png, b"IEND", &[]);
    png
}
