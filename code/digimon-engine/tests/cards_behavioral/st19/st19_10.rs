//! ST19-10 ExTyrannomon.
//! Printed text covered here: <Armor Purge>. Inherited: <Barrier>.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

#[test]
fn st19_10_has_armor_purge_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-10")
        .expect("ST19-10 YAML loads")
        .start();
    let extyranno = runner.place_on_field(0, "ST19-10", Some(0));

    assert!(
        runner.game.has_keyword(extyranno, Keyword::ArmorPurge),
        "ST19-10 has printed Armor Purge"
    );
}

#[test]
fn st19_10_inherited_barrier_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-10")
        .expect("ST19-10 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["ST19-10", "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from ST19-10"
    );
}
