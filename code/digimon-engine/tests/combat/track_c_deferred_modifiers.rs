use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType, PlaySource};
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload, PlayerModifierEntry};

fn card(
    card_id: &str,
    kind: CardKind,
    level: Option<u8>,
    color: CardColor,
    dp: Option<i32>,
) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, card_id);
    d.card_kind = kind;
    d.level = level;
    d.colors = vec![color];
    d.dp = dp;
    d
}

#[test]
fn payload_identity_modifiers_feed_synth_identity() {
    let mut r = DebugRunner::builder()
        .add_card(card(
            "BASE",
            CardKind::Digimon,
            Some(3),
            CardColor::Red,
            Some(2000),
        ))
        .start();
    let h = r.place_on_field(0, "BASE", Some(0));

    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeTraits, 0, Expiry::Permanent, 0).with_payload(
            ModifierPayload::Traits {
                add: vec!["Holy".to_string()],
                replace: false,
            },
        ),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeBaseCardColor, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::Colors {
                value: vec![CardColor::Yellow],
                base: true,
            }),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::Name {
                value: "Aliasmon".to_string(),
                base: true,
            }),
    );

    let perm = &r.game.player(0).battle_area[h.index as usize];
    assert!(perm.has_trait_for_rules("holy", &r.game.card_data, &r.game.modifiers, h));
    assert_eq!(
        perm.colors_for_rules(&r.game.card_data, &r.game.modifiers, h),
        vec![CardColor::Yellow]
    );
    assert!(perm.contains_card_name_for_rules("Alias", &r.game.card_data, &r.game.modifiers, h));
}

#[test]
fn treat_as_digimon_enables_attack_dp_and_digivolve_material() {
    let mut evo = card(
        "EVO",
        CardKind::Digimon,
        Some(5),
        CardColor::Yellow,
        Some(7000),
    );
    evo.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 2,
    }];
    let mut r = DebugRunner::builder()
        .add_card(card("TAMER", CardKind::Tamer, None, CardColor::White, None))
        .add_card(evo)
        .hand(0, &["EVO"])
        .memory(5)
        .start();
    let h = r.place_on_field(0, "TAMER", Some(0));
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::TreatAsDigimon, 0, Expiry::Permanent, 0).with_payload(
            ModifierPayload::SynthIdentity {
                kind: CardKind::Digimon,
                level: 4,
                colors: vec![CardColor::Yellow],
                traits: vec!["Holy".to_string()],
                dp: 5000,
            },
        ),
    );

    assert!(r.game.can_attack(h, false));
    assert_eq!(r.game.effective_dp(h), Some(5000));
    assert!(r
        .game
        .digivolve_from_hand(0, 0, h.index as usize, PlaySource::ByHand));
}

#[test]
fn change_card_dp_and_security_attack_are_consulted() {
    let mut r = DebugRunner::builder()
        .add_card(card(
            "ATK",
            CardKind::Digimon,
            Some(4),
            CardColor::Red,
            Some(2000),
        ))
        .start();
    let h = r.place_on_field(0, "ATK", Some(0));
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeCardDP, 0, Expiry::Permanent, 0).with_payload(
            ModifierPayload::Dp {
                value: 6000,
                base: true,
                origin: false,
            },
        ),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeSAttack, 0, Expiry::Permanent, 0).with_payload(
            ModifierPayload::SecurityAttack {
                delta: -1,
                invert: false,
            },
        ),
    );

    assert_eq!(r.game.effective_dp(h), Some(6000));
    assert_eq!(
        r.attack_player(h, 1, true),
        digimon_engine::combat::AttackResult::SecurityCheckSurvived
    );
}

#[test]
fn player_scoped_trash_and_cost_reduction_locks_apply() {
    let mut r = DebugRunner::builder()
        .add_card(card(
            "BODY",
            CardKind::Digimon,
            Some(3),
            CardColor::Red,
            Some(2000),
        ))
        .start();
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BODY")
        .unwrap();
    let card_index = r.game.next_card_index();
    r.game
        .player_mut(0)
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, 0, card_index,
        ));
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayFromTrash,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
    assert_eq!(r.game.play_from_trash(0, 0), None);

    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(
            ModifierType::OpponentCannotReduceDigivolveCost,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(
            ModifierType::CannotReducePlayCost,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    assert!(r
        .game
        .modifiers
        .any_player_has(ModifierType::CannotReducePlayCost));
    assert!(r
        .game
        .modifiers
        .any_other_player_has(0, ModifierType::OpponentCannotReduceDigivolveCost));
}

#[test]
fn change_permanent_level_and_origin_dp_overlay_identity() {
    let mut r = DebugRunner::builder()
        .add_card(card(
            "BASE",
            CardKind::Digimon,
            Some(3),
            CardColor::Red,
            Some(2000),
        ))
        .start();
    let h = r.place_on_field(0, "BASE", Some(0));

    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangePermanentLevel, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::LevelOverride {
                value: 2,
                delta: true,
            }),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeOriginDP, 0, Expiry::Permanent, 0).with_payload(
            ModifierPayload::Dp {
                value: 7000,
                base: true,
                origin: true,
            },
        ),
    );

    let perm = &r.game.player(0).battle_area[h.index as usize];
    assert_eq!(
        perm.level_for_rules(&r.game.card_data, &r.game.modifiers, h),
        Some(5)
    );
    assert_eq!(r.game.effective_dp(h), Some(7000));
}

#[test]
fn digixros_alias_and_link_deltas_are_queryable() {
    let mut r = DebugRunner::builder()
        .add_card(card(
            "BASE",
            CardKind::Digimon,
            Some(3),
            CardColor::Red,
            Some(2000),
        ))
        .start();
    let h = r.place_on_field(0, "BASE", Some(0));

    r.game.modifiers.add(
        h,
        ModifierEntry::simple(
            ModifierType::ChangeCardNamesForDigiXros,
            0,
            Expiry::Permanent,
            0,
        )
        .with_payload(ModifierPayload::DigiXrosNames {
            aliases: vec!["Shoutmon".to_string()],
        }),
    );
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::ChangeLinkCost, 0, Expiry::Permanent, None, 0)
            .with_payload(ModifierPayload::LinkCost { delta: -1 }),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeLinkMax, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::LinkMax { delta: 2 }),
    );

    assert!(r.game.permanent_can_satisfy_digixros_name(h, "Shoutmon"));
    assert_eq!(r.game.modifiers.link_cost_delta_for_player(0), -1);
    assert_eq!(r.game.modifiers.link_max_delta(h), 2);
}

#[test]
fn end_turn_min_memory_clamps_before_rotation() {
    let mut r = DebugRunner::builder()
        .add_card(card(
            "BODY",
            CardKind::Digimon,
            Some(3),
            CardColor::Red,
            Some(2000),
        ))
        .start();
    r.game.memory = -3;
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::ChangeEndTurnMinMemory,
            0,
            Expiry::Permanent,
            None,
            0,
        )
        .with_payload(ModifierPayload::EndTurnMinMemory { value: 0 }),
    );
    r.game.end_turn();
    assert_eq!(r.game.memory, 0);
}
