//! EX8-006 DemiMeramon — Digi-Egg, Purple, Lv.2. Traits: Flame, NSo.
//!
//! Round-2 integration: this is the ex8-nso-opponent lane's suite, kept beside the
//! digitama-inherited-effects lane's `ex8_006` (the YAML kept is that lane's, which
//! carries DCGO's always-shown Yes/No `outer_prompt`, isOptional TRUE); each test
//! here accepts that gate first.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-006.md)
//! Inherited Effect
//! [When Attacking] [Once Per Turn] If this Digimon has the [NSo] trait, by
//! trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Purple/EX8_006.cs — OnAllyAttack,
//! inherited, OPT (hash "WhenAttacking_EX8_006"); CanActivate = on the battle
//! area, top card ContainsTraits("NSo"), non-empty hand. Body: `canNoSelect:
//! true` hand discard (the "by trashing" cost), then — only if a card was
//! trashed — a MANDATORY pick of an opponent level-3 Digimon, deleted.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! E2 optional-by-cost (select_hand cost + trash), inherited OPT
//! when_attacking, trait-gated clause (source_permanent_trait_has).

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;

use super::nso_support::*;

const CARD_ID: &str = "EX8-006";

fn builder() -> DebugRunnerBuilder {
    let mut egg = make_test_egg("EGG-X", "Plain Egg");
    egg.colors = vec![CardColor::Purple];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-006 YAML loads")
        .add_card(egg)
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("HOST-NSO", 4, 5000, 4, CardColor::Purple))
        .add_card(plain("HOST-PLAIN", 4, 5000, CardColor::Purple))
        .add_card(plain("OPP3", 3, 3000, CardColor::Blue))
        .add_card(plain("OPP3B", 3, 3000, CardColor::Blue))
        .add_card(plain("OPP4", 4, 4000, CardColor::Blue))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
}

/// P0's `host` Digimon stacked on the DemiMeramon egg, P1 with the given
/// Digimon, P0's hand holding two filler cards. Returns the runner + attacker.
fn board(host: &str, opp: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let attacker = r.place_stack(0, &[CARD_ID, host]);
    for id in opp {
        r.place_on_field(1, id, Some(0));
    }
    push_hand(&mut r, 0, "FILL");
    push_hand(&mut r, 0, "FILL");
    (r, attacker)
}

#[test]
fn ex8_006_compiles_inherited_once_per_turn_when_attacking() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when == vec![CompiledTiming::WhenAttacking]
                && t.once_per_turn)));
}

#[test]
fn ex8_006_trash_a_card_to_delete_a_level_3_digimon() {
    let (mut r, attacker) = board("HOST-NSO", &["OPP3", "OPP4"]);
    r.attack_player(attacker, 1, false);
    r.accept_optional_trigger().expect("the always-shown Yes/No (DCGO isOptional: true)");

    let view = r.pending_selection_view().expect("the 'by trashing' cost prompt");
    assert!(view.is_optional, "the trash cost is declinable");
    pick_hand(&mut r, 0, "FILL");
    assert_eq!(hand_ids(&r, 0).len(), 1, "one card trashed from hand");
    assert_eq!(trash_ids(&r, 0), vec!["FILL".to_string()]);

    // Only the level 3 Digimon is a legal deletion target.
    let view = r.pending_selection_view().expect("the level 3 delete pick");
    let opp3 = digimon_engine::action::space::encode_attack(0, slot_of(&r, 1, "OPP3") as u16);
    let opp4 = digimon_engine::action::space::encode_attack(0, slot_of(&r, 1, "OPP4") as u16);
    assert!(view.valid_action_ids.contains(&opp3));
    assert!(!view.valid_action_ids.contains(&opp4), "level 4 is not a legal target");
    assert!(!view.is_optional, "the delete is mandatory once the cost is paid");
    pick_field(&mut r, 1, "OPP3");
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()]);
    assert!(trash_ids(&r, 1).contains(&"OPP3".to_string()));
}

#[test]
fn ex8_006_declining_the_cost_deletes_nothing() {
    let (mut r, attacker) = board("HOST-NSO", &["OPP3"]);
    r.attack_player(attacker, 1, false);
    r.accept_optional_trigger().expect("the always-shown Yes/No");
    pass(&mut r);
    assert_eq!(hand_ids(&r, 0).len(), 2, "nothing was trashed");
    assert!(field_ids(&r, 1).contains(&"OPP3".to_string()), "nothing deleted");
}

#[test]
fn ex8_006_requires_the_nso_trait_on_the_digimon() {
    let (mut r, attacker) = board("HOST-PLAIN", &["OPP3"]);
    r.attack_player(attacker, 1, false);
    // The effect never offers its cost: the carrier has no [NSo] trait.
    if let Some(view) = r.pending_selection_view() {
        assert_ne!(
            view.kind,
            digimon_engine::selection::SelectionKind::Hand,
            "no trash-a-card prompt without the [NSo] trait: {view:?}"
        );
    }
    drain_declining(&mut r);
    assert!(field_ids(&r, 1).contains(&"OPP3".to_string()));
    assert_eq!(hand_ids(&r, 0).len(), 2);
}

#[test]
fn ex8_006_with_no_level_3_target_the_card_is_still_trashed_and_nothing_is_deleted() {
    // DCGO: the discard cost is independent of target existence; the delete
    // pick is simply skipped when no level 3 Digimon exists.
    let (mut r, attacker) = board("HOST-NSO", &["OPP4"]);
    r.attack_player(attacker, 1, false);
    r.accept_optional_trigger().expect("the always-shown Yes/No");
    pick_hand(&mut r, 0, "FILL");
    assert_eq!(hand_ids(&r, 0).len(), 1);
    drain_declining(&mut r);
    assert!(field_ids(&r, 1).contains(&"OPP4".to_string()));
}

#[test]
fn ex8_006_is_once_per_turn() {
    let (mut r, attacker) = board("HOST-NSO", &["OPP3", "OPP3B"]);
    r.attack_player(attacker, 1, false);
    r.accept_optional_trigger().expect("the always-shown Yes/No");
    pick_hand(&mut r, 0, "FILL");
    pick_field(&mut r, 1, "OPP3");
    drain_declining(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP3B".to_string()]);
    // A second attack in the same turn does not offer the effect again.
    let attacker = handle_of(&r, 0, "HOST-NSO");
    r.game.players[0].battle_area[attacker.index as usize].is_suspended = false;
    r.attack_player(attacker, 1, false);
    if let Some(view) = r.pending_selection_view() {
        assert_ne!(view.kind, digimon_engine::selection::SelectionKind::Hand, "{view:?}");
    }
    drain_declining(&mut r);
    assert!(field_ids(&r, 1).contains(&"OPP3B".to_string()), "second Digimon survives");
}
