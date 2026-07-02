//! `EffectContext::trash_card_source` — unit tests.
//!
//! The primitive removes a specific `CardSource` from anywhere in a
//! permanent's `card_sources` stack (not just the top) and pushes it to the
//! controller's trash.
//!
//! Used by:
//! - `<Fragment (N)>` keyword auto-install (Phase D Task 4)
//! - `<Partition>` keyword auto-install (Phase D Task 9)

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Helper: push a card onto an existing permanent's stack (makes it the new top).
fn push_source_on_top(
    r: &mut DebugRunner,
    perm_player: u8,
    perm_idx: usize,
    card_id: &str,
) -> digimon_engine::card_source::CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_on_top: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, perm_player, card_index);
    let handle = card.handle();
    r.game.players[perm_player as usize].battle_area[perm_idx]
        .card_sources
        .push(card);
    handle
}

// ── Test 1: Trash a mid-stack source ─────────────────────────────────────────

/// Stack: [BASE, MID-SOURCE, TOP].
/// Call trash_card_source(perm, MID-SOURCE).
/// Expected: stack = [BASE, TOP], MID-SOURCE in trash.
#[test]
fn trash_card_source_removes_mid_stack_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID-SOURCE"))
        .add_card(make_digimon("TOP"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let perm_idx = perm_handle.index as usize;

    let mid_handle = push_source_on_top(&mut r, tp, perm_idx, "MID-SOURCE");
    push_source_on_top(&mut r, tp, perm_idx, "TOP");

    // Pre-condition: 3-card stack, no trash.
    {
        let perm = &r.game.players[tp as usize].battle_area[perm_idx];
        assert_eq!(perm.card_sources.len(), 3, "3-card stack before");
    }
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        0,
        "trash empty before"
    );

    // Apply trash_card_source targeting MID-SOURCE.
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        let trashed = ctx.trash_card_source(perm_handle, mid_handle);
        assert!(
            trashed,
            "happy path: valid handle in stack must return true"
        );
    }

    // Stack should now be [BASE, TOP] (MID-SOURCE removed from middle).
    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 2, "stack shrunk to 2");
    assert_eq!(
        perm.card_sources[0].card_id(&r.game.card_data),
        "BASE",
        "BASE remains at bottom"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TOP",
        "TOP remains as top"
    );

    // MID-SOURCE is in trash.
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "one card in trash");
    assert_eq!(
        trash[0].card_id(&r.game.card_data),
        "MID-SOURCE",
        "MID-SOURCE went to trash"
    );
}

// ── Test 2: Trash the bottom source ──────────────────────────────────────────

/// Stack: [BASE, TOP].
/// Call trash_card_source(perm, BASE).
/// Expected: stack = [TOP], BASE in trash. The permanent is now a single-card stack.
#[test]
fn trash_card_source_removes_bottom_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("TOP"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let perm_idx = perm_handle.index as usize;

    // Capture BASE handle before pushing TOP.
    let base_handle = r.game.players[tp as usize].battle_area[perm_idx].card_sources[0].handle();

    push_source_on_top(&mut r, tp, perm_idx, "TOP");

    // Pre-condition: 2-card stack.
    assert_eq!(
        r.game.players[tp as usize].battle_area[perm_idx]
            .card_sources
            .len(),
        2
    );

    // Trash BASE (bottom of stack).
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        let trashed = ctx.trash_card_source(perm_handle, base_handle);
        assert!(
            trashed,
            "happy path: valid bottom-card handle must return true"
        );
    }

    // Stack = [TOP] only.
    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 1, "single card remains");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TOP",
        "TOP is now the only (top) card"
    );

    // BASE in trash.
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "one card in trash");
    assert_eq!(
        trash[0].card_id(&r.game.card_data),
        "BASE",
        "BASE went to trash"
    );
}

// ── Test 3: Trashing the only remaining card removes the zombie carrier ──────

/// Regression for `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` —
/// `trash_card_source` removing the carrier's only card must soft-remove
/// the now-empty slot rather than leave a zombie permanent in battle_area.
///
/// Setup: single-card permanent (stack = [ONLY]).
/// Call: `trash_card_source(perm, ONLY)`.
/// Expected post: no permanent in any battle_area has `card_sources.is_empty()`;
/// ONLY is in trash; `OnDigivolutionCardTrashed` was fired BEFORE the slot
/// removal so observers attribute the event to the correct host.
#[test]
fn trash_card_source_emptying_carrier_removes_slot() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ONLY"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "ONLY", Some(0));
    let only_handle = r.game.players[tp as usize].battle_area[perm_handle.index as usize]
        .card_sources[0]
        .handle();
    assert_eq!(
        r.game.players[tp as usize].battle_area[perm_handle.index as usize]
            .card_sources
            .len(),
        1,
        "precondition: carrier has exactly 1 source"
    );

    {
        let dummy_handle = only_handle;
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        let trashed = ctx.trash_card_source(perm_handle, only_handle);
        assert!(
            trashed,
            "happy path: trashing the only source must return true (carrier then soft-removed)"
        );
    }

    // No zombies anywhere.
    for (pid, player) in r.game.players.iter().enumerate() {
        for (slot, perm) in player.battle_area.iter().enumerate() {
            assert!(
                !perm.card_sources.is_empty(),
                "zombie permanent at p{}.battle_area[{}]: card_sources is empty",
                pid,
                slot
            );
        }
    }

    // Carrier slot removed; ONLY is in trash.
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        0,
        "carrier slot must be soft-removed"
    );
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "ONLY routed to trash");
    assert_eq!(
        trash[0].card_id(&r.game.card_data),
        "ONLY",
        "ONLY went to trash"
    );
}

// ── Test 4: Stale CardHandle silently no-ops ─────────────────────────────────

/// Regression for `G-DSL-TRASH-SOURCES-STALE-HANDLE` (sibling of
/// `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION`). When a captured source
/// handle is no longer in the carrier's `card_sources` at trash time
/// (intervening observer trashed/moved it), `trash_card_source` must return
/// `false` without panicking and without mutating any state. Mirrors DCGO
/// `ITrashDigivolutionCards.TrashDigivolutionCards`'s
/// `_trashTargetCards.Filter(c => _permanent.DigivolutionCards.Contains(c))`
/// at `DCGO/Assets/Scripts/Script/CardController.cs:5191`.
#[test]
fn trash_card_source_returns_false_on_stale_card_handle() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("STALE-SRC"))
        .add_card(make_digimon("TOP"))
        .start();
    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let perm_idx = perm_handle.index as usize;
    let stale_handle = push_source_on_top(&mut r, tp, perm_idx, "STALE-SRC");
    push_source_on_top(&mut r, tp, perm_idx, "TOP");

    // Simulate an intervening observer effect trashing STALE-SRC directly.
    // (In production this happens via a sibling clause's `trash_selected_sources`
    // running between SourceMulti install and the picker callback's tail.)
    r.game.players[tp as usize].battle_area[perm_idx]
        .card_sources
        .retain(|c| c.handle() != stale_handle);

    let stack_len_before = r.game.players[tp as usize].battle_area[perm_idx]
        .card_sources
        .len();
    let trash_len_before = r.game.players[tp as usize].trash.len();
    assert_eq!(stack_len_before, 2, "STALE-SRC removed, BASE + TOP remain");

    // Call trash_card_source with the now-stale handle.
    let trashed = {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.trash_card_source(perm_handle, stale_handle)
    };

    assert!(!trashed, "stale handle must return false");
    let stack_len_after = r.game.players[tp as usize].battle_area[perm_idx]
        .card_sources
        .len();
    let trash_len_after = r.game.players[tp as usize].trash.len();
    assert_eq!(
        stack_len_before, stack_len_after,
        "no-mutation invariant: card_sources unchanged"
    );
    assert_eq!(
        trash_len_before, trash_len_after,
        "no-mutation invariant: trash unchanged"
    );
}

// ── Test 5: Missing carrier slot silently no-ops ─────────────────────────────

/// When the carrier `PermanentHandle.index` is out of bounds (the permanent
/// was removed by a sibling effect — e.g., soft-removed empty, returned to
/// hand, deleted), `trash_card_source` must return `false` rather than
/// panicking via `.expect("permanent not found")`. DCGO parity:
/// `if (_permanent == null) yield break;` at `CardController.cs:5185`.
#[test]
fn trash_card_source_returns_false_on_missing_carrier() {
    use digimon_engine::permanent::PermanentHandle;

    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("SRC"))
        .start();
    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let captured_card = push_source_on_top(&mut r, tp, perm_handle.index as usize, "SRC");

    // Simulate removal of the entire carrier slot (an intervening delete or
    // batched return). Direct mutation matches what `Game::soft_remove_if_emptied`
    // + the batched-delete path would do at the battle_area level.
    r.game.players[tp as usize].battle_area.clear();

    // Stale handle points to index 0, which is now out of bounds.
    let stale_perm = PermanentHandle {
        player: perm_handle.player,
        index: 0,
    };
    assert!(
        r.game.players[tp as usize]
            .battle_area
            .get(stale_perm.index as usize)
            .is_none(),
        "precondition: carrier slot is gone"
    );

    let trash_len_before = r.game.players[tp as usize].trash.len();
    let trashed = {
        // dummy handle for EffectContext::new; not used by trash_card_source
        // beyond constructing the ctx.
        let mut ctx = EffectContext::new(&mut r.game, captured_card, None, tp);
        ctx.trash_card_source(stale_perm, captured_card)
    };
    assert!(!trashed, "missing carrier must return false");
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        trash_len_before,
        "no-mutation invariant: trash unchanged when carrier missing"
    );
}

// ── Test 6: Empty-stack zombie carrier silently no-ops ───────────────────────

/// When the carrier exists but `card_sources` is empty (a zombie slot that
/// hasn't been soft-removed yet — e.g., during the middle of a multi-source
/// trash where the previous iteration emptied the stack but the host hasn't
/// been cleaned up), `trash_card_source` must NOT call `top_card()` (which
/// would panic on empty stack) and must return `false`. DCGO parity:
/// `if (_permanent.HasNoDigivolutionCards) yield break;` at
/// `CardController.cs:5189`.
#[test]
fn trash_card_source_returns_false_on_empty_stack() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .start();
    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let perm_idx = perm_handle.index as usize;
    // Capture a handle BEFORE emptying — its identity will not match anything
    // in the empty stack, but the test point is that we never even get to
    // the lookup because the empty-stack guard fires first.
    let captured = r.game.players[tp as usize].battle_area[perm_idx].card_sources[0].handle();

    // Empty the stack directly (zombie slot).
    r.game.players[tp as usize].battle_area[perm_idx]
        .card_sources
        .clear();
    assert!(
        r.game.players[tp as usize].battle_area[perm_idx]
            .card_sources
            .is_empty(),
        "precondition: stack is empty"
    );

    let trash_len_before = r.game.players[tp as usize].trash.len();
    let trashed = {
        let mut ctx = EffectContext::new(&mut r.game, captured, Some(perm_handle), tp);
        // Must not panic on `top_card()` even though stack is empty.
        ctx.trash_card_source(perm_handle, captured)
    };
    assert!(
        !trashed,
        "empty stack must return false (no top_card panic)"
    );
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        trash_len_before,
        "no-mutation invariant: trash unchanged when stack empty"
    );
}
