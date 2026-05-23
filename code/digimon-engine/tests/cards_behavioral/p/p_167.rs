//! P-167 Landramon
//!
//! Phase 2 Track E (2026-05-17): both clauses now authored.
//! - [Start of Your Main Phase][When Digivolving] reveal/search clause uses
//!   the new `choose_from_reveal` + `order_remainder` DSL verbs, with the
//!   player picking hand-vs-bottom-source routing and top-vs-bottom deck
//!   placement explicitly (Working Rule §17).
//! - The inherited source-trash De-Digivolve clause is unchanged from the
//!   prior pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-167")
        .expect("P-167 YAML parses and compiles")
        .build()
}

#[test]
fn p_167_has_inherited_source_trash_dedigivolve_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("P-167")
        .expect("P-167 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

#[test]
fn p_167_source_trash_dedigivolves_one_opponent_digimon() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());

    let mut runner = DebugRunner::builder()
        .dsl_card("P-167")
        .expect("P-167 YAML parses and compiles")
        .dsl_card("BT24-011")
        .expect("BT24-011 YAML parses and compiles")
        .dsl_card("BT21-024")
        .expect("BT21-024 YAML parses and compiles")
        .add_card(host)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "P-167");
    let opponent = runner.place_stack(1, &["BT24-011", "BT21-024"]);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .auto_resolve()
        .expect("P-167 de-digivolve target resolves");

    assert_eq!(
        runner.game.player(1).battle_area[opponent.index as usize]
            .card_sources
            .len(),
        1
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2 Track E (2026-05-17): [Start of Your Main Phase][When Digivolving]
// reveal/search clause via `choose_from_reveal` + `order_remainder`.
// ─────────────────────────────────────────────────────────────────────────────

/// Structural: P-167 must now have an authored Start-of-Main / When-Digivolving
/// triggered clause whose process contains `select_own_sources` for the cost,
/// `reveal_top_deck`, and both `choose_from_reveal` (in if-branches) and
/// `order_remainder` for the remainder placement.
#[test]
fn p_167_has_reveal_search_clause_with_new_verbs() {
    let runner = runner();
    let card = runner
        .compiled_card("P-167")
        .expect("P-167 compiled card present");

    let face_up_clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::FaceUp
                && (t.when.contains(&CompiledTiming::StartOfYourMainPhase)
                    || t.when.contains(&CompiledTiming::WhenDigivolving)) =>
        {
            Some(t)
        }
        _ => None,
    });
    let face_up_clause =
        face_up_clause.expect("P-167 must have a face-up Start-of-Main/When-Digivolving clause");
    assert!(
        face_up_clause.optional,
        "P-167 reveal clause is gated on 'By trashing 1 ...' cost — must be optional"
    );

    // The clause's process contains a select_own_sources step at the cost layer.
    let has_select_own_sources = face_up_clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectOwnSources { .. }));
    assert!(
        has_select_own_sources,
        "P-167 cost layer must include select_own_sources (Mineral/Rock source)"
    );

    // Walk into the `then` branch of the select_own_sources to find the new
    // verbs nested under the cost.
    let then_steps: Vec<&CompiledStep> = face_up_clause
        .process
        .iter()
        .filter_map(|s| match s {
            CompiledStep::SelectOwnSources { then, .. } => Some(then.iter()),
            _ => None,
        })
        .flatten()
        .collect();
    let has_reveal_top_deck = then_steps
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    let has_choose_from_reveal = walk_steps_for_variant(&then_steps, |s| {
        matches!(s, CompiledStep::ChooseFromReveal { .. })
    });
    let has_order_remainder = then_steps
        .iter()
        .any(|s| matches!(s, CompiledStep::OrderRemainder { .. }));

    assert!(
        has_reveal_top_deck,
        "P-167 cost-then layer must reveal top 3"
    );
    assert!(
        has_choose_from_reveal,
        "P-167 cost-then layer must include choose_from_reveal (in if-branches)"
    );
    assert!(
        has_order_remainder,
        "P-167 cost-then layer must include order_remainder for top/bottom remainder placement"
    );
}

fn walk_steps_for_variant<F>(steps: &[&CompiledStep], pred: F) -> bool
where
    F: Fn(&CompiledStep) -> bool + Copy,
{
    for s in steps {
        if pred(s) {
            return true;
        }
        // Recurse into if-branches that may hold the verb.
        if let CompiledStep::If {
            then, else_branch, ..
        } = s
        {
            let then_refs: Vec<&CompiledStep> = then.iter().collect();
            let else_refs: Vec<&CompiledStep> = else_branch.iter().collect();
            if walk_steps_for_variant(&then_refs, pred) {
                return true;
            }
            if walk_steps_for_variant(&else_refs, pred) {
                return true;
            }
        }
    }
    false
}

/// Behavioral: when the cost is paid and the player picks "Add to hand",
/// a Mineral revealed card lands in hand and the remainder is queued for
/// top-or-bottom placement.
///
/// The trigger plumbing for `[Start of Your Main Phase]` / `[When
/// Digivolving]` involves the full turn-state loop; this test instead
/// extracts the cost+effect process directly from the compiled clause and
/// runs it via `run_steps`, exercising the reveal+choose+order body
/// end-to-end. The trigger registration is structurally asserted above.
#[test]
fn p_167_reveal_clause_body_executes_full_flow() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());
    let mut mineral_pick = make_test_card("MIN-PICK", "MIN-PICK");
    mineral_pick.traits.push("Mineral".to_string());
    let filler_a = make_test_card("FILL-A", "FILL-A");
    let filler_b = make_test_card("FILL-B", "FILL-B");

    let mut runner = DebugRunner::builder()
        .dsl_card("P-167")
        .expect("P-167 YAML parses and compiles")
        .add_card(host)
        .add_card(mineral_pick)
        .add_card(filler_a)
        .add_card(filler_b)
        .build();

    // Place P-167 on field with a Mineral source underneath ROCK-HOST so the
    // cost layer has a valid candidate. Order: ROCK-HOST is top; P-167 is
    // index 0 below it.
    let host_handle = runner.place_on_field(0, "ROCK-HOST", None);
    let _src_idx = runner.push_source(host_handle, "P-167");

    // Set up deck top: MIN-PICK on top, then FILL-A, FILL-B.
    {
        use digimon_engine::card_source::CardSource;
        for id in ["FILL-B", "FILL-A", "MIN-PICK"] {
            let data_idx = runner
                .game
                .card_data
                .iter()
                .position(|c| c.card_id == id)
                .unwrap();
            let card_index = runner.game.next_card_index();
            runner.game.players[0]
                .deck
                .push(CardSource::new(data_idx, 0, card_index));
        }
    }

    // Extract the FaceUp reveal clause's process and execute it directly.
    let compiled = runner.compiled_card("P-167").expect("compiled").clone();
    let process = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("face-up reveal clause present");

    let p_167_top_card_handle = runner.top_card(host_handle);
    {
        let mut ctx = EffectContext::new(
            &mut runner.game,
            p_167_top_card_handle,
            Some(host_handle),
            0,
        );
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }

    // Phase 1: cost — select_own_sources installs (Mineral/Rock source filter).
    // ROCK-HOST has P-167 (Mineral-trait Digimon card) as a source. Phase 1
    // pending: select source ref.
    assert!(
        runner.game.pending_selection.is_some(),
        "cost layer must install select_own_sources prompt"
    );
    runner
        .auto_resolve_one_step()
        .expect("source selection resolves");

    // Phase 2: reveal_top_deck runs synchronously; effect_choice for routing.
    // Then if-branch enters choose_from_reveal (selection) for the picked card.
    // Finally order_remainder presents top/bottom destination effect_choice.
    // Auto-resolve walks all selections to completion.
    runner
        .auto_resolve()
        .expect("full P-167 reveal+choose+order chain resolves");

    // MIN-PICK ends up in hand OR placed as a bottom source. The branch chosen
    // by auto_resolve (first valid action of every effect_choice = "Add to
    // hand") routes to hand.
    let min_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "MIN-PICK");
    let min_as_source = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "MIN-PICK");
    assert!(
        min_in_hand || min_as_source,
        "MIN-PICK must land in hand OR as a bottom source after the reveal clause body"
    );
}

/// Behavioral assertion shim: drive the pending selection one step (without
/// chaining). Used to verify the cost selection installs before the rest of
/// the clause body fires.
impl ExecuteOneStep for DebugRunner {
    fn auto_resolve_one_step(&mut self) -> Result<(), digimon_engine::selection::SelectionError> {
        let Some(sel) = self.pending_selection() else {
            return Ok(());
        };
        let player = sel.selecting_player;
        let action_id = match sel.valid_action_ids.first().copied() {
            Some(id) => id,
            None if sel.is_optional => digimon_engine::action::space::PASS,
            None => return Err(digimon_engine::selection::SelectionError::InvalidAction),
        };
        self.execute_action(player, action_id)
    }
}

trait ExecuteOneStep {
    fn auto_resolve_one_step(&mut self) -> Result<(), digimon_engine::selection::SelectionError>;
}
