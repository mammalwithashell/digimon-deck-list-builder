//! BT21-095 Wind Guardians — security-zone-sourced filter aura fixture.
//!
//! Printed text:
//!   While you have no face-up security cards, you can ignore this card's
//!   color requirements.
//!   [Security] [All Turns] All of your [WG] trait Digimon gain ＜Vortex＞
//!     (At the end of your turn, this Digimon may attack an opponent's
//!     Digimon. With this effect, it can attack the turn it was played.).
//!   [Main] Add your bottom security card to the hand. Then, place this
//!     card face up as the bottom security card.
//! Inherited:
//!   [Security] You may play 1 level 5 or lower Digimon with the [WG]
//!     trait from your hand without paying the cost.
//!
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT21/Blue/BT21_095.cs.
//!
//! This fixture covers the [Security] [All Turns] aura half via Track H
//! §5 (security-zone-sourced auras). The other clauses are tracked
//! separately:
//!   - "ignore color requirements while no face-up security": tracked
//!     under the IgnoreColorRequirement player-scope gap.
//!   - [Main] replace-bottom-security: zone-movement primitive, separate.
//!   - [Security] play-1-WG-from-hand-without-cost: standard security
//!     trigger, separate.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};

fn wg_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits.push("WG".to_string());
    c
}

fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

#[test]
fn bt21_095_yaml_compiles_with_correct_printed_face() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt21")
        .join("BT21-095.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT21-095 YAML exists");
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(&yaml).expect("BT21-095 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("bt21", &[spec]).expect("BT21-095 YAML compiles");
    let compiled = registry
        .lookup("BT21-095")
        .expect("BT21-095 in registry")
        .clone();

    assert_eq!(compiled.card, "BT21-095");
    assert_eq!(compiled.name, "Wind Guardians");
    assert_eq!(compiled.cost, Some(2));
    assert!(
        compiled.traits.iter().any(|t| t == "WG"),
        "BT21-095 must carry the [WG] trait"
    );
    // Compile asserts: a single declarative aura clause with scope=Security.
    assert_eq!(compiled.effects.len(), 1, "fixture authors only the [Security] [All Turns] aura clause");
}

#[test]
fn bt21_095_face_up_in_security_grants_vortex_to_owner_wg_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-095")
        .expect("BT21-095 in embedded DSL pack")
        .add_card(wg_digimon("TEST-WG-A"))
        .add_card(wg_digimon("TEST-WG-B"))
        .add_card(plain_digimon("TEST-NON-WG"))
        .security(0, &["BT21-095"])
        .build();

    // The card's [Main] effect would normally place itself face-up;
    // here we go straight to the resulting state for the unit test.
    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    let wg_a = runner.place_on_field(0, "TEST-WG-A", None);
    let wg_b = runner.place_on_field(0, "TEST-WG-B", None);
    let non_wg = runner.place_on_field(0, "TEST-NON-WG", None);

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(wg_a, Keyword::Vortex),
        "[WG] ally A must gain Vortex from BT21-095 in security"
    );
    assert!(
        runner.game.has_keyword(wg_b, Keyword::Vortex),
        "[WG] ally B must gain Vortex"
    );
    assert!(
        !runner.game.has_keyword(non_wg, Keyword::Vortex),
        "non-[WG] ally must NOT receive the keyword grant (filter excludes)"
    );
}

#[test]
fn bt21_095_face_down_in_security_does_not_grant_vortex() {
    // While the card is face-down (the default for security cards
    // until revealed), the [Security] [All Turns] aura must NOT fire.
    // DCGO reference: BT21_095.cs `CanUseCondition` checks
    // `IsExistInSecurity(card, false)` where the second arg flips the
    // face-up/down test.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-095")
        .expect("BT21-095 in embedded DSL pack")
        .add_card(wg_digimon("TEST-WG-FD"))
        .security(0, &["BT21-095"])
        .build();

    // Deliberately do NOT add to face_up_security.
    let wg = runner.place_on_field(0, "TEST-WG-FD", None);

    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(wg, Keyword::Vortex),
        "face-down security source must NOT fire the [Security] aura"
    );
}

#[test]
fn bt21_095_aura_evicts_when_card_leaves_security() {
    // When BT21-095 is removed from the security stack (security
    // check + trash, Recovery, security-add-then-trash, etc.), the
    // [WG]+Vortex grant must evict on the next tick. The
    // materialized-declarative clear+re-install model handles this
    // automatically — no explicit OnLoseSecurity wiring needed.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-095")
        .expect("BT21-095 in embedded DSL pack")
        .add_card(wg_digimon("TEST-WG-EVICT"))
        .security(0, &["BT21-095"])
        .build();

    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);
    let wg = runner.place_on_field(0, "TEST-WG-EVICT", None);

    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(wg, Keyword::Vortex));

    // Source leaves security.
    runner.game.players[0].security.clear();
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(wg, Keyword::Vortex),
        "Vortex grant must evict on next tick after BT21-095 leaves security"
    );
}
