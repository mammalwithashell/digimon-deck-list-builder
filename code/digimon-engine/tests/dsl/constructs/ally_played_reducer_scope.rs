//! `kind: cost_reduction` + `when_any_ally_played` is a PLAY reducer
//! ("When any ... would be played from your hand, ... reduce the play cost").
//! CR 4.3 §15-7-4 (optional "by X" processing) and DCGO
//! `CanTriggerWhenPermanentWouldPlay` (ST20_12.cs / ST21_13.cs).
//!
//! - G-COST-REDUCTION-ALLY-PLAYED-ON-DIGIVOLVE: a digivolve cost is not a play,
//!   so the reducer is never offered for a digivolution.
//! - `cost_target_from_hand: true` restricts it to plays from hand: a
//!   cost-paying effect play from the trash is not offered the reducer.
//!
//! All main-phase actions are driven through the action mask.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::permanent::PermanentHandle;

fn reducer(from_hand_leaf: bool) -> String {
    let cond = if from_hand_leaf {
        "    condition: { cost_target_from_hand: true }\n"
    } else {
        ""
    };
    format!(
        r#"
card: TEST-ALLY-PLAY-REDUCER
name: Ally play reducer
kind: tamer
color: [red]
cost: 3
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when: {{ your_turn: true }}
    optional: true
    when_any_ally_played:
      all_of:
        - kind: digimon
        - trait_has: Fuel
{cond}    amount: 1
    pay_cost:
      - suspend: {{ target: source }}
"#
    )
}

struct Setup {
    r: DebugRunner,
    tamer: PermanentHandle,
    base: PermanentHandle,
}

fn setup(from_hand_leaf: bool, hand: &[&str]) -> Setup {
    let mut fuel = make_test_card("FUEL-DIGI", "Fuel Digimon");
    fuel.level = Some(4);
    fuel.play_cost = 5;
    fuel.colors = vec![CardColor::Red];
    fuel.traits = vec!["Fuel".into()];
    fuel.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 2,
    }];
    let mut base = make_test_card("BASE3", "Base");
    base.level = Some(3);
    base.colors = vec![CardColor::Red];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&reducer(from_hand_leaf))
        .expect("fixture parses")
        .add_card(fuel)
        .add_card(base)
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, hand)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(10);
    let tamer = r.place_on_field(0, "TEST-ALLY-PLAY-REDUCER", Some(0));
    let base = r.place_on_field(0, "BASE3", Some(0));
    Setup { r, tamer, base }
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[0].battle_area[h.index as usize].is_suspended
}

#[test]
fn ally_played_reducer_offered_for_masked_hand_play_and_declinable() {
    for accept in [true, false] {
        let Setup { mut r, tamer, .. } = setup(true, &["FUEL-DIGI"]);
        let before = r.memory();
        assert!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.5);
        r.game.decode_action(PLAY_HAND_START, 0);
        let p = r.pending_selection().expect("optional reducer prompt");
        assert!(p.is_optional);
        let mask = build_action_mask(&r.game, 0);
        assert!(mask[PASS as usize] > 0.5, "decline is in the action space");
        let yes = *p.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        r.execute_action(0, if accept { yes } else { PASS }).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(before - r.memory(), if accept { 4 } else { 5 });
        assert_eq!(suspended(&r, tamer), accept);
    }
}

#[test]
fn ally_played_reducer_not_offered_for_masked_digivolve() {
    for leaf in [false, true] {
        let Setup { mut r, tamer, base } = setup(leaf, &["FUEL-DIGI"]);
        let before = r.memory();
        let action = encode_digivolve(0, base.index as u16);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
        r.game.decode_action(action, 0);
        assert!(
            r.pending_selection().is_none(),
            "leaf={leaf}: no play reducer on a digivolve: {:?}",
            r.pending_selection_view()
        );
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[0].battle_area[base.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "FUEL-DIGI"
        );
        assert_eq!(before - r.memory(), 2, "printed digivolve cost");
        assert!(!suspended(&r, tamer));
    }
}

#[test]
fn cost_target_from_hand_leaf_scopes_ally_played_reducer_to_hand() {
    for leaf in [false, true] {
        let Setup { mut r, tamer, .. } = setup(leaf, &[]);
        r.inject_trash(0, "FUEL-DIGI");
        let idx = r.game.players[0].trash.len() - 1;
        let before = r.memory();
        let _ = r.game.play_from_trash(0, idx);
        if leaf {
            assert!(r.pending_selection().is_none(), "trash play is not from hand");
            r.auto_resolve().unwrap();
            assert_eq!(before - r.memory(), 5);
            assert!(!suspended(&r, tamer));
        } else {
            // Control: without the leaf the reducer chain does see the play.
            assert!(r.pending_selection().is_some(), "control: offered without leaf");
        }
    }
}
