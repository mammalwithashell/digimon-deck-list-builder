//! Phase 2 Track J Task S2.2 — `G-UNION-HAND-TRASH-NAME-EXCLUSION`.
//!
//! Substrate proof for the BT23-013 Jesmon family "union" play:
//!
//!   "[When Digivolving] [When Attacking] You may play 1 [Atho, René & Por]
//!    Token … or, from your hand or trash, 1 Digimon card with [Sistermon]
//!    in its name without paying the cost. **This effect can't play cards
//!    with the same names as any of your Digimon.**"
//!
//! Two substrate pieces are exercised here:
//!
//!   (a) `select_union_zone` (hand+trash in one prompt) must APPLY its
//!       `filter` predicate — the DSL lowering previously dropped it.
//!   (b) the `name_not_shared_by_field_digimon` predicate leaf — a card
//!       passes only if NO battle-area Digimon of the named player shares
//!       its card name.
//!
//! Both shape the legal action mask. They never auto-pick: every surviving
//! candidate (from hand AND trash) surfaces through `pending_selection`,
//! and the PASS path stays open for the optional play (§17).

use std::sync::Arc;

use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

fn make_card(id: &str, name: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::White],
        traits: vec!["Royal Knight".to_string()],
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

fn push_to_trash(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = r.game.next_card_index();
    r.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

/// Place a Digimon directly into a player's battle area (no play flow), so
/// the name-exclusion predicate has an in-play name to exclude against.
fn push_to_battle_area(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_battle_area: unknown card_id {card_id}"));
    let card_index = r.game.next_card_index();
    let source = CardSource::new(data_idx, player, card_index);
    let turn = r.game.turn_count;
    r.game.players[player as usize]
        .battle_area
        .push(digimon_engine::permanent::Permanent::new(source, turn));
}

/// Build the synthetic Jesmon-family vehicle card: an `on_play` effect with a
/// single `select_union_zone` step over hand+trash, restricted by name to the
/// [Sistermon] set, and additionally excluding any name already among the
/// controller's battle-area Digimon.
fn jesmon_family_vehicle_yaml() -> &'static str {
    r#"
card: S2-2-UNION
name: Union Vehicle
kind: digimon
color: [white]
level: 6
cost: 6
dp: 6000
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          prompt: "Play 1 Sistermon from hand or trash"
          filter:
            all_of:
              - name_contains: Sistermon
              - name_not_shared_by_field_digimon: you
"#
}

/// (a) + (b): with `Sistermon Blanc` ALREADY in the controller's battle
/// area, the union-zone prompt must offer the legal `[Sistermon]` cards from
/// hand AND trash and EXCLUDE the in-play `Sistermon Blanc` name, as well as
/// the non-Sistermon card. The mask is shaped; nothing auto-picks.
#[test]
fn union_zone_filter_excludes_in_play_name_across_hand_and_trash() {
    let yaml = jesmon_family_vehicle_yaml();
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("yaml compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_card("S2-2-UNION", "Union Vehicle"))
        // Hand: a Sistermon NOT in play (legal) + one already in play
        // (must be excluded) + a non-Sistermon (must be excluded by name).
        .add_card(make_card("SIS-NOIR-H", "Sistermon Noir"))
        .add_card(make_card("SIS-BLANC-H", "Sistermon Blanc"))
        .add_card(make_card("NON-SIS-H", "Gankoomon"))
        // Trash: another distinct Sistermon (legal) + a copy of the in-play
        // name (must be excluded).
        .add_card(make_card("SIS-CIEL-T", "Sistermon Ciel"))
        .add_card(make_card("SIS-BLANC-T", "Sistermon Blanc"))
        .hand(0, &["S2-2-UNION", "SIS-NOIR-H", "SIS-BLANC-H", "NON-SIS-H"])
        .build();

    push_to_trash(&mut runner, 0, "SIS-CIEL-T");
    push_to_trash(&mut runner, 0, "SIS-BLANC-T");
    // Sistermon Blanc is on the field — its name is now excluded.
    push_to_battle_area(&mut runner, 0, "SIS-BLANC-H");

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let card: CardHandle = runner.game.players[0].hand[0].handle();
    let effects = dsl_effect.effects(card);
    let process = effects[0].process.as_ref().expect("has process closure");
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");

    // Legal candidates: Sistermon Noir (hand idx 1) + Sistermon Ciel
    // (trash idx 0). Excluded: Sistermon Blanc in hand (idx 2, name in
    // play), Gankoomon in hand (idx 3, not a Sistermon), Sistermon Blanc
    // in trash (idx 1, name in play).
    let hand_noir = PLAY_HAND_START + 1;
    let hand_blanc = PLAY_HAND_START + 2;
    let hand_gankoomon = PLAY_HAND_START + 3;
    let trash_ciel = TRASH_EFFECT_START;
    let trash_blanc = TRASH_EFFECT_START + 1;

    assert!(
        pending.valid_action_ids.contains(&hand_noir),
        "Sistermon Noir in hand is a legal candidate"
    );
    assert!(
        pending.valid_action_ids.contains(&trash_ciel),
        "Sistermon Ciel in trash is a legal candidate"
    );
    assert!(
        !pending.valid_action_ids.contains(&hand_blanc),
        "Sistermon Blanc in hand must be excluded — name already in play"
    );
    assert!(
        !pending.valid_action_ids.contains(&trash_blanc),
        "Sistermon Blanc in trash must be excluded — name already in play"
    );
    assert!(
        !pending.valid_action_ids.contains(&hand_gankoomon),
        "Gankoomon must be excluded — not a [Sistermon] card"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "exactly the two distinct legal Sistermon names survive the filter"
    );
    assert!(
        pending.is_optional,
        "the play is a 'may' — PASS must stay open (§17)"
    );
}

/// With NO name on the field, the name-exclusion is inert: every
/// `[Sistermon]` card in hand and trash stays a legal candidate. Guards
/// against the predicate spuriously rejecting cards.
#[test]
fn union_zone_filter_keeps_all_sistermon_when_field_empty() {
    let yaml = jesmon_family_vehicle_yaml();
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("yaml compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_card("S2-2-UNION", "Union Vehicle"))
        .add_card(make_card("SIS-NOIR-H", "Sistermon Noir"))
        .add_card(make_card("SIS-CIEL-T", "Sistermon Ciel"))
        .hand(0, &["S2-2-UNION", "SIS-NOIR-H"])
        .build();

    push_to_trash(&mut runner, 0, "SIS-CIEL-T");

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let card: CardHandle = runner.game.players[0].hand[0].handle();
    let effects = dsl_effect.effects(card);
    let process = effects[0].process.as_ref().expect("has process closure");
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "both Sistermon (hand + trash) are legal when no name is in play"
    );
}
