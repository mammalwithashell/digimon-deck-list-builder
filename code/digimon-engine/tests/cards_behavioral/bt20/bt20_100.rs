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
            // CR 16-17-3: placed on an earlier turn, so the <Delay> is usable now.
            placed_on_turn: r.game.turn_count.saturating_sub(1),
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
            .replacement_condition(|ctx, _| {
                ctx.replacement_cause() == Some(ReplacementCause::Cost)
            })
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

    // "You may play ... Then, place this card in the battle area": the "may"
    // is the optional hand-or-trash pick; the placement is mandatory (DCGO
    // BT20_100.cs:161 `SetUpActivateClass(null, ..., -1, false, ...)`, not
    // optional; :204-221 the zone choice exists only to reach a card pick).
    assert!(!security.optional, "the clause itself is mandatory");
    assert!(security.process.iter().any(|step| matches!(
        step,
        CompiledStep::SelectUnionZone { optional: true, .. }
    )));
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
    let offered = r.pending_selection_view().expect("Delay must offer a decision");
    assert_eq!(offered.selecting_player, 0);
    assert!(offered.is_optional, "only Last Guardian is eligible on the original leave");
    assert!(offered.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)
        [REPLACEMENT_ACCEPT as usize] > 0.0);
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

// ─── [Security] hand-or-trash pick legality ──────────────────────────────────
// DCGO BT20_100.cs:170-191 `CanSelectCardCondition`: own card &&
// CanPlayAsNewPermanent(payCost: false) && name [Omekamon] / [Cool Boy]; the
// hand / trash picks (:245, :269) use it; :201-222 the "From hand / From
// trash" choice is asked only when both zones hold such a card, else the
// zone with one is used, and with none no pick is made — the Option is
// placed in the battle area either way (:304-306). A hand-or-trash union
// pick offers exactly those cards. CR 4.3 1-3-2.

fn named(card_id: &str, name: &str, kind: digimon_engine::enums::CardKind) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, name);
    cd.card_kind = kind;
    cd.colors = vec![CardColor::White];
    if kind == digimon_engine::enums::CardKind::Tamer {
        cd.level = None;
        cd.dp = None;
    }
    cd
}

/// The Last Guardian as P1's only security card; P1's `hand` / `trash`; P0
/// attacks with ATTACKER. Returns the runner once the security check ran.
fn security_runner(hand: &[&str], trash: &[&str], prep: impl FnOnce(&mut DebugRunner)) -> DebugRunner {
    use digimon_engine::enums::CardKind;
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
        .add_card(named("OMEKA", "Omekamon", CardKind::Digimon))
        .add_card(named("COOL", "Cool Boy", CardKind::Tamer))
        .add_card(named("T-OMEKA", "Omekamon", CardKind::Digimon))
        .add_card(named("ATTACKER", "Attacker", CardKind::Digimon))
        .add_card(named("FILL", "Filler", CardKind::Digimon))
        .memory(10)
        .hand(1, hand)
        .security(1, &["BT20-100"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    for id in trash {
        r.inject_trash(1, id);
    }
    prep(&mut r);
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    r
}

/// Accept any outer "may" prompt, then return the committable rows of the
/// hand-or-trash pick (panics if the pending prompt is not a card pick, e.g.
/// a "From hand / From trash" zone choice).
fn security_offered(r: &mut DebugRunner) -> Vec<String> {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    while r.pending_kind() == Some(SelectionKind::Replacement) {
        r.accept_optional_trigger().unwrap();
    }
    let s = r.game.pending_selection.as_ref().expect("a card pick is pending");
    assert!(s.effect_choices.is_none(), "a card pick, not a zone choice");
    let page = s.card_choice.as_ref().expect("hand-or-trash card pick").view(&r.game, 1);
    let mut v: Vec<String> = page
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .filter_map(|row| row.card_id.clone())
        .collect();
    v.sort();
    v
}

fn delayed_on_field(r: &DebugRunner, player: u8) -> bool {
    r.game.player(player).battle_area.iter().any(|p| {
        p.top_card().card_id(&r.game.card_data) == "BT20-100"
            && matches!(p.option_state, OptionState::Delayed { .. })
    })
}

/// "Can't play Digimon by effects": the Omekamon in hand and in trash are not
/// offered, the Cool Boy Tamer is.
#[test]
fn bt20_100_security_offers_only_playable_cards() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = security_runner(&["OMEKA", "COOL"], &["T-OMEKA"], |r| {
        r.game.modifiers.add_player_modifier(
            1,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(security_offered(&mut r), vec!["COOL".to_string()]);
}

/// Only the hand holds a candidate: the card pick is offered directly, with
/// no "From trash" zone choice leading nowhere; picking plays it free.
#[test]
fn bt20_100_security_single_zone_offers_the_card_pick_directly() {
    let mut r = security_runner(&["OMEKA"], &[], |_| {});
    assert_eq!(security_offered(&mut r), vec!["OMEKA".to_string()]);
    let action = owned_union_fixture::action(&r.game, 1, "OMEKA", false).expect("hand row");
    r.game.resolve_selection(1, action).unwrap();
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 1, "OMEKA"), "Omekamon played");
    assert_eq!(r.memory(), 10, "played without paying the cost");
    assert!(delayed_on_field(&r, 1), "then the Option is placed");
}

/// Declining the play still places the Option in the battle area.
#[test]
fn bt20_100_security_decline_still_places_the_option() {
    let mut r = security_runner(&["OMEKA"], &["T-OMEKA"], |_| {});
    assert!(r.pending_selection().is_some(), "the optional play is offered");
    r.game.resolve_selection(1, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(!field_contains(&r, 1, "OMEKA"));
    assert!(delayed_on_field(&r, 1), "placement is not part of the 'may'");
}

/// P1's battle area at capacity: nothing can be played, so no card pick.
#[test]
fn bt20_100_security_full_battle_area_offers_no_pick() {
    let r = security_runner(&["OMEKA", "COOL"], &["T-OMEKA"], |r| {
        r.place_on_field(1, "FILL", None);
        r.game.rules.field_slots = 1;
    });
    assert!(
        r.game.pending_selection.as_ref().is_none_or(|s| s.card_choice.is_none()
            && s.kind != SelectionKind::Hand
            && s.kind != SelectionKind::Trash),
        "no playable card: {:?}",
        r.pending_selection_view().map(|v| (v.kind, v.prompt))
    );
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
