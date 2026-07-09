//! EX12 keyword behavior: `<Guard>`.
//!
//! Official rules 16-45 define Guard as an optional immediate replacement:
//! when another of your Digimon would leave the battle area by an opponent's
//! effect, delete the Guard carrier to prevent that other Digimon leaving.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

use super::helpers::{digimon_with_keywords, plain_digimon, plain_tamer};

fn guard_card(id: &str) -> CardData {
    digimon_with_keywords(id, 4, 5000, vec![Keyword::Guard])
}

fn field_ids(r: &DebugRunner, player: u8) -> Vec<String> {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn guard_accept_deletes_carrier_and_prevents_other_digimon_leaving() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    assert_ne!(guard, ally);

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Guard should offer an optional replacement for another own Digimon"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Guard replacement");

    assert_eq!(
        field_ids(&r, 0),
        vec!["ALLY".to_string()],
        "the protected Digimon should remain and the Guard carrier should be deleted"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "GUARD"),
        "Guard carrier should be in trash after paying the replacement cost"
    );
}

#[test]
fn guard_accept_saves_all_simultaneous_matching_digimon_and_keeps_indices_stable() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY-A"))
        .add_card(plain_digimon("ALLY-B"))
        .add_card(plain_digimon("BYSTANDER"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally_a = r.place_on_field(0, "ALLY-A", Some(0));
    let ally_b = r.place_on_field(0, "ALLY-B", Some(0));
    let _bystander = r.place_on_field(0, "BYSTANDER", Some(0));
    assert_ne!(guard, ally_a);
    assert_ne!(guard, ally_b);

    let outcome = r
        .game
        .delete_permanents_batch(vec![ally_a, ally_b], ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Guard should offer one optional replacement for the simultaneous leave batch"
    );
    assert!(
        outcome.completed.is_empty(),
        "batch should park before trashing any permanent"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Guard replacement");

    assert!(
        r.game.pending_selection.is_none(),
        "one Guard acceptance should settle the simultaneous matching leave batch"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec![
            "ALLY-A".to_string(),
            "ALLY-B".to_string(),
            "BYSTANDER".to_string()
        ],
        "Guard should delete only the carrier, save both simultaneous allies, and not corrupt shifted indices"
    );
    let trash_ids: Vec<_> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        trash_ids.contains(&"GUARD".to_string()),
        "Guard carrier should be the only trashed card from the batch"
    );
    assert!(
        !trash_ids.contains(&"ALLY-A".to_string())
            && !trash_ids.contains(&"ALLY-B".to_string())
            && !trash_ids.contains(&"BYSTANDER".to_string()),
        "protected allies and unrelated bystander must not be trashed"
    );
}

#[test]
fn guard_does_not_prompt_when_carrier_cannot_be_deleted_to_pay_cost() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.modifiers.add(
        guard,
        ModifierEntry::simple(ModifierType::CannotBeDestroyed, 0, Expiry::Permanent, 0),
    );

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not offer an impossible replacement when the carrier cannot be deleted"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string()],
        "the original Digimon should leave when the Guard cost cannot be paid"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ALLY"),
        "the unprotected ally should be trashed"
    );
}

#[test]
fn guard_decline_allows_original_leave_event() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, PASS)
        .expect("decline Guard replacement");

    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string()],
        "declining Guard should leave the carrier and let the ally leave"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ALLY"),
        "the original Digimon should be trashed when Guard is declined"
    );
}

#[test]
fn guard_replacement_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Guard should park an optional replacement prompt"
    );
    assert!(
        r.game.pending_selection_resume.is_some(),
        "Guard replacement prompt must be resume-driven so cloned games can resolve it"
    );

    let mut cloned = r.game.clone();
    cloned
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("clone accepts Guard replacement");

    assert_eq!(
        cloned.players[0]
            .battle_area
            .iter()
            .map(|p| p.top_card().card_id(&cloned.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["ALLY".to_string()],
        "clone resolves to the protected state"
    );
    assert!(
        cloned.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&cloned.card_data) == "GUARD"),
        "clone deletes the Guard carrier"
    );

    assert!(
        r.game.pending_selection.is_some(),
        "original prompt survives resolving the clone"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string(), "ALLY".to_string()],
        "original battle area is untouched while the clone resolves"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("original accepts Guard replacement");
    assert_eq!(
        field_ids(&r, 0),
        vec!["ALLY".to_string()],
        "original reaches the clone's protected state"
    );
}

#[test]
fn aura_granted_guard_behaves_like_printed_guard() {
    let yaml = r#"
card: TEST-GUARD-AURA-SRC
name: Guard Aura Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, kind: digimon, trait: ME }
    grant_keyword: { keyword: Guard }
"#;

    let mut carrier = plain_digimon("AURA-GUARD-CARRIER");
    carrier.traits.push("ME".to_string());

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register Guard aura DSL source")
        .add_card(carrier)
        .add_card(plain_digimon("ALLY"))
        .start();

    r.place_on_field(0, "TEST-GUARD-AURA-SRC", Some(0));
    let guard = r.place_on_field(0, "AURA-GUARD-CARRIER", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(guard, Keyword::Guard),
        "aura should grant Guard to the ME carrier"
    );

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "aura-granted Guard should offer the same replacement as printed Guard"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept aura-granted Guard replacement");

    assert_eq!(
        field_ids(&r, 0),
        vec!["TEST-GUARD-AURA-SRC".to_string(), "ALLY".to_string()],
        "aura-granted Guard should delete the carrier and keep the ally"
    );
}

#[test]
fn guard_ignores_own_effects_self_and_non_digimon_subjects() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .add_card(plain_tamer("TAMER"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let _tamer = r.place_on_field(0, "TAMER", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OwnEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not prompt for own-effect leave events"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string(), "TAMER".to_string()],
        "own-effect deletion should proceed"
    );

    let shifted_tamer = PermanentHandle {
        player: 0,
        index: 1,
    };
    r.game.set_effect_source_player_for_test(Some(1));
    let returned = r.game.return_to_hand(shifted_tamer);
    r.game.set_effect_source_player_for_test(None);
    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not prompt for non-Digimon subjects"
    );
    assert!(
        returned.is_some(),
        "the Tamer should return to hand normally"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string()],
        "opponent-effect Tamer leave event should proceed"
    );

    r.game
        .delete_permanent_with_cause(guard, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not protect itself because the rule says another Digimon"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "self deletion should proceed without a Guard replacement"
    );
}
