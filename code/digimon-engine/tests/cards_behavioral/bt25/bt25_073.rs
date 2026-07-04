//! BT25-073 Dragomon — Digimon, Lv.5, Black, DP 7000, Cost 7.
//! Trait line: Aquabeast/Titan/TS. Attribute: Virus.
//!
//! # Card text (data/card_bundles/BT25-073.md — official Bandai DB;
//! confirmed vs card image + DCGO BT25_073.cs)
//! <Jamming> [On Play] [When Digivolving] By trashing 1 of your Digimon's
//!   link cards, you may play or use 1 [TS] trait card with a play or use
//!   cost of 5 or less from your hand without paying the cost.
//! Inherited [All Turns]: When this Digimon would leave the battle area, by
//!   trashing 1 of its link cards, it doesn't leave.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_073.cs
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3 / §5 / §11)
//! - grant_keyword Jamming (self, non-inherited)
//! - alt_paths digivolve from { level_eq: 4, trait_has: TS } cost 3
//! - trash_link_card_of_own_digimon (G-DSL-LINK-TRASH-AS-COST) two-selection
//!   activation cost (which own Digimon w/ link card -> which of its link
//!   cards), gated tail (select_hand TS cost<=5 -> play_or_use_from_hand free)
//! - unpayable-abort when no own Digimon carries a link card
//! - OPT lockout: both cost picks declinable + the hand pick itself declinable
//! - inherited [All Turns] leave-replacement: trash_own_link_card, optional,
//!   outcome prevent (G-DSL-LINK-TRASH-AS-REPLACEMENT-COST)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-073";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn ts_digimon(id: &str, cost: u16) -> CardData {
    make_digimon(id, 3, 3000, cost, &["TS"])
}

fn ts_option(id: &str, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = cost;
    card.traits = vec!["TS".to_string()];
    card
}

fn link_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-073 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("HOST-A", 3, 3000, 3, &["Beast"]))
        .add_card(link_option("LINK-A1"))
        .add_card(link_option("LINK-A2"))
        .add_card(ts_digimon("TS-DIGI-5", 5))
        .add_card(ts_option("TS-OPT-5", 5))
        .add_card(ts_digimon("TS-DIGI-6", 6))
        .add_card(make_digimon("NON-TS", 3, 3000, 4, &["Beast"]))
        .add_card(make_digimon("TOP-DIGI", 6, 8000, 8, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn hand_index_of(r: &DebugRunner, p: PlayerId, card_id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand of p{p}"))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_073_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Dragomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    assert!(card.traits.iter().any(|t| t == "TS"));
    assert!(card.traits.iter().any(|t| t == "Aquabeast"));
    assert!(card.traits.iter().any(|t| t == "Titan"));
}

#[test]
fn bt25_073_has_ts_lv4_cost3_alt_path() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("must have a digivolve alt-path");
    assert_eq!(path.cost, Some(CompiledCost::Literal(3)));
}

#[test]
fn bt25_073_grants_jamming() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let drago = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(drago, Keyword::Jamming),
        "BT25-073 has <Jamming>"
    );
}

#[test]
fn bt25_073_main_clause_declares_link_trash_cost_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let main = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("the main [OP][WD] clause must be present");
    let has_cost_step = main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::TrashLinkCardOfOwnDigimon { .. }));
    assert!(
        has_cost_step,
        "main clause's activation cost is trash_link_card_of_own_digimon"
    );
}

#[test]
fn bt25_073_inherited_leave_replacement_present() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has_replacement = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { scope, .. })
                if *scope == CompiledScope::Inherited
        )
    });
    assert!(
        has_replacement,
        "BT25-073 declares the inherited leave-replacement"
    );
}

// ─── Section 2 — Main clause: payable path, play a Digimon ───────────────────

#[test]
fn bt25_073_on_play_trash_link_card_then_play_ts_digimon_free() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);
    {
        // Push the TS Digimon into hand alongside Dragomon.
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TS-DIGI-5")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, next));
    };
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    let field_before = r.battle_area_size(0);

    let drago_idx = r.play(0, 0).expect("Dragomon played");
    // Playing Dragomon out of hand index 0 shifts TS-DIGI-5 down by one —
    // recompute its hand index AFTER the play rather than trusting the
    // pre-play snapshot.
    let ts_hand_idx = hand_index_of(&r, 0, "TS-DIGI-5");
    // Baseline memory AFTER Dragomon's own play cost has already applied, so
    // the assertion below isolates whether the free TS play consumes memory.
    let mem_before = r.memory();

    // First selection: which own Digimon carries a link card (only HOST-A).
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("first selection (which Digimon) installs");
    let pick_host = encode_attack(0, host_a.index as u16);
    assert!(
        p.valid_action_ids.contains(&pick_host),
        "HOST-A is a valid first pick"
    );
    r.game
        .resolve_selection(0, pick_host)
        .expect("resolve first selection");

    // Second selection: which of HOST-A's link cards (only LINK-A1).
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("second selection (which link card) installs");
    let trash_id = HAND_EFFECT_START;
    assert!(p.valid_action_ids.contains(&trash_id));
    r.game
        .resolve_selection(0, trash_id)
        .expect("resolve second selection (trash link card)");

    assert_eq!(
        r.game.player(0).battle_area[host_a.index as usize]
            .linked_cards
            .len(),
        0,
        "HOST-A's link card was trashed as the cost"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly the link card went to trash"
    );

    // Third selection: hand pick — the TS Digimon (cost 5 <= 5).
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand pick installs after the cost is paid");
    let want = PLAY_HAND_START + ts_hand_idx as u16;
    assert!(p.valid_action_ids.contains(&want), "TS-DIGI-5 is a legal pick");
    r.game
        .resolve_selection(0, want)
        .expect("resolve hand pick");

    assert!(
        r.game.player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&r.game.card_data) == "TS-DIGI-5"),
        "the TS Digimon was played"
    );
    assert_eq!(
        r.battle_area_size(0),
        field_before + 2, // Dragomon itself + the TS Digimon
        "Dragomon and the TS Digimon both entered the field"
    );
    assert_eq!(
        r.memory(),
        mem_before,
        "the TS Digimon was played WITHOUT paying its cost (memory only reflects Dragomon's play)"
    );
}

// ─── Section 3 — Main clause: payable path, use a TS Option ──────────────────

#[test]
fn bt25_073_when_digivolving_trash_link_card_then_use_ts_option_free() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let _link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);

    let opt_hand_idx = {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TS-OPT-5")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, next));
        r.game.players[0].hand.len() - 1
    };
    advance_to_main(&mut r);

    // Simulate digivolution by firing the WhenDigivolving trigger directly on
    // Dragomon once it's placed on the field.
    let drago = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(drago),
    );
    r.game.drain_effect_queue();

    let mem_before = r.memory();
    let hand_before = r.game.players[0].hand.len();

    // First selection: which own Digimon (HOST-A; Dragomon itself has no link
    // cards so it is not offered as a candidate).
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("first selection installs");
    let pick_host = encode_attack(0, host_a.index as u16);
    assert!(p.valid_action_ids.contains(&pick_host));
    r.game.resolve_selection(0, pick_host).unwrap();

    // Second selection: the link card.
    let p = r.game.pending_selection.as_ref().expect("second selection");
    let trash_id = HAND_EFFECT_START;
    r.game.resolve_selection(0, trash_id).unwrap();

    // Third selection: hand pick — the TS Option.
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand pick installs");
    let want = PLAY_HAND_START + opt_hand_idx as u16;
    assert!(p.valid_action_ids.contains(&want), "TS-OPT-5 is a legal pick");
    r.game.resolve_selection(0, want).unwrap();

    assert!(
        r.game.players[0]
            .hand
            .iter()
            .all(|c| c.card_id(&r.game.card_data) != "TS-OPT-5"),
        "the TS Option was USED (left the hand)"
    );
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before - 1,
        "exactly the Option left the hand"
    );
    assert_eq!(r.memory(), mem_before, "the Option was used for free");
}

// ─── Section 4 — Negative: cost paid but no qualifying hand card ─────────────

#[test]
fn bt25_073_cost_too_high_hand_card_is_not_a_legal_pick() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let _link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);

    let hi = {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TS-DIGI-6")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, next));
        r.game.players[0].hand.len() - 1
    };
    advance_to_main(&mut r);

    let field_before = r.battle_area_size(0);
    let _drago_idx = r.play(0, 0).expect("Dragomon played");

    let pick_host = encode_attack(0, host_a.index as u16);
    r.game.resolve_selection(0, pick_host).unwrap();
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();

    // With zero hand cards satisfying the filter, `select_hand` no-ops
    // (established convention — the RL policy never sees a mandatory prompt
    // with no legal answer): no pending selection installs at all, and the
    // clause resolves having only paid the link-card cost.
    let _ = hi; // retained for documentation of what does NOT qualify
    assert!(
        r.game.pending_selection.is_none(),
        "cost-6 TS Digimon exceeds the cost<=5 filter — zero qualifying \
         candidates means the hand-pick step no-ops rather than installing \
         a decline-only prompt"
    );
    assert_eq!(
        r.battle_area_size(0),
        field_before + 1, // only Dragomon entered
        "no additional card was played"
    );
}

#[test]
fn bt25_073_non_ts_hand_card_is_not_a_legal_pick() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let _link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);

    let hi = {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "NON-TS")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, next));
        r.game.players[0].hand.len() - 1
    };
    advance_to_main(&mut r);

    let field_before = r.battle_area_size(0);
    let _drago_idx = r.play(0, 0).expect("Dragomon played");
    let pick_host = encode_attack(0, host_a.index as u16);
    r.game.resolve_selection(0, pick_host).unwrap();
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();

    // NON-TS is the only hand card and fails the `trait_has: TS` filter, so
    // zero candidates qualify: the hand-pick step no-ops (same convention as
    // the cost-too-high case above) rather than installing a decline-only
    // prompt.
    let _ = hi; // retained for documentation of what does NOT qualify
    assert!(
        r.game.pending_selection.is_none(),
        "non-TS card is not a legal pick regardless of cost — zero \
         qualifying candidates means the hand-pick step no-ops"
    );
    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "no additional card was played"
    );
}

// ─── Section 5 — Unpayable: no own Digimon has a link card ───────────────────

#[test]
fn bt25_073_unpayable_when_no_own_digimon_has_link_card() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    // HOST-A on the field but carries NO link card.
    let _host_a = r.place_on_field(0, "HOST-A", Some(0));
    advance_to_main(&mut r);

    let field_before = r.battle_area_size(0);
    let _drago_idx = r.play(0, 0).expect("Dragomon played");

    // No candidate carries a link card -> cost unpayable -> clause aborts ->
    // no selection installs beyond the play itself.
    assert!(
        r.game.pending_selection.is_none(),
        "no own Digimon has a link card -> the activation cost is unpayable, no selection installs"
    );
    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "only Dragomon entered; nothing else was played"
    );
}

// ─── Section 6 — OPT lockouts (decline at each stage) ────────────────────────

#[test]
fn bt25_073_decline_first_pick_skips_trash_and_tail() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let _link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    let _drago_idx = r.play(0, 0).expect("Dragomon played");

    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("first selection installs");
    assert!(p.is_optional, "the first pick (which Digimon) is declinable");
    r.game.resolve_selection(0, PASS).expect("decline first pick");

    assert!(
        r.game.pending_selection.is_none(),
        "declining the first pick resolves the clause with no further selection"
    );
    assert_eq!(
        r.game.player(0).battle_area[host_a.index as usize]
            .linked_cards
            .len(),
        1,
        "declining trashes no link card"
    );
    assert_eq!(r.trash_size(0), trash_before, "no trash on decline");
}

#[test]
#[ignore = "ENGINE GAP (not fixable from cards/bt25/BT25-073.yaml or this \
    test file alone): DCGO's SelectCardEffect for the which-link-card pick \
    is canNoSelect:true (independently declinable, same as the which-Digimon \
    pick), and `trash_link_card_of_own_digimon`'s `optional` flag is intended \
    to thread through to both selections. But the second selection is \
    installed via `EffectContext::select_effect_choice` \
    (code/digimon-engine/src/effect_context/selections.rs), which hardcodes \
    `is_optional: false` (\"effect choice must pick a branch\") and never \
    appends PASS to valid_action_ids; `install_link_card_trash_second_selection` \
    (code/digimon-engine/src/dsl_cards/step/selections.rs) wires an \
    `on_decline` callback but never overrides `pending.is_optional`, so \
    `resolve_generic_selection`'s `is_pass && !sel.is_optional` gate \
    (code/digimon-engine/src/effect_queue.rs) rejects PASS before the \
    on_decline handler is ever reached. Fix requires widening \
    `select_effect_choice` (or the second-selection installer) to expose an \
    optional/declinable effect-choice variant — out of scope for the \
    2-file BT25-073 deliverable. Logged to qa/dsl-vocab-gaps.md."]
fn bt25_073_decline_second_pick_skips_trash_and_tail() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let _link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    let _drago_idx = r.play(0, 0).expect("Dragomon played");

    let pick_host = encode_attack(0, host_a.index as u16);
    r.game.resolve_selection(0, pick_host).unwrap();

    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("second selection installs");
    assert!(
        p.is_optional,
        "the second pick (which link card) is declinable"
    );
    r.game.resolve_selection(0, PASS).expect("decline second pick");

    assert!(
        r.game.pending_selection.is_none(),
        "declining the second pick resolves the clause with no further selection"
    );
    assert_eq!(
        r.game.player(0).battle_area[host_a.index as usize]
            .linked_cards
            .len(),
        1,
        "declining trashes no link card"
    );
    assert_eq!(r.trash_size(0), trash_before, "no trash on decline");
}

#[test]
fn bt25_073_decline_hand_pick_after_paying_cost_still_trashes_link_card() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host_a = r.place_on_field(0, "HOST-A", Some(0));
    let _link_a1 = r.push_linked_owned(host_a, "LINK-A1", 0);
    let _ts_hand = {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TS-DIGI-5")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, next));
    };
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    let field_before = r.battle_area_size(0);
    let _drago_idx = r.play(0, 0).expect("Dragomon played");

    let pick_host = encode_attack(0, host_a.index as u16);
    r.game.resolve_selection(0, pick_host).unwrap();
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();

    // Cost already paid (link card trashed) — the hand pick is a separate
    // decline point (canNoSelect on SelectHandEffect): the trash from the
    // ACTIVATION cost is NOT refunded on decline here.
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the link card is already trashed once the cost step resolved"
    );

    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand pick installs");
    assert!(p.is_optional, "the hand pick itself is declinable");
    r.game.resolve_selection(0, PASS).expect("decline hand pick");

    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "only Dragomon entered; declining the hand pick plays nothing else"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the link card stays trashed even though the play/use was declined"
    );
}

// ─── Section 7 — Inherited leave-replacement ─────────────────────────────────

// Per rule 15-3 ("Inherited effects (15-3): an effect gained from a
// digivolution card; can't be activated by just one card"), Dragomon's
// inherited leave-replacement only grants the ability while Dragomon is a
// BELOW-TOP digivolution source (something has digivolved from/onto it), not
// while it is standing alone as the sole/top card of its own stack. Build a
// 2-card stack (Dragomon at the bottom, a filler Digimon on top) so the
// inherited effect is live, matching how the engine's `inherited_source`
// gate (`source_index + 1 < stack_size`) and DCGO's own "gained from a
// digivolution card" model both work. `linked_cards` live on the shared
// `Permanent`, so attaching them to the stack handle still counts as
// Dragomon's own link cards for this purpose.

#[test]
fn bt25_073_leave_replacement_trashes_own_link_card_and_stays() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let stack = r.place_stack(0, &[CARD_ID, "TOP-DIGI"]);
    r.push_linked_owned(stack, "LINK-A1", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(stack);

    assert!(
        r.game.pending_selection.is_some(),
        "leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the replacement");

    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("link-card-trash selection installed")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    assert_eq!(r.battle_area_size(0), 1, "the stack did NOT leave");
    assert_eq!(
        r.game.player(0).battle_area[stack.index as usize]
            .linked_cards
            .len(),
        0,
        "the chosen link card was trashed as the cost"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly the link card went to trash"
    );
}

#[test]
fn bt25_073_leave_replacement_declined_lets_it_leave() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let stack = r.place_stack(0, &[CARD_ID, "TOP-DIGI"]);
    r.push_linked_owned(stack, "LINK-A1", 0);
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(stack);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, PASS)
        .expect("decline the replacement");

    assert_eq!(r.battle_area_size(0), 0, "declined -> the stack leaves");
}

#[test]
fn bt25_073_leave_replacement_not_offered_without_link_cards() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let stack = r.place_stack(0, &[CARD_ID, "TOP-DIGI"]);
    // No link cards attached.
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(stack);
    assert!(
        r.game.pending_selection.is_none(),
        "0 link cards -> the replacement is gated off (not offered)"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "with no link card to trash, the stack leaves normally"
    );
}

#[test]
fn bt25_073_leave_replacement_not_offered_when_standing_alone() {
    // Rule 15-3: an inherited effect requires the card to be gained FROM a
    // digivolution card — it does not apply to the printed card's own
    // inherited-text box while that card is standing alone as the sole/top
    // permanent. Dragomon alone (no lower/upper stack partner) must NOT
    // offer the leave-replacement even with a link card attached.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let drago = r.place_on_field(0, CARD_ID, Some(0));
    r.push_linked_owned(drago, "LINK-A1", 0);
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(drago);
    assert!(
        r.game.pending_selection.is_none(),
        "Dragomon alone (not a below-top digivolution source) does not \
         grant its own inherited leave-replacement to itself (rule 15-3)"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "with the inherited effect inactive, Dragomon leaves normally"
    );
}
