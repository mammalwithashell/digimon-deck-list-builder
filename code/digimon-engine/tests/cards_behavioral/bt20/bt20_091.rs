//! BT20-091 Cool Boy - Tamer, White, LIBERATOR.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .memory(10)
        .start()
}

fn royal_knight_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, "Royal Knight Test");
    cd.traits = vec!["Royal Knight".into()];
    cd.colors = vec![CardColor::White];
    cd
}

fn omekamon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, "Omekamon");
    cd.card_kind = CardKind::Digimon;
    cd.colors = vec![CardColor::White];
    cd
}

fn field_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == card_id)
    })
}

#[test]
fn bt20_091_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    assert_eq!(card.name, "Cool Boy");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "LIBERATOR"));
}

#[test]
fn bt20_091_has_rk_play_or_digivolve_suspend_draw_memory_clause() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && triggered.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Royal Knight observer exists");
    assert!(clause.optional);
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Suspend { .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::GainMemory(1))));
}

#[test]
fn bt20_091_security_plays_self() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnSecurity)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlayFromSecurity))
        }
        _ => false,
    }));
}

#[test]
fn bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .add_card(omekamon_card("OMEKAMON-HAND"))
        .hand(0, &["OMEKAMON-HAND"])
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;

    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Cool Boy should offer the optional would-leave response");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Cool Boy response");
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("accepting asks which Omekamon to play");
    assert_eq!(pending.kind, SelectionKind::Hand);
    assert!(pending.valid_action_ids.contains(&PLAY_HAND_START));

    r.game
        .resolve_selection(0, PLAY_HAND_START)
        .expect("play Omekamon from hand");

    assert!(
        field_contains(&r, 0, "OMEKAMON-HAND"),
        "Cool Boy plays the selected Omekamon"
    );
    assert!(
        !field_contains(&r, 0, "RK-TARGET"),
        "the original Royal Knight leave event still proceeds"
    );
    assert!(
        field_contains(&r, 0, "BT20-091"),
        "Cool Boy itself remains on field"
    );
}

#[test]
fn bt20_091_decline_would_leave_response_proceeds_without_playing_omekamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .add_card(omekamon_card("OMEKAMON-HAND"))
        .hand(0, &["OMEKAMON-HAND"])
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;

    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, PASS)
        .expect("decline Cool Boy response");

    assert!(!field_contains(&r, 0, "RK-TARGET"));
    assert!(
        !field_contains(&r, 0, "OMEKAMON-HAND"),
        "decline leaves Omekamon in hand"
    );
}

#[test]
fn bt20_091_no_omekamon_in_hand_does_not_offer_response() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;

    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "without an Omekamon to play, no would-leave prompt is offered"
    );
    assert!(!field_contains(&r, 0, "RK-TARGET"));
}
