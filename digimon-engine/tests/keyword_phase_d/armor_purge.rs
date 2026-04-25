//! Phase D Task 5 — `Keyword::ArmorPurge` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::ArmorPurge]` (no hand-rolled
//! `CardEffect`) must, on `WhenWouldBeDeleted`:
//!   1. If `card_sources.len() >= 2`: park an outer optional accept dialog
//!      ("by trashing the top card"). On ACCEPT, synchronously trash the
//!      current top Digimon, promote the next-highest source as the new
//!      visible top, and cancel the original deletion.
//!   2. On DECLINE of the outer dialog: original deletion proceeds; the top
//!      is NOT independently trashed (the optional cost wasn't paid).
//!   3. If `card_sources.len() < 2`: gate fails — no replacement fires, the
//!      original deletion proceeds normally.
//!   4. Self-scope: when a NEIGHBORING permanent is deleted, the ArmorPurge
//!      carrier's auto-install body MUST NOT mutate any state.
//!   5. OnDigivolutionCardTrashed fires when the swap actually trashes the
//!      top.
//!
//! Mirrors DCGO `ArmorPurge.cs:40-78`. Optional ("by [cost]") per
//! RULES_CONTEXT 16-18 — the outer accept dialog represents the optional
//! "by trashing" cost. Once accepted, the synchronous body trashes the top
//! and cancels the deletion (no nested player selection — the cost target
//! is fixed to the visible top).

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

use super::helpers::{assert_outer_accept_dialog_shape, drain_outer_accept_dialog};

fn armor_purge_card(id: &str) -> CardData {
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
        // Printed-only Armor Purge: the auto-install MUST be the sole source
        // of behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::ArmorPurge],
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

// ─── Test 1: happy path — top swaps with source, deletion cancelled ─────────

/// 2-source stack: [BOTTOM, ARMOR-TOP]. Opponent deletes ARMOR-TOP. Outer
/// accept dialog installs ("by trashing the top card"); on ACCEPT the auto-
/// install body MUST:
///   - Trash ARMOR-TOP (it lands in controller's trash).
///   - Promote BOTTOM to be the new visible top.
///   - Cancel the original deletion (carrier survives).
#[test]
fn armor_purge_swaps_top_and_cancels_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(armor_purge_card("ARMOR-TOP"))
        .add_card(plain_digimon("BOTTOM"))
        .start();

    // Build stack [BOTTOM (base), ARMOR-TOP (top)] on player 0's field.
    let perm = r.place_on_field(0, "BOTTOM", None);
    push_source_card(&mut r, 0, perm.index as usize, "ARMOR-TOP");
    {
        let p = &r.game.players[0].battle_area[perm.index as usize];
        assert_eq!(p.card_sources.len(), 2, "preconditions: 2-card stack");
        assert_eq!(
            p.top_card().card_id(&r.game.card_data),
            "ARMOR-TOP",
            "ARMOR-TOP is the visible top"
        );
    }

    let trash_before = r.game.players[0].trash.len();

    // Opponent triggers deletion (any cause works; use the bare effects path).
    r.game.delete_permanent_with_effects(perm);

    // Outer optional accept dialog installs ("by trashing the top card").
    assert_outer_accept_dialog_shape(&r);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // After accept, the synchronous body resolves — no further selection
    // pending (no nested player choice).
    assert!(
        r.game.pending_selection.is_none(),
        "after accept, ArmorPurge resolves synchronously — no further selection; got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );

    // Carrier survived (deletion cancelled).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "permanent must survive when ArmorPurge cancels deletion"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(
        surviving.card_sources.len(),
        1,
        "stack shrunk by 1 (ARMOR-TOP trashed, BOTTOM remains)"
    );
    assert_eq!(
        surviving.top_card().card_id(&r.game.card_data),
        "BOTTOM",
        "BOTTOM is the new visible top after promotion"
    );

    // ARMOR-TOP went to trash.
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 1,
        "exactly 1 card was added to trash"
    );
    assert_eq!(
        r.game.players[0].trash.last().unwrap().card_id(&r.game.card_data),
        "ARMOR-TOP",
        "the trashed card is the previous top"
    );
}

// ─── Test 2: outer-accept decline → no swap, deletion proceeds ─────────────

/// 2-source stack: [BOTTOM, ARMOR-TOP]. Opponent deletes ARMOR-TOP. Outer
/// accept dialog installs ("by trashing the top card"); on DECLINE (PASS) the
/// "by trashing" cost isn't paid, the synchronous body never runs, the top
/// card is NOT trashed, and the original deletion completes — both stack
/// cards land in the trash exactly like a plain deletion. This proves the
/// optional cost semantics: declining skips the replacement entirely.
#[test]
fn armor_purge_declining_outer_accept_proceeds_with_plain_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(armor_purge_card("ARMOR-TOP"))
        .add_card(plain_digimon("BOTTOM"))
        .start();

    let perm = r.place_on_field(0, "BOTTOM", None);
    push_source_card(&mut r, 0, perm.index as usize, "ARMOR-TOP");
    {
        let p = &r.game.players[0].battle_area[perm.index as usize];
        assert_eq!(p.card_sources.len(), 2, "preconditions: 2-card stack");
    }

    let trash_before = r.game.players[0].trash.len();

    r.game.delete_permanent_with_effects(perm);

    // Outer optional accept dialog installs.
    assert_outer_accept_dialog_shape(&r);

    // DECLINE the dialog (PASS is admitted implicitly via is_optional).
    r.game.resolve_selection(0, PASS).expect("decline");

    // No follow-up selection should be installed.
    assert!(
        r.game.pending_selection.is_none(),
        "declining the outer accept dialog must NOT install a follow-up \
         selection; got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );

    // Original deletion proceeded: the carrier is gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ArmorPurge carrier must be deleted normally when outer accept is \
         declined"
    );

    // Plain deletion sends the entire 2-card stack to the trash. The top
    // (ARMOR-TOP) is NOT trashed independently via the synchronous body —
    // both cards land in trash via the deletion path itself.
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 2,
        "both stack cards must land in trash on plain deletion (no \
         independent top-card trash from declined ArmorPurge); got {} new \
         trash entries",
        r.game.players[0].trash.len() - trash_before,
    );
    let trashed_ids: std::collections::HashSet<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        trashed_ids.contains("ARMOR-TOP"),
        "ARMOR-TOP must be in trash; got {:?}",
        trashed_ids,
    );
    assert!(
        trashed_ids.contains("BOTTOM"),
        "BOTTOM must be in trash; got {:?}",
        trashed_ids,
    );
}

// ─── Test 3: gate fail — single-card stack → normal deletion ────────────────

/// Stack: [ARMOR-TOP] (no source under). Opponent deletes ARMOR-TOP. Gate
/// `card_sources.len() >= 2` fails → no replacement fires → original deletion
/// proceeds normally.
#[test]
fn armor_purge_with_no_source_does_not_protect() {
    let mut r = DebugRunner::builder()
        .add_card(armor_purge_card("ARMOR-TOP"))
        .start();

    let perm = r.place_on_field(0, "ARMOR-TOP", None);
    {
        let p = &r.game.players[0].battle_area[perm.index as usize];
        assert_eq!(p.card_sources.len(), 1, "preconditions: 1-card stack");
    }

    r.game.delete_permanent_with_effects(perm);

    // Gate fails: no selection should be installed and the permanent is gone.
    assert!(
        r.game.pending_selection.is_none(),
        "no selection should be installed when gate fails — got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ARMOR-TOP must be deleted normally when ArmorPurge gate fails"
    );
    // The card itself should be in the trash (deleted normally).
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ARMOR-TOP"),
        "ARMOR-TOP went to trash via normal deletion"
    );
}

// ─── Test 4: self-scope — auto-install no-ops on neighbor's deletion ────────

/// When a NEIGHBORING permanent is deleted, the ArmorPurge carrier MUST NOT
/// mutate any state — the carrier and its sources stay intact and the
/// neighbor is deleted normally. ArmorPurge is now optional, so the
/// candidate-walk MAY install an outer accept dialog for the carrier when a
/// neighbor's deletion fires the timing; the body's self-scope guard then
/// no-ops on accept (no outcome set → original neighbor deletion proceeds).
#[test]
fn armor_purge_does_not_fire_on_neighbor_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(armor_purge_card("ARMOR-TOP"))
        .add_card(plain_digimon("BOTTOM"))
        .add_card(plain_digimon("NEIGHBOR"))
        .start();

    // ArmorPurge carrier with 1 source beneath.
    let armor = r.place_on_field(0, "BOTTOM", None);
    push_source_card(&mut r, 0, armor.index as usize, "ARMOR-TOP");

    // Plain neighbor (no Armor Purge, no other keywords).
    let neighbor = r.place_on_field(0, "NEIGHBOR", None);

    let armor_idx = armor.index as usize;
    assert_eq!(r.game.players[0].battle_area.len(), 2);

    // Snapshot ArmorPurge carrier's stack composition for later comparison.
    let armor_stack_before: Vec<String> = r.game.players[0].battle_area[armor_idx]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();

    // Delete the neighbor.
    r.game.delete_permanent_with_effects(neighbor);

    // ArmorPurge is optional → an outer accept dialog MAY install for the
    // carrier even when the subject is the neighbor. If so, drain it by
    // accepting: the body's self-scope guard no-ops and the original neighbor
    // deletion proceeds. (`drain_outer_accept_dialog` asserts the dialog's
    // valid_action_ids are exactly {REPLACEMENT_ACCEPT, PASS}.)
    drain_outer_accept_dialog(&mut r, REPLACEMENT_ACCEPT);

    // After draining: no further selection should be pending. The body's
    // self-scope guard MUST have prevented any state mutation.
    assert!(
        r.game.pending_selection.is_none(),
        "ArmorPurge body must self-scope-no-op on accept for a neighbor's \
         deletion; got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );

    // Behavioral outcome: neighbor was deleted normally; ArmorPurge carrier intact.
    // (After NEIGHBOR is removed, the ArmorPurge perm's index may shift — find
    // it by name.)
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "neighbor was deleted → only the ArmorPurge carrier remains"
    );
    let armor_perm = &r.game.players[0].battle_area[0];
    assert_eq!(
        armor_perm.top_card().card_id(&r.game.card_data),
        "ARMOR-TOP",
        "ArmorPurge carrier untouched by neighbor's deletion"
    );
    let armor_stack_after: Vec<String> = armor_perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        armor_stack_after, armor_stack_before,
        "ArmorPurge carrier's stack must be unchanged"
    );
    // The trashed card is the neighbor.
    assert_eq!(
        r.game.players[0].trash.last().unwrap().card_id(&r.game.card_data),
        "NEIGHBOR",
        "the trashed card is the neighbor"
    );
}

// ─── Test 5: OnDigivolutionCardTrashed fires for the trashed top ────────────

/// When ArmorPurge cancels deletion by trashing the top card, the
/// `OnDigivolutionCardTrashed` timing MUST fire so that any observers
/// (e.g. Rocks archetype permanents listening for source-trash events) see
/// the event. This mirrors the DCGO `WhenTopCardTrashed` dispatch in
/// `ArmorPurge.cs:65-78`.
///
/// Verified by registering a hand-rolled observer card on the field that
/// counts `OnDigivolutionCardTrashed` fires by adding a fixed-amount DP
/// modifier to itself each time it fires, then asserting the counter
/// advanced by 1 after `armor_purge_top` ran.
struct ObserverEffect;

impl digimon_engine::effect::CardEffect for ObserverEffect {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        use digimon_engine::effect::Effect;
        use digimon_engine::enums::{Expiry, ModifierType};
        use digimon_engine::modifiers::ModifierEntry;
        vec![Effect::on_digivolution_card_trashed(card)
            .name("count source-trash fires")
            .process(|ctx| {
                if let Some(perm) = ctx.source_permanent {
                    ctx.game.modifiers.add(
                        perm,
                        ModifierEntry::simple(
                            ModifierType::ChangeDp,
                            100,
                            Expiry::Permanent,
                            ctx.player,
                        ),
                    );
                }
            })
            .build()]
    }
}

#[test]
fn armor_purge_fires_on_digivolution_card_trashed() {
    use digimon_engine::cards::CardEffectRegistry;
    use digimon_engine::enums::ModifierType;
    use std::sync::Arc;

    // OBSERVER: gains +100 ChangeDp every time OnDigivolutionCardTrashed fires
    // anywhere on its controller's battle area.
    let mut registry = CardEffectRegistry::default();
    registry.insert("OBSERVER", Arc::new(ObserverEffect));

    let mut r = DebugRunner::builder()
        .add_card(armor_purge_card("ARMOR-TOP"))
        .add_card(plain_digimon("BOTTOM"))
        .add_card(plain_digimon("OBSERVER"))
        .with_registry(registry)
        .start();

    // ARMOR-TOP carrier with BOTTOM beneath.
    let armor = r.place_on_field(0, "BOTTOM", None);
    push_source_card(&mut r, 0, armor.index as usize, "ARMOR-TOP");

    // OBSERVER on the same side.
    let observer = r.place_on_field(0, "OBSERVER", None);

    // No source-trash events fired yet (nothing observed).
    let dp_before = r.game.modifiers.sum(observer, ModifierType::ChangeDp);
    assert_eq!(dp_before, 0, "no OnDigivolutionCardTrashed fires before");

    // Trigger the deletion → outer accept dialog → on accept, top is trashed
    // → observer fires.
    r.game.delete_permanent_with_effects(armor);
    assert!(
        r.game.pending_selection.is_some(),
        "ArmorPurge must install an outer accept dialog"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Observer should have fired exactly once.
    let dp_after = r.game.modifiers.sum(observer, ModifierType::ChangeDp);
    assert_eq!(
        dp_after,
        100,
        "OnDigivolutionCardTrashed must fire exactly once when ArmorPurge \
         trashes the top card; observer DP delta = {}",
        dp_after - dp_before
    );

    // Sanity: the carrier survived with the promoted top.
    let surviving = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BOTTOM")
        .expect("BOTTOM is now the top of the surviving carrier");
    assert_eq!(surviving.card_sources.len(), 1);

}
