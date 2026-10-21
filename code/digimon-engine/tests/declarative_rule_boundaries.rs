//! Continuous DP refresh at completed-effect rule boundaries (17-1-3-1-1).
//! Like judge_quiz/b_deferred_rules_check Q6/Q13/Q14/Q24, a parked
//! sub-choice is not the completion of the effect which installed it.

use std::sync::Arc;
use digimon_engine::action::{mask::build_action_mask, space::{PASS, PLAY_HAND_START}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, Expiry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const LETHAL: &str = "card: BOUNDARY-LETHAL\nname: Boundary Lethal\nkind: tamer\ncolor: [yellow]\ncost: 0\neffects:\n  - kind: aura\n    target: { owner: opponent, kind: digimon, zone: [battle_area] }\n    dp_modifier: -4000\n";
const SUPPORT: &str = "card: BOUNDARY-SUPPORT\nname: Boundary Support\nkind: tamer\ncolor: [yellow]\ncost: 0\neffects:\n  - kind: aura\n    target: { owner: you, kind: digimon, zone: [battle_area] }\n    dp_modifier: 3000\n";
const DRIVER: &str = r#"
card: BOUNDARY-DRIVER
name: Boundary Driver
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: entrant
          filter: { name_is: Boundary Entrant }
          prompt: Choose the entrant
      - play_from_hand_free: { of: you, hand_index: entrant }
      - select_opponent_permanent:
          bind_as: aura
          optional: true
          continue_on_decline: true
          filter: { name_is: Boundary Lethal }
          prompt: Remove the lethal aura before this effect finishes?
      - if:
          condition: { binding_exists: aura }
          then:
            - delete_permanent: { target: aura }
      - gain_memory: 1
"#;

fn entrant() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("BOUNDARY-ENTRANT", "Boundary Entrant");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

fn legal_choice(r: &mut DebugRunner, action: u16) {
    let player = r.pending_selection_view().expect("explicit pending choice").selecting_player;
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.resolve_selection(player, action).unwrap();
}

#[test]
fn lethal_aura_removal_waits_for_whole_effect_including_parked_clone() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(LETHAL).unwrap()
        .from_dsl_yaml(DRIVER).unwrap()
        .add_card(entrant())
        .hand(0, &["BOUNDARY-DRIVER", "BOUNDARY-ENTRANT"])
        .memory(3).start();
    r.skip_mulligan();
    r.place_on_field(1, "BOUNDARY-LETHAL", Some(0));
    r.game.tick_declarative_effects();
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    let hand = r.pending_selection_view().expect("entrant choice");
    assert_eq!(hand.valid_action_ids.len(), 1);
    legal_choice(&mut r, hand.valid_action_ids[0]);
    let aura_choice = r.pending_selection_view().expect("effect must park before deleting aura");
    assert!(aura_choice.is_optional);
    assert_eq!(aura_choice.valid_action_ids.len(), 1);
    let aura_action = aura_choice.valid_action_ids[0];
    assert_eq!(r.battle_area_size(0), 2, "lethal entrant survives while effect is parked");
    let host = PermanentHandle { player: 0, index: 1 };
    r.game.tick_declarative_effects();
    assert_eq!(r.game.effective_dp(host), Some(0), "the entrant really is under a lethal continuous effect");
    let parked = r.game.clone();
    let before = r.memory();

    legal_choice(&mut r, aura_action);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.battle_area_size(0), 2, "removing the aura before effect completion saves entrant");
    assert_eq!(r.game.effective_dp(host), Some(2000));
    assert_eq!(r.memory(), before + 1);

    r.game = parked;
    assert_eq!(r.battle_area_size(1), 1, "clone retains its own unchosen aura");
    legal_choice(&mut r, PASS);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.battle_area_size(0), 1, "declining aura removal deletes entrant at completion");
    assert!(r.game.players[0].trash.iter().any(|c| c.card_id(&r.game.card_data) == "BOUNDARY-ENTRANT"));
    assert_eq!(r.memory(), before + 1, "the mandatory tail still resolves");
}

struct RemoveSupportThenObserve;
impl CardEffect for RemoveSupportThenObserve {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::when_attacking(card).name("Remove support first")
                .process(|ctx| { ctx.delete_permanent(PermanentHandle { player: 0, index: 1 }); }).build(),
            Effect::when_attacking(card).name("Observe after boundary")
                .process(|ctx| {
                    let remains = ctx.game.players[0].battle_area.iter().any(|p|
                        p.top_card().card_id(&ctx.game.card_data) == "BOUNDARY-ENTRANT");
                    ctx.gain_memory(if remains { 5 } else { 1 });
                }).build(),
        ]
    }
}

#[test]
fn removing_support_refreshes_dp_and_deletes_before_next_queued_effect() {
    let mut driver = make_test_card("BOUNDARY-OBSERVER", "Boundary Observer");
    driver.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder().from_dsl_yaml(SUPPORT).unwrap()
        .add_card(driver).add_card(entrant()).memory(3).start();
    r.skip_mulligan();
    r.register_effect("BOUNDARY-OBSERVER", Arc::new(RemoveSupportThenObserve));
    let source = r.place_on_field(0, "BOUNDARY-OBSERVER", Some(0));
    r.place_on_field(0, "BOUNDARY-SUPPORT", Some(0));
    let victim = r.place_on_field(0, "BOUNDARY-ENTRANT", Some(0));
    let card = r.top_card(source);
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(source), 0);
        ctx.add_dp_modifier(victim, -4000, Expiry::Permanent);
    }
    r.game.tick_declarative_effects();
    assert_eq!(r.game.effective_dp(victim), Some(1000));
    let before = r.memory();
    r.game.enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
    let order = r.pending_selection_view().expect("both simultaneous effects expose ordering");
    let choices = order.effect_choices.as_ref().expect("effect-slot order choices");
    assert_eq!(choices.len(), 2);
    // Trigger-order labels identify the registered slot, not Effect::name.
    let remove = choices.iter().find(|c| c.label == "BOUNDARY-OBSERVER slot 0 (mandatory)")
        .expect("registered removal effect is slot 0").action_id;
    assert!(choices.iter().any(|c| c.label == "BOUNDARY-OBSERVER slot 1 (mandatory)"));
    legal_choice(&mut r, remove);
    assert_eq!(r.battle_area_size(0), 1, "loss of continuous support deletes the zero-DP recipient");
    if let Some(view) = r.pending_selection_view() {
        let observe = view.effect_choices.as_ref().unwrap().iter()
            .find(|c| c.label == "BOUNDARY-OBSERVER slot 1 (mandatory)")
            .expect("registered observer effect is slot 1").action_id;
        legal_choice(&mut r, observe);
    }
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), before + 1, "the subsequent queued effect must observe recipient already gone");
    let trash: Vec<_> = r.game.players[0].trash.iter().map(|c| c.card_id(&r.game.card_data)).collect();
    assert!(trash.contains(&"BOUNDARY-SUPPORT") && trash.contains(&"BOUNDARY-ENTRANT"));
}
