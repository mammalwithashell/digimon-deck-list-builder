//! BT22-042 Nyabootmon — Digimon, Lv.7, Yellow, Puppet / LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! [Digivolve] from a Yellow Lv.6 for 4.
//! [Digivolve] While you have [Arisa Kinosaki], [Chaperomon]: Cost 6.
//! <Overclock ([Puppet] Trait)>.
//! [When Digivolving] You may play 1 level 4 or lower [Puppet] trait
//!   Digimon card from your hand without paying the cost. Then, to 1 of your
//!   opponent's Digimon, give -3000 DP until their turn ends for each of
//!   your Digimon.
//! [All Turns] [Once Per Turn] When any of your other Digimon are deleted,
//!   you may activate 1 of this Digimon's [When Digivolving] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_042.cs
//!
//! # Patterns this test covers
//! - Conditional alt-digivolve route (G-ALT-PATH-CONDITION, RESOLVED
//!   2026-05-15): the Chaperomon cost-6 route is legal only while an
//!   [Arisa Kinosaki] is on the controller's field.
//! - H11 Overclock with a Puppet/Token cost filter.
//! - E1/E2 [When Digivolving] optional free-play branch resuming into a
//!   mandatory scaled DP-reduction tail.
//! - [All Turns] [Once Per Turn] other-deletion refire of the
//!   [When Digivolving] body (refire_effect).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt22_042_has_yellow_lv6_digivolve_cost_4() {
    let compiled = compiled_bt22_042();
    let path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(path.cost, Some(CompiledCost::Literal(4)))
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|pred| pred.level_eq == Some(6))
                    && from
                        .all_of
                        .iter()
                        .any(|pred| pred.color_is == Some(CompiledColor::Yellow))
            })
    });

    assert!(
        path.is_some(),
        "Nyabootmon must digivolve from yellow Lv6 for cost 4"
    );
}

#[test]
fn bt22_042_yellow_lv6_base_can_digivolve_for_cost_4() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("YELLOW-LV6", 6, 11000, CardColor::Yellow, &[]))
        .hand(0, &["BT22-042"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "YELLOW-LV6", Some(0));
    let action = encode_digivolve(0, base.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "printed yellow Lv6 digivolve route should be action-mask legal"
    );

    runner.game.decode_action(action, 0);

    assert_eq!(runner.memory(), 6, "cost 4 should be paid");
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT22-042"
    );
}

#[test]
fn bt22_042_grants_overclock_with_puppet_cost_filter() {
    let compiled = compiled_bt22_042();
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
        .expect("Nyabootmon must grant Overclock with a cost filter");

    assert!(
        overclock
            .any_of
            .iter()
            .any(|pred| pred.kind == Some(CompiledCardKind::Token)),
        "Overclock cost allows deleting one of your Tokens"
    );
    assert!(
        overclock.any_of.iter().any(|pred| {
            pred.all_of
                .iter()
                .any(|child| child.kind == Some(CompiledCardKind::Digimon))
                && pred
                    .all_of
                    .iter()
                    .any(|child| child.trait_has.as_deref() == Some("Puppet"))
        }),
        "Overclock cost allows deleting another Puppet Digimon"
    );

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .start();
    let nyabootmon = runner.place_on_field(0, "BT22-042", Some(0));
    assert!(runner.game.has_keyword(nyabootmon, Keyword::Overclock));
}

#[test]
fn bt22_042_when_digivolving_only_offers_level4_or_lower_puppet_digimon_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .dsl_card("EX11-019")
        .expect("EX11-019 YAML loads")
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .add_card(make_digimon(
            "NONPUPPET-L4",
            4,
            4000,
            CardColor::Yellow,
            &[],
        ))
        .add_card(make_digimon("OPPONENT", 5, 14000, CardColor::Red, &[]))
        .hand(0, &["EX11-019", "EX7-027", "NONPUPPET-L4"])
        .start();
    let nyabootmon = runner.place_stack(0, &["BASE", "BT22-042"]);
    runner.place_on_field(1, "OPPONENT", Some(0));

    trigger_when_digivolving(&mut runner, nyabootmon);

    let choice = runner
        .pending_selection_view()
        .expect("optional free-play effect choice");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("choose to play a Puppet");

    let view = runner
        .pending_selection_view()
        .expect("level 4 or lower Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        view.valid_action_ids.contains(&PLAY_HAND_START),
        "level 4 Puppet Digimon is legal"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "level 5 Puppet Digimon is too high"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
        "level 4 non-Puppet Digimon is not legal"
    );
}

#[test]
fn bt22_042_when_digivolving_plays_puppet_then_debuffs_by_own_digimon_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .dsl_card("EX11-019")
        .expect("EX11-019 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .add_card(make_digimon("FILLER", 3, 1000, CardColor::Yellow, &[]))
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .add_card(make_digimon("OPPONENT", 5, 14000, CardColor::Red, &[]))
        .hand(0, &["EX11-019"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .start();
    let nyabootmon = runner.place_stack(0, &["BASE", "BT22-042"]);
    runner.place_on_field(0, "ARISA", Some(0));
    let target = runner.place_on_field(1, "OPPONENT", Some(0));

    trigger_when_digivolving(&mut runner, nyabootmon);
    runner.execute_branch(0).expect("choose to play a Puppet");
    pick_first_pending(&mut runner, "choose level 4 Puppet");

    let view = runner.pending_selection_view().expect("DP target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    pick_first_pending(&mut runner, "choose opponent Digimon");
    runner.auto_resolve().expect("finish Nyabootmon effect");

    assert!(
        permanent_exists(&runner, 0, "EX11-019"),
        "selected Puppet enters the battle area for free"
    );
    assert_eq!(
        runner.dp_of(target),
        Some(8000),
        "Nyabootmon + played Puppet are 2 own Digimon, so the target gets -6000; Arisa is not counted"
    );
    let entries: Vec<_> = runner
        .modifiers()
        .permanent_modifiers_iter(target)
        .collect();
    assert_eq!(entries.len(), 1, "DP reduction installs one modifier");
    assert_eq!(
        entries[0].source_player, 0,
        "expiry must be keyed to Nyabootmon's controller, not the targeted opponent"
    );

    runner.pass_turn();
    if runner.game.current_phase == digimon_engine::enums::GamePhase::EndOfTurnAction {
        runner.game.pass_end_of_turn_action();
    }
    assert_eq!(
        runner.turn_player(),
        1,
        "first turn rotation reaches the opponent"
    );
    assert_eq!(
        runner.turn_player(),
        1,
        "first turn rotation reaches the opponent"
    );
    assert_eq!(
        runner.dp_of(target),
        Some(8000),
        "debuff persists through the opponent's turn"
    );
    runner.pass_turn();
    if runner.game.current_phase == digimon_engine::enums::GamePhase::EndOfTurnAction {
        runner.game.pass_end_of_turn_action();
    }
    assert_eq!(
        runner.turn_player(),
        0,
        "second turn rotation returns to Nyabootmon's controller"
    );
    assert_eq!(
        runner.turn_player(),
        0,
        "second turn rotation returns to Nyabootmon's controller"
    );
    assert_eq!(
        runner.dp_of(target),
        Some(14000),
        "debuff expires when the opponent's turn ends"
    );
}

#[test]
fn bt22_042_declining_free_play_still_applies_scaled_dp_reduction() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .add_card(make_digimon(
            "PUPPET-L4",
            4,
            4000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_digimon("ALLY", 3, 3000, CardColor::Yellow, &[]))
        .add_card(make_digimon("OPPONENT", 5, 14000, CardColor::Red, &[]))
        .hand(0, &["PUPPET-L4"])
        .start();
    let nyabootmon = runner.place_stack(0, &["BASE", "BT22-042"]);
    runner.place_on_field(0, "ALLY", Some(0));
    let target = runner.place_on_field(1, "OPPONENT", Some(0));

    trigger_when_digivolving(&mut runner, nyabootmon);
    let decline = runner
        .pending_selection_view()
        .expect("optional free-play choice");
    assert_eq!(decline.kind, SelectionKind::EffectChoice);
    runner
        .execute_branch(1)
        .expect("decline optional free play");
    pick_first_pending(&mut runner, "choose opponent Digimon");
    runner.auto_resolve().expect("finish Nyabootmon effect");

    assert_eq!(runner.hand_size(0), 1, "declining keeps the Puppet in hand");
    assert_eq!(
        runner.dp_of(target),
        Some(8000),
        "Nyabootmon + existing ally are 2 own Digimon, so the target gets -6000"
    );
}

#[test]
fn bt22_042_has_conditional_chaperomon_cost_6_alt_path() {
    let compiled = compiled_bt22_042();
    let path = compiled
        .alt_paths
        .iter()
        .find(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(6)))
        })
        .expect("Nyabootmon must declare a cost-6 Chaperomon digivolve route");

    assert!(
        path.condition.is_some(),
        "the Chaperomon cost-6 route must carry a condition gate so it is not always legal"
    );
    let from = path
        .from
        .as_ref()
        .expect("the Chaperomon route filters its digivolve source");
    assert!(
        from.name_contains.as_deref() == Some("Chaperomon")
            || from
                .all_of
                .iter()
                .any(|pred| pred.name_contains.as_deref() == Some("Chaperomon")),
        "the cost-6 route digivolves from a Chaperomon"
    );
}

#[test]
fn bt22_042_chaperomon_alt_digivolve_requires_arisa_kinosaki() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon(
            "CHAPEROMON",
            5,
            7000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .hand(0, &["BT22-042"])
        .memory(10)
        .start();
    let chaperomon = runner.place_on_field(0, "CHAPEROMON", Some(0));
    let action = encode_digivolve(0, chaperomon.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "without Arisa Kinosaki, the Chaperomon cost-6 route is not legal"
    );
}

#[test]
fn bt22_042_chaperomon_alt_digivolve_costs_6_while_arisa_is_present() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon(
            "CHAPEROMON",
            5,
            7000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .hand(0, &["BT22-042"])
        .memory(10)
        .start();
    let chaperomon = runner.place_on_field(0, "CHAPEROMON", Some(0));
    runner.place_on_field(0, "ARISA", Some(0));

    runner
        .game
        .decode_action(encode_digivolve(0, chaperomon.index as u16), 0);

    assert_eq!(runner.memory(), 4, "conditional route should cost 6");
    assert_eq!(
        runner.game.players[0].battle_area[chaperomon.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT22-042"
    );
}

#[test]
fn bt22_042_other_own_digimon_deletion_may_refire_when_digivolving_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .add_card(make_digimon("ALLY", 3, 3000, CardColor::Yellow, &[]))
        .add_card(make_digimon(
            "PUPPET-L4",
            4,
            4000,
            CardColor::Yellow,
            &["Puppet"],
        ))
        .add_card(make_digimon("OPPONENT", 5, 14000, CardColor::Red, &[]))
        .hand(0, &["PUPPET-L4"])
        .start();
    runner.place_stack(0, &["BASE", "BT22-042"]);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPPONENT", Some(0));

    runner.game.delete_permanent_with_cause(
        ally,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    assert!(
        runner.pending_is_optional(),
        "refiring the When Digivolving effect is optional"
    );
    pick_first_pending(&mut runner, "choose to refire");
    assert_eq!(
        runner.pending_selection_view().map(|view| view.kind),
        Some(SelectionKind::EffectChoice),
        "after accepting the refire, the When Digivolving optional play choice appears"
    );
    runner.execute_branch(0).expect("choose to play a Puppet");
    assert_eq!(
        runner.pending_selection_view().map(|view| view.kind),
        Some(SelectionKind::Hand),
        "choosing the play branch exposes the hand-play prompt"
    );
}

#[test]
fn bt22_042_when_digivolving_dp_tail_noops_without_an_opponent_digimon() {
    // Negative branch for the mandatory DP tail: with no opponent Digimon on
    // the field the "give -3000 DP ... to 1 of your opponent's Digimon" tail
    // has no legal target and must resolve to nothing.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .start();
    let nyabootmon = runner.place_stack(0, &["BASE", "BT22-042"]);

    trigger_when_digivolving(&mut runner, nyabootmon);
    // No Puppet in hand → the optional play branch is skipped; the mandatory
    // DP tail is reached but has no opponent Digimon to target.
    runner.auto_resolve().expect("finish Nyabootmon effect");

    assert!(
        runner.pending_selection().is_none(),
        "the DP tail installs no selection when the opponent has no Digimon"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "opponent has no Digimon to debuff"
    );
}

#[test]
fn bt22_042_refire_does_not_trigger_for_self_or_opponent_deletion() {
    // Negative branch: the printed clause says "your OTHER Digimon" — neither
    // Nyabootmon itself leaving nor an opponent Digimon leaving may offer the
    // refire.
    let mut self_case = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .start();
    let nyabootmon = self_case.place_stack(0, &["BASE", "BT22-042"]);
    self_case.game.delete_permanent_with_cause(
        nyabootmon,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    assert!(
        self_case.pending_selection_view().is_none(),
        "Nyabootmon's own deletion must not refire its When Digivolving effect"
    );

    let mut opponent_case = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .add_card(make_digimon("OPPONENT", 5, 14000, CardColor::Red, &[]))
        .start();
    opponent_case.place_stack(0, &["BASE", "BT22-042"]);
    let opponent = opponent_case.place_on_field(1, "OPPONENT", Some(0));
    opponent_case.game.delete_permanent_with_cause(
        opponent,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    assert!(
        opponent_case.pending_selection_view().is_none(),
        "an opponent Digimon's deletion must not refire your Nyabootmon"
    );
}

#[test]
fn bt22_042_refire_is_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 YAML loads")
        .add_card(make_digimon("BASE", 6, 11000, CardColor::Yellow, &[]))
        .add_card(make_digimon("ALLY-1", 3, 3000, CardColor::Yellow, &[]))
        .add_card(make_digimon("ALLY-2", 3, 3000, CardColor::Yellow, &[]))
        .start();
    runner.place_stack(0, &["BASE", "BT22-042"]);
    let ally1 = runner.place_on_field(0, "ALLY-1", Some(0));
    let ally2 = runner.place_on_field(0, "ALLY-2", Some(0));

    runner.game.delete_permanent_with_cause(
        ally1,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    assert!(
        runner.pending_selection_view().is_some(),
        "first other own Digimon deletion should offer the optional refire"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline the first optional refire");

    runner.game.delete_permanent_with_cause(
        ally2,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    assert!(
        runner.pending_selection_view().is_none(),
        "Once Per Turn must suppress the refire on a second same-turn deletion"
    );
}

fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn compiled_bt22_042() -> digimon_dsl::compiled::CompiledCard {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt22")
        .join("BT22-042.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT22-042 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT22-042 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT22-042 YAML compiles");
    registry
        .lookup("BT22-042")
        .expect("BT22-042 in registry")
        .clone()
}

fn make_digimon(id: &str, level: u8, dp: i32, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.colors = vec![color];
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}
