//! ST19-07 Tobucatmon.
//! Printed text covered here: <Jamming>. Inherited: <Barrier>.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

#[test]
fn st19_07_has_jamming_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-07")
        .expect("ST19-07 YAML loads")
        .start();
    let tobucat = runner.place_on_field(0, "ST19-07", Some(0));

    assert!(
        runner.game.has_keyword(tobucat, Keyword::Jamming),
        "ST19-07 has printed Jamming"
    );
}

#[test]
fn st19_07_inherited_barrier_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-07")
        .expect("ST19-07 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["ST19-07", "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from ST19-07"
    );
}
