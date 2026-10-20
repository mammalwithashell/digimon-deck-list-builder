//! P-138: official observation c4eb467a..., Q4244/Q4245 and card image.
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn card(id: &str, name: &str, kind: CardKind, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.colors = colors;
    c.play_cost = 2;
    c.level = Some(3);
    c
}
fn runner(deck: &[&str], hand: &[&str]) -> DebugRunner {
    // Callers specify top first; the builder stores the top at the vector end.
    let deck: Vec<_> = deck.iter().rev().copied().collect();
    DebugRunner::builder()
        .dsl_card("P-138")
        .expect("P-138 loads")
        .add_card(card(
            "V",
            "AeroVeedramon",
            CardKind::Digimon,
            vec![CardColor::Blue],
        ))
        .add_card(card(
            "T",
            "Blue Tamer",
            CardKind::Tamer,
            vec![CardColor::Blue, CardColor::Green],
        ))
        .add_card(card(
            "R",
            "Red Tamer",
            CardKind::Tamer,
            vec![CardColor::Red],
        ))
        .add_card(card(
            "N",
            "Veedramon Option",
            CardKind::Option,
            vec![CardColor::Blue],
        ))
        .add_card(card(
            "F",
            "Other Digimon",
            CardKind::Digimon,
            vec![CardColor::Blue],
        ))
        .deck(0, &deck)
        .hand(0, hand)
        .memory(10)
        .start()
}
fn pick(r: &mut DebugRunner, id: &str) {
    let index = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap();
    let action = SEL_REVEAL_START + index as u16;
    assert!(r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .contains(&action));
    r.execute_action(0, action).expect("legal reveal choice");
}
fn ids(r: &DebugRunner, hand: bool) -> Vec<String> {
    let zone = if hand {
        &r.game.players[0].hand
    } else {
        &r.game.players[0].deck
    };
    zone.iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}
#[test]
fn p_138_search_requires_both_buckets_and_bottoms_rest() {
    let mut r = runner(&["V", "T", "R", "F"], &["P-138"]);
    r.play(0, 0).unwrap();
    let view = r
        .pending_selection_view()
        .expect("mandatory Digimon bucket");
    assert_eq!(view.valid_action_ids, vec![SEL_REVEAL_START]);
    assert!(!view.valid_action_ids.contains(&PASS));
    pick(&mut r, "V");
    let view = r
        .pending_selection_view()
        .expect("mandatory blue Tamer bucket");
    assert_eq!(view.valid_action_ids.len(), 1);
    assert!(!view.valid_action_ids.contains(&PASS));
    pick(&mut r, "T");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["V", "T"]);
    assert_eq!(ids(&r, false), vec!["R", "F"]);
}
#[test]
fn p_138_search_rejects_named_option_and_red_tamer_orders_remainder() {
    let mut r = runner(&["N", "R", "T", "F"], &["P-138"]);
    r.play(0, 0).unwrap();
    let view = r.pending_selection_view().expect("only blue Tamer legal");
    assert_eq!(view.valid_action_ids, vec![SEL_REVEAL_START + 2]);
    pick(&mut r, "T");
    let view = r
        .pending_selection_view()
        .expect("bottom order is player choice");
    assert_eq!(view.valid_action_ids.len(), 2);
    pick(&mut r, "R");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["T"]);
    assert_eq!(ids(&r, false), vec!["N", "R", "F"]);
}

#[test]
fn p_138_digivolution_search_and_printed_cost() {
    use digimon_engine::enums::PlaySource;
    let mut r = runner(&["F", "V", "T", "R"], &["P-138"]);
    let base = r.place_on_field(0, "F", None);
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    assert_eq!(r.memory(), 8);
    pick(&mut r, "V");
    pick(&mut r, "T");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["F", "V", "T"]);
    assert_eq!(ids(&r, false), vec!["R"]);
}
#[test]
fn p_138_inherited_requires_self_and_is_once_per_turn() {
    let mut r = runner(&["F"; 8], &[]);
    r.game.set_memory(5);
    let host = r.place_stack(0, &["P-138", "V"]);
    let other = r.place_on_field(0, "F", None);
    r.game.suspend(other);
    r.game.unsuspend(other);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 5, "other Digimon cannot trigger or consume OPT");
    r.game.suspend(host);
    r.game.unsuspend(host);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 6);
    r.game.suspend(host);
    r.game.unsuspend(host);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 6, "second self unsuspend does not gain memory");
}
#[test]
fn p_138_inherited_requires_your_turn_and_source_scope() {
    for inherited in [false, true] {
        let mut r = runner(&["F"; 8], &[]);
        let host = if inherited {
            r.place_stack(0, &["P-138", "V"])
        } else {
            r.place_on_field(0, "P-138", None)
        };
        if inherited {
            r.set_first_player(1);
        }
        let memory = r.memory();
        r.game.suspend(host);
        r.game.unsuspend(host);
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), memory);
    }
}
#[test]
fn p_138_single_category_and_empty_deck_resolve() {
    let mut r = runner(&["V"], &["P-138"]);
    r.play(0, 0).unwrap();
    pick(&mut r, "V");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["V"]);
    assert_eq!(r.deck_size(0), 0);
    let mut r = runner(&[], &["P-138"]);
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert!(r.pending_selection().is_none());
}
