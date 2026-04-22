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

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
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
        r.game
            .pending_option
            .as_ref()
            .unwrap()
            .resolution_phase,
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
    let bit_phantom = digimon_engine::action::space::encode_attack(attacker.index as u16, 1)
        as usize;
    assert_eq!(
        mask[bit_phantom], 0.0,
        "no attack bit for a non-existent slot"
    );
    // Direct attack to the phantom linked slot must also be rejected.
    let phantom = digimon_engine::permanent::PermanentHandle { player: 0, index: 1 };
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
