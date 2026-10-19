//! Official BT20-025 observation 22da842eebcac8dc27cb5ed7cd43ff4d9b45c3022b9b92e554bf7af718ff137f, image and Q4314.
//! Rules: DNA requirements 8-2-3-1, Security A. 16-3-1.
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, DNA_DIGIVOLVE_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
const ID: &str = "BT20-025";
fn setup(hand: &[&str]) -> DebugRunner {
    let yaml = include_str!("../../../cards/bt20/BT20-025.yaml");
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .dsl_card("BT20-045")
        .unwrap();
    for (id, name, level, color, dp) in [
        ("BLUE4", "Blue", 4, CardColor::Blue, 5000),
        ("RED4", "Red", 4, CardColor::Red, 5000),
        ("CORE", "Coredramon", 4, CardColor::Green, 5000),
        ("WRONG", "Coredramon X", 4, CardColor::Green, 5000),
        ("LOW", "Low", 3, CardColor::Yellow, 5000),
        ("LIMIT", "Limit", 4, CardColor::Yellow, 6000),
        ("HIGH", "High", 5, CardColor::Yellow, 7000),
        ("BREAK", "Breakdramon", 6, CardColor::Green, 10000),
        ("TOP", "Top", 6, CardColor::Blue, 10000),
        ("FILL", "Fill", 3, CardColor::Red, 1000),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![color];
        c.dp = Some(dp);
        b = b.add_card(c);
    }
    let mut t = make_test_card("TAMER", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.dp = Some(1000);
    b = b.add_card(t);
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
#[test]
fn bt20_025_both_timings_offer_all_and_only_mandatory_dp_targets() {
    for played in [true, false] {
        let mut r = setup(&[ID]);
        for id in ["LOW", "LIMIT", "HIGH", "TAMER"] {
            r.place_on_field(1, id, None);
        }
        r.place_on_field(0, "LOW", None);
        if played {
            r.play(0, 0).unwrap();
        } else {
            let h = r.place_on_field(0, "BLUE4", None);
            let action = encode_digivolve(0, h.index as u16);
            assert!(digimon_engine::action::build_action_mask(&r.game, 0)[action as usize] > 0.0);
            r.game.decode_action(action, 0);
        }
        let v = r.pending_selection_view().expect("mandatory deletion");
        assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids, vec![ATTACK_START, ATTACK_START + 1]);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
            0.0
        );
        r.execute_action(0, ATTACK_START + 1).unwrap();
        assert!(r.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "LIMIT"));
        assert_eq!(r.game.players[1].battle_area.len(), 3);
    }
}
#[test]
fn bt20_025_no_legal_target_completes_without_a_prompt() {
    let mut r = setup(&[ID]);
    r.place_on_field(1, "HIGH", None);
    r.place_on_field(1, "TAMER", None);
    r.play(0, 0).unwrap();
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt20_025_native_and_exact_named_evolution_masks_and_costs() {
    for (base, cost) in [("BLUE4", 4), ("RED4", 4), ("CORE", 3), ("WRONG", -1)] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, base, None);
        let a = encode_digivolve(0, h.index as u16);
        let legal = digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0;
        assert_eq!(legal, cost >= 0, "{base}");
        if legal {
            let before = r.memory();
            r.game.decode_action(a, 0);
            assert_eq!(before - r.memory(), cost);
        }
    }
}
#[test]
fn bt20_025_dna_identity_is_result_scoped_and_face_only() {
    let mut r = setup(&["BT20-045", "TOP"]);
    let h = r.place_on_field(0, ID, None);
    r.place_on_field(0, "BREAK", None);
    r.game.tick_declarative_effects();
    let exa = r.game.players[0].hand[0].clone();
    let other = r.game.players[0].hand[1].clone();
    let extras = r.game.dna_material_extras(h, &exa);
    assert_eq!(extras.levels, vec![6]);
    assert_eq!(extras.names, vec!["Slayerdramon"]);
    assert!(r.game.dna_material_extras(h, &other).is_empty());
    assert!(r.game.has_valid_dna_route_for_hand_card(0, 0));
    assert!(
        digimon_engine::action::build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize] > 0.0
    );
    let p = &r.game.players[0].battle_area[h.index as usize];
    assert_eq!(p.level(&r.game.card_data), Some(5));
    assert!(!p
        .top_card()
        .card_names(&r.game.card_data)
        .contains(&"Slayerdramon"));
    let carrier = r.place_stack(0, &[ID, "TOP"]);
    assert!(r.game.dna_material_extras(carrier, &exa).is_empty());
}
#[test]
fn bt20_025_hand_material_does_not_gain_slayerdramon_identity() {
    // Positive control: the exact same two materials enable Blast DNA when Wingdramon is on field.
    let mut positive = setup(&[]);
    positive.add_to_hand(1, "BT20-045");
    positive.add_to_hand(1, "BREAK");
    let atk = positive.place_on_field(0, "FILL", Some(0));
    let target = positive.place_on_field(1, ID, None);
    positive.game.tick_declarative_effects();
    positive.attack_digimon(atk, target, false);
    assert!(positive
        .pending_selection()
        .expect("Blast DNA counter")
        .valid_action_ids
        .contains(&DNA_DIGIVOLVE_START));
    let mut r = setup(&[]);
    r.add_to_hand(1, "BT20-045");
    r.add_to_hand(1, ID);
    let atk = r.place_on_field(0, "FILL", Some(0));
    let target = r.place_on_field(1, "BREAK", None);
    r.game.tick_declarative_effects();
    r.attack_digimon(atk, target, false);

    assert!(
        !r.pending_selection()
            .map(|p| p.valid_action_ids.contains(&DNA_DIGIVOLVE_START))
            .unwrap_or(false),
        "Q4314: Wingdramon in hand is not Slayerdramon"
    );
}
#[test]
fn bt20_025_security_attack_is_inherited_on_both_turns_only() {
    let mut r = setup(&[]);
    let face = r.place_on_field(0, ID, None);
    let h = r.place_stack(0, &[ID, "TOP"]);
    assert!(!r.game.has_keyword(face, Keyword::SecurityAttackPlus(1)));
    assert!(r.game.has_keyword(h, Keyword::SecurityAttackPlus(1)));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert!(r.game.has_keyword(h, Keyword::SecurityAttackPlus(1)));
}
