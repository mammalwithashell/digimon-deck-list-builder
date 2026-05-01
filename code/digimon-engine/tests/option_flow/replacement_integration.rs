//! Phase 8 Task 6 — Phase 7 replacement integration for Option flow.
//!
//! Wires Standard Option self-trash through the Phase 7 `try_replace`
//! dispatcher at `WhenWouldBeTrashed`. Delay's end-of-turn trash already
//! routes through `delete_permanent_with_cause(Cost)` (Phase 7 fires
//! `WhenWouldLeaveBattleArea` + `WhenWouldBeDeleted` there — see Task 3's
//! `delay_fires_observer_wwbd_at_end_of_turn_trash` test). The linked-card
//! trash cascade stays OUTSIDE the replacement framework in v1
//! (too-recursive constraint documented in the spec §8.2).

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{HAND_EFFECT_START, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Zone};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Option body that gains 2 memory when played. Copied from standard_flow.
struct GainTwoMemory;
impl CardEffect for GainTwoMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 2 memory")
            .option_main()
            .process(|ctx| {
                ctx.gain_memory(2);
            })
            .build()]
    }
}

/// Mandatory WWBT cancel replacement on a field Digimon. Records when the
/// replacement process fires (sentinel), then cancels. Cancel is unscoped —
/// the entire WWBT event is cancelled for any subject.
struct WwbtCancelSentinel(Arc<Mutex<u32>>);
impl CardEffect for WwbtCancelSentinel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::when_would_be_trashed(card)
            .name("Cancel any WWBT")
            .replacement_process(move |rctx| {
                *slot.lock().unwrap() += 1;
                rctx.cancel();
            })
            .build()]
    }
}

/// Mandatory WWBT cancel replacement that only fires when
/// `rctx.cause == ReplacementCause::Cost` (matching the dispose-option path).
struct WwbtCancelOnCost(Arc<Mutex<u32>>);
impl CardEffect for WwbtCancelOnCost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::when_would_be_trashed(card)
            .name("Cancel WWBT on Cost cause")
            .replacement_process(move |rctx| {
                if matches!(rctx.cause, ReplacementCause::Cost) {
                    *slot.lock().unwrap() += 1;
                    rctx.cancel();
                }
            })
            .build()]
    }
}

/// Mandatory WWBT redirect-to-Deck replacement. Routes the card to the
/// bottom of the owner's deck instead of the trash.
struct WwbtRedirectToDeck;
impl CardEffect for WwbtRedirectToDeck {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_trashed(card)
            .name("Redirect WWBT to Deck")
            .replacement_process(|rctx| {
                rctx.redirect_to(Zone::Deck);
            })
            .build()]
    }
}

/// A linked card that carries a WWBT sentinel. The sentinel MUST NOT fire
/// when the host is deleted and the linked-card trash cascade runs — v1
/// constraint (spec §8.2). Linked card combines a `.link()` declaration
/// with a `when_would_be_trashed` witness.
struct LinkedWithWwbtWitness(Arc<Mutex<u32>>);
impl CardEffect for LinkedWithWwbtWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Link any host (with WWBT sentinel)")
                .link(0, |_ctx, _host| true)
                .process(|_| {})
                .build(),
            Effect::when_would_be_trashed(card)
                .name("WWBT witness on linked card")
                .replacement_process(move |_rctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

/// Mandatory WWBD cancel replacement that only fires when a permanent would
/// be deleted as Cost. Used to prove Delay prevention cannot proceed when
/// the Delay option fails to pay itself to trash.
struct WwbdCancelCostDeletion;
impl CardEffect for WwbdCancelCostDeletion {
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

/// Optional WWBD replacement that only appears while the Delay option itself
/// is being deleted as the replacement cost. Accepting it leaves the cost
/// deletion unhandled, so the Delay should still reach trash afterward.
struct PromptThenAllowDelayCost;
impl CardEffect for PromptThenAllowDelayCost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Prompt before allowing Delay cost")
            .optional()
            .replacement_condition(|ctx, subject| {
                if !matches!(ctx.replacement_cause(), Some(ReplacementCause::Cost)) {
                    return false;
                }
                let Some(handle) = subject.permanent() else {
                    return false;
                };
                ctx.game
                    .player(handle.player)
                    .battle_area
                    .get(handle.index as usize)
                    .is_some_and(|perm| perm.top_card().card_id(&ctx.game.card_data) == "BT17-097")
            })
            .replacement_process(|_| {})
            .build()]
    }
}

/// Post-cost observer that rewrites hand contents after the Delay option is
/// trashed. The Delay hand prompt must reflect this post-cost hand state.
struct ReplaceHandAfterDeletion;
impl CardEffect for ReplaceHandAfterDeletion {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_any_deletion(card)
            .name("Replace hand after deletion")
            .process(|ctx| {
                let Some(index) = ctx
                    .game
                    .player(0)
                    .deck
                    .iter()
                    .position(|card| card.card_id(&ctx.game.card_data) == "IMPERIAL-NEW")
                else {
                    return;
                };

                ctx.game.player_mut(0).hand.clear();
                let card = ctx.game.player_mut(0).deck.remove(index);
                ctx.game.player_mut(0).hand.push(card);
            })
            .build()]
    }
}

// ─── Fixture helpers ──────────────────────────────────────────────────

fn option_card(card_id: &str, cost: u16, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

fn trait_digimon_card(card_id: &str, name: &str, traits: &[&str]) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, name);
    cd.traits = traits.iter().map(|s| s.to_string()).collect();
    cd
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn find_battle_permanent(
    r: &DebugRunner,
    player: u8,
    card_id: &str,
) -> Option<digimon_engine::permanent::PermanentHandle> {
    r.game
        .player(player)
        .battle_area
        .iter()
        .position(|perm| {
            perm.card_sources
                .iter()
                .any(|source| source.card_id(&r.game.card_data) == card_id)
        })
        .map(|index| digimon_engine::permanent::PermanentHandle {
            player,
            index: index as u8,
        })
}

fn trash_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game
        .player(player)
        .trash
        .iter()
        .any(|card| card.card_id(&r.game.card_data) == card_id)
}

fn place_delay_option(r: &mut DebugRunner, player: u8, card_id: &str) {
    let handle = r.place_on_field(player, card_id, Some(0));
    r.game.player_mut(player).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: player,
            trash_on_turn: r.game.turn_count + 1,
        };
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: a Standard Option dispose fires `WhenWouldBeTrashed`. A
/// mandatory cancel sentinel on an observing field Digimon intercepts the
/// dispose-trash; the sentinel fires exactly once and the Option goes back
/// to its owner's hand (cancel → restore original zone).
#[test]
fn standard_option_trash_fires_when_would_be_trashed() {
    let sentinel = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-SENT", 0, CardColor::Red))
        .add_card(digimon_card("GATE", CardColor::Red))
        .hand(0, &["OPT-SENT"])
        .memory(0)
        .start();
    r.register_effect("OPT-SENT", Arc::new(GainTwoMemory));
    r.register_effect("GATE", Arc::new(WwbtCancelSentinel(sentinel.clone())));
    r.place_on_field(0, "GATE", Some(0));
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);
    let trash_before = r.trash_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(
        *sentinel.lock().unwrap(),
        1,
        "WhenWouldBeTrashed fired on Standard Option dispose"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "cancel routed the Option back to the owner's hand"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "Option did NOT trash — replacement cancelled"
    );
    assert!(r.game.pending_option.is_none(), "pending_option cleared");
}

/// Test 2: cost is still paid + body still fires when WWBT cancels the
/// dispose. Cancel filter restricted to `ReplacementCause::Cost` (matching
/// the dispose-option cause), so this also asserts the cause is plumbed
/// correctly from dispose_option into try_replace.
#[test]
fn standard_option_trash_cancel_returns_to_hand() {
    let sentinel = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-COST", 2, CardColor::Red))
        .add_card(digimon_card("GATE-COST", CardColor::Red))
        .hand(0, &["OPT-COST"])
        .memory(3)
        .start();
    r.register_effect("OPT-COST", Arc::new(GainTwoMemory));
    r.register_effect("GATE-COST", Arc::new(WwbtCancelOnCost(sentinel.clone())));
    r.place_on_field(0, "GATE-COST", Some(0));
    advance_to_main(&mut r);

    let memory_before = r.memory();
    let hand_before = r.hand_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(
        *sentinel.lock().unwrap(),
        1,
        "WWBT cancel filtered by Cost cause fired exactly once"
    );
    // Cost 2 paid up-front, body adds 2 back → net 0 memory change.
    assert_eq!(
        r.memory(),
        memory_before - 2 + 2,
        "cost paid + body fired, even though the trash was cancelled"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "Option back in hand after cancel"
    );
    assert_eq!(r.trash_size(0), 0, "not trashed — replacement cancelled");
}

/// Test 3: a mandatory WWBT redirect-to-Deck routes the Option to the
/// bottom of its owner's deck instead of the trash.
#[test]
fn standard_option_trash_redirect_to_deck_bottom() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-RED", 0, CardColor::Red))
        .add_card(digimon_card("GATE-RED", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["OPT-RED"])
        .deck(0, &["FILLER", "FILLER"])
        .memory(0)
        .start();
    r.register_effect("OPT-RED", Arc::new(GainTwoMemory));
    r.register_effect("GATE-RED", Arc::new(WwbtRedirectToDeck));
    r.place_on_field(0, "GATE-RED", Some(0));
    advance_to_main(&mut r);

    let deck_before = r.game.player(0).deck.len();
    let trash_before = r.trash_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before + 1,
        "Option redirected into the deck"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "Option did NOT trash — redirected"
    );
    // Bottom of the deck convention: `deck.insert(0, card)` means index 0
    // is the bottom; the most-recently-drawn cards live at the end.
    let bottom = r
        .game
        .player(0)
        .deck
        .first()
        .expect("deck non-empty after redirect");
    assert_eq!(
        bottom.card_id(&r.game.card_data),
        "OPT-RED",
        "redirected Option parks at the bottom of the deck"
    );
    assert!(r.game.pending_option.is_none());
}

/// Test 4: the WWBT replacement fires from the `Card(_, Zone::Hand)` subject
/// with `ReplacementCause::Cost`. Uses direct `try_replace` to observe the
/// exact subject/cause threaded through dispose_option, independent of the
/// end-to-end cancel path (which tests 1–3 already cover). Asserts that the
/// subject carries the pending Option's card handle and the original zone.
#[test]
fn standard_option_trash_subject_and_cause_are_correct() {
    // Witness receives (subject, cause) on each fire; we then assert them.
    let recorded: Arc<Mutex<Option<(ReplacementSubject, ReplacementCause)>>> =
        Arc::new(Mutex::new(None));
    struct SubjectRecorder(Arc<Mutex<Option<(ReplacementSubject, ReplacementCause)>>>);
    impl CardEffect for SubjectRecorder {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let slot = self.0.clone();
            vec![Effect::when_would_be_trashed(card)
                .name("Record subject + cause")
                .replacement_process(move |rctx| {
                    *slot.lock().unwrap() = Some((rctx.subject, rctx.cause));
                    // Let the trash proceed — no outcome override.
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-REC", 0, CardColor::Red))
        .add_card(digimon_card("GATE-REC", CardColor::Red))
        .hand(0, &["OPT-REC"])
        .memory(0)
        .start();
    r.register_effect("OPT-REC", Arc::new(GainTwoMemory));
    r.register_effect("GATE-REC", Arc::new(SubjectRecorder(recorded.clone())));
    r.place_on_field(0, "GATE-REC", Some(0));

    // Snapshot the Option's CardHandle pre-play so we can compare in the
    // assertion. The card lives in hand slot 0.
    let option_handle = r.game.player(0).hand[0].handle();

    advance_to_main(&mut r);
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);

    let guard = recorded.lock().unwrap();
    let (subj, cause) = guard.expect("WWBT fired and recorded subject + cause");
    match subj {
        ReplacementSubject::Card(handle, zone) => {
            assert_eq!(handle, option_handle, "subject carries the Option handle");
            assert_eq!(
                zone,
                Zone::Hand,
                "source zone reflects where the Option came from"
            );
        }
        other => panic!("expected Card subject, got {:?}", other),
    }
    assert_eq!(
        cause,
        ReplacementCause::Cost,
        "dispose-trash cause is Cost per spec"
    );
    // Trash still happened (no outcome override).
    assert_eq!(r.trash_size(0), 1, "no redirect → Option trashed normally");
}

/// Test 5 (v1 constraint): linked-card trash on host deletion does NOT fire
/// `WhenWouldBeTrashed`. The spec §8.2 carve-out keeps the host-cascade
/// outside the replacement framework to avoid recursive dispatch while the
/// host itself is being deleted. A WWBT sentinel on the linked card must
/// not fire when the host is deleted; the linked card trashes anyway.
#[test]
fn linked_card_trash_on_host_deletion_does_not_fire_wwbt() {
    let sentinel = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-OPT", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect(
        "LINK-OPT",
        Arc::new(LinkedWithWwbtWitness(sentinel.clone())),
    );

    // Place a host Digimon + the link Option on the field.
    let host = r.place_on_field(0, "HOST", Some(0));

    // Build a linked CardSource for LINK-OPT and attach it directly under
    // the host — this bypasses the Link-Option play flow and puts us in the
    // post-attach state we want to test.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "LINK-OPT")
        .unwrap();
    let next_idx = r.game.next_card_index();
    let mut cs = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    cs.card_index = next_idx;
    r.game.player_mut(0).battle_area[host.index as usize]
        .linked_cards
        .push(cs);

    // Sanity: linked_cards populated pre-delete.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );

    // Delete the host (cost cause) — triggers the linked-card cascade.
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::Cost);

    // Host gone.
    assert_eq!(
        r.game.player(0).battle_area.len(),
        0,
        "host deleted from the battle_area"
    );
    // Linked card trashed as part of the cascade.
    assert!(
        r.game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "LINK-OPT"),
        "linked card cascaded into trash with the host"
    );
    // v1 constraint: the WWBT sentinel on the linked card did NOT fire.
    assert_eq!(
        *sentinel.lock().unwrap(),
        0,
        "v1 constraint — linked-card host-cascade does NOT fire WWBT"
    );
}

#[test]
fn bt17_097_delay_prevents_deletion_and_digivolves_from_hand() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 fixture is available")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-HAND",
            "Imperial Hand",
            &["Imperialdramon"],
        ))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();

    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_some(),
        "Delay prevention prompt is exposed"
    );
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("select Imperialdramon-like hand target");

    assert_eq!(r.battle_area_size(0), 1, "target survived as one stack");
    let target = find_battle_permanent(&r, 0, "FREE-TARGET").expect("target still present");
    let target_stack = &r.game.player(0).battle_area[target.index as usize];
    assert_eq!(
        target_stack.card_sources.len(),
        2,
        "hand card digivolved onto the target"
    );
    assert_eq!(
        target_stack.top_card().card_id(&r.game.card_data),
        "IMPERIAL-HAND",
        "hand card is the new top card"
    );
    assert!(
        trash_contains(&r, 0, "BT17-097"),
        "Delay option paid itself to trash"
    );

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "no Delay remains, so no prevention prompt is exposed"
    );
    assert!(find_battle_permanent(&r, 0, "FREE-TARGET").is_none());
}

#[test]
fn bt17_097_delay_before_subject_re_resolves_shifted_target() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 fixture is available")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-HAND",
            "Imperial Hand",
            &["Imperialdramon"],
        ))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();

    place_delay_option(&mut r, 0, "BT17-097");
    let target = r.place_on_field(0, "FREE-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("select Imperialdramon-like hand target");

    let target = find_battle_permanent(&r, 0, "FREE-TARGET").expect("target still present");
    let target_stack = &r.game.player(0).battle_area[target.index as usize];
    assert_eq!(
        target_stack.top_card().card_id(&r.game.card_data),
        "IMPERIAL-HAND",
        "hand card digivolves onto the original threatened subject after Delay cost shifts indices"
    );
    assert!(trash_contains(&r, 0, "BT17-097"));
}

#[test]
fn bt17_097_paid_delay_decline_commits_original_deletion() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 fixture is available")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-HAND",
            "Imperial Hand",
            &["Imperialdramon"],
        ))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();

    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_some(),
        "paid BT17-097 replacement should ask for a hand choice"
    );
    r.game
        .resolve_selection(0, PASS)
        .expect("decline paid Delay hand choice");

    assert!(
        find_battle_permanent(&r, 0, "FREE-TARGET").is_none(),
        "declining the hand choice should allow the original deletion"
    );
    assert!(
        trash_contains(&r, 0, "BT17-097"),
        "BT17-097 should be trashed after paying its Delay cost"
    );
    assert!(
        r.game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == "IMPERIAL-HAND"),
        "declined hand card should remain in hand"
    );
}

#[test]
fn bt17_097_unpaid_delay_cost_does_not_prevent_original_deletion() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 fixture is available")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-HAND",
            "Imperial Hand",
            &["Imperialdramon"],
        ))
        .add_card(digimon_card("COST-GATE", CardColor::Red))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();
    r.register_effect("COST-GATE", Arc::new(WwbdCancelCostDeletion));
    r.place_on_field(0, "COST-GATE", Some(0));
    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "unpaid Delay must not expose the hand choice"
    );
    assert!(
        find_battle_permanent(&r, 0, "FREE-TARGET").is_none(),
        "original deletion proceeds when Delay cost is not paid"
    );
    assert!(
        find_battle_permanent(&r, 0, "BT17-097").is_some(),
        "Delay option stayed on field because its cost deletion was cancelled"
    );
    assert!(!trash_contains(&r, 0, "BT17-097"));
}

#[test]
fn bt17_097_delay_waits_for_pending_cost_replacement_before_hand_choice() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 fixture is available")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-HAND",
            "Imperial Hand",
            &["Imperialdramon"],
        ))
        .add_card(digimon_card("COST-PROMPT", CardColor::Red))
        .hand(0, &["IMPERIAL-HAND"])
        .memory(0)
        .start();
    r.register_effect("COST-PROMPT", Arc::new(PromptThenAllowDelayCost));
    r.place_on_field(0, "COST-PROMPT", Some(0));
    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Delay cost replacement prompt is exposed before hand choice");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert!(
        find_battle_permanent(&r, 0, "BT17-097").is_some(),
        "Delay source is not trashed until the cost prompt resolves"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept cost replacement prompt");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("paid Delay resumes into the hand choice");
    assert_eq!(pending.kind, SelectionKind::EffectChoice);
    assert!(pending.valid_action_ids.contains(&HAND_EFFECT_START));

    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("select Imperialdramon-like hand target");

    let target = find_battle_permanent(&r, 0, "FREE-TARGET").expect("target still present");
    let target_stack = &r.game.player(0).battle_area[target.index as usize];
    assert_eq!(
        target_stack.top_card().card_id(&r.game.card_data),
        "IMPERIAL-HAND"
    );
    assert!(trash_contains(&r, 0, "BT17-097"));
}

#[test]
fn bt17_097_delay_rebuilds_hand_candidates_after_cost_triggers() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-097")
        .expect("BT17-097 fixture is available")
        .add_card(trait_digimon_card("FREE-TARGET", "Free Target", &["Free"]))
        .add_card(trait_digimon_card(
            "IMPERIAL-OLD",
            "Imperial Old",
            &["Imperialdramon"],
        ))
        .add_card(trait_digimon_card(
            "IMPERIAL-NEW",
            "Imperial New",
            &["Imperialdramon"],
        ))
        .add_card(digimon_card("HAND-MUTATOR", CardColor::Red))
        .hand(0, &["IMPERIAL-OLD"])
        .deck(0, &["IMPERIAL-NEW"])
        .memory(0)
        .start();
    r.register_effect("HAND-MUTATOR", Arc::new(ReplaceHandAfterDeletion));
    r.place_on_field(0, "HAND-MUTATOR", Some(0));
    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    place_delay_option(&mut r, 0, "BT17-097");

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    let choices = r
        .game
        .pending_selection
        .as_ref()
        .and_then(|selection| selection.effect_choices.as_ref())
        .expect("Delay hand choice is exposed");
    assert!(
        choices.iter().any(|choice| choice.label == "Imperial New"),
        "post-cost hand card is offered"
    );
    assert!(
        !choices.iter().any(|choice| choice.label == "Imperial Old"),
        "pre-cost stale hand card is not offered"
    );

    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("select post-cost Imperialdramon-like hand target");

    let target = find_battle_permanent(&r, 0, "FREE-TARGET").expect("target still present");
    let target_stack = &r.game.player(0).battle_area[target.index as usize];
    assert_eq!(
        target_stack.top_card().card_id(&r.game.card_data),
        "IMPERIAL-NEW",
        "hand prompt resolved the post-cost candidate"
    );
}
