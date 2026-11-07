//! Tests d'acceptation du module `image` : sommes de contrôle et écrivain PNG relu octet par octet.

use mycelium_lab::image::{adler32, crc32, ecrire_png};

const SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// CRC-32 de référence, calculé bit à bit dans le test.
fn crc_reference(octets: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &o in octets {
        crc ^= u32::from(o);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Adler-32 de référence, calculé dans le test.
fn adler_reference(octets: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &o in octets {
        a = (a + u32::from(o)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn u32_be(octets: &[u8]) -> u32 {
    u32::from_be_bytes([octets[0], octets[1], octets[2], octets[3]])
}

/// Un bloc PNG relu : type, données, et si le CRC écrit est juste.
struct Bloc {
    type_: [u8; 4],
    donnees: Vec<u8>,
    crc_juste: bool,
}

/// Découpe un PNG en blocs ; la longueur doit coller exactement jusqu'à la fin du fichier.
fn lire_blocs(png: &[u8]) -> Vec<Bloc> {
    assert!(png.len() >= 8, "PNG trop court");
    assert_eq!(png[..8], SIGNATURE, "signature PNG");
    let mut blocs = Vec::new();
    let mut pos = 8;
    while pos < png.len() {
        assert!(pos + 12 <= png.len(), "bloc tronqué");
        let longueur = u32_be(&png[pos..]) as usize;
        assert!(pos + 12 + longueur <= png.len(), "longueur de bloc trop grande");
        let type_: [u8; 4] = png[pos + 4..pos + 8].try_into().unwrap();
        let donnees = png[pos + 8..pos + 8 + longueur].to_vec();
        let crc_ecrit = u32_be(&png[pos + 8 + longueur..]);
        let crc_juste = crc_ecrit == crc_reference(&png[pos + 4..pos + 8 + longueur]);
        blocs.push(Bloc { type_, donnees, crc_juste });
        pos += 12 + longueur;
    }
    assert_eq!(pos, png.len());
    blocs
}

/// Décode un flux zlib en blocs stockés ; vérifie l'en-tête et l'Adler32, renvoie le flux brut.
fn lire_zlib_stocke(flux: &[u8]) -> Vec<u8> {
    assert!(flux.len() >= 2 + 4, "flux zlib trop court");
    let (cmf, flg) = (flux[0], flux[1]);
    assert_eq!(cmf & 0x0F, 8, "méthode de compression deflate");
    assert!(cmf >> 4 <= 7, "fenêtre valide");
    assert_eq!((u32::from(cmf) * 256 + u32::from(flg)) % 31, 0, "FCHECK");
    assert_eq!(flg & 0x20, 0, "pas de dictionnaire");
    let fin_donnees = flux.len() - 4;
    let mut pos = 2;
    let mut brut = Vec::new();
    loop {
        assert!(pos + 5 <= fin_donnees, "bloc stocké tronqué");
        let entete = flux[pos];
        assert_eq!(entete & 0b110, 0, "bloc stocké (BTYPE = 00)");
        let len = usize::from(u16::from_le_bytes([flux[pos + 1], flux[pos + 2]]));
        let nlen = u16::from_le_bytes([flux[pos + 3], flux[pos + 4]]);
        assert_eq!(nlen, !(len as u16), "NLEN complément de LEN");
        pos += 5;
        assert!(pos + len <= fin_donnees, "données stockées tronquées");
        brut.extend_from_slice(&flux[pos..pos + len]);
        pos += len;
        if entete & 1 == 1 {
            break;
        }
    }
    assert_eq!(pos, fin_donnees, "rien entre le dernier bloc et l'Adler32");
    assert_eq!(u32_be(&flux[fin_donnees..]), adler_reference(&brut), "Adler32 du flux brut");
    brut
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn crc32_de_123456789() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn crc32_du_vide() {
    assert_eq!(crc32(b""), 0);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn adler32_de_wikipedia() {
    assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn adler32_du_vide() {
    assert_eq!(adler32(b""), 1);
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn png_de_deux_par_deux_relu_octet_par_octet() {
    let pixels: [u8; 12] = [255, 0, 0, 0, 255, 0, 0, 0, 255, 10, 20, 30];
    let png = ecrire_png(2, 2, &pixels);
    let blocs = lire_blocs(&png);

    let types: Vec<&[u8; 4]> = blocs.iter().map(|b| &b.type_).collect();
    assert_eq!(types, [b"IHDR", b"IDAT", b"IEND"]);
    assert!(blocs.iter().all(|b| b.crc_juste), "CRC32 de chaque bloc");

    let ihdr = &blocs[0].donnees;
    assert_eq!(ihdr.len(), 13);
    assert_eq!(u32_be(&ihdr[0..4]), 2, "largeur");
    assert_eq!(u32_be(&ihdr[4..8]), 2, "hauteur");
    assert_eq!(ihdr[8], 8, "profondeur");
    assert_eq!(ihdr[9], 2, "type de couleur : RGB");
    assert_eq!(ihdr[10..13], [0, 0, 0], "compression, filtre, entrelacement");

    assert_eq!(blocs[2].donnees.len(), 0, "IEND vide");

    let brut = lire_zlib_stocke(&blocs[1].donnees);
    assert_eq!(brut.len(), 2 * (1 + 6), "deux lignes de filtre + 6 octets");
    let mut relus = Vec::new();
    for ligne in brut.chunks(7) {
        assert_eq!(ligne[0], 0, "octet de filtre 0");
        relus.extend_from_slice(&ligne[1..]);
    }
    assert_eq!(relus, pixels, "pixels relus égaux aux pixels écrits");
}

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn png_plus_grand_que_un_bloc_stocke() {
    // 300 × 100 × 3 = 90 000 octets de pixels : dépasse 65 535, donc plusieurs blocs stockés.
    let (l, h) = (300u32, 100u32);
    let pixels: Vec<u8> = (0..l * h * 3).map(|i| (i % 251) as u8).collect();
    let png = ecrire_png(l, h, &pixels);
    let blocs = lire_blocs(&png);
    assert!(blocs.iter().all(|b| b.crc_juste));
    assert_eq!(u32_be(&blocs[0].donnees[0..4]), l);
    assert_eq!(u32_be(&blocs[0].donnees[4..8]), h);
    let brut = lire_zlib_stocke(&blocs[1].donnees);
    let ligne = 1 + 3 * l as usize;
    assert_eq!(brut.len(), ligne * h as usize);
    let mut relus = Vec::new();
    for rangee in brut.chunks(ligne) {
        assert_eq!(rangee[0], 0);
        relus.extend_from_slice(&rangee[1..]);
    }
    assert_eq!(relus, pixels);
}
