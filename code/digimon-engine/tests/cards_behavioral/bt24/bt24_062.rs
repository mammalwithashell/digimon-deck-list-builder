//! BT24-062 MasterBlimpmon — inherited [Your Turn] target-lock fixture.
//!
//! Printed inherited text:
//!   "[Your Turn] This Digimon's attack target can't change."
//!
//! Pinned via the raw_rust declarative `bt24_062_attack_target_lock`, which
//! installs `ModifierType::CannotSwitchAttackTarget` on the host permanent
//! every declarative tick when the host's controller is the turn player.
//! Track D's combat consult sites read the modifier in `try_enter_block`,
//! `try_enter_raid_retarget`, and `apply_attack_target_substitution`.
//!
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Black/BT24_062.cs.

use digimon_dsl::compiled::CompiledColor;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, GamePhase, Keyword, ModifierType};

fn blocker_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c
}

fn defender_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c
}

fn master_blimpmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT24-062")
        .expect("BT24-062 YAML loads")
        .add_card(blocker_card("BLK"))
        .add_card(defender_card("DEF"))
}

#[test]
fn bt24_062_yaml_compiles_with_correct_printed_face() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt24")
        .join("BT24-062.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT24-062 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT24-062 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("bt24", &[spec]).expect("BT24-062 YAML compiles");
    let compiled = registry
        .lookup("BT24-062")
        .expect("BT24-062 in registry")
        .clone();

    assert_eq!(compiled.card, "BT24-062");
    assert_eq!(compiled.name, "MasterBlimpmon");
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Black, CompiledColor::Blue]
    );
    for trait_name in ["Machine", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
    // The inherited target-lock clause must compile to a single declarative.
    assert_eq!(
        compiled.effects.len(),
        1,
        "fixture only authors the inherited target-lock clause"
    );
}

#[test]
fn bt24_062_target_lock_suppresses_block_window_on_controllers_turn() {
    // BT24-062 attacks on its controller's turn (turn 1, player 0). A
    // Blocker is up across the board. Without the inherited text the
    // engine would install BlockTiming; with the text active, the
    // declarative tick installs `CannotSwitchAttackTarget` on
    // BT24-062 and `try_enter_block` early-returns.
    let mut r = master_blimpmon_runner().memory(10).start();
    let atk = r.place_on_field(0, "BT24-062", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    // Refresh the materialized declarative — same hook the action-mask
    // build runs.
    r.game.tick_declarative_effects();

    assert!(
        r.game
            .modifiers
            .has(atk, ModifierType::CannotSwitchAttackTarget),
        "declarative tick must install CannotSwitchAttackTarget on BT24-062 during its controller's turn"
    );

    let result = r.attack_digimon(atk, r.perm_handle(1, 0), false);
    assert_ne!(
        result,
        AttackResult::InProgress,
        "Block window must be suppressed → attack resolves synchronously"
    );
    assert_ne!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "BlockTiming must not install when CannotSwitchAttackTarget is set"
    );
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt24_062_target_lock_inactive_on_opponents_turn() {
    // The inherited text reads "[Your Turn]" — when the host's
    // controller is NOT the turn player, the declarative process
    // closure short-circuits and the modifier is not installed.
    // Verifying via tick + has-check is sufficient; we don't need to
    // simulate an opponent-turn attack (which requires Counter or
    // similar mechanics).
    let mut r = master_blimpmon_runner().memory(10).start();
    let atk = r.place_on_field(0, "BT24-062", Some(0));

    // Hand the turn to the opponent.
    r.game.pass_turn();
    assert_eq!(r.game.turn_player(), 1, "expected opponent to be on turn");

    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .modifiers
            .has(atk, ModifierType::CannotSwitchAttackTarget),
        "modifier must NOT be active on the opponent's turn"
    );
}

#[test]
fn bt24_062_target_lock_does_not_affect_other_attackers() {
    // The lock only applies to BT24-062 itself (via `source_permanent`).
    // A separate attacker on the same field must still install Block
    // when there is a candidate Blocker.
    let mut r = master_blimpmon_runner()
        .add_card({
            let mut c = make_test_card("OTHER", "Other Attacker");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(5000);
            c
        })
        .memory(10)
        .start();
    let _atk_locked = r.place_on_field(0, "BT24-062", Some(0));
    let other = r.place_on_field(0, "OTHER", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.game.tick_declarative_effects();

    let result = r.attack_digimon(other, r.perm_handle(1, 0), false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "the unlocked attacker must surface BlockTiming"
    );
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
}
