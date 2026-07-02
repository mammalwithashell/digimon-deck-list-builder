//! EX10-068 Digimon Emperor — Tamer, cost 5. Also treated as [Ken Ichijoji].
//!
//! # Card text (cards.json / card image — verbatim)
//!
//! [Start of Your Main Phase] For every 2 colors your opponent's Digimon and
//!   Tamers have, gain 1 memory.
//! [On Play] Delete 1 of your opponent's play cost 5 or lower Digimon. Then, by
//!   returning 1 Digimon card from your opponent's trash to the bottom of the
//!   deck, from your hand or trash and without paying the cost, you may play 1
//!   level 4 or lower Digimon card with the same color as the card this effect
//!   returned.
//! [Security] Play this card without paying the cost.
//!
//! # Patterns covered
//! - [On Play] mandatory delete of a play-cost-5-or-lower opponent Digimon.
//! - [On Play] tail: optional return of an opponent-trash Digimon to deck
//!   bottom, then optional same-color Lv.4-or-lower play from hand/trash free.
//!   The play candidate gate keys on the RETURNED card's color
//!   (G-RETURNED-CARD-COLOR-BINDING — `color_matches_returned_card`).
//! - [Start of Your Main Phase] distinct-color memory: +1 memory per 2 distinct
//!   colors across the opponent's Digimon + Tamers (G-DISTINCT-COLOR-COUNT —
//!   `distinct_colors_count` + `gain_memory_fn` + `floor_div`).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Card data helpers ──────────────────────────────────────────────────────

/// A Digimon with an explicit single color, level, and play cost.
fn make_digimon(id: &str, color: CardColor, level: u8, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Digimon"));
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = play_cost;
    c
}

/// A Tamer with an explicit single color.
fn make_tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Tamer"));
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

/// Push a registered card_id into player `player`'s trash. Returns the handle.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered"));
    let idx = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, idx));
}

/// Push a registered card_id into player `player`'s hand.
fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered"));
    let idx = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, idx));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex10_068_loads_delete_and_security_play_slice() {
    DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 must load from embedded DSL pack")
        .start();
}

/// EX10-068 must carry a Start-of-Your-Main-Phase clause and an On Play clause
/// in addition to the Security clause — i.e. the two previously-stubbed clauses
/// are now authored.
#[test]
fn ex10_068_has_start_main_and_on_play_clauses() {
    use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
    let runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("EX10-068")
        .expect("EX10-068 compiled card present");

    let has_start_main = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::StartOfYourMainPhase))
    });
    assert!(
        has_start_main,
        "EX10-068 must carry a Start-of-Your-Main-Phase distinct-color memory clause"
    );

    let has_on_play = card.effects.iter().any(
        |c| matches!(c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)),
    );
    assert!(
        has_on_play,
        "EX10-068 must carry an On Play delete/return/play clause"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] delete (already implemented; lock it in)
// ═══════════════════════════════════════════════════════════════════════════

/// The On Play delete targets only opponent Digimon with play cost 5 or lower.
#[test]
fn ex10_068_on_play_deletes_play_cost_5_or_lower_opponent_digimon() {
    let victim = make_digimon("VICTIM-5", CardColor::Red, 5, 5);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .add_card(victim)
        .memory(10)
        .build();

    let opp = runner.place_on_field(1, "VICTIM-5", Some(0));
    let emperor = runner.place_on_field(0, "EX10-068", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(emperor));
    runner.game.drain_effect_queue();

    // The delete is mandatory and the only legal target is the cost-5 Digimon.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "On Play delete must install an OppField target selection"
    );
    runner.auto_resolve().expect("delete target resolves");

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .get(opp.index as usize)
            .is_none()
            || runner.game.player(1).battle_area.is_empty(),
        "the play-cost-5 opponent Digimon must be deleted"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] tail: returned-card color gates the play
//             (G-RETURNED-CARD-COLOR-BINDING)
// ═══════════════════════════════════════════════════════════════════════════

/// Full On Play flow: delete → return a Blue opponent-trash Digimon → the play
/// selection offers ONLY same-color (Blue) Lv.4-or-lower candidates across hand
/// AND trash, and EXCLUDES the color-mismatched (Red) hand card.
#[test]
fn ex10_068_returned_card_color_gates_play_tail() {
    let victim = make_digimon("VICTIM", CardColor::Green, 4, 4);
    let returned_blue = make_digimon("RET-BLUE", CardColor::Blue, 5, 6); // opp trash
    let hand_blue = make_digimon("HAND-BLUE", CardColor::Blue, 4, 4); // legal play
    let hand_red = make_digimon("HAND-RED", CardColor::Red, 4, 4); // illegal (wrong color)
    let trash_blue = make_digimon("TRASH-BLUE", CardColor::Blue, 3, 3); // legal (union)

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .add_card(victim)
        .add_card(returned_blue)
        .add_card(hand_blue)
        .add_card(hand_red)
        .add_card(trash_blue)
        .memory(10)
        .build();

    let _opp = runner.place_on_field(1, "VICTIM", Some(0));
    push_to_trash(&mut runner, 1, "RET-BLUE");
    push_to_hand(&mut runner, 0, "HAND-BLUE");
    push_to_hand(&mut runner, 0, "HAND-RED");
    push_to_trash(&mut runner, 0, "TRASH-BLUE");

    let emperor = runner.place_on_field(0, "EX10-068", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(emperor));
    runner.game.drain_effect_queue();

    // Prompt 1: mandatory delete (OppField).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "first prompt is the mandatory delete"
    );
    runner.auto_resolve_one().expect("resolve delete");

    // Prompt 2: optional return from opponent trash (Trash kind, opponent's).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "second prompt is the opponent-trash return (the only returnable Digimon is RET-BLUE)"
    );
    // Exactly one returnable card (RET-BLUE); pick it.
    let ret_view = runner
        .pending_selection_view()
        .expect("return selection present");
    // valid_action_ids should include the single RET-BLUE pick (plus possibly PASS for optional).
    let pick = ret_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("a concrete return target must be offered");
    runner
        .execute_action(0, pick)
        .expect("return RET-BLUE to deck bottom");

    // Prompt 3: optional union-zone play, color-gated by the returned Blue card.
    let play_kind = runner.pending_kind().expect("play selection must install");
    assert!(
        matches!(play_kind, SelectionKind::UnionZone { .. }),
        "third prompt is a hand/trash union play, got: {play_kind:?}"
    );
    let play_view = runner
        .pending_selection_view()
        .expect("play selection present");
    let concrete: Vec<u16> = play_view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    // HAND-BLUE + TRASH-BLUE are legal; HAND-RED is excluded → exactly 2.
    assert_eq!(
        concrete.len(),
        2,
        "only the two Blue Lv.4-or-lower cards (hand + trash) may be played; the Red hand card is gated out (got {} concrete candidates)",
        concrete.len()
    );

    // Pick the first legal candidate and confirm a Digimon was played.
    let field_before = runner.battle_area_size(0);
    runner
        .execute_action(0, concrete[0])
        .expect("play a same-color candidate");
    runner.auto_resolve().ok();
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "playing a same-color Lv.4 card must add a permanent to the controller's field"
    );

    // The returned Blue card must be on the bottom of the opponent's deck.
    let opp_deck = &runner.game.player(1).deck;
    let bottom = opp_deck
        .first()
        .map(|cs| cs.card_id(&runner.game.card_data).to_string());
    assert_eq!(
        bottom.as_deref(),
        Some("RET-BLUE"),
        "the returned card must be the bottom card of the opponent's deck"
    );
}

/// Negative: when the returned card is Blue but only a Red Lv.4 card is
/// available in hand/trash, the color gate offers NO concrete play candidate
/// (the play is skippable to a no-op).
#[test]
fn ex10_068_wrong_color_only_offers_no_play() {
    let victim = make_digimon("VICTIM-N", CardColor::Green, 4, 4);
    let returned_blue = make_digimon("RET-BLUE-N", CardColor::Blue, 5, 6);
    let hand_red = make_digimon("HAND-RED-N", CardColor::Red, 4, 4); // wrong color

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .add_card(victim)
        .add_card(returned_blue)
        .add_card(hand_red)
        .memory(10)
        .build();

    let _opp = runner.place_on_field(1, "VICTIM-N", Some(0));
    push_to_trash(&mut runner, 1, "RET-BLUE-N");
    push_to_hand(&mut runner, 0, "HAND-RED-N");

    let emperor = runner.place_on_field(0, "EX10-068", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(emperor));
    runner.game.drain_effect_queue();

    // delete
    runner.auto_resolve_one().expect("resolve delete");

    // return RET-BLUE-N
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    let ret_view = runner.pending_selection_view().unwrap();
    let pick = ret_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("a return target must be offered");
    runner
        .execute_action(0, pick)
        .expect("return the Blue card");

    // The play either does not install (no candidates) or installs optional with
    // no concrete candidate — in both cases the controller plays nothing.
    let field_before = runner.battle_area_size(0);
    if let Some(kind) = runner.pending_kind() {
        assert!(
            matches!(kind, SelectionKind::UnionZone { .. }),
            "if a prompt installs it must be the union-zone play, got: {kind:?}"
        );
        let view = runner.pending_selection_view().unwrap();
        let concrete = view
            .valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count();
        assert_eq!(
            concrete, 0,
            "no concrete play candidate may be offered when only a wrong-color card is available"
        );
        // Decline (PASS) to finish.
        runner.auto_resolve().ok();
    }
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no Digimon may be played when only a color-mismatched card is available"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Start of Your Main Phase] distinct-color memory
//             (G-DISTINCT-COLOR-COUNT)
// ═══════════════════════════════════════════════════════════════════════════

/// 3 distinct colors across the opponent's Digimon + Tamers → +1 memory
/// (floor(3 / 2) = 1).
#[test]
fn ex10_068_gains_one_memory_per_two_opponent_colors() {
    let red_d = make_digimon("OPP-RED-D", CardColor::Red, 4, 4);
    let blue_d = make_digimon("OPP-BLUE-D", CardColor::Blue, 4, 4);
    let green_t = make_tamer("OPP-GREEN-T", CardColor::Green);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .add_card(red_d)
        .add_card(blue_d)
        .add_card(green_t)
        .memory(0)
        .build();

    let emperor = runner.place_on_field(0, "EX10-068", None);
    runner.place_on_field(1, "OPP-RED-D", Some(0));
    runner.place_on_field(1, "OPP-BLUE-D", Some(0));
    runner.place_on_field(1, "OPP-GREEN-T", Some(0));

    let memory_before = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(emperor),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    // Memory moves toward the controller (player 0) by +1.
    assert_eq!(
        memory_signed_gain(memory_before, runner.memory()),
        1,
        "3 distinct opponent colors must grant floor(3/2)=1 memory (before {memory_before}, after {})",
        runner.memory()
    );
}

/// 4 distinct colors → +2 memory (floor(4 / 2) = 2).
#[test]
fn ex10_068_gains_two_memory_for_four_opponent_colors() {
    let red_d = make_digimon("OPP4-RED", CardColor::Red, 4, 4);
    let blue_d = make_digimon("OPP4-BLUE", CardColor::Blue, 4, 4);
    let green_d = make_digimon("OPP4-GREEN", CardColor::Green, 4, 4);
    let yellow_t = make_tamer("OPP4-YELLOW", CardColor::Yellow);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .add_card(red_d)
        .add_card(blue_d)
        .add_card(green_d)
        .add_card(yellow_t)
        .memory(0)
        .build();

    let emperor = runner.place_on_field(0, "EX10-068", None);
    runner.place_on_field(1, "OPP4-RED", Some(0));
    runner.place_on_field(1, "OPP4-BLUE", Some(0));
    runner.place_on_field(1, "OPP4-GREEN", Some(0));
    runner.place_on_field(1, "OPP4-YELLOW", Some(0));

    let memory_before = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(emperor),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert_eq!(
        memory_signed_gain(memory_before, runner.memory()),
        2,
        "4 distinct opponent colors must grant floor(4/2)=2 memory (before {memory_before}, after {})",
        runner.memory()
    );
}

/// 1 distinct color → +0 memory (floor(1 / 2) = 0), and no prompt installs.
#[test]
fn ex10_068_gains_no_memory_for_one_opponent_color() {
    let red_d = make_digimon("OPP1-RED", CardColor::Red, 4, 4);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 YAML parses and compiles")
        .add_card(red_d)
        .memory(0)
        .build();

    let emperor = runner.place_on_field(0, "EX10-068", None);
    runner.place_on_field(1, "OPP1-RED", Some(0));

    let memory_before = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(emperor),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "the distinct-color memory clause is synchronous; no prompt installs"
    );

    assert_eq!(
        memory_signed_gain(memory_before, runner.memory()),
        0,
        "1 distinct opponent color must grant floor(1/2)=0 memory"
    );
}

/// Signed memory gain *toward player 0*. The shared memory gauge is positive
/// when player 0 holds memory; a gain to player 0 increases it.
fn memory_signed_gain(before: i16, after: i16) -> i16 {
    after - before
}

// `auto_resolve_one` is a thin helper: resolve exactly the current pending
// selection by picking its first valid action (or PASS if optional-empty),
// then return, leaving any newly-installed selection pending.
trait ResolveOne {
    fn auto_resolve_one(&mut self) -> Result<(), digimon_engine::selection::SelectionError>;
}
impl ResolveOne for DebugRunner {
    fn auto_resolve_one(&mut self) -> Result<(), digimon_engine::selection::SelectionError> {
        let (player, action_id) = {
            let sel = self
                .pending_selection()
                .ok_or(digimon_engine::selection::SelectionError::NoPendingSelection)?;
            let action = match sel.valid_action_ids.first().copied() {
                Some(id) => id,
                None if sel.is_optional => digimon_engine::action::space::PASS,
                None => return Err(digimon_engine::selection::SelectionError::InvalidAction),
            };
            (sel.selecting_player, action)
        };
        self.execute_action(player, action_id)
    }
}
