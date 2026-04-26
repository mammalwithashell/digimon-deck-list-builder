//! Phase D Task 5 — `Keyword::ArmorPurge` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::ArmorPurge]` (no hand-rolled
//! `CardEffect`) must, on `WhenWouldBeDeleted`:
//!   1. If `card_sources.len() >= 2`: trash the current top Digimon, promote
//!      the next-highest source as the new visible top, and cancel the
//!      original deletion. **No player selection** — the action is forced.
//!   2. If `card_sources.len() < 2`: gate fails — no replacement fires, the
//!      original deletion proceeds normally.
//!   3. Self-scope: when a NEIGHBORING permanent is deleted, the ArmorPurge
//!      carrier's auto-install body MUST NOT mutate any state.
//!
//! Mirrors DCGO `ArmorPurge.cs:40-78`. Unlike `Fragment(N)` (which goes through
//! the parked-replacement substrate because it carries a nested selection),
//! ArmorPurge is purely synchronous: the auto-install body calls
//! `rctx.cancel()` directly inside the mandatory replacement process, with no
//! `.optional()` wrapper required.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

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

/// 2-source stack: [BOTTOM, ARMOR-TOP]. Opponent deletes ARMOR-TOP. The auto-
/// install body MUST:
///   - Trash ARMOR-TOP (it lands in controller's trash).
///   - Promote BOTTOM to be the new visible top.
///   - Cancel the original deletion (carrier survives).
///   - NOT install any pending_selection (no player choice involved).
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

    // ArmorPurge runs synchronously (mandatory + no nested selection) — no
    // pending_selection should be installed at any point.
    assert!(
        r.game.pending_selection.is_none(),
        "ArmorPurge must resolve synchronously — no pending selection should be \
         installed; got {:?}",
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

// ─── Test 2: gate fail — single-card stack → normal deletion ────────────────

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

// ─── Test 3: self-scope — auto-install no-ops on neighbor's deletion ────────

/// When a NEIGHBORING permanent is deleted, the ArmorPurge carrier MUST NOT
/// mutate any state — the carrier and its sources stay intact and the
/// neighbor is deleted normally.
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

    // No selection should be pending.
    assert!(
        r.game.pending_selection.is_none(),
        "ArmorPurge must not install any selection on a neighbor's deletion; \
         got {:?}",
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

// ─── Test 4: OnDigivolutionCardTrashed fires for the trashed top ────────────

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

    // Trigger the deletion → ArmorPurge fires → top is trashed → observer fires.
    r.game.delete_permanent_with_effects(armor);

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
