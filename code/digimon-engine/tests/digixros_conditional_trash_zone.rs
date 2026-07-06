//! Gap 4 — conditional trash-origin enablement for DigiXros (BT18-065).
//!
//! "While you have no Digimon other than [Vemmon], cards in your trash can also
//! be placed for this card's DigiXros." Proven via an inline DigiXros card with
//! the new `extra_material_zones: [{ zone: trash, while: <no Digimon other than
//! Vemmon> }]` alt-path key. The `while:` predicate is evaluated once at
//! transaction build: when it holds, a trash card is exposed as a legal
//! DigiXros material action; when it does not, it is not.

use digimon_engine::action::space::TRASH_EFFECT_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlayerId};

fn conditional_trash_digixros_yaml() -> &'static str {
    r#"
card: DX-COND-TRASH
name: Conditional Trash Xros
kind: digimon
level: 4
color: [black]
cost: 6
dp: 6000
traits: [LIBERATOR]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_contains: "Vemmon" }
        repeat: unbounded
        zones: [battle_area]
        cost_delta: -1
    cost: 6
    extra_material_zones:
      - zone: trash
        while:
          no_permanent:
            of: you
            zone: [battle_area]
            kind: digimon
            none_of:
              - name_contains: "Vemmon"
"#
}

fn make_vemmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn make_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn push_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown {card_id}"));
    let next = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next);
    runner.game.players[player as usize].trash.push(card);
}

/// Condition MET — the only own Digimon is a Vemmon ("no Digimon other than
/// [Vemmon]" holds): a [Vemmon] in trash is a legal DigiXros material.
#[test]
fn trash_material_exposed_when_only_vemmon_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(conditional_trash_digixros_yaml())
        .expect("inline conditional-trash DigiXros compiles")
        .add_card(make_vemmon("VEMMON-FIELD"))
        .add_card(make_vemmon("VEMMON-TRASH"))
        .memory(10)
        .hand(0, &["DX-COND-TRASH"])
        .start();
    runner.place_on_field(0, "VEMMON-FIELD", None);
    push_to_trash(&mut runner, 0, "VEMMON-TRASH");

    assert_eq!(runner.play(0, 0), None, "DigiXros play installs a material prompt");
    let prompt = runner
        .pending_selection()
        .expect("DigiXros material selection installs");
    assert!(
        prompt.valid_action_ids.contains(&TRASH_EFFECT_START),
        "with only [Vemmon] on the field, the [Vemmon] in trash is a legal DigiXros material"
    );
}

/// Condition NOT MET — a non-Vemmon own Digimon is present: the trash [Vemmon]
/// is NOT a legal DigiXros material (only the static battle_area zone remains).
#[test]
fn trash_material_hidden_when_non_vemmon_digimon_present() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(conditional_trash_digixros_yaml())
        .expect("compiles")
        .add_card(make_vemmon("VEMMON-FIELD"))
        .add_card(make_digimon("AGUMON", "Agumon"))
        .add_card(make_vemmon("VEMMON-TRASH"))
        .memory(10)
        .hand(0, &["DX-COND-TRASH"])
        .start();
    runner.place_on_field(0, "VEMMON-FIELD", None);
    runner.place_on_field(0, "AGUMON", None);
    push_to_trash(&mut runner, 0, "VEMMON-TRASH");

    assert_eq!(runner.play(0, 0), None, "DigiXros play installs a material prompt");
    let prompt = runner
        .pending_selection()
        .expect("DigiXros material selection installs");
    assert!(
        !prompt.valid_action_ids.contains(&TRASH_EFFECT_START),
        "a non-[Vemmon] Digimon breaks the condition — the trash material is not exposed"
    );
}
