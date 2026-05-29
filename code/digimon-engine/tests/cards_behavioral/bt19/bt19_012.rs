use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT19-012";

fn trait_digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 4);
    card.colors = vec![CardColor::Red];
    card.traits = traits
        .iter()
        .map(|trait_name| (*trait_name).to_string())
        .collect();
    card
}

fn opponent(card_id: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.dp = Some(dp);
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

fn trash_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in trash"))
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-012 YAML loads")
        .add_card(trait_digimon("LV4-XROS", "Xros Level 4", &["Xros Heart"]))
        .add_card(trait_digimon("SHOUTMON-BASE", "Shoutmon", &[]))
        .add_card(trait_digimon("BLUE-FLARE", "Blue Flare", &["Blue Flare"]))
        .add_card(opponent("OPP-6000", 6000))
        .add_card(opponent("OPP-3000", 3000))
        .add_card(tamer("XROS-TAMER"))
        .memory(10)
        .hand(0, &["BLUE-FLARE"])
        .start()
}

#[test]
fn bt19_012_declares_shoutmon_digixros_alias_and_alt_paths() {
    let runner = runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT19-012 compiled card present");

    assert_eq!(card.digixros_aliases, vec!["Shoutmon".to_string()]);
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|filter| filter.name_contains.as_deref() == Some("Shoutmon"))
        }),
        "OmniShoutmon should digivolve from Shoutmon for 4"
    );
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|filter| {
                    filter.level_eq == Some(4) && filter.trait_has.as_deref() == Some("Xros Heart")
                })
        }),
        "OmniShoutmon should digivolve from Lv.4 Xros Heart for 3"
    );
}

#[test]
fn bt19_012_on_play_reduces_dp_then_deletes_three_thousand_or_less() {
    let mut runner = runner();
    let omni = runner.place_on_field(0, CARD_ID, Some(0));
    let large = runner.place_on_field(1, "OPP-6000", Some(0));
    runner.place_on_field(1, "OPP-3000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(omni));
    runner.game.drain_effect_queue();

    choose_permanent(&mut runner, large, "choose DP -3000 target");
    choose_permanent(&mut runner, large, "choose now-3000 DP target for deletion");
    runner.auto_resolve().expect("finish OmniShoutmon On Play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|permanent| permanent.top_card().card_id(&runner.game.card_data) != "OPP-6000"),
        "the opponent Digimon reduced to 3000 DP should be deleted"
    );
}

#[test]
fn bt19_012_on_deletion_places_xros_or_blue_flare_from_hand_or_trash_under_tamer() {
    let mut runner = runner();
    push_to_trash(&mut runner, 0, "LV4-XROS");
    let omni = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner
        .game
        .delete_permanent_with_cause(omni, ReplacementCause::OpponentEffect);

    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("BT19-012 should prompt through its on-deletion flow");
        if matches!(
            view.kind,
            SelectionKind::UnionZone { .. } | SelectionKind::Hand | SelectionKind::Trash
        ) {
            let hand_action = PLAY_HAND_START;
            let trash_action = TRASH_EFFECT_START + trash_index(&runner, 0, "LV4-XROS") as u16;
            assert!(view.valid_action_ids.contains(&hand_action));
            assert!(view.valid_action_ids.contains(&trash_action));
            runner
                .execute_action(0, trash_action)
                .expect("select trash Xros Heart card");
            break;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != digimon_engine::action::space::PASS)
            .expect("intermediate prompt should have a non-PASS action");
        runner.execute_action(0, action).expect("advance prompt");
    }
    runner.auto_resolve().expect("finish on-deletion flow");

    let tamer_sources = &runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "XROS-TAMER")
        .expect("XROS-TAMER should remain in battle area")
        .card_sources;
    assert_eq!(tamer_sources.len(), before + 1);
    assert!(tamer_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "LV4-XROS"));
}

#[test]
fn bt19_012_inherited_grants_rush_to_xros_heart_carrier() {
    let mut runner = runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "LV4-XROS"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Rush),
        "OmniShoutmon inherited text should grant Rush to a Xros Heart carrier"
    );
}

fn choose_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_attack(0, handle.index as u16);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}
