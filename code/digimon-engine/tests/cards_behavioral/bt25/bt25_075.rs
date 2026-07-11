//! BT25-075 Vulcanusmon — Digimon, Lv.6, Black/Red, DP 12000, Cost 12.
//! Traits: Shaman / Olympos XII / Iliad / TS. Form: Mega. Attribute: Data.
//!
//! # Printed text (official Bandai DB bundle + card image — authoritative)
//! Digivolve: Lv.5 / Cost 4 (standard box; dual-color — Black OR Red Lv.5).
//! [Digivolve] Lv.5 w/[TS] trait: Cost 3.
//! When this card would be played, if you have fewer Digimon than your
//!   opponent, reduce the cost by 5.
//! [On Play] [When Digivolving] You may link up to 2 cards from your hand or
//!   trash to any of your Digimon without paying the cost. Then, for each of
//!   your link cards, <De-Digivolve 1> all of your opponent's Digimon.
//! [All Turns] All of your [TS] trait Digimon gain <Rush> and <Link +1>.
//! [Your Turn] When your Digimon get linked, one of them may attack.
//!
//! # DCGO C# reference (authoritative — READ-ONLY)
//! C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/
//! CardEffect/BT25/Black/BT25_075.cs
//!   - AddSelfDigivolutionRequirementStaticEffect(TopCard.EqualsTraits("TS"),
//!     cost 3, level 5) — alt-digivolve.
//!   - MandatorySelfPlayCostReduction(5, card, Owner.Count < Enemy.Count) —
//!     mandatory -5 cost when you have fewer Digimon than your opponent.
//!   - Shared OnPlay/WhenDigivolving: link up to 2 cards from hand/trash to
//!     ANY of your Digimon (free, looped zone→card→host selects), then
//!     `degenerationCount = Owner.GetBattleAreaDigimons().Map(LinkedCards).Count()`
//!     repeated `IMassDegeneration(Enemy Digimon, 1)` `degenerationCount` times
//!     — proven equivalent to a single `de_digivolve amount: degenerationCount`
//!     per opponent Digimon (see YAML header + group7_formula_batch.rs).
//!   - All Turns: RushStaticEffect + ChangeLinkMaxStaticEffect(+1), both gated
//!     on own [TS] trait Digimon.
//!   - Your Turn (EffectTiming.WhenLinked): SelectAttackEffect over the just
//!     linked host, skippable.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 cost reduction (mandatory, BeforePayCost, board-count condition)
//! - C1 linked-card source (link from hand OR trash to any own Digimon)
//! - Mass de_digivolve scaled by `own_link_card_count` formula
//! - D4 declarative aura (keyword grant + valued modifier) gated on trait
//! - on_any_link board-wide observer + may_attack_now (Track D)
//! - Structural: dual-color standard digivolve boxes + TS alt-digivolve box
//!
//! # New DSL vocabulary already landed (verified present after worktree sync)
//! `link_cards` (Gap 2, `to: own_digimon`, `from: [hand, trash]`),
//! `own_link_card_count` formula selector (G-DSL-FORMULA-OWN-LINK-CARD-COUNT),
//! `on_any_link` timing (G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledFormula,
    CompiledPerSelector, CompiledPlayerRef, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-075";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn make_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = 1;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-075 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Generic own Digimon (link host candidate).
        .add_card(make_digimon("OWN-ALLY", 4, 5000, 4, &["Beast"]))
        // A second own Digimon so >1 host can be offered.
        .add_card(make_digimon("OWN-ALLY-2", 4, 4000, 4, &["Beast"]))
        // A [TS] trait own Digimon for the All-Turns aura.
        .add_card(make_digimon("TS-ALLY", 4, 6000, 5, &["TS"]))
        // Link-eligible Option cards (link_cards has no printed filter on
        // Vulcanusmon's clause — any card from hand/trash is eligible).
        .add_card(make_option("LINK-OPT-A"))
        .add_card(make_option("LINK-OPT-B"))
        // Opponent Digimon with a digivolution stack (de-digivolve target).
        .add_card(make_digimon("OPP-LV3", 3, 3000, 3, &["Beast"]))
        .add_card(make_digimon("OPP-LV5", 5, 7000, 6, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Fire the [On Play] batch for the permanent already placed at `field_index`.
fn fire_on_play(runner: &mut DebugRunner, player: PlayerId, field_index: usize) {
    runner.fire_on_play(player, field_index);
    runner.game.drain_effect_queue();
}

/// Fire the [When Digivolving] activated effect of the permanent at `field_index`.
fn fire_when_digivolving(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

/// Push `card_id` into player `p`'s trash (direct injection, mirrors the
/// bt25_096 / bt21_101 helper convention).
fn push_to_trash(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_idx, p, next_idx));
}

/// Encode the host-select action for `select_own_permanent` (mirrors
/// `link_cards`'s `LinkStage::HostSelect` decode: `ATTACK_START` offset mod
/// `TARGETS_PER_ATTACKER` gives the field index).
fn host_action(field_index: u8) -> u16 {
    digimon_engine::action::space::encode_attack(0, field_index as u16)
}

/// Resolve every pending selection by always taking the FIRST valid action
/// (or PASS when none is offered). Used to clear follow-up prompts whose
/// specific branch is not the subject of the test.
fn resolve_all_first(r: &mut DebugRunner, player: PlayerId) {
    while r.game.pending_selection.is_some() {
        let act = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .first()
            .copied();
        match act {
            Some(a) => {
                let _ = r.game.resolve_selection(player, a);
            }
            None => {
                let _ = r
                    .game
                    .resolve_selection(player, digimon_engine::action::space::PASS);
            }
        }
    }
}

/// True when `sel` is the on_any_link observer's may-attack offer (a
/// `Target`-kind selection). The drive loop must LEAVE this prompt for the
/// test body to assert/resolve — it is not part of the link flow.
fn is_attack_offer(sel: &digimon_engine::selection::PendingSelection) -> bool {
    matches!(sel.kind, digimon_engine::selection::SelectionKind::Target)
}

/// Drive the full "link N cards to a chosen host" flow, always picking the
/// FIRST available option at every zone/card sub-prompt, but explicitly
/// choosing `host_field_index` whenever the host-select installs. Declines
/// (`PASS`) once `stop_after` cards have been linked (for `up_to` semantics).
/// A `Target`-kind prompt (the on_any_link may-attack offer, which installs
/// as soon as the link step exhausts its candidates) is never consumed —
/// the loop stops and leaves it pending for the caller.
/// Returns the number of cards actually linked via this drive loop.
fn drive_link_up_to(
    r: &mut DebugRunner,
    player: PlayerId,
    host_field_index: u8,
    stop_after: u8,
) -> u8 {
    let mut linked = 0u8;
    loop {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        if is_attack_offer(sel) {
            // The observer's may-attack offer — not ours to answer.
            break;
        }
        let host_act = host_action(host_field_index);
        if sel.valid_action_ids.contains(&host_act) {
            // Host-select stage: pick the requested host.
            let _ = r.game.resolve_selection(player, host_act);
            linked += 1;
            if linked >= stop_after {
                // Next pick's zone/card select should be declined via PASS —
                // but never the observer's may-attack offer.
                if let Some(next) = r.game.pending_selection.as_ref() {
                    if next.is_optional && !is_attack_offer(next) {
                        let _ = r
                            .game
                            .resolve_selection(player, digimon_engine::action::space::PASS);
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        if linked >= stop_after && sel.is_optional {
            let _ = r
                .game
                .resolve_selection(player, digimon_engine::action::space::PASS);
            break;
        }
        let ids = sel.valid_action_ids.clone();
        let act = ids.first().copied();
        match act {
            Some(a) => {
                let _ = r.game.resolve_selection(player, a);
            }
            None => {
                let _ = r
                    .game
                    .resolve_selection(player, digimon_engine::action::space::PASS);
                break;
            }
        }
    }
    linked
}

// ─────────────────────────────────────────────────────────────────────────────
// STRUCTURAL TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt25_075_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Vulcanusmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "trait {t} present in merged trait line"
        );
    }
}

#[test]
fn bt25_075_has_dual_color_standard_digivolve_boxes() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let digivolve_count = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert!(
        digivolve_count >= 3,
        "expected >=3 digivolve alt-paths (Black Lv.5, Red Lv.5, TS-trait Lv.5); got {digivolve_count}"
    );
}

#[test]
fn bt25_075_has_cost_reduction_declarative() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this,
                amount,
                ..
            }) if *when_playing_this && *amount == Some(5)
        )
    });
    assert!(
        has,
        "mandatory -5 cost_reduction declarative (when_playing_this) must be compiled"
    );
}

#[test]
fn bt25_075_has_wd_op_link_then_mass_de_digivolve_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        let CompiledClause::Triggered(t) = c else {
            return false;
        };
        let op = t.when.contains(&CompiledTiming::OnPlay);
        let wd = t.when.contains(&CompiledTiming::WhenDigivolving);
        let links = t
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::LinkCards { .. }));
        let for_each_dedigi = t.process.iter().any(|s| {
            matches!(s, CompiledStep::ForEach { body, .. }
                if body.iter().any(|b| matches!(b, CompiledStep::DeDigivolve { .. })))
        });
        op && wd && links && for_each_dedigi
    });
    assert!(
        has,
        "[On Play][When Digivolving] clause links up to 2 cards then mass de-digivolves"
    );
}

#[test]
fn bt25_075_de_digivolve_amount_scales_with_own_link_card_count() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let mut found = false;
    for c in &card.effects {
        let CompiledClause::Triggered(t) = c else {
            continue;
        };
        for step in &t.process {
            if let CompiledStep::ForEach { body, .. } = step {
                for b in body {
                    if let CompiledStep::DeDigivolve { amount_fn, .. } = b {
                        if let Some(CompiledFormula::BasePerDelta { per, .. }) = amount_fn {
                            if matches!(
                                per,
                                CompiledPerSelector::OwnLinkCardCount {
                                    of: CompiledPlayerRef::You
                                }
                            ) {
                                found = true;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        found,
        "de_digivolve amount must be a base_per_delta formula over own_link_card_count"
    );
}

#[test]
fn bt25_075_has_all_turns_ts_aura_with_rush_and_link_max() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            grant_keyword, modifier, modifier_value, ..
        }) if grant_keyword.is_some() && modifier.as_deref() == Some("ChangeLinkMax") && *modifier_value == Some(1)
    ));
    assert!(
        has,
        "[All Turns] aura must grant Rush + ChangeLinkMax(+1) to own [TS] Digimon"
    );
}

#[test]
fn bt25_075_has_on_any_link_may_attack_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        let CompiledClause::Triggered(t) = c else {
            return false;
        };
        let oal = t.when.contains(&CompiledTiming::OnAnyLink);
        let attacks = t
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::MayAttackNow { .. }));
        oal && t.optional && attacks
    });
    assert!(
        has,
        "[Your Turn][OPT] on_any_link clause must be optional and offer may_attack_now"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIORAL TESTS — Cost reduction
// ─────────────────────────────────────────────────────────────────────────────

/// Positive: fewer own Digimon than opponent's → -5 cost when played from hand.
#[test]
fn bt25_075_cost_reduced_by_5_when_fewer_own_digimon_than_opponent() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    // Opponent has 2 Digimon; controller has 0 → fewer own Digimon.
    r.place_on_field(1, "OPP-LV3", None);
    r.place_on_field(1, "OPP-LV5", None);
    advance_to_main(&mut r);

    let mem_before = r.memory();
    let played = r.play(0, 0);
    assert!(played.is_some(), "BT25-075 plays");
    let cost = mem_before - r.memory();
    assert_eq!(
        cost, 7,
        "printed cost 12 reduced by 5 when fewer own Digimon than opponent; got {cost}"
    );
}

/// Negative: equal Digimon count → full printed cost is paid (no reduction).
#[test]
fn bt25_075_no_cost_reduction_when_not_fewer_own_digimon() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    // Controller has 1 own Digimon, opponent has 1 → NOT fewer.
    r.place_on_field(0, "OWN-ALLY", None);
    r.place_on_field(1, "OPP-LV3", None);
    advance_to_main(&mut r);

    let mem_before = r.memory();
    let played = r.play(0, 0);
    assert!(played.is_some(), "BT25-075 plays");
    let cost = mem_before - r.memory();
    assert_eq!(
        cost, 12,
        "full printed cost (12) paid when not fewer own Digimon; got {cost}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIORAL TESTS — [On Play] link + mass de-digivolve
// ─────────────────────────────────────────────────────────────────────────────

/// [On Play]: linking 1 card from hand to a chosen own Digimon, then
/// <De-Digivolve 1> hits all opponent Digimon exactly once (1 own link card).
#[test]
fn bt25_075_on_play_links_one_from_hand_then_de_digivolves_opponents_once() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    // Opponent Digimon with a 2-card digivolution stack (de-digivolve target).
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV5"]);
    let opp_sources_before = r.game.player(1).battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_sources_before, 2,
        "precondition: opponent stack has 2 sources"
    );

    fire_on_play(&mut r, 0, vulcan.index as usize);
    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] installs the link-card selection (LINK-OPT-A is a candidate)"
    );

    let linked = drive_link_up_to(&mut r, 0, ally.index, 1);
    assert_eq!(linked, 1, "exactly 1 card linked to the chosen ally");
    resolve_all_first(&mut r, 0);

    assert_eq!(
        r.game.player(0).battle_area[ally.index as usize]
            .linked_cards
            .len(),
        1,
        "LINK-OPT-A attached to the chosen own Digimon"
    );
    assert_eq!(
        r.game.player(1).battle_area[opp.index as usize]
            .card_sources
            .len(),
        opp_sources_before - 1,
        "opponent's Digimon de-digivolved by exactly 1 (1 own link card)"
    );
}

/// Linking 2 cards (from hand) means <De-Digivolve 2> hits opponents (the
/// mass de-digivolve amount scales with the TOTAL own link card count).
#[test]
fn bt25_075_on_play_links_two_then_de_digivolves_opponents_by_two() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A", "LINK-OPT-B"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    // Opponent stack with 3 sources so a -2 de-digivolve is observable.
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV3", "OPP-LV5"]);
    let opp_sources_before = r.game.player(1).battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(opp_sources_before, 3);

    fire_on_play(&mut r, 0, vulcan.index as usize);
    let linked = drive_link_up_to(&mut r, 0, ally.index, 2);
    assert_eq!(linked, 2, "exactly 2 cards linked (the printed cap)");
    resolve_all_first(&mut r, 0);

    assert_eq!(
        r.game.player(0).battle_area[ally.index as usize]
            .linked_cards
            .len(),
        2,
        "both cards attached to the chosen own Digimon"
    );
    assert_eq!(
        r.game.player(1).battle_area[opp.index as usize]
            .card_sources
            .len(),
        opp_sources_before - 2,
        "opponent Digimon de-digivolved by 2 (2 total own link cards)"
    );
}

/// Negative: declining the link (no candidates linked) means the
/// de-digivolve amount is 0 — opponent Digimon are untouched.
#[test]
fn bt25_075_on_play_decline_link_does_not_de_digivolve() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV5"]);
    let opp_sources_before = r.game.player(1).battle_area[opp.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut r, 0, vulcan.index as usize);
    assert!(r.game.pending_selection.is_some());
    // Decline the very first prompt (the card-select step; up_to is declinable).
    let _ = r
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS);
    resolve_all_first(&mut r, 0);

    assert_eq!(
        r.game.player(0).battle_area.len(),
        1,
        "no own Digimon gained a linked card"
    );
    assert_eq!(
        r.game.player(1).battle_area[opp.index as usize]
            .card_sources
            .len(),
        opp_sources_before,
        "declining the link means 0 own link cards → no de-digivolve"
    );
}

/// [When Digivolving] fires the SAME shared clause (link then de-digivolve).
#[test]
fn bt25_075_when_digivolving_links_then_de_digivolves() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV5"]);
    let opp_sources_before = r.game.player(1).battle_area[opp.index as usize]
        .card_sources
        .len();

    assert!(
        fire_when_digivolving(&mut r, 0, vulcan.index as usize),
        "[When Digivolving] installs the link-card selection"
    );
    let linked = drive_link_up_to(&mut r, 0, ally.index, 1);
    assert_eq!(linked, 1);
    resolve_all_first(&mut r, 0);

    assert_eq!(
        r.game.player(1).battle_area[opp.index as usize]
            .card_sources
            .len(),
        opp_sources_before - 1,
        "[When Digivolving] mass de-digivolve fires exactly like [On Play]"
    );
}

/// Link from TRASH is offered when hand has no eligible card but trash does.
#[test]
fn bt25_075_on_play_links_from_trash_when_hand_empty() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(15).start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    push_to_trash(&mut r, 0, "LINK-OPT-A");
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV5"]);
    let opp_sources_before = r.game.player(1).battle_area[opp.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut r, 0, vulcan.index as usize);
    assert!(
        r.game.pending_selection.is_some(),
        "trash-only candidate still installs the link selection"
    );
    let linked = drive_link_up_to(&mut r, 0, ally.index, 1);
    assert_eq!(linked, 1, "the trash card was linked");
    resolve_all_first(&mut r, 0);

    assert_eq!(
        r.game.player(0).battle_area[ally.index as usize]
            .linked_cards
            .len(),
        1,
        "LINK-OPT-A linked from trash"
    );
    assert_eq!(
        r.game.player(1).battle_area[opp.index as usize]
            .card_sources
            .len(),
        opp_sources_before - 1,
        "de-digivolve fires for a trash-sourced link too"
    );
}

/// Linking pays NO memory ("without paying the cost").
#[test]
fn bt25_075_link_is_free_no_memory_paid() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));

    let mem_before = r.memory();
    fire_on_play(&mut r, 0, vulcan.index as usize);
    let _ = drive_link_up_to(&mut r, 0, ally.index, 1);
    resolve_all_first(&mut r, 0);

    assert_eq!(
        r.memory(),
        mem_before,
        "linking up to 2 cards costs 0 memory (free)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIORAL TESTS — [All Turns] TS aura
// ─────────────────────────────────────────────────────────────────────────────

/// A [TS] trait own Digimon gains Rush + Link+1 while Vulcanusmon is on field.
#[test]
fn bt25_075_ts_ally_gains_rush_and_link_plus_one() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).start();
    r.place_on_field(0, CARD_ID, Some(0));
    let ts_ally = r.place_on_field(0, "TS-ALLY", Some(1));
    advance_to_main(&mut r);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(ts_ally, Keyword::Rush),
        "[TS] ally gains <Rush> from Vulcanusmon's All-Turns aura"
    );
    assert_eq!(
        r.game.modifiers.link_max_delta(ts_ally),
        1,
        "[TS] ally gains <Link +1> from the aura"
    );
}

/// Negative: a non-[TS] own Digimon is unaffected by the aura.
#[test]
fn bt25_075_non_ts_ally_unaffected_by_aura() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).start();
    r.place_on_field(0, CARD_ID, Some(0));
    let plain_ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    advance_to_main(&mut r);
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(plain_ally, Keyword::Rush),
        "a non-[TS] ally must NOT gain <Rush>"
    );
    assert_eq!(
        r.game.modifiers.link_max_delta(plain_ally),
        0,
        "a non-[TS] ally must NOT gain <Link +1>"
    );
}

/// Vulcanusmon itself carries the [TS] trait, so it also benefits from its
/// own All-Turns aura.
#[test]
fn bt25_075_self_is_ts_and_gains_own_aura() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(vulcan, Keyword::Rush),
        "Vulcanusmon is itself [TS] and gains <Rush> from its own aura"
    );
    assert_eq!(
        r.game.modifiers.link_max_delta(vulcan),
        1,
        "Vulcanusmon gains its own <Link +1>"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIORAL TESTS — [Your Turn] on_any_link may-attack
// ─────────────────────────────────────────────────────────────────────────────

/// When one of the controller's Digimon gets linked (via Vulcanusmon's own
/// link step), the board-wide on_any_link observer offers "1 of your Digimon
/// may attack" targeting the just-linked host.
///
/// The ally is placed with `turn_played_override = Some(0)` (a PREVIOUS turn
/// — the third `place_on_field` arg is the turn the permanent entered play,
/// not a field slot). DCGO's WhenLinked gate requires the linked host to pass
/// the full `Permanent.CanAttack` check (BT25_075.cs `PermanentCondition`),
/// which includes the played-this-turn restriction
/// (`EnterFieldTurnCount == TurnCount && !HasRush → false`), so a
/// summoning-sick host gets no offer — see the companion negative test.
#[test]
fn bt25_075_on_any_link_offers_attack_for_linked_ally() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .security(1, &["DECK-PAD"; 3])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(0));
    advance_to_main(&mut r);

    fire_on_play(&mut r, 0, vulcan.index as usize);
    let linked = drive_link_up_to(&mut r, 0, ally.index, 1);
    assert_eq!(linked, 1, "1 card linked onto OWN-ALLY");

    // After the link (and the de-digivolve for-each over an empty opponent
    // board — a no-op), the on_any_link observer must offer an attack
    // targeting the just-linked ally.
    assert!(
        r.game.pending_selection.is_some(),
        "on_any_link observer installs a may-attack selection after the link"
    );
    let attack_action = digimon_engine::action::space::encode_attack(
        ally.index as u16,
        digimon_engine::action::space::SECURITY_TARGET,
    );
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .contains(&attack_action),
        "the just-linked ally is offered as the attacker"
    );

    let security_before = r.security_count(1);
    let _ = r.game.resolve_selection(0, attack_action);
    assert_eq!(
        r.security_count(1),
        security_before - 1,
        "the offered attack actually resolves against the opponent's security"
    );
}

/// Declining the on_any_link may-attack offer leaves the board unchanged.
/// (Ally placed on a previous turn — `Some(0)` — so it is attack-capable;
/// see the summoning-sickness note on the offers_attack test above.)
#[test]
fn bt25_075_on_any_link_decline_does_not_attack() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .security(1, &["DECK-PAD"; 3])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(0));
    advance_to_main(&mut r);

    fire_on_play(&mut r, 0, vulcan.index as usize);
    let _ = drive_link_up_to(&mut r, 0, ally.index, 1);

    assert!(r.game.pending_selection.is_some());
    assert!(
        r.game.pending_selection.as_ref().unwrap().is_optional,
        "the may-attack offer is optional (skippable in DCGO)"
    );
    let security_before = r.security_count(1);
    let _ = r
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS);

    assert_eq!(
        r.security_count(1),
        security_before,
        "declining the may-attack offer does not touch security"
    );
    assert!(
        !r.game.player(0).battle_area[ally.index as usize].is_suspended,
        "declining does not suspend the would-be attacker"
    );
}

/// Negative: a linked host that was played THIS turn (no <Rush>) gets NO
/// may-attack offer. DCGO gates the whole WhenLinked trigger on the host
/// passing `Permanent.CanAttack` (BT25_075.cs `PermanentCondition` →
/// `CanAttackTargetDigimon`: `EnterFieldTurnCount == TurnCount && !HasRush
/// → false`); the engine reaches the same user-visible outcome through
/// `may_attack_now`'s empty-candidate no-op (no prompt installs).
#[test]
fn bt25_075_on_any_link_no_attack_offer_for_summoning_sick_host() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .security(1, &["DECK-PAD"; 3])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    // Played THIS turn (turn_count starts at 1) → summoning-sick, no Rush
    // (OWN-ALLY is a plain Beast — Vulcanusmon's TS aura does not apply).
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    advance_to_main(&mut r);
    assert_eq!(
        r.turn_count(),
        1,
        "precondition: ally entered play this turn"
    );

    fire_on_play(&mut r, 0, vulcan.index as usize);
    let linked = drive_link_up_to(&mut r, 0, ally.index, 1);
    assert_eq!(linked, 1, "the link itself still resolves");

    assert!(
        r.game.pending_selection.is_none(),
        "a summoning-sick linked host must NOT be offered an attack \
         (DCGO WhenLinked CanAttack gate)"
    );
    let security_before = r.security_count(1);
    resolve_all_first(&mut r, 0);
    assert_eq!(
        r.security_count(1),
        security_before,
        "no attack happened against the opponent's security"
    );
}

/// Negative: on_any_link must NOT fire on the opponent's turn (Vulcanusmon's
/// clause is [Your Turn]).
#[test]
fn bt25_075_on_any_link_does_not_fire_on_opponent_turn() {
    let mut r = base()
        .hand(0, &["LINK-OPT-A"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(15)
        .start();
    let vulcan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    advance_to_main(&mut r);
    r.end_turn(); // now the opponent's turn.
    assert_eq!(r.turn_player(), 1, "precondition: opponent's turn");

    // Manually attach a link card as if Vulcanusmon's link step ran mid
    // opponent-turn (synthetic — isolates the on_any_link gate itself).
    r.push_linked_owned(ally, "LINK-OPT-A", 0);
    for pid in 0..r.game.players.len() {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host: ally,
                card: r.game.player(0).battle_area[ally.index as usize].linked_cards[0].handle(),
            },
        );
    }
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "on_any_link must NOT fire on the opponent's turn ([Your Turn] gate)"
    );
}
