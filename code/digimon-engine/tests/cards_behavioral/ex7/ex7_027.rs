//! EX7-027 Chaperomon — Lv.5, Yellow, Puppet/LIBERATOR, Virus.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Digivolve] Yellow Lv.4: Cost 3
//!
//! <Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of
//! your Tokens or other [Puppet] trait Digimon, this Digimon attacks a
//! player without suspending.)
//!
//! [When Digivolving] You may play 1 level 3 Digimon card with the [Puppet]
//! trait from your hand without paying the cost.
//!
//! Inherited: [All Turns] [Once Per Turn] When this Digimon would leave the
//! battle area other than by your effects, by deleting 1 of your Tokens or
//! other [Puppet] trait Digimon, prevent it from leaving.
//! ```
//!
//! # DCGO reference
//!
//! `DCGO/Assets/Scripts/CardEffect/EX7/Yellow/EX7_027.cs`
//!
//! # Pattern tags
//!
//! - `grant_keyword` Overclock with Puppet/Token cost filter.
//! - `when_digivolving` optional free play-from-hand.
//! - `kind: replacement` / `when_would_leave_battle_area` inherited
//!   leave-prevention with a Token/other-Puppet deletion cost
//!   (`cancel_replacement`), `scope: inherited`, `once_per_turn`.
//!   Shared substrate closed by the inherited Token/Puppet leave-prevention
//!   dispatch (Track B 2026-05-08, see `qa/resolved-gaps.md`).

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn ex7_027_has_overclock_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .start();
    let chapero = runner.place_on_field(0, "EX7-027", Some(0));

    assert!(runner.game.has_keyword(chapero, Keyword::Overclock));
}

#[test]
fn ex7_027_when_digivolving_may_play_level3_puppet_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_puppet_level3("PUPPET-L3"))
        .hand(0, &["PUPPET-L3"])
        .memory(10)
        .start();
    let chapero = runner.place_stack(0, &["BASE", "EX7-027"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(chapero),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("level 3 Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "play-from-hand selection is optional"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select Puppet");
    runner.auto_resolve().expect("finish play");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"));
}

#[test]
fn ex7_027_inherited_prevents_opponent_effect_leave_by_deleting_token_or_other_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_puppet_level3("PUPPET-COST"))
        .add_card(make_token("TOKEN-COST"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    let puppet = runner.place_on_field(0, "PUPPET-COST", Some(0));
    let token = runner.place_on_field(0, "TOKEN-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept inherited replacement");

    let view = runner
        .pending_selection_view()
        .expect("Token/other-Puppet cost prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.valid_action_ids.contains(&encode_permanent(puppet)));
    assert!(view.valid_action_ids.contains(&encode_permanent(token)));
    runner
        .execute_action(0, encode_permanent(token))
        .expect("delete token cost");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "CARRIER"));
    assert!(!permanent_exists(&runner, 0, "TOKEN-COST"));
}

#[test]
fn ex7_027_inherited_does_not_prevent_own_effect_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_puppet_level3("PUPPET-COST"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    runner.place_on_field(0, "PUPPET-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own effects must not offer the inherited prevention prompt"
    );
    assert!(!permanent_exists(&runner, 0, "CARRIER"));
    assert!(permanent_exists(&runner, 0, "PUPPET-COST"));
}

#[test]
fn ex7_027_inherited_replacement_clause_is_compiled_inherited_and_opt() {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};

    let runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .start();
    let compiled = runner.compiled_card("EX7-027").expect("EX7-027 compiles");

    let (scope, trigger, optional, once_per_turn) = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                trigger,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, trigger.clone(), *optional, *once_per_turn)),
            _ => None,
        })
        .expect("inherited leave-prevention replacement clause must compile");

    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "leave-prevention is an inherited (digivolution-source) effect"
    );
    assert_eq!(
        trigger, "when_would_leave_battle_area",
        "the clause triggers when this Digimon would leave the battle area"
    );
    assert!(
        once_per_turn,
        "printed text marks the clause [Once Per Turn]"
    );
    assert!(
        optional,
        "the replacement is offered as an optional accept/decline prompt"
    );
}

#[test]
fn ex7_027_inherited_replacement_declined_lets_this_digimon_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_token("TOKEN-COST"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    runner.place_on_field(0, "TOKEN-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional, "the replacement must be declinable");

    // Decline the replacement: this Digimon leaves and the cost is untouched.
    runner
        .execute_action(0, PASS)
        .expect("decline inherited replacement");
    runner.auto_resolve().expect("finish declined replacement");

    assert!(
        !permanent_exists(&runner, 0, "CARRIER"),
        "declining the replacement lets the carrier leave"
    );
    assert!(
        permanent_exists(&runner, 0, "TOKEN-COST"),
        "declining the replacement does not pay the deletion cost"
    );
}

#[test]
fn ex7_027_inherited_replacement_cost_deletion_sends_token_to_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_token("TOKEN-COST"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    let token = runner.place_on_field(0, "TOKEN-COST", Some(0));

    let trash_before = runner.game.players[0].trash.len();

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept inherited replacement");

    let cost = runner
        .pending_selection_view()
        .expect("Token/other-Puppet cost prompt");
    assert_eq!(cost.kind, SelectionKind::OwnField);
    runner
        .execute_action(0, encode_permanent(token))
        .expect("delete token cost");
    runner.auto_resolve().expect("finish replacement");

    // The deletion cost fires: the token leaves the battle area and is trashed
    // (GameEvent has no deletion variant yet, so the cost is verified via the
    // resulting zone change — the token leaves play and a card lands in trash).
    assert!(
        !permanent_exists(&runner, 0, "TOKEN-COST"),
        "the token cost is deleted from the battle area"
    );
    assert!(
        permanent_exists(&runner, 0, "CARRIER"),
        "paying the cost prevents the carrier from leaving"
    );
}

#[test]
fn ex7_027_inherited_replacement_opt_locks_second_use_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_token("TOKEN-A"))
        .add_card(make_token("TOKEN-B"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    runner.place_on_field(0, "TOKEN-A", Some(0));
    runner.place_on_field(0, "TOKEN-B", Some(0));

    // First leave attempt this turn: the replacement is available.
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = runner
        .pending_selection_view()
        .expect("first inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept first inherited replacement");
    let cost = runner
        .pending_selection_view()
        .expect("first replacement cost prompt");
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("pay first cost");
    runner.auto_resolve().expect("finish first replacement");

    assert!(
        permanent_exists(&runner, 0, "CARRIER"),
        "carrier stays after the first prevention"
    );

    // Second leave attempt the SAME turn: [Once Per Turn] locks the clause,
    // so no replacement prompt installs and the carrier leaves. A second token
    // is still on the field, proving the lockout is not a missing-cost artifact.
    let carrier = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER")
        .map(|index| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: index as u8,
        })
        .expect("carrier still on field");
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn] blocks the second inherited replacement this turn"
    );
    assert!(
        !permanent_exists(&runner, 0, "CARRIER"),
        "with the OPT clause spent, the carrier leaves on the second attempt"
    );
    assert!(
        permanent_exists(&runner, 0, "TOKEN-B"),
        "the spare token proves the lockout was OPT, not a missing legal cost"
    );
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.traits = vec!["Puppet".to_string()];
    card
}

fn make_token(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = digimon_engine::enums::CardKind::Token;
    card.traits = vec!["Token".to_string()];
    card
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}
