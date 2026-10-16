//! Official general_rule.pdf p17, 8-2-3-3: stack, draw one, then finish DNA.
use digimon_engine::action::{build_action_mask, space::{DNA_DIGIVOLVE_START, PLAY_HAND_START, PASS}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::CardColor;
use digimon_engine::game::Game;
use std::sync::Arc;

struct DrawWitness;
impl CardEffect for DrawWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card).name("draw already completed")
            .optional().needs_outer_optional_prompt()
            .process(|ctx| {
                assert_eq!(ctx.game.players[1].hand.len(), 1, "bonus is available before effects");
                assert_eq!(ctx.game.players[1].hand[0].card_id(&ctx.game.card_data), "BONUS");
            }).build()]
    }
}
fn setup() -> DebugRunner {
    let mut field = make_test_card("FIELD", "Breakdramon");
    field.level = Some(6); field.dp = Some(12000); field.colors = vec![CardColor::Green];
    let mut hand = make_test_card("HAND", "Slayerdramon");
    hand.level = Some(6); hand.dp = Some(12000); hand.colors = vec![CardColor::Blue];
    let mut atk = make_test_card("ATK", "Attacker");
    atk.level = Some(6); atk.dp = Some(1000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../cards/bt20/BT20-045.yaml")).unwrap()
        .add_card(field).add_card(hand).add_card(atk)
        .add_card(make_test_card("BONUS", "Bonus"))
        .hand(1, &["BT20-045", "HAND"])
        .deck(1, &["BONUS", "BONUS"])
        .security(1, &["BONUS"])
        .start();
    r.register_effect("BT20-045", Arc::new(DrawWitness));
    let a = r.place_on_field(0, "ATK", Some(0));
    let target = r.place_on_field(1, "FIELD", Some(0));
    r.attack_digimon(a, target, false);
    r
}
fn choose(g: &mut Game, action: u16) {
    let actor = g.pending_selection.as_ref().unwrap().selecting_player;
    assert!(build_action_mask(g, actor)[action as usize] > 0.5);
    g.resolve_selection(actor, action).unwrap();
}
#[test]
fn blast_dna_draws_once_before_effects_and_survives_cloned_material_and_effect_choices() {
    let mut r = setup();
    choose(&mut r.game, DNA_DIGIVOLVE_START);
    choose(&mut r.game, 0);
    let mut material_clone = r.game.clone();
    for game in [&mut r.game, &mut material_clone] {
        choose(game, PLAY_HAND_START + 1);
        assert_eq!(game.players[1].hand.len(), 1, "Blast DNA grants its bonus draw");
        assert_eq!(game.players[1].deck.len(), 1);
        assert_eq!(game.players[1].battle_area[0].top_card().card_id(&game.card_data), "BT20-045");
        let mut effect_clone = game.clone();
        for resumed in [game, &mut effect_clone] {
            let yes = *resumed.pending_selection.as_ref().unwrap().valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
            choose(resumed, yes);
            assert_eq!(resumed.players[1].hand.len(), 1, "resume must not draw again");
            assert_eq!(resumed.players[1].deck.len(), 1);
        }
    }
}
#[test]
fn declined_blast_counter_keeps_materials_and_does_not_draw() {
    let mut r = setup();
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        choose(game, PASS);
        assert_eq!(game.players[1].hand.len(), 2);
        assert_eq!(game.players[1].deck.len(), 2);
        assert_eq!(game.players[1].battle_area[0].top_card().card_id(&game.card_data), "FIELD");
    }
}


