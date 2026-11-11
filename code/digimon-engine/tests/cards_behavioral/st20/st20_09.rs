//! ST20-09 MegaKabuterimon — Lv.5, Green, Insectoid / ADVENTURE, Vaccine, DP 7000, Cost 7.
//!
//! # Card text (official Bandai DB, `data/card_bundles/ST20-09.md`)
//!
//! **[On Play] [When Digivolving]** 1 of your Digimon unsuspends. Then, for every
//! 2 colors your Tamers with the [ADVENTURE] trait have, suspend 1 of your
//! opponent's Digimon.
//!
//! **[Your Turn] [Once Per Turn]** When your other Digimon are played or
//! digivolve, if any of them have the [ADVENTURE] trait, 1 of your Digimon gains
//! `<Alliance>` for the turn. Then, 1 of your Digimon may attack.
//!
//! **Inherited:** `<Alliance>`
//!
//! Digivolve circle: Green Lv.4 / cost 3.
//! `[Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3` (xros_req).
//!
//! Official Q&A (bundle): the `<Alliance>` grant is mandatory - "you must give to
//! 1 of your Digimon".
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Green/ST20_09.cs`
//! - `EffectTiming.None` `AddSelfDigivolutionRequirementStaticEffect`:
//!   permanentCondition = Lv.4 && HasAdventureTraits, cost 3,
//!   ignoreDigivolutionRequirement false.
//! - Shared OP / WD body: mandatory `SelectPermanentEffect` Mode.UnTap over own
//!   Digimon (any, even unsuspended), then if an opponent Digimon exists and
//!   `TamerTwoColourCount() > 0`, mandatory Mode.Tap over opponent Digimon with
//!   `maxCount = min(opp Digimon, TamerTwoColourCount())`, `canEndNotMax: false`.
//!   `TamerTwoColourCount` = distinct colors among own ADVENTURE Tamers / 2.
//! - [Your Turn][OPT] (`maxCount 1`): fires on an own *other* Digimon played or
//!   digivolved, requires that Digimon to be [ADVENTURE]; mandatory Alliance pick
//!   (UntilEachTurnEnd) then optional (`canNoSelect: true`) attacker pick.
//! - Inherited `AllianceSelfEffect`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md section 4.3)
//! - xros_req alt-path (`kind: digivolve`, ST20-10 idiom): positive / negative per
//!   condition (trait route, colour route, wrong level, wrong colour/trait), cost
//!   proven by memory delta and the `Digivolve` event.
//! - [On Play]/[When Digivolving] unsuspend + formula-capped opponent multi-suspend
//!   (`select_count_capped_multi` + `clamp_to_available`): colour counting (shared
//!   colours, non-ADVENTURE and opponent Tamers excluded), clamp to available
//!   targets, mandatory picks, Tamers never targetable.
//! - Observer trigger on own other Digimon played / digivolved (EX12-036 idiom)
//!   with OPT, `<Alliance>` for the turn and an optional effect attack (EX13-064).
//! - Inherited `<Alliance>` via the digivolution-stack walk.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, ATTACK_START, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST20-09";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, level: u8, cost: u16, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![color];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

/// A hand/evo Digimon that can digivolve from a Lv.3 Red base for free.
fn evo_digimon(id: &str, adventure: bool) -> CardData {
    let mut c = digimon(id, 4, 4, CardColor::Red, adventure);
    c.evo_costs = vec![EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 0,
    }];
    c
}

fn tamer(id: &str, colors: &[CardColor], adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = colors.to_vec();
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-09 in embedded DSL pack")
        .add_card(filler("PAD"))
        // Own board / hand Digimon.
        .add_card(digimon("OTHER", 4, 4, CardColor::Red, false))
        .add_card(digimon("SUSP", 4, 4, CardColor::Red, false))
        .add_card(evo_digimon("ADV-HAND", true))
        .add_card(evo_digimon("ADV-HAND2", true))
        .add_card(evo_digimon("PLAIN-HAND", false))
        .add_card(digimon("BASE3", 3, 3, CardColor::Red, false))
        .add_card(digimon("BASE-ADV", 4, 4, CardColor::Red, true))
        // Opponent board.
        .add_card(digimon("OPP-A", 4, 4, CardColor::Blue, false))
        .add_card(digimon("OPP-B", 4, 4, CardColor::Blue, false))
        .add_card(digimon("OPP-C", 4, 4, CardColor::Blue, false))
        .add_card(tamer("OPP-T", &[CardColor::Blue], false))
        // Tamers by colour set.
        .add_card(tamer("T-RY", &[CardColor::Red, CardColor::Yellow], true))
        .add_card(tamer("T-BG", &[CardColor::Blue, CardColor::Green], true))
        .add_card(tamer("T-RG", &[CardColor::Red, CardColor::Green], true))
        .add_card(tamer("T-R", &[CardColor::Red], true))
        .add_card(tamer(
            "T-NON",
            &[CardColor::Black, CardColor::Purple, CardColor::White],
            false,
        ))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn is_suspended(runner: &DebugRunner, player: u8, index: u8) -> bool {
    runner.game.players[player as usize].battle_area[index as usize].is_suspended
}

fn set_suspended(runner: &mut DebugRunner, player: u8, index: u8, value: bool) {
    runner.game.players[player as usize].battle_area[index as usize].is_suspended = value;
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand"))
}

/// Pick an own-field slot at the current OwnField prompt.
fn pick_own(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .execute_action(0, ATTACK_START + h.index as u16)
        .expect("pick own permanent");
}

/// Pick an opponent-field slot at the current OppField prompt.
fn pick_opp(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .execute_action(0, encode_attack(0, h.index as u16))
        .expect("pick opponent permanent");
}

/// Run the clause-1 unsuspend prompt, picking `h`, and assert it was a
/// mandatory own-field prompt.
fn answer_unsuspend(runner: &mut DebugRunner, h: PermanentHandle) {
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "the unsuspend prompt comes first"
    );
    assert!(
        !runner.pending_is_optional(),
        "'1 of your Digimon unsuspends' is mandatory"
    );
    pick_own(runner, h);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_09_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "MegaKabuterimon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    let traits = card.all_traits();
    for want in ["Insectoid", "ADVENTURE", "Vaccine"] {
        assert!(
            traits.iter().any(|t| t.eq_ignore_ascii_case(want)),
            "missing trait {want}: {traits:?}"
        );
    }
}

#[test]
fn st20_09_has_two_digivolve_alt_paths_both_cost_3_respecting_requirements() {
    let card = compiled(CARD_ID);
    assert_eq!(card.alt_paths.len(), 2, "circle route + xros_req route");
    for (i, path) in card.alt_paths.iter().enumerate() {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve, "path {i}");
        assert_eq!(
            path.cost,
            Some(CompiledCost::Literal(3)),
            "path {i} costs 3"
        );
        assert!(
            !path.ignore_requirements,
            "path {i}: DCGO ignoreDigivolutionRequirement is false"
        );
        assert_eq!(
            path.from.as_ref().and_then(|f| f.level_eq),
            Some(4),
            "path {i} digivolves from Lv.4"
        );
    }
}

#[test]
fn st20_09_clause_shapes() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        2,
        "[OP][WD] clause + [Your Turn] observer clause"
    );

    let op_wd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD clause");
    assert!(op_wd.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(
        !op_wd.optional,
        "unsuspend + suspend are mandatory (DCGO canNoSelect false)"
    );
    assert!(!op_wd.once_per_turn);

    let observer = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAllyPlayed))
        .expect("observer clause");
    assert!(observer.when.contains(&CompiledTiming::OnDigivolve));
    assert!(observer.once_per_turn, "[Once Per Turn]");
    assert!(
        observer
            .active_when
            .as_ref()
            .is_some_and(|p| p.your_turn == Some(true)),
        "[Your Turn]"
    );
    assert!(
        observer.trigger_if.is_some(),
        "other own ADVENTURE Digimon trigger qualifier"
    );

    let inherited_alliance = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Alliance")
        )
    });
    assert!(inherited_alliance, "inherited <Alliance> grant present");
    assert_eq!(card.effects.len(), 3, "no clauses beyond the printed three");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve routes ([Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3)
// ═══════════════════════════════════════════════════════════════════════════════

/// Try to digivolve ST20-09 (from hand) onto `base_id`; returns (success,
/// memory paid, did the Digivolve event record the cost).
fn digivolve_onto(base: CardData) -> (bool, i16, Option<i16>) {
    let base_id = base.card_id.clone();
    let mut runner = builder()
        .add_card(base)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let handle = runner.place_on_field(0, &base_id, Some(0));
    let idx = hand_index(&runner, 0, CARD_ID);
    let mem_before = runner.memory();
    let cp = runner.event_checkpoint();
    let ok =
        runner
            .game
            .digivolve_from_hand(0, idx, handle.index as usize, PlaySource::ByDigivolve);
    let paid = mem_before - runner.memory();
    let event_paid = runner.events_since(cp).iter().find_map(|e| match e {
        GameEvent::Digivolve {
            top_card_id,
            memory_paid,
            ..
        } if top_card_id == CARD_ID => Some(*memory_paid),
        _ => None,
    });
    (ok, paid, event_paid)
}

/// xros_req route: a Lv.4 [ADVENTURE] Digimon of ANY colour (red here, not green).
#[test]
fn st20_09_digivolves_from_lv4_adventure_of_any_colour_for_3() {
    let (ok, paid, event_paid) =
        digivolve_onto(digimon("BASE-ADV-RED", 4, 4, CardColor::Red, true));
    assert!(ok, "Lv.4 [ADVENTURE] Digimon is a legal base");
    assert_eq!(paid, 3, "cost 3");
    assert_eq!(
        event_paid,
        Some(3),
        "the Digivolve event records memory_paid 3"
    );
}

/// Printed circle: a green Lv.4 without the trait.
#[test]
fn st20_09_digivolves_from_green_lv4_for_3() {
    let (ok, paid, _) = digivolve_onto(digimon("BASE-GREEN", 4, 4, CardColor::Green, false));
    assert!(ok, "green Lv.4 is a legal base via the printed circle");
    assert_eq!(paid, 3, "cost 3");
}

/// Negative: Lv.4 that is neither green nor [ADVENTURE].
#[test]
fn st20_09_cannot_digivolve_from_non_green_non_adventure_lv4() {
    let (ok, paid, _) = digivolve_onto(digimon("BASE-RED", 4, 4, CardColor::Red, false));
    assert!(!ok, "red non-ADVENTURE Lv.4 is not a legal base");
    assert_eq!(paid, 0, "nothing paid");
}

/// Negative: the xros_req is "Lv.4 w/[ADVENTURE]" - a Lv.3 [ADVENTURE] does not qualify.
#[test]
fn st20_09_cannot_digivolve_from_lv3_adventure() {
    let (ok, _, _) = digivolve_onto(digimon("BASE-ADV-L3", 3, 3, CardColor::Red, true));
    assert!(!ok, "Lv.3 [ADVENTURE] is not a legal base");
}

/// Negative: a green Lv.5 is the wrong level for both routes.
#[test]
fn st20_09_cannot_digivolve_from_green_lv5() {
    let (ok, _, _) = digivolve_onto(digimon("BASE-GREEN-L5", 5, 5, CardColor::Green, false));
    assert!(!ok, "Lv.5 is not a legal base");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] [When Digivolving]: unsuspend, then suspend per 2 Tamer colours
// ═══════════════════════════════════════════════════════════════════════════════

/// 4 distinct ADVENTURE-Tamer colours -> 2 opponent Digimon suspended, chosen by
/// the controller; the third is untouched; the unsuspend picks a suspended Digimon.
#[test]
fn st20_09_on_play_unsuspends_then_suspends_two_for_four_colours() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let susp = runner.place_on_field(0, "SUSP", Some(0));
    let own_tamer = runner.place_on_field(0, "T-RY", Some(0));
    runner.place_on_field(0, "T-BG", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    let c = runner.place_on_field(1, "OPP-C", Some(0));
    set_suspended(&mut runner, 0, susp.index, true);
    set_suspended(&mut runner, 0, own_tamer.index, true);

    fire(&mut runner, EffectTiming::OnPlay, me);

    // Unsuspend prompt: Digimon only (the suspended Tamer is not offered).
    let view = runner.pending_selection_view().expect("unsuspend prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.valid_action_ids
            .contains(&(ATTACK_START + susp.index as u16)),
        "the suspended Digimon is offered"
    );
    assert!(
        view.valid_action_ids
            .contains(&(ATTACK_START + me.index as u16)),
        "an already-unsuspended Digimon stays selectable (DCGO)"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&(ATTACK_START + own_tamer.index as u16)),
        "Tamers are not Digimon"
    );
    answer_unsuspend(&mut runner, susp);
    assert!(
        !is_suspended(&runner, 0, susp.index),
        "the chosen Digimon unsuspended"
    );
    assert!(
        is_suspended(&runner, 0, own_tamer.index),
        "the Tamer was not touched"
    );

    // Two mandatory suspend picks (4 colours / 2).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional(), "the suspends are mandatory");
    pick_opp(&mut runner, a);
    let view = runner
        .pending_selection_view()
        .expect("second suspend pick is required");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, a.index as u16)),
        "an already-chosen Digimon is not offered twice"
    );
    pick_opp(&mut runner, b);

    assert!(runner.pending_selection().is_none(), "exactly 2 picks");
    assert!(is_suspended(&runner, 1, a.index), "A suspended");
    assert!(is_suspended(&runner, 1, b.index), "B suspended");
    assert!(!is_suspended(&runner, 1, c.index), "C untouched");
}

/// 3 distinct colours (red+yellow, red+green share red) -> floor(3/2) = 1.
#[test]
fn st20_09_three_distinct_colours_suspend_only_one() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    runner.place_on_field(0, "T-RG", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    let c = runner.place_on_field(1, "OPP-C", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    pick_opp(&mut runner, b);

    assert!(
        runner.pending_selection().is_none(),
        "floor(3 / 2) = 1 pick only"
    );
    assert!(is_suspended(&runner, 1, b.index));
    assert!(!is_suspended(&runner, 1, a.index));
    assert!(!is_suspended(&runner, 1, c.index));
}

/// One two-colour ADVENTURE Tamer -> 1 suspend.
#[test]
fn st20_09_single_two_colour_tamer_suspends_one() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);
    pick_opp(&mut runner, a);

    assert!(
        runner.pending_selection().is_none(),
        "2 colours -> exactly 1 pick"
    );
    assert!(is_suspended(&runner, 1, a.index));
    assert!(!is_suspended(&runner, 1, b.index));
}

/// Negative: a single colour is below the threshold of 2 -> nothing is suspended
/// (the unsuspend still happens).
#[test]
fn st20_09_one_colour_suspends_nobody() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let susp = runner.place_on_field(0, "SUSP", Some(0));
    runner.place_on_field(0, "T-R", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    set_suspended(&mut runner, 0, susp.index, true);

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, susp);

    assert!(
        runner.pending_selection().is_none(),
        "floor(1 / 2) = 0: no suspend prompt"
    );
    assert!(
        !is_suspended(&runner, 0, susp.index),
        "the unsuspend half still resolves"
    );
    assert!(!is_suspended(&runner, 1, a.index));
}

/// Negative: Tamers WITHOUT [ADVENTURE] do not contribute colours. T-RY (2) +
/// T-NON (3 colours, not ADVENTURE) -> 2 colours -> 1 pick (5 would give 2).
#[test]
fn st20_09_non_adventure_tamer_colours_do_not_count() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    runner.place_on_field(0, "T-NON", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    let c = runner.place_on_field(1, "OPP-C", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);
    pick_opp(&mut runner, a);

    assert!(
        runner.pending_selection().is_none(),
        "the non-ADVENTURE Tamer's 3 colours must not add a second suspend"
    );
    assert!(is_suspended(&runner, 1, a.index));
    assert!(!is_suspended(&runner, 1, b.index));
    assert!(!is_suspended(&runner, 1, c.index));
}

/// Negative: "your Tamers" - an ADVENTURE Tamer on the OPPONENT's side does not count.
#[test]
fn st20_09_opponents_adventure_tamers_do_not_count() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(1, "T-RY", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);

    assert!(
        runner.pending_selection().is_none(),
        "no own ADVENTURE Tamer -> 0 suspends"
    );
    assert!(!is_suspended(&runner, 1, a.index));
}

/// Clamp: 4 colours (2 suspends) but only 1 opponent Digimon -> it is suspended,
/// no second prompt.
#[test]
fn st20_09_suspend_count_clamps_to_available_opponent_digimon() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    runner.place_on_field(0, "T-BG", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);
    let _ = runner.auto_resolve();

    assert!(runner.pending_selection().is_none());
    assert!(
        is_suspended(&runner, 1, a.index),
        "the only opponent Digimon is suspended"
    );
}

/// Opponent Tamers are never suspend targets ("Digimon"), even when they are the
/// only other permanents; the picker only offers opponent Digimon.
#[test]
fn st20_09_opponent_tamers_are_not_suspend_targets() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    let opp_t = runner.place_on_field(1, "OPP-T", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);
    let view = runner.pending_selection_view().expect("suspend prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, a.index as u16)));
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, b.index as u16)));
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, opp_t.index as u16)),
        "the opponent's Tamer is not a legal target"
    );
}

/// Negative: no opponent Digimon at all -> no suspend prompt.
#[test]
fn st20_09_no_opponent_digimon_means_no_suspend_prompt() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    runner.place_on_field(0, "T-BG", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);
    answer_unsuspend(&mut runner, other);

    assert!(runner.pending_selection().is_none(), "nothing to suspend");
}

/// The unsuspend pick is mandatory even when nothing is suspended (an unsuspended
/// Digimon is still selectable in DCGO), and it is never optional.
#[test]
fn st20_09_unsuspend_prompt_is_mandatory_when_nothing_is_suspended() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OTHER", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, me);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(
        !runner.pending_is_optional(),
        "no PASS: the choice is mandatory"
    );
}

/// [When Digivolving] runs the same body.
#[test]
fn st20_09_when_digivolving_runs_the_same_unsuspend_then_suspend() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let susp = runner.place_on_field(0, "SUSP", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    set_suspended(&mut runner, 0, susp.index, true);

    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    answer_unsuspend(&mut runner, susp);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    pick_opp(&mut runner, b);

    assert!(runner.pending_selection().is_none());
    assert!(
        !is_suspended(&runner, 0, susp.index),
        "unsuspended on [When Digivolving]"
    );
    assert!(
        is_suspended(&runner, 1, b.index),
        "suspended on [When Digivolving]"
    );
    assert!(!is_suspended(&runner, 1, a.index));
}

/// Integrated through a real play from hand (cost 7).
#[test]
fn st20_09_real_play_from_hand_runs_the_on_play_effect() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    let other = runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "T-RY", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let mem_before = runner.memory();

    let _ = runner.play(0, 0);
    assert_eq!(runner.memory(), mem_before - 7, "printed cost 7 paid");
    answer_unsuspend(&mut runner, other);
    let _ = runner.auto_resolve();

    assert!(
        is_suspended(&runner, 1, a.index),
        "[On Play] suspended an opponent Digimon"
    );
    // ST20-09 is itself an [ADVENTURE] Digimon, but the observer clause says
    // "your OTHER Digimon": its own entry must not hand out <Alliance>.
    for (i, _) in runner.game.players[0].battle_area.iter().enumerate() {
        let h = PermanentHandle {
            player: 0,
            index: i as u8,
        };
        assert!(
            !runner.game.has_keyword(h, Keyword::Alliance),
            "nobody gained <Alliance> from ST20-09's own play"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Your Turn] [Once Per Turn] other Digimon played/digivolve -> Alliance + attack
// ═══════════════════════════════════════════════════════════════════════════════

fn observer_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .hand(0, &["ADV-HAND", "ADV-HAND2", "PLAIN-HAND"])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    runner.skip_mulligan();
    // Placed on a previous turn so they are free to attack.
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    (runner, me, other)
}

/// Play an [ADVENTURE] Digimon: the mandatory Alliance pick appears (including the
/// new Digimon), the grant lands, then an optional attack prompt offers only
/// Digimon that can attack (not the one just played).
#[test]
fn st20_09_adventure_play_grants_alliance_then_offers_an_optional_attack() {
    let (mut runner, me, other) = observer_runner();
    let slot = hand_index(&runner, 0, "ADV-HAND");
    let new_idx = runner.play(0, slot).expect("ADVENTURE Digimon plays");

    // Prompt 1: Alliance recipient - mandatory, any own Digimon.
    let view = runner.pending_selection_view().expect("alliance prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        !view.is_optional,
        "official Q&A: you must give <Alliance> to 1 of your Digimon"
    );
    for h in [me.index, other.index, new_idx as u8] {
        assert!(
            view.valid_action_ids.contains(&(ATTACK_START + h as u16)),
            "every own Digimon is a legal recipient (slot {h})"
        );
    }
    pick_own(&mut runner, other);
    assert!(
        runner.game.has_keyword(other, Keyword::Alliance),
        "OTHER gained <Alliance> for the turn"
    );

    // Prompt 2: "may attack" - optional, only Digimon that can attack.
    let view = runner.pending_selection_view().expect("attack prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "'1 of your Digimon may attack'");
    assert!(view
        .valid_action_ids
        .contains(&(ATTACK_START + me.index as u16)));
    assert!(view
        .valid_action_ids
        .contains(&(ATTACK_START + other.index as u16)));
    assert!(
        !view
            .valid_action_ids
            .contains(&(ATTACK_START + new_idx as u16)),
        "the Digimon just played is summoning-sick and cannot attack"
    );
}

/// Choosing an attacker really starts the attack (and suspends it).
#[test]
fn st20_09_chosen_digimon_attacks() {
    let (mut runner, me, other) = observer_runner();
    let slot = hand_index(&runner, 0, "ADV-HAND");
    runner.play(0, slot).expect("plays");
    pick_own(&mut runner, other); // Alliance to OTHER
    pick_own(&mut runner, me); // MegaKabuterimon attacks
    let sec_before = runner.security_count(1);
    let _ = runner.auto_resolve();

    assert!(
        is_suspended(&runner, 0, me.index),
        "the attacker suspended to attack"
    );
    assert!(
        runner.security_count(1) < sec_before,
        "the attack hit the security stack"
    );
}

/// Declining the attack: Alliance stays granted, nobody attacks.
#[test]
fn st20_09_attack_can_be_declined_but_alliance_is_still_granted() {
    let (mut runner, me, other) = observer_runner();
    let slot = hand_index(&runner, 0, "ADV-HAND");
    runner.play(0, slot).expect("plays");
    pick_own(&mut runner, other);
    runner.execute_action(0, PASS).expect("decline the attack");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.has_keyword(other, Keyword::Alliance),
        "Alliance still granted"
    );
    assert!(!is_suspended(&runner, 0, me.index), "nobody attacked");
    assert_eq!(runner.security_count(1), 3, "security untouched");
}

/// "For the turn": the Alliance is gone once the turn ends.
#[test]
fn st20_09_alliance_expires_at_end_of_turn() {
    let (mut runner, _me, other) = observer_runner();
    let slot = hand_index(&runner, 0, "ADV-HAND");
    runner.play(0, slot).expect("plays");
    pick_own(&mut runner, other);
    runner.execute_action(0, PASS).expect("decline the attack");
    let _ = runner.auto_resolve();
    assert!(
        runner.game.has_keyword(other, Keyword::Alliance),
        "precondition"
    );

    runner.game.set_memory(-3);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    assert!(
        !runner.game.has_keyword(other, Keyword::Alliance),
        "<Alliance> lasted only for the turn"
    );
}

/// Negative: a Digimon without [ADVENTURE] does not trigger the clause.
#[test]
fn st20_09_non_adventure_play_does_not_trigger() {
    let (mut runner, _me, _other) = observer_runner();
    let slot = hand_index(&runner, 0, "PLAIN-HAND");
    runner.play(0, slot).expect("plays");
    assert!(
        runner.pending_selection().is_none(),
        "no prompt for a non-ADVENTURE Digimon"
    );
}

/// Once per turn: the second ADVENTURE Digimon in the same turn does not retrigger.
#[test]
fn st20_09_observer_is_once_per_turn() {
    let (mut runner, _me, other) = observer_runner();
    let slot = hand_index(&runner, 0, "ADV-HAND");
    runner.play(0, slot).expect("first plays");
    pick_own(&mut runner, other);
    runner.execute_action(0, PASS).expect("decline the attack");
    let _ = runner.auto_resolve();

    let slot = hand_index(&runner, 0, "ADV-HAND2");
    runner.play(0, slot).expect("second plays");
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] is spent"
    );
}

/// [Your Turn]: the clause is off outside the controller's turn.
#[test]
fn st20_09_observer_is_off_on_the_opponents_turn() {
    let mut runner = builder().memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OTHER", Some(0));
    let adv = runner.place_on_field(0, "ADV-HAND", Some(0));
    runner.game.turn_player_idx = 1;

    runner
        .game
        .fire_play_event_triggers(0, adv.index as usize, true, false);
    let _ = runner.auto_resolve();

    for (i, _) in runner.game.players[0].battle_area.iter().enumerate() {
        let h = PermanentHandle {
            player: 0,
            index: i as u8,
        };
        assert!(
            !runner.game.has_keyword(h, Keyword::Alliance),
            "[Your Turn] gate"
        );
    }
}

/// Only "your" Digimon: the opponent playing an [ADVENTURE] Digimon on their own
/// turn does not trigger our clause.
#[test]
fn st20_09_opponents_adventure_play_does_not_trigger() {
    let mut runner = builder().hand(1, &["ADV-HAND"]).memory(0).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OTHER", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    let _ = runner.play(1, 0);
    assert!(
        runner.pending_selection().is_none(),
        "opponent's play does not trigger"
    );
}

/// Digivolving another own Digimon into an [ADVENTURE] Digimon also triggers.
#[test]
fn st20_09_adventure_digivolve_grants_alliance() {
    let mut runner = builder()
        .hand(0, &["ADV-HAND"])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE3", Some(0));
    let idx = hand_index(&runner, 0, "ADV-HAND");

    let ok = runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "ADV-HAND digivolves onto the Lv.3 base");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(!runner.pending_is_optional(), "Alliance pick is mandatory");
    pick_own(&mut runner, base);
    assert!(
        runner.game.has_keyword(base, Keyword::Alliance),
        "Alliance granted"
    );
}

/// Digivolving into a non-[ADVENTURE] Digimon does not trigger.
#[test]
fn st20_09_non_adventure_digivolve_does_not_trigger() {
    let mut runner = builder().hand(0, &["PLAIN-HAND"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE3", Some(0));
    let idx = hand_index(&runner, 0, "PLAIN-HAND");

    let ok = runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "PLAIN-HAND digivolves onto the Lv.3 base");
    assert!(
        runner.pending_selection().is_none(),
        "no prompt for a non-ADVENTURE result"
    );
}

/// "Other": ST20-09's OWN digivolution (it is an [ADVENTURE] Digimon) does not hand
/// out <Alliance> - only its [When Digivolving] prompt appears.
#[test]
fn st20_09_own_digivolve_does_not_trigger_the_observer() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    let other = runner.place_on_field(0, "OTHER", Some(0));
    let base = runner.place_on_field(0, "BASE-ADV", Some(0));
    let idx = hand_index(&runner, 0, CARD_ID);

    let ok = runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "ST20-09 digivolves onto the Lv.4 [ADVENTURE] base");

    // Only the [When Digivolving] unsuspend prompt (clause 1) - no Alliance pick.
    answer_unsuspend(&mut runner, other);
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
    for (i, _) in runner.game.players[0].battle_area.iter().enumerate() {
        let h = PermanentHandle {
            player: 0,
            index: i as u8,
        };
        assert!(
            !runner.game.has_keyword(h, Keyword::Alliance),
            "ST20-09's own digivolution must not trigger its 'other Digimon' clause"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited <Alliance>
// ═══════════════════════════════════════════════════════════════════════════════

/// With ST20-09 as a digivolution card, the carrier above it has <Alliance>.
#[test]
fn st20_09_inherited_alliance_reaches_the_carrier() {
    let mut runner = builder()
        .add_card(digimon("CARRIER", 6, 8, CardColor::Red, false))
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Alliance),
        "inherited <Alliance> flows to the carrier through the stack walk"
    );
}

/// Negative: a carrier without ST20-09 beneath it has no <Alliance>.
#[test]
fn st20_09_carrier_without_it_has_no_alliance() {
    let mut runner = builder()
        .add_card(digimon("CARRIER", 6, 8, CardColor::Red, false))
        .memory(10)
        .start();
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    assert!(!runner.game.has_keyword(carrier, Keyword::Alliance));
}

/// Negative: an inherited effect is only active while the card is UNDER another
/// card, so ST20-09 on top has no <Alliance> of its own.
#[test]
fn st20_09_on_top_it_does_not_have_its_own_inherited_alliance() {
    let mut runner = builder().memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner.game.has_keyword(me, Keyword::Alliance));
}
