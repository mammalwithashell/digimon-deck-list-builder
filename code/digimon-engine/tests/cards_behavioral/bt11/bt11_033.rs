//! BT11-033 MirageGaogamon — Digimon, Lv.6, Blue, DP 12000, Cost 12, [Beast Knight].
//!
//! Printed effect text (authoritative):
//!   [When Digivolving] Return 1 of your opponent's level 5 or lower Digimon to
//!   its owner's hand. If no Digimon was returned by this effect, your opponent
//!   adds the top card of their security stack to their hand.
//!   [All Turns] [Once Per Turn] When an effect adds cards to your opponent's
//!   hand, gain 1 memory for every 4 cards in your opponent's hand.
//!
//! (No inherited effect, no security effect.)
//!
//! DCGO behavioral reference:
//!   DCGO/Assets/Scripts/CardEffect/BT11/Blue/BT11_033.cs
//!
//! Tags: when_digivolving, return_to_hand, conditional-fallback, security-to-hand,
//!       memory-arithmetic (Q10/Q11), all_turns, once_per_turn, observer.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! Clause decomposition
//! ─────────────────────────────────────────────────────────────────────────────
//! Clause 1 — [When Digivolving] (IMPLEMENTED):
//!   * Select 1 of the OPPONENT's level-5-or-lower Digimon and return it to its
//!     owner's hand. DCGO `CanSelectPermanentCondition`: opp battle-area Digimon
//!     with `Level <= 5` and `TopCard.HasLevel`. Mandatory pick when a target
//!     exists (DCGO `canNoSelect: false`, `maxCount = min(1, matchCount)`).
//!   * If NO Digimon was returned (none eligible / none existed), the opponent
//!     adds the top card of THEIR security stack to THEIR hand. DCGO: guarded by
//!     `!bounced()` then `SecurityCards[0]` → AddHandCards on the enemy + reduce
//!     their security. Modeled with `effect_returned_any_card: false`.
//!
//! Clause 2 — [All Turns][Once Per Turn] memory observer (BLOCKED):
//!   "When an EFFECT adds cards to your opponent's hand, gain 1 memory for every
//!    4 cards in your opponent's hand." DCGO: `EffectTiming.OnAddHand`, gated by
//!    `CanTriggerWhenAddHand(player => player == Owner.Enemy, cardEffect =>
//!    cardEffect != null)` — i.e. effect-initiated adds to the OPPONENT's hand
//!    only, not normal draws. Reward = `Owner.Enemy.HandCards.Count / 4` (integer
//!    floor, counted at trigger time). OPT-locked via SetHashString.
//!
//!    GAP (gap_kind: engine-primitive + dsl-vocab): the engine defines
//!    `EffectTiming::OnAddToHand` but NEVER dispatches it, and the DSL
//!    `TriggerWhen` enum has no `on_add_to_hand` token. There is no way to
//!    observe "an effect added cards to the opponent's hand". The memory
//!    ARITHMETIC is fully composable today (`gain_memory_fn` + a `floor_div`
//!    of `card_count_in_zone { zone: hand, of: opponent }` by 4), so only the
//!    TRIGGER is missing. Clause 2 is OMITTED from the YAML per the
//!    no-approximations policy; the math/OPT tests below are `#[ignore]`'d with
//!    this gap ID until the observer timing is wired. See
//!    `docs/RUST_ENGINE_GAPS.md` (G-ON-ADD-TO-HAND-OBSERVER).

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPlayerRef, CompiledStep,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT11-033";
const GAP_ON_ADD_TO_HAND: &str =
    "G-ON-ADD-TO-HAND-OBSERVER: EffectTiming::OnAddToHand is never dispatched and the DSL \
     has no on_add_to_hand trigger token; clause 2 (memory-per-4-cards observer) is blocked.";

// ─── Card fixtures ──────────────────────────────────────────────────────────

fn opp_digimon(card_id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card_with_level(card_id, card_id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn bt11_033_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT11-033 YAML loads")
        .add_card(opp_digimon("OPP-LV3", 3))
        .add_card(opp_digimon("OPP-LV5", 5))
        .add_card(opp_digimon("OPP-LV6", 6))
        .add_card(make_test_card("OPP-SEC-1", "OPP-SEC-1"))
        .add_card(make_test_card("OPP-SEC-2", "OPP-SEC-2"))
        .add_card(make_test_card("HAND-FILLER", "HAND-FILLER"))
}

/// Set `player`'s hand to exactly `target` HAND-FILLER cards (registered in the
/// pool by `bt11_033_runner`).
fn set_hand_size(r: &mut DebugRunner, player: PlayerId, target: usize) {
    use digimon_engine::card_source::CardSource;
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "HAND-FILLER")
        .expect("HAND-FILLER registered");
    r.game.players[player as usize].hand.clear();
    for _ in 0..target {
        let next_idx = r.game.next_card_index();
        r.game.players[player as usize]
            .hand
            .push(CardSource::new(data_idx, player, next_idx));
    }
}

/// Fire an EFFECT-driven add to `player`'s hand (the `OnAddToHand` observer
/// timing) without going through a specific add verb — mirrors how
/// `fire_when_digivolving` fires its trigger directly.
fn fire_effect_add_to_hand(r: &mut DebugRunner, player: PlayerId) {
    r.game.enqueue_triggered(
        EffectTiming::OnAddToHand,
        TriggerSource::HandGained {
            player,
            effect_initiated: true,
        },
    );
    r.game.drain_effect_queue();
}

fn hand_contains(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn field_contains(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

/// Fire BT11-033's [When Digivolving] trigger on a permanent already on the
/// field (mirrors the production digivolve flow's `WhenDigivolving` enqueue).
fn fire_when_digivolving(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt11_033_structural_identity_and_when_digivolving_clause() {
    let runner = bt11_033_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT11-033 compiled card present");

    assert_eq!(compiled.card, "BT11-033");
    assert_eq!(compiled.name, "MirageGaogamon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert!(
        compiled.traits.iter().any(|t| t == "Beast Knight"),
        "BT11-033 must carry the [Beast Knight] trait"
    );

    // Clause 1 — the [When Digivolving] triggered clause.
    let wd = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::WhenDigivolving] => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT11-033 must have a [When Digivolving] triggered clause");

    // Step a — select an opponent's level-5-or-lower Digimon and return it.
    assert!(
        wd.process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectOpponentPermanent {
                optional: false,
                ..
            }
        )),
        "clause 1 must mandatorily select an opponent Digimon (DCGO canNoSelect: false)"
    );
    assert!(
        wd.process
            .iter()
            .any(|step| matches!(step, CompiledStep::ReturnToHand { .. })),
        "clause 1 must return the selected Digimon to hand"
    );

    // Step b — if nothing returned, opponent adds top security to their hand.
    let if_step = wd
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::If {
                condition, then, ..
            } if condition.effect_returned_any_card == Some(false) => Some(then),
            _ => None,
        })
        .expect("clause 1 must branch on 'no Digimon returned' (effect_returned_any_card: false)");
    assert!(
        if_step.iter().any(|step| matches!(
            step,
            CompiledStep::AddTopSecurityToHand {
                of: CompiledPlayerRef::Opponent
            }
        )),
        "the fallback must add the OPPONENT's top security to the OPPONENT's hand"
    );
}

#[test]
fn bt11_033_authors_the_on_add_to_hand_memory_observer() {
    // G-ON-ADD-TO-HAND-OBSERVER resolved (2026-06-04): clause 2 is now an
    // `on_add_to_hand` observer carrying a `gain_memory` step. (This replaces the
    // earlier guard that asserted the clause was ABSENT while the gap was open.)
    let runner = bt11_033_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT11-033 compiled card present");

    let observer = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnAddToHand] => Some(t),
            _ => None,
        })
        .expect("clause 2 must be an `on_add_to_hand` observer");
    assert!(
        observer.once_per_turn,
        "the memory observer is [Once Per Turn]"
    );
    assert!(
        observer
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemoryFn { .. })),
        "the observer must gain memory via a formula step"
    );
    let _ = GAP_ON_ADD_TO_HAND;
}

// ─── Behavioral — Clause 1 POSITIVE (return-to-hand) ────────────────────────

#[test]
fn bt11_033_when_digivolving_returns_a_level5_or_lower_opp_digimon_to_hand() {
    let mut runner = bt11_033_runner()
        .security(1, &["OPP-SEC-1", "OPP-SEC-2"])
        .start();

    runner.place_on_field(1, "OPP-LV5", Some(0));
    runner.place_on_field(1, "OPP-LV6", Some(0));
    let mirage = runner.place_on_field(0, CARD_ID, Some(0));

    let opp_sec_before = runner.security_count(1);
    let opp_hand_before = runner.game.players[1].hand.len();

    fire_when_digivolving(&mut runner, mirage);

    // The pick must offer ONLY the Lv.5 (Lv.6 is ineligible). Resolve it.
    let view = runner
        .pending_selection_view()
        .expect("[When Digivolving] must install an opponent-Digimon return selection");
    let non_pass: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "only the level-5-or-lower opponent Digimon (the Lv.5) should be selectable; got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, non_pass[0])
        .expect("return the Lv.5 opponent Digimon to hand");
    let _ = runner.auto_resolve();

    assert!(
        !field_contains(&runner, 1, "OPP-LV5"),
        "the returned Lv.5 Digimon must leave the opponent's field"
    );
    assert!(
        hand_contains(&runner, 1, "OPP-LV5"),
        "the returned Lv.5 Digimon must be in its owner's (opponent's) hand"
    );
    assert!(
        field_contains(&runner, 1, "OPP-LV6"),
        "the Lv.6 Digimon is ineligible and must remain on field"
    );
    // A Digimon WAS returned → the security fallback must NOT fire.
    assert_eq!(
        runner.security_count(1),
        opp_sec_before,
        "when a Digimon is returned, the opponent's security must be untouched"
    );
    assert!(
        !hand_contains(&runner, 1, "OPP-SEC-1"),
        "the security fallback must not fire when a Digimon was returned"
    );
    assert_eq!(
        runner.game.players[1].hand.len(),
        opp_hand_before + 1,
        "exactly the returned Digimon should have entered the opponent's hand"
    );
}

// ─── Behavioral — Clause 1 NEGATIVE (no return → security to hand) ──────────

#[test]
fn bt11_033_when_digivolving_with_no_eligible_target_adds_opp_top_security_to_hand() {
    let mut runner = bt11_033_runner()
        .security(1, &["OPP-SEC-1", "OPP-SEC-2"])
        .start();

    // Only a Lv.6 on the opponent's field → no level-5-or-lower target.
    runner.place_on_field(1, "OPP-LV6", Some(0));
    let mirage = runner.place_on_field(0, CARD_ID, Some(0));

    // The `.security()` builder pushes in array order, so the LAST id is the
    // top of the stack: OPP-SEC-1 (bottom), OPP-SEC-2 (top).
    let opp_sec_before = runner.security_count(1);
    assert!(
        !hand_contains(&runner, 1, "OPP-SEC-2"),
        "precondition: top security not yet in opponent hand"
    );

    fire_when_digivolving(&mut runner, mirage);
    let _ = runner.auto_resolve();

    // No Digimon returned → opponent adds the TOP of their OWN security to hand.
    assert_eq!(
        runner.security_count(1),
        opp_sec_before - 1,
        "with no return, the opponent's top security card must move to their hand"
    );
    assert!(
        hand_contains(&runner, 1, "OPP-SEC-2"),
        "the opponent's TOP security card (OPP-SEC-2) must be added to their hand"
    );
    assert!(
        !hand_contains(&runner, 1, "OPP-SEC-1"),
        "only the top security card moves; the bottom card (OPP-SEC-1) stays in security"
    );
    assert!(
        field_contains(&runner, 1, "OPP-LV6"),
        "the ineligible Lv.6 Digimon stays on the field"
    );
}

#[test]
fn bt11_033_when_digivolving_with_empty_opp_board_adds_opp_top_security_to_hand() {
    let mut runner = bt11_033_runner().security(1, &["OPP-SEC-1"]).start();

    // No opponent Digimon at all → nothing returned → security fallback fires.
    let mirage = runner.place_on_field(0, CARD_ID, Some(0));

    fire_when_digivolving(&mut runner, mirage);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "empty opponent board → top security moves to opponent hand"
    );
    assert!(
        hand_contains(&runner, 1, "OPP-SEC-1"),
        "the opponent's only security card must be added to their hand"
    );
}

// ─── Behavioral — Clause 2 memory arithmetic (BLOCKED; gap-gated) ───────────
//
// These encode the printed/DCGO contract for the [All Turns][Once Per Turn]
// memory observer: floor(opp_hand_size / 4) memory each time an EFFECT adds
// cards to the opponent's hand, once per turn, cleared at end of turn.
// They are `#[ignore]`'d because the OnAddToHand observer timing is not wired
// (see GAP_ON_ADD_TO_HAND). The arithmetic boundaries (3→0, 4→1, 8→2) and the
// OPT lockout are the load-bearing assertions to honor when the gap closes.

#[test]
fn bt11_033_memory_gain_is_floor_of_opp_hand_over_four() {
    // 3 cards → 0 memory; 4 → 1; 8 → 2. The observer reads the OPPONENT's hand
    // size AT TRIGGER TIME and gains floor(size / 4).
    for (hand_size, expected_gain) in [(3usize, 0i16), (4, 1), (8, 2)] {
        let mut r = bt11_033_runner().memory(0).start();
        r.set_first_player(0);
        // BT11-033 belongs to player 0; player 1 is the opponent whose hand is
        // watched.
        r.place_on_field(0, CARD_ID, Some(0));
        set_hand_size(&mut r, 1, hand_size);

        let mem_before = r.memory();
        fire_effect_add_to_hand(&mut r, 1);

        assert_eq!(
            (r.memory() - mem_before).abs(),
            expected_gain,
            "opp hand {hand_size} → gain floor({hand_size}/4) = {expected_gain}"
        );
    }
}

#[test]
fn bt11_033_memory_observer_does_not_fire_for_own_hand_gain() {
    // The observer gates on the OPPONENT's hand (`event_add_to_hand_player:
    // opponent`); an effect adding to the controller's OWN hand must NOT gain.
    let mut r = bt11_033_runner().memory(0).start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    set_hand_size(&mut r, 0, 8); // controller's own hand large

    let mem_before = r.memory();
    fire_effect_add_to_hand(&mut r, 0); // add to the controller's OWN hand

    assert_eq!(
        r.memory(),
        mem_before,
        "an add to the controller's own hand must not trigger the opponent-hand observer"
    );
}

#[test]
fn bt11_033_memory_observer_is_once_per_turn_and_clears_after_end_turn() {
    // First effect-add this turn gains memory; a second effect-add the same turn
    // gains nothing (OPT lockout). After end_turn, the lockout clears.
    let mut r = bt11_033_runner().memory(0).start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    set_hand_size(&mut r, 1, 4); // floor(4/4) = 1 each fire

    let m0 = r.memory();
    fire_effect_add_to_hand(&mut r, 1);
    let after_first = r.memory();
    assert_eq!(
        (after_first - m0).abs(),
        1,
        "the first effect-add this turn gains floor(4/4)=1"
    );

    // Second add the SAME turn — OPT lockout, no further gain.
    fire_effect_add_to_hand(&mut r, 1);
    assert_eq!(
        r.memory(),
        after_first,
        "a second effect-add the same turn gains nothing (Once Per Turn lockout)"
    );

    // The lockout is TURN-SCOPED: clearing the per-turn activation counters
    // (exactly what `Player::new_turn` does at the start of the owner's next
    // turn) restores the gain. This isolates the turn-scope of the OPT from the
    // non-deterministic first-player/seed turn sequencing.
    for perm in &mut r.game.players[0].battle_area {
        perm.new_turn();
    }
    set_hand_size(&mut r, 1, 4);
    r.game.memory = 0; // clean baseline so a ±1 gain is not memory-capped
    let m_reset = r.memory();
    fire_effect_add_to_hand(&mut r, 1);
    assert_eq!(
        (r.memory() - m_reset).abs(),
        1,
        "once the per-turn OPT counter resets (new turn), the observer gains again"
    );
}
