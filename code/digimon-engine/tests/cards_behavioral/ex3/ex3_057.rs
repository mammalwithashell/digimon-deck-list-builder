//! EX3-057 Growlmon — Digimon, Lv.4, Purple, Cost 5, DP 5000.
//! Traits: Dark Dragon. Attribute: Virus.
//!
//! # Card text (card image — verbatim)
//!
//! ```text
//! Digivolve: 2 if name contains [Guilmon]
//! [When Digivolving] Delete 1 of your opponent's Digimon with 3000 DP or
//!   less. If no Digimon was deleted by this effect, both players trash the
//!   top 2 cards of their decks.
//! Inherited [When Attacking] [Once Per Turn] You may delete 1 of your other
//!   Digimon to have this Digimon gain <Security Attack +1> for the turn.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX3/Purple/EX3_057.cs — select <=3000 →
//! delete; FailureProcess mills 2 from EACH player's deck. Inherited
//! OnAllyAttack OPT (isOptional: true, canNoSelect: true): select 1 of your
//! OTHER Digimon → delete; on a successful deletion the carrier gets
//! Security Attack +1 (UntilEachTurnEnd).
//!
//! # DSL YAML
//! code/digimon-engine/cards/ex3/EX3-057.yaml

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "EX3-057";

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX3-057 in embedded DSL pack")
        .add_card({
            let mut c = make_test_card("OPP-SMALL", "OppSmall");
            c.card_kind = CardKind::Digimon;
            c.level = Some(3);
            c.dp = Some(3000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-BIG", "OppBig");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card({
            let mut c = make_test_card("ALLY", "Ally");
            c.card_kind = CardKind::Digimon;
            c.level = Some(3);
            c.dp = Some(2000);
            c
        })
        .add_card({
            let mut c = make_test_card("TOP-MON", "TopMon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(7000);
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex3_057_compiles_from_dsl_pack() {
    let card = compiled(CARD_ID);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert!(card.traits.iter().any(|t| t == "Dark Dragon"));
}

/// Alt-path: "Digivolve: 2 if name contains [Guilmon]".
#[test]
fn ex3_057_has_guilmon_alt_path_cost2() {
    let card = compiled(CARD_ID);
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
        }),
        "must have the [Guilmon]-name cost-2 digivolve alt-path"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] delete <=3000 or both players mill 2
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE delete branch: a 3000-DP opponent Digimon is deleted; NO mill.
#[test]
fn ex3_057_when_digivolving_deletes_3000_or_less() {
    let mut r = base_runner();
    let growl = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-SMALL", Some(0));
    let deck0 = r.game.players[0].deck.len();
    let deck1 = r.game.players[1].deck.len();

    fire(&mut r, EffectTiming::WhenDigivolving, growl);
    let view = r
        .pending_selection_view()
        .expect("mandatory delete pick when a <=3000 DP target exists");
    assert!(!view.is_optional);
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "the 3000-DP Digimon is deleted");
    assert_eq!(
        r.game.players[0].deck.len(),
        deck0,
        "deleted ⇒ no mill for you"
    );
    assert_eq!(
        r.game.players[1].deck.len(),
        deck1,
        "deleted ⇒ no mill for the opponent"
    );
}

/// FALLBACK branch: only a 4000-DP Digimon (no legal target) ⇒ both players
/// trash the top 2 cards of their decks.
#[test]
fn ex3_057_mills_both_decks_when_nothing_deleted() {
    let mut r = base_runner();
    let growl = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-BIG", Some(0));
    let deck0 = r.game.players[0].deck.len();
    let deck1 = r.game.players[1].deck.len();
    let trash0 = r.trash_size(0);
    let trash1 = r.trash_size(1);

    fire(&mut r, EffectTiming::WhenDigivolving, growl);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "the 4000-DP Digimon survives");
    assert_eq!(r.game.players[0].deck.len(), deck0 - 2, "you mill 2");
    assert_eq!(r.game.players[1].deck.len(), deck1 - 2, "opponent mills 2");
    assert_eq!(r.trash_size(0), trash0 + 2);
    assert_eq!(r.trash_size(1), trash1 + 2);
}

/// FALLBACK also fires with an empty opponent board.
#[test]
fn ex3_057_mills_with_no_opponent_digimon() {
    let mut r = base_runner();
    let growl = r.place_on_field(0, CARD_ID, Some(0));
    let deck0 = r.game.players[0].deck.len();
    fire(&mut r, EffectTiming::WhenDigivolving, growl);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[0].deck.len(), deck0 - 2);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — inherited [When Attacking][OPT] sacrifice for <Security A. +1>
// ════════════════════════════════════════════════════════════════════════════

/// Carrier with EX3-057 as an inherited source.
fn carrier_with_growlmon_source(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "TOP-MON"])
}

/// POSITIVE: accept the optional sacrifice ⇒ the other Digimon is deleted and
/// the carrier gains Security Attack +1 for the turn.
#[test]
fn ex3_057_inherited_sacrifice_grants_security_attack_plus_1() {
    let mut r = base_runner();
    let carrier = carrier_with_growlmon_source(&mut r);
    let _ally = r.place_on_field(0, "ALLY", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("the optional sacrifice pick must surface");
    assert!(view.is_optional, "printed 'you may delete' ⇒ declinable");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("the other Digimon must be offered");
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(0),
        1,
        "the sacrificed Digimon left the battle area"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "the carrier gains <Security A. +1> for the turn"
    );
}

/// DECLINE: passing the optional pick leaves everything unchanged.
#[test]
fn ex3_057_inherited_sacrifice_is_declinable() {
    let mut r = base_runner();
    let carrier = carrier_with_growlmon_source(&mut r);
    let _ally = r.place_on_field(0, "ALLY", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(r.pending_is_optional());
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("declining is legal");
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 2, "nothing was deleted");
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "declined ⇒ no Security Attack bonus"
    );
}

/// OPT lockout: a second [When Attacking] in the same turn must not offer a
/// second sacrifice (the bonus stays at +1).
#[test]
fn ex3_057_inherited_sacrifice_is_once_per_turn() {
    let mut r = base_runner();
    let carrier = carrier_with_growlmon_source(&mut r);
    let _a = r.place_on_field(0, "ALLY", Some(0));
    let _b = r.place_on_field(0, "ALLY", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r.pending_selection_view().expect("first sacrifice pick");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .unwrap();
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1
    );

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "[Once Per Turn] ⇒ no second +1 in the same turn"
    );
}

/// NEGATIVE: with no OTHER Digimon there is nothing to offer.
#[test]
fn ex3_057_inherited_no_other_digimon_no_prompt() {
    let mut r = base_runner();
    let carrier = carrier_with_growlmon_source(&mut r);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "no other Digimon ⇒ no sacrifice prompt"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0
    );
}
