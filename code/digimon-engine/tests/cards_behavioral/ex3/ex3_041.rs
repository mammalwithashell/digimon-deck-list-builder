//! EX3-041: official primary observation a596d63c... and printed card image.
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, DNA_DIGIVOLVE_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::{DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};
const ID: &str = "EX3-041";
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cards/ex3/EX3-041.yaml"
            ))
            .expect("EX3-041 restored production YAML"),
        )
        .expect("EX3-041 production YAML");
    for (id, name, level, color) in [
        ("CORE", "Coredramon", 4, CardColor::Red),
        ("NEAR", "Coredramon X", 4, CardColor::Red),
        ("BLUE4", "Blue", 4, CardColor::Blue),
        ("GREEN4", "Green", 4, CardColor::Green),
        ("BLUE3", "Blue", 3, CardColor::Blue),
        ("PARTNER", "Wingdramon", 6, CardColor::Blue),
        ("BADNAME", "Partner", 6, CardColor::Blue),
        ("BADLEVEL", "Wingdramon", 5, CardColor::Blue),
        ("TOP", "Other", 6, CardColor::Red),
        ("EXAMON_TOP", "Examon X", 7, CardColor::Red),
        ("FILL", "Fill", 3, CardColor::Red),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![color];
        c.dp = Some(8000);
        b = b.add_card(c);
    }
    for (id, name, level, name_req) in [
        ("DNA", "Examon", 6, ""),
        ("WRONG", "Examon X", 6, ""),
        ("NAMED", "Examon", 6, "Breakdramon"),
        ("OTHER", "Other DNA", 5, ""),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(7);
        c.dna_costs = vec![DnaCost {
            requirement1: DnaRequirement {
                level,
                card_colors: vec![CardColor::Green],
                name_contains: name_req.into(),
                text_contains: String::new(),
            },
            requirement2: DnaRequirement {
                level: 6,
                card_colors: vec![CardColor::Blue],
                name_contains: String::new(),
                text_contains: String::new(),
            },
            memory_cost: 2,
        }];
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.tick_declarative_effects();
    r.game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn activate(r: &mut DebugRunner) {
    if let Some(v) = r.pending_selection_view() {
        if v.kind == SelectionKind::Replacement {
            let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
            r.execute_action(v.selecting_player, a).unwrap();
        }
    }
}
fn exact_choices(r: &DebugRunner, expected: &[u16]) {
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids, expected);
    assert!(v.is_optional);
    let mut legal = expected.to_vec();
    if !legal.contains(&PASS) {
        legal.push(PASS);
    }
    legal.sort_unstable();
    let actual: Vec<u16> = digimon_engine::action::build_action_mask(&r.game, v.selecting_player)
        .iter()
        .enumerate()
        .filter_map(|(i, x)| (*x > 0.5).then_some(i as u16))
        .collect();
    assert_eq!(actual, legal);
}
fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}
#[test]
fn ex3_041_native_routes_and_exact_named_route() {
    for (base, cost) in [
        ("CORE", 3),
        ("BLUE4", 4),
        ("GREEN4", 4),
        ("NEAR", 0),
        ("BLUE3", 0),
    ] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, base, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0,
            cost > 0
        );
        if cost > 0 {
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 10 - cost);
            assert_eq!(top(&r, h), ID);
        }
    }
}
#[test]
fn ex3_041_identity_is_own_turn_field_only_exact_result_without_name_grant() {
    let mut r = setup(&["DNA", "WRONG", ID]);
    let h = r.place_on_field(0, ID, Some(0));
    let inherited = r.place_stack(0, &[ID, "TOP"]);
    r.game.tick_declarative_effects();
    let dna = r.game.players[0].hand[0].clone();
    let wrong = r.game.players[0].hand[1].clone();
    let extra = r.game.dna_material_extras(h, &dna);
    assert_eq!(extra.levels, vec![6]);
    assert!(extra.names.is_empty());
    assert!(r.game.dna_material_extras(h, &wrong).is_empty());
    assert!(r.game.dna_material_extras(inherited, &dna).is_empty());
    let blue = r.place_on_field(0, "PARTNER", Some(0));
    let meta = &r.game.card_data[dna.handle().0 as usize];
    let hand_mat = &r.game.players[0].hand[2];
    assert!(
        digimon_engine::dna_digivolve::matching_dna_cost_perm_and_card(
            meta,
            &r.game.players[0].battle_area[blue.index as usize],
            &r.game.card_data[hand_mat.handle().0 as usize],
            &r.game.card_data
        )
        .is_none()
    );
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(r.game.dna_material_extras(h, &dna).is_empty());
}
#[test]
fn ex3_041_identity_applies_only_to_result_in_owners_hand() {
    let mut r = setup(&["DNA"]);
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    let dna = r.game.players[0].hand.remove(0);
    r.game.players[0].trash.push(dna.clone());
    assert!(
        r.game.dna_material_extras(h, &dna).is_empty(),
        "printed identity is only for Examon in hand, never a trash result"
    );
}
#[test]
fn ex3_041_face_and_inherited_blocker_scope_both_turns() {
    for turn in [0, 1] {
        for (carrier, expected) in [("PARTNER", true), ("EXAMON_TOP", true), ("TOP", false)] {
            let mut r = setup(&[]);
            let face = r.place_on_field(0, ID, Some(0));
            let inherited = r.place_stack(0, &[ID, carrier]);
            let unrelated = r.place_on_field(0, "TOP", Some(0));
            r.game.turn_player_idx = turn;
            r.game.tick_declarative_effects();
            assert!(r.game.has_keyword(face, Keyword::Blocker));
            assert_eq!(r.game.has_keyword(inherited, Keyword::Blocker), expected);
            assert!(!r.game.has_keyword(unrelated, Keyword::Blocker));
        }
    }
}
#[test]
fn ex3_041_eot_partner_and_result_masks_cost_and_clone() {
    let mut r = setup(&["DNA", "WRONG", "NAMED", "OTHER"]);
    let h = r.place_on_field(0, ID, Some(0));
    let good = r.place_on_field(0, "PARTNER", Some(0));
    let badname = r.place_on_field(0, "BADNAME", Some(0));
    let badlevel = r.place_on_field(0, "BADLEVEL", Some(0));
    r.place_on_field(1, "PARTNER", Some(0));
    fire(&mut r, h);
    activate(&mut r);
    let v = r.pending_selection_view().expect("partner choice");
    assert!(v.is_optional);
    assert!(v
        .valid_action_ids
        .contains(&encode_attack(0, good.index as u16)));
    for bad in [h, badname, badlevel] {
        assert!(!v
            .valid_action_ids
            .contains(&encode_attack(0, bad.index as u16)));
    }
    let actions = v.valid_action_ids.clone();
    exact_choices(&r, &[encode_attack(0, good.index as u16)]);
    r.game = r.game.clone();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids,
        actions
    );
    r.execute_action(0, encode_attack(0, good.index as u16))
        .unwrap();
    let v = r.pending_selection_view().unwrap();
    assert!(v.valid_action_ids.contains(&PLAY_HAND_START));
    assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + 1)));
    assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + 2)));
    assert!(
        v.valid_action_ids.contains(&(PLAY_HAND_START + 3)),
        "EOT result is not restricted to Examon; native level 5 recipe remains legal"
    );
    exact_choices(&r, &[PLAY_HAND_START, PLAY_HAND_START + 3]);
    r.game = r.game.clone();
    exact_choices(&r, &[PLAY_HAND_START, PLAY_HAND_START + 3]);
    r.execute_action(0, PLAY_HAND_START).unwrap();
    assert_eq!(top(&r, h), "DNA");
    assert_eq!(r.memory(), 8);
    assert_eq!(r.game.players[0].battle_area.len(), 3);
}
#[test]
fn ex3_041_eot_declines_and_no_eligible_partner_or_wrong_scope() {
    for stage in 0..3 {
        let mut r = setup(&["DNA"]);
        let h = r.place_on_field(0, ID, Some(0));
        let p = r.place_on_field(0, "PARTNER", Some(0));
        fire(&mut r, h);
        if stage > 0 {
            activate(&mut r);
        }
        if stage > 1 {
            r.execute_action(0, encode_attack(0, p.index as u16))
                .unwrap();
        }
        let v = r.pending_selection_view().unwrap();
        assert!(v.is_optional);
        assert!(digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize] > 0.0);
        r.execute_action(0, PASS).unwrap();
        assert_eq!(r.memory(), 10);
        assert_eq!(r.game.players[0].battle_area.len(), 2);
    }
    for (partner, turn, inherited) in [
        ("BADNAME", 0, false),
        ("BADLEVEL", 0, false),
        ("PARTNER", 1, false),
        ("PARTNER", 0, true),
    ] {
        let mut r = setup(&["DNA"]);
        let h = if inherited {
            r.place_stack(0, &[ID, "TOP"])
        } else {
            r.place_on_field(0, ID, Some(0))
        };
        r.place_on_field(0, partner, Some(0));
        r.game.turn_player_idx = turn;
        fire(&mut r, h);
        activate(&mut r);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 10);
    }
}

#[test]
fn ex3_041_real_examon_identity_tracks_each_instance_zone_owner_and_clone() {
    let own_yaml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/cards/ex3/EX3-041.yaml"
    ))
    .unwrap();
    let mut partner = make_test_card("BLUE6", "Slayerdramon");
    partner.level = Some(6);
    partner.colors = vec![CardColor::Blue];
    partner.dp = Some(12000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&own_yaml)
        .unwrap()
        .from_dsl_yaml(include_str!("../../../cards/ex3/EX3-074.yaml"))
        .unwrap()
        .add_card(partner)
        .hand(0, &["EX3-074", "EX3-074"])
        .hand(1, &["EX3-074"])
        .memory(10)
        .start();
    r.game.current_phase = GamePhase::Main;
    let ground = r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "BLUE6", Some(0));
    r.game.tick_declarative_effects();
    let own = r.game.players[0].hand[0].clone();
    let trashed = r.game.players[0].hand.remove(1);
    r.game.players[0].trash.push(trashed.clone());
    let enemy = r.game.players[1].hand[0].clone();
    assert_eq!(r.game.dna_material_extras(ground, &own).levels, vec![6]);
    assert!(r.game.dna_material_extras(ground, &own).names.is_empty());
    assert!(r.game.dna_material_extras(ground, &trashed).is_empty());
    assert!(r.game.dna_material_extras(ground, &enemy).is_empty());
    assert_eq!(
        r.game.players[0].battle_area[0].level(&r.game.card_data),
        Some(5)
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_names(&r.game.card_data),
        vec!["Groundramon"]
    );
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);
    assert_eq!(mask[(DNA_DIGIVOLVE_START + 1) as usize], 0.0);
    let mut fork = r.game.clone();
    let moved = fork.players[0].hand.remove(0);
    fork.players[0].trash.push(moved);
    assert!(fork.dna_material_extras(ground, &own).is_empty());
    assert_eq!(
        digimon_engine::action::build_action_mask(&fork, 0)[DNA_DIGIVOLVE_START as usize],
        0.0
    );
    assert_eq!(r.game.dna_material_extras(ground, &own).levels, vec![6]);
    let returned = fork.players[0].trash.pop().unwrap();
    fork.players[0].hand.push(returned);
    assert_eq!(fork.dna_material_extras(ground, &own).levels, vec![6]);
}
