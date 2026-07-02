use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT10-029";

fn card_with_name(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 4);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn tamer(card_id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card
}

fn source_count_for_card_id(runner: &DebugRunner, player: usize, card_id: &str) -> usize {
    let data = &runner.game.card_data;
    runner.game.players[player]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} should be in battle area"))
        .card_sources
        .len()
}

fn make_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-029 YAML loads")
        .add_card(card_with_name("SHOUTMON-CARRIER", "Shoutmon X4"))
        .add_card(card_with_name("PLAIN-CARRIER", "Plain Carrier"))
        .add_card(tamer("XROS-TAMER"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start()
}

#[test]
fn bt10_029_declares_xros_heart_level_two_alt_path() {
    let runner = make_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT10-029 compiled card present");

    assert!(
        card.alt_paths.iter().any(|path| {
            let filter_debug = format!("{:?}", path.from);
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && filter_debug.contains("level_eq: Some(2)")
                && filter_debug.contains("Xros Heart")
        }),
        "Starmons should digivolve from Lv.2 Xros Heart for 0"
    );
}

#[test]
fn bt10_029_inherited_draws_for_shoutmon_named_carrier_only() {
    let mut runner = make_runner();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "SHOUTMON-CARRIER"]);

    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner
        .auto_resolve()
        .expect("resolve Starmons inherited draw");

    assert_eq!(runner.hand_size(0), hand_before + 1);

    let mut runner = make_runner();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "PLAIN-CARRIER"]);
    let hand_before = runner.hand_size(0);

    runner.attack_player(stack, 1, false);
    runner
        .auto_resolve()
        .expect("resolve non-Shoutmon attack without inherited draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "Starmons inherited text must require Shoutmon in the carrier name"
    );
}

#[test]
fn bt10_029_save_offers_own_tamer_and_is_optional() {
    let mut runner = make_runner();
    let starmons = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "XROS-TAMER", Some(0));
    let before = source_count_for_card_id(&runner, 0, "XROS-TAMER");

    runner
        .game
        .delete_permanent_with_cause(starmons, ReplacementCause::OpponentEffect);
    let pending = runner
        .pending_selection()
        .expect("Save should offer a Tamer destination");
    assert!(pending.is_optional, "Save must be optional");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the owned Tamer should be offered as a Save destination"
    );

    let action = pending.valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("choose Tamer for Save");
    runner.auto_resolve().expect("finish Save");

    let after = source_count_for_card_id(&runner, 0, "XROS-TAMER");
    assert_eq!(after, before + 1, "Save should place Starmons under Tamer");
}

#[test]
fn bt10_029_save_prompt_clones_faithfully() {
    let mut runner = make_runner();
    let starmons = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "XROS-TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(starmons, ReplacementCause::OpponentEffect);
    let action = {
        let pending = runner
            .pending_selection()
            .expect("Save should offer a Tamer destination");
        assert!(
            runner.game.pending_selection_resume.is_some(),
            "Save prompt must be resume-driven before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut cloned = runner.game.clone();
    cloned
        .resolve_selection(0, action)
        .expect("cloned Save prompt resolves");
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone must not consume the original prompt"
    );
    runner
        .execute_action(0, action)
        .expect("original Save prompt resolves");

    assert_eq!(
        cloned.players[0].battle_area[0].card_sources.len(),
        runner.game.players[0].battle_area[0].card_sources.len(),
        "clone and original should replay the same Save result"
    );
}
