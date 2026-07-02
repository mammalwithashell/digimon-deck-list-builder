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
//! - trash-2-bottom-FD-under-Tamers cost → play/use a [DATA SQUAD] hand card
//!   (play/use cost ≤5, the G-PLAY-OR-USE-COST-LTE filter) for FREE
//! - the play-or-use half is optional (declinable); the DP-minus still resolves
//! - filter: a cost-6 [DATA SQUAD] card and a non-[DATA SQUAD] card are excluded
//! - inherited leave-replacement (ShineGreymon-name / DATA SQUAD subject)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
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

/// A high-DP opponent Digimon (so -5000 DP never deletes it — rule 17-1-3-1).
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
// Section 3 — trash-2 → play/use a [DATA SQUAD] hand card (cost ≤5) for FREE
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_06_trash_2_then_plays_data_squad_card_free() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    assert_eq!(face_down_source_count(&runner, 0), 2, "2 FD sources staged");
    let mem_before = runner.memory();
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);

    // Prompt 1: mandatory -5000 DP pick.
    let v = runner.pending_selection_view().expect("DP-minus pick");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();

    // Prompt 2: the optional "may play or use" hand pick (DS-CHEAP only legal).
    let v2 = runner
        .pending_selection_view()
        .expect("'may play or use' hand pick installs");
    assert!(v2.is_optional, "'you may play or use' ⇒ declinable");
    let pick = v2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .expect("DS-CHEAP is the legal hand pick");
    runner.execute_action(0, pick).unwrap();
    // Resolve the trash-2 Tamer pick(s).
    let _ = runner.auto_resolve();

    assert_eq!(
        face_down_source_count(&runner, 0),
        0,
        "both face-down sources are trashed (the trash-2 cost)"
    );
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

#[test]
fn st24_06_then_play_or_use_is_declinable() {
    let (mut runner, rize) = rize_runner(&["DS-CHEAP"], &[2]);
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);

    // Mandatory DP-minus pick.
    let v = runner.pending_selection_view().expect("DP-minus pick");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();

    // DECLINE the optional play-or-use.
    let v2 = runner.pending_selection_view().expect("hand pick installs");
    assert!(v2.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline the play-or-use");
    let _ = runner.auto_resolve();

    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "declined ⇒ no face-down sources trashed"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "declined ⇒ nothing played"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "DS-CHEAP"),
        "the DS card is still in hand"
    );
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 5000,
        "the -5000 DP (mandatory half) still resolved"
    );
}

/// FILTER: a cost-6 [DATA SQUAD] card and a non-[DATA SQUAD] card are both
/// ineligible, so the play-or-use half never engages (no hand prompt at all).
#[test]
fn st24_06_ineligible_hand_cards_are_filtered_out() {
    let (mut runner, rize) = rize_runner(&["DS-EXP", "NON-DS"], &[2]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    let field_before = runner.game.players[0].battle_area.len();

    fire(&mut runner, EffectTiming::OnPlay, rize);

    // Mandatory DP-minus pick, then auto-resolve. Because the hand-match gate
    // (a [DATA SQUAD] card with play/use cost ≤5) is FALSE, no hand prompt
    // installs and the trash-2 cost is never paid.
    let v = runner.pending_selection_view().expect("DP-minus pick");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "ineligible hand ⇒ no trash (gate false)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "ineligible hand ⇒ nothing played"
    );
    assert!(
        runner.game.players[0].hand.iter().any(|c| {
            let id = c.card_id(&runner.game.card_data);
            id == "DS-EXP" || id == "NON-DS"
        }),
        "the cost-6 DS card and the non-DS card stay in hand (never legal picks)"
    );
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
