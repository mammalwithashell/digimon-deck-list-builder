//! Phase D Task 4 — `Keyword::Fragment(N)` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Fragment(N)]` (no hand-rolled
//! `CardEffect`) must, on `WhenWouldBeDeleted`:
//!   1. If `card_sources.len() >= N + 1`: park a multi-pick selection over
//!      the carrier's digivolution sources; the controller picks N to trash;
//!      the original deletion is cancelled (carrier survives with the
//!      remaining sources).
//!   2. If `card_sources.len() < N + 1`: gate fails — no selection is parked,
//!      original deletion proceeds normally.
//!   3. The auto-install must self-scope: when a NEIGHBORING permanent is
//!      deleted, the Fragment carrier's replacement must NOT fire.
//!
//! Mirrors DCGO `Fragment.cs:23-77`. Per Phase C substrate constraints
//! (mandatory replacement processes can NOT install a `pending_selection`),
//! the auto-install uses `Effect::optional()` with an outer accept dialog —
//! see Task 4 deviation note in the parent plan.

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

const FRAGMENT_N: u8 = 2;

fn fragment_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        // Printed-only Fragment(N): the auto-install MUST be the sole
        // source of behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::Fragment(FRAGMENT_N)],
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Append `card_id` on top of a field permanent's digivolution stack so that
/// it becomes the new visible top (since `top_card() = card_sources.last()`).
fn push_source_card(r: &mut DebugRunner, player: u8, field_index: usize, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_card: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    r.game.players[player as usize].battle_area[field_index]
        .card_sources
        .push(card);
}

// ─── Test 1: happy path — picks N, cancels deletion, carrier survives ───────

#[test]
fn fragment_2_picks_two_sources_and_cancels_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(fragment_card("FRAG"))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .add_card(plain_digimon("SRC-C"))
        .start();

    // Build the carrier on the field as the top, with 3 sources beneath.
    // Stack: [SRC-A, SRC-B, SRC-C, FRAG]  (index 0=bottom, last=top)
    let frag = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, frag.index as usize, "SRC-B");
    push_source_card(&mut r, 0, frag.index as usize, "SRC-C");
    push_source_card(&mut r, 0, frag.index as usize, "FRAG");
    {
        let perm = &r.game.players[0].battle_area[frag.index as usize];
        assert_eq!(perm.card_sources.len(), 4, "preconditions: 4-card stack");
        assert_eq!(
            perm.top_card().card_id(&r.game.card_data),
            "FRAG",
            "FRAG is the visible top"
        );
    }

    let trash_before = r.game.players[0].trash.len();
    r.game.delete_permanent_with_effects(frag);

    // The Fragment auto-install runs as an optional replacement (Phase C
    // substrate constraint). The outer accept dialog should be parked.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Fragment must park an outer accept selection");
    assert_eq!(pending.selecting_player, 0, "carrier owner selects");
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Multi-pick chain: pick 2 sources. Each pick uses the first available
    // valid action id.
    for _ in 0..FRAGMENT_N {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("inner multi-pick step must be installed");
        let action = pending.valid_action_ids[0];
        let player = pending.selecting_player;
        r.game.resolve_selection(player, action).expect("pick");
    }

    // Carrier survived (deletion cancelled).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "FRAG must survive when Fragment cancels deletion"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(
        surviving.top_card().card_id(&r.game.card_data),
        "FRAG",
        "carrier still has FRAG as the top card"
    );
    // Two sources were trashed; 2 cards remain in the stack: 1 source + top.
    assert_eq!(
        surviving.card_sources.len(),
        4 - FRAGMENT_N as usize,
        "2 sources trashed → 2-card stack remaining (1 source + top)"
    );
    // Trash gained N cards.
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + FRAGMENT_N as usize,
        "trash gained exactly N cards"
    );
}

// ─── Test 2: gate fail — fewer than N sources → no park → normal deletion ───

#[test]
fn fragment_2_with_only_one_source_does_not_park() {
    // Stack: [ONLY-SRC, FRAG] — only 1 source under the top, but N=2 required.
    let mut r = DebugRunner::builder()
        .add_card(fragment_card("FRAG"))
        .add_card(plain_digimon("ONLY-SRC"))
        .start();

    let frag = r.place_on_field(0, "ONLY-SRC", None);
    push_source_card(&mut r, 0, frag.index as usize, "FRAG");
    {
        let perm = &r.game.players[0].battle_area[frag.index as usize];
        assert_eq!(perm.card_sources.len(), 2, "preconditions: 1-source stack");
    }

    r.game.delete_permanent_with_effects(frag);

    // Gate fails: card_sources.len() (= 2) < N + 1 (= 3). The auto-install
    // body must early-return without parking, and the original deletion must
    // proceed normally.
    assert!(
        r.game.pending_selection.is_none(),
        "no selection should be installed when source count is below N+1 — \
         got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "FRAG must be deleted normally when Fragment gate fails"
    );
}

// ─── Test 3: self-scope — closure no-ops on neighbor's deletion ─────────────

/// When a NEIGHBORING permanent is deleted, the Fragment carrier's auto-install
/// must NOT mutate any state — the carrier and its sources stay intact and
/// the neighbor is deleted normally.
///
/// **Substrate note**: Phase C's dispatcher pushes a candidate from every
/// battle-area permanent's effects whose timing matches, so the Fragment
/// carrier's optional outer-accept dialog MAY be installed when a neighbor is
/// being deleted. The auto-install body's `if Some(subject) != me_perm` guard
/// then no-ops on accept (no outcome set → original deletion proceeds). We
/// resolve any installed dialog to drain the substrate and verify the
/// behavioral outcome: neighbor gone, Fragment carrier and sources unchanged.
/// (A future dispatcher-level self-scope filter would suppress the dialog
/// entirely; tracked separately.)
#[test]
fn fragment_does_not_fire_on_neighbor_deletion() {
    use digimon_engine::action::space::PASS;

    let mut r = DebugRunner::builder()
        .add_card(fragment_card("FRAG"))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .add_card(plain_digimon("SRC-C"))
        .add_card(plain_digimon("NEIGHBOR"))
        .start();

    // FRAG carrier with 3 sources beneath.
    let frag = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, frag.index as usize, "SRC-B");
    push_source_card(&mut r, 0, frag.index as usize, "SRC-C");
    push_source_card(&mut r, 0, frag.index as usize, "FRAG");

    // Plain neighbor (no Fragment, no other keywords).
    let neighbor = r.place_on_field(0, "NEIGHBOR", None);

    let battle_area_before = r.game.players[0].battle_area.len();
    assert_eq!(battle_area_before, 2);

    // Snapshot Fragment carrier's stack composition for later comparison.
    let frag_stack_before: Vec<String> = r.game.players[0].battle_area[frag.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    let trash_size_before = r.game.players[0].trash.len();

    // Delete the neighbor.
    r.game.delete_permanent_with_effects(neighbor);

    // Fragment is OPTIONAL → an outer accept dialog MAY be installed for
    // the carrier even when the subject is the neighbor. If so, drain it by
    // either accepting (closure self-scope-checks and no-ops) or declining.
    // We choose ACCEPT to exercise the closure — its body must self-scope
    // and refrain from setting an outcome.
    if r.game.pending_selection.is_some() {
        // Try accepting first. If the closure's self-scope guard is correct,
        // accept → closure no-op → original neighbor deletion proceeds.
        let pending = r.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = if pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT) {
            REPLACEMENT_ACCEPT
        } else {
            PASS
        };
        r.game.resolve_selection(player, action).expect("drain dialog");
    }

    // After draining: no further selection should be pending. The body's
    // self-scope guard MUST have prevented any nested source-pick selection.
    assert!(
        r.game.pending_selection.is_none(),
        "Fragment closure must self-scope-no-op for a neighbor's deletion (no \
         nested source-pick selection); got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );

    // Behavioral outcome: neighbor was deleted normally; FRAG is intact.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "neighbor was deleted → only FRAG remains"
    );
    let frag_perm = &r.game.players[0].battle_area[0];
    assert_eq!(
        frag_perm.top_card().card_id(&r.game.card_data),
        "FRAG",
        "FRAG carrier untouched by neighbor's deletion"
    );
    let frag_stack_after: Vec<String> = frag_perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        frag_stack_after, frag_stack_before,
        "FRAG's stack must be unchanged"
    );
    // No FRAG-stack cards moved to trash (only the neighbor itself).
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_size_before + 1,
        "exactly 1 card (the neighbor) was added to trash"
    );
    assert_eq!(
        r.game.players[0].trash.last().unwrap().card_id(&r.game.card_data),
        "NEIGHBOR",
        "the trashed card is the neighbor"
    );
}

