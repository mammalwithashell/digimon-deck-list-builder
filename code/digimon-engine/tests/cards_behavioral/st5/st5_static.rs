//! ST5 simple/static cards — vanilla Digimon, printed/inherited Blocker, and
//! Kapurimon's inherited DP aura.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

#[test]
fn st5_vanilla_digimon_compile_from_yaml() {
    let runner = DebugRunner::builder()
        .dsl_card("ST5-02")
        .expect("ST5-02 YAML parses")
        .dsl_card("ST5-05")
        .expect("ST5-05 YAML parses")
        .dsl_card("ST5-07")
        .expect("ST5-07 YAML parses")
        .dsl_card("ST5-10")
        .expect("ST5-10 YAML parses")
        .build();

    for card_id in ["ST5-02", "ST5-05", "ST5-07", "ST5-10"] {
        let card = runner.compiled_card(card_id).expect("compiled card exists");
        assert!(
            card.effects.is_empty(),
            "{card_id} is vanilla and should not compile gameplay effects"
        );
    }
}

#[test]
fn st5_03_agumon_has_printed_blocker() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-03")
        .expect("ST5-03 YAML parses")
        .build();
    let agumon = runner.place_on_field(0, "ST5-03", Some(0));

    assert!(runner.game.has_keyword(agumon, Keyword::Blocker));
}

#[test]
fn st5_08_darktyrannomon_has_blocker_and_loses_two_memory_when_attacking() {
    let mut defender = make_test_card("ST5-STATIC-DEF", "ST5 Static Defender");
    defender.dp = Some(9000);
    defender.level = Some(3);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-08")
        .expect("ST5-08 YAML parses")
        .add_card(defender)
        .memory(5)
        .start();
    let darktyrannomon = runner.place_on_field(0, "ST5-08", Some(0));
    let defender = runner.place_on_field(1, "ST5-STATIC-DEF", Some(0));

    assert!(runner.game.has_keyword(darktyrannomon, Keyword::Blocker));
    let memory_before = runner.memory();
    let _ = runner.attack_digimon(darktyrannomon, defender, false);
    runner.auto_resolve().expect("attack resolves");

    assert_eq!(
        runner.memory(),
        memory_before - 2,
        "ST5-08 [When Attacking] should lose 2 memory"
    );
}

#[test]
fn st5_11_megadramon_grants_inherited_blocker_to_carrier() {
    let mut host = make_test_card("ST5-STATIC-HOST", "ST5 Static Host");
    host.dp = Some(9000);
    host.level = Some(6);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-11")
        .expect("ST5-11 YAML parses")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["ST5-11", "ST5-STATIC-HOST"]);

    assert!(runner.game.has_keyword(carrier, Keyword::Blocker));
}

#[test]
fn st5_01_kapurimon_buffs_carrier_with_blocker_only_on_your_turn() {
    let mut blocker_host = make_test_card("ST5-BLOCKER-HOST", "ST5 Blocker Host");
    blocker_host.dp = Some(5000);
    blocker_host.level = Some(3);
    let mut plain_host = make_test_card("ST5-PLAIN-HOST", "ST5 Plain Host");
    plain_host.dp = Some(5000);
    plain_host.level = Some(3);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-01")
        .expect("ST5-01 YAML parses")
        .dsl_card("ST5-03")
        .expect("ST5-03 YAML parses")
        .add_card(blocker_host)
        .add_card(plain_host)
        .build();
    let blocker = runner.place_stack(0, &["ST5-01", "ST5-03"]);
    let plain = runner.place_stack(0, &["ST5-01", "ST5-PLAIN-HOST"]);

    assert_eq!(
        runner.game.source_dp_contribution(blocker, 0),
        1000,
        "ST5-01 should add +1000 DP to a carrier with Blocker on your turn"
    );
    assert_eq!(
        runner.game.source_dp_contribution(plain, 0),
        0,
        "ST5-01 should not buff a carrier without Blocker"
    );

    runner.end_turn();
    assert_eq!(
        runner.game.source_dp_contribution(blocker, 0),
        0,
        "ST5-01 buff is [Your Turn] only and should turn off on opponent's turn"
    );
}
