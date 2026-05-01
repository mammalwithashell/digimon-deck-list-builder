use std::sync::Arc;

use digimon_engine::action::space::{encode_source_select, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
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
                        ctx.play_selected_sources_without_cost(selected);
                        ctx.cancel_current_replacement();
                    },
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
