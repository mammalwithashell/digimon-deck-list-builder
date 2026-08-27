//! BT19-093 Queen Device — Option, Yellow, Cost 3, traits: [Device].
//!
//! # Card text (card image — authoritative; cross-checked data/cards.json + DCGO)
//!
//! While you don't have [Queen Device] in the battle area, you may ignore
//! this card's color requirements.
//! When this card is trashed from the battle area, until the end of your
//! opponent's turn, 1 of their Digimon can't activate [When Digivolving]
//! effects and gets -3000 DP.
//! **[Main]** Until the end of your opponent's turn, 1 of their Digimon can't
//! activate [When Digivolving] effects and gets -3000 DP. Then, place this
//! card in the battle area.
//!
//! **Inherited (Security):** [Security] Give 2 of your opponent's Digimon
//! <Security A. -2> for the turn. Then, add this card to the hand.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT19/Yellow/BT19_093.cs`
//!
//! # Patterns this test covers
//!
//! - **Negative-gate color bypass** — `kind: flood_gate` +
//!   `IgnoreColorRequirement` gated by `no_permanent { name_is: "Queen Device" }`.
//!   Mirror of BT22-099 / BT23-096 Clause 0 with the gate negated (DCGO
//!   `!HasMatchConditionOwnersPermanent(Queen Device)`).
//! - **[Main] mandatory `select_count_capped_multi { max:1, clamp_to_available }`
//!   + per_selected (-3000 DP + CannotActivateWhenDigivolvingEffects)** —
//!   DCGO `Math.Min(1, count)` select + ChangeDigimonDP(-3000) +
//!   DisableEffectClass(IsWhenDigivolving). Precedent shape: BT12-028 Clause 0b.
//! - **[Security] (inherited) `select_count_capped_multi { max:2 }` + per_selected
//!   SecurityAttackChange -2 + `add_this_option_to_hand`** — DCGO
//!   `Math.Min(2, count)` select + ChangeDigimonSAttack(-2) + AddThisCardToHand.
//!
//! # Faithfulness audit (per clause)
//!
//! 0. **Color bypass** — `flood_gate` + `IgnoreColorRequirement` +
//!    `target: { card_number_is: "BT19-093" }` matches DCGO
//!    `IgnoreColorConditionClass` (CardCondition cardSource == card). The
//!    `no_permanent { of: you, zone: [battle_area], name_is: "Queen Device" }`
//!    gate matches DCGO's negated `HasMatchConditionOwnersPermanent(Queen Device)`.
//! 1. **[Main] -3000 DP + CannotActivateWhenDigivolvingEffects** — FAITHFUL.
//!    The trailing "Then, place this card in the battle area" is STUBBED on
//!    gap G-OPTION-PERSIST-AS-FIELD-CARRIER (the only shipped self-placement
//!    step, `place_self_as_delay_option`, seats an `OptionState::Delayed`
//!    carrier rather than an `OrdinaryFieldOption`; DCGO
//!    `PlaceDelayOptionCards` persists indefinitely). NOTE (2026-08-26):
//!    the "auto-trashes at the owner's next turn end" half of this gap is
//!    GONE — G-DELAY-EVENT-WINDOW-AUTOTRASHED narrowed the no-delay-clause
//!    fallback to `DelayTrigger::ExternallyGated`, which parks indefinitely.
//!    What remains is the state-shape difference (see Section 4).
//! 2. **When trashed from battle area** — STUBBED on gap
//!    G-OPTION-SELF-TRASH-TRIGGER (`Game::trash_field_option` dispatches
//!    `OnOptionTrashed` only to OTHER live battle-area permanents, not the
//!    trashed card's own clause). Interlocks with gap 1 (the placement must
//!    seat the card persistently before it can later be trashed-with-trigger).
//! 3. **[Security] Security A. -2 on 2 opp + add self to hand** — FAITHFUL.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT19-093";
const YAML: &str = include_str!("../../../cards/bt19/BT19-093.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card — no traits, default Digimon shape.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// An opponent-side Digimon used as the [Main] / [Security] debuff target.
fn make_opp_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Red];
    c
}

/// A Digimon literally named "Queen Device" — for the color-bypass negative
/// gate (the bypass deactivates while a [Queen Device] permanent is on field).
fn make_queen_device_perm(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Queen Device");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Yellow];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_093_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT19-093 YAML must parse and compile without errors");
}

#[test]
fn bt19_093_is_option_cost_3_with_device_trait() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT19-093 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "BT19-093 must be an Option card"
    );
    assert_eq!(card.cost, Some(3), "BT19-093 prints Cost 3");
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Device")),
        "BT19-093 must carry the Device trait"
    );
}

/// Three authored clauses: [0] flood_gate (color bypass), [1] main_from_hand
/// (debuff), [2] inherited on_security (Security A. -2 + add self).
/// (The "when trashed from battle area" clause is stubbed on a real gap; see
/// the `#[ignore]` tests below.)
#[test]
fn bt19_093_has_three_authored_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT19-093 compiled");
    assert_eq!(
        card.effects.len(),
        3,
        "expected 3 authored clauses (flood_gate, main_from_hand, inherited on_security); got {}",
        card.effects.len()
    );
}

#[test]
fn bt19_093_clause_0_is_flood_gate_with_ignore_color_modifier() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT19-093 compiled");

    let is_flood_gate_with_modifier = match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. }) => {
            modifier.eq_ignore_ascii_case("IgnoreColorRequirement")
        }
        _ => false,
    };
    assert!(
        is_flood_gate_with_modifier,
        "clause 0 must be a flood_gate carrying IgnoreColorRequirement; got {:?}",
        card.effects[0]
    );
}

/// The color-bypass gate must be the NEGATIVE form: active while NO
/// [Queen Device] is in the battle area (DCGO negated condition).
#[test]
fn bt19_093_clause_0_gate_is_negative_no_queen_device() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT19-093 compiled");

    let active_dbg = match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            active_when, ..
        }) => {
            format!("{:?}", active_when)
        }
        other => panic!("clause 0 must be a FloodGate; got {:?}", other),
    };
    assert!(
        active_dbg.contains("no_permanent") && active_dbg.contains("Queen Device"),
        "clause 0 gate must be `no_permanent {{ name_is: Queen Device }}` (negative gate); got {active_dbg}"
    );
}

#[test]
fn bt19_093_clause_1_main_from_hand_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT19-093 compiled");

    match &card.effects[1] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 1 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "clause 1 must have FaceUp scope"
            );
            assert!(
                !t.optional,
                "clause 1 [Main] is NOT optional — no 'you may' in printed text"
            );
        }
        other => panic!(
            "clause 1 must be Triggered(main_from_hand); got {:?}",
            other
        ),
    }
}

#[test]
fn bt19_093_clause_2_is_inherited_on_security_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT19-093 compiled");

    match &card.effects[2] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 2 must have Inherited scope"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 2 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert!(
                !t.optional,
                "Security clause must be mandatory ([Security] effects are not opt-out; \
                 DCGO SecuritySkill factory has no canNoSelect)"
            );
            let has_add_to_hand = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::AddThisOptionToHand));
            assert!(
                has_add_to_hand,
                "clause 2 process must contain AddThisOptionToHand; got {:?}",
                t.process
            );
        }
        other => panic!(
            "clause 2 must be Triggered(inherited on_security); got {:?}",
            other
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 [Main] -3000 DP + CannotActivateWhenDigivolving
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative: no opponent Digimon → the mandatory select has zero candidates
/// (clamp_to_available) and silently no-ops; no selection prompt installs.
#[test]
fn bt19_093_main_no_opp_digimon_no_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let _fired = runner.game.activate_hand_main(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "no selection prompt should install when opponent has no Digimon"
    );
}

/// Positive: one opponent Digimon present → the [Main] effect installs a
/// capped-multi select over opponent Digimon (max 1, mandatory).
#[test]
fn bt19_093_main_one_opp_digimon_installs_capped_select() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_opp_digimon("OPP-DIG", "OppDigi"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DIG", None);
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT19-093");

    assert!(
        runner.pending_selection().is_some(),
        "[Main] must install a target selection when opponent has a Digimon"
    );
    let view = runner
        .pending_selection_view()
        .expect("selection view available");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "exactly 1 opponent Digimon should be a valid target; got {}",
        view.valid_action_ids.len()
    );
    assert!(
        !view.is_optional,
        "[Main] target selection is mandatory — DCGO canNoSelect: false"
    );
}

/// Positive end-to-end: pick the opponent Digimon → it gets -3000 DP AND the
/// CannotActivateWhenDigivolvingEffects modifier.
#[test]
fn bt19_093_main_applies_minus_3000_and_cannot_activate_when_digivolving() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_opp_digimon("OPP-DIG", "OppDigi"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let target = runner.place_on_field(1, "OPP-DIG", None);
    let dp_before = runner.effective_dp(target).expect("target has DP");

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("[Main] select installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    let _ = runner.auto_resolve();

    // -3000 DP applied (via ChangeDp modifier).
    assert_eq!(
        runner.modifiers().sum(target, ModifierType::ChangeDp),
        -3000,
        "the picked opponent Digimon must carry a -3000 ChangeDp modifier"
    );
    let dp_after = runner.effective_dp(target).expect("target has DP");
    assert_eq!(
        dp_after,
        dp_before - 3000,
        "effective DP must drop by 3000 (before={dp_before}, after={dp_after})"
    );

    // CannotActivateWhenDigivolvingEffects applied.
    assert!(
        runner
            .modifiers()
            .has(target, ModifierType::CannotActivateWhenDigivolvingEffects),
        "the picked opponent Digimon must carry CannotActivateWhenDigivolvingEffects"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 [Security] Security A. -2 on 2 opp + add self
// ═══════════════════════════════════════════════════════════════════════════════

/// Drives the inherited [Security] clause directly via `run_inherited_security_effect`
/// (the host carries BT19-093 as a digivolution source). With 2 opponent Digimon
/// present, both selected get SecurityAttackChange -2, and BT19-093 is routed to
/// the controller's hand.
///
/// Player perspective: the security clause runs for player 1 (the defender whose
/// security BT19-093 sits in), so "your opponent" = player 0.
#[test]
fn bt19_093_security_minus2_on_two_opp_then_add_self_to_hand() {
    let mut attacker = filler("ATK");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(attacker.clone())
        .add_card(filler("FILL"))
        .add_card(make_opp_digimon("P0-DIG-A", "P0 Digi A"))
        .add_card(make_opp_digimon("P0-DIG-B", "P0 Digi B"))
        .memory(10)
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .security(1, &["BT19-093"])
        .start();

    // Two of player 1's opponent (= player 0) Digimon on the field.
    let p0_a = runner.place_on_field(0, "P0-DIG-A", Some(0));
    let p0_b = runner.place_on_field(0, "P0-DIG-B", Some(0));

    // An attacker for player 0 — used only to flip BT19-093 from player 1's
    // security through the real security-check path.
    let attacker_handle = runner.place_on_field(0, "ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT19-093 in player 1 security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner
        .auto_resolve()
        .expect("security clause selections resolve");

    // Both player-0 Digimon must carry the SecurityAttackChange -2 modifier.
    assert_eq!(
        runner
            .modifiers()
            .sum(p0_a, ModifierType::SecurityAttackChange),
        -2,
        "P0-DIG-A must carry <Security A. -2>"
    );
    assert_eq!(
        runner
            .modifiers()
            .sum(p0_b, ModifierType::SecurityAttackChange),
        -2,
        "P0-DIG-B must carry <Security A. -2>"
    );

    // BT19-093 must have left security and landed in player 1's hand.
    assert_eq!(
        runner.security_count(1),
        0,
        "BT19-093 must leave player 1's security"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT19-093 must be added to player 1's hand by add_this_option_to_hand; \
         hand was {:?}",
        runner.game.players[1]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );
}

/// Security with only ONE opponent Digimon: clamp_to_available means the
/// mandatory pick clamps to 1, that one gets <Security A. -2>, and the card is
/// still added to hand (DCGO `Math.Min(2, count)`).
#[test]
fn bt19_093_security_clamps_to_one_when_only_one_opp_digimon() {
    let mut attacker = filler("ATK");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(attacker.clone())
        .add_card(filler("FILL"))
        .add_card(make_opp_digimon("P0-DIG-A", "P0 Digi A"))
        .memory(10)
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .security(1, &["BT19-093"])
        .start();

    let p0_a = runner.place_on_field(0, "P0-DIG-A", Some(0));
    let attacker_handle = runner.place_on_field(0, "ATK", Some(0));

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner
        .auto_resolve()
        .expect("security clause selections resolve");

    assert_eq!(
        runner
            .modifiers()
            .sum(p0_a, ModifierType::SecurityAttackChange),
        -2,
        "the single opponent Digimon must carry <Security A. -2>"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT19-093 must still be added to hand even with only 1 opp Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — STUBBED clauses (real interlocking substrate gaps)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Gap G-OPTION-PERSIST-AS-FIELD-CARRIER (docs/RUST_ENGINE_GAPS.md /
// qa/dsl-vocab-gaps.md): the [Main] "Then, place this card in the battle area"
// must seat BT19-093 as a persistent field Option (OrdinaryFieldOption). DCGO
// `PlaceDelayOptionCards` plays it as an ordinary permanent with no scheduled
// auto-trash; the only shipped self-placement step, `place_self_as_delay_option`,
// seats an OptionState::Delayed carrier. Needs a `place_self_as_field_option`
// step lowering to OrdinaryFieldOption.
//
// UPDATED 2026-08-26 (G-DELAY-EVENT-WINDOW-AUTOTRASHED): this comment used to
// say the seated carrier "AUTO-TRASHES at the owner's next turn — unfaithful".
// That is no longer true. BT19-093 declares no `kind: delay` clause, so it hits
// the self-placement fallback, which is now `DelayTrigger::ExternallyGated` —
// it parks indefinitely, matching DCGO's `PlaceDelayOptionCards`. What survives
// of this gap is the STATE SHAPE: `OptionState::Delayed` is not
// `OrdinaryFieldOption`, so it reports as `OptionFieldState::Delay` through
// `Game::option_field_state` and satisfies `source_is_delayed_option`, neither
// of which is right for a card that prints no <Delay> at all. Re-assess whether
// the gap is now closeable before re-authoring the YAML.

#[ignore = "pending: G-OPTION-PERSIST-AS-FIELD-CARRIER — [Main] 'place this card in the battle area' needs a persistent OrdinaryFieldOption self-placement; place_self_as_delay_option seats an OptionState::Delayed carrier instead (it no longer auto-trashes — see the Section 4 note)"]
#[test]
fn bt19_093_main_places_self_as_persistent_field_option() {
    // When implemented: play BT19-093 via [Main], resolve the debuff select,
    // then assert BT19-093 exists in the controller's battle_area as a
    // persistent (OrdinaryFieldOption) permanent AND is NOT scheduled for
    // auto-trash at the owner's next turn end.
}

// Gap G-OPTION-SELF-TRASH-TRIGGER: "When this card is trashed from the battle
// area, ..." needs the trashed option's OWN on_option_trashed effect to fire.
// `Game::trash_field_option` dispatches OnOptionTrashed only to OTHER live
// battle-area permanents (the BT23-059 observer shape); the trashed card is
// removed before dispatch, so its own clause never fires. Needs an
// enqueue-from-trashed-source branch (cf. SourceTrashedFromStack). Interlocks
// with G-OPTION-PERSIST-AS-FIELD-CARRIER (the card must be seated persistently
// before it can later be trashed-with-trigger).

#[ignore = "pending: G-OPTION-SELF-TRASH-TRIGGER — 'when this card is trashed from battle area' needs the trashed option's OWN on_option_trashed clause to fire; trash_field_option only notifies OTHER live permanents"]
#[test]
fn bt19_093_trash_from_battle_area_debuffs_one_opp_digimon() {
    // When implemented: with BT19-093 seated as a field Option and 1 opp
    // Digimon present, trash BT19-093 via an external effect; assert its
    // OnOptionTrashed clause selects 1 opp Digimon and applies -3000 DP +
    // CannotActivateWhenDigivolvingEffects until opp turn end.
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Anti-helpers (silence unused-warning across test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_queen_device_perm("X");
    let _ = PermanentHandle {
        player: 0,
        index: 0,
    };
    let _ = EffectSourceKind::Option;
}
