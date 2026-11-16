//! BT22-052 Leopardmon

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt22_052_loads_play_and_blocker_slice() {
    DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt22_052_has_blast_digivolve_marker_and_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT22-052").expect("compiled card");

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BurstDigivolve
            && path.marker
            && path.cost == Some(CompiledCost::Literal(0))
    }));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "BlastDigivolve"
    )));
}

#[test]
fn bt22_052_other_digimon_would_leave_gains_memory_and_event_proceeds() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .add_card(digimon("ALLY-A", "Ally A"))
        .add_card(digimon("ALLY-B", "Ally B"))
        .memory(0)
        .start();
    let _leopardmon = runner.place_on_field(0, "BT22-052", Some(0));
    let ally_a = runner.place_on_field(0, "ALLY-A", Some(0));
    let ally_b = runner.place_on_field(0, "ALLY-B", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally_a, ReplacementCause::OpponentEffect);
    runner
        .auto_resolve()
        .expect("resolve non-cancelling would-leave subscriber");

    assert_eq!(
        runner.memory(),
        2,
        "Leopardmon gains 2 memory when another Digimon would leave"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "ALLY-A"),
        "the original leave event still proceeds"
    );

    runner
        .game
        .delete_permanent_with_cause(ally_b, ReplacementCause::OpponentEffect);
    runner.auto_resolve().expect("resolve second leave attempt");
    assert_eq!(
        runner.memory(),
        2,
        "the once-per-turn subscriber does not gain memory twice"
    );
}

fn digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card
}

// ─── [On Play][When Digivolving] hand-pick legality ─────────────────────────
// DCGO BT22_052.cs:43-57 `CanSelectCardCondition` (IsExistOnHand && IsDigimon &&
// DP <= 5000) wrapped by `CanSelectCondition` (:86-94 On Play, :164-172 When
// Digivolving) `&& CardEffectCommons.CanPlayAsNewPermanent(cardSource, false,
// activateClass)`. CanPlayAsNewPermanent (GameContextDeterminarion.cs:419-432)
// requires `!IsOption` (so a DUAL is out — CR 4.3 7-1-1: playing needs a play
// cost, a DUAL prints "D"; 4-6-2), an empty frame, and a play the card may make (no prohibition).
// YAML: `can_play_from_effect_without_cost: true`.

fn leopard_runner(hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .dsl_card("EX13-065")
        .expect("EX13-065 YAML loads")
        .add_card(small("SMALL-5000"))
        .hand(0, hand)
        .memory(0)
        .start();
    let leopard = runner.place_on_field(0, "BT22-052", Some(0));
    (runner, leopard)
}

fn small(id: &str) -> CardData {
    let mut card = digimon(id, id);
    card.level = Some(3);
    card.dp = Some(5000);
    card.play_cost = 3;
    card
}

/// Fire [On Play] and return the card ids the hand pick offers (empty when no
/// hand pick installs).
fn offered_on_play(
    runner: &mut DebugRunner,
    leopard: digimon_engine::permanent::PermanentHandle,
) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::{SelectionKind, TriggerSource};
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(leopard));
    runner.game.drain_effect_queue();
    for _ in 0..4 {
        match runner.pending_kind() {
            Some(SelectionKind::Hand) => {
                let hand: Vec<String> = runner.game.players[0]
                    .hand
                    .iter()
                    .map(|c| c.card_id(&runner.game.card_data).to_string())
                    .collect();
                let view = runner.pending_selection_view().expect("hand pick");
                return view
                    .valid_action_ids
                    .iter()
                    .copied()
                    .filter(|&id| id != PASS)
                    .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
                    .collect();
            }
            Some(_) if runner.pending_is_optional() => {
                runner.accept_optional_trigger().expect("accept optional prompt")
            }
            _ => return Vec::new(),
        }
    }
    Vec::new()
}

/// Sistermon Blanc (Awakened) (EX13-065) is a 5000 DP DUAL: it can't be played
/// (CR 4.3 7-1-1: play cost "D"; 4-6-2), so only the plain 5000 DP Digimon is offered.
#[test]
fn bt22_052_on_play_does_not_offer_dual_5000_dp_card() {
    let (mut runner, leopard) = leopard_runner(&["EX13-065", "SMALL-5000"]);
    let offered = offered_on_play(&mut runner, leopard);
    assert_eq!(
        offered,
        vec!["SMALL-5000".to_string()],
        "the DUAL Sistermon Blanc (Awakened) (EX13-065) must not be offered for a play"
    );
}

/// "You can't play Digimon by effects": nothing is offered.
#[test]
fn bt22_052_on_play_offers_nothing_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, leopard) = leopard_runner(&["SMALL-5000"]);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_on_play(&mut runner, leopard);
    assert!(offered.is_empty(), "play prohibited ⇒ no candidate: {offered:?}");
}

/// A full battle area: nothing is offered.
#[test]
fn bt22_052_on_play_offers_nothing_when_battle_area_full() {
    let (mut runner, leopard) = leopard_runner(&["SMALL-5000"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    let offered = offered_on_play(&mut runner, leopard);
    assert!(offered.is_empty(), "full battle area ⇒ no candidate: {offered:?}");
}
