//! ST12-15 From Master to Disciple — Option, Red, Cost 2.
//!
//! # Card text (card image + cards.json)
//!
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or
//! [Sistermon] in its name or [Royal Knight] in its traits among them to your
//! hand. Trash the rest. Then, place this card in your Battle Area.
//!
//! [Main] <Delay> (Trash this card in your battle area to activate the effect
//! below. You can't activate this effect the turn this card enters play.)
//! - The next time one of your Digimon would digivolve this turn, reduce the
//!   digivolution cost by 1.
//!
//! [Security] Reveal 3 cards from the top of your deck. Add 1 card with
//! [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among
//! them to your hand. Trash the rest. Then, place this card in your Battle Area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST12/Red/ST12_15.cs — the OnDeclaration
//! (<Delay>) block installs an UNCONDITIONAL `ChangeCost: Cost -= 1` for any
//! of the controller's Digimon (`cardSource.Owner == card.Owner`, no color
//! filter, no accept/decline gate). A background AfterPayCost "Remove Effect"
//! process removes it after the next qualifying digivolve (single-fire); it
//! also expires at end of turn (UntilEachTurnEnd) if unused.
//!
//! # Implementation status
//!
//! | Clause          | Description                                          | Status      |
//! |-----------------|------------------------------------------------------|-------------|
//! | [Main]          | reveal 3 / add filtered / trash rest / place as Delay| IMPLEMENTED |
//! | [Main] <Delay>  | next digivolve this turn costs 1 less (auto, 1-shot) | IMPLEMENTED |
//! | [Security]      | same search body                                     | IMPLEMENTED |
//!
//! The <Delay> body arms a player-scoped, turn-scoped, single-fire,
//! ANY-color, NO-suspend-cost digivolve cost reducer
//! (`arm_digivolve_cost_reducer: { amount: 1, single_fire: true }`). Because
//! the reducer is FREE (`suspend_cost == false`), the engine AUTO-APPLIES the
//! -1 at the next qualifying digivolution with NO accept/decline prompt,
//! matching DCGO (G-DELAY-NEXT-DIGIVOLVE-COST-REDUCTION).

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, PlaySource};
use digimon_engine::permanent::OptionState;

// Evo-color byte: Red == 0 (matches `card_data::parse_card_color`).
const EVO_RED: u8 = 0;

// ─── Helper builders ─────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
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

/// A Royal-Knight Digimon (used to verify the [Main] search add filter).
fn royal_knight(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Royal Knight".to_string()];
    c
}

/// Place ST12-15 in `player`'s battle area as a standard `<Delay>` Option that
/// was placed on a PRIOR turn (so its [Main] activation is legal now).
fn seat_delay(runner: &mut DebugRunner, player: u8) -> usize {
    let perm = runner.place_on_field(player, "ST12-15", Some(0));
    let idx = perm.index as usize;
    // Placed on turn 0; current turn is later — activatable (16-16-3).
    runner.game.player_mut(player).battle_area[idx].option_state = OptionState::Delayed {
        owner: player,
        trash_on_turn: u16::MAX,
        trigger: DelayTrigger::MainPhaseActivated,
        placed_on_turn: 0,
    };
    idx
}

/// FIELD_EFFECT main-slot action bit for the delay Option at `idx`.
fn delay_main_bit(idx: usize) -> u16 {
    FIELD_EFFECT_START + idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural (the previously-passing clause map)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st12_15_has_main_search_and_security_search() {
    let runner = DebugRunner::builder()
        .dsl_card("ST12-15")
        .expect("ST12-15 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("ST12-15").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "ST12-15 must have Main search"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "ST12-15 must have Security search"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [Main] search behavioral (reveal 3 / add 1 filtered / trash rest
//             / place self as Delay)
// ═══════════════════════════════════════════════════════════════════════════

/// [Main] reveals the top 3, adds the 1 [Royal Knight] card to hand, trashes
/// the other 2 (non-matching) cards, and places ST12-15 itself in the battle
/// area as a `MainPhaseActivated` Delay Option.
#[test]
fn st12_15_main_search_adds_royal_knight_trashes_rest_places_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-15")
        .expect("ST12-15 must load from embedded DSL pack")
        .add_card(royal_knight("RK-CARD"))
        .add_card(lvl3_base("RBASE", CardColor::Red))
        .add_card(filler("FILL"))
        .hand(0, &["ST12-15"])
        // Deck top is `.last()`. Top 3 (popped) = RK-CARD (matches), FILL, FILL
        // (do not match); the leading 2 FILLs stay at the deck bottom.
        .deck(0, &["FILL", "FILL", "RK-CARD", "FILL", "FILL"])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    // A red Digimon on field satisfies the Option's red color requirement.
    runner.place_on_field(0, "RBASE", Some(0));
    let trash_before = runner.trash_size(0);

    // Play ST12-15 from hand as an Option through the real play path — this
    // parks the reveal/add selection so auto_resolve can pick the one matching
    // candidate (RK-CARD), then trashes the rest and places ST12-15 as a Delay.
    runner.game.enter_main_phase();
    use digimon_engine::selection::OptionPlayResult;
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "ST12-15 [Main] parks the forced add-1-from-reveal selection"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(digimon_engine::selection::SelectionKind::Reveal),
        "the add-1-from-reveal pick must surface as a forced Reveal selection"
    );
    runner
        .auto_resolve()
        .expect("resolve reveal / add / trash / place-self");

    // The Royal Knight card landed in hand.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.iter().any(|id| id == "RK-CARD"),
        "the [Royal Knight] card must be added to hand; hand={hand_ids:?}"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "ST12-15"),
        "ST12-15 must leave the hand when placed in the battle area; hand={hand_ids:?}"
    );

    // The other 2 revealed cards (FILL, FILL) are trashed.
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the 2 non-matching revealed cards must be trashed"
    );

    // ST12-15 is placed in the battle area as a delayed Option.
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "ST12-15")
        .expect("ST12-15 must be placed as a battle-area Delay Option after [Main]");
    assert!(
        matches!(
            placed.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        ),
        "ST12-15 must be a MainPhaseActivated delayed Option, got {:?}",
        placed.option_state
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] <Delay> next-digivolve cost reduction
//             (G-DELAY-NEXT-DIGIVOLVE-COST-REDUCTION)
// ═══════════════════════════════════════════════════════════════════════════

/// Activating the <Delay> arms a free (no-suspend), any-color, single-fire
/// digivolve cost reducer; the NEXT digivolution this turn auto-applies the -1
/// with NO `pending_selection` prompt, pays `printed_cost - 1`, and consumes
/// the reducer.
#[test]
fn st12_15_delay_reduces_next_digivolution_cost_by_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-15")
        .expect("ST12-15 must load from embedded DSL pack")
        .add_card(lvl3_base("RBASE", CardColor::Red))
        .add_card(lvl4_evo3("REVO", CardColor::Red, EVO_RED, 3))
        .add_card(filler("FILL"))
        .hand(0, &["REVO"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "RBASE", Some(0)).index as usize;
    let delay_idx = seat_delay(&mut runner, 0);

    // Activate the <Delay> via the FIELD_EFFECT main slot.
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    let bit = delay_main_bit(delay_idx);
    use digimon_engine::action::mask::build_action_mask;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit as usize],
        1.0,
        "ST12-15 <Delay> activation must be a legal main-phase action"
    );
    runner.game.decode_action(bit, 0);
    runner.game.drain_effect_queue();

    // The reducer is armed: amount 1, single-fire, any color, NO suspend cost.
    assert_eq!(
        runner.game.player_digivolve_cost_reducers.len(),
        1,
        "the <Delay> body must arm exactly one player-scoped reducer"
    );
    let reducer = &runner.game.player_digivolve_cost_reducers[0];
    assert_eq!(reducer.player, 0);
    assert_eq!(reducer.amount, 1, "the reduction is -1");
    assert!(
        reducer.single_fire,
        "\"next time ... this turn\" is single-fire"
    );
    assert!(
        reducer.target_color.is_none(),
        "the reduction applies to a digivolution of ANY color Digimon"
    );
    assert!(
        !reducer.suspend_cost,
        "the reduction is FREE — no suspend cost"
    );

    // ST12-15 was trashed as the <Delay> activation cost.
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST12-15"),
        "ST12-15 is trashed as the <Delay> activation cost"
    );

    // Now digivolve RBASE -> REVO (printed cost 3). The free reducer must
    // AUTO-APPLY with NO prompt — the key RED assertion.
    // After trashing ST12-15 the base may have shifted; find RBASE's slot.
    let base_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "RBASE")
        .expect("RBASE still on field");
    let _ = base;
    let memory_before = runner.memory();

    let ok = runner
        .game
        .digivolve_from_hand(0, 0, base_idx, PlaySource::ByDigivolve);
    assert!(
        ok,
        "free reducer must auto-apply with NO parked selection — digivolution completes outright"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "a FREE digivolve cost reducer must NOT install an accept/decline prompt"
    );
    assert_eq!(
        runner.memory(),
        memory_before - 2,
        "printed digivolution cost 3 reduced by 1 should pay exactly 2 memory"
    );
    assert!(
        runner.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer is consumed after the first digivolution"
    );
}

/// Single-fire: a SECOND digivolution in the same turn pays full price (the
/// reducer is one-shot, consumed by the first qualifying digivolve).
#[test]
fn st12_15_delay_reduction_fires_only_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-15")
        .expect("ST12-15 must load from embedded DSL pack")
        .add_card(lvl3_base("RBASE", CardColor::Red))
        .add_card(lvl4_evo3("REVO", CardColor::Red, EVO_RED, 3))
        .add_card(lvl4_evo3("REVO2", CardColor::Red, EVO_RED, 3))
        .add_card(filler("FILL"))
        .hand(0, &["REVO", "REVO2"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(20)
        .start();

    let _base_a = runner.place_on_field(0, "RBASE", Some(0)).index as usize;
    let _base_b = runner.place_on_field(0, "RBASE", Some(0)).index as usize;
    let delay_idx = seat_delay(&mut runner, 0);

    runner.game.enter_main_phase();
    runner.game.set_memory(20);
    runner.game.decode_action(delay_main_bit(delay_idx), 0);
    runner.game.drain_effect_queue();
    assert_eq!(runner.game.player_digivolve_cost_reducers.len(), 1);

    // First digivolution — reduced.
    let base_a_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "RBASE")
        .expect("first RBASE");
    let mem0 = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base_a_idx, PlaySource::ByDigivolve));
    assert_eq!(
        runner.memory(),
        mem0 - 2,
        "first digivolution gets the -1 (cost 3 -> pays 2)"
    );
    assert!(
        runner.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer consumed"
    );

    // Second digivolution same turn — full price.
    let base_b_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "RBASE")
        .expect("second RBASE");
    let mem1 = runner.memory();
    // REVO was consumed → REVO2 is now hand index 0.
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base_b_idx, PlaySource::ByDigivolve));
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.memory(),
        mem1 - 3,
        "second digivolution pays the FULL printed cost 3 — reducer is one-shot"
    );
}

/// "this turn" — an unused reducer is dropped at end of turn.
#[test]
fn st12_15_delay_reduction_expires_unused_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-15")
        .expect("ST12-15 must load from embedded DSL pack")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let delay_idx = seat_delay(&mut runner, 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.decode_action(delay_main_bit(delay_idx), 0);
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.game.player_digivolve_cost_reducers.len(),
        1,
        "the <Delay> arms the reducer"
    );

    runner.end_turn();
    assert!(
        runner.game.player_digivolve_cost_reducers.is_empty(),
        "an unused \"this turn\" reducer is dropped at end of the installing player's turn"
    );
}
