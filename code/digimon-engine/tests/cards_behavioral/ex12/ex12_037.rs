use digimon_engine::enums::CardColor;

use super::support::{field_contains, plain_digimon, select_first_non_pass, DebugRunner};

const CARD_ID: &str = "EX12-037";

fn choose_pending_index(runner: &mut DebugRunner, index: usize) {
    let (player, action) = {
        let view = runner
            .pending_selection_view()
            .expect("expected pending selection");
        (view.selecting_player, view.valid_action_ids[index])
    };
    runner
        .execute_action(player, action)
        .expect("choose pending action");
}

#[test]
fn ex12_037_when_digivolving_repeats_mode_per_five_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-037 YAML loads")
        .add_card(plain_digimon("MAT1", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("MAT2", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("MAT3", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("MAT4", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("MAT5", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("RECOVER", CardColor::Yellow, 3, 2000))
        .deck(0, &["RECOVER"])
        .security(1, &["SEC"])
        .start();

    let omnimon = runner.place_stack(0, &["MAT1", "MAT2", "MAT3", "MAT4", "MAT5", CARD_ID]);
    runner.place_on_field(1, "OPP", Some(0));
    let own_security_before = runner.security_count(0);

    super::support::fire_when_digivolving(&mut runner, omnimon);
    select_first_non_pass(&mut runner);
    assert!(!field_contains(&runner, 1, "OPP"));

    choose_pending_index(&mut runner, 1);
    runner.auto_resolve().expect("resolve repeated mode");

    assert_eq!(runner.security_count(1), 0, "mode 1 trashes opponent security");
    assert_eq!(
        runner.security_count(0),
        own_security_before + 1,
        "mode 1 recovers after trashing security"
    );
}
