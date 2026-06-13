//! Engine-level: the app-fusion commit can pull the result card from the
//! controller's **trash** (not just hand). Pins `Game::commit_effect_app_fuse`
//! + `AppFuseSourceZone`. The result card stacks on the host, the host's linked
//! cards fold under the new top as digivolution sources (consumed), and the
//! result card leaves its source zone.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::game_actions::AppFuseSourceZone;

/// App-Fusion result card whose materials are Kabemon & Gomimon (a single
/// `name_in` material — the engine flattens to distinct named conditions).
const APP_FUSION_CARD: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 9000
traits: [Appmon]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

fn named_digimon(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.colors = vec![CardColor::Yellow];
    card.level = Some(4);
    card
}

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(idx, player as u8, iid));
}

#[test]
fn app_fusion_commit_pulls_result_from_trash_and_stacks() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(APP_FUSION_CARD)
        .expect("App Fusion test card compiles")
        .add_card(named_digimon("TEST-KABEMON", "Kabemon"))
        .add_card(named_digimon("TEST-GOMIMON", "Gomimon"))
        .memory(5)
        .start();

    // Host: top = Kabemon, linked = Gomimon → 2 distinct named materials.
    let host = runner.place_on_field(0, "TEST-KABEMON", Some(0));
    runner.push_linked_owned(host, "TEST-GOMIMON", 0);

    // Result card (TEST-APPFUSE) lives in the TRASH.
    seed_trash(&mut runner, 0, "TEST-APPFUSE");
    let trash_idx = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "TEST-APPFUSE")
        .expect("result card seeded in trash");
    let result_handle = runner.game.players[0].trash[trash_idx].handle();

    let trash_before = runner.trash_size(0);

    let ok = runner
        .game
        .commit_effect_app_fuse(0, host, result_handle, AppFuseSourceZone::Trash);
    assert!(ok, "app-fusion commit from trash succeeds");

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "TEST-APPFUSE",
        "result card is the new top of the host"
    );
    // Gomimon (the consumed link) is now a digivolution source, not a link.
    assert!(
        perm.linked_cards.is_empty(),
        "host's linked cards were consumed (folded under the new top)"
    );
    assert!(
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "Gomimon"
                || c.card_id(&runner.game.card_data) == "TEST-GOMIMON"),
        "the consumed link is now a digivolution source under the new top"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "result card left the trash"
    );
}
