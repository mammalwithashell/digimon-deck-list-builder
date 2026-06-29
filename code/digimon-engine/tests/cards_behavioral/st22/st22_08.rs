//! ST22-08 Offensive Plug-In V — Option, Cost 4, Red, Traits: Plug-In
//!
//! # Card text (cards.json — authoritative)
//!
//! While you have a Tamer, you can ignore this card's color requirements.
//! [Security] Delete 1 of your opponent's Digimon with the lowest DP.
//!   Then, add this card to the hand.
//! [Main] You may link this card to 1 of your Digimon without paying the
//!   cost. Then, delete 1 of your opponent's Digimon with as much or less
//!   DP as 1 of your Digimon.
//!
//! Inherited (Link Requirements):
//!   Link Requirements [Link] Lv.3 or higher: Cost 2
//!   (Plug this card from the hand or battle area sideways into the
//!   specified Digimon in the battle area.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST22/Red/ST22_08.cs
//!
//! Note: DCGO additionally registers an `EndOfTurnLinkedEffect`
//! ([End of Your Turn] this Digimon may attack). That clause is NOT present
//! in the printed `effect_description_eng` / `inherited_effect_description_eng`
//! for ST22-08 in `data/cards.json`. Per the project source-priority rule
//! (printed text outranks DCGO), it is intentionally NOT implemented.
//!
//! # Clauses (4)
//!  1. Color bypass — conditional on "you have a Tamer":
//!     - from-hand play-time bypass via `use_requirement`
//!       (`any_field_permanent { of: you, predicate: { kind: tamer } }`)
//!     - face-up / battle-area scope via `kind: flood_gate`
//!  2. [Security] delete opponent's lowest-DP Digimon; add this card to hand.
//!  3. [Main] optional free link to a Lv.3+ own Digimon; then delete opponent
//!     Digimon with DP <= a chosen ally Digimon's DP. (The Standard play
//!     mode's body.)
//!  4. `kind: link_requirement` — Link Requirements [Link] Lv.3+: Cost 2.
//!     (The Link play mode.)
//!
//! # Dual-mode Plug-In Option (G-LINK-OPTION-DUAL-PLAY-MODE RESOLVED)
//!
//! ST22-08 is a Plug-In Option with TWO play modes: a Standard `[Main]`
//! Option (pay use cost 4, run clause 3) and a Link Option (pay Link
//! Requirements cost 2, plug sideways into a Lv.3+ Digimon — no
//! `[Main]`/`[Security]` effect). It carries BOTH clause 3 (a Standard
//! `[Main]` body) AND clause 4 (`kind: link_requirement`), so the engine
//! treats it as dual-mode: playing it from hand surfaces a mode-select
//! prompt ("Play as a [Main] Option" vs "Plug in via Link Requirements").
//! Each branch charges only its own cost and routes to its own disposal
//! path. See the `unblock-medusamon-tier3-cards` change design.md (D2).
//!
//! All prior BLOCKED gaps (G-DSL-LINK-VERB, G-DSL-LINKED-SCOPE,
//! G-BINDING-DP-FORMULA, G-PRED-DP-LTE, G-IGNORE-COLOR-MASK,
//! G-ADD-OPTION-SELF-TO-HAND, G-LINK-OPTION-DUAL-PLAY-MODE) are resolved;
//! this file implements every clause behaviorally with no `#[ignore]`'d
//! tests.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

/// Production YAML for ST22-08, inlined at compile time.
const YAML: &str = include_str!("../../../cards/st22/ST22-08.yaml");

// ─── Helper card builders ───────────────────────────────────────────────────

/// A red Digimon at the given level / DP / cost.
fn red_digimon(id: &str, level: u8, dp: i32, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.colors = vec![CardColor::Red];
    c
}

/// A blue Digimon (color-mismatched against ST22-08's red) at the given DP.
fn blue_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

/// A Tamer (used to satisfy the "you have a Tamer" color-bypass gate).
fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Build a runner with ST22-08 compiled from production YAML.
fn st22_08_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST22-08 YAML parses + compiles")
        .memory(20)
        .start()
}

/// Enqueue + drain ST22-08's [Security] effect for the given permanent handle.
fn fire_security(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// True when a dual-mode mode-select prompt is currently installed.
fn mode_select_pending(runner: &DebugRunner) -> bool {
    matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    )
}

/// Play ST22-08 from hand index 0. When the dual-mode mode-select prompt
/// installs, pick a mode: `link_mode == false` → Standard `[Main]` (choice
/// index 0); `link_mode == true` → Link Requirements (choice index 1).
/// Returns the `OptionPlayResult` of the initial play call.
fn play_st22_08(runner: &mut DebugRunner, link_mode: bool) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if mode_select_pending(runner) {
        let ids = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .clone();
        let action = if link_mode { ids[1] } else { ids[0] };
        runner
            .game
            .resolve_selection(0, action)
            .expect("resolve mode-select");
    }
    result
}

// ═══════════════════════════════════════════════════════════════════════════
// § 1  Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

/// ST22-08.yaml must parse as a valid `kind: option` card spec.
#[test]
fn st22_08_yaml_parses_as_option_kind() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("ST22-08.yaml must parse");
    assert_eq!(spec.card, "ST22-08", "card ID must be ST22-08");
}

/// ST22-08.yaml must compile through the DSL registry without errors.
#[test]
fn st22_08_yaml_compiles_without_errors() {
    let _runner = st22_08_runner();
}

/// ST22-08 must carry a `use_requirement` predicate (the conditional from-hand
/// color bypass — "While you have a Tamer").
#[test]
fn st22_08_has_use_requirement_for_color_bypass() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("parse");
    assert!(
        spec.use_requirement.is_some(),
        "ST22-08 must declare a conditional use_requirement for the Tamer-gated color bypass"
    );
}

/// ST22-08 must compile to exactly 4 clauses:
///   0: flood_gate (color bypass, face-up scope)
///   1: triggered on_security (delete lowest-DP + add self to hand)
///   2: triggered main_from_hand (optional free link + DP-bounded delete)
///   3: link_requirement (Link Requirements — the Link play mode)
#[test]
fn st22_08_has_four_clauses() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "ST22-08 must have exactly 4 clauses (flood_gate, security, main, link_requirement); got {}",
        card.effects.len()
    );
}

/// Clause: a `flood_gate` declarative clause grants IgnoreColorRequirement.
#[test]
fn st22_08_has_flood_gate_color_bypass_clause() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    let has_flood_gate = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
        )
    });
    assert!(
        has_flood_gate,
        "ST22-08 must have a flood_gate clause for the field-side color bypass"
    );
}

/// ST22-08 must compile a `kind: link_requirement` clause — it is the Link
/// play mode of the dual-mode Plug-In Option. The lowered effect carries a
/// `link_cost` of 2.
#[test]
fn st22_08_has_link_requirement_clause() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    let has_link_requirement = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { .. })
        )
    });
    assert!(
        has_link_requirement,
        "ST22-08 must have a link_requirement clause (the Link play mode)"
    );

    let dsl = digimon_engine::dsl_cards::DslCardEffect::new(std::sync::Arc::new(card.clone()));
    let effects = digimon_engine::effect::CardEffect::effects(
        &dsl,
        digimon_engine::card_source::CardHandle(0),
    );
    assert!(
        effects.iter().any(|e| e.link_cost == Some(2)),
        "ST22-08's link_requirement must lower to an effect with link_cost 2"
    );
}

/// Clause: the [Security] clause is a triggered `on_security` clause.
#[test]
fn st22_08_has_on_security_clause() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    let sec = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
        _ => None,
    });
    assert!(
        sec.is_some(),
        "ST22-08 must have a triggered clause with OnSecurity timing"
    );
}

/// The [Security] clause must contain a delete step and an add-self-to-hand step.
#[test]
fn st22_08_security_clause_has_delete_and_add_self_to_hand() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("on_security clause");
    assert!(
        sec.process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "[Security] clause must delete a permanent"
    );
    assert!(
        sec.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddThisOptionToHand)),
        "[Security] clause must add this Option to the hand"
    );
}

/// Clause: the [Main] clause is a triggered `main_from_hand` clause and it is
/// optional ("You may link...").
#[test]
fn st22_08_has_optional_main_from_hand_clause() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("main_from_hand clause");
    assert!(
        main.optional,
        "[Main] clause must be optional (printed: 'You may link this card')"
    );
}

/// The [Main] clause must contain a `link_to_own_digimon` step and a
/// `delete_permanent` step (free optional link, then DP-bounded delete).
#[test]
fn st22_08_main_clause_has_link_step_and_delete_step() {
    let runner = st22_08_runner();
    let card = runner.compiled_card("ST22-08").expect("ST22-08 compiled");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("main_from_hand clause");
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::LinkToOwnDigimon { .. })),
        "[Main] clause must have a link_to_own_digimon step"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "[Main] clause must have a delete_permanent step"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// § 2  Clause 1 — color requirement bypass conditioned on "you have a Tamer"
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with a Tamer on the field (and no red board source), ST22-08 is
/// playable from hand despite the color mismatch — the conditional color
/// bypass is active.
#[test]
fn st22_08_color_bypass_active_when_tamer_present() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(blue_digimon("BLUE-FILLER", 3, 3000))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    // P0 has a (blue) Tamer on the field — color of the Tamer is irrelevant;
    // the gate is "you have a Tamer". No red board source.
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "ST22-08 must be playable from hand when a Tamer satisfies the color-bypass gate"
    );
}

/// Negative: with NO Tamer and no matching-color board source, ST22-08 is NOT
/// playable from hand — the color requirement is enforced.
#[test]
fn st22_08_color_requirement_enforced_without_tamer() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(blue_digimon("BLUE-FILLER", 3, 3000))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    // P0 has only a blue (color-mismatched) Digimon — no Tamer, no red source.
    runner.place_on_field(0, "BLUE-FILLER", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "ST22-08 must NOT be playable from hand without a Tamer or matching-color board source"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// § 3  Clause 2 — [Security] delete opponent's lowest-DP Digimon + add to hand
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: when the opponent has two Digimon (3000 / 5000 DP), the
/// [Security] delete prompt offers ONLY the lowest-DP (3000) Digimon.
#[test]
fn st22_08_security_offers_only_lowest_dp_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(red_digimon("OPP-LOW", 4, 3000, 4))
        .add_card(red_digimon("OPP-HIGH", 5, 5000, 6))
        .memory(20)
        .start();

    // Seat ST22-08 on P0's field so we can fire its [Security] effect.
    let st22 = runner.place_on_field(0, "ST22-08", Some(0));
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    fire_security(&mut runner, st22);

    let view = runner
        .pending_selection_view()
        .expect("[Security] delete selection must install");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "[Security] must offer only the lowest-DP (3000) opponent Digimon as a delete target"
    );
}

/// Positive (tie): when the opponent has two Digimon tied at the lowest DP,
/// BOTH are offered (the player resolves the tie-break choice).
#[test]
fn st22_08_security_offers_all_tied_lowest_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(red_digimon("OPP-A", 4, 3000, 4))
        .add_card(red_digimon("OPP-B", 4, 3000, 4))
        .add_card(red_digimon("OPP-BIG", 6, 9000, 8))
        .memory(20)
        .start();

    let st22 = runner.place_on_field(0, "ST22-08", Some(0));
    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);
    runner.place_on_field(1, "OPP-BIG", None);

    fire_security(&mut runner, st22);

    let view = runner
        .pending_selection_view()
        .expect("[Security] delete selection must install");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "[Security] must offer BOTH tied lowest-DP (3000) Digimon, excluding the 9000-DP one"
    );
}

/// Negative: when the opponent has no Digimon, the [Security] delete step
/// installs no selection (and the add-self-to-hand tail still runs).
#[test]
fn st22_08_security_no_selection_when_no_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .memory(20)
        .start();

    let st22 = runner.place_on_field(0, "ST22-08", Some(0));
    fire_security(&mut runner, st22);

    assert!(
        runner.game.pending_selection.is_none(),
        "[Security] must install no delete selection when the opponent has no Digimon"
    );
}

/// The [Security] delete is mandatory when a candidate exists (DCGO:
/// `canNoSelect: false`). [Security] effects have no opt-out per rules §16.
#[test]
fn st22_08_security_delete_is_mandatory_when_candidate_exists() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(red_digimon("OPP-D", 4, 3000, 4))
        .memory(20)
        .start();

    let st22 = runner.place_on_field(0, "ST22-08", Some(0));
    runner.place_on_field(1, "OPP-D", None);

    fire_security(&mut runner, st22);

    assert!(
        runner.game.pending_selection.is_some(),
        "[Security] delete selection must install when the opponent has a Digimon"
    );
    assert!(
        !runner.pending_is_optional(),
        "[Security] delete must be mandatory (not declinable) when a target exists"
    );
}

/// Full [Security] flow through the real combat / security path: an attack
/// flips ST22-08 from the defender's security stack; its [Security] effect
/// deletes the attacker-side opponent's lowest-DP Digimon and ST22-08 goes
/// to the defender's hand (not the trash).
#[test]
fn st22_08_security_deletes_lowest_dp_and_adds_self_to_hand_via_combat() {
    let mut attacker = filler("ATTACKER");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        // P0 (the attacking player, == the security-effect controller's
        // opponent) has a low-DP and a high-DP Digimon.
        .add_card(attacker)
        .add_card(red_digimon("P0-LOW", 4, 3000, 4))
        .add_card(red_digimon("P0-HIGH", 6, 9000, 8))
        .memory(20)
        // ST22-08 sits in P1's security stack.
        .security(1, &["ST22-08"])
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.place_on_field(0, "P0-LOW", None);
    runner.place_on_field(0, "P0-HIGH", None);
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");

    // P0 attacks P1's security — ST22-08 flips and its [Security] resolves.
    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The lowest-DP P0 Digimon (3000) was deleted; the 9000-DP one survives.
    let p0_ids: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| {
            p.card_sources[0]
                .card_id(&runner.game.card_data)
                .to_string()
        })
        .collect();
    assert!(
        !p0_ids.iter().any(|id| id == "P0-LOW"),
        "[Security] must delete P0's lowest-DP (3000) Digimon"
    );
    assert!(
        p0_ids.iter().any(|id| id == "P0-HIGH"),
        "[Security] must NOT delete the 9000-DP Digimon"
    );

    // ST22-08 left security and went to P1's hand (not the trash).
    assert_eq!(runner.security_count(1), 0, "ST22-08 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "ST22-08 must be added to the defender's hand after [Security] resolves"
    );
    assert_eq!(
        runner.trash_size(1),
        0,
        "ST22-08 must NOT be trashed — it returns to hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// § 4  Clause 3 — [Main] (Standard mode) optional free link + DP-comparative
//      delete. Each test plays ST22-08 and picks the Standard `[Main]` mode
//      at the dual-mode mode-select.
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: playing ST22-08 as a [Main] Option (Standard mode) with a Tamer
/// on field installs a pending selection for the optional free link.
#[test]
fn st22_08_main_installs_link_host_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-DIGI", 4, 5000, 4))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "MY-DIGI", None);
    runner.game.enter_main_phase();

    let result = play_st22_08(&mut runner, false);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "playing ST22-08 must park a pending flow"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "[Main] (Standard mode) must install the link host-selection prompt"
    );
}

/// The optional link step must be declinable (DCGO: `canNoSelect: true`).
/// PASS must be a legal action in the link host-selection prompt.
#[test]
fn st22_08_main_link_step_is_optional() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-DIGI", 4, 5000, 4))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "MY-DIGI", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);
    assert!(
        runner.game.pending_selection.is_some(),
        "link host selection installed"
    );
    assert!(
        runner.pending_is_optional(),
        "the optional free-link step must be declinable (the player may decline linking)"
    );
    // The decline action (PASS) must resolve cleanly without attaching.
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("PASS must be accepted to decline the optional link");
}

/// make-engine-cloneable (Wave A): the dual-mode Plug-In Option play-mode select
/// is resume-driven, so cloning the game at the mode prompt is faithful — the
/// clone plays the chosen mode (advancing into the [Main] link prompt) while the
/// original is untouched and replays identically.
#[test]
fn st22_08_mode_select_clones_faithfully() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-DIGI", 4, 5000, 4))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();
    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "MY-DIGI", None);
    runner.game.enter_main_phase();

    let _ = runner.game.play_option_from_hand(0, 0);
    assert!(mode_select_pending(&runner), "dual-mode mode-select installed");
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "the mode-select must be resume-driven (clone-safe)"
    );
    let main_mode = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];

    // Clone at the mode-select; play Standard [Main] on the clone only.
    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, main_mode)
        .expect("clone resolves the mode");
    // Standard [Main] advances to the optional link host-selection prompt
    // (an OwnField select, not the EffectChoice mode prompt).
    assert!(
        clone
            .pending_selection
            .as_ref()
            .is_some_and(|s| !matches!(s.kind, SelectionKind::EffectChoice)),
        "clone advanced past the mode-select into the [Main] link prompt"
    );
    let clone_disc = clone
        .pending_selection
        .as_ref()
        .map(|s| std::mem::discriminant(&s.kind));

    // INDEPENDENCE: the original is still at the mode-select.
    assert!(
        mode_select_pending(&runner),
        "original survives the clone, still at the mode-select"
    );

    // REPLAYS IDENTICALLY.
    runner
        .game
        .resolve_selection(0, main_mode)
        .expect("original resolves the mode");
    assert_eq!(
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| std::mem::discriminant(&s.kind)),
        clone_disc,
        "original reaches the clone's pending-selection state"
    );
}

/// Positive: resolving the link host-selection attaches ST22-08 to the chosen
/// Digimon (it appears in that permanent's `linked_cards`).
#[test]
fn st22_08_main_link_attaches_card_to_chosen_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-DIGI", 4, 5000, 4))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    let host = runner.place_on_field(0, "MY-DIGI", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);
    // First valid non-PASS action links to the host Digimon.
    let host_action = {
        let pending = runner.game.pending_selection.as_ref().expect("link prompt");
        *pending
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("a host-link action must be available")
    };
    let _ = runner.game.resolve_selection(0, host_action);

    assert_eq!(
        runner.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "ST22-08 must attach to the chosen Digimon's linked_cards after the free link"
    );
}

/// Positive: after linking (or declining), the [Main] clause then deletes an
/// opponent Digimon with DP <= a chosen ally Digimon's DP. With P0's ally at
/// 5000 DP, only the 4000-DP opponent Digimon is a valid delete target — the
/// 8000-DP one is excluded.
#[test]
fn st22_08_main_delete_offers_only_opponent_digimon_at_or_below_ally_dp() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-ALLY", 4, 5000, 4))
        .add_card(red_digimon("OPP-SMALL", 4, 4000, 4))
        .add_card(red_digimon("OPP-BIG", 6, 8000, 7))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "MY-ALLY", None);
    runner.place_on_field(1, "OPP-SMALL", None);
    runner.place_on_field(1, "OPP-BIG", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);

    // Step 1: decline the optional free link (PASS).
    assert!(
        runner.pending_is_optional(),
        "the link host selection must be declinable"
    );
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline link");

    // Step 2: choose the ally DP comparator (MY-ALLY @ 5000). There is one
    // own Digimon, so exactly one choice.
    {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("ally-comparator selection installs");
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("pick ally comparator");
    }

    // Step 3: the opponent-delete selection must offer only OPP-SMALL (4000).
    let view = runner
        .pending_selection_view()
        .expect("opponent-delete selection installs");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "[Main] delete must offer only the opponent Digimon with DP <= the chosen ally's DP"
    );
}

/// Negative: when no opponent Digimon has DP <= the chosen ally's DP, the
/// [Main] delete installs no target selection. With the ally at 2000 DP and
/// the only opponent Digimon at 5000 DP, nothing is deletable.
#[test]
fn st22_08_main_delete_skipped_when_no_opponent_at_or_below_ally_dp() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-SMALL-ALLY", 3, 2000, 3))
        .add_card(red_digimon("OPP-BIG", 6, 5000, 6))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "MY-SMALL-ALLY", None);
    runner.place_on_field(1, "OPP-BIG", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);

    // Decline the optional link.
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline link");

    // Pick the only ally comparator (2000 DP).
    {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("ally-comparator selection installs");
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("pick ally comparator");
    }

    // No opponent Digimon at or below 2000 DP — no delete selection installs.
    let no_delete = match runner.pending_selection_view() {
        None => true,
        Some(v) => v.valid_action_ids.iter().all(|&a| a == PASS),
    };
    assert!(
        no_delete,
        "[Main] delete must install no opponent target when none has DP <= the ally's DP"
    );
}

/// Positive: the full [Main] flow actually deletes the chosen opponent Digimon.
#[test]
fn st22_08_main_delete_removes_chosen_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("MY-ALLY", 5, 6000, 5))
        .add_card(red_digimon("OPP-TARGET", 4, 4000, 4))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "MY-ALLY", None);
    runner.place_on_field(1, "OPP-TARGET", None);
    assert_eq!(runner.battle_area_size(1), 1, "precondition");
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline link");

    // Pick the ally comparator.
    {
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .expect("ally comparator")
            .valid_action_ids[0];
        runner.game.resolve_selection(0, action).expect("ally pick");
    }
    // Pick (delete) the opponent target.
    {
        let action = *runner
            .game
            .pending_selection
            .as_ref()
            .expect("opponent delete")
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("a delete target");
        runner
            .game
            .resolve_selection(0, action)
            .expect("delete opponent");
    }
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "[Main] must delete the chosen opponent Digimon (DP <= ally DP)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// § 5  Link Requirements host constraint — [Main] free link demands Lv.3+
//
// The inherited "Link Requirements [Link] Lv.3 or higher" host constraint is
// enforced on the [Main] effect's free link via the `link_to_own_digimon`
// step's `level_gte: 3` filter.
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: a Lv.3+ own Digimon is a valid host for the [Main] free link.
#[test]
fn st22_08_main_link_attaches_to_level_3_plus_host() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    let host = runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);
    let host_action = {
        let pending = runner.game.pending_selection.as_ref().expect("link prompt");
        *pending
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("Lv.3 host must be a valid link target")
    };
    let _ = runner.game.resolve_selection(0, host_action);

    assert_eq!(
        runner.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "ST22-08's [Main] free link must attach to a Lv.3+ Digimon"
    );
}

/// Negative: a Lv.2 Digimon is NOT a valid host for the [Main] free link —
/// the Link Requirements demand Lv.3 or higher. The link step must offer no
/// host (only PASS) and the Lv.2 Digimon must not receive ST22-08.
#[test]
fn st22_08_main_link_excludes_level_2_host() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV2-HOST", 2, 2000, 2))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    let host = runner.place_on_field(0, "LV2-HOST", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, false);

    // The link step finds no eligible Lv.3+ host. It either installs no
    // host-selection at all (skips straight to the ally comparator) or
    // installs a prompt with only PASS — either way the Lv.2 Digimon must
    // never be a valid link target. Resolving the first valid action must
    // not attach ST22-08 to the Lv.2 Digimon.
    if runner.game.pending_selection.is_some() {
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        let _ = runner.game.resolve_selection(0, action);
    }
    assert_eq!(
        runner.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        0,
        "ST22-08 must NOT link to a Lv.2 Digimon (Link Requirements: Lv.3 or higher)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// § 6  Dual-mode play — G-LINK-OPTION-DUAL-PLAY-MODE
//
// ST22-08 is both a Standard [Main] Option (use cost 4) and a Link Option
// (Link Requirements cost 2). Played from hand with both modes affordable it
// surfaces a mode-select prompt; each branch charges only its own cost.
// ═══════════════════════════════════════════════════════════════════════════

/// A dual-mode Plug-In Option installs a mode-select prompt offering both the
/// Standard `[Main]` mode and the Link mode.
#[test]
fn st22_08_dual_mode_installs_mode_select() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "a dual-mode Option play must park a mode-select prompt"
    );
    assert!(
        mode_select_pending(&runner),
        "playing ST22-08 with both modes affordable must install a mode-select prompt"
    );
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "the mode-select must offer exactly two modes (Standard [Main] + Link)"
    );
    let choices = pending
        .effect_choices
        .as_ref()
        .expect("mode-select must carry labeled effect_choices");
    assert_eq!(choices.len(), 2, "two labeled mode choices");
    assert!(
        choices[0].label.contains("Main"),
        "choice 0 must be the Standard [Main] Option mode; got {:?}",
        choices[0].label
    );
    assert!(
        choices[1].label.contains("Link"),
        "choice 1 must be the Link Requirements mode; got {:?}",
        choices[1].label
    );
}

/// Choosing the Standard `[Main]` mode charges exactly the printed use cost
/// (4) and resolves through the `[Main]` Option flow.
#[test]
fn st22_08_standard_mode_charges_use_cost_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();

    let before = runner.memory();
    let _ = play_st22_08(&mut runner, false);
    assert_eq!(
        runner.memory(),
        before - 4,
        "the Standard [Main] mode must charge exactly the printed use cost of 4"
    );
    // The Standard [Main] body is in flight — its optional free link prompt
    // is installed.
    assert!(
        runner.game.pending_selection.is_some(),
        "the Standard [Main] body must run after the mode-select resolves"
    );
}

/// Choosing the Link mode charges exactly the Link Requirements cost (2).
#[test]
fn st22_08_link_mode_charges_link_cost_2() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();

    let before = runner.memory();
    let _ = play_st22_08(&mut runner, true);
    assert_eq!(
        runner.memory(),
        before - 2,
        "the Link mode must charge exactly the Link Requirements cost of 2 (not the use cost)"
    );
}

/// Choosing the Link mode installs a host-selection over the Lv.3+ Digimon
/// and attaches ST22-08 sideways to the chosen host.
#[test]
fn st22_08_link_mode_attaches_to_host() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    let host = runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, true);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Link mode installs a host-selection prompt");
    assert!(
        matches!(pending.kind, SelectionKind::OwnField),
        "the Link host-selection is an own-field prompt"
    );
    let host_action = pending.valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, host_action)
        .expect("resolve Link host-selection");

    assert_eq!(
        runner.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the Link play must attach ST22-08 sideways to the chosen Lv.3+ Digimon"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "a plugged-in Plug-In is NOT trashed — it rides the host"
    );
}

/// The Link host-selection offers only Lv.3+ Digimon — a Lv.2 Digimon is not
/// a legal plug-in host (Link Requirements: Lv.3 or higher).
#[test]
fn st22_08_link_mode_host_select_offers_only_level_3_plus() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .add_card(red_digimon("LV2-DIGI", 2, 2000, 2))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.place_on_field(0, "LV2-DIGI", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, true);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Link mode installs a host-selection prompt");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "the Link host-selection must offer only the Lv.3+ Digimon (not the Lv.2 one)"
    );
}

/// The Link play resolves to an attach only — the `[Main]` effect (its
/// DP-comparative delete) does NOT run. An opponent Digimon that the `[Main]`
/// effect could have deleted survives.
#[test]
fn st22_08_link_mode_skips_main_effect() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 7000, 3))
        .add_card(red_digimon("OPP-WEAK", 3, 1000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.place_on_field(1, "OPP-WEAK", None);
    assert_eq!(runner.battle_area_size(1), 1, "precondition");
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, true);
    // Resolve the Link host-selection.
    let host_action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Link host-selection")
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, host_action)
        .expect("resolve Link host-selection");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the Link play must NOT run the [Main] delete — the opponent Digimon survives"
    );
}

/// When only the Link mode is affordable (memory covers the Cost-2 link but
/// not the Cost-4 use), ST22-08 plays directly as a Link Option with no
/// mode-select prompt.
#[test]
fn st22_08_plays_link_directly_when_standard_unaffordable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();
    // Drop memory so the Cost-2 Link mode is affordable but the Cost-4
    // Standard mode would push memory below the floor.
    runner.game.set_memory(-7);

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "a single affordable mode plays directly and parks its own flow"
    );
    assert!(
        !mode_select_pending(&runner),
        "no mode-select prompt installs when only one mode is affordable"
    );
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the Link host-selection installs directly");
    assert!(
        matches!(pending.kind, SelectionKind::OwnField),
        "the single affordable mode (Link) goes straight to the host-selection"
    );
    assert_eq!(
        runner.memory(),
        -9,
        "the direct Link play charges exactly the Cost-2 link cost"
    );
}

/// Choosing the Link mode with no eligible Lv.3+ host trashes ST22-08 (there
/// is nothing to plug into) — consistent with a single-mode Link Option.
#[test]
fn st22_08_link_mode_with_no_host_trashes() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV2-DIGI", 2, 2000, 2))
        .hand(0, &["ST22-08"])
        .memory(20)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    // Only a Lv.2 Digimon — not a legal Lv.3+ plug-in host.
    runner.place_on_field(0, "LV2-DIGI", None);
    runner.game.enter_main_phase();

    let _ = play_st22_08(&mut runner, true);
    assert!(
        runner.game.pending_selection.is_none(),
        "with no eligible host the Link play installs no host-selection"
    );
    assert!(
        runner.game.pending_option.is_none(),
        "the Link play completes — no Option resolution is left parked"
    );
    assert_eq!(
        runner.trash_size(0),
        1,
        "a Link play with no eligible host trashes ST22-08"
    );
}

/// ST22-08 played as a [Main] Option costs exactly its printed use cost (4) —
/// the [Main] link is free ("without paying the cost").
#[test]
fn st22_08_main_play_costs_only_printed_use_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(red_digimon("LV3-HOST", 3, 4000, 3))
        .hand(0, &["ST22-08"])
        .memory(10)
        .start();

    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "LV3-HOST", None);
    runner.game.enter_main_phase();

    let before = runner.memory();
    let _ = play_st22_08(&mut runner, false);
    assert_eq!(
        runner.memory(),
        before - 4,
        "ST22-08 played as a [Main] Option must cost exactly its printed use cost of 4"
    );
}
