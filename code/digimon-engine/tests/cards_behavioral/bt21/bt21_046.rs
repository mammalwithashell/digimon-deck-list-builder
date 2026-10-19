//! BT21-046: official observation e69c56cb... plus local printed image and DCGO BT21_046.cs.
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{DnaCost, DnaRequirement, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ID: &str = "BT21-046";
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt21/BT21-046.yaml"))
        .expect("BT21-046 YAML");
    for (id, name, level, color) in [
        ("GOOD", "Coredramon", 4, CardColor::Green),
        ("WRONG", "Coredramon (X Antibody)", 4, CardColor::Green),
        ("ILLEGAL", "Coredramon", 4, CardColor::Red),
        ("BASE", "Dracomon", 3, CardColor::Red),
        ("PARTNER", "Partner", 4, CardColor::Green),
        ("BADPARTNER", "Partner", 4, CardColor::Red),
        ("FILL", "Filler", 3, CardColor::Red),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![color];
        c.evo_costs = vec![EvoCost {
            card_color: color as u8,
            level: 3,
            memory_cost: 3,
        }];
        b = b.add_card(c);
    }
    let mut dna = make_test_card("DNA", "DNA result");
    dna.level = Some(5);
    let req = DnaRequirement {
        level: 4,
        card_colors: vec![CardColor::Green],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    dna.dna_costs = vec![DnaCost {
        requirement1: req.clone(),
        requirement2: req,
        memory_cost: 2,
    }];
    let mut r = b
        .add_card(dna)
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner, t: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}
#[test]
fn free_coredramon_pick_preserves_name_and_requirements_at_both_timings() {
    for t in [
        EffectTiming::StartOfYourMainPhase,
        EffectTiming::WhenDigivolving,
    ] {
        let mut r = setup(&["GOOD", "WRONG", "ILLEGAL"]);
        let h = r.place_on_field(0, ID, Some(0));
        fire(&mut r, t, h);
        let v = r.pending_selection_view().expect("Coredramon choice");
        assert!(v.is_optional);
        assert_eq!(
            v.valid_action_ids
                .iter()
                .copied()
                .filter(|a| *a != PASS)
                .collect::<Vec<_>>(),
            vec![PLAY_HAND_START]
        );
        let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
        assert!(mask[PLAY_HAND_START as usize] > 0.0);
        assert_eq!(mask[PLAY_HAND_START as usize + 1], 0.0);
        assert_eq!(mask[PLAY_HAND_START as usize + 2], 0.0);
        assert!(mask[PASS as usize] > 0.0);
        let memory = r.memory();
        let deck = r.game.players[0].deck.len();
        r.execute_action(0, PLAY_HAND_START).unwrap();
        assert_eq!(top(&r, h), "GOOD");
        assert_eq!(r.memory(), memory);
        assert_eq!(
            r.game.players[0].deck.len(),
            deck - 1,
            "digivolution bonus draw"
        );
    }
}
#[test]
fn free_evolution_can_be_declined_and_missing_legal_target_is_a_noop() {
    let mut r = setup(&["GOOD"]);
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(top(&r, h), ID);
    let mut r = setup(&["WRONG", "ILLEGAL"]);
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, h), ID);
}
#[test]
fn inherited_dna_pays_printed_cost_and_exposes_each_choice() {
    let mut r = setup(&["DNA"]);
    let h = r.place_stack(0, &[ID, "GOOD"]);
    r.place_on_field(0, "PARTNER", Some(0));
    let m = r.memory();
    fire(&mut r, EffectTiming::EndOfYourTurn, h);
    let mut choices = 0;
    while let Some(v) = r.pending_selection_view() {
        assert!(choices < 5);
        assert!(v.is_optional);
        let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
        r.execute_action(v.selecting_player, a).unwrap();
        choices += 1;
    }
    assert!(choices >= 2);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(top(&r, h), "DNA");
    assert_eq!(r.memory(), m - 2);
}
#[test]
fn inherited_dna_decline_and_illegal_partner_preserve_board() {
    for partner in ["PARTNER", "BADPARTNER"] {
        let mut r = setup(&["DNA"]);
        let h = r.place_stack(0, &[ID, "GOOD"]);
        r.place_on_field(0, partner, Some(0));
        fire(&mut r, EffectTiming::EndOfYourTurn, h);
        if partner == "BADPARTNER" {
            accept_dna_activation(&mut r);
            assert!(
                r.pending_selection().is_none(),
                "no DNA-legal result means no misleading partner choice"
            );
        }
        if let Some(v) = r.pending_selection_view() {
            r.execute_action(v.selecting_player, PASS).unwrap();
        }
        assert_eq!(r.game.players[0].battle_area.len(), 2);
        assert_eq!(top(&r, h), "GOOD");
    }
}

#[test]
fn named_dracomon_route_is_free_and_actual_evolution_triggers_coredramon_choice() {
    use digimon_engine::enums::PlaySource;
    let mut r = setup(&[ID, "GOOD"]);
    let h = r.place_on_field(0, "BASE", Some(0));
    let m = r.memory();
    let action = digimon_engine::action::space::encode_digivolve(0, h.index as u16);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[action as usize] > 0.0);
    assert!(r
        .game
        .digivolve_from_hand(0, 0, h.index as usize, PlaySource::ByHand));
    assert_eq!(top(&r, h), ID);
    assert_eq!(r.memory(), m);
    let v = r.pending_selection_view().expect("When Digivolving choice");
    assert!(v.valid_action_ids.contains(&PLAY_HAND_START));
    r.execute_action(0, PLAY_HAND_START).unwrap();
    assert_eq!(top(&r, h), "GOOD");
}

#[test]
fn main_phase_entry_fires_only_top_effect_and_inherited_dna_is_own_turn_only() {
    let mut r = setup(&["GOOD"]);
    let h = r.place_on_field(0, ID, Some(0));
    r.game.enter_main_phase();
    assert!(r.pending_selection_view().is_some());
    r.execute_action(0, PASS).unwrap();
    assert_eq!(top(&r, h), ID);
    let mut r = setup(&["DNA"]);
    let h = r.place_stack(0, &[ID, "GOOD"]);
    r.place_on_field(0, "PARTNER", Some(0));
    r.game.turn_player_idx = 1;
    fire(&mut r, EffectTiming::EndOfYourTurn, h);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}

#[test]
fn inherited_dna_masks_illegal_partner_when_a_valid_partner_exists() {
    use digimon_engine::action::space::encode_attack;
    let mut r = setup(&["DNA"]);
    let h = r.place_stack(0, &[ID, "GOOD"]);
    let good = r.place_on_field(0, "PARTNER", Some(0));
    let bad = r.place_on_field(0, "BADPARTNER", Some(0));
    fire(&mut r, EffectTiming::EndOfYourTurn, h);
    accept_dna_activation(&mut r);
    let v = r.pending_selection_view().expect("legal partner choice");
    assert!(v
        .valid_action_ids
        .contains(&(encode_attack(0, good.index as u16))));
    assert!(!v
        .valid_action_ids
        .contains(&(encode_attack(0, bad.index as u16))));
    assert!(!v
        .valid_action_ids
        .contains(&(encode_attack(0, h.index as u16))));
    // Cloning drops callback closures and resumes through the serializable
    // continuation. It must preserve the filtered partner choice and cost.
    let actions = v.valid_action_ids.clone();
    let memory = r.memory();
    r.game = r.game.clone();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids,
        actions
    );
    r.execute_action(0, encode_attack(0, good.index as u16))
        .unwrap();
    let target = r.pending_selection_view().expect("DNA target after clone");
    assert!(target.valid_action_ids.contains(&PLAY_HAND_START));
    r.execute_action(0, PLAY_HAND_START).unwrap();
    assert_eq!(top(&r, h), "DNA");
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.memory(), memory - 2);
}

fn accept_dna_activation(r: &mut DebugRunner) {
    if let Some(v) = r.pending_selection_view() {
        if v.kind == digimon_engine::selection::SelectionKind::Replacement {
            assert!(v.is_optional);
            r.execute_action(v.selecting_player, v.valid_action_ids[0])
                .unwrap();
        }
    }
}
