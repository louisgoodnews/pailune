//! Strict checks of the installed game's indexes and of Pailune's deployed
//! overlay (`cargo test strict_index -- --ignored --nocapture`, read-only).

use super::{pamt_check, papgt};
use std::fs;

#[test]
#[ignore = "reads the local game installation"]
fn strict_index_checks_of_vanilla_and_overlay() {
    let game = crate::steam::library::find_game(crate::config::CRIMSON_DESERT_APP_ID).unwrap();
    let live = fs::read(game.join("meta/0.papgt")).unwrap();
    let table = papgt::parse(&live).unwrap();
    println!(
        "live 0.papgt: {} groups, unknown0 {:#x}, unknown1 {:#x}",
        table.groups.len(),
        table.unknown0,
        table.unknown1
    );
    let mut bad = 0;
    for g in &table.groups {
        let dir = game.join(&g.name);
        let Ok(data) = fs::read(dir.join("0.pamt")) else {
            println!("{}: no index (optional {})", g.name, g.is_optional);
            continue;
        };
        let crc_ok = u32::from_le_bytes(data[..4].try_into().unwrap()) == g.pamt_crc;
        let found = pamt_check::problems(&data, &dir).unwrap();
        if !found.is_empty() || !crc_ok || g.name == table.groups[0].name {
            println!(
                "{}: lang {:#06x} optional {} crc matches {crc_ok}, {} problem(s) {:?}",
                g.name,
                g.lang_mask,
                g.is_optional,
                found.len(),
                &found[..found.len().min(5)]
            );
        }
        bad += usize::from(!found.is_empty() || !crc_ok);
    }
    println!("{bad} group(s) with problems");
}
