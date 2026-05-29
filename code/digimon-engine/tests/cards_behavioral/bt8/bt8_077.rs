//! BT8-077 BlackGatomon
//!
//! <Rush>
//! Inherited Effect <Retaliation>

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

const YAML: &str = include_str!("../../../cards/bt8/BT8-077.yaml");

#[test]
fn bt8_077_grants_rush_while_face_up() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT8-077 YAML parses")
        .start();
    let handle = runner.place_on_field(0, "BT8-077", Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Rush));
}

#[test]
fn bt8_077_grants_retaliation_as_inherited_source() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT8-077 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["BT8-077", "CARRIER"]);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(carrier, Keyword::Retaliation));
}
