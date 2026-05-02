//! Phase 8 Task 8 — Behavioral end-to-end scenarios.
//!
//! Integration tests that exercise multiple Phase 8 Option subtypes together
//! across a multi-turn game. These catch cross-subtype interaction bugs that
//! the per-flow unit tests (Tasks 2/3/4/5) miss.
//!
//! Counter-timed Options are deferred to Phase 9 and are NOT exercised here;
//! the "multi-turn" scenario below combines Standard + Delay + Link.

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{
    CardColor, CardKind, DelayTrigger, EffectTiming, Expiry, ModifierType,
};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Standard Option: bumps a witness counter, no memory mutation. We avoid
/// gain_memory here so the multi-turn flow doesn't accidentally rotate
/// turns via `check_turn_end` between our explicit `end_turn` calls.
struct StandardWitnessOnly(Arc<Mutex<u32>>);
impl CardEffect for StandardWitnessOnly {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("Standard: witness")
            .option_main()
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Delay Option: at EndOfYourNextTurn, draw 1 card for the owner. The
/// witness records that the body fired.
struct DelayDrawOneNextTurn(Arc<Mutex<u32>>);
impl CardEffect for DelayDrawOneNextTurn {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("Delay: draw 1 at end of your next turn")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(move |ctx| {
                let owner = ctx.player;
                ctx.draw(owner, 1);
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Link Option with a sideways +2000 DP buff on OnLink. Plug-In style:
/// after attach, the linked card's OnLink fires via the host's dispatch
/// (because of `.linked()`), and the body installs a permanent DP modifier
/// on the host. The Option's attach itself is declared via `.link()`.
struct PlugInDpBuffLink;
impl CardEffect for PlugInDpBuffLink {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            // Link declaration + empty OptionMain body.
            Effect::on_play(card)
                .name("Link: attach to any digimon")
                .link(0, |_ctx, _host| true)
                .process(|_| {})
                .build(),
            // Sideways OnLink buff: fires via the host's OnLink dispatch
            // after attach (because `.linked()` opts into host-side routing),
            // installs a permanent +2000 DP modifier on the host.
            Effect::on_play(card)
                .name("Link: +2000 DP on host (sideways)")
                .timing(EffectTiming::OnLink)
                .linked()
                .process(|ctx| {
                    if let Some(host) = ctx.source_permanent {
                        ctx.add_dp_modifier(host, 2000, Expiry::Permanent);
                    }
                })
                .build(),
        ]
    }
}

/// Plain Plug-In with just a link declaration — no sideways effects. Used
/// by Test 2 to exercise the host-deletion cascade without entangling
/// with multi-trigger `TriggerOrder` prompts that the DP-buff variant
/// would install when two Plug-Ins share an OnLink timing.
struct PlugInBare;
impl CardEffect for PlugInBare {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Plug-In: link any digimon")
            .link(0, |_ctx, _host| true)
            .process(|_| {})
            .build()]
    }
}

/// Observer that bumps a counter when the host fires `OnDeletion`.
/// Used by Test 2 to assert the host's deletion hook still fires when the
/// linked-card cascade runs.
struct OnDeletionWitness(Arc<Mutex<u32>>);
impl CardEffect for OnDeletionWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_deletion(card)
            .name("OnDeletion witness")
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Observer that bumps a counter when `OnLinkedCardTrashed` fires.
struct OnLinkedTrashedWitness(Arc<Mutex<u32>>);
impl CardEffect for OnLinkedTrashedWitness {
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

fn digimon_card(card_id: &str, color: CardColor, dp: i32) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd.dp = Some(dp);
    cd
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: Standard → Delay → Link across a multi-turn game.
///
/// Turn 1 (P0): plays a Standard Option. Body fires (+2 memory), card
/// lands in trash, `pending_option` cleared.
/// Turn 2 (P1): plays a Delay Option (EndOfYourNextTurn). Card parks as
/// a `Delayed` permanent, `pending_option` cleared.
/// Turn 3 (P0): plays a Link Option. A host-select `PendingSelection` is
/// installed; resolving it attaches the Option to P0's Digimon, the
/// sideways OnLink buff fires on the host (+2000 DP), `pending_option`
/// cleared.
/// Turn 4 (P1): end of this turn is P1's "next turn" relative to the
/// turn-2 Delay play — the Delay body fires and the permanent trashes.
///
/// Final: host's `effective_dp` reflects the Plug-In buff; no soft-locks;
/// every dispose cleared `pending_option`.
#[test]
fn multi_turn_standard_then_delay_then_link_flow_end_to_end() {
    let std_witness = Arc::new(Mutex::new(0u32));
    let delay_witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        // Option cards
        .add_card(option_card("STD-OPT", 0, CardColor::Red))
        .add_card(option_card("DELAY-OPT", 0, CardColor::Red))
        .add_card(option_card("LINK-OPT", 0, CardColor::Red))
        // Digimon on P0 (the one that receives the Link buff) + a
        // color-match/filler Digimon for P1.
        .add_card(digimon_card("P0-HOST", CardColor::Red, 5000))
        .add_card(digimon_card("P1-MATCH", CardColor::Red, 4000))
        .add_card(digimon_card("FILLER", CardColor::Red, 3000))
        // P0: Standard then Link in order played.
        .hand(0, &["STD-OPT", "LINK-OPT"])
        .hand(1, &["DELAY-OPT"])
        // Enough filler in each deck to survive 4-ish turns of end-of-turn
        // draws + the Delay's draw body.
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(0)
        .start();
    r.register_effect(
        "STD-OPT",
        Arc::new(StandardWitnessOnly(std_witness.clone())),
    );
    r.register_effect(
        "DELAY-OPT",
        Arc::new(DelayDrawOneNextTurn(delay_witness.clone())),
    );
    r.register_effect("LINK-OPT", Arc::new(PlugInDpBuffLink));

    // Seed the field: P0 has the to-be-linked host, P1 has a Red Digimon
    // so its Option color-match gate passes.
    let host = r.place_on_field(0, "P0-HOST", Some(0));
    r.place_on_field(1, "P1-MATCH", Some(0));

    // ── Turn 1 (P0): play Standard Option ────────────────────────────
    advance_to_main(&mut r);
    assert_eq!(r.turn_player(), 0);
    // Pin memory at 0 so `check_turn_end` doesn't auto-rotate during play
    // dispose — we want explicit, turn-at-a-time advancement.
    r.game.set_memory(0);
    let std_result = r.game.play_option_from_hand(0, 0);
    assert_eq!(
        std_result,
        OptionPlayResult::Trashed,
        "Standard Option disposes to trash"
    );
    assert_eq!(
        *std_witness.lock().unwrap(),
        1,
        "Standard OptionMain body fired once"
    );
    assert!(
        r.game.pending_option.is_none(),
        "pending_option cleared after Standard dispose"
    );
    assert_eq!(r.trash_size(0), 1, "Standard Option in P0 trash");

    // End P0's turn → P1's turn. Reset memory to 0 again so the seesaw
    // flip doesn't leave P1 at negative and trigger auto-end.
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    r.game.set_memory(0);

    // ── Turn 2 (P1): play Delay Option ───────────────────────────────
    advance_to_main(&mut r);
    let p1_battle_before = r.battle_area_size(1);
    let p1_turn_count = r.turn_count();
    let delay_result = r.game.play_option_from_hand(1, 0);
    assert_eq!(
        delay_result,
        OptionPlayResult::Trashed,
        "Delay Option returns Trashed from play_option_from_hand"
    );
    assert!(
        r.game.pending_option.is_none(),
        "pending_option cleared after Delay park"
    );
    assert_eq!(
        r.battle_area_size(1),
        p1_battle_before + 1,
        "Delay parks as a Delayed permanent on P1's field"
    );
    assert_eq!(
        *delay_witness.lock().unwrap(),
        0,
        "Delay body NOT fired yet"
    );
    // Verify the parked state is Delayed with the correct fire-turn
    // (EndOfYourNextTurn on P1's own turn = p1_turn_count + 2 → turn 4).
    let delay_idx = r.battle_area_size(1) - 1;
    match r.game.player(1).battle_area[delay_idx].option_state {
        OptionState::Delayed {
            owner,
            trash_on_turn,
        } => {
            assert_eq!(owner, 1);
            assert_eq!(
                trash_on_turn,
                p1_turn_count + 2,
                "EndOfYourNextTurn on own turn = turn + 2"
            );
        }
        ref other => panic!("expected Delayed state, got {:?}", other),
    }

    // End P1's turn → P0's turn 3.
    r.end_turn();
    assert_eq!(r.turn_player(), 0);
    assert_eq!(*delay_witness.lock().unwrap(), 0, "Delay still pending");
    r.game.set_memory(0);

    // ── Turn 3 (P0): play Link Option ────────────────────────────────
    advance_to_main(&mut r);
    let dp_before = r.effective_dp(host).expect("host has base DP");
    let link_result = r.game.play_option_from_hand(0, 0); // hand[0] = LINK-OPT
    assert_eq!(
        link_result,
        OptionPlayResult::Pending,
        "Link Option installs a host-select PendingSelection"
    );
    assert!(r.game.pending_selection.is_some(), "host selection live");
    assert!(
        r.game.pending_option.is_some(),
        "pending_option carries through LinkSelectHost"
    );
    // Resolve the host selection — Task 8 charter forbids auto-selection,
    // so we explicitly consume one of the valid action ids.
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action_id);

    assert!(
        r.game.pending_option.is_none(),
        "pending_option cleared after Link attach"
    );
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "Link Option attached to host"
    );
    // The host's effective DP should now reflect the sideways OnLink +2000.
    let dp_after_link = r.effective_dp(host).expect("host still on field");
    assert_eq!(
        dp_after_link,
        dp_before + 2000,
        "Plug-In OnLink sideways buff added +2000 DP to host"
    );
    // Sanity: the +2000 actually comes from the ChangeDp modifier registry.
    assert_eq!(
        r.modifiers().sum(host, ModifierType::ChangeDp),
        2000,
        "ChangeDp modifier is installed on the host"
    );

    // End P0 turn 3 → P1 turn 4.
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    r.game.set_memory(0);

    // ── Turn 4 (P1): end this turn, Delay fires ──────────────────────
    advance_to_main(&mut r);
    let p1_trash_before = r.trash_size(1);
    let p1_hand_before = r.hand_size(1);
    r.end_turn();
    assert_eq!(
        *delay_witness.lock().unwrap(),
        1,
        "Delay body fired at end of P1's next turn (turn 4)"
    );
    // Delay trashed + body drew one card.
    assert_eq!(
        r.trash_size(1),
        p1_trash_before + 1,
        "Delayed permanent moved into trash after its body fired"
    );
    assert_eq!(
        r.hand_size(1),
        p1_hand_before + 1,
        "Delay body drew one card for its owner"
    );
    let delayed_still_there = r
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| matches!(p.option_state, OptionState::Delayed { .. }));
    assert!(
        !delayed_still_there,
        "Delay permanent left P1 battle_area after resolving"
    );

    // Host is still linked + DP-buffed — the Delay resolving did not
    // disturb P0's linked permanent.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "host still has the Link attached after P1's Delay resolved"
    );
    assert_eq!(
        r.effective_dp(host).unwrap(),
        dp_before + 2000,
        "Plug-In DP buff persists across turn boundary"
    );
}

/// Test 2: Rocks-style host deletion cascade.
///
/// P0 has a host Digimon with 2 Plug-Ins linked. P1 deletes the host.
/// Expected (per Task 4 host-cascade semantics):
/// - Both linked cards land in P0's trash.
/// - `OnDeletion` fires for the host (witnessed on a bystander Digimon so
///   we don't need a self-referencing observer slot on the host's own
///   stack).
/// - `OnLinkedCardTrashed` fires **once per player battle-area scan**
///   (not once per linked card — Task 4 chose fire-once-per-trash-event
///   cardinality). With 2 players, the observer on P0's field sees it
///   exactly once.
#[test]
fn rocks_plug_in_host_deletion_cascade() {
    let on_deletion_witness = Arc::new(Mutex::new(0u32));
    let on_linked_trashed_witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        // Host: small DP so P1's attacker can delete it in one shot.
        .add_card(digimon_card("HOST", CardColor::Red, 3000))
        // Two Plug-Ins — distinct ids so we can confirm both land in trash.
        .add_card(option_card("PLUG-A", 0, CardColor::Red))
        .add_card(option_card("PLUG-B", 0, CardColor::Red))
        // A bystander Digimon on P0 to host the OnDeletion / OnLinkedCardTrashed
        // witness observers. (Observers on the deleted host itself don't
        // help — after deletion the host is gone, so its own top-card
        // effects can't be re-looked-up by downstream scans.)
        .add_card(digimon_card("P0-WATCHER", CardColor::Red, 2000))
        // P1 attacker strong enough to delete HOST even with a Plug-In buff
        // suppressed (we test w/o buff for the cascade — plain observers).
        .add_card(digimon_card("P1-ATKR", CardColor::Red, 9000))
        .hand(0, &["PLUG-A", "PLUG-B"])
        .memory(5)
        .start();
    r.register_effect("PLUG-A", Arc::new(PlugInBare));
    r.register_effect("PLUG-B", Arc::new(PlugInBare));
    // Host carries the OnDeletion witness — it fires BEFORE the host is
    // removed from battle_area (see `commit_permanent_deletion`), so the
    // observer lookup succeeds.
    r.register_effect(
        "HOST",
        Arc::new(OnDeletionWitness(on_deletion_witness.clone())),
    );
    r.register_effect(
        "P0-WATCHER",
        Arc::new(OnLinkedTrashedWitness(on_linked_trashed_witness.clone())),
    );

    let host = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "P0-WATCHER", Some(0));

    // Attach PLUG-A.
    advance_to_main(&mut r);
    let r1 = r.game.play_option_from_hand(0, 0); // PLUG-A (hand[0])
    assert_eq!(r1, OptionPlayResult::Pending);
    let aid = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, aid);

    // Attach PLUG-B.
    let r2 = r.game.play_option_from_hand(0, 0); // PLUG-B is now hand[0]
    assert_eq!(r2, OptionPlayResult::Pending);
    let aid2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, aid2);

    // Sanity: both linked.
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        2,
        "both Plug-Ins attached to host"
    );
    assert_eq!(*on_deletion_witness.lock().unwrap(), 0);
    assert_eq!(*on_linked_trashed_witness.lock().unwrap(), 0);

    let trash_before = r.trash_size(0);

    // Delete the host directly — mirrors the "P1 attacks and deletes" path
    // without threading through the combat state machine (the combat
    // pipeline ultimately calls `delete_permanent_with_effects`; we call
    // it directly to isolate the cascade under test). The `_unused`
    // attacker fixture remains as documentation of the scenario intent.
    let _unused = r.place_on_field(1, "P1-ATKR", Some(0));
    r.game.delete_permanent_with_effects(host);

    // Host gone.
    assert_eq!(
        r.battle_area_size(0),
        1,
        "only the watcher remains on P0 field"
    );
    // Both linked cards + host top card landed in P0's trash.
    assert_eq!(
        r.trash_size(0),
        trash_before + 3,
        "host top card (1) + 2 linked cards = 3 new cards in owner's trash"
    );
    // OnDeletion fired once for the host.
    assert_eq!(
        *on_deletion_witness.lock().unwrap(),
        1,
        "OnDeletion fired once for the host before removal"
    );
    // OnLinkedCardTrashed semantics: fires once per player battle-area
    // scan, not once per trashed linked card. The watcher is on P0, so
    // it observes exactly one fire — even though 2 cards trashed.
    assert_eq!(
        *on_linked_trashed_witness.lock().unwrap(),
        1,
        "OnLinkedCardTrashed fires once per scan — 2 linked trashes = 1 observer fire"
    );
}
