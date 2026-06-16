//! EX11-002 Hiyarimon — Digi-Egg (Digitama), Lv.2, Blue.
//! Traits: Lesser / LIBERATOR. Form: In-Training.
//!
//! # Card text (cards.json; verified against the card image)
//!
//! Inherited Effect [Your Turn] While your opponent has no Digimon with
//! digivolution cards, this [Ice-Snow] trait Digimon can also attack your
//! opponent's unsuspended Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Blue/EX11_002.cs
//!   - CanAttackTargetDefendingPermanentClass at EffectTiming.None,
//!     SetIsInheritedEffect(true).
//!   - CanUseCondition: source on battle area AND opponent has zero Digimon
//!     with DigivolutionCards.Count >= 1.
//!   - DefenderCondition: opponent battle-area Digimon, !IsSuspended.
//!   - NOTE: DCGO omits both the printed [Your Turn] gate and the printed
//!     "this [Ice-Snow] trait Digimon" self-trait gate. The Rust spec follows
//!     the printed text (no-approximations): the aura is additionally gated on
//!     your_turn + the carrier having the Ice-Snow trait. ([Your Turn] is
//!     near-vacuous for attack legality — you only attack on your own turn —
//!     but the modifier presence is still gated faithfully.)
//!
//! # Patterns this test file covers
//! - Inherited conditional self-aura (`scope: inherited`, `kind: aura`,
//!   `target: {}`, `active_when`) granting a named modifier
//!   (CanAttackUnsuspended) — AD1-012 idiom.
//! - `no_permanent` existential with `stack_size_gte: 2` ("no Digimon with
//!   digivolution cards" — BT17-077 stack-size idiom).
//! - Carrier self-trait gate via `trait_has` in an inherited `active_when`
//!   (subject resolves to the carrier permanent).
//! - §4.4 mask consequence: attack bit on an unsuspended opponent Digimon.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::action::{build_action_mask, encode_attack};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType};

const CARD_ID: &str = "EX11-002";

// ─── Card factories ───────────────────────────────────────────────────────────

/// An [Ice-Snow] trait Digimon to carry the Hiyarimon egg in its stack.
fn ice_snow_carrier() -> CardData {
    let mut c = make_test_card("ICE-CARRIER", "IceCarrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier WITHOUT the Ice-Snow trait — the aura must stay off.
fn plain_carrier() -> CardData {
    let mut c = make_test_card("PLAIN-CARRIER", "PlainCarrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.traits = vec!["Beast".to_string()];
    c
}

/// Generic opponent Digimon (defender / digivolution source filler).
fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

fn hiyarimon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-002 YAML loads from embedded DSL pack")
        .add_card(ice_snow_carrier())
        .add_card(plain_carrier())
        .add_card(opp_digimon("OPP-DEF"))
        .add_card(opp_digimon("OPP-SRC"))
        .add_card(opp_digimon("OPP-DEF-2"))
        .start()
}

// ─── SECTION 1 — Structural assertions ────────────────────────────────────────

/// EX11-002 compiles to exactly one clause: an inherited declarative Aura
/// granting the CanAttackUnsuspended modifier with an active_when gate.
#[test]
fn ex11_002_compiles_single_inherited_aura_granting_can_attack_unsuspended() {
    let runner = hiyarimon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled spec");

    assert_eq!(
        compiled.effects.len(),
        1,
        "EX11-002 has exactly one effect clause; got {}",
        compiled.effects.len()
    );

    match &compiled.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            active_when,
            modifier,
            ..
        }) => {
            assert_eq!(
                *scope,
                CompiledScope::Inherited,
                "the aura is inherited text"
            );
            assert!(
                active_when.is_some(),
                "the aura must carry an active_when gate ([Your Turn] + opponent \
                 stack condition + Ice-Snow self-trait)"
            );
            assert_eq!(
                modifier.as_deref(),
                Some("CanAttackUnsuspended"),
                "the aura grants the CanAttackUnsuspended modifier"
            );
        }
        other => panic!("EX11-002's clause must be a declarative Aura; got {other:?}"),
    }
}

// ─── SECTION 2 — Aura ON: your turn, opp has no sourced Digimon ───────────────

/// Carrier has Ice-Snow, it's the controller's turn, and the opponent's only
/// Digimon has NO digivolution cards → the carrier gets CanAttackUnsuspended
/// and the attack bit on the unsuspended opponent Digimon lights up in the
/// action mask.
#[test]
fn ex11_002_aura_on_grants_modifier_and_unlocks_unsuspended_attack() {
    let mut r = hiyarimon_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    // Egg under an Ice-Snow carrier (EX11-002 is a digivolution source).
    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // Opponent: a single sourceless, unsuspended Digimon.
    let defender = r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        r.game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "carrier must hold CanAttackUnsuspended while the gate is satisfied"
    );

    let mask = build_action_mask(&r.game, tp);
    let atk_bit = encode_attack(carrier.index as u16, defender.index as u16) as usize;
    assert_eq!(
        mask[atk_bit], 1.0,
        "attack on the unsuspended opponent Digimon must be legal in the mask"
    );
}

// ─── SECTION 3 — Aura OFF: an opponent Digimon has digivolution cards ─────────

/// The opponent's Digimon has a digivolution card (stack size 2) → the gate
/// fails, no modifier, and the unsuspended target stays off-limits.
#[test]
fn ex11_002_aura_off_when_opp_digimon_has_digivolution_cards() {
    let mut r = hiyarimon_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // Opponent Digimon WITH a digivolution card under it.
    let defender = r.place_stack(opp, &["OPP-SRC", "OPP-DEF"]);

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "modifier must be absent while any opponent Digimon has digivolution cards"
    );

    let mask = build_action_mask(&r.game, tp);
    let atk_bit = encode_attack(carrier.index as u16, defender.index as u16) as usize;
    assert_eq!(
        mask[atk_bit], 0.0,
        "attack on the unsuspended (sourced) opponent Digimon must stay illegal"
    );
}

/// "No Digimon with digivolution cards" is universal: ONE sourced opponent
/// Digimon turns the aura off even if another opponent Digimon is sourceless.
#[test]
fn ex11_002_aura_off_when_any_one_opp_digimon_is_sourced() {
    let mut r = hiyarimon_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // One sourceless opponent Digimon...
    let sourceless = r.place_on_field(opp, "OPP-DEF-2", Some(0));
    // ...and one WITH a digivolution card.
    let _sourced = r.place_stack(opp, &["OPP-SRC", "OPP-DEF"]);

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "a single sourced opponent Digimon anywhere turns the aura off"
    );

    let mask = build_action_mask(&r.game, tp);
    let atk_bit = encode_attack(carrier.index as u16, sourceless.index as u16) as usize;
    assert_eq!(
        mask[atk_bit], 0.0,
        "even the sourceless unsuspended Digimon must stay off-limits"
    );
}

// ─── SECTION 4 — Aura OFF: opponent's turn ────────────────────────────────────

/// [Your Turn]: on the opponent's turn the carrier's controller does not get
/// the modifier even with the board condition satisfied.
#[test]
fn ex11_002_aura_off_on_opponents_turn() {
    let mut r = hiyarimon_runner();
    let tp = r.game.turn_player();
    let non_tp = 1 - tp;

    // Carrier stack belongs to the NON-turn player; the turn player has a
    // single sourceless Digimon (so the board condition holds for non_tp).
    let carrier = r.place_stack(non_tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_on_field(tp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "[Your Turn] gate: the modifier must be absent on the opponent's turn"
    );
}

// ─── SECTION 5 — Aura OFF: carrier lacks the Ice-Snow trait ───────────────────

/// "this [Ice-Snow] trait Digimon": a carrier without the Ice-Snow trait does
/// not get the modifier even when every other condition is satisfied.
#[test]
fn ex11_002_aura_off_when_carrier_lacks_ice_snow_trait() {
    let mut r = hiyarimon_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "PLAIN-CARRIER"]);
    let defender = r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "modifier must be absent when the carrier lacks the Ice-Snow trait"
    );

    let mask = build_action_mask(&r.game, tp);
    let atk_bit = encode_attack(carrier.index as u16, defender.index as u16) as usize;
    assert_eq!(
        mask[atk_bit], 0.0,
        "non-Ice-Snow carrier must not attack the unsuspended opponent Digimon"
    );
}
