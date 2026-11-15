//! DCGO replay: a "do it?" bool in FRONT of a card's "From hand" / "From
//! trash" bool.
//!
//! ShineGreymon (ST24-07)'s [When Digivolving] [When Attacking] "You may play
//! 1 Tamer card with a play cost of 5 or less from your hand or trash" asks,
//! in DCGO, first "Play a Tamer" = true / "Don't Play" = false
//! (`SetBoolSelection`, ST24_07.cs:84-95), and only then - when BOTH the hand
//! and the trash hold a playable Tamer - "From hand" = true / "From trash" =
//! false (ST24_07.cs:98-111). With only one zone holding one, DCGO sets the
//! zone itself (`SetBool(canSelectHand)`, :113-115: `UserSelectionManager.
//! SetBool` sets the value locally, no `SetBoolForPlayer`, so no row). Then
//! the pick: a `SelectHandEffect` (:136-157) or a `SelectCardEffect` rooted in
//! the trash (:167-197). The engine asks ONE optional pick over the hand and
//! the trash (ST24-07.yaml `select_union_zone`).
//!
//! The replay read the FIRST bool ("Play a Tamer") as the zone menu's "From
//! hand", then met the real zone bool where it expected the pick:
//! raw-20261113T025510Z#g6 replay step 115 (raw seq 1014-1017).
//!
//! Staged with `DebugRunner` (real ST24-07 YAML, synthetic Tamers). No
//! recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c
}

/// P0's ShineGreymon (ST24-07) [When Digivolving] waits on its Tamer pick;
/// TAMER copies in the hand (`in_hand`) and/or the trash (`in_trash`). P1 has
/// one Digimon with 13000 DP for the -9000.
fn shinegreymon_digivolved(in_hand: bool, in_trash: bool) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card("ST24-07")
        .expect("ST24-07 production YAML")
        .add_card(ds_tamer("TAMER"))
        .add_card(digimon("BASE", 5, 7000))
        .add_card(digimon("OPP", 4, 13000))
        .deck(0, &["BASE"; 10])
        .deck(1, &["BASE"; 10])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let me = r.place_stack(0, &["BASE", "ST24-07"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    if in_hand {
        r.add_to_hand(0, "TAMER");
    }
    if in_trash {
        let i = r.add_to_hand(0, "TAMER");
        let card = r.game.players[0].hand.remove(i);
        r.game.players[0].trash.push(card);
    }
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    let p = r.pending_selection().expect("the Tamer pick waits");
    assert!(matches!(p.kind, SelectionKind::UnionZone { .. }) && p.is_optional, "{:?} {}", p.kind, p.prompt);
    (r, opp)
}

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
}

impl RecordingSource for StagedDcgo {
    fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Ok(self.game.clone())
    }
    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        *game = self.game.clone();
        Ok(())
    }
    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
    fn omits_dcgo_silent_prompts(&self) -> bool {
        true
    }
}

fn sel(json: serde_json::Value) -> StepSpec {
    let mut v = serde_json::json!({ "step": 0, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    let row: digimon_engine::dcgo_recording::SelectionRow =
        serde_json::from_value(v).expect("selection row");
    StepSpec {
        actor: row.actor,
        action_id: 0,
        phase: "Main".into(),
        source: "selection".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: Some(row),
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

fn bool_row(v: bool) -> StepSpec {
    sel(serde_json::json!({"prompt": "generic_bool", "bool_value": v}))
}

fn hand_pick() -> StepSpec {
    sel(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["TAMER"]}))
}

/// DCGO's trash pick: a `SelectCardEffect` with no zone (the 2026-11-13
/// recordings carry none).
fn trash_pick() -> StepSpec {
    sel(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["TAMER"]}))
}

fn minus_9000_pick() -> StepSpec {
    sel(serde_json::json!({
        "prompt": "SelectPermanentEffect",
        "targets": [{"player": 1, "frame": 0}],
        "board_p0": ["ST24-07"],
        "board_p1": ["OPP"],
    }))
}

fn tamers_on_field(game: &Game) -> usize {
    game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&game.card_data) == CardKind::Tamer)
        .count()
}

fn count(cards: &[digimon_engine::card_source::CardSource], game: &Game, id: &str) -> usize {
    cards.iter().filter(|c| c.card_id(&game.card_data) == id).count()
}

/// Both zones hold a Tamer: "Play a Tamer" (true), "From hand" (true), the
/// hand pick, the -9000 pick. The hand copy is played; the trash copy stays.
#[test]
fn play_then_from_hand_plays_the_hand_copy() {
    let (r, opp) = shinegreymon_digivolved(true, true);
    let mut s = staged(r.game, vec![bool_row(true), bool_row(true), hand_pick(), minus_9000_pick()]);
    replay_all(&mut s);
    assert_eq!(tamers_on_field(&s.game), 1);
    assert_eq!(count(&s.game.players[0].hand, &s.game, "TAMER"), 0);
    assert_eq!(count(&s.game.players[0].trash, &s.game, "TAMER"), 1);
    assert_eq!(s.game.effective_dp(opp), Some(4000));
}

/// Both zones: "Play a Tamer" (true), "From trash" (false), the trash pick.
/// The trash copy is played; the hand copy stays.
#[test]
fn play_then_from_trash_plays_the_trash_copy() {
    let (r, _) = shinegreymon_digivolved(true, true);
    let mut s = staged(r.game, vec![bool_row(true), bool_row(false), trash_pick(), minus_9000_pick()]);
    replay_all(&mut s);
    assert_eq!(tamers_on_field(&s.game), 1);
    assert_eq!(count(&s.game.players[0].hand, &s.game, "TAMER"), 1);
    assert_eq!(count(&s.game.players[0].trash, &s.game, "TAMER"), 0);
}

/// Only the hand holds a Tamer: DCGO sets the zone without a row, so
/// "Play a Tamer" (true) is followed straight by the hand pick.
#[test]
fn play_with_only_the_hand_holding_a_tamer_goes_straight_to_the_pick() {
    let (r, _) = shinegreymon_digivolved(true, false);
    let mut s = staged(r.game, vec![bool_row(true), hand_pick(), minus_9000_pick()]);
    replay_all(&mut s);
    assert_eq!(tamers_on_field(&s.game), 1);
}

/// "Don't Play" (false): no Tamer is played, the -9000 still resolves. (It
/// was read as "From trash" and halted for want of a trash pick.)
#[test]
fn dont_play_declines_the_pick() {
    let (r, opp) = shinegreymon_digivolved(true, true);
    let mut s = staged(r.game, vec![bool_row(false), minus_9000_pick()]);
    replay_all(&mut s);
    assert_eq!(tamers_on_field(&s.game), 0);
    assert_eq!(s.game.effective_dp(opp), Some(4000));
}

/// "Play a Tamer" followed by neither a zone bool nor a pick of the actor:
/// nothing grounds the rest, so the replay stops at the gate.
#[test]
fn play_followed_by_no_menu_or_pick_halts() {
    let (r, _) = shinegreymon_digivolved(true, true);
    let mut s = staged(r.game, vec![bool_row(true), minus_9000_pick()]);
    s.step();
    match s.divergences() {
        [d] => assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }), "{d:?}"),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.is_paused());
}
