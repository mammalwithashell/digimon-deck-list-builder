//! BT21-072 Arresterdramon: Superior Mode — Digimon Lv.5, DP10000, Cost9, Red/Purple.
//!
//! # Card text (cards.json)
//!
//! Effect:
//! <Raid> (When this Digimon attacks, you may switch the target of attack to
//! 1 of your opponent's unsuspended Digimon with the highest DP.)
//! <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and
//! survives the battle, it performs any security checks it normally would.)
//! [When Digivolving] This Digimon may attack without suspending.
//! [All Turns] This Digimon gets +1000 DP for each of its digivolution cards.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_072.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H3: Piercing keyword grant (declarative)
//! - H9: Raid keyword grant (declarative)
//! - D4: Inherited self-aura [Your Turn] +2000 DP
//! - [When Digivolving] may attack without suspending
//! - BLOCKED (G-AURA-DP-FORMULA): [All Turns] +1000 DP per digivolution card
//!   (AuraBody.dp_modifier is Option<i32>; no formula/dynamic variant exists)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (1) <Raid> | none | PASS — structural + has_keyword |
//! | (2) <Piercing> | none | PASS — structural + has_keyword |
//! | (3) [When Digivolving] may attack without suspending | none | PASS — may_attack_now optional/without_suspending |
//! | (4) [All Turns] +1000 DP per digivolution card | G-AURA-DP-FORMULA | BLOCKED — #[ignore] |
//! | (5) Inherited [Your Turn] +2000 DP | G-INHERITED-DISPATCH (triggered only, declarative may be ok) | PASS — structural |

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::TriggerSource;

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

// Production YAML loaded by reference (not inlined) — tests drive through dsl_card.

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn arresterdramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-072")
        .expect("BT21-072 in embedded DSL pack")
        .memory(9)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT21-072 must have two declarative GrantKeyword clauses (Raid, Piercing),
/// one declarative inherited Aura clause ([Your Turn] +2000 DP), and one
/// triggered [When Digivolving] may-attack clause.
#[test]
fn bt21_072_structural_three_declarative_clauses_and_one_triggered() {
    let runner = arresterdramon_runner();
    let compiled = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let declaratives: Vec<&CompiledDeclarativeClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();

    assert_eq!(
        declaratives.len(),
        3,
        "Expected 2 GrantKeyword + 1 Aura (inherited) = 3 declaratives"
    );

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 1,
        "Expected one [When Digivolving] may_attack_now triggered clause"
    );
}

/// The two GrantKeyword declaratives must be Raid and Piercing.
#[test]
fn bt21_072_structural_raid_and_piercing_grant_keyword_clauses() {
    let runner = arresterdramon_runner();
    let compiled = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let kw_names: Vec<&str> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(
        kw_names.len(),
        2,
        "Expected exactly 2 GrantKeyword declaratives (Raid + Piercing)"
    );
    assert!(
        kw_names.contains(&"Raid"),
        "Raid GrantKeyword clause must be present"
    );
    assert!(
        kw_names.contains(&"Piercing"),
        "Piercing GrantKeyword clause must be present"
    );
}

/// The inherited Aura must be scope=Inherited, active_when=your_turn, dp_modifier=2000.
#[test]
fn bt21_072_structural_inherited_aura_your_turn_dp_2000() {
    let runner = arresterdramon_runner();
    let compiled = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            active_when,
            ..
        }) => Some((scope.clone(), *dp_modifier, active_when.is_some())),
        _ => None,
    });

    let (scope, dp, has_active_when) = aura.expect("Expected an Aura declarative clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "Aura must be Inherited scope"
    );
    assert_eq!(
        dp,
        Some(2000),
        "Inherited aura must carry +2000 dp_modifier"
    );
    assert!(
        has_active_when,
        "Inherited aura must have active_when (your_turn gate)"
    );
}

/// The [When Digivolving] triggered clause must lower to an optional
/// may_attack_now step that skips the suspend cost.
#[test]
fn bt21_072_when_digivolving_clause_contains_unsuspended_may_attack_now() {
    let runner = arresterdramon_runner();
    let compiled = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::FaceUp)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT21-072 must have a face-up [When Digivolving] clause");

    let may_attack = triggered.process.iter().find_map(|s| match s {
        CompiledStep::MayAttackNow {
            without_suspending,
            optional,
            ..
        } => Some((*without_suspending, *optional)),
        _ => None,
    });
    assert_eq!(
        may_attack,
        Some((true, true)),
        "[When Digivolving] must contain optional may_attack_now with without_suspending=true"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Keyword behavioral tests (Raid + Piercing)
// ═══════════════════════════════════════════════════════════════════════════════

/// BT21-072 must report Raid via has_keyword when on the field.
///
/// NOTE (G-DECLARATIVE-KEYWORD): Declarative GrantKeyword clauses compile and
/// are registered, but if the runtime declarative-tick path is incomplete they
/// may not be visible via has_keyword. If this test fails, add:
/// `#[ignore = "pending: G-DECLARATIVE-KEYWORD"]`
#[test]
fn bt21_072_has_raid_keyword_when_on_field() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", Some(0));

    assert!(
        runner.game.has_keyword(handle, Keyword::Raid),
        "Arresterdramon: Superior Mode must have Raid when on the field"
    );
}

/// BT21-072 must report Piercing via has_keyword when on the field.
#[test]
fn bt21_072_has_piercing_keyword_when_on_field() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", Some(0));

    assert!(
        runner.game.has_keyword(handle, Keyword::Piercing),
        "Arresterdramon: Superior Mode must have Piercing when on the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited +2000 DP aura behavioral tests (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// When BT21-072 is a digivolution source under a carrier, the
/// source_dp_contribution at index 0 must be +2000 on the CONTROLLER'S turn.
///
/// Uses a synthetic CARRIER card (make_test_card) so we avoid loading a second
/// DSL card that may not be in the embedded pack. The carrier has dp=5000 so
/// the effect of the +2000 inheritance is clearly distinguishable.
///
/// NOTE (G-INHERITED-DISPATCH): G-INHERITED-DISPATCH blocks triggered inherited
/// effects dispatched through the trigger queue. However, declarative inherited
/// auras write to Effect.dp_modifier which source_dp_contribution reads directly
/// — no queue involved. This test should pass independently of that gap.
/// If it fails, tag: `#[ignore = "pending: G-INHERITED-DISPATCH"]`
#[test]
fn bt21_072_inherited_aura_contributes_2000_dp_your_turn() {
    let arresterdramon_data = dsl_card_data::card_data_from_compiled("BT21-072");

    // Synthetic carrier — no effects of its own; just a Digimon shell.
    let mut carrier_card = make_test_card("CARRIER-LV5", "CarrierLv5");
    carrier_card.level = Some(5);
    carrier_card.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(arresterdramon_data)
        .add_card(carrier_card)
        .memory(9)
        .start();

    // Place BT21-072 on field as position 0 (it becomes source[0]).
    let perm_handle = runner.place_on_field(0, "BT21-072", Some(0));

    // Push CARRIER-LV5 on top via digivolve so BT21-072 is the bottom source.
    {
        let game = runner.game_mut();
        let carrier_data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "CARRIER-LV5")
            .expect("CARRIER-LV5 in card_data");
        let next_idx = game.next_card_index();
        let carrier_src = CardSource::new(carrier_data_idx, 0, next_idx);
        let turn = game.turn_count;
        game.players[0].battle_area[perm_handle.index as usize].digivolve(carrier_src, turn);
    }

    assert_eq!(
        runner.turn_player(),
        0,
        "precondition: P0 must be the turn player"
    );

    // source_dp_contribution at index 0 = BT21-072 inherited contribution.
    let contribution = runner.game.source_dp_contribution(perm_handle, 0);

    assert_eq!(
        contribution, 2000,
        "Arresterdramon: Superior Mode inherited aura must contribute +2000 DP \
         on controller's turn; got {contribution}"
    );
}

/// The inherited +2000 DP aura must NOT contribute on the opponent's turn
/// (active_when: { your_turn: true } gate must block it).
///
/// After end_turn the turn player becomes P1, so P0's cards are on the
/// opponent's turn — the aura's condition check should return 0.
#[test]
fn bt21_072_inherited_aura_contributes_zero_dp_opponents_turn() {
    let arresterdramon_data = dsl_card_data::card_data_from_compiled("BT21-072");

    let mut carrier_card = make_test_card("CARRIER-LV5", "CarrierLv5");
    carrier_card.level = Some(5);
    carrier_card.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(arresterdramon_data)
        .add_card(carrier_card)
        .memory(9)
        .start();

    let perm_handle = runner.place_on_field(0, "BT21-072", Some(0));

    {
        let game = runner.game_mut();
        let carrier_data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "CARRIER-LV5")
            .expect("CARRIER-LV5 in card_data");
        let next_idx = game.next_card_index();
        let carrier_src = CardSource::new(carrier_data_idx, 0, next_idx);
        let turn = game.turn_count;
        game.players[0].battle_area[perm_handle.index as usize].digivolve(carrier_src, turn);
    }

    // Advance to opponent's turn.
    runner.end_turn();
    assert_ne!(
        runner.turn_player(),
        0,
        "precondition: P0 must NOT be the turn player"
    );

    let contribution = runner.game.source_dp_contribution(perm_handle, 0);

    assert_eq!(
        contribution, 0,
        "Inherited aura must be inactive on opponent's turn (active_when: your_turn); \
         got {contribution}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — BLOCKED tests (pending gap closure)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_072_when_digivolving_may_attack_without_suspending() {
    let mut security_card = make_test_card("SEC", "Security");
    security_card.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-072")
        .expect("BT21-072 in embedded DSL pack")
        .add_card(security_card)
        .security(1, &["SEC"])
        .memory(9)
        .start();
    runner.game.turn_count = 1;

    let attacker = runner.place_on_field(0, "BT21-072", Some(0));
    let security_before = runner.game.player(1).security.len();

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(attacker),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("When Digivolving should install a may_attack_now prompt");
    assert!(
        pending.is_optional,
        "printed 'may attack' must expose decline/PASS"
    );
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "optional may_attack_now prompt must expose PASS through the action mask"
    );

    let attack_player = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert!(
        pending.valid_action_ids.contains(&attack_player),
        "BT21-072 should be able to choose the opponent player as the attack target"
    );

    runner
        .game
        .resolve_selection(0, attack_player)
        .expect("resolve unsuspended effect-created attack");

    assert_eq!(
        runner.game.player(1).security.len(),
        security_before - 1,
        "effect-created attack should resolve the normal security flow"
    );
    assert!(
        !runner.game.player(0).battle_area[attacker.index as usize].is_suspended,
        "BT21-072 must remain unsuspended after attacking without suspending"
    );
}

/// [All Turns] This Digimon gets +1000 DP for each of its digivolution cards.
///
/// BLOCKED (G-AURA-DP-FORMULA): The DSL `kind: aura` declarative with an
/// empty (self) target only accepts `dp_modifier: <i32>` (a static literal).
/// There is no `dp_modifier_fn` or `dp_modifier_formula` field on AuraBody.
/// The DCGO uses `ChangeSelfDPStaticEffect(changeValue: 1000 * count(), ...)`
/// at EffectTiming.None where count() is a live lambda reading
/// `PermanentOfThisCard().DigivolutionCards.Count()` (= stack_size - 1 in DSL
/// terminology, i.e. material_count). This requires the DP to recompute
/// dynamically after de_digivolve operations, which a snapshot-at-event
/// `add_dp_modifier` cannot model.
///
/// When gap closes, the YAML clause should be:
///   - kind: aura
///     active_when: { all_turns: true }   # or omit for always-on
///     target: {}                          # self
///     dp_modifier_fn:                     # NEW: formula-based variant
///       base: 0
///       per: material_count
///       delta: 1000
///
/// Test plan when gap closes:
///   1. Place BT21-072 alone (0 digivolution cards → +0 DP; base 10000).
///   2. Add 1 digivolution source → DP = 11000.
///   3. Add 2nd source → DP = 12000.
///   4. De-digivolve (trash top source) → DP drops to 11000.
///   5. Assert both own-turn and opponent's-turn snapshots ([All Turns] = no gate).
#[test]
#[ignore = "pending: G-AURA-DP-FORMULA — AuraBody.dp_modifier does not accept a formula"]
fn bt21_072_all_turns_plus1000_dp_per_digivolution_card_dynamic() {
    let _ = arresterdramon_runner();
}
