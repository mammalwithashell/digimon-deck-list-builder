use super::{board, trigger};
use digimon_engine::{action::space::PASS, enums::EffectTiming};

#[test]
fn bt27_084_limits_the_hand_selection_to_knightmon_text() {
    let mut r = board("BT27-084");
    let lordknightmon = r.place_on_field(0, "BT27-084", Some(0));
    r.add_to_hand(0, "KNIGHTMON");
    r.add_to_hand(0, "FILLER");
    trigger(&mut r, EffectTiming::WhenDigivolving, lordknightmon);
    let pick = r
        .pending_selection_view()
        .expect("Choose cards with Knightmon in text");
    assert!(pick.is_optional);
    assert!(pick.valid_action_ids.contains(&PASS));
    assert_eq!(r.prompt_card_candidates().len(), 1);
    r.choose_prompt_card("KNIGHTMON", None).unwrap();
}
