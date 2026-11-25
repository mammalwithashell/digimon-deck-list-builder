use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::{EffectSourceKind, EffectTiming};

#[test]
fn bt27_016_returns_a_source_to_activate_the_option_face() {
    let mut r = board("BT27-016");
    let jesmon = r.place_stack(0, &["JESMON", "BT27-016"]);
    r.place_on_field(1, "FILLER", Some(0));
    trigger(&mut r, EffectTiming::WhenDigivolving, jesmon);
    assert_eq!(r.effective_dp(jesmon), Some(18000));
    let choices = r.pending_selection_view().expect("Source-cost choice");
    let choices = choices.effect_choices.unwrap();
    let return_source = choices
        .iter()
        .find(|choice| choice.label == "Return a digivolution card")
        .unwrap();
    r.execute_action(0, return_source.action_id).unwrap();
    resolve_only_bt27_target(&mut r);
    let option = r
        .pending_selection_view()
        .expect("Choose an opponent Digimon");
    assert_eq!(option.source_kind, EffectSourceKind::Option);
    assert!(option.is_optional);
    let target = option
        .valid_action_ids
        .into_iter()
        .find(|id| *id != PASS)
        .unwrap();
    r.execute_action(0, target).unwrap();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}
