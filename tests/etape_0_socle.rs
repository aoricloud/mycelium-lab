//! Test provisoire : prouve que les signatures du socle compilent.

use mycelium_lab::empreinte::sha256_hex;

#[test]
#[cfg_attr(not(feature = "etape-0"), ignore)]
fn signatures_du_socle_compilent() {
    let _ = sha256_hex(b"");
}
