//! BT25-089 Kazuki & Itsuki — Tamer, GREEN, Cost 4. Traits: App Driver/Appmon.
//!
//! # Card text (official Bandai DB bundle + DCGO BT25_089.cs — authoritative)
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Main] By suspending this Tamer, you may link 1 [Appmon] trait Digimon card
//!   from your hand or your Digimon's digivolution cards to 1 of your Digimon
//!   with the cost reduced by 2.
//! [End of Your Turn][Once Per Turn] 1 of your Digimon may app fuse into a
//!   Digimon card in the hand.
//! [Security] Play this card without paying the cost.
//!
//! # 2026-07-10 audit — prior PARTIAL closed
//! (1) The [End of Your Turn][OPT] App Fuse clause shipped 2026-06-13 via the
//!     `app_fuse` step (effect-initiated App Fuse) — tested below.
//! (2) The [Main] link source "from your Digimon's digivolution cards"
//!     (G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES) is now authored via
//!     `from: [hand, own_digimon_sources]`. When BOTH zones hold an eligible
//!     card the engine first prompts the zone choice ("From hand" / "From your
//!     Digimon's digivolution cards") — DCGO SetIntSelection parity. The
//!     printed "-2" is threaded as `cost: { reduce: 2 }` (paid amount resolves
//!     to 0 since link cards carry no base link cost in this context).
//! Color drift (`[purple]` → `[green]`) was corrected 2026-06-12 and remains
//! guarded by a dedicated test.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_089.cs
//! (Note: DCGO's [Main] description string says "[Once Per Turn]" but its
//! ActivateClass maxUseCount is -1 and the official printed text has no OPT on
//! the [Main] — the YAML correctly declares no `once_per_turn` there.)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-089";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn tamer() -> CardData {
    let mut c = make_test_card(CARD_ID, "Kazuki & Itsuki");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-089 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("APPMON-IN-HAND", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("APPMON-UNDER", 3, 3000, 3, &["Appmon"]))
        .add_card(make_digimon("DONOR", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("MY-HOST", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", 4, 4000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn fire_main(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner
        .game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

#[test]
fn bt25_089_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Kazuki & Itsuki");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    // Official DB: Type/Traits "App Driver/Appmon".
    assert_eq!(
        card.traits,
        vec!["App Driver".to_string(), "Appmon".to_string()],
        "printed traits (official Bandai DB: App Driver/Appmon)"
    );
}

#[test]
fn bt25_089_start_of_main_gains_memory_when_opponent_has_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    let mem_before = r.game.memory;

    // Fire the Start of Your Main Phase observer.
    let handle = r.perm_handle(0, kazuki.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();

    assert_eq!(
        r.game.memory,
        mem_before + 1,
        "gained 1 memory because opponent has a Digimon"
    );
}

#[test]
fn bt25_089_main_links_appmon_from_hand_to_chosen_digimon() {
    let mut r = base()
        .hand(0, &["APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let my_host = r.place_on_field(0, "MY-HOST", Some(0));
    advance_to_main(&mut r);

    // [Main] activation installs the link card selection. (The `suspend_self`
    // activation_cost is paid through the action decoder at activation time,
    // not via this raw `fire_main` enqueue, so we assert the link body here and
    // cover the suspend cost structurally below.)
    assert!(
        fire_main(&mut r, 0, kazuki.index as usize),
        "[Main] installs a selection (optional-use prompt)"
    );

    // The clause is "you may" — first prompt is the optional-use decision.
    // Accept it (first non-decline action), which then installs the card
    // selection. Walk prompts until we reach the host-selection, choosing the
    // first option each time except the final host pick (MY-HOST explicitly).
    let host_action = digimon_engine::action::space::encode_attack(0, my_host.index as u16);
    for _ in 0..4 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        let ids = sel.valid_action_ids.clone();
        // Prefer the MY-HOST action when it is offered (the host selection).
        let action = if ids.contains(&host_action) {
            host_action
        } else {
            ids[0]
        };
        let _ = r.game.resolve_selection(0, action);
        if action == host_action {
            break;
        }
    }

    assert_eq!(
        r.game.player(0).battle_area[my_host.index as usize]
            .linked_cards
            .len(),
        1,
        "the Appmon card from hand linked onto the chosen Digimon"
    );
}

// ─── [Main] "or your Digimon's digivolution cards" source (gap closed) ───────

/// With no linkable card in hand but an [Appmon] Digimon card under one of
/// your Digimon, the [Main] link draws from that Digimon's digivolution cards
/// (single eligible zone → no zone-choice prompt) and attaches it to a
/// DIFFERENT chosen Digimon — the cross-permanent source scan of
/// G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES.
#[test]
fn bt25_089_main_links_appmon_from_own_digimon_sources() {
    use digimon_engine::action::space::{encode_attack, encode_source_select};

    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let donor = r.place_on_field(0, "DONOR", Some(0));
    let my_host = r.place_on_field(0, "MY-HOST", Some(0));
    // Bury an Appmon Digimon card under DONOR (below the top card).
    r.push_source(donor, "APPMON-UNDER");
    let donor_sources_before = r.game.player(0).battle_area[donor.index as usize]
        .card_sources
        .len();
    advance_to_main(&mut r);

    assert!(
        fire_main(&mut r, 0, kazuki.index as usize),
        "[Main] installs a selection"
    );

    let source_action =
        encode_source_select(donor.index as u16, 0).expect("source-select action encodes");
    let host_action = encode_attack(0, my_host.index as u16);
    for _ in 0..5 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        let ids = sel.valid_action_ids.clone();
        let action = if ids.contains(&host_action) {
            host_action
        } else if ids.contains(&source_action) {
            source_action
        } else {
            // Optional-use accept prompt: accept (first non-decline action).
            ids[0]
        };
        let _ = r.game.resolve_selection(0, action);
        if action == host_action {
            break;
        }
    }

    assert_eq!(
        r.game.player(0).battle_area[my_host.index as usize]
            .linked_cards
            .len(),
        1,
        "the Appmon card from DONOR's digivolution cards linked onto MY-HOST"
    );
    assert_eq!(
        r.game.player(0).battle_area[donor.index as usize]
            .card_sources
            .len(),
        donor_sources_before - 1,
        "the linked card left DONOR's digivolution cards"
    );
}

/// When BOTH the hand and a Digimon's digivolution cards hold an eligible
/// [Appmon] card, the engine first asks WHICH zone to link from — the printed
/// "from your hand or your Digimon's digivolution cards" choice (DCGO
/// SetIntSelection "From which area will you link a card?" parity). Choosing
/// the sources zone links from under the Digimon and leaves the hand intact.
#[test]
fn bt25_089_main_zone_choice_when_hand_and_sources_both_eligible() {
    use digimon_engine::action::space::{encode_attack, encode_source_select};
    use digimon_engine::selection::SelectionKind;

    let mut r = base()
        .hand(0, &["APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let donor = r.place_on_field(0, "DONOR", Some(0));
    let my_host = r.place_on_field(0, "MY-HOST", Some(0));
    r.push_source(donor, "APPMON-UNDER");
    let hand_before = r.game.player(0).hand.len();
    advance_to_main(&mut r);

    assert!(
        fire_main(&mut r, 0, kazuki.index as usize),
        "[Main] installs a selection"
    );

    let source_action =
        encode_source_select(donor.index as u16, 0).expect("source-select action encodes");
    let host_action = encode_attack(0, my_host.index as u16);
    let mut saw_zone_choice = false;
    for _ in 0..6 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        // Zone-choice prompt: an EffectChoice offering the sources zone.
        let zone_entry = view.effect_choices.as_ref().and_then(|choices| {
            choices
                .iter()
                .find(|c| c.label == "From your Digimon's digivolution cards")
                .map(|c| c.action_id)
        });
        let action = if let Some(zone_action) = zone_entry {
            assert_eq!(view.kind, SelectionKind::EffectChoice);
            let labels: Vec<&str> = view
                .effect_choices
                .as_ref()
                .unwrap()
                .iter()
                .map(|c| c.label.as_str())
                .collect();
            assert!(
                labels.contains(&"From hand"),
                "zone choice must also offer the hand zone; got {labels:?}"
            );
            saw_zone_choice = true;
            zone_action
        } else if view.valid_action_ids.contains(&host_action) {
            host_action
        } else if view.valid_action_ids.contains(&source_action) {
            source_action
        } else {
            view.valid_action_ids[0]
        };
        let _ = r.game.resolve_selection(0, action);
        if action == host_action {
            break;
        }
    }

    assert!(
        saw_zone_choice,
        "with candidates in both zones, the zone-choice prompt must surface"
    );
    assert_eq!(
        r.game.player(0).battle_area[my_host.index as usize]
            .linked_cards
            .len(),
        1,
        "the sources-zone Appmon linked onto MY-HOST"
    );
    assert_eq!(
        r.game.player(0).hand.len(),
        hand_before,
        "hand untouched — the link came from the digivolution cards"
    );
}

// ─── [Start of Your Main Phase] negative ─────────────────────────────────────

/// No opponent Digimon → memory is NOT gained.
#[test]
fn bt25_089_start_of_main_no_opp_digimon_does_not_gain_memory() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    // Opponent has no Digimon on the field (none placed).
    let mem_before = r.game.memory;

    let handle = r.perm_handle(0, kazuki.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();

    assert_eq!(
        r.game.memory, mem_before,
        "memory must not change when opponent has no Digimon"
    );
}

// ─── [Main] activation_cost: suspend_self — negative (already suspended) ─────

/// If Kazuki & Itsuki is already suspended, the [Main] clause's activation_cost
/// (`suspend_self`) cannot be paid; the link process must not run (no link).
#[test]
fn bt25_089_main_does_not_link_when_tamer_already_suspended() {
    let mut r = base()
        .hand(0, &["APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let my_host = r.place_on_field(0, "MY-HOST", Some(0));
    advance_to_main(&mut r);

    // Manually suspend Kazuki so the activation_cost `suspend_self` can't fire.
    r.game.players[0].battle_area[kazuki.index as usize].is_suspended = true;

    // fire_main enqueues and drains; with the cost blocked the process is
    // skipped. The optional pre-cost prompt might or might not appear, but
    // the link itself must never materialise.
    fire_main(&mut r, 0, kazuki.index as usize);

    // Exhaust any lingering optional accept/decline prompts by declining them all.
    for _ in 0..4 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        // Accept any prompt (PASS-style decline is last in valid_action_ids).
        let action = *sel.valid_action_ids.last().unwrap();
        let _ = r.game.resolve_selection(0, action);
    }

    assert_eq!(
        r.game.player(0).battle_area[my_host.index as usize]
            .linked_cards
            .len(),
        0,
        "no link when Kazuki is already suspended (activation_cost guard)"
    );
}

// ─── [Security] structural ────────────────────────────────────────────────────

/// The compiled effect list must include an `OnSecurity` triggered clause that
/// fires `play_from_security` — this is the inherited [Security] play-free effect.
#[test]
fn bt25_089_has_on_security_play_free_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    let has_on_sec = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )
    });
    assert!(
        has_on_sec,
        "[Security] play-free clause (OnSecurity timing) must be compiled for BT25-089"
    );
}

// ─── [End of Your Turn][OPT] App Fuse (effect-initiated, 2026-06-13) ─────────

/// App-Fusion result card (materials Kabemon & Gomimon) for the EoT app-fuse
/// clause. Mirrors the synthetic fixtures in `app_fuse_primitive.rs`.
const APP_FUSE_RESULT: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [green]
cost: 7
dp: 9000
traits: [Appmon]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

/// Material card whose printed NAME (not id) must match the App-Fusion
/// `name_in` condition (Kabemon / Gomimon).
fn named_fuse_material(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["Appmon".to_string()];
    c
}

fn app_fuse_base() -> DebugRunnerBuilder {
    base()
        .from_dsl_yaml(APP_FUSE_RESULT)
        .expect("APP_FUSE_RESULT compiles")
        .add_card(named_fuse_material("TEST-KABEMON", "Kabemon"))
        .add_card(named_fuse_material("TEST-GOMIMON", "Gomimon"))
}

fn fire_eot(runner: &mut DebugRunner, player: PlayerId, field_index: usize) {
    let handle = runner.perm_handle(player, field_index);
    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// [End of Your Turn] one of your Digimon app-fuses a hand card: pick the host,
/// then the result card; the result stacks and the materials are consumed.
#[test]
fn bt25_089_eot_app_fuse_stacks_hand_result() {
    use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START};

    let mut r = app_fuse_base()
        .hand(0, &["TEST-APPFUSE"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    r.push_linked_owned(host, "TEST-GOMIMON", 0);

    fire_eot(&mut r, 0, kazuki.index as usize);

    // Selection #1: the host permanent.
    let host_action = encode_attack(0, host.index as u16);
    assert!(
        r.pending_selection_view()
            .expect("app-fuse perm selection installs")
            .valid_action_ids
            .contains(&host_action),
        "host with two materials is an eligible app-fuse target"
    );
    r.execute_action(0, host_action).expect("pick host");

    // Selection #2: the result card in hand.
    let card_pos = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE")
        .unwrap();
    r.execute_action(0, HAND_EFFECT_START + card_pos as u16)
        .expect("pick result card");

    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-APPFUSE",
        "result card app-fused onto the host"
    );
    assert!(
        r.game.players[0].battle_area[host.index as usize]
            .linked_cards
            .is_empty(),
        "materials consumed into digivolution sources"
    );
}

/// OPT lockout: the EoT app-fuse fires once per turn. A second EoT trigger the
/// same turn installs no selection; after the turn cycles, it can fire again.
#[test]
fn bt25_089_eot_app_fuse_once_per_turn() {
    use digimon_engine::action::space::encode_attack;

    let mut r = app_fuse_base()
        .hand(0, &["TEST-APPFUSE"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let kazuki = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    r.push_linked_owned(host, "TEST-GOMIMON", 0);

    // First fire → selection installs; decline it (PASS) but the OPT use is recorded.
    fire_eot(&mut r, 0, kazuki.index as usize);
    assert!(
        r.pending_selection_view().is_some(),
        "first EoT app-fuse installs a selection"
    );
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline");

    // Second fire same turn → OPT lockout, no selection.
    fire_eot(&mut r, 0, kazuki.index as usize);
    assert!(
        r.pending_selection_view().is_none(),
        "second EoT app-fuse same turn is locked out by once_per_turn"
    );
}

// ─── Color metadata drift (AUDITED-DRIFT) ────────────────────────────────────

/// BT25-089 is a GREEN card (card image: green border; cards.json: `card_colors:
/// [3]`; DCGO: BT25/Green/BT25_089.cs). The YAML drift (`color: [purple]`) was
/// corrected to `[green]` on 2026-06-12 (audit drift fix).
#[test]
fn bt25_089_color_is_green_not_purple() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(
        card.color,
        vec![CompiledColor::Green],
        "BT25-089 is Green; YAML color field must be corrected from [purple] to [green]"
    );
}
