//! ST12-13: mandatory Huckmon/Royal Knight search and continuous Reboot.
//! Image and official bundle include the Sistermon Noir / Virus identity rule.
use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword};

fn builder() -> DebugRunnerBuilder {
    let mut royal = make_test_card("ROYAL", "Gallantmon");
    royal.traits = vec!["Royal Knight".into()];
    let mut option = royal.clone();
    option.card_id = "ROYAL-OPTION".into();
    option.card_kind = CardKind::Option;
    DebugRunner::builder()
        .dsl_card("ST12-13")
        .expect("ST12-13 production YAML")
        .add_card(make_test_card("HUCK", "BaoHuckmon"))
        .add_card(royal)
        .add_card(option)
        .add_card(make_test_card("MISS", "Sistermon Blanc"))
        .hand(0, &["ST12-13"])
        .memory(10)
}

#[test]
fn st12_13_identity_and_face_up_effects_match_printed_card() {
    let r = builder().start();
    let c = r.compiled_card("ST12-13").unwrap();
    assert_eq!(c.form.as_deref(), Some("Champion"));
    assert_eq!(c.attribute, ["Data", "Virus"]);
    assert!(c.also_treated_as.iter().any(|n| n == "Sistermon Noir"));
    assert_eq!(c.effects.len(), 2);
    for clause in &c.effects {
        match clause {
            CompiledClause::Triggered(t) => assert_eq!(t.scope, CompiledScope::FaceUp),
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. }) => {
                assert_eq!(*scope, CompiledScope::FaceUp)
            }
            _ => panic!("only face-up search and aura are printed"),
        }
    }
}

#[test]
fn st12_13_search_exposes_both_matches_and_trashes_unchosen_cards() {
    for pick in 0..2 {
        let mut r = builder().deck(0, &["MISS", "HUCK", "ROYAL"]).start();
        r.play(0, 0).unwrap();
        let pending = r.pending_selection().expect("search choice");
        let choices = pending.valid_action_ids.clone();
        assert_eq!(choices.len(), 2, "only name or trait matches");
        assert!(!r.pending_is_optional());
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(mask[PASS as usize], 0.0);
        for action in &choices {
            assert_eq!(mask[*action as usize], 1.0);
        }
        r.execute_action(0, choices[pick]).unwrap();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(r.trash_size(0), 2);
        assert_eq!(r.deck_size(0), 0);
        let selected = r.game.players[0].hand[0].card_id(&r.game.card_data);
        assert_eq!(selected, if pick == 0 { "ROYAL" } else { "HUCK" });
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "MISS"));
    }
}

#[test]
fn st12_13_search_accepts_royal_knight_option() {
    let mut r = builder().deck(0, &["MISS", "ROYAL-OPTION"]).start();
    r.play(0, 0).unwrap();
    let action = r.pending_selection().unwrap().valid_action_ids[0];
    r.execute_action(0, action).unwrap();
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "ROYAL-OPTION"
    );
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn st12_13_search_no_matches_and_empty_deck_finish_without_prompt() {
    for deck in [vec![], vec!["MISS"; 3]] {
        let mut r = builder().deck(0, &deck).start();
        r.play(0, 0).unwrap();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.hand_size(0), 0);
        assert_eq!(r.trash_size(0), deck.len());
    }
}

#[test]
fn st12_13_aura_targets_own_matching_digimon_and_expires_with_source() {
    let mut r = builder().start();
    let huck = r.place_on_field(0, "HUCK", None);
    let royal = r.place_on_field(0, "ROYAL", None);
    let miss = r.place_on_field(0, "MISS", None);
    let opp = r.place_on_field(1, "HUCK", None);
    r.place_on_field(0, "ST12-13", None);
    for active in [0, 1] {
        r.game.turn_player_idx = active;
        r.game.tick_declarative_effects();
        assert!(r.game.has_keyword(huck, Keyword::Reboot));
        assert!(r.game.has_keyword(royal, Keyword::Reboot));
        assert!(!r.game.has_keyword(miss, Keyword::Reboot));
        assert!(!r.game.has_keyword(opp, Keyword::Reboot));
    }
    r.game.players[0].battle_area.pop();
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(huck, Keyword::Reboot));
    assert!(!r.game.has_keyword(royal, Keyword::Reboot));
}
