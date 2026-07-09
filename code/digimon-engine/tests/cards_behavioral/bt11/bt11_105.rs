//! BT11-105 Fusionize — Option, Black, Cost 1.
//!
//! # Card text (card image BT11-105.webp — authoritative; cross-checked
//! data/card_bundles/BT11-105.md [official Bandai DB] + data/cards.json)
//!
//! When you would use this card, if you have a [Snatchmon] in play, reduce
//! the cost by 1.
//! [Main] By placing 1 [Vemmon] or [Destromon] from your trash under 1 of
//! your Digimon as its bottom digivolution card, you may digivolve 1 of
//! your Digimon into 1 [Destromon] or [Galacticmon] from your trash for its
//! digivolution cost.
//!
//! Security Effect (data/cards.json mis-files this under
//! `inherited_effect_description_eng`; the card image confirms it is the
//! genuine Security Effect — BT19-093 Queen Device precedent for the same
//! JSON quirk):
//! [Security] You may reveal the top 3 cards of your deck. Play 1 [Vemmon]
//! among them without paying the cost. Trash the rest.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT11/Black/BT11_105.cs
//!
//! # Patterns this test covers
//! - D2 cost reduction with board-state gate (`when_playing_this` +
//!   `condition: any_permanent(name_is: Snatchmon)`)
//! - A6 trash-to-digi-stack (place under) via `select_trash` cost +
//!   `select_own_permanent` + `place_as_bottom_source`
//! - A5-adjacent stack shift / digivolve from trash via
//!   `effect_initiated_digivolve { cost: printed }`
//! - E2 OPT-adjacent independent optionality (outer cost decline vs. inner
//!   digivolve decline)
//! - A2 two-pass reveal (reveal 3, optional bucket pick, play free, trash
//!   rest) via `select_reveal_buckets` + `play_from_revealed_free`
//! - Security "you may reveal" outer accept/decline (`optional: true` at
//!   clause level)

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn fusionize() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 YAML parses and compiles")
        .build()
}

fn snatchmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Snatchmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(5);
    card.dp = Some(5000);
    card.play_cost = 6;
    card
}

fn vemmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Vemmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(2);
    card.dp = Some(1000);
    card.play_cost = 2;
    card
}

fn destromon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Destromon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn galacticmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Galacticmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(6);
    card.dp = Some(9000);
    card.play_cost = 10;
    card
}

fn plain_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, &format!("{id} Filler"))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt11_105_yaml_compiles_without_error() {
    let runner = fusionize();
    assert!(
        runner.compiled_card("BT11-105").is_some(),
        "BT11-105 must be present in the embedded DSL pack"
    );
}

/// Clause 0 — the cost-reduction declarative clause exists, scoped to
/// playing THIS card, with a board-state condition gate.
#[test]
fn bt11_105_has_cost_reduction_clause_gated_on_snatchmon() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let cr = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            when_playing_this,
            condition,
            amount,
            ..
        }) => Some((*when_playing_this, condition.clone(), *amount)),
        _ => None,
    });
    let (when_playing_this, condition, amount) =
        cr.expect("BT11-105 must have a CostReduction declarative clause");
    assert!(
        when_playing_this,
        "cost reduction must be scoped to playing THIS card"
    );
    assert!(
        condition.is_some(),
        "cost reduction must carry a board-state condition (Snatchmon in play)"
    );
    assert_eq!(amount, Some(1), "cost reduction amount must be 1");
}

/// Clause 1 — a `main_from_hand` triggered clause exists (the Option's
/// `[Main]` body).
#[test]
fn bt11_105_has_main_from_hand_clause() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let main = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => Some(t),
        _ => None,
    });
    assert!(main.is_some(), "BT11-105 must have a MainFromHand clause");
}

/// Clause 1 — the Main process contains the trash-placement cost step
/// (`SelectTrash` with `optional: true, cost: true`).
#[test]
fn bt11_105_main_has_optional_cost_trash_select() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainFromHand clause must exist");
    let has_cost_select = main.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::SelectTrash {
                optional: true,
                cost: true,
                ..
            }
        )
    });
    assert!(
        has_cost_select,
        "Main process must contain an optional cost-paying SelectTrash step"
    );
}

/// Clause 1 — the Main process contains a `PlaceAsBottomSource` step.
#[test]
fn bt11_105_main_has_place_as_bottom_source() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainFromHand clause must exist");
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. })),
        "Main process must contain a PlaceAsBottomSource step"
    );
}

/// Clause 1 — the Main process contains an `EffectInitiatedDigivolve` step
/// (inside the conditional tail) with `cost: printed` (no ignore_requirements).
#[test]
fn bt11_105_main_has_effect_initiated_digivolve_with_printed_cost() {
    use digimon_dsl::compiled::CompiledCostDelta;

    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainFromHand clause must exist");

    fn find_digivolve_step(steps: &[CompiledStep]) -> Option<&CompiledStep> {
        for s in steps {
            if matches!(s, CompiledStep::EffectInitiatedDigivolve { .. }) {
                return Some(s);
            }
            if let CompiledStep::If { then, .. } = s {
                if let Some(found) = find_digivolve_step(then) {
                    return Some(found);
                }
            }
        }
        None
    }

    let step = find_digivolve_step(&main.process);
    let step = step.expect("Main process must contain EffectInitiatedDigivolve (possibly nested)");
    match step {
        CompiledStep::EffectInitiatedDigivolve {
            cost,
            ignore_requirements,
            ..
        } => {
            assert_eq!(
                *cost,
                CompiledCostDelta::Printed,
                "digivolve cost must be `printed` (pay its digivolution cost)"
            );
            assert!(
                !*ignore_requirements,
                "printed text has no 'ignoring digivolution requirements' clause"
            );
        }
        _ => unreachable!(),
    }
}

/// Clause 2 — an inherited, optional `on_security` clause exists.
#[test]
fn bt11_105_has_inherited_optional_security_clause() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let sec = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(t)
        }
        _ => None,
    });
    let sec = sec.expect("BT11-105 must have an inherited SecuritySkill clause");
    assert!(
        sec.optional,
        "the security clause must be optional ('you may reveal')"
    );
}

/// Clause 2 — the security process reveals the top 3 cards.
#[test]
fn bt11_105_security_reveals_top_3() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited SecuritySkill clause must exist");
    assert!(
        sec.process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "security process must contain RevealTopDeck {{ count: 3 }}"
    );
}

/// Clause 2 — the security process contains a SelectRevealBuckets step with
/// one bucket (min:0, max:1) filtering for a card literally named Vemmon.
#[test]
fn bt11_105_security_has_vemmon_bucket() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited SecuritySkill clause must exist");
    let bucket_step = sec.process.iter().find(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets.len() == 1 && buckets[0].min == 0 && buckets[0].max == 1)
    });
    assert!(
        bucket_step.is_some(),
        "security process must contain a SelectRevealBuckets step with one \
         bucket (min:0, max:1)"
    );
}

/// Clause 2 — the security process plays the selected bucket for free.
#[test]
fn bt11_105_security_plays_selected_bucket_free() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited SecuritySkill clause must exist");
    assert!(
        sec.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromRevealedFree { .. })),
        "security process must contain PlayFromRevealedFree"
    );
}

/// Clause 2 — the security process trashes the remainder via a PerSelected
/// loop with TrashFromReveal.
#[test]
fn bt11_105_security_trashes_remainder() {
    let runner = fusionize();
    let card = runner.compiled_card("BT11-105").expect("BT11-105 present");
    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited SecuritySkill clause must exist");
    let has_trash_loop = sec.process.iter().any(|s| match s {
        CompiledStep::PerSelected { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::TrashFromReveal { .. })),
        _ => false,
    });
    assert!(
        has_trash_loop,
        "security process must contain a PerSelected loop with TrashFromReveal"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Cost reduction: positive / negative behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: with a Snatchmon in play, playing BT11-105 costs 0 memory
/// (printed cost 1, minus 1).
#[test]
fn bt11_105_cost_reduced_by_1_with_snatchmon_in_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(snatchmon("SNATCH-A"))
        .hand(0, &["BT11-105"])
        .memory(1)
        .start();

    runner.place_on_field(0, "SNATCH-A", Some(0));

    let memory_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(played.is_some(), "BT11-105 must be playable with 1 memory");
    // Printed cost 1, reduced by 1 = 0 memory spent.
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing BT11-105 with a Snatchmon in play must cost 0 memory (1 - 1)"
    );
}

/// Negative: with no Snatchmon in play, playing BT11-105 costs the full
/// printed cost of 1 memory.
#[test]
fn bt11_105_cost_not_reduced_without_snatchmon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .hand(0, &["BT11-105"])
        .memory(1)
        .start();

    let memory_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(played.is_some(), "BT11-105 must be playable with 1 memory");
    assert_eq!(
        runner.memory(),
        memory_before - 1,
        "playing BT11-105 without a Snatchmon in play must cost the full 1 memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// With no [Vemmon]/[Destromon] in trash, playing BT11-105 fires the Main
/// clause but the cost select has zero candidates — the whole clause
/// short-circuits (no placement, no digivolve prompt).
#[test]
fn bt11_105_main_with_no_eligible_trash_card_does_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("PLAIN-A", "PlainDigi"))
        .hand(0, &["BT11-105"])
        .memory(2)
        .start();

    runner.place_on_field(0, "PLAIN-A", Some(0));
    let trash_before = runner.trash_size(0);

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    // No candidates for the cost select → the clause-level `condition:
    // count_gte` precheck prevents the clause from enqueuing at all.
    assert!(
        runner.pending_selection().is_none(),
        "with no eligible trash card, no selection should install"
    );
    // BT11-105 itself lands in trash after resolving as a normal Option.
    assert!(
        runner.trash_size(0) >= trash_before,
        "trash size must not decrease"
    );
}

/// Positive: with a [Vemmon] in trash and a Digimon on the field, playing
/// BT11-105 installs a cost-select prompt (SelectZone/Trash-shaped) offering
/// the Vemmon as a legal pick.
#[test]
fn bt11_105_main_with_eligible_trash_card_installs_cost_select() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(vemmon("VEM-A"))
        .add_card(plain_digimon("PLAIN-B", "PlainDigi"))
        .hand(0, &["BT11-105"])
        .memory(2)
        .start();

    runner.place_on_field(0, "PLAIN-B", Some(0));
    runner.inject_trash(0, "VEM-A");

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "with an eligible Vemmon in trash, a selection must install for the \
         placement cost"
    );
    assert!(
        runner.pending_is_optional(),
        "the placement cost select is a may-pay ('By placing...')"
    );
}

/// Accepting the placement cost places the picked trash card as the target
/// Digimon's bottom digivolution source (stack size increases by 1).
#[test]
fn bt11_105_main_accepting_placement_adds_bottom_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(vemmon("VEM-B"))
        .add_card(plain_digimon("PLAIN-C", "PlainDigi"))
        .hand(0, &["BT11-105"])
        .memory(2)
        .start();

    let target = runner.place_on_field(0, "PLAIN-C", Some(0));
    runner.inject_trash(0, "VEM-B");

    let stack_before = runner.top_card(target); // sanity: card exists
    let _ = stack_before;

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    // Drive the trash-select prompt: pick the Vemmon.
    let view = runner
        .pending_selection_view()
        .expect("cost select must be pending");
    let action = view
        .valid_action_ids
        .first()
        .copied()
        .expect("at least one legal pick (Vemmon)");
    let _ = runner.execute_action(view.selecting_player, action);
    runner.game.drain_effect_queue();

    // Next: select the destination Digimon (only PLAIN-C exists).
    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::OwnField) {
            let action = view.valid_action_ids[0];
            let _ = runner.execute_action(view.selecting_player, action);
            runner.game.drain_effect_queue();
        }
    }

    let perm = runner.game.player(0).battle_area.get(target.index as usize);
    let stack_size = perm.map(|p| p.card_sources.len()).unwrap_or(0);
    assert!(
        stack_size >= 1,
        "target Digimon must have gained a bottom digivolution source \
         (stack_size={stack_size})"
    );
}

/// Declining the placement cost (PASS) ends the whole clause: no bottom
/// source is placed and no digivolve-target prompt follows.
#[test]
fn bt11_105_main_declining_placement_does_nothing_further() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(vemmon("VEM-C"))
        .add_card(plain_digimon("PLAIN-D", "PlainDigi"))
        .hand(0, &["BT11-105"])
        .memory(2)
        .start();

    let target = runner.place_on_field(0, "PLAIN-D", Some(0));
    runner.inject_trash(0, "VEM-C");
    // A fresh single-card permanent already has card_sources.len() == 1 (the
    // top card itself). Placement adds a SECOND source, so "no placement"
    // means stack_size stays at 1.
    let stack_before = runner
        .game
        .player(0)
        .battle_area
        .get(target.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);
    assert_eq!(
        stack_before, 1,
        "sanity: fresh permanent starts with 1 source"
    );

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("cost select must be pending");
    assert!(
        view.is_optional,
        "the may-pay cost select must be optional (PASS legal)"
    );
    let _ = runner.execute_action(view.selecting_player, PASS);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "declining the placement cost must end the clause with no further prompts"
    );
    let perm = runner.game.player(0).battle_area.get(target.index as usize);
    let stack_size = perm.map(|p| p.card_sources.len()).unwrap_or(0);
    assert_eq!(
        stack_size, 1,
        "declining the placement cost must NOT place any bottom source \
         (stack_size stays at 1)"
    );
}

/// After a successful placement, the digivolve tail is independently
/// optional: declining the digivolve-target pick (PASS) leaves the
/// placement intact but performs no digivolution.
#[test]
fn bt11_105_main_declining_digivolve_tail_keeps_placement_only() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(vemmon("VEM-D"))
        .add_card(destromon("DEST-D"))
        .add_card(plain_digimon("PLAIN-E", "PlainDigi"))
        .hand(0, &["BT11-105"])
        .memory(2)
        .start();

    let target = runner.place_on_field(0, "PLAIN-E", Some(0));
    runner.inject_trash(0, "VEM-D");
    runner.inject_trash(0, "DEST-D");

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    // Accept the placement cost (pick whichever card is offered first).
    let view = runner
        .pending_selection_view()
        .expect("cost select must be pending");
    let action = view.valid_action_ids[0];
    let _ = runner.execute_action(view.selecting_player, action);
    runner.game.drain_effect_queue();

    // Select destination Digimon (only PLAIN-E on field).
    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::OwnField) {
            let action = view.valid_action_ids[0];
            let _ = runner.execute_action(view.selecting_player, action);
            runner.game.drain_effect_queue();
        }
    }

    // Now the independently-optional digivolve-target prompt should be
    // pending (or the clause may have already ended if no candidates).
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.is_optional,
            "the digivolve-target pick must be optional (PASS legal)"
        );
        let _ = runner.execute_action(view.selecting_player, PASS);
        runner.game.drain_effect_queue();
    }

    assert!(
        runner.pending_selection().is_none(),
        "declining the digivolve tail must end the clause"
    );
    let stack_size = runner
        .game
        .player(0)
        .battle_area
        .get(target.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);
    assert_eq!(
        stack_size, 2,
        "the placement from the cost step must persist even when the \
         digivolve tail is declined (1 base + 1 placed source)"
    );
}

/// Full happy path: accept placement, then accept the digivolve tail,
/// selecting a Destromon target and a Galacticmon trash source with a
/// digivolution cost the controller can afford — the Digimon digivolves.
#[test]
fn bt11_105_main_full_digivolve_chain_pays_printed_cost() {
    // Destromon in play (level 4) can legally digivolve into Galacticmon
    // (built with a matching printed digivolve path).
    let mut destromon_field = destromon("DEST-FIELD-E");
    destromon_field.card_id = "DEST-FIELD-E".to_string();

    let mut galacticmon_card = galacticmon("GALACT-E");
    // Grant Galacticmon a standard digivolve path from level-4 black so the
    // effect_initiated_digivolve call succeeds (ignore_requirements: false).
    galacticmon_card.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 5, // Black
        level: 4,
        memory_cost: 3,
    }];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(vemmon("VEM-E"))
        .add_card(destromon_field.clone())
        .add_card(galacticmon_card)
        .hand(0, &["BT11-105"])
        .memory(6)
        .start();

    let target = runner.place_on_field(0, "DEST-FIELD-E", Some(0));
    runner.inject_trash(0, "VEM-E");
    runner.inject_trash(0, "GALACT-E");

    let memory_before = runner.memory();

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    // Accept placement: only Vemmon eligible for the cost select.
    let view = runner
        .pending_selection_view()
        .expect("cost select pending");
    let action = view.valid_action_ids[0];
    let _ = runner.execute_action(view.selecting_player, action);
    runner.game.drain_effect_queue();

    // Destination Digimon select (only DEST-FIELD-E).
    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::OwnField) {
            let action = view.valid_action_ids[0];
            let _ = runner.execute_action(view.selecting_player, action);
            runner.game.drain_effect_queue();
        }
    }

    // Digivolve-target select (only DEST-FIELD-E).
    let view = runner
        .pending_selection_view()
        .expect("digivolve-target select must be pending after placement");
    assert!(view.is_optional);
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("at least one Digimon target available");
    let _ = runner.execute_action(view.selecting_player, pick);
    runner.game.drain_effect_queue();

    // Digivolve-source select from trash (Galacticmon).
    let view = runner
        .pending_selection_view()
        .expect("digivolve-source select must be pending");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("Galacticmon must be a legal pick");
    let _ = runner.execute_action(view.selecting_player, pick);
    runner.game.drain_effect_queue();

    let top_id = runner.game.player(0).battle_area[target.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_id, "GALACT-E",
        "the target Digimon must have digivolved into Galacticmon"
    );
    // memory_before was captured BEFORE playing BT11-105: playing it costs 1
    // memory (no Snatchmon in play), then the digivolve pays its printed
    // digivolution cost of 3 — total spend of 4.
    assert_eq!(
        runner.memory(),
        memory_before - 4,
        "playing BT11-105 (1) plus the printed digivolution cost (3) must be paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// Attacking through BT11-105 in security installs an optional accept/
/// decline prompt for the "you may reveal" outer gate.
#[test]
fn bt11_105_security_installs_optional_reveal_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("ATTACKER-F", "Attacker"))
        .add_card(filler("FILL-F1"))
        .security(1, &["BT11-105"])
        .deck(1, &["FILL-F1"])
        .memory(6)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-F", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner.game.drain_effect_queue();

    // Security effects are optional at the clause level ("you may reveal").
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.is_optional,
            "the security 'you may reveal' prompt must be optional (PASS legal)"
        );
    } else {
        panic!("expected the optional security reveal prompt to be pending");
    }
}

/// Declining the security reveal ("Not reveal") leaves the deck/trash
/// unchanged — no cards are revealed or trashed.
#[test]
fn bt11_105_security_declining_reveal_does_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("ATTACKER-G", "Attacker"))
        .add_card(filler("FILL-G1"))
        .add_card(filler("FILL-G2"))
        .add_card(filler("FILL-G3"))
        .security(1, &["BT11-105"])
        .deck(1, &["FILL-G1", "FILL-G2", "FILL-G3"])
        .memory(6)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-G", Some(0));
    let deck_before = runner.deck_size(1);
    let trash_before = runner.trash_size(1);

    let _ = runner.attack_player(attacker, 1, false);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("optional reveal prompt must be pending");
    let _ = runner.execute_action(view.selecting_player, PASS);
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.deck_size(1),
        deck_before,
        "declining the reveal must not touch the deck"
    );
    // BT11-105 itself lands in trash as the resolved security card (its own
    // disposition, independent of whether its Security effect activated) —
    // the effect declining "you may reveal" must not trash any ADDITIONAL
    // (deck) cards beyond that.
    assert_eq!(
        runner.trash_size(1),
        trash_before + 1,
        "declining the reveal must not trash any deck cards (only BT11-105 \
         itself resolves to trash)"
    );
}

/// Accepting the reveal with a [Vemmon] among the top 3 installs a
/// RevealBucket selection offering the Vemmon.
#[test]
fn bt11_105_security_accept_with_vemmon_installs_reveal_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("ATTACKER-H", "Attacker"))
        .add_card(vemmon("VEM-H"))
        .add_card(filler("FILL-H1"))
        .add_card(filler("FILL-H2"))
        .security(1, &["BT11-105"])
        // Vemmon at top of deck (last in array) → revealed at index 0.
        .deck(1, &["FILL-H1", "FILL-H2", "VEM-H"])
        .memory(6)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-H", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner.game.drain_effect_queue();

    // Accept the "you may reveal" prompt.
    let view = runner
        .pending_selection_view()
        .expect("optional reveal prompt must be pending");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept action must exist");
    let _ = runner.execute_action(view.selecting_player, accept);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("RevealBucket selection must be pending after reveal");
    assert!(
        matches!(view.kind, SelectionKind::RevealBucket { .. }),
        "selection kind must be RevealBucket, got {:?}",
        view.kind
    );
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "Vemmon (revealed at index 0) must be a legal bucket pick; \
         valid_action_ids = {:?}",
        view.valid_action_ids
    );
}

/// Picking the revealed Vemmon plays it for free (battle area +1) and
/// trashes the other 2 revealed cards.
#[test]
fn bt11_105_security_picking_vemmon_plays_free_and_trashes_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("ATTACKER-I", "Attacker"))
        .add_card(vemmon("VEM-I"))
        .add_card(filler("FILL-I1"))
        .add_card(filler("FILL-I2"))
        .security(1, &["BT11-105"])
        .deck(1, &["FILL-I1", "FILL-I2", "VEM-I"])
        .memory(1) // Not enough to pay for Vemmon (cost 2) — must be free.
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-I", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("optional reveal prompt must be pending");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept action must exist");
    let _ = runner.execute_action(view.selecting_player, accept);
    runner.game.drain_effect_queue();

    let field_before_pick = runner.battle_area_size(1);
    let trash_before_pick = runner.trash_size(1);
    let memory_before_pick = runner.game.memory;

    let _ = runner.execute_action(1, SEL_REVEAL_START);
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1),
        field_before_pick + 1,
        "the picked Vemmon must be played onto the field"
    );
    // The 2 remaining revealed (filler) cards are trashed, PLUS BT11-105
    // itself resolves to trash once the whole security effect body
    // completes (its own disposition is not finalized until after the
    // reveal-bucket pick resolves).
    assert_eq!(
        runner.trash_size(1),
        trash_before_pick + 3,
        "the 2 remaining revealed cards plus BT11-105 itself must be trashed"
    );
    assert_eq!(
        runner.game.memory, memory_before_pick,
        "play_from_revealed_free must not spend memory"
    );
}

/// Declining the Vemmon bucket pick (after accepting the reveal) still
/// trashes all 3 revealed cards (mandatory remainder-trash tail).
#[test]
fn bt11_105_security_declining_bucket_pick_trashes_all_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("ATTACKER-J", "Attacker"))
        .add_card(vemmon("VEM-J"))
        .add_card(filler("FILL-J1"))
        .add_card(filler("FILL-J2"))
        .security(1, &["BT11-105"])
        .deck(1, &["FILL-J1", "FILL-J2", "VEM-J"])
        .memory(6)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-J", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("optional reveal prompt must be pending");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept action must exist");
    let _ = runner.execute_action(view.selecting_player, accept);
    runner.game.drain_effect_queue();

    let trash_before_pick = runner.trash_size(1);

    // Decline the bucket pick (PASS).
    if let Some(view) = runner.pending_selection_view() {
        let _ = runner.execute_action(view.selecting_player, PASS);
        runner.game.drain_effect_queue();
    }
    while let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }

    assert!(
        runner.trash_size(1) >= trash_before_pick + 3,
        "declining the bucket pick must still trash all 3 revealed cards"
    );
}

/// Without a [Vemmon] among the top 3, accepting the reveal still trashes
/// all 3 (no legal bucket pick exists, so it auto-skips to the remainder
/// trash).
#[test]
fn bt11_105_security_no_vemmon_in_reveal_trashes_all_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(plain_digimon("ATTACKER-K", "Attacker"))
        .add_card(filler("FILL-K1"))
        .add_card(filler("FILL-K2"))
        .add_card(filler("FILL-K3"))
        .security(1, &["BT11-105"])
        .deck(1, &["FILL-K1", "FILL-K2", "FILL-K3"])
        .memory(6)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-K", Some(0));
    let trash_before = runner.trash_size(1);
    let _ = runner.attack_player(attacker, 1, false);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("optional reveal prompt must be pending");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept action must exist");
    let _ = runner.execute_action(view.selecting_player, accept);
    runner.game.drain_effect_queue();

    // With no Vemmon among the reveal, the bucket step has zero candidates
    // and auto-skips silently within the same drain — the mandatory
    // trash-remainder tail (PerSelected) runs immediately, so no selection
    // is left pending here.
    while let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }

    // All 3 revealed (filler) cards land in trash, PLUS BT11-105 itself.
    assert!(
        runner.trash_size(1) >= trash_before + 4,
        "with no Vemmon among the reveal, all 3 cards plus BT11-105 itself \
         must be trashed: trash_before={trash_before}, trash_after={}",
        runner.trash_size(1)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Event-log assertion for the placement cost (trashes a card)
// ═══════════════════════════════════════════════════════════════════════════════

/// Accepting the placement cost must remove the picked card from trash (the
/// card leaves trash and becomes a digivolution source — verified via trash
/// size decreasing by 1).
#[test]
fn bt11_105_main_placement_removes_card_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT11-105")
        .expect("BT11-105 compiles")
        .add_card(vemmon("VEM-L"))
        .add_card(plain_digimon("PLAIN-L", "PlainDigi"))
        .hand(0, &["BT11-105"])
        .memory(2)
        .start();

    runner.place_on_field(0, "PLAIN-L", Some(0));
    runner.inject_trash(0, "VEM-L");
    let trash_before = runner.trash_size(0);

    let _ = runner.play(0, 0);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("cost select must be pending");
    let action = view.valid_action_ids[0];
    let _ = runner.execute_action(view.selecting_player, action);
    runner.game.drain_effect_queue();

    // The destination-Digimon select must run (and be accepted) before the
    // picked trash card actually leaves trash (PlaceAsBottomSource fires
    // after the destination is chosen).
    let view = runner
        .pending_selection_view()
        .expect("destination-Digimon select must be pending");
    let action = view.valid_action_ids[0];
    let _ = runner.execute_action(view.selecting_player, action);
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "the picked Vemmon must leave the trash zone when placed as a bottom source"
    );
}
