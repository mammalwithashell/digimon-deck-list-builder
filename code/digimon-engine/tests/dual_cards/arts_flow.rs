use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn base_lv5(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.level = Some(5);
    card.dp = Some(7000);
    card.colors = vec![color];
    card
}

/// Lv5 Red base for the Arts-rule-check test. Carries POSITIVE DP so the
/// general ≤0-DP state-based rules-check (`run_state_based_rules_check`) does
/// not sweep it during setup drains. The 0-DP stack the test asserts on is
/// produced by the 0-DP Arts card (`zero_dp_arts_dual`) becoming the stack top
/// after the Arts digivolve — the base's DP is irrelevant once it's a source.
fn arts_red_base() -> CardData {
    base_lv5("ZERO-BASE", CardColor::Red)
}

fn purple_anchor() -> CardData {
    let mut card = make_test_card("PURPLE-ANCHOR", "Purple Anchor");
    card.level = Some(3);
    card.colors = vec![CardColor::Purple];
    card
}

fn arts_dual() -> CardData {
    let mut card = make_test_card("DUAL-ARTS", "Dual Arts");
    card.card_kind = CardKind::Dual;
    card.level = Some(6);
    card.dp = Some(12000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 5,
        memory_cost: 3,
    }];
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 12000,
            colors: vec![CardColor::Red],
            traits: vec!["DualTrait".to_string()],
            evo_costs: card.evo_costs.clone(),
            effect_text: "[When Digivolving] Draw 1.".to_string(),
            inherited_text: String::new(),
            keywords: vec![Keyword::ArtsDigivolve],
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::Purple],
            effect_text: "[Main] Gain 2 memory.".to_string(),
            security_text: String::new(),
            keywords: vec![Keyword::ArtsDigivolve],
        },
    });
    card
}

fn zero_dp_arts_dual() -> CardData {
    let mut card = arts_dual();
    card.dp = Some(0);
    if let Some(dual) = card.dual.as_mut() {
        dual.digimon.dp = 0;
    }
    card
}

struct GainTwo;
impl CardEffect for GainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Dual Option Main")
            .option_main()
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

struct DrawOnDigivolve(Arc<Mutex<u32>>);
impl CardEffect for DrawOnDigivolve {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Dual Option Main")
                .option_main()
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::when_digivolving(card)
                .name("When Digivolving witness")
                .process(move |_ctx| {
                    *witness.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

struct HandMainDirect;
impl CardEffect for HandMainDirect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Direct hand main")
            .timing(digimon_engine::enums::EffectTiming::MainFromHand)
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

struct ArtsRuleCheckWitness {
    digivolve: Arc<Mutex<u32>>,
    deletion: Arc<Mutex<u32>>,
}
impl CardEffect for ArtsRuleCheckWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let digivolve = self.digivolve.clone();
        let deletion = self.deletion.clone();
        vec![
            Effect::on_play(card)
                .name("Dual Option Main")
                .option_main()
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::when_digivolving(card)
                .name("When Digivolving witness")
                .process(move |_ctx| {
                    *digivolve.lock().unwrap() += 1;
                })
                .build(),
            Effect::on_deletion(card)
                .name("On Deletion witness")
                .process(move |_ctx| {
                    *deletion.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

#[test]
fn arts_prompt_installs_after_option_main_and_pass_declines_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(GainTwo));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let sel = r.game.pending_selection.as_ref().expect("arts selection");
    assert_eq!(sel.kind, SelectionKind::OwnField);
    assert!(sel.is_optional, "PASS declines Arts");
    assert!(sel.valid_action_ids.contains(&encode_attack(0, 1)));

    r.game.resolve_selection(0, PASS).expect("decline arts");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.trash_size(0), 1, "declining Arts trashes normally");
    assert_eq!(r.battle_area_size(0), 2, "no Arts stack was created");
}

#[test]
fn arts_accept_stacks_pending_dual_draws_and_fires_when_digivolving() {
    let witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(DrawOnDigivolve(witness.clone())));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let action_id = encode_attack(0, 1);
    r.game.resolve_selection(0, action_id).expect("accept Arts");

    assert!(r.game.pending_option.is_none());
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.trash_size(0), 0, "Arts prevents normal Option trash");
    assert_eq!(r.hand_size(0), 1, "digivolution bonus draw happened");
    let perm = &r.game.player(0).battle_area[1];
    assert_eq!(perm.stack_size(), 2);
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "DUAL-ARTS");
    assert_eq!(*witness.lock().unwrap(), 1, "When Digivolving fired");
    assert_eq!(r.memory(), 2, "paid Option use cost, no digivolution cost");
}

#[test]
fn arts_can_target_legal_breeding_area_digimon() {
    use digimon_engine::action::space::BREEDING_SELECTION_TARGET;

    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(GainTwo));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));

    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BASE-RED")
        .unwrap();
    let next_idx = r.game.next_card_index();
    let base = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    r.game.players[0].breeding_area = Some(digimon_engine::permanent::Permanent::new(base, 0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let sel = r.game.pending_selection.as_ref().expect("arts selection");
    assert!(
        sel.valid_action_ids.contains(&BREEDING_SELECTION_TARGET),
        "breeding target appears as legal Arts target"
    );

    r.game
        .resolve_selection(0, BREEDING_SELECTION_TARGET)
        .expect("accept breeding Arts");
    let breeding = r
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .expect("breeding remains");
    assert_eq!(breeding.stack_size(), 2);
    assert_eq!(breeding.top_card().card_id(&r.game.card_data), "DUAL-ARTS");
    assert_eq!(r.hand_size(0), 1, "bonus draw happened");
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn direct_hand_main_activation_does_not_enable_arts() {
    let mut r = DebugRunner::builder()
        .add_card(arts_dual())
        .add_card(base_lv5("BASE-RED", CardColor::Red))
        .add_card(purple_anchor())
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect("DUAL-ARTS", Arc::new(HandMainDirect));
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    advance_to_main(&mut r);

    assert!(r.game.activate_hand_main(0, 0));
    assert!(
        r.game.pending_selection.is_none(),
        "direct MainFromHand activation must not open Arts selection"
    );
    assert_eq!(
        r.hand_size(0),
        1,
        "direct activation does not use/trash the card"
    );
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn arts_runs_rule_check_before_trigger_resolution() {
    let deletion_witness = Arc::new(Mutex::new(0));
    let digivolve_witness = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(zero_dp_arts_dual())
        .add_card(arts_red_base())
        .add_card(purple_anchor())
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-ARTS"])
        .memory(5)
        .start();
    r.register_effect(
        "DUAL-ARTS",
        Arc::new(ArtsRuleCheckWitness {
            digivolve: digivolve_witness.clone(),
            deletion: deletion_witness.clone(),
        }),
    );
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "ZERO-BASE", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    r.game
        .resolve_selection(0, encode_attack(0, 1))
        .expect("accept Arts");

    // The ≤0-DP rule-check deletion leaves [On Deletion] and [When
    // Digivolving] pending simultaneously for player 0, so the engine
    // surfaces the resolution order as a TriggerOrder prompt. Either
    // order fires both witnesses exactly once.
    while r
        .game
        .pending_selection
        .as_ref()
        .is_some_and(|sel| sel.kind == SelectionKind::TriggerOrder)
    {
        let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        r.game
            .resolve_selection(0, action)
            .expect("resolve trigger-order prompt");
    }

    assert_eq!(r.battle_area_size(0), 1, "0-DP Arts stack was deleted");
    assert_eq!(
        r.trash_size(0),
        2,
        "base and DUAL moved to trash by deletion"
    );
    assert_eq!(*deletion_witness.lock().unwrap(), 1, "On Deletion fired");
    assert_eq!(
        *digivolve_witness.lock().unwrap(),
        1,
        "When Digivolving fired"
    );
}
