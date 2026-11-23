//! DCGO replay: an action's own-board operand naming one of two copies of a card.
//!
//! The recorded boards name each permanent by its card id only, in DCGO's
//! frame order (`GetFieldPermanents()`, Player.cs:41-54 / :669-684), while the
//! engine's battle area is in play order. With two copies of one card on a
//! board the id cannot tell them apart, and the occurrence-order slot map
//! (`build_slot_map`) pairs DCGO's first copy with the engine's first - which
//! need not be the same permanent. A selection row's target can name its copy
//! (`FrameTarget::card_index`, `duplicate_copy_target`); an ACTION's own-board
//! operand (the attacker of an attack, the digivolve target, the host of an
//! activated permanent effect) could not, and `resolve_duplicate_attacker`
//! only rescues an attack the occurrence pairing makes ILLEGAL.
//!
//! Field witness: raw-20261115T153822Z#g10 raw seq 821. P1 held two
//! Tyrannomon-line ST24-05 copies; DCGO's attacker (board position 0) was the
//! newer one, which carried BT26-036 and whose When Attacking effect DCGO
//! triggered. Both copies could legally attack, the occurrence pairing sent
//! the older copy, and the effect DCGO went on to resolve never fired. The
//! recorder 0.2.0 row logs `board_indexes` (the top card's CardIndex per
//! board entry), and the converter writes the attacker's index as
//! `operand_card_index` (docs/DCGO_RECORDING_SCHEMA.md).
//!
//! Staged with `DebugRunner` (synthetic cards). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::dcgo_recording::CardIndexSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};

/// DCGO's CardIndex of the hand card the rows digivolve (any value).
const DIGIVOLVED: i32 = 57;

fn lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 0 }];
    c
}

/// P0's main phase. Battle area: [OLD (a Lv.4 that has a source under it,
/// in play since an earlier turn), BASE (a Lv.3)]. Hand: [NEW, the same
/// card as OLD]. The rows digivolve NEW onto BASE (slot 1), so the engine
/// then holds two `COPY` that differ (one has a source), in slots 0 and 1,
/// and both can attack (a digivolve does not reset the turn played).
fn board() -> Game {
    board_with_defender_test_setup(false)
}

fn board_with_defender_test_setup(defender_test: bool) -> Game {
    let mut attacker = make_test_card("ATTACKER", "ATTACKER");
    attacker.dp = Some(10000);
    let mut r = DebugRunner::builder()
        .add_card(lv4("COPY"))
        .add_card(attacker)
        .add_card(make_test_card("BASE", "BASE"))
        .add_card(make_test_card("MISS", "MISS"))
        .deck(0, &["MISS"; 10])
        .deck(1, &["MISS"; 10])
        .security(0, &["MISS"; 3])
        .security(1, &["MISS"; 3])
        .memory(8)
        .start();
    r.set_first_player(0);
    let old = r.place_on_field(0, "COPY", Some(0));
    r.push_source(old, "MISS");
    r.place_on_field(0, "BASE", Some(0));
    if defender_test {
        r.place_on_field(1, "ATTACKER", Some(0));
        for p in &mut r.game.players[0].battle_area {
            p.is_suspended = true;
        }
    }
    r.add_to_hand(0, "COPY");
    r.game.enter_main_phase();
    r.game
}

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
    /// DCGO card index of each action row's played card, by row.
    action_indexes: Vec<Option<i32>>,
    /// DCGO card index of each action row's own-board operand, by row.
    operand_indexes: Vec<Option<(i32, Option<CardIndexSource>)>>,
    attack_target_index: Option<i32>,
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
    fn dcgo_action_card_index(&self, row: u32) -> Option<i32> {
        self.action_indexes.get(row as usize).copied().flatten()
    }
    fn dcgo_action_operand_index(&self, row: u32) -> Option<(i32, Option<CardIndexSource>)> {
        self.operand_indexes.get(row as usize).copied().flatten()
    }
    fn dcgo_attack_target_index(&self, row: u32) -> Option<i32> {
        (row as usize == self.steps.len() - 1).then_some(self.attack_target_index).flatten()
    }
}

fn spec(action_id: u16, card_id: Option<&str>, boards: Option<(&[&str], &[&str])>) -> StepSpec {
    StepSpec {
        actor: 0,
        action_id,
        phase: "Main".into(),
        source: "main_phase".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: None,
        board_p0: boards.map(|(a, _)| a.iter().map(|s| s.to_string()).collect()),
        board_p1: boards.map(|(_, b)| b.iter().map(|s| s.to_string()).collect()),
        card_id: card_id.map(str::to_string),
        dna_materials: None,
        inferred: false,
    }
}

/// Digivolve the hand COPY onto BASE, then attack the player with DCGO's
/// board position 0 - DCGO lists the NEWER copy first, so the position
/// pairs with the engine's older copy in slot 0 by occurrence order.
fn session(operand: Option<(i32, Option<CardIndexSource>)>) -> ReplaySession {
    let game = board();
    let digivolve = spec(encode_digivolve(0, 1), Some("COPY"), Some((&["BASE", "COPY"], &[])));
    let attack = spec(encode_attack(0, SECURITY_TARGET), None, Some((&["COPY", "COPY"], &[])));
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo {
            game,
            steps: vec![digivolve, attack],
            action_indexes: vec![Some(DIGIVOLVED), None],
            operand_indexes: vec![None, operand],
            attack_target_index: None,
        }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    for _ in 0..4 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    s
}

/// (older copy, newer copy) suspended after the attack. The older copy holds
/// a MISS under it; the newer one is the digivolved BASE.
fn suspended(game: &Game) -> (bool, bool) {
    let area = &game.players[0].battle_area;
    let under = |p: &digimon_engine::permanent::Permanent| p.card_sources[0].card_id(&game.card_data).to_string();
    let old = area.iter().find(|p| under(p) == "MISS").expect("older copy");
    let new = area.iter().find(|p| under(p) == "BASE").expect("newer copy");
    (old.is_suspended, new.is_suspended)
}

/// The attack row names the newer copy by DCGO identity: that copy attacks,
/// not the older copy occurrence order would pick.
#[test]
fn the_attacker_is_the_copy_its_operand_card_index_names() {
    let s = session(Some((DIGIVOLVED, Some(CardIndexSource::Recorder))));
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(suspended(&s.game), (false, true), "the newer copy attacked");
}

/// Without the index the copies are told apart only by position: the replay
/// still pairs them in occurrence order (the older copy attacks) but says so,
/// because the two copies are different permanents and both could attack.
#[test]
fn without_an_operand_index_the_occurrence_pairing_is_noted() {
    let s = session(None);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(suspended(&s.game), (true, false), "occurrence order picked the older copy");
    assert!(
        s.replay_notes().iter().any(|n| n.contains("occurrence order") && n.contains("COPY")),
        "{:?}",
        s.replay_notes()
    );
}

/// An index the replay never saw move names no engine card: the replay falls
/// back to occurrence order and says the index named nothing.
#[test]
fn an_unpaired_operand_index_falls_back_with_a_note() {
    let s = session(Some((12345, Some(CardIndexSource::Recorder))));
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(suspended(&s.game), (true, false));
    assert!(s.replay_notes().iter().any(|n| n.contains("12345")), "{:?}", s.replay_notes());
}

/// The identity names a copy that cannot attack: the replay reports that
/// copy's illegal attack instead of quietly substituting the sibling.
#[test]
fn a_named_copy_that_cannot_attack_is_a_divergence_not_a_substitution() {
    let mut game = board();
    // The newer copy (BASE in slot 1) is suspended already: it cannot attack.
    game.players[0].battle_area[1].is_suspended = true;
    let digivolve = spec(encode_digivolve(0, 1), Some("COPY"), Some((&["BASE", "COPY"], &[])));
    let attack = spec(encode_attack(0, SECURITY_TARGET), None, Some((&["COPY", "COPY"], &[])));
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo {
            game,
            steps: vec![digivolve, attack],
            action_indexes: vec![Some(DIGIVOLVED), None],
            operand_indexes: vec![None, Some((DIGIVOLVED, Some(CardIndexSource::Recorder)))],
            attack_target_index: None,
        }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    for _ in 0..4 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.is_paused(), "the replay stops: {:?}", s.divergences());
    assert!(!s.game.players[0].battle_area[0].is_suspended, "the older copy was not substituted");
}

fn defender_session(target_index: Option<i32>) -> ReplaySession {
    let digivolve = spec(encode_digivolve(0, 1), Some("COPY"), Some((&["COPY", "BASE"], &["ATTACKER"])));
    let pass = spec(PASS, None, None);
    let mut attack = spec(encode_attack(0, 0), None, Some((&["COPY", "COPY"], &["ATTACKER"])));
    attack.actor = 1;
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo {
            game: board_with_defender_test_setup(true),
            steps: vec![digivolve, pass, attack],
            action_indexes: vec![Some(DIGIVOLVED), None, None],
            operand_indexes: vec![],
            attack_target_index: target_index,
        }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    s.step();
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s
}

#[test]
fn the_attack_hits_the_recorded_copy_not_the_copy_in_occurrence_order() {
    let mut s = defender_session(Some(DIGIVOLVED));
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    let [survivor] = s.game.players[0].battle_area.as_slice() else {
        panic!("exactly one copy should survive");
    };
    assert_eq!(survivor.card_sources[0].card_id(&s.game.card_data), "MISS");
}

#[test]
fn an_unpaired_defender_index_stops_without_attacking_either_copy() {
    let mut s = defender_session(Some(12345));
    s.step();
    assert!(s.is_paused(), "the replay must not substitute a defender");
    assert!(format!("{:?}", s.divergences()).contains("12345"));
    assert_eq!(s.game.players[0].battle_area.len(), 2);
}

#[test]
fn a_missing_named_defender_is_not_replaced_by_the_only_engine_copy() {
    let mut s = defender_session(Some(DIGIVOLVED));
    s.game.players[0].battle_area.remove(1);
    s.step();
    assert!(s.is_paused(), "the replay must not substitute a defender");
    assert_eq!(s.game.players[0].battle_area.len(), 1);
}
