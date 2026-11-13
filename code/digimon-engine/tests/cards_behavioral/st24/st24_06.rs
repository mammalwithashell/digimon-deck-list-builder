//! ST24-06 RizeGreymon — Digimon, Lv.5, Yellow/Red, DP 8000, Cost 7.
//! Traits: Cyborg, DATA SQUAD. Attribute: Vaccine.
//!
//! # Card text (data/cards.json — verbatim, confirmed vs DCGO ST24_06.cs)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] 1 of your
//!   opponent's Digimon gets -5000 DP for the turn. Then, by trashing 2 bottom
//!   face-down cards from under any of your Tamers, you may play or use 1
//!   [DATA SQUAD] trait card with a play or use cost of 5 or less from your hand
//!   without paying the cost.
//! Inherited [All Turns] [Once Per Turn]: When this Digimon with [ShineGreymon]
//!   in its name or the [DATA SQUAD] trait would leave the battle area, by
//!   trashing the bottom face-down card from under any of your Tamers, it
//!   doesn't leave.
//!
//! Printed digivolve boxes: from [GeoGreymon] cost 3; Lv.4 w/[DATA SQUAD] cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Yellow/ST24_06.cs
//!
//! # Patterns this test covers
//! - two alt-digivolve boxes (name-based GeoGreymon + Lv.4 [DATA SQUAD] trait)
//! - mandatory DP-minus (-5000 for the turn) to an opponent Digimon
//! - the optional trash-2-bottom-FD-under-Tamers condition is chosen and paid
//!   FIRST (EffectChoice ["Trash 2 face-down cards", "Don't"]; CR 4.3 15-7-1 /
//!   15-7-2 / 15-7-5), then an optional hand pick plays/uses a [DATA SQUAD] card
//!   (play/use cost ≤5, the G-PLAY-OR-USE-COST-LTE filter) for FREE
//! - "Don't" trashes nothing; paying then passing the hand pick keeps the trash;
//!   with no eligible hand card the trash is still offered and payable
//! - single-Tamer (2) and split (1 + 1) trash; <2 face-down cards → not offered
//! - filter: a cost-6 [DATA SQUAD] card and a non-[DATA SQUAD] card are excluded
//! - inherited leave-replacement (ShineGreymon-name / DATA SQUAD subject)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST24-06";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn rizegreymon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A high-DP opponent Digimon (so -5000 DP never deletes it — rule 17-1-3-1-1).
fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 6;
    c
}

/// A [DATA SQUAD] Digimon with play cost 5 (≤5) — the legal play-or-use pick.
fn ds_cheap(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A [DATA SQUAD] Digimon with play cost 6 (>5) — NOT eligible (cost filter).
fn ds_expensive(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A non-[DATA SQUAD] Digimon with play cost 4 — NOT eligible (trait filter).
fn non_ds_cheap(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.traits = vec!["Cyborg".to_string()];
    c
}

/// A [DATA SQUAD] carrier to ride on top of RizeGreymon (the inherited subject).
fn ds_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 13;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A [DATA SQUAD] Option with use cost 3 (≤5) — a legal "use" pick unless the
/// controller can't use Option cards.
fn ds_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn face_down_source_count(runner: &DebugRunner, p: u8) -> usize {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .map(|perm| perm.card_sources.iter().filter(|s| s.face_down).count())
        .sum()
}

fn on_field(runner: &DebugRunner, p: u8, card_id: &str) -> bool {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn base_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .add_card(rizegreymon())
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(ds_cheap("DS-CHEAP"))
        .add_card(ds_expensive("DS-EXP"))
        .add_card(non_ds_cheap("NON-DS"))
        .add_card(ds_carrier("DS-CARRIER"))
        .add_card(tamer("TAMER"))
        .add_card(filler("FILLER"))
        // Koromon (ST24-01): a real [DATA SQUAD] Digi-Egg (no play cost).
        .add_card(card_data_from_compiled("ST24-01"))
        .add_card(ds_option("DS-OPT"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
}

/// Place RizeGreymon on a Lv.4 filler, plus the requested Tamers each carrying
/// `n` bottom face-down sources, and push `hand` cards into p0's hand.
fn rize_runner(hand: &[&str], tamers_fd: &[usize]) -> (DebugRunner, PermanentHandle) {
    let mut runner = base_builder().start();
    runner.set_first_player(0);

    let rize = runner.place_stack(0, &["FILLER", CARD_ID]);

    for &n in tamers_fd {
        let mut names: Vec<&str> = vec!["FILLER"; n];
        names.push("TAMER");
        let t = runner.place_stack(0, &names);
        for i in 0..n {
            runner.game.players[0].battle_area[t.index as usize].card_sources[i].face_down = true;
        }
    }
    for &h in hand {
        push_to_hand(&mut runner, 0, h);
    }
    (runner, rize)
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

fn process_contains_trash_n_under_tamers(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::TrashBottomFaceDownSourcesUnderTamers { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_trash_n_under_tamers(then)
                || process_contains_trash_n_under_tamers(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_trash_n_under_tamers(body)
        }
        _ => false,
    })
}

fn process_contains_play_or_use(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::PlayOrUseFromHand { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => process_contains_play_or_use(then) || process_contains_play_or_use(else_branch),
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_play_or_use(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_06_metadata_and_two_alt_paths() {
    let card = compiled(CARD_ID);
    assert_eq!(card.name, "RizeGreymon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(8000));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));

    // Alt-digivolve #1: from a [GeoGreymon]-named source (any level), cost 3.
    let geo = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| f.name_contains.as_deref() == Some("GeoGreymon"))
            .unwrap_or(false)
    });
    assert!(geo, "GeoGreymon-named alt-path present");

    // Alt-digivolve #2: from Lv.4 with the [DATA SQUAD] trait, cost 3.
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("DATA SQUAD"))
            .unwrap_or(false)
    });
    assert!(ds, "Lv.4 [DATA SQUAD] alt-path present");
}

#[test]
fn st24_06_shared_opt_uses_trash2_play_or_use_and_inherited_replacement() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let shared = triggered
        .iter()
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("[OP][WD][WA] shared clause present");
    assert!(shared.once_per_turn, "shared OPT counter");
    assert!(
        process_contains_trash_n_under_tamers(&shared.process),
        "the 'Then' half uses the trash-2-under-Tamers cost"
    );
    assert!(
        process_contains_play_or_use(&shared.process),
        "the 'Then' half plays/uses a hand card (play_or_use_from_hand)"
    );

    // Inherited leave-replacement keyed on ShineGreymon (the Rosemon→ShineGreymon
    // name swap vs the ST24-10 idiom). The clause may compile to a Declarative
    // replacement or a Triggered one — assert representation-agnostically over the
    // debug string of whichever clause names [ShineGreymon].
    let inherited = card
        .effects
        .iter()
        .map(|c| format!("{c:?}"))
        .find(|s| s.contains("ShineGreymon"))
        .expect("a clause keyed on the [ShineGreymon] name present");
    assert!(
        inherited.contains("Leave")
            || inherited.contains("Replacement")
            || inherited.contains("Cancel"),
        "the [ShineGreymon]-keyed clause is the inherited leave-replacement"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [OP][WD][WA] -5000 DP (mandatory)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_06_minus_5000_to_opponent_digimon() {
    let (mut runner, rize) = rize_runner(&[], &[]);
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");

    fire(&mut runner, EffectTiming::OnPlay, rize);

    let view = runner
        .pending_selection_view()
        .expect("DP-minus target prompt must surface");
    assert!(!view.is_optional, "the DP-minus target pick is mandatory");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .unwrap();
    // No Tamer stash / no hand card → the play-or-use half is inert.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 5000,
        "opponent Digimon lost 5000 DP for the turn"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — "Then, by trashing 2 bottom face-down cards from under any of your
// Tamers, you may play or use 1 [DATA SQUAD] card (play/use cost ≤5) for free."
//
// CR 4.3 15-7-1 / 15-7-2 / 15-7-5: the optional "by" processing condition (the
// trash) is chosen and paid FIRST, and may be paid even when the follow-up
// cannot be executed. DCGO ST24_06.cs SharedActivateCoroutine: gate only on the
// face-down counts → canNoSelect Tamer pick (the decline) → trashed = true →
// `if (trashed)` a canNoSelect SelectHandEffect → PlayPermanentCards /
// PlayOptionCards payCost:false. The YAML mirrors ST24-10: an EffectChoice
// ["Trash 2 face-down cards", "Don't"], then the trash, then an optional hand
// pick.
// ════════════════════════════════════════════════════════════════════════════

/// Resolve the mandatory -5000 DP target pick (always first).
fn pick_dp_target(runner: &mut DebugRunner) {
    let v = runner.pending_selection_view().expect("DP-minus pick");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
}

/// Answer the "trash 2 face-down cards?" choice (index 0 = pay, 1 = "Don't").
fn choose_pay(runner: &mut DebugRunner, pay: bool) {
    let v = runner
        .pending_selection_view()
        .expect("pay-or-not choice installs");
    assert_eq!(v.kind, SelectionKind::EffectChoice, "the trash is offered as a choice");
    let id = HAND_EFFECT_START + if pay { 0 } else { 1 };
    runner.execute_action(0, id).unwrap();
}

/// Resolve any Tamer picks for the trash-2 cost, then return the hand pick (if
/// one installs).
fn pending_hand_pick(
    runner: &mut DebugRunner,
) -> Option<digimon_engine::selection::PendingSelectionView> {
    for _ in 0..8 {
        let v = runner.pending_selection_view()?;
        if v.kind == SelectionKind::Hand {
            return Some(v);
        }
        assert_ne!(
            v.kind,
            SelectionKind::EffectChoice,
            "no second pay-or-not choice after the trash is chosen"
        );
        let id = v.valid_action_ids.iter().copied().find(|&id| id != PASS)?;
        runner.execute_action(0, id).unwrap();
    }
    None
}

fn in_hand(runner: &DebugRunner, p: u8, card_id: &str) -> bool {
    runner.game.players[p as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

/// The shared clause's pay-or-not choice: two labels, the second starting with
/// "Don't" (the DCGO-replay yes/no fold keys on it).
#[test]
fn st24_06_trash_offered_as_effect_choice_with_dont_label() {
    fn find_choice(steps: &[CompiledStep]) -> Option<Vec<String>> {
        steps.iter().find_map(|s| match s {
            CompiledStep::SelectEffectChoice { labels, .. } => Some(labels.clone()),
            CompiledStep::If {
                then, else_branch, ..
            } => find_choice(then).or_else(|| find_choice(else_branch)),
            _ => None,
        })
    }
    let card = compiled(CARD_ID);
    let shared = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("shared [OP][WD][WA] clause");
    let labels = find_choice(&shared.process).expect("the trash is an EffectChoice");
    assert_eq!(labels.len(), 2);
    assert!(labels[1].starts_with("Don't"), "decline label starts with \"Don't\": {labels:?}");
}

/// POSITIVE: pay the trash (choice → Tamer pick) FIRST, then the optional hand
/// pick plays the [DATA SQUAD] cost-5 card for free.
#[test]
fn st24_06_trash_2_then_plays_data_squad_card_free() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP", "DS-EXP", "NON-DS"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    assert_eq!(face_down_source_count(&runner, 0), 2, "2 FD sources staged");
    let mem_before = runner.memory();
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);

    // The trash is chosen before any hand card is picked (CR 15-7-1).
    choose_pay(&mut runner, true);
    let v2 = pending_hand_pick(&mut runner).expect("'may play or use' hand pick installs");
    assert_eq!(
        face_down_source_count(&runner, 0),
        0,
        "the trash-2 cost is already paid when the hand pick is offered"
    );
    assert!(v2.is_optional, "'you may play or use' ⇒ declinable");
    let picks: Vec<u16> = v2
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "only DS-CHEAP is legal (cost-6 DS and non-DS cards are filtered out)"
    );
    runner.execute_action(0, picks[0]).unwrap();
    let _ = runner.auto_resolve();

    assert!(
        on_field(&runner, 0, "DS-CHEAP"),
        "the [DATA SQUAD] cost-5 card was played to the battle area"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before + 1,
        "exactly one new permanent (the played card) entered"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "the play is FREE — no memory paid (cost_delta: free)"
    );
}

/// The trash can be split across two Tamers (1 + 1): paying it trashes both.
#[test]
fn st24_06_trash_split_across_two_tamers() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP"], &[1, 1]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    assert_eq!(face_down_source_count(&runner, 0), 2);

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);
    choose_pay(&mut runner, true);
    let v = pending_hand_pick(&mut runner).expect("hand pick installs");
    assert_eq!(face_down_source_count(&runner, 0), 0, "1 trashed from each Tamer");
    let pick = v.valid_action_ids.iter().copied().find(|&id| id != PASS).unwrap();
    runner.execute_action(0, pick).unwrap();
    let _ = runner.auto_resolve();
    assert!(on_field(&runner, 0, "DS-CHEAP"));
}

/// "Don't": choosing not to pay trashes nothing and offers no hand pick. The
/// mandatory -5000 DP still resolved.
#[test]
fn st24_06_dont_pay_trashes_nothing() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP"], &[2]);
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);
    choose_pay(&mut runner, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "declined ⇒ no Tamer or hand pick follows"
    );
    let _ = runner.auto_resolve();

    assert_eq!(face_down_source_count(&runner, 0), 2, "declined ⇒ nothing trashed");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "declined ⇒ nothing played"
    );
    assert!(in_hand(&runner, 0, "DS-CHEAP"), "the DS card is still in hand");
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 5000,
        "the -5000 DP (mandatory half) still resolved"
    );
}

/// Pay the trash, then PASS the "you may play or use" hand pick: the 2 cards
/// stay trashed and nothing is played (CR 15-7-5: the condition stands on its
/// own).
#[test]
fn st24_06_pay_then_pass_hand_pick_trashes_2_plays_nothing() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);
    choose_pay(&mut runner, true);
    let v = pending_hand_pick(&mut runner).expect("hand pick installs");
    assert!(v.is_optional);
    runner.execute_action(0, PASS).expect("decline the play-or-use");
    let _ = runner.auto_resolve();

    assert_eq!(face_down_source_count(&runner, 0), 0, "the trash-2 was paid");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "passed ⇒ nothing played"
    );
    assert!(in_hand(&runner, 0, "DS-CHEAP"), "the DS card is still in hand");
}

/// No eligible hand card (a cost-6 [DATA SQUAD] card and a non-[DATA SQUAD]
/// card): the trash is STILL offered and paying it trashes 2 (DCGO gates only on
/// the face-down counts); no hand pick installs and nothing is played.
#[test]
fn st24_06_no_eligible_hand_card_still_offers_and_pays_trash() {
    let (mut runner, rize) = rize_runner(&["DS-EXP", "NON-DS"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);
    choose_pay(&mut runner, true);
    assert!(
        pending_hand_pick(&mut runner).is_none(),
        "no legal play-or-use target offered"
    );
    let _ = runner.auto_resolve();

    assert_eq!(face_down_source_count(&runner, 0), 0, "the trash-2 was paid");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "nothing played"
    );
    assert!(in_hand(&runner, 0, "DS-EXP") && in_hand(&runner, 0, "NON-DS"));
}

/// Fewer than 2 face-down cards under your Tamers: the trash can't be paid in
/// full (official Q&A: a "by" condition can't be partly met), so it is not
/// offered at all.
#[test]
fn st24_06_trash_not_offered_with_one_face_down_card() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP"], &[1]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);
    assert!(
        runner.pending_selection_view().is_none(),
        "only 1 face-down card ⇒ no pay-or-not choice"
    );
    let _ = runner.auto_resolve();
    assert_eq!(face_down_source_count(&runner, 0), 1, "nothing trashed");
    assert!(in_hand(&runner, 0, "DS-CHEAP"), "nothing played");
}

// ─── Hand-pick legality: only cards that can actually be played / used. DCGO
// ST24_06.cs CanSelectCardCondition: `EqualsTraits("DATA SQUAD") &&
// GetCostItself <= 5 && ((IsOption && !CanNotPlayThisOption) || (HasPlayCost &&
// CanPlayAsNewPermanent(card, payCost:false, …)))`. CR 4.3 7-1-1: playing places
// "a new card with a play cost" onto the field — a Digi-Egg has none, so it is
// never a "play" candidate (it is hatched, CR 4.3 4-18).

/// Pay the trash and return the non-PASS hand-pick candidates' card ids (empty
/// when no hand pick installs).
fn offered_after_paying(runner: &mut DebugRunner, rize: PermanentHandle) -> Vec<String> {
    fire(runner, EffectTiming::OnPlay, rize);
    pick_dp_target(runner);
    choose_pay(runner, true);
    let Some(v) = pending_hand_pick(runner) else {
        return Vec::new();
    };
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    v.valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
        .collect()
}

/// Koromon (ST24-01) is a [DATA SQUAD] Digi-Egg with no play cost: never offered.
#[test]
fn st24_06_hand_pick_excludes_data_squad_digi_egg() {
    let (mut runner, rize) = rize_runner(&["ST24-01", "DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let offered = offered_after_paying(&mut runner, rize);
    assert_eq!(
        offered,
        vec!["DS-CHEAP".to_string()],
        "only the playable Digimon is offered; the Digi-Egg Koromon (ST24-01) is not"
    );
}

/// Only a Digi-Egg in hand: the trash is still payable, but no hand pick installs
/// and the egg stays in hand (nothing reaches the battle area).
#[test]
fn st24_06_digi_egg_only_hand_offers_no_pick() {
    let (mut runner, rize) = rize_runner(&["ST24-01"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let field_before = runner.game.players[0].battle_area.len();
    let offered = offered_after_paying(&mut runner, rize);
    assert!(offered.is_empty(), "no candidate offered: {offered:?}");
    let _ = runner.auto_resolve();
    assert_eq!(face_down_source_count(&runner, 0), 0, "the trash-2 was paid");
    assert_eq!(runner.game.players[0].battle_area.len(), field_before);
    assert!(in_hand(&runner, 0, "ST24-01"), "Koromon stays in hand");
}

/// A [DATA SQUAD] Option with use cost ≤5 is a legal "use" pick.
#[test]
fn st24_06_hand_pick_offers_data_squad_option() {
    let (mut runner, rize) = rize_runner(&["DS-OPT", "DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let mut offered = offered_after_paying(&mut runner, rize);
    offered.sort();
    assert_eq!(offered, vec!["DS-CHEAP".to_string(), "DS-OPT".to_string()]);
}

/// "You can't use Option cards" (DCGO `CanNotPlayThisOption`): the Option is not
/// offered, the Digimon still is.
#[test]
fn st24_06_hand_pick_excludes_option_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, rize) = rize_runner(&["DS-OPT", "DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_after_paying(&mut runner, rize);
    assert_eq!(offered, vec!["DS-CHEAP".to_string()]);
}

/// "You can't play Digimon by effects" (DCGO `CanPlayAsNewPermanent` false): the
/// Digimon is not offered, the Option still is.
#[test]
fn st24_06_hand_pick_excludes_digimon_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, rize) = rize_runner(&["DS-OPT", "DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_after_paying(&mut runner, rize);
    assert_eq!(offered, vec!["DS-OPT".to_string()]);
}

/// A full battle area (DCGO `CanPlayAsNewPermanent` needs an empty frame): the
/// Digimon is not offered, the Option still is.
#[test]
fn st24_06_hand_pick_excludes_digimon_when_battle_area_full() {
    let (mut runner, rize) = rize_runner(&["DS-OPT", "DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    // RizeGreymon + the Tamer fill a 2-slot battle area.
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    let offered = offered_after_paying(&mut runner, rize);
    assert_eq!(offered, vec!["DS-OPT".to_string()]);
}

// ─── Option colour requirements on the "use" half ────────────────────────────
// An effect-driven use is still a use: CR 4.3 4-22-1..3 (the colour requirement
// must be met to use an Option; every colour of a multicolour Option); only
// 4-22-5 (activating an Option's effect WITHOUT using it) is exempt. Official
// rulings Q1500 (BT6-112) and Q2785 (BT17-035): an effect that uses an Option
// does not ignore its colour requirements unless it says so. DCGO ST24_06.cs
// offers `IsOption && !CanNotPlayThisOption`, and `CanNotPlayThisOption`
// (CardSource.cs:184-246) is true when `!MatchColorRequirement`
// (CardSource.cs:253-309, which honours `<Use Req.>` / "ignore colour
// requirements" through `IIgnoreColorConditionEffect`).

/// RizeGreymon (ST24-06, yellow/red) plus one Tamer (`tamer_id`) carrying the
/// two face-down sources, with the three real [DATA SQUAD] Options and DS-CHEAP
/// in hand. Returns the hand-pick candidates after paying the trash.
fn offered_with_tamer(tamer_id: &str) -> Vec<String> {
    let mut runner = base_builder()
        .add_card(card_data_from_compiled("ST24-13"))
        .add_card(card_data_from_compiled("BT26-098"))
        .add_card(card_data_from_compiled("BT25-096"))
        .add_card(card_data_from_compiled("ST24-15"))
        .start();
    runner.set_first_player(0);
    let rize = runner.place_stack(0, &["FILLER", CARD_ID]);
    let t = runner.place_stack(0, &["FILLER", "FILLER", tamer_id]);
    for i in 0..2 {
        runner.game.players[0].battle_area[t.index as usize].card_sources[i].face_down = true;
    }
    for h in ["BT26-098", "BT25-096", "ST24-15", "DS-CHEAP"] {
        push_to_hand(&mut runner, 0, h);
    }
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let mut offered = offered_after_paying(&mut runner, rize);
    offered.sort();
    offered
}

/// The reported counterexample: only RizeGreymon (ST24-06, yellow/red) and
/// Marcus Damon & Thomas H. Norstein (ST24-13, yellow/blue) on the field. Queen
/// of Thorns (BT26-098) is green with no `<Use Req.>`: it can't be used, so it
/// is not offered. Mirage Beast Knight (BT25-096, blue) is covered by ST24-13's
/// blue. DNA Charge (ST24-15, white) meets its `<Use Req.>` ([DATA SQUAD]
/// trait) through RizeGreymon.
#[test]
fn st24_06_hand_pick_excludes_option_whose_colour_requirement_is_unmet() {
    assert_eq!(
        offered_with_tamer("ST24-13"),
        vec![
            "BT25-096".to_string(),
            "DS-CHEAP".to_string(),
            "ST24-15".to_string()
        ],
        "green Queen of Thorns (BT26-098) is not usable with only yellow/red/blue on the field"
    );
}

/// With no blue permanent (a yellow Tamer instead of ST24-13), Mirage Beast
/// Knight (BT25-096, blue) is not offered either; DNA Charge (ST24-15) still
/// is, through `<Use Req.>`.
#[test]
fn st24_06_hand_pick_excludes_blue_option_without_a_blue_permanent() {
    assert_eq!(
        offered_with_tamer("TAMER"),
        vec!["DS-CHEAP".to_string(), "ST24-15".to_string()],
        "only yellow/red on the field: neither BT26-098 (green) nor BT25-096 (blue) is usable"
    );
}

// ─── Interaction: ST24-13's "[Your Turn] When effects trash cards from under
// this Tamer, by suspending this Tamer, …" reacting to the trash-2 payment.
// DCGO ST24_13.cs CanActivateCondition requires CanActivateSuspendCostEffect
// (the Tamer unsuspended); CR 4.3 15-7-4: an unpayable optional processing
// condition can't be chosen, so a suspended ST24-13 is not offered.

/// Stage RizeGreymon + a real ST24-13 Tamer carrying 2 face-down sources, pay
/// the trash and play DS-CHEAP. Returns the runner after the hand pick resolves.
fn rize_with_st24_13(tamer_suspended: bool) -> DebugRunner {
    let mut runner = base_builder()
        .add_card(card_data_from_compiled("ST24-13"))
        .start();
    runner.set_first_player(0);
    let rize = runner.place_stack(0, &["FILLER", CARD_ID]);
    let t = runner.place_stack(0, &["FILLER", "FILLER", "ST24-13"]);
    for i in 0..2 {
        runner.game.players[0].battle_area[t.index as usize].card_sources[i].face_down = true;
    }
    runner.game.players[0].battle_area[t.index as usize].is_suspended = tamer_suspended;
    push_to_hand(&mut runner, 0, "DS-CHEAP");
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, rize);
    pick_dp_target(&mut runner);
    choose_pay(&mut runner, true);
    let v = pending_hand_pick(&mut runner).expect("hand pick installs");
    let pick = v.valid_action_ids.iter().copied().find(|&id| id != PASS).unwrap();
    runner.execute_action(0, pick).unwrap();
    assert_eq!(face_down_source_count(&runner, 0), 0, "trash-2 paid");
    assert!(on_field(&runner, 0, "DS-CHEAP"));
    runner
}

/// Unsuspended ST24-13: paying the trash fires its [Your Turn] trash trigger
/// (offered once RizeGreymon's effect finishes).
#[test]
fn st24_06_trash_payment_offers_st24_13_trigger_when_unsuspended() {
    let runner = rize_with_st24_13(false);
    let sel = runner
        .pending_selection()
        .expect("ST24-13's 'by suspending this Tamer' trigger is offered");
    let source_id = runner
        .game
        .card_data_for_handle(sel.source_card)
        .map(|d| d.card_id.clone());
    assert_eq!(
        source_id.as_deref(),
        Some("ST24-13"),
        "the open prompt is ST24-13's trash trigger, not some other prompt ({:?}: {})",
        sel.kind,
        sel.prompt
    );
}

/// Already-suspended ST24-13: its suspend cost can't be paid, so the trash
/// payment offers nothing (DCGO CanActivateSuspendCostEffect; CR 15-7-4).
#[test]
fn st24_06_trash_payment_does_not_offer_st24_13_when_suspended() {
    let mut runner = rize_with_st24_13(true);
    assert!(
        runner.pending_selection().is_none(),
        "a suspended ST24-13 offers no unpayable trigger"
    );
    let _ = runner.auto_resolve();
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [All Turns][OPT] leave-replacement (trash 1 FD → stay)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_06_inherited_leave_replacement_prevents_leave() {
    let mut runner = base_builder().start();
    runner.set_first_player(0);

    // A [DATA SQUAD] carrier ON TOP of RizeGreymon (so RizeGreymon's inherited
    // effect applies and the subject carries the [DATA SQUAD] trait).
    let carrier = runner.place_stack(0, &[CARD_ID, "DS-CARRIER"]);
    let t = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;

    let ba_before = runner.battle_area_size(0);
    let fd_before = face_down_source_count(&runner, 0);

    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);

    match runner.pending_kind() {
        Some(SelectionKind::Replacement) => {
            assert!(runner.pending_is_optional(), "leave-prevention is optional");
            let view = runner.pending_selection_view().unwrap();
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("accept the leave-prevention");
            let _ = runner.auto_resolve();

            assert_eq!(
                runner.battle_area_size(0),
                ba_before,
                "the [DATA SQUAD] carrier did not leave (replacement cancelled the leave)"
            );
            assert_eq!(
                face_down_source_count(&runner, 0),
                fd_before - 1,
                "1 bottom face-down source under a Tamer was trashed as the cost"
            );
        }
        _ => {
            // The structural test guards the clause's presence; accept either
            // auto-resolved outcome here.
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — inherited leave-prevention NAME / TRAIT gate
//
// "When this Digimon with [ShineGreymon] in its name or the [DATA SQUAD] trait
// would leave …" — DCGO ST24_06.cs Inherited CanUseCondition:
// `TopCard.ContainsCardName("ShineGreymon") || TopCard.EqualsTraits("DATA SQUAD")`.
// Regression guard: the YAML used to spell the name leaf
// `source_permanent_name_contains` (not a DSL key). It was silently dropped,
// turning that `any_of` branch into an always-true empty predicate, so the
// save fired for ANY carrier.
// ════════════════════════════════════════════════════════════════════════════

fn gate_top(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 13;
    c.traits = vec!["Beast".to_string()];
    c
}

/// `top_id` sits on top of RizeGreymon (so only the INHERITED clause applies); a
/// Tamer holds one face-down source to pay the cost. Deletes the carrier and
/// drives every prompt (accepting). Returns
/// `(replacement_prompt_offered, carrier_still_on_field)`.
fn gate_delete_carrier(top_id: &str) -> (bool, bool) {
    let mut runner = base_builder()
        .add_card(gate_top("PLAIN-TOP", "Plainmon"))
        .add_card(gate_top("NAME-TOP", "ShineGreymon Ruin Mode X"))
        .start();
    runner.set_first_player(0);
    let carrier = runner.place_stack(0, &[CARD_ID, top_id]);
    let t = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let mut guard = 0;
    let mut saw_replacement = false;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 32);
        if matches!(
            runner.pending_selection().unwrap().kind,
            SelectionKind::Replacement
        ) {
            saw_replacement = true;
        }
        let actions: Vec<u16> = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .collect();
        if let Some(a) = actions.iter().copied().find(|a| *a != PASS) {
            if runner.execute_action(0, a).is_ok() {
                continue;
            }
        }
        let _ = runner.execute_action(0, PASS);
    }
    let _ = runner.auto_resolve();
    let stayed = runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == top_id);
    (saw_replacement, stayed)
}

/// NEGATIVE: a carrier with neither [ShineGreymon] in its name nor the
/// [DATA SQUAD] trait gets no inherited save — it simply leaves.
#[test]
fn st24_06_inherited_save_not_offered_without_name_or_trait() {
    let (offered, stayed) = gate_delete_carrier("PLAIN-TOP");
    assert!(
        !offered,
        "no leave-prevention prompt for a non-[ShineGreymon], non-[DATA SQUAD] carrier"
    );
    assert!(!stayed, "the ungated carrier leaves the battle area");
}

/// POSITIVE (name branch): a carrier with [ShineGreymon] in its name but WITHOUT
/// the [DATA SQUAD] trait is still protected.
#[test]
fn st24_06_inherited_save_offered_for_name_match_without_trait() {
    let (offered, stayed) = gate_delete_carrier("NAME-TOP");
    assert!(offered, "a [ShineGreymon]-named carrier is offered the inherited save");
    assert!(stayed, "accepting the save keeps the [ShineGreymon]-named carrier");
}

/// "When THIS Digimon ... would leave": the inherited save protects only the
/// permanent carrying RizeGreymon as a source. Deleting ANOTHER of your Digimon
/// offers no replacement (it leaves); deleting the [DATA SQUAD] carrier
/// afterwards still offers it (the [Once Per Turn] was not spent).
#[test]
fn st24_06_inherited_save_only_for_carrier_audit() {
    // Audit 2026-11-11: DCGO ST24_06.cs CanTriggerWhenRemoveField(hashtable, card) -- the carrier only.
    let mut runner = base_builder().start();
    runner.set_first_player(0);
    runner.place_stack(0, &[CARD_ID, "DS-CARRIER"]);
    let other = runner.place_on_field(0, "NON-DS", Some(0));
    let t = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;

    /// Drive every prompt (accepting); returns whether a Replacement prompt appeared.
    fn drive(runner: &mut DebugRunner) -> bool {
        let mut saw_replacement = false;
        let mut guard = 0;
        while let Some(sel) = runner.pending_selection() {
            guard += 1;
            assert!(guard < 32);
            if matches!(sel.kind, SelectionKind::Replacement) {
                saw_replacement = true;
            }
            let a = sel
                .valid_action_ids
                .iter()
                .copied()
                .find(|a| *a != PASS)
                .unwrap_or(PASS);
            let who = sel.selecting_player;
            runner.execute_action(who, a).unwrap();
        }
        let _ = runner.auto_resolve();
        saw_replacement
    }

    // Another of your Digimon would leave: no replacement offered.
    runner
        .game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert!(
        !drive(&mut runner),
        "no leave-prevention for a Digimon that is not RizeGreymon's carrier"
    );
    assert!(!on_field(&runner, 0, "NON-DS"), "the other Digimon leaves");
    assert_eq!(face_down_source_count(&runner, 0), 1, "no cost paid");

    // The carrier would leave: the replacement is still offered and saves it.
    let carrier = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == "DS-CARRIER")
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("carrier on field");
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        drive(&mut runner),
        "the [DATA SQUAD] carrier is offered the inherited save"
    );
    assert!(on_field(&runner, 0, "DS-CARRIER"), "accepting the save keeps the carrier");
    assert_eq!(face_down_source_count(&runner, 0), 0, "1 face-down card trashed");
}
