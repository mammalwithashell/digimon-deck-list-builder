//! BT27-019 Gaomon. Official EN category 522040, image BT27-019.png,
//! Q7631–7634 checked 2026-11-22: placement is face down at the bottom.
//! New authoring, not recovered historical implementation; synthetic targets only.
use digimon_engine::action::{
    build_action_mask, encode_digivolve, ATTACK_START, BREEDING_TARGET, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;
const CID: &str = "BT27-019";
fn card(id: &str, name: &str, kind: CardKind, level: Option<u8>, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.level = level;
    c.colors = vec![color];
    c.dp = Some(5000);
    c
}
fn setup(hand: &[&str]) -> DebugRunner {
    let mut squad = card("SQUAD", "Egg", CardKind::DigiEgg, Some(2), CardColor::Red);
    squad.traits = vec!["DATA SQUAD".into()];
    let mut sec = card(
        "SEC",
        "Security",
        CardKind::Digimon,
        Some(3),
        CardColor::Blue,
    );
    sec.dp = Some(0);
    let mut r = DebugRunner::builder()
        .dsl_card(CID)
        .expect("BT27-019 production YAML must exist")
        .add_card(card(
            "THOMAS",
            "Thomas H. Norstein",
            CardKind::Tamer,
            None,
            CardColor::Blue,
        ))
        .add_card(card(
            "OTHER",
            "Thomas H. Norstein X",
            CardKind::Tamer,
            None,
            CardColor::Blue,
        ))
        .add_card(card(
            "IMPOSTOR",
            "Thomas H. Norstein",
            CardKind::Digimon,
            Some(4),
            CardColor::Blue,
        ))
        .add_card(card(
            "PAD",
            "Filler",
            CardKind::Digimon,
            Some(4),
            CardColor::Blue,
        ))
        .add_card(card(
            "OPTION",
            "Payment",
            CardKind::Option,
            None,
            CardColor::Red,
        ))
        .add_card(card(
            "BLUE",
            "Egg",
            CardKind::DigiEgg,
            Some(2),
            CardColor::Blue,
        ))
        .add_card(card(
            "RED",
            "Egg",
            CardKind::DigiEgg,
            Some(2),
            CardColor::Red,
        ))
        .add_card(card(
            "WANYA",
            "Wanyamon",
            CardKind::DigiEgg,
            Some(2),
            CardColor::Green,
        ))
        .add_card(sec)
        .add_card(squad)
        .hand(0, hand)
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
        .security(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}
fn picks(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .expect("player choice")
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != PASS)
        .collect()
}
fn act(r: &mut DebugRunner, a: u16) {
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.execute_action(0, a).unwrap();
}
fn sole(r: &mut DebugRunner) {
    let p = picks(r);
    assert_eq!(p.len(), 1);
    act(r, p[0]);
}
fn sizes(r: &DebugRunner) -> (usize, usize) {
    (r.deck_size(0), r.deck_size(1))
}
fn sources(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}
#[test]
fn bt27_019_printed_dp_and_routes() {
    for (host, legal) in [
        ("BLUE", true),
        ("WANYA", true),
        ("SQUAD", true),
        ("RED", false),
        ("PAD", false),
    ] {
        let mut r = setup(&[CID]);
        assert_eq!(r.compiled_card(CID).unwrap().dp, Some(1000));
        r.place_in_breeding(0, host);
        let a = encode_digivolve(0, BREEDING_TARGET);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize],
            if legal { 1.0 } else { 0.0 },
            "{host}"
        );
        if legal {
            let memory = r.memory();
            let d = r.deck_size(0);
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), memory);
            assert_eq!(r.deck_size(0), d - 1);
        }
    }
}
#[test]
fn bt27_019_play_pays_any_card_face_down_bottom_then_both_draw() {
    let mut r = setup(&[CID, "OPTION", "PAD"]);
    let t = r.place_stack(0, &["PAD", "THOMAS"]);
    let other = r.place_on_field(0, "THOMAS", Some(0));
    let wrong = r.place_on_field(0, "OTHER", Some(0));
    let imp = r.place_on_field(0, "IMPOSTOR", Some(0));
    r.place_on_field(1, "THOMAS", Some(0));
    let d = sizes(&r);
    r.play(0, 0).expect("ordinary play");
    let choices = picks(&r);
    assert_eq!(choices.len(), 2);
    act(&mut r, choices[0]);
    let targets = picks(&r);
    assert!(targets.contains(&(ATTACK_START + t.index as u16)));
    assert!(targets.contains(&(ATTACK_START + other.index as u16)));
    assert!(!targets.contains(&(ATTACK_START + wrong.index as u16)));
    assert!(!targets.contains(&(ATTACK_START + imp.index as u16)));
    act(&mut r, ATTACK_START + t.index as u16);
    assert_eq!(sources(&r, t), vec!["OPTION", "PAD", "THOMAS"]);
    let s = &r.game.players[0].battle_area[t.index as usize].card_sources;
    assert!(s[0].face_down);
    assert!(!s[1].face_down);
    assert_eq!(sizes(&r), (d.0 - 1, d.1 - 1));
    assert_eq!(r.memory(), 7);
}
#[test]
fn bt27_019_declining_hand_payment_does_not_draw() {
    let mut r = setup(&[CID, "OPTION"]);
    r.place_on_field(0, "THOMAS", Some(0));
    let d = sizes(&r);
    r.play(0, 0);
    act(&mut r, PASS);
    assert_eq!(sizes(&r), d);
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn bt27_019_declining_destination_does_not_pay_or_draw() {
    let mut r = setup(&[CID, "OPTION"]);
    let t = r.place_on_field(0, "THOMAS", Some(0));
    let d = sizes(&r);
    r.play(0, 0);
    sole(&mut r);
    act(&mut r, PASS);
    assert_eq!(sizes(&r), d);
    assert_eq!(sources(&r, t), vec!["THOMAS"]);
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn bt27_019_no_payment_or_no_own_thomas_no_draw() {
    for no_card in [true, false] {
        let mut r = setup(if no_card { &[CID] } else { &[CID, "OPTION"] });
        if no_card {
            r.place_on_field(0, "THOMAS", Some(0));
        } else {
            r.place_on_field(1, "THOMAS", Some(0));
            r.place_on_field(0, "OTHER", Some(0));
        }
        let d = sizes(&r);
        r.play(0, 0);
        assert!(r.pending_selection().is_none());
        assert_eq!(sizes(&r), d);
    }
}
#[test]
fn bt27_019_actual_move_can_pay_and_draw() {
    let mut r = setup(&["OPTION"]);
    let t = r.place_on_field(0, "THOMAS", Some(0));
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, CID);
    let d = sizes(&r);
    assert!(r.move_from_breeding(0));
    sole(&mut r);
    sole(&mut r);
    assert_eq!(sizes(&r), (d.0 - 1, d.1 - 1));
    assert_eq!(sources(&r, t), vec!["OPTION", "THOMAS"]);
}
#[test]
fn bt27_019_other_move_does_not_trigger() {
    let mut r = setup(&["OPTION"]);
    r.place_on_field(0, "THOMAS", Some(0));
    r.place_on_field(0, CID, Some(0));
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, "PAD");
    let d = sizes(&r);
    assert!(r.move_from_breeding(0));
    assert!(r.pending_selection().is_none());
    assert_eq!(sizes(&r), d);
}
#[test]
fn bt27_019_inherited_actual_attack_draws_both_once() {
    let mut r = setup(&[]);
    let h = r.place_stack(0, &[CID, "PAD"]);
    let d = sizes(&r);
    r.attack_player(h, 1, false);
    assert_eq!(sizes(&r), (d.0 - 1, d.1 - 1));
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    r.attack_player(h, 1, false);
    assert_eq!(sizes(&r), (d.0 - 1, d.1 - 1));
}
#[test]
fn bt27_019_top_card_attack_has_no_inherited_draw() {
    let mut r = setup(&[]);
    let h = r.place_on_field(0, CID, Some(0));
    let d = sizes(&r);
    r.attack_player(h, 1, false);
    assert_eq!(sizes(&r), d);
}
#[test]
fn bt27_019_other_attacker_does_not_fire_inherited() {
    let mut r = setup(&[]);
    r.place_stack(0, &[CID, "PAD"]);
    let h = r.place_on_field(0, "PAD", Some(0));
    let d = sizes(&r);
    r.attack_player(h, 1, false);
    assert_eq!(sizes(&r), d);
}
#[test]
fn bt27_019_player_can_choose_second_card_and_second_thomas() {
    let mut r = setup(&[CID, "OPTION", "PAD"]);
    let a = r.place_on_field(0, "THOMAS", Some(0));
    let b = r.place_on_field(0, "THOMAS", Some(0));
    r.play(0, 0);
    let choices = picks(&r);
    assert_eq!(choices.len(), 2);
    act(&mut r, choices[1]);
    act(&mut r, ATTACK_START + b.index as u16);
    assert_eq!(sources(&r, a), vec!["THOMAS"]);
    assert_eq!(sources(&r, b), vec!["PAD", "THOMAS"]);
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "OPTION"));
}
#[test]
fn bt27_019_inherited_opt_is_per_physical_source() {
    let mut r = setup(&[]);
    let a = r.place_stack(0, &[CID, "PAD"]);
    let b = r.place_stack(0, &[CID, "PAD"]);
    let d = sizes(&r);
    r.attack_player(a, 1, false);
    r.attack_player(b, 1, false);
    assert_eq!(sizes(&r), (d.0 - 2, d.1 - 2));
}
