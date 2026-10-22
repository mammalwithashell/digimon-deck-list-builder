//! BT23-013 Jesmon.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::action::space::{decode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-013")
        .expect("BT23-013 YAML loads")
        .add_card(digimon("ALLY", "Other Ally", &[]))
        .add_card(digimon("SIS-HAND", "Sistermon Ciel", &[]))
        .add_card(digimon("SIS-TRASH", "Sistermon Blanc", &[]))
        .add_card(digimon("SEC", "Security Filler", &[]))
        .memory(20)
        .start()
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

fn accept_optional_prompt(runner: &mut DebugRunner) {
    while matches!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder | SelectionKind::Replacement)
    ) {
        let action = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("accept action");
        runner
            .execute_action(0, action)
            .expect("accept optional trigger");
    }
}

fn find_field_card(runner: &DebugRunner, card_id: &str) -> PermanentHandle {
    let index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} on field"));
    PermanentHandle {
        player: 0,
        index: index as u8,
    }
}

#[test]
fn bt23_013_has_metadata_keywords_and_authored_triggers() {
    let runner = runner();
    let card = runner.compiled_card("BT23-013").expect("compiled card");

    assert_eq!(card.name, "Jesmon");
    assert_eq!(card.form.as_deref(), Some("Mega"));
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert!(card.traits.iter().any(|name| name == "CS"));

    for keyword in ["Rush", "Alliance"] {
        assert!(card.effects.iter().any(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(
                    digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword { keyword: k, .. }
                ) if k == keyword
            )
        }));
    }
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::WhenDigivolving)
                && triggered.when.contains(&CompiledTiming::WhenAttacking)
        }
        _ => false,
    }));
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnAllyPlayed) && triggered.once_per_turn
        }
        _ => false,
    }));
}

#[test]
fn bt23_013_when_digivolving_can_play_atho_rene_por_token() {
    let mut runner = runner();
    let jesmon = runner.place_on_field(0, "BT23-013", Some(0));
    push_to_trash(&mut runner, 0, "SIS-TRASH");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(jesmon),
    );
    runner.game.drain_effect_queue();
    accept_optional_prompt(&mut runner);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    let token_branch = runner.pending_selection().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, token_branch)
        .expect("choose token branch");
    runner.auto_resolve().expect("play token");

    let token = find_field_card(&runner, "TOKEN_ATHO_RENE_POR");
    assert!(runner.game.has_keyword(token, Keyword::Reboot));
    assert!(runner.game.has_keyword(token, Keyword::Blocker));
}

#[test]
fn bt23_013_sistermon_union_play_filters_names_already_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-013")
        .expect("BT23-013 YAML loads")
        .add_card(digimon("SIS-HAND", "Sistermon Ciel", &[]))
        .add_card(digimon("SIS-TRASH", "Sistermon Blanc", &[]))
        .memory(20)
        .start();
    let jesmon = runner.place_on_field(0, "BT23-013", Some(0));
    runner.place_on_field(0, "SIS-HAND", Some(0));
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "SIS-HAND")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(data_idx, 0, next));
    push_to_trash(&mut runner, 0, "SIS-TRASH");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(jesmon),
    );
    runner.game.drain_effect_queue();
    accept_optional_prompt(&mut runner);
    let sistermon_branch = runner.pending_selection().unwrap().valid_action_ids[1];
    runner
        .execute_action(0, sistermon_branch)
        .expect("choose Sistermon branch");

    let union_prompt = runner.pending_selection_view().expect("union prompt");
    assert_eq!(
        union_prompt.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        }
    );
    assert_eq!(
        union_prompt.valid_action_ids.len(),
        1,
        "Sistermon Ciel in hand is excluded because that name is already on field"
    );
    runner
        .execute_action(0, union_prompt.valid_action_ids[0])
        .expect("choose legal Sistermon");
    runner.auto_resolve().expect("play Sistermon");

    find_field_card(&runner, "SIS-TRASH");
}

#[test]
fn bt23_013_other_digimon_play_may_attack_observer_ignores_self_play() {
    let mut runner = runner();
    let jesmon = runner.place_on_field(0, "BT23-013", Some(0));
    let card = runner.game.players[0].battle_area[jesmon.index as usize]
        .top_card()
        .handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: 0,
            permanent: jesmon,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "event_target_is_source: false keeps Jesmon from reacting to itself"
    );
}

#[test]
fn bt23_013_other_digimon_play_allows_this_jesmon_to_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-013")
        .expect("BT23-013 YAML loads")
        .add_card(digimon("ALLY", "Other Ally", &[]))
        .add_card(digimon("SEC", "Security Filler", &[]))
        .hand(0, &["ALLY"])
        .security(1, &["SEC"])
        .memory(20)
        .start();
    runner.game.turn_count = 5;
    let jesmon = runner.place_on_field(0, "BT23-013", Some(0));

    runner.play(0, 0).expect("play other ally");
    accept_optional_prompt(&mut runner);

    let attack_prompt = runner
        .pending_selection_view()
        .expect("may_attack_now opens attack targets");
    assert!(
        attack_prompt
            .valid_action_ids
            .iter()
            .copied()
            .filter(|action| *action != PASS)
            .all(|action| decode_attack(action).0 as u8 == jesmon.index),
        "the attack prompt must be for this Jesmon"
    );
}

#[test]
fn bt23_013_printed_evolution_routes_masks_and_costs() {
    for (name, level, cs, dp, expected) in [
        ("SaviorHuckmon", 5, false, 0, Some(3)),
        ("Other", 5, true, 0, Some(3)),
        ("Huckmon", 3, false, 10000, Some(5)),
        ("Huckmon", 3, false, 9999, None),
        ("Huckmon X Antibody", 3, false, 10000, None),
        ("SaviorHuckmon X Antibody", 5, false, 0, None),
        ("Other", 4, true, 10000, None),
    ] {
        let mut base = digimon("BASE", name, if cs { &["CS"] } else { &[] });
        base.colors = vec![CardColor::Blue];
        base.level = Some(level);
        let mut enemy = digimon("ENEMY", "Enemy", &[]);
        enemy.dp = Some(dp);
        let mut r = DebugRunner::builder()
            .dsl_card("BT23-013")
            .unwrap()
            .add_card(base)
            .add_card(enemy)
            .add_card(digimon("FILL", "Filler", &[]))
            .hand(0, &["BT23-013"])
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .memory(10)
            .start();
        r.place_on_field(0, "BASE", Some(0));
        if dp > 0 {
            r.place_on_field(1, "ENEMY", Some(0));
        }
        let action = digimon_engine::action::space::encode_digivolve(0, 0);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[action as usize] > 0.5,
            expected.is_some(),
            "{name}, level {level}, CS {cs}, opponent DP {dp}"
        );
        if let Some(cost) = expected {
            r.game.decode_action(action, 0);
            assert_eq!(r.memory(), 10 - cost, "{name} evolution cost");
        }
    }
}

#[test]
fn bt23_013_existing_token_masks_duplicate_but_keeps_sistermon() {
    let mut r = runner();
    let jesmon = r.place_on_field(0, "BT23-013", Some(0));
    // Give the first activation both branches; explicitly choose the token.
    push_to_trash(&mut r, 0, "SIS-TRASH");
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(jesmon),
    );
    r.game.drain_effect_queue();
    accept_optional_prompt(&mut r);
    let branch = r.pending_selection().unwrap().valid_action_ids[0];
    r.execute_action(0, branch).unwrap();
    // Decline the consequent optional attack and any other optional triggers.
    while let Some(view) = r.pending_selection_view() {
        assert!(
            digimon_engine::action::build_action_mask(&r.game, view.selecting_player)
                [PASS as usize]
                > 0.5
        );
        r.execute_action(view.selecting_player, PASS).unwrap();
    }
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(jesmon),
    );
    r.game.drain_effect_queue();
    accept_optional_prompt(&mut r);
    let view = r
        .pending_selection_view()
        .expect("Sistermon selection remains legal");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "duplicate token must not be offered: {view:?}"
    );
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert!(view
        .valid_action_ids
        .iter()
        .all(|id| mask[*id as usize] > 0.5));
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    assert_eq!(
        r.game.players[0]
            .battle_area
            .iter()
            .filter(|p| p.top_card().card_id(&r.game.card_data) == "TOKEN_ATHO_RENE_POR")
            .count(),
        1
    );
    find_field_card(&r, "SIS-TRASH");
}

#[test]
fn bt23_013_optional_play_can_be_declined() {
    let mut r = runner();
    let j = r.place_on_field(0, "BT23-013", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(j));
    r.game.drain_effect_queue();
    r.pending_selection_view().expect("optional activation");

    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

#[test]
fn bt23_013_played_digimon_snapshot_survives_its_departure() {
    let mut r = runner();
    r.game.turn_count = 5;
    let j = r.place_on_field(0, "BT23-013", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let card = r.game.players[0].battle_area[ally.index as usize]
        .top_card()
        .handle();
    r.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: 0,
            permanent: ally,
            card,
            effect_initiated: true,
        },
    );
    let removed = r.game.players[0].battle_area.remove(ally.index as usize);
    r.game.players[0].trash.extend(removed.card_sources);
    r.game.drain_effect_queue();
    accept_optional_prompt(&mut r);
    let view = r
        .pending_selection_view()
        .expect("Jesmon may attack after played ally leaves");
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert!(view
        .valid_action_ids
        .iter()
        .all(|a| mask[*a as usize] > 0.5));
    assert!(view
        .valid_action_ids
        .iter()
        .any(|a| *a != PASS && decode_attack(*a).0 as u8 == j.index));
}

#[test]
fn bt23_013_duplicate_token_and_no_sistermon_finishes_without_choice() {
    let mut r = runner();
    let j = r.place_on_field(0, "BT23-013", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(j));
    r.game.drain_effect_queue();
    accept_optional_prompt(&mut r);
    // Only token branch is possible and is selected after optional consent.
    while r.pending_selection().is_some() {
        r.decline_optional_trigger().unwrap();
    }
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(j));
    r.game.drain_effect_queue();
    accept_optional_prompt(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}

#[test]
fn bt23_013_token_play_triggers_attack_once_per_turn() {
    let mut r = runner();
    r.game.turn_count = 5;
    let j = r.place_on_field(0, "BT23-013", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(j));
    r.game.drain_effect_queue();
    accept_optional_prompt(&mut r);
    let view = r
        .pending_selection_view()
        .expect("played token must trigger Jesmon attack");
    assert!(view
        .valid_action_ids
        .iter()
        .any(|a| *a != PASS && decode_attack(*a).0 as u8 == j.index));
    r.decline_optional_trigger().unwrap();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let card = r.game.players[0].battle_area[ally.index as usize]
        .top_card()
        .handle();
    r.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: 0,
            permanent: ally,
            card,
            effect_initiated: false,
        },
    );
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "OPT consumed by first activation"
    );
}
