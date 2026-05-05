//! BT22-036 Chaperomon.
//!
//! Printed text:
//! - [Hand] [Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon]
//!   from your trash as any of your [Shoemon]'s bottom digivolution card, it
//!   digivolves into this card for a digivolution cost of 3, ignoring
//!   digivolution requirements.
//! - <Overclock ([Puppet] Trait)>.
//! - Inherited [All Turns] [Once Per Turn] When this Digimon would leave the
//!   battle area other than by your effects, by deleting 1 of your Tokens or
//!   other [Puppet] trait Digimon, it doesn't leave.
//!
//! Gap-routed slices:
//! - The [Hand][Main] activation needs a proven hand-action-mask precondition
//!   over trash contents. Without that, the engine could expose a legal hand
//!   effect when no [ShoeShoemon] exists in trash, so the slice stays ignored.
//! - The inherited replacement does not currently dispatch from this card while
//!   it is beneath a carrier, so those positive tests stay ignored.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK,
    FIELD_EFFECT_START, HAND_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn bt22_036_structural_metadata_and_yellow_lv4_evo_match_printed_card() {
    let compiled = compiled_bt22_036();

    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    assert_eq!(compiled.attribute.as_deref(), Some("Virus"));
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "Puppet"));
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "LIBERATOR"));

    let evo = compiled
        .alt_paths
        .iter()
        .find(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(3)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(4) && from.color_is == Some(CompiledColor::Yellow)
                })
        })
        .expect("Chaperomon must digivolve from yellow Lv4 for cost 3");

    assert_eq!(evo.kind, CompiledAltPathKind::Digivolve);
}

#[test]
fn bt22_036_yellow_lv4_base_can_digivolve_for_cost_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .add_card(make_digimon("YELLOW-LV4", 4, 5000, CardColor::Yellow, &[]))
        .hand(0, &["BT22-036"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "YELLOW-LV4", Some(0));
    let action = encode_digivolve(0, base.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "printed yellow Lv4 digivolve route should be action-mask legal"
    );

    runner.game.decode_action(action, 0);

    assert_eq!(runner.memory(), 7, "cost 3 should be paid");
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT22-036"
    );
}

#[test]
fn bt22_036_grants_overclock_with_puppet_or_token_cost_filter() {
    let compiled = compiled_bt22_036();
    let overclock = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                overclock_cost_filter,
                ..
            }) if keyword.eq_ignore_ascii_case("Overclock") => overclock_cost_filter.as_ref(),
            _ => None,
        })
        .expect("Chaperomon must grant Overclock with a cost filter");

    assert!(
        overclock
            .any_of
            .iter()
            .any(|branch| branch.kind == Some(CompiledCardKind::Token)),
        "Overclock cost allows deleting one of your Tokens"
    );
    assert!(
        overclock.any_of.iter().any(|branch| {
            branch
                .all_of
                .iter()
                .any(|leaf| leaf.kind == Some(CompiledCardKind::Digimon))
                && branch
                    .all_of
                    .iter()
                    .any(|leaf| leaf.trait_has.as_deref() == Some("Puppet"))
        }),
        "Overclock cost allows deleting another Puppet trait Digimon"
    );

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .start();
    let chaperomon = runner.place_on_field(0, "BT22-036", Some(0));
    assert!(runner.game.has_keyword(chaperomon, Keyword::Overclock));
}

#[test]
fn bt22_036_runtime_overclock_masks_only_token_or_other_puppet_costs() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .add_card(make_token("COST-TOKEN"))
        .add_card(make_digimon(
            "NON-PUPPET",
            4,
            4000,
            CardColor::Yellow,
            &["Beast"],
        ))
        .add_card(make_digimon(
            "PUPPET-ALLY",
            4,
            4000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .start();

    runner.place_on_field(0, "BT22-036", Some(0));
    runner.place_on_field(0, "COST-TOKEN", Some(0));
    runner.place_on_field(0, "NON-PUPPET", Some(0));
    runner.place_on_field(0, "PUPPET-ALLY", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    let effect_bit = encode_field_effect(0, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[effect_bit as usize], 1.0, "Overclock is legal");

    runner.game.decode_action(effect_bit, 0);

    let cost_mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        cost_mask[encode_attack(0, 1) as usize],
        1.0,
        "token is a legal Overclock cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 2) as usize],
        0.0,
        "non-token non-Puppet Digimon is not a legal Overclock cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 3) as usize],
        1.0,
        "other Puppet Digimon is a legal Overclock cost"
    );
    assert_eq!(cost_mask[PASS as usize], 1.0, "Overclock is optional");
}

#[test]
#[ignore = "BLOCKED: G-INHERITED-REPLACEMENT-DISPATCH — inherited replacement does not install from beneath the carrier"]
fn bt22_036_inherited_replacement_deletes_token_to_prevent_opponent_effect_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .add_card(make_digimon(
            "CARRIER",
            6,
            11000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_token("COST-TOKEN"))
        .start();
    let carrier = runner.place_stack(0, &["BT22-036", "CARRIER"]);
    runner.place_on_field(0, "COST-TOKEN", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("inherited leave-prevention should ask for a cost");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(runner.pending_is_optional(), "replacement may be declined");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the token cost should be legal"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay token cost");
    runner.auto_resolve().expect("finish replacement");

    assert!(
        permanent_exists(&runner, 0, "CARRIER"),
        "carrier stays after BT22-036 inherited replacement cancels leave"
    );
    assert!(
        !permanent_exists(&runner, 0, "COST-TOKEN"),
        "token cost is deleted"
    );
}

#[test]
#[ignore = "BLOCKED: G-INHERITED-REPLACEMENT-DISPATCH — inherited replacement does not install from beneath the carrier"]
fn bt22_036_inherited_replacement_allows_other_puppet_but_not_self_or_non_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .add_card(make_digimon(
            "CARRIER",
            6,
            11000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_digimon(
            "PUPPET-ALLY",
            4,
            4000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_digimon(
            "NON-PUPPET",
            4,
            4000,
            CardColor::Yellow,
            &["Beast"],
        ))
        .start();
    let carrier = runner.place_stack(0, &["BT22-036", "CARRIER"]);
    runner.place_on_field(0, "PUPPET-ALLY", Some(0));
    runner.place_on_field(0, "NON-PUPPET", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("Puppet cost selection");
    assert_eq!(
        view.valid_action_ids,
        vec![encode_attack(0, 1)],
        "only the other Puppet Digimon is a legal cost"
    );
}

#[test]
fn bt22_036_inherited_replacement_does_not_prevent_own_effect_or_no_cost_leave() {
    let mut own_effect_runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .add_card(make_digimon(
            "CARRIER",
            6,
            11000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_token("COST-TOKEN"))
        .start();
    let carrier = own_effect_runner.place_stack(0, &["BT22-036", "CARRIER"]);
    own_effect_runner.place_on_field(0, "COST-TOKEN", Some(0));

    own_effect_runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);

    assert!(
        !permanent_exists(&own_effect_runner, 0, "CARRIER"),
        "own-effect leave is not prevented"
    );
    assert!(
        own_effect_runner.pending_selection().is_none(),
        "own-effect leave should not ask for a cost"
    );

    let mut no_cost_runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .add_card(make_digimon(
            "CARRIER",
            6,
            11000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .start();
    let carrier = no_cost_runner.place_stack(0, &["BT22-036", "CARRIER"]);

    no_cost_runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        !permanent_exists(&no_cost_runner, 0, "CARRIER"),
        "without a token or other Puppet cost, leave proceeds"
    );
    assert!(
        no_cost_runner.pending_selection().is_none(),
        "no legal cost means no pending selection"
    );
}

#[test]
#[ignore = "BLOCKED: G-HAND-MAIN-TRASH-PREFLIGHT - hand [Main] mask cannot prove a required ShoeShoemon exists in trash"]
fn bt22_036_hand_main_is_masked_without_shoeshoemon_in_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .dsl_card("BT22-029")
        .expect("BT22-029 YAML loads")
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .hand(0, &["BT22-036"])
        .start();
    runner.place_on_field(0, "BT22-029", Some(0));
    runner.place_on_field(0, "ARISA", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[HAND_EFFECT_START as usize], 0.0,
        "hand [Main] must not be legal without a ShoeShoemon in trash"
    );
}

#[test]
#[ignore = "BLOCKED: G-HAND-MAIN-SELF-DIGIVOLVE - prove chained trash-to-bottom-source then effect digivolve from resolving hand card"]
fn bt22_036_hand_main_places_shoeshoemon_under_shoemon_then_digivolves_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-036")
        .expect("BT22-036 YAML loads")
        .dsl_card("BT22-029")
        .expect("BT22-029 YAML loads")
        .dsl_card("BT22-032")
        .expect("BT22-032 YAML loads")
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .hand(0, &["BT22-036"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "BT22-032");
    let shoemon = runner.place_on_field(0, "BT22-029", Some(0));
    runner.place_on_field(0, "ARISA", Some(0));

    let action = HAND_EFFECT_START;
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[action as usize], 1.0, "hand [Main] should be legal");
    runner.game.decode_action(action, 0);

    let trash_pick = runner
        .pending_selection_view()
        .expect("ShoeShoemon trash prompt");
    runner
        .execute_action(0, trash_pick.valid_action_ids[0])
        .expect("choose ShoeShoemon");
    let shoemon_pick = runner
        .pending_selection_view()
        .expect("Shoemon field prompt");
    runner
        .execute_action(0, shoemon_pick.valid_action_ids[0])
        .expect("choose Shoemon");
    runner.auto_resolve().expect("finish hand [Main]");

    let stack = &runner.game.players[0].battle_area[shoemon.index as usize].card_sources;
    assert_eq!(
        stack[0].card_id(&runner.game.card_data),
        "BT22-032",
        "ShoeShoemon is placed as bottom source"
    );
    assert_eq!(
        stack.last().unwrap().card_id(&runner.game.card_data),
        "BT22-036",
        "Chaperomon digivolves from hand onto Shoemon"
    );
    assert_eq!(runner.memory(), 7, "effect digivolve costs 3");
}

fn compiled_bt22_036() -> digimon_dsl::compiled::CompiledCard {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt22")
        .join("BT22-036.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT22-036 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT22-036 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT22-036 YAML compiles");
    registry
        .lookup("BT22-036")
        .expect("BT22-036 in registry")
        .clone()
}

fn encode_field_effect(field_index: u16, effect_slot: u16) -> u16 {
    FIELD_EFFECT_START + field_index * EFFECTS_PER_PERMANENT + effect_slot
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn make_digimon(id: &str, level: u8, dp: i32, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.colors = vec![color];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn make_token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.level = None;
    card
}

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card data registered");
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(card);
}
