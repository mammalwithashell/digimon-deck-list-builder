//! BT3-103 Hidden Potential Discovered! — Option, Green, Cost 4.
//!
//! # Card text (cards.json)
//!
//! [Main] For the turn, when one of your green Digimon would next digivolve,
//! by suspending 1 of your Digimon, reduce the digivolution cost by 5.
//!
//! Inherited: Security Effect [Security] Add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_103.cs
//!
//! # Implementation status: IMPLEMENTED
//!
//! | Clause | Description                                       | Status      |
//! |--------|---------------------------------------------------|-------------|
//! | 0      | [Main] one-shot green-Digimon digivolve cost -5   | IMPLEMENTED |
//! | 1      | [Security] Add this card to the hand              | IMPLEMENTED |
//!
//! # Clause 0 — IMPLEMENTED (G-COST-REDUCE-ALLY-DIGIVOLVE, 2026-05-21)
//!
//! The main effect arms a player-scoped, turn-scoped, single-fire digivolve
//! cost reducer (`arm_digivolve_cost_reducer`). At the next qualifying green
//! digivolution, the digivolve cost path installs an accept/decline prompt;
//! on accept, the player is prompted to suspend 1 of their own Digimon, and
//! the digivolution cost is reduced by 5.
//!
//! The primitive (`Game::player_digivolve_cost_reducers` +
//! `EffectContext::arm_player_digivolve_cost_reducer` +
//! `Game::try_prompt_player_digivolve_cost_reducer`) closes three gaps:
//!
//! * **G-COST-REDUCE-ALLY-DIGIVOLVE** — a player-scoped reducer with no
//!   field permanent to host it; consulted by the digivolve cost path.
//! * **G-COST-REDUCE-NEXT-SINGLE-FIRE** — `single_fire` consumes the reducer
//!   on the first successful application.
//! * **G-PAY-COST-SELECT-ARBITRARY-SUSPEND** — `suspend_cost` installs an
//!   interactive `OwnField` selection (suspend 1 of your own Digimon).
//!
//! # Clause 1 — IMPLEMENTED
//!
//! The inherited security clause uses `add_this_option_to_hand: {}`, resolved
//! 2026-05-01. `EffectContext::add_pending_security_to_hand()` moves the card
//! from `Game.pending_security` to the defender/controller hand.
//!
//! # Test coverage
//!
//! Structural:
//!   - `bt3_103_compiles_as_option_card` — card parses + compiles, cost 4, green
//!   - `bt3_103_has_main_and_inherited_security_clause` — 2 clauses; [Main]
//!     carries `ArmDigivolveCostReducer`
//!   - `bt3_103_security_clause_uses_add_this_option_to_hand` — step-kind assertion
//!   - `bt3_103_security_clause_is_not_optional` — mandatory per rules
//!
//! Clause 0 behavioral:
//!   - `bt3_103_main_arms_digivolve_cost_reduction_for_turn`
//!   - `bt3_103_cost_reduction_fires_on_green_digimon_only`
//!   - `bt3_103_cost_reduction_does_not_fire_on_non_green_digimon`
//!   - `bt3_103_cost_reduction_requires_player_to_suspend_one_digimon`
//!   - `bt3_103_cost_reduction_fires_at_most_once_per_play`

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

// Evo-color byte: Green == 3, Red == 0 (matches `card_data::parse_card_color`).
const EVO_GREEN: u8 = 3;
const EVO_RED: u8 = 0;

// ─── Helper builders ─────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_green_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c
}

fn make_red_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c
}

/// A Lv.3 base Digimon of `color` — a legal digivolve base.
fn lvl3_base(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![color];
    c
}

/// A Lv.4 Digimon with a printed `Lv.3 <evo_color>, cost <memory_cost>`
/// digivolution cost.
fn lvl4_evo3(id: &str, color: CardColor, evo_color: u8, memory_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.evo_costs = vec![EvoCost {
        card_color: evo_color,
        level: 3,
        memory_cost,
    }];
    c
}

/// Base runner: BT3-103 registered in hand index 0, filler decks on both
/// sides so no deck-out on draw, filler security for P1.
fn runner() -> DebugRunner {
    let filler = make_filler("FILL");
    DebugRunner::builder()
        .dsl_card("BT3-103")
        .expect("BT3-103 YAML must parse and compile")
        .add_card(filler.clone())
        .hand(0, &["BT3-103"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(4)
        .start()
}

/// Digivolve-scenario runner: BT3-103 in hand index 0, a green Lv.4 evo
/// card and a red Lv.4 evo card also in hand, a green Lv.3 base and a red
/// Lv.3 base on P1's field. Both Lv.4 cards have printed digivolution cost
/// 4. `memory` is set to 10 so cost payment is observable.
///
/// Returns `(runner, green_base_index, red_base_index)`.
fn digivolve_scenario() -> (DebugRunner, usize, usize) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-103")
        .expect("BT3-103 YAML must parse and compile")
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl3_base("RBASE", CardColor::Red))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .add_card(lvl4_evo3("REVO", CardColor::Red, EVO_RED, 4))
        .hand(0, &["BT3-103", "GEVO", "REVO"])
        .deck(0, &["GBASE"; 5])
        .memory(10)
        .start();
    let green_base = r.place_on_field(0, "GBASE", Some(0)).index as usize;
    let red_base = r.place_on_field(0, "RBASE", Some(0)).index as usize;
    (r, green_base, red_base)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (active)
// ═══════════════════════════════════════════════════════════════════════════

/// BT3-103 compiles as a green Option with cost 4.
#[test]
fn bt3_103_compiles_as_option_card() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 must be in compiled_cards");

    use digimon_dsl::compiled::CompiledCardKind;
    assert_eq!(
        card.kind,
        CompiledCardKind::Option,
        "must be an Option card"
    );
    assert_eq!(card.cost, Some(4), "play cost must be 4");

    // Color is not stored in CompiledCard but we can verify via the raw
    // card registry instead.
    // The YAML specifies color: [green]; the card registry does not surface
    // CompiledCard.colors — just assert the cost and kind are correct here.
}

/// BT3-103 has exactly 2 clauses: the [Main] cost-reduction clause and the
/// inherited [Security] clause.
#[test]
fn bt3_103_has_main_and_inherited_security_clause() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 must be in compiled_cards");

    assert_eq!(
        card.effects.len(),
        2,
        "BT3-103 must have exactly 2 clauses ([Main] cost reducer + \
         inherited [Security])"
    );

    // The [Main] clause must be a triggered clause carrying the
    // `ArmDigivolveCostReducer` step.
    let main_has_arm = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::ArmDigivolveCostReducer { .. })),
        _ => false,
    });
    assert!(
        main_has_arm,
        "the [Main] clause must contain an ArmDigivolveCostReducer step"
    );
}

/// The single clause must be the inherited security clause with
/// `OnSecurity` timing.
#[test]
fn bt3_103_security_clause_is_inherited_with_on_security_timing() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have a clause with OnSecurity timing");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "security clause must have Inherited scope"
    );
}

/// The security clause must NOT be optional — [Security] effects are
/// mandatory per the rules manual and DCGO.
#[test]
fn bt3_103_security_clause_is_not_optional() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause must exist");

    assert!(
        !clause.optional,
        "security clause must not be optional — [Security] effects are mandatory"
    );
}

/// The security clause process must contain exactly one step and that step
/// must be `AddThisOptionToHand` (the resolved G-ADD-OPTION-SELF-TO-HAND
/// primitive).
#[test]
fn bt3_103_security_clause_uses_add_this_option_to_hand() {
    let r = runner();
    let card = r
        .compiled_card("BT3-103")
        .expect("BT3-103 compiled card present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause must exist");

    assert_eq!(
        clause.process.len(),
        1,
        "security clause must have exactly 1 process step"
    );

    let is_add_to_hand = matches!(&clause.process[0], CompiledStep::AddThisOptionToHand);
    assert!(
        is_add_to_hand,
        "security clause process step must be AddThisOptionToHand, got: {:?}",
        &clause.process[0]
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0 (Main) — behavioral tests
// ═══════════════════════════════════════════════════════════════════════════
//
// Closed by G-COST-REDUCE-ALLY-DIGIVOLVE (2026-05-21): the player-scoped
// one-shot future-digivolve cost reducer. `arm_digivolve_cost_reducer`
// installs a turn-scoped reducer; the digivolve cost path prompts an
// accept/decline + suspend selection before the cost is paid.
//
// Gap tags (all closed):
//   G-COST-REDUCE-ALLY-DIGIVOLVE — player-scoped digivolve cost reducer
//   G-COST-REDUCE-NEXT-SINGLE-FIRE — single-fire one-shot lifecycle
//   G-PAY-COST-SELECT-ARBITRARY-SUSPEND — interactive suspend cost

/// Playing BT3-103's [Main] effect arms a player-scoped digivolve cost
/// reducer for the current turn (`G-COST-REDUCE-ALLY-DIGIVOLVE`).
#[test]
fn bt3_103_main_arms_digivolve_cost_reduction_for_turn() {
    let (mut r, _g, _red) = digivolve_scenario();
    assert!(r.game.player_digivolve_cost_reducers.is_empty());

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must fire BT3-103's [Main] clause");

    assert_eq!(
        r.game.player_digivolve_cost_reducers.len(),
        1,
        "the [Main] clause must arm exactly one player-scoped reducer"
    );
    let reducer = &r.game.player_digivolve_cost_reducers[0];
    assert_eq!(reducer.player, 0);
    assert_eq!(reducer.amount, 5, "the reduction is -5");
    assert!(reducer.single_fire, "\"next digivolve\" is single-fire");
    assert!(reducer.suspend_cost, "the cost is suspending 1 Digimon");

    // "For the turn" — the reducer expires at end of turn.
    r.end_turn();
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "the reducer is dropped at end of the installing player's turn"
    );
}

/// A GREEN Digimon digivolving fires the reducer prompt; accepting,
/// suspending a Digimon, applies the -5 reduction once.
#[test]
fn bt3_103_cost_reduction_fires_on_green_digimon_only() {
    let (mut r, green_base, _red) = digivolve_scenario();
    r.game.activate_hand_main(0, 0); // arm the reducer
    let memory_before = r.memory();

    // `activate_hand_main` runs the [Main] process without consuming the
    // card, so the hand is still [BT3-103, GEVO, REVO] — GEVO is index 1.
    let ok = r
        .game
        .digivolve_from_hand(0, 1, green_base, PlaySource::ByDigivolve);
    assert!(!ok, "digivolve parks on the accept/decline prompt");
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("green digivolution installs the reducer prompt");
    assert!(matches!(pending.kind, SelectionKind::EffectChoice));
    assert!(pending.is_optional, "the prompt is declinable");
    assert_eq!(r.memory(), memory_before, "no cost paid before the prompt");

    // Accept → suspend the green base itself.
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("accept resolves");
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("suspend-cost selection installed");
    assert!(matches!(pending.kind, SelectionKind::OwnField));
    r.game
        .resolve_selection(0, encode_attack(0, green_base as u16))
        .expect("suspend selection resolves");

    assert!(r.game.pending_selection.is_none(), "digivolution completes");
    // printed digivolution cost 4 - 5 = clamped to 0.
    assert_eq!(
        r.memory(),
        memory_before,
        "the reduced digivolution cost is 0 (4 - 5, clamped)"
    );
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer consumed after the application"
    );
}

/// Negative test: a non-green (red) Digimon digivolving never sees the
/// prompt — the reducer's green target filter excludes it.
#[test]
fn bt3_103_cost_reduction_does_not_fire_on_non_green_digimon() {
    let (mut r, _green, red_base) = digivolve_scenario();
    r.game.activate_hand_main(0, 0); // arm the reducer
    let memory_before = r.memory();

    // Hand is [BT3-103, GEVO, REVO] (activate_hand_main does not consume
    // the card) — REVO is index 2.
    let ok = r
        .game
        .digivolve_from_hand(0, 2, red_base, PlaySource::ByDigivolve);
    assert!(ok, "non-green digivolution succeeds outright");
    assert!(
        r.game.pending_selection.is_none(),
        "a non-green digivolution must NOT install the reducer prompt"
    );
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "non-green digivolution pays the full digivolution cost"
    );
    assert_eq!(
        r.game.player_digivolve_cost_reducers.len(),
        1,
        "the green-only reducer is untouched by a non-green digivolution"
    );
}

/// The suspend cost is a real player choice: declining the prompt keeps the
/// full digivolution cost and leaves the reducer armed
/// (`G-PAY-COST-SELECT-ARBITRARY-SUSPEND`).
#[test]
fn bt3_103_cost_reduction_requires_player_to_suspend_one_digimon() {
    let (mut r, green_base, _red) = digivolve_scenario();
    r.game.activate_hand_main(0, 0); // arm the reducer
    let memory_before = r.memory();

    // Hand is [BT3-103, GEVO, REVO] — GEVO is index 1.
    r.game
        .digivolve_from_hand(0, 1, green_base, PlaySource::ByDigivolve);
    // Decline — the player chooses NOT to suspend a Digimon.
    r.game
        .resolve_selection(0, PASS)
        .expect("declining the reducer prompt resolves");

    assert!(
        r.game.pending_selection.is_none(),
        "digivolution completes after decline"
    );
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "declining keeps the full digivolution cost (no suspend, no -5)"
    );
    assert_eq!(
        r.game.player_digivolve_cost_reducers.len(),
        1,
        "a declined prompt must leave the single-fire reducer armed"
    );
}

/// "Next digivolve" is single-fire: after the first successful application,
/// a second green digivolution in the same turn does not get the reduction
/// (`G-COST-REDUCE-NEXT-SINGLE-FIRE`).
#[test]
fn bt3_103_cost_reduction_fires_at_most_once_per_play() {
    // Two green Lv.3 bases so the second digivolution has a fresh target.
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-103")
        .expect("BT3-103 YAML must parse and compile")
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .add_card(lvl4_evo3("GEVO2", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["BT3-103", "GEVO", "GEVO2"])
        .deck(0, &["GBASE"; 5])
        .memory(20)
        .start();
    let base_a = r.place_on_field(0, "GBASE", Some(0)).index as usize;
    let base_b = r.place_on_field(0, "GBASE", Some(0)).index as usize;

    r.game.activate_hand_main(0, 0); // arm the reducer

    // Hand is [BT3-103, GEVO, GEVO2] (activate_hand_main does not consume
    // BT3-103). First green digivolution uses GEVO at index 1.
    r.game
        .digivolve_from_hand(0, 1, base_a, PlaySource::ByDigivolve);
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("accept");
    r.game
        .resolve_selection(0, encode_attack(0, base_b as u16))
        .expect("suspend");
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer consumed after the first digivolution"
    );

    let memory_before = r.memory();
    // GEVO was consumed → hand is [BT3-103, GEVO2]; GEVO2 is now index 1.
    // Second green digivolution onto base_b — no prompt.
    let ok = r
        .game
        .digivolve_from_hand(0, 1, base_b, PlaySource::ByDigivolve);
    assert!(ok, "second green digivolution succeeds outright (no prompt)");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "second digivolution pays the full cost — the reducer is one-shot"
    );
}
