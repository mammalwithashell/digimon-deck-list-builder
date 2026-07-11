//! BT21-018 DoGatchmon — Digimon, Lv.4, Red/Green, DP 6000, Cost 6.
//!
//! # Card text (CORRECTED — transcribed from card image; authoritative)
//!
//! Trait line: Sup./Appmon | Social | Super Search/Hero
//!
//! Digivolve circles (official Bandai DB, data/card_bundles/BT21-018.md):
//!   Red Lv.3: Cost 3  AND  Green "Stnd." icon: Cost 3 (DB grades it Green
//!   Lv.3 / cost 3). Both authored as alt_paths at printed cost; the
//!   colour-free `trait_has: "Stnd."` path is the DCGO behavioral reading.
//!
//! App Fusion [Gatchmon] & [Navimon] & [Tweetmon]: Cost 0 — If 2 such cards
//!   are linked together, stack the link card on top and digivolve.
//!
//! ＜Rush＞ ＜Raid＞
//! [Your Turn] [Once Per Turn] When this Digimon gets linked, it may attack.
//!
//! Link box:
//! - Link [Appmon] trait: Cost 2
//! - Link DP bonus: +3000 DP
//! - [When Linking] This Digimon may attack. ("this Digimon" = the HOST)
//!
//! # DCGO C# reference (READ-ONLY, base repo)
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_018.cs
//!
//! Two WhenLinked blocks in DCGO:
//!   ~L68–138: host-side [Your Turn][OPT] when this gets linked → host may attack
//!             (timing WhenLinked, NO SetIsLinkedEffect)
//!   ~L144–205: linked-card side [When Linking] → HOST may attack
//!             (timing WhenLinked, SetIsLinkedEffect(true))
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - H1 Rush keyword grant (grant_keyword FaceUp)
//! - H9 Raid keyword grant (grant_keyword FaceUp)
//! - C1 link_condition (trait_has Appmon, cost 2)
//! - Link DP aura +3000 while linked (briefing §1.2, scope:linked aura dp_modifier 3000)
//! - App Fusion alt_path (Gatchmon / Navimon / Tweetmon)
//! - Stnd. form alt digivolve (from level 3 trait_has "Stnd.", cost 3)
//! - E2 OPT + optional (when_card_linked_to_this, your_turn gate)
//! - DigiLink Shape-B (scope:linked, when:when_linked) → host may attack

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT21-018";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Compute the action ID for the on-field "link this card" action.
/// Pass the permanent that IS being linked (the link card).
fn link_bit(perm: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Link `link_card_perm` to `host_perm` by activating the link action on the
/// link card and then resolving the host-target selection.
/// Returns the index of the host permanent after the link.
fn perform_link(
    r: &mut DebugRunner,
    link_card_perm: PermanentHandle,
    host_perm: PermanentHandle,
    controller: PlayerId,
) {
    // Activate the link action (fires the "link this card" slot on the link card permanent).
    r.game.decode_action(link_bit(link_card_perm), controller);
    // The engine installs a host-target selection — find the action that selects `host_perm`.
    // The valid_action_ids typically encode own-field targets via FIELD_EFFECT_START etc.
    // Just pick the first valid action (there should be exactly one valid Appmon host).
    if let Some(sel) = r.game.pending_selection.as_ref() {
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(controller, action);
    }
}

/// Inline YAML for BT21-018 DoGatchmon.

/// Inline YAML for a clean link card with ONLY a link_condition (no when_linked effects).
/// Use this as the link card in host-side tests so the link card's own when_linked effect
/// doesn't interfere with the DoGatchmon host-side trigger assertions.
const CLEAN_LINKER_YAML: &str = r#"
card: DSL-CLEAN-LINKER
name: Clean Linker
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Appmon]
attribute: Data
effects:
  - kind: link_condition
    cost: 1
    filter: { trait_has: Appmon }
"#;

/// Base builder for structural and most behavioral tests.
/// Uses from_dsl_yaml for BT21-018 to avoid embedded-pack caching issues.
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT21-018")
        .expect("BT21-018 YAML parses and compiles")
        // BT25-007 (Gatchmon) has link_condition targeting Appmon hosts, cost 1.
        // Used as the "link card" that attaches TO DoGatchmon (which has Appmon trait).
        .dsl_card("BT25-007")
        .expect("BT25-007 (Gatchmon) YAML parses")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon("OPP-DIG", 4, 5000, 5, &["Beast"]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
}

/// Builder variant for host-side (when_card_linked_to_this) behavioral tests.
/// Uses DSL-CLEAN-LINKER (no when_linked effects) so only DoGatchmon's host-side trigger fires.
fn base_host_test() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT21-018")
        .expect("BT21-018 YAML parses and compiles")
        .from_dsl_yaml(CLEAN_LINKER_YAML)
        .expect("DSL-CLEAN-LINKER YAML parses")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_digimon("OPP-DIG", 4, 5000, 5, &["Beast"]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_018_yaml_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "DoGatchmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
}

/// Rush is declared as a self (FaceUp scope) grant_keyword.
#[test]
fn bt21_018_has_rush_keyword_declarative() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has_rush = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Rush" && matches!(scope, CompiledScope::FaceUp)
        )
    });
    assert!(
        has_rush,
        "BT21-018 must declare <Rush> as a FaceUp grant_keyword"
    );
}

/// Raid is declared as a self (FaceUp scope) grant_keyword.
#[test]
fn bt21_018_has_raid_keyword_declarative() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has_raid = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Raid" && matches!(scope, CompiledScope::FaceUp)
        )
    });
    assert!(
        has_raid,
        "BT21-018 must declare <Raid> as a FaceUp grant_keyword"
    );
}

/// Link condition: self link-condition costs 2 for Appmon hosts.
#[test]
fn bt21_018_has_link_condition_appmon_cost_2() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 2
        )
    });
    assert!(
        has,
        "BT21-018 must declare a self link-condition with cost 2"
    );
}

/// App Fusion alt-path for Gatchmon/Navimon/Tweetmon is declared.
#[test]
fn bt21_018_has_app_fusion_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::AppFusion));
    assert!(has, "BT21-018 must declare an App Fusion alt_path");
}

/// App Fusion cost is 0.
#[test]
fn bt21_018_app_fusion_cost_is_zero() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let fusion = card
        .alt_paths
        .iter()
        .find(|p| matches!(p.kind, CompiledAltPathKind::AppFusion));
    assert!(fusion.is_some(), "App Fusion alt-path must exist");
    let fusion = fusion.unwrap();
    assert_eq!(
        fusion.cost,
        Some(CompiledCost::Literal(0)),
        "App Fusion cost must be 0"
    );
}

/// Stnd. form digivolve alt-path is declared with cost 3 and a "Stnd." trait
/// gate (DCGO HasStandardAppTraits — no level/colour gate).
#[test]
fn bt21_018_has_stnd_digivolve_alt_path_cost_3() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(3)))
            && p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("Stnd."))
    });
    assert!(
        has,
        "BT21-018 must declare a cost-3 digivolve alt-path gated on the Stnd. trait"
    );
}

/// Printed standard circle #1 (official Bandai DB): Red Lv.3 / Cost 3.
#[test]
fn bt21_018_has_red_lv3_standard_circle_cost_3() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(3)))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(3)
                    && f.color_is == Some(CompiledColor::Red)
                    && f.trait_has.is_none()
            })
    });
    assert!(
        has,
        "BT21-018 must declare its printed Red Lv.3 / cost 3 standard digivolve circle"
    );
}

/// Printed standard circle #2 (official Bandai DB): Green Lv.3 ("Stnd." icon
/// on the card face) / Cost 3.
#[test]
fn bt21_018_has_green_lv3_standard_circle_cost_3() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(3)))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(3)
                    && f.color_is == Some(CompiledColor::Green)
                    && f.trait_has.is_none()
            })
    });
    assert!(
        has,
        "BT21-018 must declare its printed Green Lv.3 (Stnd. icon) / cost 3 standard digivolve circle"
    );
}

/// The host-side when_card_linked_to_this triggered clause is OPT and optional.
#[test]
fn bt21_018_when_card_linked_to_this_is_opt_and_optional() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenCardLinkedToThis) => {
            Some(t)
        }
        _ => None,
    });
    let clause = clause.expect("must have when_card_linked_to_this triggered clause");
    assert!(
        clause.once_per_turn,
        "when_card_linked_to_this clause must be once_per_turn"
    );
    assert!(
        clause.optional,
        "when_card_linked_to_this clause must be optional (it may attack)"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "when_card_linked_to_this clause must be FaceUp scope (own card)"
    );
}

/// The linked-scope when_linked triggered clause ([When Linking] in link box) is declared.
#[test]
fn bt21_018_has_linked_scope_when_linked_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Linked
                    && t.when.contains(&CompiledTiming::WhenLinked)
        )
    });
    assert!(
        has,
        "must have a scope:linked when:when_linked triggered clause ([When Linking])"
    );
}

/// The linked-scope DP aura (+3000 while linked) is declared.
#[test]
fn bt21_018_has_linked_dp_aura_3000() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                dp_modifier: Some(3000),
                scope,
                ..
            }) if matches!(scope, CompiledScope::Linked)
        )
    });
    assert!(
        has,
        "BT21-018 must declare a scope:linked aura with dp_modifier 3000"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Rush: installed on field
// ═══════════════════════════════════════════════════════════════════════════════

/// <Rush> — DoGatchmon placed on field has Rush installed (grant_keyword
/// declarative makes it native via card_data_from_compiled).
#[test]
fn bt21_018_rush_installed_on_field() {
    let mut runner = base().start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .has_keyword(perm, digimon_engine::enums::Keyword::Rush),
        "DoGatchmon must have Rush on the field after being placed"
    );
}

/// <Raid> — DoGatchmon placed on field has Raid installed.
#[test]
fn bt21_018_raid_installed_on_field() {
    let mut runner = base().start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .has_keyword(perm, digimon_engine::enums::Keyword::Raid),
        "DoGatchmon must have Raid on the field after being placed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Host-side when_card_linked_to_this OPT attack prompt
// "When this Digimon gets linked, it may attack."
// (DoGatchmon is the HOST; another Appmon links TO DoGatchmon)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: [Your Turn][OPT] when another card links TO DoGatchmon, an optional
/// attack prompt surfaces on your turn.
/// Uses DSL-CLEAN-LINKER (no when_linked effects) so only DoGatchmon's host-side trigger fires.
#[test]
fn bt21_018_host_side_when_linked_prompts_attack_your_turn() {
    let mut r = base_host_test().memory(10).start();
    advance_to_main(&mut r);

    // DoGatchmon is the HOST.
    let dogatch = r.place_on_field(0, CARD_ID, Some(0));
    // DSL-CLEAN-LINKER is a simple Appmon linker with no when_linked effects.
    let linker = r.place_on_field(0, "DSL-CLEAN-LINKER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIG", Some(0));

    // Link DSL-CLEAN-LINKER TO DoGatchmon: activate link_bit(linker) → pick dogatch.
    perform_link(&mut r, linker, dogatch, 0);

    // After linking, when_card_linked_to_this OPT clause should fire on DoGatchmon
    // and surface an optional attack prompt.
    assert!(
        r.pending_selection().is_some(),
        "[Your Turn][OPT] when_card_linked_to_this must surface a prompt on your turn"
    );
    assert!(
        r.pending_is_optional(),
        "the attack prompt must be optional (it MAY attack)"
    );
}

/// PASS on the optional attack prompt does nothing to opponent board.
/// Uses DSL-CLEAN-LINKER (no when_linked effects) so only DoGatchmon's host-side trigger fires.
#[test]
fn bt21_018_host_side_when_linked_pass_does_nothing() {
    let mut r = base_host_test().memory(10).start();
    advance_to_main(&mut r);

    let dogatch = r.place_on_field(0, CARD_ID, Some(0));
    let linker = r.place_on_field(0, "DSL-CLEAN-LINKER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIG", Some(0));

    perform_link(&mut r, linker, dogatch, 0);

    let opp_ba_before = r.battle_area_size(1);
    // PASS all optional prompts.
    while r.pending_selection().is_some() && r.pending_is_optional() {
        let _ = r.execute_action(0u8, PASS);
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        opp_ba_before,
        "PASS on optional attack must not change opponent board"
    );
}

/// Negative gate ([Your Turn]): the host-side when_card_linked_to_this clause
/// must compile with an `active_when: { your_turn: true }` gate, so a link
/// happening on the opponent's turn (via an opponent effect) cannot fire it.
/// A behavioral opponent-turn link needs an opponent-side linking effect
/// (main-phase link actions belong to the turn player), so the gate is
/// asserted structurally on the compiled clause — the same predicate the
/// dispatcher evaluates at trigger time.
#[test]
fn bt21_018_when_card_linked_to_this_gated_to_your_turn() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenCardLinkedToThis) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have when_card_linked_to_this triggered clause");
    let gate = clause
        .active_when
        .as_ref()
        .expect("[Your Turn] must compile to an active_when gate");
    assert_eq!(
        gate.your_turn,
        Some(true),
        "the when_card_linked_to_this clause must be gated to [Your Turn]"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — OPT lockout on when_card_linked_to_this
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT: second link to DoGatchmon the same turn must NOT re-surface the
/// attack prompt. Uses DSL-CLEAN-LINKER (no when_linked effects), so after a
/// link the ONLY possible prompt source is DoGatchmon's host-side OPT clause.
/// The engine records the once-per-turn activation BEFORE the body's optional
/// attack selection surfaces (run_queued_effect_process_tail), so PASSing the
/// first prompt still consumes the OPT.
#[test]
fn bt21_018_opt_lockout_second_link_same_turn() {
    let mut r = base_host_test().memory(15).start();
    advance_to_main(&mut r);

    let dogatch = r.place_on_field(0, CARD_ID, Some(0));
    let linker1 = r.place_on_field(0, "DSL-CLEAN-LINKER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIG", Some(0));

    // First link → the host-side OPT attack prompt fires.
    perform_link(&mut r, linker1, dogatch, 0);
    assert!(
        r.pending_selection().is_some(),
        "first link must surface the host-side OPT attack prompt"
    );
    while r.pending_selection().is_some() && r.pending_is_optional() {
        let _ = r.execute_action(0u8, PASS);
    }
    let _ = r.auto_resolve();
    assert!(
        r.pending_selection().is_none(),
        "prompts must be fully drained after the first link"
    );

    // Second clean linker placed AFTER the first link resolved (battle-area
    // indices shift when a linked card leaves the area).
    let linker2 = r.place_on_field(0, "DSL-CLEAN-LINKER", Some(0));
    perform_link(&mut r, linker2, dogatch, 0);
    let _ = r.auto_resolve();

    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: a second link to DoGatchmon the same turn must NOT \
         re-surface the host-side attack prompt"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Linked DP aura: +3000 when DoGatchmon is a link card on a host
// ═══════════════════════════════════════════════════════════════════════════════

/// When DoGatchmon is linked TO a host Appmon, the host's effective DP rises by +3000.
#[test]
fn bt21_018_linked_dp_aura_raises_host_dp() {
    // Use a minimal setup: a generic Appmon host (no effects; just a field permanent).
    let mut r = DebugRunner::builder()
        .dsl_card("BT21-018")
        .expect("BT21-018")
        .add_card(make_digimon("APP-HOST", 4, 5000, 5, &["Appmon"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    // Place APP-HOST as the host that DoGatchmon will link to.
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    // Place DoGatchmon as the link card.
    let dogatch = r.place_on_field(0, CARD_ID, Some(0));

    let dp_before = r.effective_dp(host).unwrap_or(0);

    // Link DoGatchmon onto APP-HOST.
    perform_link(&mut r, dogatch, host, 0);
    // Drain any follow-up prompts (may_attack_now from when_linked).
    while r.pending_selection().is_some() && r.pending_is_optional() {
        let _ = r.execute_action(0u8, PASS);
    }
    let _ = r.auto_resolve();

    let dp_after = r.effective_dp(host).unwrap_or(0);
    assert_eq!(
        dp_after,
        dp_before + 3000,
        "APP-HOST effective DP must rise by +3000 while DoGatchmon is linked to it; \
         before={dp_before}, after={dp_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Linked-scope [When Linking]: host may attack
// "When Linking": DoGatchmon is the link card, the HOST may attack
// ═══════════════════════════════════════════════════════════════════════════════

/// [When Linking] (scope:linked, when:when_linked): when DoGatchmon gets linked
/// to a host, an optional attack prompt for the HOST surfaces.
#[test]
fn bt21_018_when_linking_surfaces_host_attack_prompt() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT21-018")
        .expect("BT21-018")
        .add_card(make_digimon("APP-HOST", 4, 5000, 5, &["Appmon"]))
        .add_card(make_digimon("OPP-TARGET", 4, 4000, 4, &["Beast"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    let host = r.place_on_field(0, "APP-HOST", Some(0));
    let dogatch = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-TARGET", Some(0));

    // Link DoGatchmon onto APP-HOST.
    perform_link(&mut r, dogatch, host, 0);

    // After linking, the scope:linked when:when_linked clause fires (on DoGatchmon as
    // the link card). source_permanent = host. may_attack_now { attacker: source } → host attacks.
    // An optional attack prompt must surface.
    assert!(
        r.pending_selection().is_some(),
        "[When Linking] scope:linked when:when_linked must surface an optional attack prompt for the host"
    );
    assert!(
        r.pending_is_optional(),
        "[When Linking] host attack must be optional (it MAY attack)"
    );
}

/// [When Linking] — PASS declines the host attack.
#[test]
fn bt21_018_when_linking_pass_declines() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT21-018")
        .expect("BT21-018")
        .add_card(make_digimon("APP-HOST", 4, 5000, 5, &["Appmon"]))
        .add_card(make_digimon("OPP-TARGET", 4, 4000, 4, &["Beast"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    let host = r.place_on_field(0, "APP-HOST", Some(0));
    let dogatch = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-TARGET", Some(0));

    perform_link(&mut r, dogatch, host, 0);

    let opp_ba_before = r.battle_area_size(1);

    // PASS all optional prompts.
    while r.pending_selection().is_some() && r.pending_is_optional() {
        let _ = r.execute_action(0u8, PASS);
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        opp_ba_before,
        "PASS on [When Linking] attack must not change opponent board"
    );
}
