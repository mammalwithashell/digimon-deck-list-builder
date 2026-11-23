use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::{EffectTiming, Keyword};

#[test]
fn bt27_015_deletes_within_the_chosen_dp_limit() {
    let mut r = board("BT27-015");
    let ouryumon = r.place_on_field(0, "BT27-015", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    assert!(r.game.has_keyword(ouryumon, Keyword::Piercing));
    trigger(&mut r, EffectTiming::OnPlay, ouryumon);
    resolve_only_bt27_target(&mut r);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(1), 0);
}

#[test]
fn bt27_015_assembly_reduces_cost_with_three_chronicle_levels() {
    let mut r = board("BT27-015");
    r.game.memory = 9;
    let hand = r.add_to_hand(0, "BT27-015");
    for id in ["BT27-008", "BT27-009", "BT27-013", "LEVEL4"] {
        r.inject_trash(0, id);
    }
    let play = PLAY_HAND_START + hand as u16;
    assert!(digimon_engine::build_action_mask(&r.game, 0)[play as usize] > 0.5);
    r.game.decode_action(play, 0);
    for expected in ["BT27-013", "BT27-009", "BT27-008"] {
        let prompt = r
            .pending_selection_view()
            .expect("Choose an Assembly material");
        assert!(prompt.prompt.contains("Assembly"), "{prompt:?}");
        let candidates = r.prompt_card_candidates();
        assert_eq!(candidates.len(), 1, "{prompt:?}");
        assert_eq!(candidates[0].1, expected);
        r.execute_action(0, candidates[0].0).unwrap();
    }
    assert_eq!(r.game.memory, 2);
    let stack = &r.game.players[0].battle_area[0].card_sources;
    assert_eq!(stack.len(), 4);
    assert_eq!(stack.last().unwrap().card_id(&r.game.card_data), "BT27-015");
}
