//! BT20-100 The Last Guardian - Option, White, Royal Knight.
//!
//! Supported slice:
//! - [Main] reveal top 3; add 1 [Cool Boy] and 1 [Royal Knight]/[X Antibody]
//!   trait card; bottom the rest; place self in battle area.
//! - [Security] branch over hand/trash for optional [Omekamon]/[Cool Boy] play,
//!   then place self in battle area.
//!
//! Gap-routed:
//! - Delay leave-prevention for an [Omnimon]-named Digimon needs a
//!   cross-permanent replacement that trashes the Delay option and cancels the
//!   specific would-leave event.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate, CompiledStackPosition,
    CompiledStep, CompiledTiming,
};
use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, DelayTrigger};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;
use digimon_engine::{action::space::PASS, action::space::REPLACEMENT_ACCEPT};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
        .memory(10)
        .start()
}

fn omnimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, "Omnimon Test");
    cd.colors = vec![CardColor::White];
    cd
}

fn digimon_card(card_id: &str, name: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, name);
    cd.colors = vec![CardColor::White];
    cd
}

fn place_delay_option(r: &mut DebugRunner, player: u8, card_id: &str) -> PermanentHandle {
    let handle = r.place_on_field(player, card_id, Some(0));
    r.game.player_mut(player).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: player,
            trash_on_turn: r.game.turn_count + 1,
            trigger: DelayTrigger::EndOfYourNextTurn,
            placed_on_turn: r.game.turn_count,
        };
    handle
}

fn field_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == card_id)
    })
}

fn trash_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game
        .player(player)
        .trash
        .iter()
        .any(|card| card.card_id(&r.game.card_data) == card_id)
}

struct CancelCostDeletion;
impl CardEffect for CancelCostDeletion {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Cancel cost deletion")
            .replacement_process(|rctx| {
                if matches!(rctx.cause, ReplacementCause::Cost) {
                    rctx.cancel();
                }
            })
            .build()]
    }
}

fn predicate_contains_name(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_is.as_deref() == Some(needle)
        || predicate.name_contains.as_deref() == Some(needle)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
}

fn predicate_contains_trait(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.trait_has.as_deref() == Some(needle)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
}

#[test]
fn bt20_100_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-100")
        .expect("BT20-100 compiled card present");

    assert_eq!(card.name, "The Last Guardian");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
}

#[test]
fn bt20_100_main_uses_dual_reveal_buckets_and_places_self_as_delay_option() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-100")
        .expect("BT20-100 compiled card present");

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] reveal/search clause exists");

    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 3, .. })
    ));
    let (buckets, no_duplicate_cards) = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectRevealBuckets {
                buckets,
                no_duplicate_cards,
                ..
            } => Some((buckets, no_duplicate_cards)),
            _ => None,
        })
        .expect("main search uses reveal buckets");
    assert!(*no_duplicate_cards);
    assert_eq!(buckets.len(), 2);
    assert!(predicate_contains_name(
        buckets[0].filter.as_ref().expect("Cool Boy bucket filter"),
        "Cool Boy"
    ));
    assert!(
        predicate_contains_trait(
            buckets[1].filter.as_ref().expect("trait bucket filter"),
            "Royal Knight"
        ) && predicate_contains_trait(
            buckets[1].filter.as_ref().expect("trait bucket filter"),
            "X Antibody"
        )
    );
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn bt20_100_security_places_self_after_optional_hand_or_trash_play_branch() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-100")
        .expect("BT20-100 compiled card present");

    let security = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Security] clause exists");

    assert!(security.optional, "printed Security play says 'may'");
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectEffectChoice { .. })));
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn bt20_100_delay_prevents_omnimon_named_digimon_from_leaving() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
        .add_card(omnimon_card("OMNI-TARGET"))
        .memory(0)
        .start();

    let target = r.place_on_field(0, "OMNI-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT20-100");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Delay replacement accept/decline prompt is exposed");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept BT20-100 Delay prevention");

    assert!(
        field_contains(&r, 0, "OMNI-TARGET"),
        "accepted Delay replacement keeps the threatened Omnimon on field"
    );
    assert!(
        trash_contains(&r, 0, "BT20-100"),
        "BT20-100 pays its Delay cost by trashing itself"
    );
}

#[test]
fn bt20_100_delay_decline_allows_original_leave_to_proceed() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
        .add_card(omnimon_card("OMNI-TARGET"))
        .memory(0)
        .start();

    let target = r.place_on_field(0, "OMNI-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT20-100");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "optional Delay replacement is offered"
    );

    r.game
        .resolve_selection(0, PASS)
        .expect("decline BT20-100 Delay prevention");

    assert!(
        !field_contains(&r, 0, "OMNI-TARGET"),
        "declining allows the Omnimon to leave"
    );
    assert!(
        field_contains(&r, 0, "BT20-100"),
        "declining does not pay the Delay self-trash cost"
    );
}

#[test]
fn bt20_100_does_not_offer_for_non_omnimon_subject() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
        .add_card(digimon_card("NON-OMNI", "Gallantmon Test"))
        .memory(0)
        .start();

    let target = r.place_on_field(0, "NON-OMNI", Some(0));
    place_delay_option(&mut r, 0, "BT20-100");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "BT20-100 only protects Digimon with Omnimon in name"
    );
    assert!(!field_contains(&r, 0, "NON-OMNI"));
    assert!(field_contains(&r, 0, "BT20-100"));
}

#[test]
fn bt20_100_unpaid_delay_cost_does_not_prevent_original_leave() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
        .add_card(omnimon_card("OMNI-TARGET"))
        .add_card(digimon_card("COST-GATE", "Cost Gate"))
        .memory(0)
        .start();
    r.register_effect("COST-GATE", Arc::new(CancelCostDeletion));

    let target = r.place_on_field(0, "OMNI-TARGET", Some(0));
    r.place_on_field(0, "COST-GATE", Some(0));
    place_delay_option(&mut r, 0, "BT20-100");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("attempt BT20-100 Delay prevention");

    assert!(
        !field_contains(&r, 0, "OMNI-TARGET"),
        "if the Delay cost is not paid, the original leave proceeds"
    );
    assert!(
        field_contains(&r, 0, "BT20-100"),
        "cost-cancelled Delay option stays on field"
    );
    assert!(!trash_contains(&r, 0, "BT20-100"));
}
