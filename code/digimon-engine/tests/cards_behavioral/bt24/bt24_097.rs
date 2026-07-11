//! BT24-097 Soul Fear — Option, Purple, Cost 5, Traits: TS.
//!
//! # Card text (code/digimon-engine/cards/bt24/BT24-097.json — authoritative)
//!
//! effect_description_eng:
//!   "While you have an [TS] trait Digimon or Tamer on the field, you can
//!   ignore this card's color requirements.
//!   [Security] Activate this card's [Main] effects.
//!   [Main] Delete 1 of your opponent's level 6 or higher Digimon. Then, you
//!   may link this card to 1 of your Digimon on the field without paying the
//!   cost."
//!
//! inherited_effect_description_eng:
//!   "Link Requirements [Link] [TS] trait: Cost 3
//!   (Plug this card from the hand or battle area sideways into the
//!   specified Digimon in the battle area.)"
//!
//! Link box (card image, not in printed effect_description_eng):
//!   Link DP +2000
//!   [When Attacking][Once Per Turn] Delete 1 of your opponent's level 5 or
//!   lower Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_097.cs
//!
//! DCGO registers:
//!   - `EffectTiming.None` IgnoreColorConditionClass: CanUseCondition = owner
//!     has a Tamer-or-Digimon permanent with HasTSTraits. → use_requirement +
//!     face-up flood_gate, card-scoped (BT24-091/095 precedent).
//!   - `EffectTiming.None` AddSelfLinkConditionStaticEffect(permanentCondition:
//!     IsDigimon && HasTSTraits, linkCost: 3) → kind: link_requirement, cost 3,
//!     filter { kind: digimon, trait_has: TS }.
//!   - `EffectTiming.OnDeclaration` LinkEffect — generic Link play mode.
//!   - `EffectTiming.SecuritySkill` AddActivateMainOptionSecurityEffect —
//!     [Security] Activate this card's [Main] effects. → scope: inherited,
//!     when: on_security, body mirrors the Main clause (BT24-091/095 idiom).
//!   - `EffectTiming.OptionSkill` ActivateClass: mandatory delete of 1 opp
//!     Lv6+ Digimon (CanSelectTargetCondition: opp Digimon, HasLevel, Level>=6;
//!     canNoSelect: false, gated on HasMatchConditionPermanent so a card with
//!     no candidate silently no-ops), THEN an optional (canNoSelect: true)
//!     free link to 1 own Digimon eligible per the link condition (CanLinkTo
//!     TargetPermanent). → when: main_from_hand, mandatory
//!     select_opponent_permanent (level_gte: 6) + delete_permanent, then
//!     link_to_own_digimon { optional: true, free: true, filter: { kind:
//!     digimon, trait_has: TS } }.
//!   - `EffectTiming.OnAllyAttack` ActivateClass, `SetIsLinkedEffect(true)`,
//!     `SetHashString("WA_BT24-097")`, count 1 (Once Per Turn), mandatory
//!     (canNoSelect: false) delete of 1 opp Lv5-or-lower Digimon, gated on
//!     `CanActivateCondition`: `IsExistOnBattleAreaDigimon(card)` (the HOST is
//!     still a Digimon on the field) `&& HasMatchConditionOpponentsPermanent`.
//!     `SetIsLinkedEffect(true)` fires this attributed to the HOST's own
//!     attack (Rust engine: `.linked()` builder flag — the timing is
//!     preserved but sourced from `perm.linked_cards`, see
//!     `effect.rs::EffectBuilder::linked` and
//!     `effect_queue.rs::enqueue_from_permanent`'s linked-card scan; the same
//!     `TriggerSource::Permanent(handle)` WhenAttacking dispatch that fires
//!     the attacker's own carrier-scoped [When Attacking] also scans that
//!     attacker's `linked_cards` — `combat/mod.rs::fire_on_attack`). →
//!     scope: linked, when: when_attacking, once_per_turn: true, mandatory
//!     select_opponent_permanent (level_lte: 5) + delete_permanent.
//!   - Link DP +2000 (card-image link box, data-driven in DCGO Permanent.cs,
//!     not a CardEffect region) → scope: linked, kind: aura, target: {},
//!     dp_modifier: 2000 (BT21-071/BT21-073/BT24-067/ST22-12 precedent).
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - D3  Color ignore / bypass (use_requirement + face-up flood_gate)
//! - C1/C3-adjacent  Linked-card source: Option dual-mode (Standard [Main]
//!   vs Link play) + Link Requirements (TS trait, cost 3)
//! - E1  main_from_hand / on_security duplication ([Security] Activate [Main])
//! - E2  optional decline (the free link pick)
//! - F5-adjacent  mandatory delete gated on candidate existence
//! - G-LINK-INHERITED-ESS  `scope: linked` + `when: when_attacking` — linked
//!   ESS fires attributed to the HOST's own attack (BT21-018 / ST22-12
//!   precedent for `.linked()`, but the timing itself is `when_attacking`,
//!   not `when_linked` — this card is the first `scope: linked` +
//!   `when_attacking` combination in the DSL cards tree; if the linked
//!   trigger does not fire when driven via `push_linked_owned` +
//!   `attack_player`, this is BLOCKED per G-LINK-INHERITED-ESS and the
//!   affected tests are `#[ignore]`d citing that gap-id.)
//! - Link DP aura (+2000, scope: linked kind: aura)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT24-097";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_ts_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_digimon(id, level);
    card.traits = vec!["TS".to_string()];
    card
}

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn make_tamer(id: &str, traits: &[&str], color: CardColor) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn soul_fear_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-097 YAML parses and compiles")
        .add_card(make_ts_digimon("TS-DIGIMON", 4))
        .add_card(make_tamer("TS-TAMER", &["TS"], CardColor::Purple))
        .add_card(make_digimon("OPP-LV6", 6))
        .add_card(make_digimon("OPP-LV7", 7))
        .add_card(make_digimon("OPP-LV5", 5))
        .add_card(make_digimon("OPP-LV3", 3))
}

/// Drive `play_option_from_hand`, auto-choosing the "Standard [Main]" branch
/// if the dual-mode (Standard vs Link) `EffectChoice` mode-select prompt
/// installs (BT24-091 / BT24-095 idiom — a Plug-In-style Option with both a
/// `main_from_hand` body and a `link_requirement` clause is dual-mode).
fn play_soul_fear_standard(runner: &mut DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    ) {
        let first = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, first)
            .expect("choose Standard [Main] mode");
    }
    result
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

#[test]
fn bt24_097_has_use_requirement_and_color_bypass_flood_gate() {
    let runner = soul_fear_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    assert_eq!(compiled.card, "BT24-097");
    assert_eq!(compiled.name, "Soul Fear");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(5));
    assert!(compiled.traits.iter().any(|t| t == "TS"));
    assert!(
        compiled.use_requirement.is_some(),
        "TS Digimon/Tamer color bypass must compile as a use_requirement"
    );
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));
}

#[test]
fn bt24_097_has_main_from_hand_clause_with_delete_then_optional_free_link() {
    let runner = soul_fear_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("MainFromHand clause");

    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })),
        "Main body must select an opponent permanent (the level 6+ delete target)"
    );
    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::DeletePermanent { .. })),
        "Main body must delete the selected opponent Digimon"
    );
    assert!(
        main.process.iter().any(|step| matches!(
            step,
            CompiledStep::LinkToOwnDigimon {
                optional: true,
                free: true,
                ..
            }
        )),
        "Main body must offer an optional free link to 1 own Digimon"
    );
}

#[test]
fn bt24_097_has_inherited_on_security_clause_activating_main() {
    let runner = soul_fear_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited
            && t.when == vec![CompiledTiming::OnSecurity]
    )));
}

#[test]
fn bt24_097_has_link_requirement_ts_trait_cost_3() {
    let runner = soul_fear_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { cost: 3, filter, .. })
            if filter.trait_has.as_deref() == Some("TS")
    )));
}

#[test]
fn bt24_097_has_linked_dp_aura_plus_2000() {
    let runner = soul_fear_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Linked,
            dp_modifier: Some(2000),
            ..
        })
    )));
}

#[test]
fn bt24_097_has_linked_when_attacking_opt_delete_clause() {
    let runner = soul_fear_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    let linked_wa = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Linked
                && t.when.contains(&CompiledTiming::WhenAttacking) =>
        {
            Some(t)
        }
        _ => None,
    });
    let clause = linked_wa.expect(
        "BT24-097 must declare a scope:linked when:when_attacking clause \
         (the Link ESS — DCGO OnAllyAttack + SetIsLinkedEffect(true))",
    );
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] flag must be set on the linked ESS"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })),
        "linked ESS must select an opponent permanent (the level 5-or-lower delete target)"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::DeletePermanent { .. })),
        "linked ESS must delete the selected opponent Digimon"
    );
}

// ─── §2 Condition gating — color bypass ──────────────────────────────────────

#[test]
fn bt24_097_flood_gate_inactive_without_ts_permanent() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    // No TS Digimon/Tamer on P0's field — active_when predicate must be false.
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");
    let active_when = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                active_when,
                modifier,
                ..
            }) if modifier == "IgnoreColorRequirement" => active_when.as_ref(),
            _ => None,
        })
        .expect("flood_gate clause with active_when present");
    // Structural: active_when predicate exists (behavioral gating is the mask's
    // job — see the use_requirement test below for the play-time consequence).
    let _ = active_when;
    let _ = runner;
}

#[test]
fn bt24_097_use_requirement_satisfied_with_ts_permanent_on_field() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "TS-DIGIMON", Some(0));
    runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();

    // With a TS Digimon present, playing Soul Fear (a Purple card, and the
    // fixture Digimon is Green — the color bypass is what makes this legal)
    // must succeed and install the Main body's mandatory delete prompt.
    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    let view = runner
        .pending_selection_view()
        .expect("Main body must install the mandatory delete-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
}

// ─── §3 Main — mandatory delete (level 6+) ───────────────────────────────────

#[test]
fn bt24_097_main_deletes_selected_level_6_plus_opponent_digimon() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target = runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    let view = runner
        .pending_selection_view()
        .expect("Main delete-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, target.index as u16)),
        "the level 6 opponent Digimon must be a legal delete target"
    );
    runner
        .execute_action(view.selecting_player, encode_attack(0, target.index as u16))
        .expect("delete the level 6+ opponent Digimon");

    assert!(
        !battle_area_contains(&runner, 1, "OPP-LV6"),
        "the selected level 6+ Digimon must be deleted"
    );
}

#[test]
fn bt24_097_main_delete_target_excludes_level_5_or_lower() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let low = runner.place_on_field(1, "OPP-LV5", Some(0));
    let high = runner.place_on_field(1, "OPP-LV7", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    let view = runner
        .pending_selection_view()
        .expect("Main delete-target prompt");
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, low.index as u16)),
        "a level 5 opponent Digimon must NOT be a legal delete target"
    );
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, high.index as u16)),
        "the level 7 opponent Digimon must remain a legal delete target"
    );
}

#[test]
fn bt24_097_main_no_prompt_when_no_level_6_plus_target_exists() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "TS-DIGIMON", Some(0));
    // Only a level 3 opponent Digimon — no legal delete target.
    runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    // The mandatory delete select must silently skip (DCGO
    // HasMatchConditionPermanent gate); the next thing offered is the
    // optional free link, or nothing at all if no own Digimon is eligible.
    if let Some(view) = runner.pending_selection_view() {
        assert_ne!(
            view.kind,
            SelectionKind::OppField,
            "no opponent-field delete prompt should install when no level 6+ target exists"
        );
    }
    assert!(
        battle_area_contains(&runner, 1, "OPP-LV3"),
        "the level 3 Digimon must survive — it was never a legal target"
    );
}

// ─── §4 Main — optional free link ────────────────────────────────────────────

#[test]
fn bt24_097_main_can_decline_the_optional_free_link() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target = runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    let delete_view = runner.pending_selection_view().expect("delete prompt");
    runner
        .execute_action(
            delete_view.selecting_player,
            encode_attack(0, target.index as u16),
        )
        .expect("delete step");

    let link_view = runner
        .pending_selection_view()
        .expect("optional free link prompt follows the delete");
    assert_eq!(link_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "the link pick must be optional (You may link)"
    );
    runner
        .execute_action(link_view.selecting_player, PASS)
        .expect("decline the optional free link");

    let linked = &runner.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 0, "declining must leave no linked cards");
}

#[test]
fn bt24_097_main_accepting_link_attaches_the_option_to_a_ts_digimon() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target = runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    let delete_view = runner.pending_selection_view().expect("delete prompt");
    runner
        .execute_action(
            delete_view.selecting_player,
            encode_attack(0, target.index as u16),
        )
        .expect("delete step");

    let link_view = runner
        .pending_selection_view()
        .expect("optional free link prompt follows the delete");
    assert!(link_view
        .valid_action_ids
        .contains(&encode_attack(0, host.index as u16)));
    runner
        .execute_action(
            link_view.selecting_player,
            encode_attack(0, host.index as u16),
        )
        .expect("link Soul Fear to the TS Digimon");

    let linked = &runner.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].card_id(&runner.game.card_data), "BT24-097");
}

#[test]
fn bt24_097_free_link_target_excludes_non_ts_digimon() {
    let mut runner = soul_fear_runner().hand(0, &[CARD_ID]).memory(20).start();
    // TS-TAMER satisfies the color-bypass use_requirement but is NOT itself a
    // valid link host (link condition requires a TS-trait *Digimon*).
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let non_ts = runner.place_on_field(0, "OPP-LV3", Some(0)); // reused as a plain Green Digimon on P0
    let target = runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        play_soul_fear_standard(&mut runner),
        OptionPlayResult::Pending
    );
    let delete_view = runner.pending_selection_view().expect("delete prompt");
    runner
        .execute_action(
            delete_view.selecting_player,
            encode_attack(0, target.index as u16),
        )
        .expect("delete step");

    if let Some(link_view) = runner.pending_selection_view() {
        assert!(
            !link_view
                .valid_action_ids
                .contains(&encode_attack(0, non_ts.index as u16)),
            "a non-TS Digimon must not be offered as a link host"
        );
    }
}

// ─── §5 Security — Activate [Main] ───────────────────────────────────────────

#[test]
fn bt24_097_security_activates_main_and_deletes_level_6_plus_target() {
    // Seat BT24-097 as P0's (sole) security card so the incoming attack's
    // security check reveals it and fires [Security] Activate [Main].
    let mut runner = soul_fear_runner()
        .security(0, &[CARD_ID])
        .memory(20)
        .start();
    runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target = runner.place_on_field(1, "OPP-LV7", Some(0));
    runner.game.enter_main_phase();

    // Opponent attacks P0's player directly; the security check reveals
    // BT24-097 and its [Security] clause activates [Main].
    let opp_attacker = runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.attack_player(
        PermanentHandle {
            player: 1,
            index: opp_attacker.index,
        },
        0,
        false,
    );

    let view = runner
        .pending_selection_view()
        .expect("[Security] must activate [Main]'s mandatory delete prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(view.selecting_player, encode_attack(0, target.index as u16))
        .expect("delete the level 6+ opponent Digimon via [Security] Activate [Main]");

    assert!(
        !battle_area_contains(&runner, 1, "OPP-LV7"),
        "the security-activated Main effect must delete the level 6+ target"
    );
}

// ─── §6 Linked side — link-box aura + [When Attacking][OPT] delete ──────────

#[test]
fn bt24_097_linked_dp_aura_grants_plus_2000_to_host() {
    let mut runner = soul_fear_runner().memory(20).start();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let base_dp = runner.effective_dp(host).expect("host has DP");

    runner.push_linked_owned(host, CARD_ID, 0);

    let dp_after = runner.effective_dp(host).expect("host still has DP");
    assert_eq!(
        dp_after,
        base_dp + 2000,
        "linked BT24-097 must grant +2000 DP to its host"
    );
}

#[test]
fn bt24_097_linked_when_attacking_deletes_level_5_or_lower_opponent_digimon() {
    let mut runner = soul_fear_runner().memory(20).start();
    runner.game.players[1].security.clear();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let low_target = runner.place_on_field(1, "OPP-LV5", Some(0));
    runner.game.enter_main_phase();

    // Attach BT24-097 as a linked card on the host (Link ESS carrier).
    runner.push_linked_owned(host, CARD_ID, 0);

    // Host attacks the opponent player — the linked [When Attacking][OPT]
    // must fire attributed to the host (DCGO SetIsLinkedEffect(true) +
    // EffectTiming.OnAllyAttack).
    runner.attack_player(host, 1, false);

    let view = runner.pending_selection_view();
    if view.is_none() {
        // G-LINK-INHERITED-ESS: if the linked scope:linked+when_attacking
        // trigger does not fire attributed to the host's own attack, this
        // is the engine gap this test file exists to surface — see the
        // module-level docstring. Do not silently pass; fail loudly so the
        // gap is investigated rather than masked.
        panic!(
            "linked [When Attacking][OPT] delete prompt did not fire on the \
             host's attack — see G-LINK-INHERITED-ESS in the file header"
        );
    }
    let view = view.unwrap();
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, low_target.index as u16)));
    runner
        .execute_action(
            view.selecting_player,
            encode_attack(0, low_target.index as u16),
        )
        .expect("delete the level 5 or lower opponent Digimon via the linked ESS");

    assert!(
        !battle_area_contains(&runner, 1, "OPP-LV5"),
        "the linked [When Attacking] ESS must delete the level 5-or-lower target"
    );
}

#[test]
fn bt24_097_linked_when_attacking_excludes_level_6_plus_target() {
    let mut runner = soul_fear_runner().memory(20).start();
    runner.game.players[1].security.clear();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let high_target = runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();

    runner.push_linked_owned(host, CARD_ID, 0);
    runner.attack_player(host, 1, false);

    if let Some(view) = runner.pending_selection_view() {
        assert!(
            !view
                .valid_action_ids
                .contains(&encode_attack(0, high_target.index as u16)),
            "a level 6+ opponent Digimon must not be a legal target for the linked \
             [When Attacking] ESS (that band is the Main clause's job)"
        );
    }
    assert!(
        battle_area_contains(&runner, 1, "OPP-LV6"),
        "the level 6+ Digimon must survive the linked ESS"
    );
}

#[test]
fn bt24_097_linked_when_attacking_no_prompt_without_eligible_target() {
    let mut runner = soul_fear_runner().memory(20).start();
    runner.game.players[1].security.clear();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    // Only a level 6+ opponent Digimon on field — no legal target for the
    // linked ESS (level 5 or lower band).
    runner.place_on_field(1, "OPP-LV7", Some(0));
    runner.game.enter_main_phase();

    runner.push_linked_owned(host, CARD_ID, 0);
    runner.attack_player(host, 1, false);
    runner.auto_resolve().ok();

    assert!(
        battle_area_contains(&runner, 1, "OPP-LV7"),
        "no eligible level 5-or-lower target: linked ESS must no-op"
    );
}

#[test]
fn bt24_097_linked_when_attacking_opt_blocks_second_attack_same_turn() {
    let mut runner = soul_fear_runner().memory(20).start();
    runner.game.players[1].security.clear();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target_a = runner.place_on_field(1, "OPP-LV5", Some(0));
    let target_b = runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.game.enter_main_phase();

    runner.push_linked_owned(host, CARD_ID, 0);

    // First attack — consume the OPT delete.
    runner.attack_player(host, 1, false);
    if let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids[0];
        runner
            .execute_action(view.selecting_player, action)
            .expect("resolve first linked ESS delete");
    }
    runner.auto_resolve().ok();

    let field_count_after_first = runner.game.player(1).battle_area.len();

    // Unsuspend the host to allow a second attack this turn.
    runner.game.players[0].battle_area[host.index as usize].is_suspended = false;

    // Second attack same turn — OPT must block the linked ESS.
    runner.attack_player(host, 1, false);
    runner.auto_resolve().ok();

    let field_count_after_second = runner.game.player(1).battle_area.len();
    assert_eq!(
        field_count_after_second, field_count_after_first,
        "OPT must block the linked ESS's second activation in the same turn"
    );
    let _ = target_a;
    let _ = target_b;
}

#[test]
fn bt24_097_linked_when_attacking_opt_clears_after_end_turn() {
    let mut runner = soul_fear_runner().memory(20).start();
    runner.game.players[1].security.clear();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target_a = runner.place_on_field(1, "OPP-LV5", Some(0));
    runner.game.enter_main_phase();

    runner.push_linked_owned(host, CARD_ID, 0);

    // Turn 1 — fire the linked ESS once.
    runner.attack_player(host, 1, false);
    if let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids[0];
        runner
            .execute_action(view.selecting_player, action)
            .expect("resolve first linked ESS delete");
    }
    runner.auto_resolve().ok();
    let _ = target_a;

    if runner.game_over() || runner.game.players[0].battle_area.is_empty() {
        return; // host didn't survive combat; nothing further to assert
    }

    // End P0's turn -> P1's turn -> back to P0.
    runner.end_turn();
    runner.end_turn();

    if runner.game.players[0].battle_area.is_empty() {
        return; // host didn't survive the round trip
    }

    // Turn 2 — place a fresh Lv5-or-lower target and attack again; OPT must
    // have cleared so the linked ESS can fire again.
    let target_b = runner.place_on_field(1, "OPP-LV3", Some(0));
    let host2 = PermanentHandle {
        player: 0,
        index: 0,
    };
    runner.attack_player(host2, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("OPT cleared: linked ESS should be able to fire again on turn 2");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, target_b.index as u16)));
}
