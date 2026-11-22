use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;

const CARD_ID: &str = "BT10-111";

const ONE_SLOT_BALLISTAMON_XROS_TARGET: &str = r#"
card: TEST-BALLISTAMON-XROS
name: Ballistamon Requirement Target
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_is: Ballistamon }
        zones: [battle_area]
        cost_delta: -2
    cost: 5
"#;

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
}

#[test]
fn bt10_111_on_play_wildcard_can_replace_one_missing_digixros_requirement_this_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-111 YAML loads without raw_rust wildcard placeholder")
        .from_dsl_yaml(ONE_SLOT_BALLISTAMON_XROS_TARGET)
        .expect("test DigiXros target YAML compiles")
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    let memory_before = runner.memory();
    let _ = runner.play(0, 0);
    if runner
        .pending_selection()
        .is_some_and(|selection| selection.valid_action_ids.contains(&PASS))
    {
        runner
            .execute_action(0, PASS)
            .expect("decline BT10-111's own DigiXros materials");
    }
    runner
        .auto_resolve()
        .expect("resolve BT10-111 on-play wildcard registration");
    let king_index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|permanent| permanent.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT10-111 should be played") as u8;
    push_to_hand(&mut runner, 0, "TEST-BALLISTAMON-XROS");

    let _ = runner.play(0, 0);

    let material_prompt = runner
        .pending_selection()
        .expect("DigiXros material prompt should be installed");
    let king_action = physical_menu::battle_top_action(&runner.game, digimon_engine::permanent::PermanentHandle { player: 0, index: king_index });
    assert!(
        material_prompt
            .valid_action_ids
            .contains(&king_action),
        "BT10-111 should be legal as a wildcard material for one missing Ballistamon requirement"
    );
    runner
        .execute_action(0, king_action)
        .expect("select BT10-111 as wildcard DigiXros material");
    // The recipe is full (no eligible material left): DCGO's
    // `SelectDigiXrosClass.Select` ends by itself (a one-"End"-only element is
    // skipped without a prompt), so the play commits with no trailing stop prompt.

    assert_eq!(
        memory_before - runner.memory(),
        8,
        "BT10-111 costs 5, then target cost 5 is reduced by one wildcard material to 3"
    );
    let target = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == "TEST-BALLISTAMON-XROS"
        })
        .expect("DigiXros target should be played");
    assert!(
        target
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == CARD_ID),
        "BT10-111 should become the selected DigiXros source"
    );
}

// --- "The name of this card/Digimon is also treated as [Shoutmon]." -----------
//
// Printed (official Bandai DB, first sentence of the effect box). CR 4.3
// 2-3-1-2: a brackets-only reference ("[Digivolve] [Shoutmon]: Cost 4",
// "[Shoutmon] x [Ballistamon]") matches a card with that NAME, and this card
// has the name [Shoutmon] wherever it is. DCGO BT10_111.cs: an unconditional
// `ChangeCardNamesClass` adding "Shoutmon" (`CanUseCondition` returns true).
// The spec carried no `also_treated_as`, so only substring gates saw it; the
// 2026-11 name-gate audit made the [Shoutmon] routes of OmniShoutmon (BT19-012)
// and OmniShoutmon (BT21-021) exact, which dropped this card from them.

/// A Lv.5 blue card whose ONLY digivolve route is the brackets-only
/// "[Digivolve] [Shoutmon]: Cost 1": Shoutmon (King Version) is red Lv.4, so
/// nothing but the exact-name gate can let it through.
const FROM_SHOUTMON_ONLY: &str = r#"
card: TEST-FROM-SHOUTMON
name: Shoutmon Route Target
kind: digimon
level: 5
color: [blue]
cost: 8
dp: 7000
alt_paths:
  - kind: digivolve
    from: { name_is: Shoutmon }
    cost: 1
"#;

#[test]
fn bt10_111_has_the_name_shoutmon_in_hand_and_as_a_digivolution_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-111 YAML loads")
        .from_dsl_yaml(FROM_SHOUTMON_ONLY)
        .expect("route target YAML compiles")
        .hand(0, &[CARD_ID])
        .start();
    // `card_names` lists the card's exact names (a substring test would pass on
    // the printed name "Shoutmon (King Version)" alone).
    assert!(
        runner.game.players[0].hand[0]
            .card_names(&runner.game.card_data)
            .contains(&"Shoutmon"),
        "in the hand: \"the name of this card ... is also treated as [Shoutmon]\""
    );
    runner.place_stack(0, &[CARD_ID, "TEST-FROM-SHOUTMON"]);
    assert!(
        runner.game.players[0].battle_area[0].card_sources[0]
            .card_names(&runner.game.card_data)
            .contains(&"Shoutmon"),
        "as a digivolution card"
    );
}

#[test]
fn bt10_111_meets_a_brackets_only_shoutmon_digivolve_route() {
    use digimon_engine::action::mask::build_action_mask;
    use digimon_engine::action::space::encode_digivolve;
    use digimon_engine::enums::{GamePhase, PlaySource};

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-111 YAML loads")
        .from_dsl_yaml(FROM_SHOUTMON_ONLY)
        .expect("route target YAML compiles")
        .hand(0, &["TEST-FROM-SHOUTMON"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    let slot = runner.place_on_field(0, CARD_ID, Some(0)).index as usize;

    assert!(
        build_action_mask(&runner.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5,
        "Shoutmon (King Version) is a card named [Shoutmon]: the exact-name route is offered"
    );
    let memory = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(memory - runner.memory(), 1, "the [Shoutmon] route's cost");
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
