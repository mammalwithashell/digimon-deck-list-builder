//! BT21-059 Timemon — Digimon, Lv.4, Black, DP 5000, Play Cost 5.
//! Trait line: Sup./Appmon | Tool | Time Slip
//!   → traits: ["Sup.", Appmon, Tool, "Time Slip"], attribute: Tool
//!
//! # Corrected printed card text (from card image — authoritative)
//!
//! ```text
//! ＜Blocker＞
//! [Your Turn] [Once Per Turn] When this Digimon gets linked, ＜De-Digivolve 1＞
//!   1 of your opponent's Digimon. (Trash the top card. You can't trash past
//!   level 3 cards.)
//!
//! Link box:
//!   [Link] [Appmon] trait: Cost 2
//!   +3000 DP (while linked to a host, host gains +3000 DP)
//!   [When Linking] ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//!
//! Digivolve boxes:
//!   Standard: Black Lv.3, Cost 2
//!   Stnd. alt-path: from a Digimon with [Stnd.] trait, Cost 2
//!     (DCGO EqualsTraits("Stnd.") — no level gate)
//!
//! App Fusion box:
//!   [App Fusion] [Watchmon] & [Savemon] & [Calendamon]: Cost 0
//! ```
//!
//! # DCGO C# reference (READ-ONLY)
//! C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_059.cs
//!
//! # Key DCGO analysis
//! - `BlockerSelfStaticEffect` → kind: grant_keyword, keyword: Blocker
//! - `AddSelfDigivolutionRequirementStaticEffect(EqualsTraits("Stnd."), cost 2)` — NO level gate
//! - `AddSelfLinkConditionStaticEffect(HasAppmonTraits, linkCost 2)` → link_condition cost 2
//! - `AddAppfuseMethodByName(["Watchmon","Savemon","Calendamon"])` → app_fusion
//! - `WhenLinked` host clause (ActivateClass priority 1, NOT SetIsLinkedEffect):
//!     `CanUseCondition` checks `IsOwnerTurn` → `when: when_card_linked_to_this,
//!     once_per_turn: true, active_when: { your_turn: true }`, mandatory (canNoSelect: false)
//! - `WhenLinked` linked clause (SetIsLinkedEffect(true), priority -1):
//!     `scope: linked, when: when_linked`, mandatory (canNoSelect: false)
//! - Linked DP aura (+3000) → scope: linked, kind: aura, dp_modifier: 3000
//!
//! # Patterns covered
//! - A1: Blocker self keyword
//! - B2: link_condition (self, Appmon trait, cost 2)
//! - B3: linked DP aura (+3000)
//! - B4: when_card_linked_to_this host-side OPT Your-Turn De-Digivolve 1
//! - B5: scope: linked when_linked De-Digivolve 1
//! - C1: alt_paths digivolve from Stnd. trait, cost 2
//! - C2: alt_paths app_fusion (Watchmon & Savemon & Calendamon), cost 0
//!
//! # Clause map
//! | Clause | Timing / Scope         | Effect                                   | Optional? |
//! |--------|------------------------|------------------------------------------|-----------|
//! | 1      | declarative            | <Blocker>                                | —         |
//! | 2      | declarative            | link_condition [Appmon] cost 2           | —         |
//! | 3      | scope: linked, aura    | +3000 DP to host while linked            | —         |
//! | 4      | when_card_linked_to_this | [YT][OPT] De-Digivolve 1 opp Digimon  | NO (mandatory select when eligible) |
//! | 5      | scope: linked, when_linked | [When Linking] De-Digivolve 1 opp    | NO (mandatory select when eligible) |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-059";

// ─── Card helpers ─────────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn appmon_host(id: &str) -> CardData {
    make_digimon(id, 5, 7000, 7, &["Appmon"])
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-059 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(appmon_host("HOST-APP"))
        .add_card(make_digimon("OPP-LV4", 4, 5000, 5, &["Beast"]))
        .add_card(make_digimon("OPP-LV3", 3, 3000, 3, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

/// Stack a Lv3 under OPP-LV4 on P1 so De-Digivolve 1 can trash the Lv4 top.
fn push_on_stack(r: &mut DebugRunner, owner: u8, perm_idx: usize, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    let cs = CardSource::new(data_idx, owner, next);
    let turn = r.game.turn_count;
    r.game.players[owner as usize].battle_area[perm_idx].digivolve(cs, turn);
}

/// Calculate the link-card action index for a permanent on P0's battle area.
fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt21_059_yaml_printed_metadata() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Timemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.cost, Some(5));
}

#[test]
fn bt21_059_traits_include_sup_appmon_tool_time_slip() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present in pack");
    let traits = &card.traits;
    assert!(traits.iter().any(|t| t == "Sup."), "must have Sup. trait");
    assert!(
        traits.iter().any(|t| t == "Appmon"),
        "must have Appmon trait"
    );
    assert!(traits.iter().any(|t| t == "Tool"), "must have Tool trait");
    assert!(
        traits.iter().any(|t| t == "Time Slip"),
        "must have Time Slip trait"
    );
}

/// Blocker is a declarative grant_keyword clause.
#[test]
fn bt21_059_has_blocker_grant_keyword() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let has_blocker = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) if keyword == "Blocker"
        )
    });
    assert!(
        has_blocker,
        "BT21-059 must have a Blocker declarative grant_keyword"
    );
}

/// Blocker is installed on the permanent after placement on the field.
#[test]
fn bt21_059_blocker_active_on_field() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(timemon, Keyword::Blocker),
        "Timemon must have <Blocker> while on the field"
    );
}

/// Self link-condition: link onto an [Appmon] host for cost 2.
#[test]
fn bt21_059_has_link_condition_appmon_cost_2() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 2
        )
    });
    assert!(
        has,
        "BT21-059 must declare a self link-condition with cost 2"
    );
}

/// Alt-path: digivolve from a [Stnd.] trait Digimon for cost 2.
#[test]
fn bt21_059_has_stnd_alt_digivolve_cost_2() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
    });
    assert!(has, "BT21-059 must have a Stnd. cost-2 alt-digivolve");
}

/// App Fusion with Watchmon, Savemon, Calendamon at cost 0.
#[test]
fn bt21_059_has_app_fusion_alt_path() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(has, "BT21-059 must have an App Fusion alt-path at cost 0");
}

/// Linked DP aura clause exists (scope: linked, kind: aura, dp_modifier: +3000).
#[test]
fn bt21_059_has_linked_dp_aura_3000() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let has_aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(3000),
                ..
            })
        )
    });
    assert!(has_aura, "BT21-059 must have a linked-scope +3000 DP aura");
}

/// When linked to a host, the host's effective DP increases by 3000.
#[test]
fn bt21_059_linked_aura_increases_host_dp_by_3000() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();

    let dp_before = r.effective_dp(host).unwrap_or(0);

    // Link Timemon to HOST-APP via the link action.
    r.game.decode_action(link_bit(timemon) as u16, 0);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("link host-select installs");
    let host_pick = sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_pick);

    // Drain any post-link selections (WhenLinked De-Digivolve prompt — no opp Digimon).
    let _ = r.auto_resolve();

    let dp_after = r.effective_dp(host).unwrap_or(0);
    assert_eq!(
        dp_after,
        dp_before + 3000,
        "host DP must increase by 3000 while Timemon is linked to it"
    );
}

/// Host-side: when_card_linked_to_this clause exists and is OPT + Your Turn.
#[test]
fn bt21_059_has_when_card_linked_to_this_once_per_turn_your_turn() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::WhenCardLinkedToThis)
                && t.scope == CompiledScope::FaceUp =>
        {
            Some(t)
        }
        _ => None,
    });
    let clause = clause.expect("must have a host-side when_card_linked_to_this clause");
    assert!(
        clause.once_per_turn,
        "host when_card_linked_to_this must be [Once Per Turn]"
    );
    assert!(
        clause.active_when.is_some(),
        "must gate on your turn (active_when.your_turn)"
    );
}

/// Linked-side: scope: linked when_linked clause exists.
#[test]
fn bt21_059_has_linked_when_linked_clause() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenLinked)
                    && t.scope == CompiledScope::Linked
        )
    });
    assert!(has, "BT21-059 must have a scope: linked when_linked clause");
}

// ─── Section 2 — Host-side when_card_linked_to_this behavioral ───────────────
//
// The `when_card_linked_to_this` clause fires on `EffectTiming::OnLink` with
// `TriggerSource::Linked { player, host, card }` where `host` is the receiving
// permanent (Timemon) and `card` is the card being linked TO it.
// The DSL condition checks `event_permanent == source_permanent` (host == timemon).
// TriggerSource::Permanent(timemon) sets event_permanent=None (fires for other
// timings); we must use TriggerSource::Linked to set event_permanent=Some(timemon).

/// Helper: fire TriggerSource::Linked on `host` with the host's top-card handle as
/// the being-linked card identifier. Simulates a card being linked to `host` (Timemon).
fn fire_link_to_host(r: &mut DebugRunner, host: PermanentHandle) {
    // Use the top card handle of the host as a stand-in "linked card" identifier.
    let linked_card = r.game.players[host.player as usize].battle_area[host.index as usize]
        .top_card()
        .handle();
    r.game.enqueue_triggered(
        EffectTiming::OnLink,
        TriggerSource::Linked {
            player: host.player,
            host,
            card: linked_card,
        },
    );
    r.game.drain_effect_queue();
}

/// POSITIVE (host-side): P0 has Timemon on field; a card is linked TO Timemon on
/// P0's turn → De-Digivolve target-select surfaces; resolving it pops the Lv4
/// top source from the opponent's stack (Lv3 base remains).
#[test]
fn bt21_059_host_when_linked_de_digivolve_fires_on_your_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    // Timemon as the host on P0's field.
    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    // Opponent has a Lv4 with Lv3 underneath.
    let opp_base = r.place_on_field(1, "OPP-LV3", Some(0));
    push_on_stack(&mut r, 1, opp_base.index as usize, "OPP-LV4");
    let trash_before = r.trash_size(1);

    // Fire the link event: a card is being linked TO Timemon.
    fire_link_to_host(&mut r, timemon);

    // De-Digivolve target-select must surface.
    assert!(
        r.game.pending_selection.is_some(),
        "when_card_linked_to_this must surface a De-Digivolve target selection"
    );

    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    let _ = r.auto_resolve();

    // Lv4 top source should be trashed; the Lv3 base remains.
    let perm = &r.game.player(1).battle_area[opp_base.index as usize];
    assert_eq!(perm.stack_size(), 1, "Lv4 top trashed; Lv3 base remains");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "OPP-LV3",
        "Lv3 is now the top card"
    );
    assert_eq!(
        r.trash_size(1),
        trash_before + 1,
        "the Lv4 source went to opponent's trash"
    );
}

/// STOP AT LEVEL 3: a Lv3-only opponent Digimon cannot be de-digivolved past Lv3.
/// With a single-stack Lv3 opponent, De-Digivolve 1 is a no-op (can't trash past Lv3).
#[test]
fn bt21_059_host_when_linked_de_digivolve_stops_at_level_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    let opp_lv3 = r.place_on_field(1, "OPP-LV3", Some(0)); // single Lv3 stack

    fire_link_to_host(&mut r, timemon);

    // Selection surfaces (no auto-skip: mandatory select with eligible target).
    assert!(
        r.game.pending_selection.is_some(),
        "must surface selection even for Lv3 target"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    let _ = r.auto_resolve();

    // Lv3 stays intact; stack_size unchanged (can't trash past Lv3).
    let perm = &r.game.player(1).battle_area[opp_lv3.index as usize];
    assert_eq!(
        perm.stack_size(),
        1,
        "Lv3 single-stack is a no-op for stop_at_level 3"
    );
    assert_eq!(
        r.trash_size(1),
        0,
        "no card trashed (stop_at_level 3 boundary)"
    );
}

/// OPT lockout: second when_card_linked_to_this trigger in the same turn must not fire.
#[test]
fn bt21_059_host_when_linked_opt_lockout() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    let _opp_a = r.place_on_field(1, "OPP-LV4", Some(0));
    let _opp_b = r.place_on_field(1, "OPP-LV4", Some(0));

    // First fire — must install selection.
    fire_link_to_host(&mut r, timemon);
    let first_sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("first fire installs");
    let action = first_sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    let _ = r.auto_resolve();

    // Second fire same turn — OPT lockout should block it.
    fire_link_to_host(&mut r, timemon);
    assert!(
        r.game.pending_selection.is_none(),
        "OPT lockout: second when_card_linked_to_this must not surface a selection same turn"
    );
}

/// OPT clears next turn: after end_turn the lockout resets.
#[test]
fn bt21_059_host_when_linked_opt_clears_next_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-LV4", Some(0));

    // Fire once; resolve to set the OPT flag.
    fire_link_to_host(&mut r, timemon);
    let action = r
        .game
        .pending_selection
        .as_ref()
        .expect("first fire")
        .valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    let _ = r.auto_resolve();

    // Cycle through P1's turn back to P0's turn.
    r.game.end_turn(); // P0 → P1
    r.game.end_turn(); // P1 → P0

    // Fire again — lockout should be cleared.
    fire_link_to_host(&mut r, timemon);
    assert!(
        r.game.pending_selection.is_some(),
        "OPT lockout must clear after end_turn — clause fires again next turn"
    );
}

/// NEGATIVE (opponent's turn): when_card_linked_to_this must NOT fire on P1's turn.
/// Because the clause is active_when: { your_turn: true } (P0's turn only).
#[test]
fn bt21_059_host_when_linked_does_not_fire_on_opponent_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-LV4", Some(0));

    // Advance to P1's turn.
    r.game.end_turn(); // now P1's turn

    // Fire OnLink on Timemon while it's P1's turn.
    fire_link_to_host(&mut r, timemon);

    assert!(
        r.game.pending_selection.is_none(),
        "when_card_linked_to_this (active_when: your_turn) must not fire on opponent's turn"
    );
}

// ─── Section 3 — Linked-side when_linked behavioral ──────────────────────────

/// POSITIVE (linked-side): when Timemon gets linked to a host, the [When Linking]
/// De-Digivolve clause fires (scope: linked, when: when_linked).
/// Drive via the link action on Timemon itself.
#[test]
fn bt21_059_linked_when_linking_de_digivolve_fires() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    let opp_base = r.place_on_field(1, "OPP-LV3", Some(0));
    push_on_stack(&mut r, 1, opp_base.index as usize, "OPP-LV4");
    let trash_before = r.trash_size(1);

    // Activate the link action on Timemon → select host → link fires WhenLinked.
    r.game.decode_action(link_bit(timemon) as u16, 0);
    let host_sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("host-pick installs");
    let host_action = host_sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);

    // [When Linking] De-Digivolve prompt must surface.
    assert!(
        r.game.pending_selection.is_some(),
        "[When Linking] De-Digivolve selection must surface after Timemon links to host"
    );
    let de_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, de_action);
    let _ = r.auto_resolve();

    // Lv4 source trashed, Lv3 base remains.
    let perm = &r.game.player(1).battle_area[opp_base.index as usize];
    assert_eq!(
        perm.stack_size(),
        1,
        "[When Linking] De-Digivolve trashed the Lv4 top source"
    );
    assert_eq!(
        r.trash_size(1),
        trash_before + 1,
        "Lv4 source trashed to opponent's trash"
    );
}

/// The linked-side De-Digivolve also stops at Lv3 (same stop_at_level: 3 rule).
#[test]
fn bt21_059_linked_when_linking_de_digivolve_stops_at_level_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    r.game.enter_main_phase();

    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let timemon = r.place_on_field(0, CARD_ID, Some(0));
    let opp_lv3 = r.place_on_field(1, "OPP-LV3", Some(0)); // single Lv3 stack

    r.game.decode_action(link_bit(timemon) as u16, 0);
    let host_sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("host-pick installs");
    let host_action = host_sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);

    // Selection surfaces.
    assert!(
        r.game.pending_selection.is_some(),
        "De-Digivolve selection surfaces"
    );
    let de_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, de_action);
    let _ = r.auto_resolve();

    // Lv3 stays intact.
    let perm = &r.game.player(1).battle_area[opp_lv3.index as usize];
    assert_eq!(
        perm.stack_size(),
        1,
        "Lv3-only is a no-op — stop_at_level 3"
    );
    assert_eq!(r.trash_size(1), 0, "nothing trashed");
}
