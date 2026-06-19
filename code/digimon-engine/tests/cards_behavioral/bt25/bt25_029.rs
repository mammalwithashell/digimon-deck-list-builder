//! BT25-029 MirageGaogamon — Digimon, Lv.6, Blue/Purple, DP 12000, Cost 12.
//! Traits: Beast Knight, DATA SQUAD. Attribute: Data.
//!
//! # Card text (data/cards.json — verbatim, confirmed vs DCGO BT25_029.cs)
//! <Reboot> <Blocker> <Evade>
//! [When Digivolving] [When Attacking] [Once Per Turn] You may return 1 of your
//!   opponent's level 5 or lower Digimon to the hand. Then, by trashing the
//!   bottom face-down card under any of your Tamers, return 1 of your opponent's
//!   lowest level Digimon to the hand.
//! [All Turns] [Once Per Turn] When effects add cards to your opponent's hand or
//!   trash cards from under your Tamers, this Digimon may unsuspend.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_029.cs
//!
//! # Patterns this test covers
//! - Reboot / Blocker / Evade keyword grants
//! - WD/WA: may bounce opp Lv≤5; then INDEPENDENTLY-optional trash-FD →
//!   (tail, mandatory) bounce opp lowest-level (level_matches_aggregate)
//! - the trash is declinable apart from the first bounce (G-OPTIONAL-TRASH-…)
//! - G-SHARED-OPT-HETEROGENEOUS-TIMING: one [All Turns][OPT] clause whose two
//!   timings (on_add_to_hand + on_digivolution_card_trashed) share one OPT
//!   counter while each carries its own per-timing condition

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-029";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn mirage() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn opp_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(3000 + level as i32 * 1000);
    c.play_cost = level as u16;
    c
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn seed_tamer_with_face_down(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    let tamer = runner.place_stack(player, &["FD-SRC", "TAMER"]);
    runner.game.players[player as usize].battle_area[tamer.index as usize].card_sources[0]
        .face_down = true;
    tamer
}

fn fire_wd(r: &mut DebugRunner, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

fn fire_effect_add_to_hand(r: &mut DebugRunner, player: PlayerId) {
    r.game.enqueue_triggered(
        EffectTiming::OnAddToHand,
        TriggerSource::HandGained {
            player,
            effect_initiated: true,
        },
    );
    r.game.drain_effect_queue();
}

fn in_hand(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn face_down_count(r: &DebugRunner, p: u8) -> usize {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&r.game.card_data) == CardKind::Tamer)
        .map(|perm| perm.card_sources.iter().filter(|s| s.face_down).count())
        .sum()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_029_metadata_keywords_and_alt_path() {
    let card = compiled(CARD_ID);
    assert_eq!(card.name, "MirageGaogamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));

    let kws: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(format!("{keyword:?}")),
            _ => None,
        })
        .collect();
    for want in ["Reboot", "Blocker", "Evade"] {
        assert!(kws.iter().any(|k| k.contains(want)), "<{want}> grant present");
    }

    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve)),
        "Gaogamon-name / DATA SQUAD Lv.5 alt-digivolve present"
    );
}

#[test]
fn bt25_029_shared_opt_all_turns_clause_has_two_timing_conditions() {
    let card = compiled(CARD_ID);
    let shared = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnAddToHand)
                && t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed)
        })
        .expect("the shared [All Turns][OPT] clause lists both timings");
    assert!(shared.once_per_turn, "shared OPT counter");
    assert!(shared.optional, "'may unsuspend' ⇒ optional");
    assert_eq!(shared.timing_conditions.len(), 2, "one gate per timing");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [WD][WA] bounce Lv≤5; INDEPENDENTLY-optional trash → lowest bounce
// ════════════════════════════════════════════════════════════════════════════

fn wd_runner() -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(mirage())
        .add_card(opp_digimon("OPP-LOW", 3)) // Lv.3 — ≤5 AND the lowest
        .add_card(opp_digimon("OPP-HIGH", 7)) // Lv.7 — not ≤5
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("FD-SRC"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));
    seed_tamer_with_face_down(&mut runner, 0);
    let mir = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, mir)
}

/// Engage the first bounce (the Lv≤5 OPP-LOW) but DECLINE the trash → no
/// lowest-level bounce, face-down intact.
#[test]
fn bt25_029_wd_first_bounce_then_declines_trash() {
    let (mut runner, mir) = wd_runner();
    fire_wd(&mut runner, mir);

    // Prompt 1: optional bounce of a Lv≤5 Digimon — only OPP-LOW (Lv3) is legal.
    let v = runner.pending_selection_view().expect("first bounce prompt");
    assert!(v.is_optional, "the first bounce is 'you may' ⇒ optional");
    let pick = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .expect("OPP-LOW is a legal Lv≤5 bounce target");
    runner.execute_action(0, pick).unwrap();
    assert!(in_hand(&runner, 1, "OPP-LOW"), "OPP-LOW returned to hand");

    // Prompt 2: the optional trash — DECLINE it.
    let v2 = runner.pending_selection_view().expect("optional trash prompt");
    assert!(v2.is_optional, "the trash+lowest-bounce is declinable");
    runner.execute_action(0, PASS).expect("decline the trash");
    let _ = runner.auto_resolve();

    assert_eq!(face_down_count(&runner, 0), 1, "declined ⇒ no FD trashed");
    assert!(
        !in_hand(&runner, 1, "OPP-HIGH"),
        "declined trash ⇒ no lowest-level bounce (OPP-HIGH stays on field)"
    );
}

/// Decline the first bounce, ENGAGE the trash → the mandatory lowest-level
/// bounce returns the opponent's lowest-level Digimon (OPP-LOW, Lv3).
#[test]
fn bt25_029_wd_trash_then_bounces_lowest_level() {
    let (mut runner, mir) = wd_runner();
    fire_wd(&mut runner, mir);

    // Prompt 1: DECLINE the optional first bounce.
    let v = runner.pending_selection_view().expect("first bounce prompt");
    assert!(v.is_optional);
    runner.execute_action(0, PASS).expect("decline first bounce");

    // Prompt 2: ENGAGE the optional trash (pick the Tamer).
    let v2 = runner.pending_selection_view().expect("optional trash prompt");
    let tamer_pick = v2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .expect("a Tamer with a face-down source is pickable");
    runner.execute_action(0, tamer_pick).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(face_down_count(&runner, 0), 0, "engaged ⇒ the FD source trashed");
    assert!(
        in_hand(&runner, 1, "OPP-LOW"),
        "the lowest-level opp Digimon (OPP-LOW, Lv3) was returned to hand"
    );
    assert!(
        !in_hand(&runner, 1, "OPP-HIGH"),
        "the higher-level Digimon (OPP-HIGH, Lv7) is NOT the lowest ⇒ stays"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns][OPT] may unsuspend (heterogeneous shared OPT)
// ════════════════════════════════════════════════════════════════════════════

fn at_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(mirage())
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("FD-SRC"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .memory(10)
        .start();
    runner.set_first_player(0);
    let mir = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[mir.index as usize].is_suspended = true;
    let tamer = seed_tamer_with_face_down(&mut runner, 0);
    (runner, mir, tamer)
}

#[test]
fn bt25_029_unsuspends_on_opponent_hand_gain_by_effect() {
    let (mut runner, mir, _t) = at_runner();
    fire_effect_add_to_hand(&mut runner, 1);
    assert!(runner.pending_selection().is_some(), "optional unsuspend prompt installs");
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.players[0].battle_area[mir.index as usize].is_suspended,
        "may unsuspend after opp hand gained cards by effect"
    );
}

#[test]
fn bt25_029_does_not_unsuspend_on_own_hand_gain() {
    let (mut runner, mir, _t) = at_runner();
    fire_effect_add_to_hand(&mut runner, 0);
    assert!(
        runner.pending_selection().is_none(),
        "own-hand gain must NOT fire the opponent-hand-gated clause"
    );
    assert!(
        runner.game.players[0].battle_area[mir.index as usize].is_suspended,
        "stays suspended (own-hand gain does not trigger)"
    );
}

#[test]
fn bt25_029_unsuspends_on_own_tamer_source_trashed() {
    let (mut runner, mir, tamer) = at_runner();
    runner.trash_one_source(tamer);
    assert!(runner.pending_selection().is_some(), "optional unsuspend prompt installs");
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.players[0].battle_area[mir.index as usize].is_suspended,
        "may unsuspend after a source under an own Tamer is trashed"
    );
}

#[test]
fn bt25_029_shared_opt_fires_once_across_both_timings() {
    let (mut runner, mir, tamer) = at_runner();

    fire_effect_add_to_hand(&mut runner, 1);
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.players[0].battle_area[mir.index as usize].is_suspended,
        "unsuspended on the first fire"
    );

    runner.game.players[0].battle_area[mir.index as usize].is_suspended = true;
    runner.trash_one_source(tamer);
    assert!(
        runner.pending_selection().is_none(),
        "the shared OPT was consumed by the first timing → no second prompt this turn"
    );
    assert!(
        runner.game.players[0].battle_area[mir.index as usize].is_suspended,
        "stays suspended (shared once-per-turn already used)"
    );
}
