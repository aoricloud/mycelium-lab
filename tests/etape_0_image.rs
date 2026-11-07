//! Test provisoire : prouve que les signatures de l'image compilent.

use mycelium_lab::image::crc32;

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn signatures_de_l_image_compilent() {
    let _ = crc32(b"");
}
