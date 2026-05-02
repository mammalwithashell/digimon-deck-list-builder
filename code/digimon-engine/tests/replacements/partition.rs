use std::sync::Arc;

use digimon_engine::action::space::{encode_source_select, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::replacement::ReplacementCause;

fn colored_card(id: &str, color: CardColor, level: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

struct PaildramonPartition;

impl CardEffect for PaildramonPartition {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Partition BT16-025")
            .optional()
            .replacement_condition(|ctx, _subject| {
                ctx.replacement_cause() == Some(ReplacementCause::OpponentEffect)
            })
            .replacement_process(|rctx| {
                rctx.effect.select_partition_sources(
                    rctx.subject
                        .permanent()
                        .expect("Partition subject is a permanent"),
                    "Partition BT16-025",
                    vec![
                        digimon_engine::effect_context::PartitionRequirement::new(
                            "Blue Lv.4",
                            |game, source| {
                                game.card(source.card).is_color(CardColor::Blue)
                                    && game.card(source.card).level == Some(4)
                            },
                        ),
                        digimon_engine::effect_context::PartitionRequirement::new(
                            "Green Lv.4",
                            |game, source| {
                                game.card(source.card).is_color(CardColor::Green)
                                    && game.card(source.card).level == Some(4)
                            },
                        ),
                    ],
                    move |ctx, selected| {
                        if ctx.play_selected_sources_without_cost(selected) {
                            ctx.cancel_current_replacement();
                        }
                    },
                );
            })
            .build()]
    }
}

struct OnPlayInstallEffectPlayFloodgate;

impl CardEffect for OnPlayInstallEffectPlayFloodgate {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Install effect-play floodgate")
            .process(|ctx| {
                ctx.game.modifiers.add_player_modifier(
                    ctx.player,
                    PlayerModifierEntry::simple(
                        ModifierType::CannotPlayDigimonByEffect,
                        0,
                        Expiry::EndOfTurn,
                        None,
                        ctx.player,
                    ),
                );
            })
            .build()]
    }
}

#[test]
fn bt16_025_partition_requires_one_each_matching_source() {
    let mut dual = colored_card("DUAL-LV4", CardColor::Blue, 4);
    dual.colors.push(CardColor::Green);

    {
        let mut r = DebugRunner::builder()
            .add_card(colored_card("BT16-025", CardColor::Blue, 5))
            .add_card(dual)
            .start();
        r.register_effect("BT16-025", Arc::new(PaildramonPartition));

        let single_dual = r.place_stack(0, &["DUAL-LV4", "BT16-025"]);
        r.game
            .delete_permanent_with_cause(single_dual, ReplacementCause::OpponentEffect);
        r.game
            .resolve_selection(0, REPLACEMENT_ACCEPT)
            .expect("accept Partition with only one dual-matching source");
        assert!(
            r.game.pending_selection.is_none(),
            "one dual-matching source must not install a valid two-requirement source selection"
        );
        assert_eq!(
            r.game.players[0].battle_area.len(),
            0,
            "without one distinct source per requirement, the original deletion proceeds"
        );
    }

    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-LV4", CardColor::Green, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition));

    let carrier = r.place_stack(0, &["BLUE-LV4", "GREEN-LV4", "BT16-025"]);
    let memory_before = r.game.memory;
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Partition");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("first source pick");
    assert_eq!(pending.selecting_player, 0);
    assert!(
        !pending.is_optional,
        "after accepting Partition, exact source requirements are mandatory"
    );
    assert_eq!(
        pending.valid_action_ids,
        vec![
            encode_source_select(0, 0).unwrap(),
            encode_source_select(0, 1).unwrap()
        ],
        "both matching sources are offered; the top card is not"
    );

    r.game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("pick blue source");
    r.game
        .resolve_selection(0, encode_source_select(0, 1).unwrap())
        .expect("pick green source");

    let field_ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        field_ids,
        vec![
            "BT16-025".to_string(),
            "BLUE-LV4".to_string(),
            "GREEN-LV4".to_string()
        ],
        "accepted Partition keeps the original carrier and plays the selected sources"
    );
    assert_eq!(r.game.memory, memory_before, "Partition plays without cost");
    assert!(
        r.game.players[0].trash.is_empty(),
        "selected sources are played from the stack, not trashed"
    );
}

#[test]
fn bt16_025_partition_decline_allows_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-LV4", CardColor::Green, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition));

    let carrier = r.place_stack(0, &["BLUE-LV4", "GREEN-LV4", "BT16-025"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let pending = r.game.pending_selection.as_ref().expect("outer accept");
    assert_eq!(pending.selecting_player, 0);
    assert_eq!(pending.valid_action_ids, vec![REPLACEMENT_ACCEPT]);

    r.game
        .resolve_selection(0, PASS)
        .expect("decline Partition");

    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "declining Partition allows the original deletion to proceed"
    );
    let trash_ids: Vec<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(trash_ids.len(), 3, "carrier and both sources move to trash");
    assert!(trash_ids.contains(&"BT16-025".to_string()));
    assert!(trash_ids.contains(&"BLUE-LV4".to_string()));
    assert!(trash_ids.contains(&"GREEN-LV4".to_string()));
}

#[test]
fn bt16_025_partition_masks_picks_that_cannot_complete_requirements() {
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-A", CardColor::Blue, 4))
        .add_card(colored_card("BLUE-B", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-LV4", CardColor::Green, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition));

    let carrier = r.place_stack(0, &["BLUE-A", "BLUE-B", "GREEN-LV4", "BT16-025"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Partition");

    r.game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("pick first blue source");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("second source pick");
    assert_eq!(
        pending.valid_action_ids,
        vec![encode_source_select(0, 2).unwrap()],
        "after a Blue-only source is picked, only a Green-completing source remains selectable"
    );
}

#[test]
fn bt16_025_partition_failed_source_play_does_not_cancel_or_strand_cards() {
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-LV4", CardColor::Green, 4))
        .add_card(colored_card("FILLER", CardColor::Red, 3))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition));

    let carrier = r.place_stack(0, &["BLUE-LV4", "GREEN-LV4", "BT16-025"]);
    for _ in 0..13 {
        r.place_on_field(0, "FILLER", None);
    }
    assert_eq!(
        r.game.players[0].battle_area.len(),
        r.game.rules.field_slots as usize,
        "precondition: battle area is full, so Partition cannot play two sources"
    );

    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Partition");
    r.game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("pick blue source");
    r.game
        .resolve_selection(0, encode_source_select(0, 1).unwrap())
        .expect("pick green source");

    assert_eq!(
        r.game.players[0].battle_area.len(),
        13,
        "failed Partition source play must leave the original deletion uncancelled"
    );
    assert!(
        r.game.players[0].hand.is_empty(),
        "failed Partition must not strand selected sources in hand"
    );
    let trash_ids: Vec<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    for id in ["BT16-025", "BLUE-LV4", "GREEN-LV4"] {
        assert!(
            trash_ids.contains(&id.to_string()),
            "{id} should be trashed by the original deletion when Partition play fails"
        );
    }
}

#[test]
fn bt16_025_partition_places_all_sources_before_draining_on_play_effects() {
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-LV4", CardColor::Green, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition));
    r.register_effect("BLUE-LV4", Arc::new(OnPlayInstallEffectPlayFloodgate));

    let carrier = r.place_stack(0, &["BLUE-LV4", "GREEN-LV4", "BT16-025"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Partition");
    r.game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("pick blue source");
    r.game
        .resolve_selection(0, encode_source_select(0, 1).unwrap())
        .expect("pick green source");

    let field_ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        field_ids,
        vec![
            "BT16-025".to_string(),
            "BLUE-LV4".to_string(),
            "GREEN-LV4".to_string()
        ],
        "Partition must place every selected source before the first source's OnPlay floodgate drains"
    );
    assert!(
        r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "the first source's OnPlay still drains after the batch placement"
    );
    assert!(
        r.game.players[0].hand.is_empty(),
        "batch placement must not strand the second selected source in hand"
    );
}
