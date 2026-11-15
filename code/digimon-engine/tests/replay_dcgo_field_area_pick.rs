//! DCGO replay: one DCGO field pick over BOTH areas of the field, against the
//! engine's "which area?" choice followed by a per-area pick.
//!
//! Kota Domoto & Yuji Musya (BT20-087)'s [Your Turn] "1 of your Digimon on the
//! field may digivolve" (the field is the breeding area and the battle area,
//! CR 4.3 3-4-6) is ONE `SelectPermanentEffect` in DCGO over every own field
//! Digimon that has a hand result (BT20_087.cs:76-91, :114-127). DCGO's index
//! is `Owner.GetFieldPermanents().IndexOf(chara)` (SelectPermanentEffect.cs
//! :593-609); `GetFieldPermanents` walks the frames in FrameID order
//! (Player.cs:669-684) and the breeding frame is the LAST FrameID (Player.cs
//! :41-54), so the breeding Digimon, when there is one, is the last entry.
//! The engine asks the same decision in two steps: "Choose a Digimon's area to
//! digivolve" (only when both areas hold a host), then the pick in that area.
//! The recorded pick names the area, so it answers both.
//!
//! raw-20261113T025510Z#g6 replay step 76 (file step 80, raw seq 961-964)
//! stopped here: the card-less `MultipleSkills` index met no entry whose game
//! led to the next row's field pick, because BT20-087's led to the area choice.
//!
//! Every game is staged with `DebugRunner` (real BT20-087 / P-176 YAML,
//! synthetic Digimon). No recording content is used.

use std::collections::HashMap;

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const AREA_PROMPT: &str = "Choose a Digimon's area to digivolve";

fn digimon(id: &str, level: u8, chronicle: bool, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(8000);
    c.colors = vec![color];
    if chronicle {
        c.traits = vec!["Chronicle".into()];
    }
    c
}

/// A level-6 [Chronicle] result digivolving from a level-5 Digimon of `from`.
fn result(id: &str, from: CardColor) -> CardData {
    let mut c = digimon(id, 6, true, CardColor::Black);
    c.evo_costs = vec![EvoCost { card_color: from as u8, level: 5, memory_cost: 3 }];
    c
}

/// EVO2: a level-6 [Chronicle] result with TWO printed routes from a black
/// level-5 Digimon, cost 4 and cost 3.
fn two_route_result() -> CardData {
    let mut c = digimon("EVO2", 6, true, CardColor::Black);
    c.evo_costs = vec![
        EvoCost { card_color: CardColor::Black as u8, level: 5, memory_cost: 4 },
        EvoCost { card_color: CardColor::Black as u8, level: 5, memory_cost: 3 },
    ];
    c
}

/// P0's main phase: BT20-087 (slot 0), a [Chronicle] ATTACKER (slot 1, blue,
/// optionally with Dorimon (P-176) under it), BASE (slot 2, black) and a
/// black BASE in the breeding area. EVO6 (from black level 5) can digivolve
/// onto either BASE; EVOB (from blue level 5) only onto ATTACKER.
fn board(hand: &[&str], dorimon_under_attacker: bool) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-087")
        .expect("BT20-087 production YAML")
        .dsl_card("P-176")
        .expect("P-176 production YAML")
        .add_card(digimon("ATTACKER", 5, true, CardColor::Blue))
        .add_card(digimon("BASE", 5, false, CardColor::Black))
        .add_card(digimon("OTHER", 5, false, CardColor::Blue))
        .add_card(result("EVO6", CardColor::Black))
        .add_card(result("EVOB", CardColor::Blue))
        .add_card(two_route_result())
        .hand(0, hand)
        .deck(0, &["OTHER"; 30])
        .deck(1, &["OTHER"; 30])
        .security(0, &["OTHER"; 3])
        .security(1, &["OTHER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "BT20-087", Some(0));
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    if dorimon_under_attacker {
        r.push_source(attacker, "P-176");
    }
    r.place_on_field(0, "BASE", Some(0));
    r.place_in_breeding(0, "BASE");
    (r, attacker)
}

/// Fire BT20-087's attack trigger alone: its optional gate is parked.
fn fire_tamer_alone(r: &mut DebugRunner, attacker: PermanentHandle) {
    let card = r.top_card(attacker);
    r.game.enqueue_triggered(
        EffectTiming::OnAllyAttack,
        TriggerSource::PlayerBattleAreaAttack { player: 0, attacker, card },
    );
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_some(), "BT20-087's gate is parked");
}

/// DCGO's field board for P0: battle area in engine order, then `last` (the
/// breeding Digimon when DCGO and the engine agree).
fn p0_board(last: &str) -> Vec<&str> {
    vec!["BT20-087", "ATTACKER", "BASE", last]
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

fn halted_at(s: &ReplaySession, at: u32) -> String {
    let reason = match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            match &d.kind {
                DivergenceKind::SelectionResolution { reason } => reason.clone(),
                other => panic!("expected a selection-resolution divergence, got {other:?}"),
            }
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    };
    assert!(s.is_paused(), "the divergence halts the replay");
    reason
}

fn yes() -> StepSpec {
    sel(serde_json::json!({"prompt": "generic_bool", "bool_value": true, "unstamped": true}))
}

fn field_pick(frame: i32, last: &str) -> StepSpec {
    sel(serde_json::json!({
        "prompt": "SelectPermanentEffect",
        "targets": [{"player": 0, "frame": frame}],
        "board_p0": p0_board(last),
        "board_p1": [],
    }))
}

fn hand_pick(id: &str) -> StepSpec {
    sel(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": [id]}))
}

fn battle_top(game: &Game, slot: usize) -> String {
    game.players[0].battle_area[slot].top_card().card_id(&game.card_data).to_string()
}

fn breeding_top(game: &Game) -> String {
    game.players[0].breeding_area.as_ref().unwrap().top_card().card_id(&game.card_data).to_string()
}

/// The pick names battle-area frame 2: the engine's area choice is "battle
/// area", then the pick takes slot 2 and EVO6 digivolves onto it.
#[test]
fn a_battle_area_field_pick_answers_the_area_choice_and_the_pick() {
    let (mut r, attacker) = board(&["EVO6"], false);
    fire_tamer_alone(&mut r, attacker);
    let mut s = staged(r.game, vec![yes(), field_pick(2, "BASE"), hand_pick("EVO6")]);
    replay_all(&mut s);
    assert_eq!(battle_top(&s.game, 2), "EVO6");
    assert_eq!(breeding_top(&s.game), "BASE");
}

/// The pick names DCGO's LAST field entry, the breeding Digimon: the area
/// choice is "breeding area" and the breeding pick takes it.
#[test]
fn a_breeding_area_field_pick_answers_the_area_choice_and_the_breeding_pick() {
    let (mut r, attacker) = board(&["EVO6"], false);
    fire_tamer_alone(&mut r, attacker);
    let mut s = staged(r.game, vec![yes(), field_pick(3, "BASE"), hand_pick("EVO6")]);
    replay_all(&mut s);
    assert_eq!(breeding_top(&s.game), "EVO6");
    assert_eq!(battle_top(&s.game, 2), "BASE");
}

/// DCGO's last field entry is not the engine's breeding Digimon: nothing
/// proves the frame is the breeding area, so the replay stops at the pick
/// with the area choice still parked (no branch's pick accepts the row).
#[test]
fn a_last_field_entry_that_is_not_the_engines_breeding_digimon_halts() {
    let (mut r, attacker) = board(&["EVO6"], false);
    fire_tamer_alone(&mut r, attacker);
    let mut s = staged(r.game, vec![yes(), field_pick(3, "OTHER"), hand_pick("EVO6")]);
    s.step();
    s.step();
    halted_at(&s, 1);
    assert_eq!(s.game.pending_selection.as_ref().map(|p| p.prompt.as_str()), Some(AREA_PROMPT));
    assert_eq!(breeding_top(&s.game), "BASE");
}

/// A breeding pick row with no DCGO board for the owner proves nothing about
/// which entry the frame is: it halts.
#[test]
fn a_breeding_frame_without_a_dcgo_board_halts() {
    let (mut r, attacker) = board(&["EVO6"], false);
    fire_tamer_alone(&mut r, attacker);
    let no_board = sel(serde_json::json!({
        "prompt": "SelectPermanentEffect",
        "targets": [{"player": 0, "frame": 3}],
    }));
    let mut s = staged(r.game, vec![yes(), no_board, hand_pick("EVO6")]);
    s.step();
    s.step();
    halted_at(&s, 1);
    assert_eq!(breeding_top(&s.game), "BASE");
}

/// raw-20261113T025510Z#g6 in miniature: ATTACKER (with Dorimon (P-176)
/// under it) attacks, so P-176's [When Attacking] and BT20-087's trigger
/// both wait. DCGO's card-less `MultipleSkills` index is followed by "use
/// it?" yes and a FIELD pick: only BT20-087's effect picks a Digimon (P-176's
/// digivolves "this Digimon" from the hand), so the index is BT20-087's.
#[test]
fn a_cardless_order_index_is_proved_by_the_area_spanning_field_pick() {
    let (mut r, attacker) = board(&["EVO6", "EVOB"], true);
    r.attack_player(attacker, 1, false);
    let p = r.pending_selection().expect("trigger order parked");
    assert_eq!(p.kind, SelectionKind::TriggerOrder, "{}", p.prompt);
    let mut s = staged(
        r.game,
        vec![
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 1})),
            yes(),
            field_pick(2, "BASE"),
            hand_pick("EVO6"),
        ],
    );
    replay_all(&mut s);
    assert_eq!(battle_top(&s.game, 2), "EVO6");
}

// ── An effect digivolve's "Which digivolution cost do you pay?" ─────────────
//
// DCGO's `SelectCountEffect` candidates are the card's PRINTED digivolution
// costs onto the host (`card.CostList` = `EvoCosts`, CardSource.cs:617-627;
// CardController.cs:648-667, minus `_reducedCost`, which only BT16-024 sets).
// An effect's "cost reduced by N" (BT20-087's -1, Koromon (ST24-01)'s -2) is
// a cost-change effect applied when the chosen base cost is PAID
// (`DigivolveIntoHandOrTrashCard` -> `ChangeDigivolutionCostPlayerEffect`,
// CardEffectCommons.cs:891-918), so DCGO's count is the printed cost. The
// engine's branch labels show the REDUCED cost ("Digivolve for cost 2" for a
// printed 3 under -1): matching the count against a label picks the wrong
// route whenever a reduced cost equals another route's printed cost.
// raw-20261113T025510Z#g6 raw seq 966 (count 3: Hisyaryumon (BT20-015)'s
// printed-3 alt route under BT20-087's -1) was replayed as the printed-4 route,
// one memory too many, with no divergence; seq 1012 (count 3 under ST24-01's
// -2, engine labels 1 and 2) stopped the replay.

fn count_row(c: i32) -> StepSpec {
    sel(serde_json::json!({"prompt": "SelectCountEffect", "count": c}))
}

/// Count 3 names EVO2's printed-3 route: paid 3 - 1 = 2, not the printed-4
/// route whose reduced label reads "cost 3".
#[test]
fn an_effect_digivolve_cost_count_is_the_printed_cost() {
    let (mut r, attacker) = board(&["EVO2"], false);
    fire_tamer_alone(&mut r, attacker);
    let memory = r.game.memory;
    let mut s = staged(r.game, vec![yes(), field_pick(2, "BASE"), hand_pick("EVO2"), count_row(3)]);
    replay_all(&mut s);
    assert_eq!(battle_top(&s.game, 2), "EVO2");
    assert_eq!(s.game.memory, memory - 2, "printed 3, reduced by 1");
}

/// Count 4 names the printed-4 route: paid 3. (Its reduced label "cost 3"
/// does not name it; neither reading may pick the other route.)
#[test]
fn an_effect_digivolve_cost_count_names_the_other_printed_route() {
    let (mut r, attacker) = board(&["EVO2"], false);
    fire_tamer_alone(&mut r, attacker);
    let memory = r.game.memory;
    let mut s = staged(r.game, vec![yes(), field_pick(2, "BASE"), hand_pick("EVO2"), count_row(4)]);
    replay_all(&mut s);
    assert_eq!(s.game.memory, memory - 3, "printed 4, reduced by 1");
}

/// A count that is no printed route of the card (2 is only a REDUCED cost
/// here) halts instead of being read as a label.
#[test]
fn an_effect_digivolve_cost_count_that_is_no_printed_route_halts() {
    let (mut r, attacker) = board(&["EVO2"], false);
    fire_tamer_alone(&mut r, attacker);
    let mut s = staged(r.game, vec![yes(), field_pick(2, "BASE"), hand_pick("EVO2"), count_row(2)]);
    for _ in 0..4 {
        s.step();
    }
    let reason = halted_at(&s, 3);
    assert!(reason.contains("printed"), "{reason}");
}
