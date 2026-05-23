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
//! - [All Turns] +1000 DP per digivolution card (dp_modifier_fn formula aura)
//! - xros_req alt-path: Lv.4 cost-4 standard + Lv.4 w/＜Save＞-in-text OR
//!   [Hero]-trait cost-3 (G-ALT-PATH-SAVE-IN-TEXT — `effect_text_contains`)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (1) <Raid> | none | PASS — structural + has_keyword |
//! | (2) <Piercing> | none | PASS — structural + has_keyword |
//! | (3) [When Digivolving] may attack without suspending | none | PASS — may_attack_now optional/without_suspending |
//! | (4) [All Turns] +1000 DP per digivolution card | none (G-AURA-DP-FORMULA resolved) | PASS — dp_modifier_fn formula aura |
//! | (5) Inherited [Your Turn] +2000 DP | none (G-INHERITED-DISPATCH resolved for declarative) | PASS — structural + behavioral |

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

/// BT21-072 must have four declarative clauses (Raid + Piercing GrantKeyword,
/// the face-up [All Turns] formula Aura, the inherited [Your Turn] +2000 Aura)
/// and one triggered [When Digivolving] may-attack clause.
#[test]
fn bt21_072_structural_four_declarative_clauses_and_one_triggered() {
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
        4,
        "Expected 2 GrantKeyword + 2 Aura (formula DP + inherited) = 4 declaratives"
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

/// The inherited Aura must be scope=Inherited, active_when=your_turn, dp_modifier=2000,
/// and carry a static (non-formula) DP modifier.
#[test]
fn bt21_072_structural_inherited_aura_your_turn_dp_2000() {
    let runner = arresterdramon_runner();
    let compiled = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: scope @ CompiledScope::Inherited,
            dp_modifier,
            dp_modifier_fn,
            active_when,
            ..
        }) => Some((
            scope.clone(),
            *dp_modifier,
            dp_modifier_fn.is_some(),
            active_when.is_some(),
        )),
        _ => None,
    });

    let (scope, dp, has_fn, has_active_when) =
        aura.expect("Expected an Inherited-scoped Aura declarative clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "Aura must be Inherited scope"
    );
    assert_eq!(
        dp,
        Some(2000),
        "Inherited aura must carry +2000 static dp_modifier"
    );
    assert!(
        !has_fn,
        "Inherited +2000 aura is a static literal, not a formula"
    );
    assert!(
        has_active_when,
        "Inherited aura must have active_when (your_turn gate)"
    );
}

/// The [All Turns] +1000-DP-per-digivolution-card aura must be a face-up
/// (non-inherited) self-aura carrying a `dp_modifier_fn` formula — not a static
/// literal — so the engine recomputes DP continuously as the stack changes.
#[test]
fn bt21_072_structural_all_turns_formula_aura_uses_dp_modifier_fn() {
    let runner = arresterdramon_runner();
    let compiled = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let formula_aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: scope @ CompiledScope::FaceUp,
            dp_modifier,
            dp_modifier_fn,
            active_when,
            ..
        }) => Some((
            scope.clone(),
            *dp_modifier,
            dp_modifier_fn.clone(),
            active_when.is_some(),
        )),
        _ => None,
    });

    let (scope, static_dp, fn_dp, has_active_when) =
        formula_aura.expect("Expected a face-up Aura declarative clause for [All Turns] +1000 DP");
    assert_eq!(
        scope,
        CompiledScope::FaceUp,
        "[All Turns] formula aura must be face-up scope (not inherited)"
    );
    assert!(
        static_dp.is_none(),
        "[All Turns] aura must NOT carry a static dp_modifier — it is formula-valued"
    );
    assert!(
        fn_dp.is_some(),
        "[All Turns] aura must carry a dp_modifier_fn formula (material_count × 1000)"
    );
    assert!(
        has_active_when,
        "[All Turns] aura must have an active_when gate (all_turns: true)"
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
// Section 4 — [When Digivolving] may attack without suspending (clause 3)
// ═══════════════════════════════════════════════════════════════════════════════

/// Accept path: BT21-072 takes the optional unsuspended attack and resolves
/// the normal security flow without becoming suspended.
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

/// Decline path: the printed "may attack" must be fully declinable. Choosing
/// PASS at the may_attack_now prompt must NOT touch the opponent's security and
/// must leave BT21-072 unsuspended (no attack ever happened).
#[test]
fn bt21_072_when_digivolving_may_attack_can_be_declined() {
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

    // Decline the optional attack.
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("declining the optional may_attack_now prompt must succeed");

    assert!(
        runner.game.pending_selection.is_none(),
        "declining must clear the pending selection"
    );
    assert_eq!(
        runner.game.player(1).security.len(),
        security_before,
        "declining the may-attack prompt must not resolve any security check"
    );
    assert!(
        !runner.game.player(0).battle_area[attacker.index as usize].is_suspended,
        "BT21-072 must remain unsuspended when the optional attack is declined"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [All Turns] +1000 DP per digivolution card (clause 4)
// ═══════════════════════════════════════════════════════════════════════════════

/// [All Turns] This Digimon gets +1000 DP for each of its digivolution cards.
///
/// With BT21-072 alone (top card, no digivolution sources beneath it),
/// material_count == 0, so the formula yields +0 DP. Effective DP == base 10000.
///
/// G-AURA-DP-FORMULA RESOLVED (Group 6): `kind: aura` accepts `dp_modifier_fn`,
/// a runtime formula closure that `Game::effective_dp` re-evaluates at query
/// time — so the DP tracks the stack continuously.
#[test]
fn bt21_072_all_turns_dp_zero_bonus_with_no_digivolution_cards() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", None);

    let effective = runner
        .effective_dp(handle)
        .expect("BT21-072 must have an effective DP");
    assert_eq!(
        effective, 10000,
        "With 0 digivolution cards, [All Turns] formula adds +0 DP; effective DP = base 10000"
    );
}

/// One digivolution card beneath BT21-072 → material_count == 1 → +1000 DP.
/// Effective DP == 10000 + 1000 = 11000.
#[test]
fn bt21_072_all_turns_dp_plus1000_with_one_digivolution_card() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", None);

    // push_source inserts a digivolution card beneath the top card.
    runner.push_source(handle, "BT21-072");

    let effective = runner
        .effective_dp(handle)
        .expect("BT21-072 must have an effective DP");
    assert_eq!(
        effective, 11000,
        "With 1 digivolution card, [All Turns] adds +1000 DP; effective DP = 11000"
    );
}

/// Two digivolution cards beneath BT21-072 → material_count == 2 → +2000 DP.
/// Effective DP == 10000 + 2000 = 12000. Proves the formula scales per-card.
#[test]
fn bt21_072_all_turns_dp_plus2000_with_two_digivolution_cards() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", None);

    runner.push_source(handle, "BT21-072");
    runner.push_source(handle, "BT21-072");

    let effective = runner
        .effective_dp(handle)
        .expect("BT21-072 must have an effective DP");
    assert_eq!(
        effective, 12000,
        "With 2 digivolution cards, [All Turns] adds +2000 DP; effective DP = 12000"
    );
}

/// The formula is recomputed continuously: removing a digivolution card (the
/// effect of a de-digivolve) must immediately drop the DP bonus. Starting from
/// 2 sources (12000), trashing the bottom source returns to 1 source (11000).
///
/// This proves the aura is a live formula closure, not an event-time snapshot
/// — a static `add_dp_modifier` snapshot would leave DP frozen at 12000.
#[test]
fn bt21_072_all_turns_dp_recomputes_after_digivolution_card_removed() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", None);

    runner.push_source(handle, "BT21-072");
    runner.push_source(handle, "BT21-072");
    assert_eq!(
        runner.effective_dp(handle),
        Some(12000),
        "precondition: 2 digivolution cards → 12000 DP"
    );

    // Simulate a de-digivolve: pop one digivolution card from beneath the top.
    {
        let perm = &mut runner.game.players[0].battle_area[handle.index as usize];
        assert!(
            perm.card_sources.len() >= 3,
            "precondition: stack has top + 2 digivolution cards"
        );
        // Remove the bottom-most digivolution card (index 0), preserving top.
        perm.card_sources.remove(0);
    }

    let effective = runner
        .effective_dp(handle)
        .expect("BT21-072 must still have an effective DP");
    assert_eq!(
        effective, 11000,
        "After a digivolution card is removed, the formula must recompute: 1 card → 11000 DP"
    );
}

/// [All Turns] has no turn gate — the +1000-per-card bonus must apply on the
/// opponent's turn just as on the controller's turn.
#[test]
fn bt21_072_all_turns_dp_bonus_applies_on_opponents_turn() {
    let mut runner = arresterdramon_runner();
    let handle = runner.place_on_field(0, "BT21-072", None);
    runner.push_source(handle, "BT21-072"); // material_count == 1

    assert_eq!(
        runner.effective_dp(handle),
        Some(11000),
        "precondition: on controller's turn, 1 digivolution card → 11000 DP"
    );

    // Advance to the opponent's turn.
    runner.end_turn();
    assert_ne!(
        runner.turn_player(),
        0,
        "precondition: P0 must NOT be the turn player"
    );

    let effective = runner
        .effective_dp(handle)
        .expect("BT21-072 must have an effective DP on opponent's turn");
    assert_eq!(
        effective, 11000,
        "[All Turns] formula aura has no turn gate — +1000 DP must persist on opponent's turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — alt-path digivolution requirements (xros_req)
// ═══════════════════════════════════════════════════════════════════════════════

/// BT21-072 must register two digivolve alt-paths: the standard Lv.4 cost-4
/// path and the xros_req "[Digivolve] Lv.4 w/＜Save＞ in text or w/[Hero]
/// trait: Cost 3". G-ALT-PATH-SAVE-IN-TEXT.
#[test]
fn bt21_072_has_two_digivolve_alt_paths_standard_and_xros_req() {
    use digimon_dsl::compiled::CompiledAltPathKind;

    let runner = arresterdramon_runner();
    let card = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();

    assert_eq!(
        digivolve_paths.len(),
        2,
        "BT21-072 must have exactly 2 digivolve alt-paths (standard cost-4 + xros_req cost-3); \
         got {}",
        digivolve_paths.len()
    );
}

/// The xros_req cost-3 alt-path's `from:` filter must be an OR of
/// (Lv.4 with ＜Save＞ in its printed text) and (Lv.4 with the [Hero] trait).
/// The "w/＜Save＞ in text" half uses the existing `effect_text_contains`
/// predicate — the requirement's own wording ("in text") is a text-presence
/// check. G-ALT-PATH-SAVE-IN-TEXT.
#[test]
fn bt21_072_xros_req_alt_path_gates_on_save_in_text_or_hero_trait() {
    use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};

    let runner = arresterdramon_runner();
    let card = runner
        .compiled_card("BT21-072")
        .expect("BT21-072 compiled card present");

    // The xros_req path is the digivolve alt-path whose `from:` is an OR.
    let xros_req = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .find(|p| p.from.as_ref().is_some_and(|f| !f.any_of.is_empty()))
        .expect("BT21-072 must have a xros_req digivolve alt-path with an any_of from-filter");

    assert_eq!(
        xros_req.cost,
        Some(CompiledCost::Literal(3)),
        "the xros_req alt-path must cost 3"
    );
    assert!(
        xros_req.ignore_requirements,
        "the xros_req alt-path replaces the printed digivolution requirement"
    );

    let from = xros_req
        .from
        .as_ref()
        .expect("xros_req path must carry a from: filter");
    assert_eq!(
        from.any_of.len(),
        2,
        "xros_req from: must be an any_of of the ＜Save＞-in-text and [Hero]-trait branches"
    );
    assert!(
        from.any_of
            .iter()
            .any(|b| b.effect_text_contains.is_some() && b.level_eq == Some(4)),
        "one branch must gate on Lv.4 + ＜Save＞ in the source card's printed text \
         (effect_text_contains)"
    );
    assert!(
        from.any_of
            .iter()
            .any(|b| b.trait_has.as_deref() == Some("Hero") && b.level_eq == Some(4)),
        "one branch must gate on Lv.4 + the [Hero] trait"
    );
}
