//! Phase 8 Task 4 — Plug-In / Link Option flow tests.
//!
//! A Link Option plays like a Standard Option (cost + `OptionMain` body drain),
//! but instead of trashing on dispose it **attaches sideways to a host Digimon**
//! via `host.linked_cards`. Host-selection surfaces through a `PendingSelection`
//! installed during dispose; after attach, `OnLink` fires globally. Sideways
//! inheritance means effects on the linked card with `.linked()` fire on the
//! host's timings. When the host leaves the field, every linked card trashes
//! and `OnLinkedCardTrashed` fires globally.

use std::sync::{Arc, Mutex};

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::OptionPlayResult;

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Link Option whose single `.link()` effect serves as both the OptionMain
/// body and the link declaration. The body writes "main" into an ordered
/// witness. Used by the OnLink ordering test so the drainer does not
/// install a TriggerOrder prompt between the body and the attach.
struct LinkToAnyDigimon {
    order: Arc<Mutex<Vec<&'static str>>>,
}
impl CardEffect for LinkToAnyDigimon {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let order = self.order.clone();
        vec![Effect::on_play(card)
            .name("Link any host + body")
            .link(0, |_ctx, _host| true)
            .process(move |_ctx| {
                order.lock().unwrap().push("main");
            })
            .build()]
    }
}

/// Minimal link declaration — no body witness, no cost.
struct LinkAnyHost;
impl CardEffect for LinkAnyHost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Link any host")
            .link(0, |_ctx, _host| true)
            .process(|_| {})
            .build()]
    }
}

/// Link Option whose `link_filter` accepts nothing — used to exercise the
/// "no eligible hosts" branch.
struct LinkNoEligibleHosts;
impl CardEffect for LinkNoEligibleHosts {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Link no eligible")
            .link(0, |_ctx, _host| false)
            .process(|_| {})
            .build()]
    }
}

/// Mandatory cancel replacement for the named pre-link window.
struct CancelWouldLink;
impl CardEffect for CancelWouldLink {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_link(card)
            .name("Cancel link")
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}

/// Optional cancel replacement for the named pre-link window.
struct OptionalCancelWouldLink;
impl CardEffect for OptionalCancelWouldLink {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_link(card)
            .name("Optional cancel link")
            .optional()
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}

/// A sideways-inherited effect: when the host fires `StartOfYourTurn`, this
/// linked-card effect fires and increments a witness counter.
struct LinkedStartOfTurnEffect(Arc<Mutex<u32>>);
impl CardEffect for LinkedStartOfTurnEffect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            // The link declaration that attaches this card.
            Effect::on_play(card)
                .name("Link any host")
                .link(0, |_ctx, _host| true)
                .process(|_| {})
                .build(),
            // A sideways-inherited StartOfYourTurn effect.
            Effect::start_of_your_turn(card)
                .name("Sideways start-of-turn")
                .linked()
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

/// Observer that appends "on_link" when `OnLink` fires — used to assert the
/// body-before-observer order.
struct OnLinkOrderWitness(Arc<Mutex<Vec<&'static str>>>);
impl CardEffect for OnLinkOrderWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let order = self.0.clone();
        vec![Effect::on_play(card)
            .name("OnLink order witness")
            .timing(EffectTiming::OnLink)
            .process(move |_ctx| {
                order.lock().unwrap().push("on_link");
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

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn register_dsl_yaml(r: &mut DebugRunner, yaml: &str) {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    let card_id = spec.card.clone();
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile card yaml");
    r.register_effect(&card_id, Arc::new(DslCardEffect::new(Arc::new(compiled))));
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: playing a Link Option installs a host-selection `PendingSelection`
/// and parks `pending_option` in `LinkSelectHost` phase. The card is NOT yet
/// attached.
#[test]
fn link_installs_host_selection() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    assert!(
        r.game.pending_selection.is_some(),
        "Link dispose installs a host-selection"
    );
    assert!(
        r.game.pending_option.is_some(),
        "pending_option carries through LinkSelectHost"
    );
    assert_eq!(
        r.game.pending_option.as_ref().unwrap().resolution_phase,
        digimon_engine::selection::OptionResolutionPhase::LinkSelectHost
    );
    // Not yet attached.
    assert!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .is_empty(),
        "no attach yet — host selection still pending"
    );
}

/// Test 2: resolving the host selection attaches the Option to the chosen
/// host and clears both `pending_selection` and `pending_option`.
#[test]
fn link_attaches_to_chosen_host() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    assert!(r.game.pending_selection.is_some());

    // Resolve the host-selection prompt.
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert!(r.game.pending_option.is_none(), "pending_option cleared");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "Option attached to host's linked_cards"
    );
    assert_eq!(r.trash_size(0), 0, "linked card does NOT trash");
    assert_eq!(r.hand_size(0), 0);
}

/// Resolving the host-selection prompt fires `WhenWouldLink` before the
/// pending Option attaches. A cancel replacement leaves the host unlinked and
/// trashes the pending Option.
#[test]
fn when_would_link_cancel_prevents_attach() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    r.register_effect("HOST", Arc::new(CancelWouldLink));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert!(r.game.pending_option.is_none(), "pending option cleared");
    assert!(r.game.pending_selection.is_none());
    assert!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .is_empty(),
        "cancelled link does not attach"
    );
    assert_eq!(r.trash_size(0), 1, "cancelled pending option is trashed");
}

#[test]
fn optional_when_would_link_accept_prevents_attach() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    r.register_effect("HOST", Arc::new(OptionalCancelWouldLink));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);

    assert!(
        r.game.pending_selection.is_some(),
        "host selection should park replacement prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept optional link replacement");

    assert!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .is_empty(),
        "accepted cancel does not attach"
    );
    assert_eq!(r.trash_size(0), 1, "cancelled pending option is trashed");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn optional_when_would_link_decline_resumes_attach() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    r.register_effect("HOST", Arc::new(OptionalCancelWouldLink));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);

    assert!(
        r.game.pending_selection.is_some(),
        "host selection should park replacement prompt"
    );
    r.game
        .resolve_selection(0, PASS)
        .expect("decline optional link replacement");

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "decline resumes attach"
    );
    assert_eq!(r.trash_size(0), 0, "linked card does not trash");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
}

/// Test 3: with no eligible hosts, the Link Option silently trashes — no
/// selection is installed.
#[test]
fn link_no_eligible_hosts_trashes_card() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkNoEligibleHosts));
    // RED-MATCH satisfies color match; filter returns false for all hosts.
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.trash_size(0), 1, "Link with no eligible host trashes");
}

/// Test 4: once attached, a `.linked()` effect on the linked card fires when
/// the host fires a matching timing (sideways inheritance).
#[test]
fn linked_card_sideways_inherits_effects() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-SIDE", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["LINK-SIDE"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(0)
        .start();
    r.register_effect(
        "LINK-SIDE",
        Arc::new(LinkedStartOfTurnEffect(witness.clone())),
    );
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    // Resolve host selection.
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "linked"
    );
    assert_eq!(*witness.lock().unwrap(), 0, "not fired yet");

    // End P0's turn, P1 plays through, and P0 enters their next turn. The
    // linked card's `StartOfYourTurn` should fire sideways via the host.
    r.end_turn(); // now P1
    r.game.enter_main_phase();
    r.end_turn(); // back to P0 — StartOfYourTurn fires at start.

    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "linked .start_of_your_turn fired via host's timing dispatch"
    );
}

/// Test 5: when the host is deleted, the linked card trashes (to the host
/// owner's trash) and `OnLinkedCardTrashed` fires globally.
#[test]
fn host_deletion_trashes_linked_card() {
    let witness = Arc::new(Mutex::new(0u32));
    // Use an observer on an unrelated Digimon on P1 so we can witness
    // OnLinkedCardTrashed firing on the non-host side.
    struct OnLinkedTrashedObserver(Arc<Mutex<u32>>);
    impl CardEffect for OnLinkedTrashedObserver {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let slot = self.0.clone();
            vec![Effect::on_play(card)
                .name("OnLinkedCardTrashed witness")
                .timing(EffectTiming::OnLinkedCardTrashed)
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("P1-WITNESS", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    r.register_effect(
        "P1-WITNESS",
        Arc::new(OnLinkedTrashedObserver(witness.clone())),
    );
    let host = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(1, "P1-WITNESS", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
    assert_eq!(r.trash_size(0), 0);

    // Delete the host — linked card should follow into trash and observer fires.
    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(host);

    assert_eq!(r.battle_area_size(0), 0, "host gone");
    // host top card + linked card both in owner's trash.
    assert_eq!(
        r.trash_size(0),
        trash_before + 2,
        "host top card + linked card both trashed to owner"
    );
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "OnLinkedCardTrashed fired globally (observed by P1)"
    );
}

/// Test 6: returning the host to hand trashes linked cards to owner's trash.
#[test]
fn host_return_to_hand_trashes_linked_card() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );

    // Return host to hand — linked card trashes (it cannot ride the host
    // back to hand; options trash when their host leaves via any path).
    let _ = r.game.return_to_hand(host);

    assert_eq!(r.battle_area_size(0), 0, "host returned to hand");
    assert_eq!(r.hand_size(0), 1, "host's top card is now in hand");
    assert_eq!(r.trash_size(0), 1, "linked card trashed");
}

/// Test 7: a linked card is NOT a standalone battle-area permanent — it
/// lives inside `host.linked_cards`. It must not be attackable or deletable
/// as a target directly.
#[test]
fn linked_card_not_targetable_by_attack_or_delete() {
    use digimon_engine::action::mask::build_action_mask;

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("P1-ATKR", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    // Core assertion: linked card does NOT consume a battle_area slot —
    // P0's battle_area holds the host only. A linked card can only ever
    // be the target of effects that reach through the host (unlink,
    // trash-linked), never of attacks or delete-target prompts that
    // enumerate standalone permanents.
    assert_eq!(
        r.game.player(0).battle_area.len(),
        1,
        "linked card does not consume a battle_area slot"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );

    r.end_turn();
    let attacker = r.place_on_field(1, "P1-ATKR", Some(0));
    r.game.enter_main_phase();

    // Attacker → index 1 (would-be-linked-slot if linked cards modeled
    // standalone) does not get a bit because no such slot exists in
    // P0.battle_area.
    let mask = build_action_mask(&r.game, 1);
    let bit_phantom =
        digimon_engine::action::space::encode_attack(attacker.index as u16, 1) as usize;
    assert_eq!(
        mask[bit_phantom], 0.0,
        "no attack bit for a non-existent slot"
    );
    // Direct attack to the phantom linked slot must also be rejected.
    let phantom = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 1,
    };
    let attack_out = r.game.attack_digimon(attacker, phantom, false);
    assert_eq!(attack_out, digimon_engine::combat::AttackResult::Invalid);
}

/// Test 8 (Appmon trait): `OnLink` fires globally on both sides after attach.
/// Witnesses on P0 and P1 both increment when a Link Option attaches to a
/// Digimon on P0.
#[test]
fn on_link_observer_fires_on_both_sides_after_attach() {
    let witness_p0 = Arc::new(Mutex::new(0u32));
    let witness_p1 = Arc::new(Mutex::new(0u32));

    // Two distinct effect structs so OnLinkObserver's slot doesn't collide.
    struct OnLinkObsA(Arc<Mutex<u32>>);
    impl CardEffect for OnLinkObsA {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let slot = self.0.clone();
            vec![Effect::on_play(card)
                .name("OnLink A")
                .timing(EffectTiming::OnLink)
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build()]
        }
    }
    struct OnLinkObsB(Arc<Mutex<u32>>);
    impl CardEffect for OnLinkObsB {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let slot = self.0.clone();
            vec![Effect::on_play(card)
                .name("OnLink B")
                .timing(EffectTiming::OnLink)
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("P0-WITNESS", CardColor::Red))
        .add_card(digimon_card("P1-WITNESS", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .memory(0)
        .start();
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    r.register_effect("P0-WITNESS", Arc::new(OnLinkObsA(witness_p0.clone())));
    r.register_effect("P1-WITNESS", Arc::new(OnLinkObsB(witness_p1.clone())));
    let host = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "P0-WITNESS", Some(0));
    r.place_on_field(1, "P1-WITNESS", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    // Attached to host.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
    // Both observers fired exactly once.
    assert_eq!(*witness_p0.lock().unwrap(), 1, "P0 OnLink observer fired");
    assert_eq!(*witness_p1.lock().unwrap(), 1, "P1 OnLink observer fired");
}

/// Test 9 (ORDER): the Link Option's `OptionMain` body fires BEFORE the
/// `OnLink` observer. The observer sees the attached state.
#[test]
fn on_link_observer_sees_option_main_already_resolved() {
    let order: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-ORDER", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("OBSERVER", CardColor::Red))
        .hand(0, &["LINK-ORDER"])
        .memory(0)
        .start();
    r.register_effect(
        "LINK-ORDER",
        Arc::new(LinkToAnyDigimon {
            order: order.clone(),
        }),
    );
    r.register_effect("OBSERVER", Arc::new(OnLinkOrderWitness(order.clone())));
    let host = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "OBSERVER", Some(0));
    advance_to_main(&mut r);

    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    // Body writes "main" first; OnLink observer writes "on_link" after attach.
    let recorded = order.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec!["main", "on_link"],
        "OptionMain fires BEFORE OnLink observer"
    );
    // And the attach actually happened.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
}

#[test]
fn dsl_free_link_step_surfaces_host_selection_mask() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("ST22-08", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["ST22-08"])
        .memory(0)
        .start();
    register_dsl_yaml(
        &mut r,
        r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - scope: face_up
    when: main
    optional: true
    process:
      - link_to_own_digimon:
          optional: true
          free: true
          filter: { kind: digimon }
"#,
    );
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let mask = build_action_mask(&r.game, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    assert_eq!(mask[action as usize], 1.0);
    let _ = r.game.resolve_selection(0, action);
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
}

#[test]
fn dsl_free_link_step_decoder_resolves_host_selection() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("ST22-08", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["ST22-08"])
        .memory(0)
        .start();
    register_dsl_yaml(
        &mut r,
        r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - scope: face_up
    when: main
    optional: true
    process:
      - link_to_own_digimon:
          optional: true
          free: true
          filter: { kind: digimon }
"#,
    );
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.decode_action(action, 0);

    assert!(
        r.game.pending_selection.is_none(),
        "decoder clears the host-selection prompt"
    );
    assert!(
        r.game.pending_option.is_none(),
        "decoder completes the parked Link option"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
}

#[test]
fn dsl_link_requirement_pays_nonzero_link_cost_before_host_selection() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-COST", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-COST"])
        .memory(3)
        .start();
    register_dsl_yaml(
        &mut r,
        r#"
card: LINK-COST
name: Costed Link
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { kind: digimon }
"#,
    );
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(r.memory(), 1, "Link requirement cost is paid");
    assert!(
        r.game.pending_selection.is_some(),
        "host selection remains player-visible after cost payment"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
}

#[test]
fn dsl_link_requirement_blocks_host_selection_when_link_cost_unaffordable() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-COST", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-COST"])
        .memory(0)
        .start();
    register_dsl_yaml(
        &mut r,
        r#"
card: LINK-COST
name: Costed Link
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 1
    filter: { kind: digimon }
"#,
    );
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);
    r.game.set_memory(-10);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[digimon_engine::action::space::PLAY_HAND_START as usize],
        0.0,
        "unaffordable Link cost must be masked before hand play"
    );
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid
    );
    assert_eq!(r.memory(), -10, "failed Link cost leaves memory unchanged");
    assert_eq!(r.hand_size(0), 1, "unaffordable Link option stays in hand");
    assert_eq!(
        r.trash_size(0),
        0,
        "unaffordable Link option is not disposed"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "unpaid Link cost must not expose a host-selection prompt"
    );
    assert!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .is_empty(),
        "unpaid Link cost must not attach for free"
    );
}

// ─── Section 1 gating diagnostics (change: implement-digilink-mechanic) ───
//
// These two tests resolve design decisions D6 and D7 before any Shape-B
// substrate is built. They assert the CURRENT engine behavior (so the suite
// stays green) and their comments record the verdict that the change's
// later tasks implement against.

/// A link card whose `WhenLinked` effect (effect 1) is modeled the faithful
/// way: an `OnLink` + `.linked()` effect SELF-FILTERED to fire only when the
/// just-linked card (`ctx.event_card()`) is this card (`ctx.source_card`).
/// This is the lowering target for DSL `when: when_linked`. The witness
/// counts every time effect 1 actually fires.
struct SelfFilteredWhenLinked(Arc<Mutex<u32>>);
impl CardEffect for SelfFilteredWhenLinked {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Link any host")
                .link(0, |_ctx, _host| true)
                .process(|_| {})
                .build(),
            Effect::on_play(card)
                .timing(EffectTiming::OnLink)
                .name("WhenLinked (OnLink + linked, self-filtered)")
                .linked()
                .condition(|rctx| rctx.event_card() == Some(rctx.source_card))
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

/// D6 regression (task 6.1 / design D6) — a faithful `WhenLinked` fires once
/// for the card that gets linked and does NOT over-fire when a sibling links
/// to the same host.
///
/// Before the fix, `OnLink` fired via `TriggerSource::PlayerBattleArea`
/// (no just-linked-card identity) and the `.linked()` fan-out re-scanned ALL
/// of a host's linked cards every attach, so a `WhenLinked`-as-`OnLink`
/// effect over-fired on every sibling link. The `TriggerSource::Linked`
/// variant now carries the just-linked card as `event_card`, so the
/// self-filter `event_card == source_card` isolates the firing to the actual
/// link event. No dedicated `WhenLinked` timing was added.
#[test]
fn d6_self_filtered_when_linked_fires_once_and_not_on_sibling() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-A", 0, CardColor::Red))
        .add_card(option_card("LINK-B", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["LINK-A", "LINK-B"])
        .memory(0)
        .start();
    r.register_effect("LINK-A", Arc::new(SelfFilteredWhenLinked(witness.clone())));
    r.register_effect("LINK-B", Arc::new(LinkAnyHost));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    // Attach A to the host — A's WhenLinked fires exactly once.
    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "WhenLinked fires once when A is the card that links"
    );

    // Attach B (sibling) to the same host — A's WhenLinked must NOT re-fire.
    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        2
    );
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "self-filter: sibling B linking does NOT re-fire A's WhenLinked"
    );
}

/// Facet #6/#11 — host-side `[When Linked]`: an effect on the HOST Digimon
/// that fires when a card gets linked **to that host**. The self-filter
/// `event_permanent() == source_permanent` isolates firing to the host the
/// card actually attached to, so a sibling host carrying the same effect does
/// not over-fire. Mirrors DCGO `CardEffectCommons.CanTriggerWhenLinked`
/// (`permanentCondition` matches the receiving permanent).
struct HostSideWhenLinked(Arc<Mutex<u32>>);
impl CardEffect for HostSideWhenLinked {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("[When Linked] host-side")
            .timing(EffectTiming::OnLink)
            .condition(|rctx| rctx.event_permanent() == rctx.source_permanent)
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Facet #6/#11 (host-side `[When Linked]`) — a card linking to a host fires
/// that host's own `[When Linked]` effect exactly once, and a sibling host
/// carrying the same effect does NOT fire when the link lands elsewhere.
#[test]
fn host_side_when_linked_fires_for_receiving_host_only() {
    let host_a_witness = Arc::new(Mutex::new(0u32));
    let host_b_witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-A", 0, CardColor::Red))
        .add_card(option_card("LINK-B", 0, CardColor::Red))
        .add_card(digimon_card("HOST-A", CardColor::Red))
        .add_card(digimon_card("HOST-B", CardColor::Red))
        .hand(0, &["LINK-A", "LINK-B"])
        .memory(0)
        .start();
    r.register_effect("LINK-A", Arc::new(LinkAnyHost));
    r.register_effect("LINK-B", Arc::new(LinkAnyHost));
    r.register_effect("HOST-A", Arc::new(HostSideWhenLinked(host_a_witness.clone())));
    r.register_effect("HOST-B", Arc::new(HostSideWhenLinked(host_b_witness.clone())));
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let host_b = r.place_on_field(0, "HOST-B", Some(0));
    advance_to_main(&mut r);

    // Link LINK-A specifically to HOST-A.
    let _ = r.game.play_option_from_hand(0, 0);
    let pa = r.game.pending_selection.as_ref().unwrap();
    let target_a = pa
        .valid_action_ids
        .iter()
        .copied()
        .find(|&aid| {
            use digimon_engine::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
            ((aid.saturating_sub(ATTACK_START)) % TARGETS_PER_ATTACKER) as u8 == host_a.index
        })
        .expect("HOST-A offered as a link host");
    let _ = r.game.resolve_selection(0, target_a);

    assert_eq!(
        r.game.player(0).battle_area[host_a.index as usize]
            .linked_cards
            .len(),
        1,
        "LINK-A attached to HOST-A"
    );
    assert_eq!(
        *host_a_witness.lock().unwrap(),
        1,
        "HOST-A's [When Linked] fires once for the card linked to it"
    );
    assert_eq!(
        *host_b_witness.lock().unwrap(),
        0,
        "HOST-B's [When Linked] does NOT fire for a link that landed on HOST-A"
    );

    // Now link LINK-B to HOST-B — only HOST-B fires this time.
    let _ = r.game.play_option_from_hand(0, 0);
    let pb = r.game.pending_selection.as_ref().unwrap();
    let target_b = pb
        .valid_action_ids
        .iter()
        .copied()
        .find(|&aid| {
            use digimon_engine::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
            ((aid.saturating_sub(ATTACK_START)) % TARGETS_PER_ATTACKER) as u8 == host_b.index
        })
        .expect("HOST-B offered as a link host");
    let _ = r.game.resolve_selection(0, target_b);

    assert_eq!(
        *host_a_witness.lock().unwrap(),
        1,
        "HOST-A unchanged when the next link lands on HOST-B"
    );
    assert_eq!(
        *host_b_witness.lock().unwrap(),
        1,
        "HOST-B's [When Linked] fires for the card linked to it"
    );
}

/// A Shape-B Appmon Link Digimon's static self link-condition: may link onto
/// any of the controller's Digimon for cost 1. Mirrors DCGO
/// `AddSelfLinkConditionStaticEffect(permanentCondition, linkCost: 1)`.
struct GatchLinkCondition;
impl CardEffect for GatchLinkCondition {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::link_condition(card)
            .name("Link condition: any Digimon host, cost 1")
            .link_host(1, |_ctx, _host| true)
            .build()]
    }
}

/// §3 (self link-condition metadata) — a Digimon carrying a `LinkCondition`
/// effect exposes its cost and legal hosts via `digimon_link_condition_targets`,
/// excludes itself as a host, and a plain Digimon returns `None`.
#[test]
fn digimon_self_link_condition_lists_hosts_and_excludes_self() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("GATCH", CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("GATCH", Arc::new(GatchLinkCondition));
    let host = r.place_on_field(0, "HOST", Some(0));
    let gatch = r.place_on_field(0, "GATCH", Some(0));

    let (cost, hosts) = r
        .game
        .digimon_link_condition_targets(gatch)
        .expect("GATCH carries a self link-condition");
    assert_eq!(cost, 1, "printed link cost");
    assert!(hosts.contains(&host), "HOST is a legal link host");
    assert!(
        !hosts.contains(&gatch),
        "a Digimon cannot link onto itself"
    );

    // A plain Digimon with no link condition is not a link source.
    assert!(
        r.game.digimon_link_condition_targets(host).is_none(),
        "HOST has no self link-condition"
    );
}

fn link_bit(perm: digimon_engine::permanent::PermanentHandle) -> usize {
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

/// §4/§5 — the on-field Link ability: an un-linked standing Appmon Link
/// Digimon is exposed a FIELD_EFFECT link sub-slot, and activating it +
/// picking a host absorbs the linking Digimon (top card → host linked cards),
/// pays the link cost, and removes it from the battle area.
#[test]
fn digimon_link_activate_absorbs_source_into_host() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("GATCH", CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(3)
        .start();
    r.register_effect("GATCH", Arc::new(GatchLinkCondition));
    let host = r.place_on_field(0, "HOST", Some(0));
    let gatch = r.place_on_field(0, "GATCH", Some(0));
    advance_to_main(&mut r);

    // The mask offers the Link ability on GATCH.
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[link_bit(gatch)], 1.0, "Link ability offered for GATCH");

    let mem_before = r.memory();
    r.game.decode_action(link_bit(gatch) as u16, 0);
    assert!(
        r.game.pending_selection.is_some(),
        "activating Link installs a host-selection prompt"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // GATCH is absorbed: battle area holds only HOST, which now carries the
    // GATCH top card as a single linked card; the link cost was paid.
    assert_eq!(r.battle_area_size(0), 1, "GATCH removed from battle area");
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "GATCH attached to HOST as a linked card"
    );
    assert_eq!(r.memory(), mem_before - 1, "link cost 1 paid");
}

/// A full Shape-B Appmon Link Digimon: a self link-condition (cost 0) + a
/// self-filtered `WhenLinked` witness + a linked `Raid` ESS. Used to prove the
/// real link-activate → absorb → OnLink path drives §6's WhenLinked/ESS.
struct GatchFull(Arc<Mutex<u32>>);
impl CardEffect for GatchFull {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        use digimon_engine::enums::{Expiry, Keyword};
        let slot = self.0.clone();
        vec![
            Effect::link_condition(card)
                .name("link cond")
                .link_host(0, |_ctx, _host| true)
                .build(),
            Effect::on_play(card)
                .timing(EffectTiming::OnLink)
                .name("WhenLinked")
                .linked()
                .condition(|rctx| rctx.event_card() == Some(rctx.source_card))
                .process(move |_| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
            Effect::declarative(card)
                .name("Linked ESS: Raid")
                .granted_keyword(Keyword::Raid)
                .materializes_declarative_state()
                .linked()
                .process(|ctx| {
                    if let Some(h) = ctx.source_permanent {
                        ctx.grant_declarative_keyword(h, Keyword::Raid, Expiry::Permanent);
                    }
                })
                .build(),
        ]
    }
}

/// §4/§5/§6 integration — activating the Link ability fires the linking
/// Digimon's own `WhenLinked` exactly once (through the absorb's OnLink) and
/// its linked `Raid` ESS reaches the host.
#[test]
fn digimon_link_activate_fires_when_linked_and_grants_ess() {
    use digimon_engine::enums::Keyword;
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("GATCH", CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(3)
        .start();
    r.register_effect("GATCH", Arc::new(GatchFull(witness.clone())));
    let host = r.place_on_field(0, "HOST", Some(0));
    let gatch = r.place_on_field(0, "GATCH", Some(0));
    advance_to_main(&mut r);

    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "GATCH's WhenLinked fired once through the absorb's OnLink"
    );
    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(host, Keyword::Raid),
        "GATCH's linked Raid ESS reaches the host"
    );
}

/// §7 — full Shape-B Appmon Link Digimon authored entirely in YAML DSL:
/// `kind: link_condition` (self link-condition) + `when: when_linked`
/// (self-filtered draw) + `scope: linked` `grant_keyword` (Raid ESS to host).
/// Proves the DSL vocabulary lowers and behaves through the real link-activate
/// → absorb → OnLink path.
#[test]
fn dsl_digimon_link_card_full_flow() {
    use digimon_engine::enums::Keyword;

    let yaml = r#"
card: GATCH-DSL
name: Gatchmon
kind: digimon
effects:
  - kind: link_condition
    cost: 1
    filter: { kind: digimon }
  - scope: linked
    when: when_linked
    process:
      - draw: { of: you, count: 1 }
  - scope: linked
    kind: grant_keyword
    keyword: Raid
"#;

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("GATCH-DSL", CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 5])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let host = r.place_on_field(0, "HOST", Some(0));
    let gatch = r.place_on_field(0, "GATCH-DSL", Some(0));
    advance_to_main(&mut r);

    // `kind: link_condition` lowers to a readable self link-condition.
    let (cost, hosts) = r
        .game
        .digimon_link_condition_targets(gatch)
        .expect("DSL link_condition recognized");
    assert_eq!(cost, 1);
    assert!(hosts.contains(&host));

    // Activate the Link ability and pick the host.
    let hand_before = r.hand_size(0);
    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // Absorbed, WhenLinked drew once, and the linked Raid ESS reaches the host.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "GATCH-DSL attached as a linked card"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "when_linked drew 1 (self-filtered, fired once)"
    );
    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(host, Keyword::Raid),
        "scope: linked grant_keyword Raid reaches the host"
    );
}

/// Facet #6/#11 (DSL host-side) — a host Digimon authored in YAML with
/// `when: when_card_linked_to_this` fires its body once when a card gets
/// linked to it. Confirms the DSL timing lowers to `OnLink` + the host
/// self-filter and does not require the linked card itself to carry anything.
#[test]
fn dsl_host_side_when_card_linked_to_this_fires_on_attach() {
    let yaml = r#"
card: HOST-DSL
name: Linked Host
kind: digimon
effects:
  - when: when_card_linked_to_this
    process:
      - draw: { of: you, count: 1 }
"#;

    let mut r = DebugRunner::builder()
        .add_card(option_card("LINK-CARD", 0, CardColor::Red))
        .add_card(digimon_card("HOST-DSL", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["LINK-CARD"])
        .deck(0, &["FILLER"; 5])
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    r.register_effect("LINK-CARD", Arc::new(LinkAnyHost));
    let host = r.place_on_field(0, "HOST-DSL", Some(0));
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);
    let _ = r.game.play_option_from_hand(0, 0);
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "LINK-CARD attached to HOST-DSL"
    );
    // hand: -1 for the played LINK-CARD, +1 for the host-side draw.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "when_card_linked_to_this drew 1 (net: -1 played +1 drawn)"
    );
}

/// §5 — standing-permanent absorb with a digivolution stack: the linking
/// Digimon's under-sources are trashed (DCGO DiscardEvoRoots); only its top
/// card becomes a linked card. Faithful to the flat linked-card model.
#[test]
fn digimon_link_absorb_trashes_evo_roots_keeps_only_top() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("LV2", CardColor::Red))
        .add_card(digimon_card("GATCH", CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(3)
        .start();
    r.register_effect("GATCH", Arc::new(GatchLinkCondition));
    let host = r.place_on_field(0, "HOST", Some(0));
    // GATCH on top of a LV2 source (a 2-card digivolution stack).
    let gatch = r.place_stack(0, &["LV2", "GATCH"]);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // Only HOST remains; it carries exactly the GATCH top card as a linked
    // card; the LV2 source was trashed.
    assert_eq!(r.battle_area_size(0), 1, "stack removed from battle area");
    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "only the top card is linked");
    assert_eq!(
        linked[0].card_id(&r.game.card_data),
        "GATCH",
        "the linked card is GATCH's top, not the LV2 source"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the LV2 digivolution source was trashed (DiscardEvoRoots)"
    );
}

/// Facet #9 — `link_chosen_card_into_host` lifts a chosen card from HAND and
/// attaches it onto a host, firing `OnLink` (so the host's `[When Linked]`
/// resolves). Mirrors DCGO `ILinkCard.LinkCard` with `root == Hand` →
/// `Permanent.AddLinkCard`.
#[test]
fn facet9_link_chosen_card_from_hand_attaches_and_fires_onlink() {
    use digimon_engine::enums::LinkCardSource;

    let host_witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("LINKEE", CardColor::Red))
        .hand(0, &["LINKEE"])
        .memory(3)
        .start();
    r.register_effect("HOST", Arc::new(HostSideWhenLinked(host_witness.clone())));
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let linkee = r.game.player(0).hand[0].handle();
    assert_eq!(r.hand_size(0), 1);

    let ok = r
        .game
        .link_chosen_card_into_host(host, linkee, LinkCardSource::Hand(0));
    assert!(ok, "card found in hand and attached");

    assert_eq!(r.hand_size(0), 0, "LINKEE left the hand");
    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "LINKEE attached as a linked card");
    assert_eq!(linked[0].handle(), linkee, "the attached card is LINKEE");
    assert_eq!(
        *host_witness.lock().unwrap(),
        1,
        "OnLink fired so the host's [When Linked] resolved once"
    );
}

/// Facet #9 — `link_chosen_card_into_host` can also lift a card from another
/// permanent's digivolution sources (DCGO `root == DigivolutionCards`). The
/// stack top is NOT eligible (it is the live Digimon); only an under-source
/// can be linked out.
#[test]
fn facet9_link_chosen_card_from_digivolution_sources() {
    use digimon_engine::enums::LinkCardSource;

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("UNDER", CardColor::Red))
        .add_card(digimon_card("TOP", CardColor::Red))
        .memory(3)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    // Donor: UNDER beneath TOP (a 2-card digivolution stack).
    let donor = r.place_stack(0, &["UNDER", "TOP"]);
    advance_to_main(&mut r);

    let under = r.game.player(0).battle_area[donor.index as usize].card_sources[0].handle();
    let top = r.game.player(0).battle_area[donor.index as usize].card_sources[1].handle();

    // The stack top cannot be linked out as a digivolution source.
    assert!(
        !r.game
            .link_chosen_card_into_host(host, top, LinkCardSource::DigivolutionSource(donor)),
        "the live top card is not an eligible under-source"
    );

    // The under-source links out fine.
    let ok =
        r.game
            .link_chosen_card_into_host(host, under, LinkCardSource::DigivolutionSource(donor));
    assert!(ok, "under-source lifted and attached");

    let host_idx = host.index as usize;
    assert_eq!(
        r.game.player(0).battle_area[host_idx].linked_cards.len(),
        1,
        "UNDER attached to the host"
    );
    // Donor still standing with just its top card.
    let donor_sources = &r.game.player(0).battle_area[donor.index as usize].card_sources;
    assert_eq!(donor_sources.len(), 1, "donor reduced to its top card");
    assert_eq!(donor_sources[0].handle(), top, "donor top is unchanged");
}

/// Facet #10 — a `ChangeLinkCost` reduction modifier lowers the memory paid
/// when a standing Digimon activates its link. ST22-12's
/// `GrantedReduceLinkCostClass(reducedCost: 2, _ => true, _ => true, _ => true)`
/// is a flat player-scoped reduction; this confirms the consult site in
/// `commit_digimon_link` applies `link_cost_delta_for_player`.
struct GatchLinkConditionCost2;
impl CardEffect for GatchLinkConditionCost2 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::link_condition(card)
            .name("Link condition: any Digimon host, cost 2")
            .link_host(2, |_ctx, _host| true)
            .build()]
    }
}

#[test]
fn facet10_change_link_cost_reduces_paid_link_cost() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::enums::ModifierType;
    use digimon_engine::modifiers::PlayerModifierEntry;

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("GATCH", CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(3)
        .start();
    r.register_effect("GATCH", Arc::new(GatchLinkConditionCost2)); // link cost 2
    let _host = r.place_on_field(0, "HOST", Some(0));
    let gatch = r.place_on_field(0, "GATCH", Some(0));
    advance_to_main(&mut r);

    // Flat -1 link-cost reduction for player 0 (ST22-12's reducedCost shape).
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::ChangeLinkCost, -1, Expiry::EndOfTurn, None, 0),
    );

    let memory_before = r.game.memory;
    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // Printed link cost 2, reduced by 1 → 1 memory paid (not 2).
    assert_eq!(
        r.game.memory,
        memory_before - 1,
        "ChangeLinkCost -1 reduced the cost-2 link to 1 memory paid"
    );
}

/// A linked Digimon whose Link-ESS grants its host both the `Raid` keyword
/// and +1000 DP, modeled the faithful way: two `.linked()` declarative
/// materializing effects targeting `ctx.source_permanent` (the host).
/// Mirrors DCGO `RaidSelfEffect(isLinkedEffect: true)`.
struct LinkedRaidAndDpEss;
impl CardEffect for LinkedRaidAndDpEss {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        use digimon_engine::enums::{Expiry, Keyword};
        vec![
            Effect::declarative(card)
                .name("Linked ESS: Raid")
                .granted_keyword(Keyword::Raid)
                .materializes_declarative_state()
                .linked()
                .process(|ctx| {
                    if let Some(h) = ctx.source_permanent {
                        ctx.grant_declarative_keyword(h, Keyword::Raid, Expiry::Permanent);
                    }
                })
                .build(),
            Effect::declarative(card)
                .name("Linked ESS: +1000 DP")
                .materializes_declarative_state()
                .linked()
                .process(|ctx| {
                    if let Some(h) = ctx.source_permanent {
                        ctx.add_declarative_dp_modifier(h, 1000, Expiry::Permanent);
                    }
                })
                .build(),
        ]
    }
}

/// D7 regression (task 6.2 / design D7) — a linked Digimon's continuous
/// Link-ESS grants (keyword + DP) reach its host.
///
/// Before the fix, `tick_declarative_effects` scanned only top cards and
/// under-stack digivolution sources, so a linked card's `.linked()`
/// declarative grants never materialized onto the host. The additive
/// linked-card pass in `tick_declarative_effects` routes them through the
/// modifier registry attributed to the host — so `has_keyword` and
/// `effective_dp` both see them.
#[test]
fn d7_linked_ess_keyword_and_dp_grant_reach_the_host() {
    use digimon_engine::enums::Keyword;
    use digimon_engine::permanent::PermanentHandle;

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("LINK-HOST", CardColor::Red))
        .add_card(digimon_card("RAID-ESS", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("RAID-ESS", Arc::new(LinkedRaidAndDpEss));

    let host: PermanentHandle = r.place_on_field(0, "LINK-HOST", Some(0));
    let base_dp = r.game.effective_dp(host).expect("host has DP");
    assert!(
        !r.game.has_keyword(host, Keyword::Raid),
        "baseline: host has no Raid before linking"
    );

    // Link the ESS card and refresh declaratives.
    r.push_linked_owned(host, "RAID-ESS", 0);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(host, Keyword::Raid),
        "linked Link-ESS grants Raid to the host"
    );
    assert_eq!(
        r.game.effective_dp(host),
        Some(base_dp + 1000),
        "linked Link-ESS grants +1000 DP to the host"
    );
}

/// A link card whose inherited Link-ESS is a CONTINUOUS DP formula + static DP
/// (not a materializing modifier grant). Exercises the two formula collectors
/// `live_declarative_formula_sum` / `static_dp_aura_bonus`, which scan
/// `card_sources` but historically not `linked_cards` (G-LINK-INHERITED-ESS).
struct LinkedFormulaEss;
impl CardEffect for LinkedFormulaEss {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            // Dynamic DP formula ESS → live_declarative_formula_sum.
            Effect::declarative(card)
                .name("linked dynamic +2000 DP")
                .linked()
                .dp_modifier_fn(|_ctx, _target| Some(2000))
                .build(),
            // Static DP ESS → static_dp_aura_bonus.
            Effect::declarative(card)
                .name("linked static +500 DP")
                .linked()
                .dp_modifier(500)
                .build(),
            // Dynamic Security Attack formula ESS → live_declarative_formula_sum
            // (security_attack=true branch).
            Effect::declarative(card)
                .name("linked Security A. +1")
                .linked()
                .security_attack_fn(|_ctx, _target| Some(1))
                .build(),
        ]
    }
}

/// G-LINK-INHERITED-ESS — a link card's continuous DP-formula / static-DP ESS
/// must reach its host (the formula collectors must scan `linked_cards`).
#[test]
fn linked_card_dp_formula_and_static_ess_reach_host() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("DP-ESS", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("DP-ESS", Arc::new(LinkedFormulaEss));
    let host = r.place_on_field(0, "HOST", Some(0));
    let base = r.game.effective_dp(host).expect("host DP");

    r.push_linked_owned(host, "DP-ESS", 0);

    assert_eq!(
        r.game.effective_dp(host),
        Some(base + 2500),
        "linked card's dynamic (+2000) and static (+500) DP ESS reach the host"
    );
    assert_eq!(
        r.game.dynamic_security_attack_aura_bonus(host),
        Some(1),
        "linked card's Security Attack formula ESS reaches the host"
    );
}

// ─── Facet #9 DSL authoring verb: `link_card_to_self` ────────────────────────

/// A test digimon trait-tagged with `Tool` so the `link_card_to_self` filter
/// (`trait_has: Tool`) can match it.
fn tool_digimon(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::Red];
    cd.traits = vec!["Tool".to_string()];
    cd
}

/// The `link_card_to_self` host: a Lv4 Digimon whose [On Play] links 1 [Tool]
/// card from the controller's hand onto itself for the printed link cost 1,
/// then a [When Linking] clause gains 5 memory (proving the step's call to the
/// engine primitive fires `OnLink` through the host's own when-linked clause).
const LINK_SELF_YAML: &str = r#"
card: LINK-SELF
name: LinkSelf
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
traits: [Tester]
effects:
  - when: on_play
    summary: "[On Play] link 1 [Tool] card from hand to this Digimon (cost 1)"
    process:
      - link_card_to_self:
          from: [hand]
          filter: { trait_has: Tool }
          cost: 1
  - when: when_card_linked_to_this
    summary: "[When this Digimon gets linked] gain 5 memory"
    process:
      - gain_memory: 5
"#;

/// A variant of `LINK-SELF` with no host-side reaction, so memory deltas are
/// purely the paid link cost (isolates the cost-reduction assertion).
const LINK_SELF_NO_REACTION_YAML: &str = r#"
card: LINK-SELF
name: LinkSelf
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
traits: [Tester]
effects:
  - when: on_play
    summary: "[On Play] link 1 [Tool] card from hand to this Digimon (cost 1)"
    process:
      - link_card_to_self:
          from: [hand]
          filter: { trait_has: Tool }
          cost: 1
"#;

/// TDD pin for G-DSL-LINK-CARD-FROM-ZONE — the `link_card_to_self` step
/// surfaces a real selection over the matching hand card (no auto-pick),
/// pays the printed link cost minus any reduction, then calls
/// `Game::link_chosen_card_into_host` so the card moves into the host's
/// `linked_cards` and `OnLink` fires.
#[test]
fn link_card_to_self_links_chosen_hand_card_pays_cost_and_fires_onlink() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut cd = make_test_card("LINK-SELF", "LinkSelf");
            cd.level = Some(4);
            cd.dp = Some(5000);
            cd.play_cost = 4;
            cd.colors = vec![CardColor::Red];
            cd
        })
        .add_card(tool_digimon("TOOL-LINKEE"))
        .add_card(digimon_card("NON-TOOL", CardColor::Red))
        .hand(0, &["LINK-SELF", "TOOL-LINKEE", "NON-TOOL"])
        .memory(10)
        .start();
    register_dsl_yaml(&mut r, LINK_SELF_YAML);
    advance_to_main(&mut r);

    // TOOL-LINKEE sits at hand index 1 before the play.
    let linkee = r.game.player(0).hand[1].handle();

    // Play LINK-SELF (hand index 0). On Play installs the link selection.
    let host_idx = r.play(0, 0).expect("LINK-SELF played");
    let memory_after_play = r.game.memory;

    assert!(
        r.game.pending_selection.is_some(),
        "link_card_to_self installs a real selection (no auto-pick)"
    );
    // Only the [Tool] card is a legal candidate; the non-Tool card is excluded.
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().valid_action_ids.len(),
        1,
        "exactly one Tool candidate is offered"
    );

    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // The chosen Tool card left the hand and attached to the host.
    let linked = &r.game.player(0).battle_area[host_idx].linked_cards;
    assert_eq!(linked.len(), 1, "Tool card attached as a linked card");
    assert_eq!(linked[0].handle(), linkee, "the attached card is TOOL-LINKEE");
    assert!(
        !r.game.player(0).hand.iter().any(|c| c.handle() == linkee),
        "TOOL-LINKEE left the hand"
    );

    // Cost paid: link cost 1 deducted at resolution. Plus +5 from When Linking.
    assert_eq!(
        r.game.memory,
        memory_after_play - 1 + 5,
        "paid the printed link cost 1, then When Linking gained 5"
    );
}

/// `ChangeLinkCost` reduction lowers the cost paid by `link_card_to_self`,
/// confirming the step consults `link_cost_delta_for_player` (facet #10).
#[test]
fn link_card_to_self_applies_change_link_cost_reduction() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;

    let mut r = DebugRunner::builder()
        .add_card({
            let mut cd = make_test_card("LINK-SELF", "LinkSelf");
            cd.level = Some(4);
            cd.dp = Some(5000);
            cd.play_cost = 4;
            cd.colors = vec![CardColor::Red];
            cd
        })
        .add_card(tool_digimon("TOOL-LINKEE"))
        .hand(0, &["LINK-SELF", "TOOL-LINKEE"])
        .memory(10)
        .start();
    register_dsl_yaml(&mut r, LINK_SELF_NO_REACTION_YAML);
    advance_to_main(&mut r);

    let host_idx = r.play(0, 0).expect("LINK-SELF played");
    // Flat -1 link-cost reduction for player 0.
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::ChangeLinkCost, -1, Expiry::EndOfTurn, None, 0),
    );
    let memory_before = r.game.memory;

    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    assert_eq!(
        r.game.player(0).battle_area[host_idx]
            .linked_cards
            .len(),
        1,
        "Tool card attached"
    );
    // Printed cost 1, reduced by 1 → 0 paid: memory unchanged by the link.
    assert_eq!(
        r.game.memory, memory_before,
        "ChangeLinkCost -1 made the cost-1 link free (no memory paid)"
    );
}
