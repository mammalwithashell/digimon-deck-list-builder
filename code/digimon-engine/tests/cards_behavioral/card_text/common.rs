use digimon_engine::action::space::PASS;
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

pub fn digimon(id: &str, level: u8, dp: i32, glowing: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![
        CardColor::Green,
        CardColor::Yellow,
        CardColor::Black,
        CardColor::Purple,
    ];
    if glowing {
        c.traits = vec!["Glowing Dawn".into(), "BEATBREAK".into()];
    }
    c.evo_costs = [2, 3, 5, 6]
        .into_iter()
        .map(|card_color| EvoCost {
            card_color,
            level: level.saturating_sub(1),
            memory_cost: 4,
        })
        .collect();
    c
}

pub fn builder(card: &str) -> DebugRunnerBuilder {
    let mut tamer = make_test_card("TEXT-TAMER", "Fixture Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.traits = vec!["Glowing Dawn".into(), "BEATBREAK".into()];
    DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(tamer)
        .add_card(digimon("TEXT-3", 3, 2000, true))
        .add_card(digimon("TEXT-4", 4, 4000, true))
        .add_card(digimon("TEXT-5", 5, 7000, true))
        .add_card(digimon("TEXT-6", 6, 12000, true))
        .add_card(digimon("TEXT-OTHER", 5, 7000, false))
        .add_card(make_test_card("TEXT-BOTTOM", "Bottom face-down card"))
        .add_card(make_test_card("TEXT-TOP", "Top face-down card"))
        .add_card(make_test_card("TEXT-FILLER", "Filler"))
        .deck(0, &["TEXT-FILLER"; 30])
        .deck(1, &["TEXT-FILLER"; 30])
        .security(0, &["TEXT-FILLER"; 5])
        .security(1, &["TEXT-FILLER"; 5])
        .memory(10)
}

pub fn start(builder: DebugRunnerBuilder) -> DebugRunner {
    let mut r = builder.start();
    r.game.enter_main_phase();
    r
}

pub fn action(r: &mut DebugRunner, player: u8, action: u16) {
    assert!(
        build_action_mask(&r.game, player)[action as usize] > 0.5,
        "illegal action {action}, phase {:?}, prompt {:?}",
        r.current_phase(),
        r.pending_selection_view()
    );
    r.game.decode_action(action, player);
}

pub fn candidates(r: &DebugRunner) -> Vec<String> {
    let mut cards: Vec<_> = r
        .prompt_card_candidates()
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    cards.sort();
    cards
}

pub fn stocked_tamer(r: &mut DebugRunner, id: &str) -> PermanentHandle {
    let h = r.place_stack(0, &["TEXT-BOTTOM", "TEXT-TOP", id]);
    for c in &mut r.game.players[0].battle_area[h.index as usize].card_sources[..2] {
        c.face_down = true;
    }
    h
}

pub fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn next_turn(r: &mut DebugRunner) {
    let player = r.game.turn_player();
    action(r, player, PASS);
    if r.current_phase() == digimon_engine::enums::GamePhase::Breeding {
        action(r, 1 - player, PASS);
    }
}
