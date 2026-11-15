//! DCGO replay: a token in a recorded board.
//!
//! DCGO builds every token from a bare `CEntity_Base` that never sets a
//! `CardID` (ContinuousController.cs:440-476: the Hinukamuy, Petrification and
//! Paishu tokens set colours, cost, level, names, kind, DP and effect class
//! only), so a token's `CardID` is empty and the recorder logs it as "" in
//! every `boards` list (and the token play itself as `cards: [""]`). The
//! replay maps DCGO's board positions onto engine slots by card identity
//! (`build_slot_map`); the engine names a token by its token id
//! (`TOKEN_PETRIFICATION`), so a "" never matched and a target on a token kept
//! DCGO's raw position - which, when DCGO's order differs from the engine's,
//! is ANOTHER permanent.
//!
//! Field witness: raw-20261113T011922Z#g3 raw seq 402, Medusamon (EX11-012)'s
//! [When Digivolving] deleting P0's Petrification Token at DCGO frame 0 of
//! `['', 'AD1-023', 'BT6-022']` while the engine's P0 board is
//! `[AD1-023, TOKEN_PETRIFICATION]`.
//!
//! Staged with `DebugRunner` (real EX11-012 YAML and token registry,
//! synthetic other cards). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::{DIGIVOLVE_START, FIELDS_PER_HAND};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
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

fn spec(action_id: u16, source: &str, selection: Option<SelectionRow>) -> StepSpec {
    StepSpec {
        actor: 0,
        action_id,
        phase: "Main".into(),
        source: source.into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection,
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

/// P0: a red level-5 Digimon on the field, Medusamon (EX11-012) in hand. P1:
/// a 5000 DP Digimon, then a Petrification Token (engine board
/// `[VICTIM, TOKEN_PETRIFICATION]`).
fn staged() -> Game {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 YAML loads")
        .add_card(digimon("DRAGON5", 5, 6000, &["Dragonkin"]))
        .add_card(digimon("VICTIM", 4, 5000, &[]))
        .add_card(digimon("MISS", 3, 1000, &[]))
        .hand(0, &["EX11-012"])
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "DRAGON5", None);
    runner.place_on_field(1, "VICTIM", None);
    EffectContext::new(&mut runner.game, CardHandle(0), None, 1).play_token(1, "petrification");
    let ids: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(ids, vec!["VICTIM", "TOKEN_PETRIFICATION"], "the staged engine board");
    runner.game
}

fn on_board(game: &Game, player: usize, id: &str) -> bool {
    game.players[player].battle_area.iter().any(|p| p.top_card().card_id(&game.card_data) == id)
}

/// DCGO lists the token first (`["", "VICTIM"]`) and Medusamon deletes frame
/// 0, the token. The replay deletes the token, not the engine's slot 0.
#[test]
fn a_dcgo_token_target_maps_onto_the_engine_token() {
    let digivolve = DIGIVOLVE_START + 0 * FIELDS_PER_HAND + 0;
    let pick: SelectionRow = serde_json::from_value(serde_json::json!({
        "step": 1, "actor": 0, "prompt": "SelectPermanentEffect",
        "board_p1": ["", "VICTIM"], "targets": [{"player": 1, "frame": 0}]
    }))
    .unwrap();
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo { game: staged(), steps: vec![spec(digivolve, "main_phase", None), spec(0, "selection", Some(pick))] }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    for _ in 0..6 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(!on_board(&s.game, 1, "TOKEN_PETRIFICATION"), "Medusamon deleted the token");
    assert!(on_board(&s.game, 1, "VICTIM"), "the engine's slot 0 was not deleted in its place");
}
