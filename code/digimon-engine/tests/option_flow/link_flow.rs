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

use digimon_dsl::compiled::CompiledStep;
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
    r.register_effect(
        "HOST-A",
        Arc::new(HostSideWhenLinked(host_a_witness.clone())),
    );
    r.register_effect(
        "HOST-B",
        Arc::new(HostSideWhenLinked(host_b_witness.clone())),
    );
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
    assert!(!hosts.contains(&gatch), "a Digimon cannot link onto itself");

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

/// Gap 1 (G-ENGINE-AURA-GRANT-LINK-MAX) — a `kind: aura` with
/// `modifier: ChangeLinkMax` now carries its scalar `modifier_value`; the aura
/// path previously installed every named modifier with a hardcoded `0`, so
/// "Link +1" was a no-op. Self-aura form (BT25-060 Rebootmon's self Link +1).
#[test]
fn gap1_self_link_max_aura_grants_nonzero_delta() {
    let yaml = r#"
card: REBOOT-DSL
name: Rebootmon
kind: digimon
effects:
  - kind: aura
    modifier: ChangeLinkMax
    modifier_value: 1
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("REBOOT-DSL", CardColor::Red))
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let h = r.place_on_field(0, "REBOOT-DSL", Some(0));
    advance_to_main(&mut r);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.modifiers.link_max_delta(h),
        1,
        "Link +1 self-aura installs ChangeLinkMax +1 (was hardcoded 0)"
    );
}

/// Gap 1 — filter-target aura form (BT25-075 / BT25-102: "all of your [TS]
/// trait Digimon gain Link +1"). The modifier_value reaches each matched host.
#[test]
fn gap1_filter_target_link_max_aura_grants_nonzero_delta() {
    let yaml = r#"
card: VULCANUS-DSL
name: Vulcanusmon
kind: digimon
effects:
  - kind: aura
    target: { trait_has: TS, controller: you }
    modifier: ChangeLinkMax
    modifier_value: 1
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("VULCANUS-DSL", CardColor::Red))
        .add_card({
            let mut c = digimon_card("TS-MON", CardColor::Red);
            c.traits = vec!["TS".to_string()];
            c
        })
        .add_card(digimon_card("PLAIN-MON", CardColor::Red))
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    r.place_on_field(0, "VULCANUS-DSL", Some(0));
    let ts = r.place_on_field(0, "TS-MON", Some(0));
    let plain = r.place_on_field(0, "PLAIN-MON", Some(0));
    advance_to_main(&mut r);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.modifiers.link_max_delta(ts),
        1,
        "TS Digimon gains Link +1 from the aura"
    );
    assert_eq!(
        r.game.modifiers.link_max_delta(plain),
        0,
        "a non-TS Digimon is unaffected"
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

// ─── Gap 2: `link_cards` DSL step ────────────────────────────────────────
//
// The authoring verb over the shipped `link_chosen_card_into_host` primitive.
// Drives BT25-060 Rebootmon / BT25-075 Vulcanusmon / BT25-089 Kazuki & Itsuki:
// "link 1..N [Appmon] cards from your hand / trash / digivolution sources to a
// Digimon without paying the cost". Faithful to DCGO ST22_12: zone-choice-first
// when multiple source zones have candidates, then a single-zone card select,
// then (for `to: own_digimon`) a host select, then attach via the primitive.

/// A digimon card carrying the [Appmon] trait — the filter target for the
/// link-cards step's `trait_has: Appmon` predicate.
fn appmon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd.traits = vec!["Appmon".to_string()];
    cd
}

/// Gap 2 test 1 — `link_cards { from: [hand], to: self, count: {exactly: 1} }`:
/// an on-play body that links one matching card from hand onto the source's own
/// permanent. The card moves hand→host.linked_cards and OnLink fires.
#[test]
fn gap2_link_card_from_hand_to_self() {
    let yaml = r#"
card: REBOOT-DSL
name: Rebootmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [hand]
          filter: { trait_has: Appmon }
          to: self
          count: { exactly: 1 }
          cost: free
"#;

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("REBOOT-DSL", CardColor::Red))
        .add_card(appmon_card("APPMON", CardColor::Red))
        .add_card(digimon_card("NOTAPP", CardColor::Red))
        .hand(0, &["APPMON", "NOTAPP"])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let host = r.place_on_field(0, "REBOOT-DSL", Some(0));
    advance_to_main(&mut r);

    let appmon = r.game.player(0).hand[0].handle();
    let hand_before = r.hand_size(0);

    // Fire the on-play body. Only one Appmon candidate exists → a single-zone
    // card select is installed (one source zone, so no zone-choice prompt).
    r.game.fire_on_play(0, host.index as usize);
    assert!(
        r.game.pending_selection.is_some(),
        "link_cards installs a card-select prompt (no auto-pick)"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    assert_eq!(r.hand_size(0), hand_before - 1, "APPMON left the hand");
    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "exactly one card linked onto self");
    assert_eq!(linked[0].handle(), appmon, "the linked card is the Appmon");
}

/// Gap 2 test 2 — `from: [self_sources]`: the link card is lifted out of the
/// source permanent's own digivolution stack (an under-source), not the hand.
#[test]
fn gap2_link_card_from_self_sources_to_self() {
    let yaml = r#"
card: REBOOT-DSL
name: Rebootmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [self_sources]
          filter: { trait_has: Appmon }
          to: self
          count: { exactly: 1 }
          cost: free
"#;

    let mut r = DebugRunner::builder()
        .add_card(appmon_card("APP-SRC", CardColor::Red))
        .add_card(digimon_card("REBOOT-DSL", CardColor::Red))
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    // REBOOT-DSL on top of an Appmon under-source (a 2-card digivolution stack).
    let host = r.place_stack(0, &["APP-SRC", "REBOOT-DSL"]);
    advance_to_main(&mut r);

    let under = r.game.player(0).battle_area[host.index as usize].card_sources[0].handle();
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .card_sources
            .len(),
        2,
        "host starts as a 2-card stack"
    );

    r.game.fire_on_play(0, host.index as usize);
    assert!(
        r.game.pending_selection.is_some(),
        "link_cards installs a source-card select prompt"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    let perm = &r.game.player(0).battle_area[host.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "the under-source was lifted out of the stack"
    );
    assert_eq!(perm.linked_cards.len(), 1, "one card linked onto self");
    assert_eq!(
        perm.linked_cards[0].handle(),
        under,
        "the linked card is the former under-source"
    );
}

/// Gap 2 test 3 — `from: [hand], to: own_digimon, count: {up_to: 2}`: links up
/// to 2 cards onto a player-selected host. Exercises the per-pick host select
/// and that declining the loop early (PASS after pick 1) stops it.
#[test]
fn gap2_link_up_to_2_to_selected_digimon() {
    let yaml = r#"
card: VULC-DSL
name: Vulcanusmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [hand]
          to: own_digimon
          count: { up_to: 2 }
          cost: free
"#;

    let mut r = DebugRunner::builder()
        .add_card(digimon_card("VULC-DSL", CardColor::Red))
        .add_card(digimon_card("TARGET", CardColor::Red))
        .add_card(option_card("CARD-A", 0, CardColor::Red))
        .add_card(option_card("CARD-B", 0, CardColor::Red))
        .hand(0, &["CARD-A", "CARD-B"])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let vulc = r.place_on_field(0, "VULC-DSL", Some(0));
    let target = r.place_on_field(0, "TARGET", Some(0));
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);
    r.game.fire_on_play(0, vulc.index as usize);

    // Pick 1: choose the card from hand.
    assert!(
        r.game.pending_selection.is_some(),
        "first card select installed"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // Then choose the host Digimon for that card.
    assert!(
        r.game.pending_selection.is_some(),
        "host select installed for pick 1"
    );
    let host_action = {
        use digimon_engine::action::space::encode_attack;
        encode_attack(target.player as u16, target.index as u16)
    };
    let _ = r.game.resolve_selection(0, host_action);

    assert_eq!(
        r.game.player(0).battle_area[target.index as usize]
            .linked_cards
            .len(),
        1,
        "first card linked onto the selected host"
    );

    // Pick 2: another card select is offered (count up_to 2). Decline (PASS)
    // to stop the loop early — only one card should be linked total.
    assert!(
        r.game.pending_selection.is_some(),
        "second card select offered"
    );
    assert!(
        r.game.pending_selection.as_ref().unwrap().is_optional,
        "the up_to loop exposes PASS"
    );
    let _ = r.game.resolve_selection(0, PASS);

    assert!(
        r.game.pending_selection.is_none(),
        "declining stops the up_to loop"
    );
    assert_eq!(
        r.game.player(0).battle_area[target.index as usize]
            .linked_cards
            .len(),
        1,
        "declining early leaves exactly one linked card"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before - 1,
        "only one card left the hand"
    );
}

/// Gap 2 test 4 — `from: [hand, self_sources]` with candidates in BOTH zones:
/// a zone-choice selection is installed FIRST (faithful to DCGO's bool prompt),
/// before any card select. Confirms the multi-zone branch.
#[test]
fn gap2_zone_choice_when_both_zones_have_candidates() {
    let yaml = r#"
card: REBOOT-DSL
name: Rebootmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [hand, self_sources]
          filter: { trait_has: Appmon }
          to: self
          count: { exactly: 1 }
          cost: free
"#;

    let mut r = DebugRunner::builder()
        .add_card(appmon_card("APP-SRC", CardColor::Red))
        .add_card(digimon_card("REBOOT-DSL", CardColor::Red))
        .add_card(appmon_card("APP-HAND", CardColor::Red))
        .hand(0, &["APP-HAND"])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    // REBOOT-DSL on top of an Appmon under-source: a candidate in self_sources,
    // plus an Appmon in hand → both zones populated.
    let host = r.place_stack(0, &["APP-SRC", "REBOOT-DSL"]);
    advance_to_main(&mut r);

    r.game.fire_on_play(0, host.index as usize);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("a selection is installed");
    assert_eq!(
        pending.kind,
        digimon_engine::selection::SelectionKind::EffectChoice,
        "with candidates in both zones, a zone-choice prompt is installed first"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "two zone options offered (hand / digivolution sources)"
    );
}

// ── make-engine-cloneable: link-loop clone-safety (all 4 pick stages) ────────

/// HostSelect stage: clone the game AT the per-pick host select (after the card
/// is chosen). The clone resolves host → attach → recurse while the original is
/// untouched and replays identically.
#[test]
fn link_loop_host_select_clones_faithfully() {
    use digimon_engine::action::space::encode_attack;
    let yaml = r#"
card: VULC-CLN
name: Vulcanusmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [hand]
          to: own_digimon
          count: { up_to: 2 }
          cost: free
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("VULC-CLN", CardColor::Red))
        .add_card(digimon_card("TARGET-CLN", CardColor::Red))
        .add_card(option_card("CCARD-A", 0, CardColor::Red))
        .add_card(option_card("CCARD-B", 0, CardColor::Red))
        .hand(0, &["CCARD-A", "CCARD-B"])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let vulc = r.place_on_field(0, "VULC-CLN", Some(0));
    let target = r.place_on_field(0, "TARGET-CLN", Some(0));
    advance_to_main(&mut r);
    r.game.fire_on_play(0, vulc.index as usize);

    // Pick-1 card select (resume-driven) → resolve → host select.
    assert!(
        r.game.pending_selection_resume.is_some(),
        "link card select must be resume-driven (clone-safe)"
    );
    let card_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, card_action)
        .expect("pick card 1");

    assert!(
        r.game.pending_selection.is_some() && r.game.pending_selection_resume.is_some(),
        "host select installed and resume-driven"
    );
    let host_action = encode_attack(target.player as u16, target.index as u16);

    // Clone AT the host select; finish the loop on the clone only.
    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, host_action)
        .expect("clone resolves host");
    assert!(
        clone.pending_selection.is_some(),
        "clone: pick-2 select offered after the attach + recurse"
    );
    clone.resolve_selection(0, PASS).expect("clone PASS pick 2");
    assert!(clone.pending_selection.is_none(), "clone: loop complete");
    assert_eq!(
        clone.player(0).battle_area[target.index as usize]
            .linked_cards
            .len(),
        1,
        "clone: exactly one card linked onto the host"
    );

    // INDEPENDENCE: the original is untouched.
    assert!(
        r.game.pending_selection.is_some(),
        "original's host select survives the clone"
    );
    assert_eq!(
        r.game.player(0).battle_area[target.index as usize]
            .linked_cards
            .len(),
        0,
        "original: nothing linked while the clone resolved"
    );

    // REPLAYS IDENTICALLY.
    r.game
        .resolve_selection(0, host_action)
        .expect("original resolves host");
    r.game
        .resolve_selection(0, PASS)
        .expect("original PASS pick 2");
    assert_eq!(
        r.game.player(0).battle_area[target.index as usize]
            .linked_cards
            .len(),
        1,
        "original reaches the clone's linked state"
    );
}

/// ZoneChoice stage: clone the game AT the zone-choice prompt (both hand +
/// self_sources have candidates). The clone picks hand → card → attach (to: self).
#[test]
fn link_loop_zone_choice_clones_faithfully() {
    let yaml = r#"
card: REBOOT-CLN
name: Rebootmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [hand, self_sources]
          filter: { trait_has: Appmon }
          to: self
          count: { exactly: 1 }
          cost: free
"#;
    let mut r = DebugRunner::builder()
        .add_card(appmon_card("APP-SRC-C", CardColor::Red))
        .add_card(digimon_card("REBOOT-CLN", CardColor::Red))
        .add_card(appmon_card("APP-HAND-C", CardColor::Red))
        .hand(0, &["APP-HAND-C"])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let host = r.place_stack(0, &["APP-SRC-C", "REBOOT-CLN"]);
    advance_to_main(&mut r);
    r.game.fire_on_play(0, host.index as usize);

    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        digimon_engine::selection::SelectionKind::EffectChoice,
        "zone choice installed first"
    );
    assert!(
        r.game.pending_selection_resume.is_some(),
        "zone choice must be resume-driven (clone-safe)"
    );
    // eligible is in `from:` order → action_ids[0] = hand.
    let zone_hand = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, zone_hand)
        .expect("clone picks the hand zone");
    let card = clone.pending_selection.as_ref().unwrap().valid_action_ids[0];
    clone
        .resolve_selection(0, card)
        .expect("clone picks the card");
    assert!(
        clone.pending_selection.is_none(),
        "clone: loop complete after the exactly-1 pick"
    );
    assert_eq!(
        clone.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "clone: one card linked to self"
    );

    // INDEPENDENCE.
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        digimon_engine::selection::SelectionKind::EffectChoice,
        "original still parked at the zone choice"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        0,
        "original: nothing linked yet"
    );

    // REPLAYS IDENTICALLY.
    r.game
        .resolve_selection(0, zone_hand)
        .expect("original picks the hand zone");
    let card2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, card2)
        .expect("original picks the card");
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "original reaches the clone's linked state"
    );
}

/// CardSelectSources stage: `from: [self_sources]` only → straight to the
/// source select. Clone there; the clone resolves the source link to self.
#[test]
fn link_loop_source_select_clones_faithfully() {
    let yaml = r#"
card: REBOOT-SRC
name: Rebootmon
kind: digimon
effects:
  - when: on_play
    process:
      - link_cards:
          from: [self_sources]
          filter: { trait_has: Appmon }
          to: self
          count: { exactly: 1 }
          cost: free
"#;
    let mut r = DebugRunner::builder()
        .add_card(appmon_card("APP-USRC", CardColor::Red))
        .add_card(digimon_card("REBOOT-SRC", CardColor::Red))
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let host = r.place_stack(0, &["APP-USRC", "REBOOT-SRC"]);
    advance_to_main(&mut r);
    r.game.fire_on_play(0, host.index as usize);

    assert!(
        r.game.pending_selection.is_some() && r.game.pending_selection_resume.is_some(),
        "source select installed and resume-driven (clone-safe)"
    );
    let src_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, src_action)
        .expect("clone picks the source");
    assert!(clone.pending_selection.is_none(), "clone: loop complete");
    assert_eq!(
        clone.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "clone: the digivolution source linked to self"
    );

    // INDEPENDENCE.
    assert!(
        r.game.pending_selection.is_some(),
        "original survives the clone"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        0,
        "original: source not yet linked"
    );

    // REPLAYS IDENTICALLY.
    r.game
        .resolve_selection(0, src_action)
        .expect("original picks the source");
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "original reaches the clone's linked state"
    );
}

/// §4.5.2 — `link_cards { to: own_digimon }` honors `host_filter` (the link
/// requirement) AND `exclude_source` ("1 of your OTHER Digimon"): the host
/// select offers only own Digimon matching the predicate, never the effect's
/// own source permanent — even when the source itself matches the filter.
#[test]
fn link_cards_host_filter_and_exclude_source() {
    use digimon_engine::action::space::encode_attack;
    let yaml = r#"
card: HOSTFILT
name: HostFilter
kind: digimon
effects:
  - when: on_play
    summary: "link 1 [Appmon] from hand to a [Marked] OTHER Digimon"
    process:
      - link_cards:
          from: [hand]
          filter: { trait_has: Appmon }
          to: own_digimon
          count: { up_to: 1 }
          cost: free
          host_filter: { trait_has: Marked }
          exclude_source: true
"#;
    let mut r = DebugRunner::builder()
        // The effect source ALSO carries [Marked], so only `exclude_source`
        // (not `host_filter`) can drop it — proving the exclusion fires.
        .add_card(traited_digimon("HOSTFILT", CardColor::Red, &["Marked"]))
        .add_card(traited_digimon("HOST-OK", CardColor::Red, &["Marked"]))
        .add_card(traited_digimon("HOST-NO", CardColor::Red, &[]))
        .add_card(appmon_card("APPMON", CardColor::Red))
        .hand(0, &["APPMON"])
        .memory(5)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let src = r.place_on_field(0, "HOSTFILT", Some(0));
    let host_ok = r.place_on_field(0, "HOST-OK", Some(0));
    let _host_no = r.place_on_field(0, "HOST-NO", Some(0));
    advance_to_main(&mut r);

    r.game.fire_on_play(0, src.index as usize);

    // Single source zone (hand) → straight to the card select; pick APPMON.
    let card_action = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand card select installed")
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("APPMON candidate");
    r.game.resolve_selection(0, card_action).expect("pick card");

    // Host select: only HOST-OK is eligible (HOST-NO lacks [Marked]; HOSTFILT is
    // the excluded source even though it carries [Marked]).
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("host select installed");
    assert_eq!(
        pending.valid_action_ids,
        vec![encode_attack(0, host_ok.index as u16)],
        "only the [Marked] OTHER Digimon (HOST-OK) is offered as a host"
    );
    r.game
        .resolve_selection(0, encode_attack(0, host_ok.index as u16))
        .expect("pick host");

    assert_eq!(
        r.game.player(0).battle_area[host_ok.index as usize]
            .linked_cards
            .len(),
        1,
        "APPMON linked onto HOST-OK"
    );
}

/// §4.5.1 — `relink_self_to_own_digimon` moves the effect's own standing
/// permanent to become a link card on a chosen OTHER own Digimon, honoring the
/// host_filter (link requirement) and always excluding self.
#[test]
fn relink_self_to_own_digimon_absorbs_self_onto_filtered_other_host() {
    let yaml = r#"
card: RELINKER
name: Relinker
kind: digimon
effects:
  - when: on_play
    summary: "link this Digimon to a [Marked] OTHER Digimon"
    process:
      - relink_self_to_own_digimon:
          host_filter: { trait_has: Marked }
"#;
    let mut r = DebugRunner::builder()
        // RELINKER itself carries [Marked], so only the exclude-self rule (not
        // host_filter) can drop it from the host candidates.
        .add_card(traited_digimon("RELINKER", CardColor::Red, &["Marked"]))
        .add_card(traited_digimon("HOST-OK", CardColor::Red, &["Marked"]))
        .add_card(traited_digimon("HOST-NO", CardColor::Red, &[]))
        .memory(5)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let src = r.place_on_field(0, "RELINKER", Some(0));
    let _ok = r.place_on_field(0, "HOST-OK", Some(0));
    let _no = r.place_on_field(0, "HOST-NO", Some(0));
    advance_to_main(&mut r);

    let self_card = r.game.player(0).battle_area[src.index as usize]
        .top_card()
        .handle();
    r.game.fire_on_play(0, src.index as usize);

    // Host select: only HOST-OK (Marked, not self). RELINKER is excluded despite
    // carrying [Marked]; HOST-NO lacks it.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("relink host select installed");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the [Marked] OTHER Digimon is offered as a relink host"
    );
    let action = pending.valid_action_ids[0];
    r.game.resolve_selection(0, action).expect("pick host");

    // RELINKER absorbed: the battle area shrank by 1 and some permanent now
    // hosts RELINKER's top card as a link card.
    assert_eq!(
        r.battle_area_size(0),
        2,
        "RELINKER left the battle area (absorbed as a link card)"
    );
    let hosted = r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.linked_cards.iter().any(|c| c.handle() == self_card));
    assert!(
        hosted,
        "RELINKER's top card is now a link card on the chosen host"
    );
}

/// make-engine-cloneable: `relink_self_to_own_digimon`'s host select is the
/// resumable-VM `FieldPermanent { post: AbsorbStandingAsLink }`, so cloning the
/// game AT the host prompt is faithful — the clone resolves to the absorb while
/// the original is untouched and replays identically.
#[test]
fn relink_self_to_own_digimon_clones_faithfully_at_host_prompt() {
    let yaml = r#"
card: RELINKER
name: Relinker
kind: digimon
effects:
  - when: on_play
    summary: "link this Digimon to a [Marked] OTHER Digimon"
    process:
      - relink_self_to_own_digimon:
          host_filter: { trait_has: Marked }
"#;
    let mut r = DebugRunner::builder()
        .add_card(traited_digimon("RELINKER", CardColor::Red, &["Marked"]))
        .add_card(traited_digimon("HOST-OK", CardColor::Red, &["Marked"]))
        .add_card(traited_digimon("HOST-NO", CardColor::Red, &[]))
        .memory(5)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let src = r.place_on_field(0, "RELINKER", Some(0));
    let _ok = r.place_on_field(0, "HOST-OK", Some(0));
    let _no = r.place_on_field(0, "HOST-NO", Some(0));
    advance_to_main(&mut r);

    let self_card = r.game.player(0).battle_area[src.index as usize]
        .top_card()
        .handle();
    r.game.fire_on_play(0, src.index as usize);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("relink host select installed");
    assert!(
        r.game.pending_selection_resume.is_some(),
        "relink host select must be resume-driven (clone-safe)"
    );
    let action = pending.valid_action_ids[0];

    // Clone AT the host prompt; resolve the absorb on the clone only.
    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, action)
        .expect("clone resolves host pick");
    assert_eq!(
        clone.player(0).battle_area.len(),
        2,
        "clone: RELINKER absorbed (battle area shrank by 1)"
    );
    assert!(
        clone
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.linked_cards.iter().any(|c| c.handle() == self_card)),
        "clone: RELINKER's top card is now a link card on the chosen host"
    );

    // INDEPENDENCE: the original is untouched by resolving the clone.
    assert!(
        r.game.pending_selection.is_some(),
        "original's host select survives cloning + resolving the clone"
    );
    assert_eq!(
        r.battle_area_size(0),
        3,
        "original still has all 3 permanents while the clone absorbed"
    );

    // REPLAYS IDENTICALLY: resolving the original the same way reaches the
    // clone's state.
    r.game
        .resolve_selection(0, action)
        .expect("original resolves host pick");
    assert_eq!(
        r.battle_area_size(0),
        clone.player(0).battle_area.len(),
        "original reaches the same battle-area size as the clone"
    );
    assert!(
        r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.linked_cards.iter().any(|c| c.handle() == self_card)),
        "original: RELINKER absorbed onto the chosen host"
    );
}

/// Recursive scan for a step matching `pred` anywhere in a (possibly nested)
/// step tree — descends If/Optional/ForEach bodies.
fn ex11_step_tree_has(steps: &[CompiledStep], pred: &dyn Fn(&CompiledStep) -> bool) -> bool {
    steps.iter().any(|s| {
        pred(s)
            || match s {
                CompiledStep::If {
                    then, else_branch, ..
                } => ex11_step_tree_has(then, pred) || ex11_step_tree_has(else_branch, pred),
                CompiledStep::Optional(b) => ex11_step_tree_has(b, pred),
                CompiledStep::ForEach { body, .. } => ex11_step_tree_has(body, pred),
                _ => false,
            }
    })
}

/// §4.5.5 — EX11-027 Maquinamon loads as PURE DSL (off raw_rust) and its
/// on_play wires the new link substrate: the heterogeneous choice routes to
/// `relink_self_to_own_digimon` (link this) and `link_cards` (link from hand).
#[test]
fn ex11_027_pure_dsl_on_play_wires_relink_and_linkcards() {
    use digimon_dsl::compiled::{
        CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
    };
    let r = DebugRunner::builder()
        .dsl_card("EX11-027")
        .expect("EX11-027 loads from the pack as pure DSL")
        .start();
    let card = r.compiled_card("EX11-027").expect("present in pack");
    assert_eq!(card.name, "Maquinamon");

    // [Link] [Maquinamon] in text: Cost 2.
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 2
        )),
        "EX11-027 declares a [Link] condition with cost 2"
    );

    // [On Play] clause whose tree uses BOTH new link verbs.
    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("on_play clause present");
    assert!(
        ex11_step_tree_has(&on_play.process, &|s| matches!(
            s,
            CompiledStep::RelinkSelfToOwnDigimon { .. }
        )),
        "on_play uses relink_self_to_own_digimon (Link this Maquinamon)"
    );
    assert!(
        ex11_step_tree_has(&on_play.process, &|s| matches!(
            s,
            CompiledStep::LinkCards { .. }
        )),
        "on_play uses link_cards (Link a Maquinamon from hand)"
    );
}

/// §4.5.5 — EX11-027's link-ESS leave-prevention: when the host this Maquinamon
/// is linked to would leave, placing a link card as the host's bottom
/// digivolution card keeps it on the field (the §4.5.4 cost wired into a card).
#[test]
fn ex11_027_linked_leave_prevention_places_link_card_as_bottom_source() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-027")
        .expect("EX11-027 loads")
        .add_card(digimon_card("HOST", CardColor::Red))
        .memory(0)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    // Maquinamon attached onto HOST as a link card → its scope:linked ESS is active.
    let maq = r.push_linked_owned(host, "EX11-027", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    let sources_before = r.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();

    r.game.delete_permanent_with_effects(host);
    assert!(
        r.game.pending_selection.is_some(),
        "EX11-027 link-ESS offers the optional leave-prevention prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the leave-prevention");
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card selection installed")
        .valid_action_ids[0];
    r.game
        .resolve_selection(0, pick)
        .expect("pick the link card");

    let host_ref = &r.game.player(0).battle_area[host.index as usize];
    assert_eq!(
        r.battle_area_size(0),
        1,
        "HOST did not leave the battle area"
    );
    assert!(
        host_ref.card_sources.iter().any(|c| c.handle() == maq),
        "Maquinamon was placed as a digivolution source under HOST"
    );
    assert_eq!(
        host_ref.card_sources.len(),
        sources_before + 1,
        "exactly one link card moved into the digivolution sources"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "the link card was NOT trashed (it was placed under the carrier)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Gap 5 — predicated host-side `WhenWouldLink` cost reduction.
//
// BT25-004 Tapmon (Digi-Egg, inherited) / BT25-045 Onmon (face-up):
//   "[Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game] trait card
//    would link to this Digimon, you may reduce the cost by 1."
//
// The reducer lives on the HOST. It fires only when:
//   (a) the card is linking onto THIS permanent (`pending_link_host` == self),
//   (b) the linking card has one of the required traits, and
//   (c) it is the controller's turn.
// It is OPTIONAL (accept/decline exposed to the RL action space) and capped at
// once per turn. The accept-branch reduces the imminent link cost by 1 via
// `reduce_pending_link_cost`; it does NOT cancel the link.
// ═══════════════════════════════════════════════════════════════════════════

use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementSubject;

/// A Digimon at the given trait set — used as the standing link SOURCE.
fn traited_digimon(card_id: &str, color: CardColor, traits: &[&str]) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd.traits = traits.iter().map(|s| s.to_string()).collect();
    cd
}

/// True when the replacement SUBJECT card carries one of the required traits.
fn subject_card_has_required_trait(
    rctx: &digimon_engine::effect_context::EffectReadContext<'_>,
    subject: &ReplacementSubject,
) -> bool {
    const REQUIRED: &[&str] = &["Social", "Tool", "Game"];
    let card = match subject {
        ReplacementSubject::Card(handle, _zone) => *handle,
        _ => return false,
    };
    let Some(data) = rctx.game.card_data_for_handle(card) else {
        return false;
    };
    data.traits.iter().any(|t| REQUIRED.iter().any(|r| r == t))
}

/// The faithful BT25-004/045 host-side reducer, hand-written for substrate TDD.
///
/// `Effect::when_would_link` builds the replacement at `WhenWouldLink`. The
/// `condition` gates on (host self-filter via `pending_link_host`) + (your
/// turn); the `replacement_condition` gates on the linking card's traits (it is
/// the only hook that can see the would-link SUBJECT card). The accept-branch
/// reduces the link cost by 1.
struct LinkCostReducer;
impl CardEffect for LinkCostReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_link(card)
            .name("Reduce link cost by 1")
            .optional()
            .once_per_turn()
            .condition(|rctx| {
                // (a) the card is linking onto THIS permanent, and
                // (c) it is the controller's turn.
                rctx.pending_link_host() == rctx.source_permanent
                    && rctx.game.turn_player() == rctx.player
            })
            // (b) the linking card carries a required trait.
            .replacement_condition(|rctx, subject| subject_card_has_required_trait(rctx, subject))
            .replacement_process(|rctx| {
                rctx.effect.reduce_pending_link_cost(1);
            })
            .build()]
    }
}

/// A standing Appmon-style link SOURCE: a static `link_condition` declaring it
/// may link onto any of the controller's Digimon for `cost`.
struct StandingLinkSource {
    cost: u16,
}
impl CardEffect for StandingLinkSource {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::link_condition(card)
            .name("Link onto any ally")
            .link_host(self.cost, |_rctx, _host| true)
            .build()]
    }
}

/// Drive the standing-link path through the action mask + decode, exactly as
/// the RL agent would: find the FIELD_EFFECT link bit for `source_idx`, take
/// it (installs the host-selection), then pick `host_idx`. Returns after
/// `begin_digimon_link` has fired the `WhenWouldLink` window — any optional
/// reducer prompt is now the live `pending_selection`.
fn activate_standing_link(r: &mut DebugRunner, source_idx: usize, host_idx: u8) {
    use digimon_engine::action::space::{
        encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    // 1. The FIELD_EFFECT link bit must be legal in the mask.
    let link_bit =
        FIELD_EFFECT_START + source_idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK;
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[link_bit as usize], 1.0,
        "standing link FIELD_EFFECT bit must be legal for source {source_idx}"
    );
    // 2. Take it — installs the own-field host-selection prompt.
    r.game.decode_action(link_bit, 0);
    // 3. Pick the host. The host-selection encodes targets as encode_attack(0, idx).
    let host_action = encode_attack(0, host_idx as u16);
    r.game
        .resolve_selection(0, host_action)
        .expect("resolve host-selection");
}

/// Build a runner with P0's link source + host already on the field and the
/// turn advanced to P0's main phase. Returns (source_handle, host_handle).
fn setup_gap5(
    r_traits: &[&str],
    link_cost: u16,
    reducer_on_host: bool,
) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(traited_digimon("LINK-SRC", CardColor::Red, r_traits))
        .add_card(traited_digimon("HOST", CardColor::Red, &[]))
        .memory(10)
        .start();
    r.register_effect("LINK-SRC", Arc::new(StandingLinkSource { cost: link_cost }));
    if reducer_on_host {
        r.register_effect("HOST", Arc::new(LinkCostReducer));
    }
    let host = r.place_on_field(0, "HOST", Some(0));
    let source = r.place_on_field(0, "LINK-SRC", Some(0));
    advance_to_main(&mut r);
    (r, source, host)
}

/// Gap 5 — accepting the optional reduce lowers the paid link cost by 1.
#[test]
fn gap5_predicated_reduce_lowers_paid_cost() {
    let (mut r, source, host) = setup_gap5(&["Social"], 3, true);
    let before = r.memory();
    activate_standing_link(&mut r, source.index as usize, host.index);

    // The optional reducer prompt is live — accept it.
    assert!(
        r.pending_is_optional(),
        "matching-trait link must offer the optional reduce"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept reduce");

    // Cost 3 reduced to 2 — memory dropped by 2, not 3.
    assert_eq!(
        r.memory(),
        before - 2,
        "accepting the reduce must pay cost-1 (3 -> 2)"
    );
    // The source actually linked onto the host.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the source must attach to the host after the reduced cost is paid"
    );
}

/// Gap 5 — declining the optional reduce pays the full link cost.
#[test]
fn gap5_predicated_reduce_declined_pays_full() {
    let (mut r, source, host) = setup_gap5(&["Tool"], 3, true);
    let before = r.memory();
    activate_standing_link(&mut r, source.index as usize, host.index);

    assert!(
        r.pending_is_optional(),
        "matching-trait link must offer the optional reduce"
    );
    r.game.resolve_selection(0, PASS).expect("decline reduce");

    assert_eq!(
        r.memory(),
        before - 3,
        "declining the reduce must pay the full cost (3)"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the link still resolves at full cost after declining"
    );
}

/// Gap 5 — a non-matching-trait linking card gets no reduce offer (full cost).
#[test]
fn gap5_predicated_reduce_wrong_trait_no_offer() {
    // The source carries no required trait — the reducer must not fire.
    let (mut r, source, host) = setup_gap5(&["Vaccine"], 3, true);
    let before = r.memory();
    activate_standing_link(&mut r, source.index as usize, host.index);

    // No optional reducer prompt installs; the link resolves directly at full
    // cost.
    assert!(
        r.game.pending_selection.is_none(),
        "a non-matching-trait link must not install the reduce prompt"
    );
    assert_eq!(
        r.memory(),
        before - 3,
        "a non-matching-trait link pays the full cost (3)"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the link resolves at full cost"
    );
}

/// Gap 5 — the reduce is once per turn: a second matching link the same turn is
/// not reduced.
#[test]
fn gap5_predicated_reduce_once_per_turn() {
    // Two standing matching-trait link sources + one host with the reducer.
    let mut r = DebugRunner::builder()
        .add_card(traited_digimon("LINK-A", CardColor::Red, &["Social"]))
        .add_card(traited_digimon("LINK-B", CardColor::Red, &["Game"]))
        .add_card(traited_digimon("HOST", CardColor::Red, &[]))
        .memory(20)
        .start();
    r.register_effect("LINK-A", Arc::new(StandingLinkSource { cost: 3 }));
    r.register_effect("LINK-B", Arc::new(StandingLinkSource { cost: 3 }));
    r.register_effect("HOST", Arc::new(LinkCostReducer));
    let host = r.place_on_field(0, "HOST", Some(0));
    let src_a = r.place_on_field(0, "LINK-A", Some(0));
    let _src_b = r.place_on_field(0, "LINK-B", Some(0));
    advance_to_main(&mut r);

    // First link — accept the reduce (3 -> 2).
    let before_a = r.memory();
    activate_standing_link(&mut r, src_a.index as usize, host.index);
    assert!(r.pending_is_optional(), "first link offers the reduce");
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept first reduce");
    assert_eq!(r.memory(), before_a - 2, "first link pays cost-1 (3 -> 2)");

    // Second link the SAME turn — once-per-turn is spent, so no reduce prompt
    // and the full cost is paid. Absorbing src_a (a later slot) shifted the
    // Vec, so re-resolve LINK-B's current battle-area index. The host is
    // index 0 (placed first), so it never shifts.
    let src_b_idx = r.game.player(0).battle_area.iter().position(|p| {
        p.card_sources
            .last()
            .map(|c| c.card_id(&r.game.card_data) == "LINK-B")
            .unwrap_or(false)
    });
    let src_b_idx = src_b_idx.expect("LINK-B still standing on the field");
    let before_b = r.memory();
    activate_standing_link(&mut r, src_b_idx, host.index);
    assert!(
        r.game.pending_selection.is_none(),
        "the second link the same turn must not offer the reduce (once per turn)"
    );
    assert_eq!(
        r.memory(),
        before_b - 3,
        "the second link pays the full cost (3)"
    );
}

/// Gap 5 (DSL) — author BT25-045 Onmon's reducer clause in YAML and prove the
/// reduce lowers the paid link cost.
#[test]
fn gap5_dsl_when_would_link_to_this() {
    const YAML: &str = "card: BT25-045\nname: Onmon\nkind: digimon\nlevel: 4\ncolor: [red]\ncost: 5\ndp: 5000\ntraits: [Appliance, Social]\neffects:\n  - when: when_would_link_to_this\n    optional: true\n    once_per_turn: true\n    active_when:\n      would_link_card_trait_any_of: [Social, Tool, Game]\n    process:\n      - reduce_link_cost: { amount: 1 }\n";
    let mut r = DebugRunner::builder()
        .add_card(traited_digimon("LINK-SRC", CardColor::Red, &["Tool"]))
        .add_card(traited_digimon("BT25-045", CardColor::Red, &[]))
        .memory(10)
        .start();
    r.register_effect("LINK-SRC", Arc::new(StandingLinkSource { cost: 3 }));
    register_dsl_yaml(&mut r, YAML);
    let host = r.place_on_field(0, "BT25-045", Some(0));
    let source = r.place_on_field(0, "LINK-SRC", Some(0));
    advance_to_main(&mut r);

    let before = r.memory();
    activate_standing_link(&mut r, source.index as usize, host.index);
    assert!(
        r.pending_is_optional(),
        "the DSL-authored reducer must offer the optional reduce"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept reduce");

    assert_eq!(
        r.memory(),
        before - 2,
        "the DSL reducer must pay cost-1 (3 -> 2)"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the source must attach after the reduced cost is paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Gap 3a — link-card-trash leave-replacement.
//
// BT25-066 / BT25-073 (inherited) / BT25-101:
//   "[All Turns] When this Digimon would leave the battle area, by trashing 1
//    of its link cards, it doesn't leave."
//
// An OPTIONAL `WhenWouldLeaveBattleArea` replacement. The cost is trashing one
// of the LEAVING permanent's OWN link cards. If paid → the leave is cancelled
// (the Digimon stays). Gated on the permanent having ≥1 link card. When >1 the
// player CHOOSES which to trash (exposed to the RL action space). The trashed
// link card routes to its owner's trash and fires `OnLinkedCardTrashed`.
//
// DCGO ref: OnTrashLinkCard.cs (CanUse: has a link card) + TrashLinkedCards.cs.
// ═══════════════════════════════════════════════════════════════════════════

/// The faithful BT25-066 leave-replacement, hand-written for substrate TDD.
///
/// `Effect::when_would_leave_battle_area` builds the replacement. The clause is
/// `.optional()` (the printed "by trashing … it doesn't leave" = you may pay).
/// The `replacement_condition` gates on the leaving subject being THIS permanent
/// AND it having ≥1 link card (so it is not offered when there are no link
/// cards). The accept-branch installs the link-card-trash selection via the new
/// engine primitive, which trashes the chosen link card and cancels the leave.
struct LinkTrashLeaveReplacement;
impl CardEffect for LinkTrashLeaveReplacement {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_leave_battle_area(card)
            .name("By trashing 1 link card, it doesn't leave")
            .optional()
            .replacement_condition(|rctx, subject| {
                let Some(handle) = subject.permanent() else {
                    return false;
                };
                // Self-filter: only THIS permanent.
                if rctx.source_permanent != Some(handle) {
                    return false;
                }
                // Gate: must have ≥1 link card to pay the cost.
                rctx.game
                    .player(handle.player)
                    .battle_area
                    .get(handle.index as usize)
                    .is_some_and(|p| !p.linked_cards.is_empty())
            })
            .replacement_process(|rctx| {
                if let Some(host) = rctx.subject.permanent() {
                    rctx.effect.trash_own_link_card_and_cancel_leave(host);
                }
            })
            .build()]
    }
}

/// Gap 3a test 1 — accept the replacement: a link card is trashed and the
/// Digimon stays on the field.
#[test]
fn gap3a_leave_replacement_trashes_link_card_and_stays() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("STAYER", CardColor::Red))
        .add_card(digimon_card("LINKEE", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("STAYER", Arc::new(LinkTrashLeaveReplacement));
    let perm = r.place_on_field(0, "STAYER", Some(0));
    r.push_linked_owned(perm, "LINKEE", 0);
    advance_to_main(&mut r);

    assert_eq!(
        r.game.player(0).battle_area[perm.index as usize]
            .linked_cards
            .len(),
        1
    );
    let trash_before = r.trash_size(0);

    // Trigger a leave (effect deletion). The optional replacement prompt installs.
    r.game.delete_permanent_with_effects(perm);
    assert!(
        r.game.pending_selection.is_some(),
        "leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the leave replacement");

    // The which-link-card-to-trash choice is surfaced even with one link card
    // (no auto-selection). Resolve the single-option selection.
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("link-card-trash selection installed")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    // After accept + pick, the Digimon remains and the link card is trashed.
    assert_eq!(r.battle_area_size(0), 1, "the Digimon did NOT leave");
    assert_eq!(
        r.game.player(0).battle_area[perm.index as usize]
            .linked_cards
            .len(),
        0,
        "the link card was trashed as the cost"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly one card (the link card) went to trash"
    );
}

/// Gap 3a test 2 — decline the replacement: no link card trashed, the Digimon
/// leaves normally.
#[test]
fn gap3a_leave_replacement_declined_leaves() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("STAYER", CardColor::Red))
        .add_card(digimon_card("LINKEE", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("STAYER", Arc::new(LinkTrashLeaveReplacement));
    let perm = r.place_on_field(0, "STAYER", Some(0));
    r.push_linked_owned(perm, "LINKEE", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(perm);
    assert!(r.game.pending_selection.is_some());

    r.game
        .resolve_selection(0, PASS)
        .expect("decline the leave replacement");

    assert_eq!(r.battle_area_size(0), 0, "declining lets the Digimon leave");
    // Both the leaving top card and its link card go to trash (host deletion
    // trashes link cards). Net: trash grows by 2.
    assert_eq!(
        r.trash_size(0),
        trash_before + 2,
        "top card + link card both trashed on the (un-prevented) leave"
    );
}

/// Gap 3a test 3 — with 0 link cards, the replacement is NOT offered; the
/// Digimon leaves.
#[test]
fn gap3a_leave_replacement_no_link_cards_not_offered() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("STAYER", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("STAYER", Arc::new(LinkTrashLeaveReplacement));
    let perm = r.place_on_field(0, "STAYER", Some(0));
    advance_to_main(&mut r);

    assert!(r.game.player(0).battle_area[perm.index as usize]
        .linked_cards
        .is_empty());

    r.game.delete_permanent_with_effects(perm);
    assert!(
        r.game.pending_selection.is_none(),
        "no link cards → no replacement prompt"
    );
    assert_eq!(r.battle_area_size(0), 0, "the Digimon leaves normally");
}

/// Gap 3a test 4 — with >1 link card the player chooses WHICH to trash. The
/// trash-choice surfaces as a multi-option selection; the chosen one is trashed
/// and the Digimon stays.
#[test]
fn gap3a_leave_replacement_chooses_which_link_card() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("STAYER", CardColor::Red))
        .add_card(digimon_card("LINK-A", CardColor::Red))
        .add_card(digimon_card("LINK-B", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("STAYER", Arc::new(LinkTrashLeaveReplacement));
    let perm = r.place_on_field(0, "STAYER", Some(0));
    let link_a = r.push_linked_owned(perm, "LINK-A", 0);
    let _link_b = r.push_linked_owned(perm, "LINK-B", 0);
    advance_to_main(&mut r);

    assert_eq!(
        r.game.player(0).battle_area[perm.index as usize]
            .linked_cards
            .len(),
        2
    );

    r.game.delete_permanent_with_effects(perm);
    // Accept the optional replacement.
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Now a which-link-card-to-trash selection is live with 2 options.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card selection installed");
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "both link cards are offered as trash choices"
    );
    // Choose the first (LINK-A).
    let action = pending.valid_action_ids[0];
    r.game.resolve_selection(0, action).expect("pick link card");

    assert_eq!(r.battle_area_size(0), 1, "the Digimon stays");
    let linked = &r.game.player(0).battle_area[perm.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "exactly one link card remains");
    assert_eq!(
        linked[0].handle(),
        _link_b,
        "the UN-chosen link card (LINK-B) remains; LINK-A was trashed"
    );
    assert!(
        r.game.player(0).trash.iter().any(|c| c.handle() == link_a),
        "the chosen link card (LINK-A) is in trash"
    );
}

#[test]
fn gap3a_link_card_trash_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("STAYER", CardColor::Red))
        .add_card(digimon_card("LINK-A", CardColor::Red))
        .add_card(digimon_card("LINK-B", CardColor::Red))
        .memory(0)
        .start();
    r.register_effect("STAYER", Arc::new(LinkTrashLeaveReplacement));
    let perm = r.place_on_field(0, "STAYER", Some(0));
    let link_a = r.push_linked_owned(perm, "LINK-A", 0);
    let _link_b = r.push_linked_owned(perm, "LINK-B", 0);
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(perm);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    assert!(
        r.game.pending_selection_resume.is_some(),
        "link-card trash prompt must be resume-driven before cloning"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];

    let mut cloned = r.game.clone();
    cloned
        .resolve_selection(0, action)
        .expect("clone resolves chosen link card");

    assert_eq!(cloned.player(0).battle_area.len(), 1, "clone: host stays");
    let linked = &cloned.player(0).battle_area[perm.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "clone: exactly one link card remains");
    assert!(
        cloned.player(0).trash.iter().any(|c| c.handle() == link_a),
        "clone: chosen link card is trashed"
    );
}

/// Gap 3a test 5 (DSL) — author BT25-066's clause in YAML and prove it.
#[test]
fn gap3a_dsl_leave_replacement() {
    let yaml = r#"
card: BT25-066
name: Leave-Stayer
kind: digimon
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    cost: { trash_own_link_card: true }
    outcome: prevent
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("BT25-066", CardColor::Red))
        .add_card(digimon_card("LINKEE", CardColor::Red))
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let perm = r.place_on_field(0, "BT25-066", Some(0));
    r.push_linked_owned(perm, "LINKEE", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(perm);
    assert!(
        r.game.pending_selection.is_some(),
        "DSL leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Resolve the which-link-card-to-trash choice (surfaced even with one card).
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("link-card-trash selection installed")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    assert_eq!(r.battle_area_size(0), 1, "the Digimon did NOT leave");
    assert_eq!(
        r.game.player(0).battle_area[perm.index as usize]
            .linked_cards
            .len(),
        0,
        "the link card was trashed as the cost"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly the link card went to trash"
    );
}

/// Gap 3a (DSL) — with 0 link cards the DSL clause is not offered and the
/// Digimon leaves (preflight gate on link-card presence).
#[test]
fn gap3a_dsl_leave_replacement_no_link_cards_not_offered() {
    let yaml = r#"
card: BT25-066
name: Leave-Stayer
kind: digimon
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    cost: { trash_own_link_card: true }
    outcome: prevent
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("BT25-066", CardColor::Red))
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let perm = r.place_on_field(0, "BT25-066", Some(0));
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(perm);
    assert!(
        r.game.pending_selection.is_none(),
        "DSL: no link cards → no replacement prompt"
    );
    assert_eq!(r.battle_area_size(0), 0, "the Digimon leaves normally");
}

/// §4.5.4 (EX11-027) — leave-replacement that PLACES a chosen link card as the
/// carrier's BOTTOM digivolution card instead of trashing it: the Digimon
/// stays, the link card relocates under it, and nothing is trashed.
#[test]
fn place_link_card_as_bottom_source_leave_replacement() {
    let yaml = r#"
card: EX11-027-PLACE
name: Place-Stayer
kind: digimon
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    cost: { place_link_card_as_bottom_digivolution: true }
    outcome: prevent
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("EX11-027-PLACE", CardColor::Red))
        .add_card(digimon_card("LINKEE", CardColor::Red))
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let perm = r.place_on_field(0, "EX11-027-PLACE", Some(0));
    let linkee = r.push_linked_owned(perm, "LINKEE", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    let sources_before = r.game.player(0).battle_area[perm.index as usize]
        .card_sources
        .len();

    r.game.delete_permanent_with_effects(perm);
    assert!(
        r.game.pending_selection.is_some(),
        "place-as-bottom leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Surface + resolve the which-link-card choice (one card here).
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("link-card selection installed")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    let perm_ref = &r.game.player(0).battle_area[perm.index as usize];
    assert_eq!(r.battle_area_size(0), 1, "the Digimon did NOT leave");
    assert_eq!(
        perm_ref.linked_cards.len(),
        0,
        "the link card left linked_cards"
    );
    assert_eq!(
        perm_ref.card_sources.len(),
        sources_before + 1,
        "the link card became a digivolution source under the carrier"
    );
    assert_eq!(
        perm_ref.card_sources[0].handle(),
        linkee,
        "the chosen link card is now the BOTTOM digivolution source"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "the link card was NOT trashed (it was placed under the carrier)"
    );
}

#[test]
fn place_link_card_as_bottom_source_prompt_clones_faithfully() {
    let yaml = r#"
card: EX11-027-PLACE
name: Place-Stayer
kind: digimon
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    cost: { place_link_card_as_bottom_digivolution: true }
    outcome: prevent
"#;
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("EX11-027-PLACE", CardColor::Red))
        .add_card(digimon_card("LINKEE", CardColor::Red))
        .memory(0)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let perm = r.place_on_field(0, "EX11-027-PLACE", Some(0));
    let linkee = r.push_linked_owned(perm, "LINKEE", 0);
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(perm);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    assert!(
        r.game.pending_selection_resume.is_some(),
        "place-link-card prompt must be resume-driven before cloning"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];

    let mut cloned = r.game.clone();
    cloned
        .resolve_selection(0, action)
        .expect("clone resolves chosen link card");

    let perm_ref = &cloned.player(0).battle_area[perm.index as usize];
    assert_eq!(perm_ref.linked_cards.len(), 0, "clone: link slot emptied");
    assert_eq!(
        perm_ref.card_sources[0].handle(),
        linkee,
        "clone: chosen link card is now the bottom source"
    );
    assert!(
        cloned.player(0).trash.iter().all(|c| c.handle() != linkee),
        "clone: placed link card was not trashed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Gap 3b — Option self-as-link-source (BT25-101 Divine Arms Version Ω).
//
//   "By trashing 1 [TS] trait card from your hand, Draw 2. After, you may link
//    THIS CARD or 1 [TS] trait card from your trash to 1 of your Digimon
//    without paying the cost."
//
// "link this card" = the Option being played attaches ITSELF as a persistent
// link card onto a chosen Digimon (instead of going to trash on dispose). The
// `link_cards` step is extended with a `self_option` from-zone.
// ═══════════════════════════════════════════════════════════════════════════

/// Gap 3b — a `link_cards { from: [self_option], to: own_digimon }` Option
/// attaches ITSELF as a link card onto the chosen host (not trashed on dispose).
#[test]
fn gap3b_option_links_itself_to_host() {
    let yaml = r#"
card: BT25-101
name: Divine Arms Version Omega
kind: option
effects:
  - when: main
    process:
      - link_cards:
          from: [self_option]
          to: own_digimon
          count: { exactly: 1 }
          cost: free
"#;

    let mut r = DebugRunner::builder()
        .add_card(option_card("BT25-101", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .hand(0, &["BT25-101"])
        .memory(3)
        .start();
    register_dsl_yaml(&mut r, yaml);
    let host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the self-link option parks for the host selection"
    );

    // Resolve the host-select prompt.
    let host_action = {
        use digimon_engine::action::space::encode_attack;
        encode_attack(host.player as u16, host.index as u16)
    };
    let _ = r.game.resolve_selection(0, host_action);

    // The Option attached ITSELF as a link card; it did NOT go to trash.
    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "the Option attached itself as a link card");
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "the self-linked Option is NOT trashed on dispose"
    );
    assert_eq!(r.hand_size(0), 0, "the Option left the hand");
}
