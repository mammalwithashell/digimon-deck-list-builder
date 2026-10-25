use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn material_save_yaml(value: u8) -> String {
    format!(
        r#"
card: MS-TEST
name: Material Save Test
kind: digimon
level: 5
color: [red]
cost: 8
dp: 8000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: {{ name_contains: Shoutmon }}
        repeat: {{ min: 1, max: 1 }}
      - filter: {{ name_contains: Ballistamon }}
        repeat: {{ min: 1, max: 1 }}
    cost:
      formula:
        base: 8
        per: material_count
        delta: -2
effects:
  - kind: grant_keyword
    keyword: MaterialSave
    value: {value}
"#
    )
}

fn xros_material(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 3);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn tamer_card(card_id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.colors = vec![CardColor::Red];
    card
}

fn runner_with_material_save(value: u8, include_tamer: bool) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(&material_save_yaml(value))
        .expect("compile Material Save fixture")
        .add_card(xros_material("SRC-SHOUT", "Shoutmon"))
        .add_card(xros_material("SRC-BALLISTA", "Ballistamon"))
        .add_card(xros_material("SRC-OTHER", "Unrelated"));
    if include_tamer {
        builder = builder.add_card(tamer_card("TAMER"));
    }
    builder.memory(10).start()
}

fn place_material_save_stack(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    runner.place_stack(0, &["SRC-SHOUT", "SRC-BALLISTA", "SRC-OTHER", "MS-TEST"])
}

fn top_ids_under_tamer(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "TAMER")
        .expect("Tamer remains on field")
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn material_save_does_not_emit_main_phase_action() {
    let mut runner = runner_with_material_save(2, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    let material_save_main_bit = FIELD_EFFECT_START
        + carrier.index as u16 * digimon_engine::action::space::EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(
        mask[material_save_main_bit as usize], 0.0,
        "Material Save must not be surfaced as a main-phase field activation"
    );
}

#[test]
fn material_save_decline_leaves_sources_in_trash() {
    let mut runner = runner_with_material_save(2, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let prompt = runner
        .pending_selection()
        .expect("Material Save Tamer prompt should be offered");
    assert_eq!(prompt.kind, SelectionKind::OwnField);
    assert!(prompt.is_optional);

    runner
        .execute_action(0, PASS)
        .expect("decline Material Save");
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        top_ids_under_tamer(&runner),
        vec!["TAMER".to_string()],
        "declining Material Save leaves the Tamer stack unchanged"
    );
    let saved_source_ids: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(saved_source_ids.iter().any(|id| id == "SRC-SHOUT"));
    assert!(saved_source_ids.iter().any(|id| id == "SRC-BALLISTA"));
}

// general_rule.pdf Ver. 4.3 §16-21-2: <Material Save> is an IMMEDIATE-type
// effect ("when the Digimon with this effect WOULD be deleted"), so it is
// offered while the carrier still stands (slot 0, the Tamer in slot 1) and
// the cards are picked from the carrier's live digivolution cards, not from
// the trash. (This test used to pin the post-deletion, from-trash timing:
// G-ENGINE-MATERIAL-SAVE-AFTER-DELETION.)
#[test]
fn material_save_filters_recipe_sources_and_enforces_cap() {
    let mut runner = runner_with_material_save(1, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let tamer_prompt = runner
        .pending_selection()
        .expect("Material Save should ask for a Tamer destination");
    assert!(tamer_prompt.is_optional);
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "MS-TEST",
        "the carrier still stands while Material Save is offered (§16-21-2)"
    );
    assert!(
        tamer_prompt.valid_action_ids.contains(&encode_attack(0, 1)),
        "the Tamer (slot 1, beside the still-standing carrier) is selectable"
    );
    runner
        .execute_action(0, encode_attack(0, 1))
        .expect("choose Tamer destination");

    let source_prompt = runner
        .pending_selection()
        .expect("Material Save should ask for eligible sources");
    assert!(matches!(
        source_prompt.kind,
        SelectionKind::CountCappedMultiSelect { .. }
    ));
    let valid_source_ids: Vec<_> = source_prompt
        .valid_action_ids
        .iter()
        .map(|action| picked_source_id(&runner, carrier, *action))
        .collect();
    assert!(valid_source_ids.iter().any(|id| id == "SRC-SHOUT"));
    assert!(valid_source_ids.iter().any(|id| id == "SRC-BALLISTA"));
    assert!(
        !valid_source_ids.iter().any(|id| id == "SRC-OTHER"),
        "source selection must be filtered by the carrier's DigiXros recipe"
    );

    let first_pick = source_prompt.valid_action_ids[0];
    runner
        .execute_action(0, first_pick)
        .expect("select one Material Save source");
    assert!(
        runner.pending_selection().is_none(),
        "Material Save 1 should commit immediately after one source"
    );
    let tamer_sources = top_ids_under_tamer(&runner);
    let saved_count = tamer_sources
        .iter()
        .filter(|id| *id == "SRC-SHOUT" || *id == "SRC-BALLISTA")
        .count();
    assert_eq!(saved_count, 1);
    assert!(!tamer_sources.iter().any(|id| id == "SRC-OTHER"));
    // Non-cancelling: the deletion then proceeds with the rest of the stack.
    let trash_ids: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(trash_ids.iter().any(|id| id == "MS-TEST"), "{trash_ids:?}");
    assert!(
        trash_ids.iter().any(|id| id == "SRC-OTHER"),
        "{trash_ids:?}"
    );
    assert_eq!(
        trash_ids
            .iter()
            .filter(|id| *id == "SRC-SHOUT" || *id == "SRC-BALLISTA")
            .count(),
        1,
        "the unplaced recipe card is trashed with the carrier: {trash_ids:?}"
    );
}

#[test]
fn material_save_prompts_clone_faithfully() {
    let mut runner = runner_with_material_save(1, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let tamer_action = {
        let pending = runner
            .pending_selection()
            .expect("Material Save should ask for a Tamer destination");
        assert!(
            runner.game.pending_selection_resume.is_some(),
            "Material Save Tamer prompt must be resume-driven before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut clone_at_tamer = runner.game.clone();
    clone_at_tamer
        .resolve_selection(0, tamer_action)
        .expect("cloned Tamer prompt resolves");
    assert!(
        clone_at_tamer.pending_selection_resume.is_some(),
        "cloned Tamer resolution should install a resume-driven source prompt"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone must leave the original Tamer prompt intact"
    );

    runner
        .execute_action(0, tamer_action)
        .expect("choose Tamer destination");
    let source_action = {
        let pending = runner
            .pending_selection()
            .expect("Material Save should ask for eligible sources");
        assert!(
            runner.game.pending_selection_resume.is_some(),
            "Material Save source prompt must be resume-driven before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut clone_at_source = runner.game.clone();
    clone_at_source
        .resolve_selection(0, source_action)
        .expect("cloned source prompt resolves");
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the source clone must leave the original source prompt intact"
    );
    runner
        .execute_action(0, source_action)
        .expect("select Material Save source");

    let cloned_saved = clone_at_source.players[0].battle_area[0]
        .card_sources
        .iter()
        .filter(|card| {
            let id = card.card_id(&clone_at_source.card_data);
            id == "SRC-SHOUT" || id == "SRC-BALLISTA"
        })
        .count();
    let original_saved = runner.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .filter(|card| {
            let id = card.card_id(&runner.game.card_data);
            id == "SRC-SHOUT" || id == "SRC-BALLISTA"
        })
        .count();
    assert_eq!(cloned_saved, original_saved);
}

#[test]
fn material_save_skips_when_no_tamer_or_no_eligible_source_exists() {
    let mut no_tamer = runner_with_material_save(2, false);
    let carrier = place_material_save_stack(&mut no_tamer);
    no_tamer
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(no_tamer.pending_selection().is_none());

    let mut no_eligible = runner_with_material_save(2, true);
    let carrier = no_eligible.place_stack(0, &["SRC-OTHER", "MS-TEST"]);
    no_eligible.place_on_field(0, "TAMER", Some(0));
    no_eligible
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(no_eligible.pending_selection().is_none());
}

/// The card of the carrier's digivolution stack a `SOURCE_SELECT` pick action
/// names, asserting the action addresses `carrier`'s own slot.
fn picked_source_id(
    runner: &DebugRunner,
    carrier: digimon_engine::permanent::PermanentHandle,
    action: u16,
) -> String {
    let pending = runner.game.pending_selection.as_ref().expect("source prompt");
    let view = pending.card_choice.as_ref().expect("owned source prompt").view(&runner.game, pending.selecting_player);
    let row = view.rows.iter().find(|row| row.action_id == action).expect("presented source row");
    assert_eq!(row.owner, Some(carrier.player));
    assert_eq!(row.carrier_slot, Some(carrier.index), "source row belongs to original carrier");
    row.card_id.clone().expect("own source identity is visible")
}

/// The field-pick action for the (only non-token) `TAMER`.
fn tamer_action(runner: &DebugRunner) -> u16 {
    let slot = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "TAMER")
        .expect("Tamer on field");
    encode_attack(0, slot as u16)
}

/// general_rule.pdf Ver. 4.3 §16-21-3: "... if <Material Save> will be
/// processed, the number of cards specified by this effect must be placed
/// under your Tamer WHENEVER POSSIBLE." With <Material Save 2> and only one
/// specified card among the digivolution cards, exactly that one card is
/// placed: the card pick is `min = max = 1`, not optional, and still a
/// prompt (no auto-selection).
#[test]
fn material_save_places_every_eligible_card_when_n_exceeds_them() {
    let mut runner = runner_with_material_save(2, true);
    let carrier = runner.place_stack(0, &["SRC-SHOUT", "SRC-OTHER", "MS-TEST"]);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let tamer = tamer_action(&runner);
    runner
        .execute_action(0, tamer)
        .expect("choose Tamer destination");

    let card_prompt = runner
        .pending_selection()
        .expect("the single specified card is still a prompt");
    assert!(
        matches!(
            card_prompt.kind,
            SelectionKind::CountCappedMultiSelect { min: 1, max: 1, .. }
        ),
        "exactly min(2, 1) = 1 card: {:?}",
        card_prompt.kind
    );
    assert!(!card_prompt.is_optional, "the card pick is mandatory");
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        0.0,
        "no PASS once the Tamer is chosen"
    );
    let actions = card_prompt.valid_action_ids.clone();
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(picked_source_id(&runner, carrier, actions[0]), "SRC-SHOUT");
    runner
        .execute_action(0, actions[0])
        .expect("place the specified card");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        top_ids_under_tamer(&runner),
        vec!["SRC-SHOUT".to_string(), "TAMER".to_string()]
    );
}

/// §16-21-2 / §15-8-5: two <Material Save> carriers deleted by one effect
/// each raise their own immediate-type effect. The player orders them (the
/// window's non-optional order prompt); each offer reads ITS OWN carrier's
/// live digivolution cards while both carriers still stand, and the batch
/// deletion then trashes the rest of both stacks.
#[test]
fn material_save_two_carriers_in_one_batch_each_offer_their_own_stack() {
    let mut runner = runner_with_material_save(1, true);
    let a = runner.place_stack(0, &["SRC-SHOUT", "SRC-OTHER", "MS-TEST"]);
    let b = runner.place_stack(0, &["SRC-BALLISTA", "SRC-OTHER", "MS-TEST"]);
    runner.place_on_field(0, "TAMER", Some(0));
    let top_b = runner.game.players[0].battle_area[b.index as usize]
        .top_card()
        .handle();

    runner
        .game
        .delete_permanents_batch(vec![a, b], ReplacementCause::OpponentEffect);

    let order = runner.pending_selection().expect("order the two offers");
    assert_eq!(order.kind, SelectionKind::EffectChoice);
    assert!(!order.is_optional);
    let choices = order.effect_choices.clone().expect("order choices");
    assert_eq!(choices.len(), 2, "one immediate effect per carrier");
    assert!(choices.iter().all(|c| c.label == "<Material Save 1>"));
    // Process B's first: the order is the player's.
    let b_first = choices
        .iter()
        .find(|c| c.source_card == Some(top_b))
        .expect("B's <Material Save> is offered")
        .action_id;
    runner.execute_action(0, b_first).expect("order B first");

    for (carrier, expected) in [(b, "SRC-BALLISTA"), (a, "SRC-SHOUT")] {
        let offer = runner.pending_selection().expect("the Tamer pick");
        assert_eq!(offer.kind, SelectionKind::OwnField);
        assert!(offer.is_optional);
        assert_eq!(offer.source_permanent, Some(carrier));
        let tamer = tamer_action(&runner);
        runner.execute_action(0, tamer).expect("choose Tamer");
        let pick = runner.pending_selection().expect("the card pick");
        assert_eq!(pick.source_permanent, Some(carrier));
        let actions = pick.valid_action_ids.clone();
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(picked_source_id(&runner, carrier, actions[0]), expected);
        runner.execute_action(0, actions[0]).expect("place it");
    }

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        top_ids_under_tamer(&runner),
        vec![
            "SRC-SHOUT".to_string(),
            "SRC-BALLISTA".to_string(),
            "TAMER".to_string()
        ],
        "B's card placed first, A's then placed beneath it"
    );
    let mut trash: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    trash.sort();
    assert_eq!(trash, ["MS-TEST", "MS-TEST", "SRC-OTHER", "SRC-OTHER"]);
}

/// §4-21-3 "Cards can't be stacked with tokens": a token Tamer can't receive
/// the cards (DCGO `MaterialSaveEffect`: `permanent.IsTamer &&
/// !permanent.IsToken`). With only a token Tamer nothing is offered; beside
/// a real Tamer, only the real one is a legal destination. (No Tamer token
/// exists in today's pool; the token flag is set on a test Tamer.)
#[test]
fn material_save_never_offers_a_token_tamer() {
    let mut only_token = runner_with_material_save(1, true);
    let carrier = place_material_save_stack(&mut only_token);
    let token = only_token.place_on_field(0, "TAMER", Some(0));
    only_token.game.players[0].battle_area[token.index as usize]
        .card_sources
        .last_mut()
        .unwrap()
        .is_token = true;
    only_token
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        only_token.pending_selection().is_none(),
        "a token Tamer is no destination: nothing to offer"
    );

    let mut both = runner_with_material_save(1, true);
    let carrier = place_material_save_stack(&mut both);
    let token = both.place_on_field(0, "TAMER", Some(0));
    let real = both.place_on_field(0, "TAMER", Some(0));
    both.game.players[0].battle_area[token.index as usize]
        .card_sources
        .last_mut()
        .unwrap()
        .is_token = true;
    both.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let offer = both.pending_selection().expect("the real Tamer is offered");
    assert_eq!(
        offer.valid_action_ids,
        vec![encode_attack(0, real.index as u16)],
        "only the non-token Tamer"
    );
}
