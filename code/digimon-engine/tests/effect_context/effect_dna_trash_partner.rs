//! G-ENGINE-DNA-TRASH-MATERIAL (gap 3) — `EffectContext::
//! effect_initiated_dna_digivolve_trash_partner` performs a DNA digivolve
//! where ONE material is a battle-area permanent (`field_partner`), the OTHER
//! material lives in the TRASH (`trash_partner`), and the result card is in
//! hand (`from_hand`).
//!
//! DCGO reference (BT18_015.cs / BT18_073.cs [On Deletion]):
//!   1. `CardObjectController.CreateNewPermanent(...)` — materialize the trash
//!      material as a permanent. This is a pure placement — it fires NO
//!      `[On Play]` / `OnEnterField` triggers.
//!   2. `PlayCardClass(dnaTarget, payCost: true, root: Hand, activateETB: true)`
//!      + `SetJogress(...)` — merge the field partner and the just-materialized
//!      trash material into the result, paying the result's printed DNA cost,
//!      firing `WhenDigivolving` on the merged permanent.
//!
//! The shape is proven here with inline fixtures (BT18-015 / BT18-073 are
//! BLOCKED cards, authored elsewhere; this test proves the engine primitive
//! their DSL will call).

#![allow(dead_code, unused_imports)]

use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

fn material(id: &str, level: u8, color: CardColor) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(level);
    d.dp = Some(4000);
    d.colors = vec![color];
    d
}

fn req(level: u8, color: CardColor) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

fn dna_result(
    id: &str,
    name: &str,
    req1: DnaRequirement,
    req2: DnaRequirement,
    memory_cost: i16,
) -> CardData {
    let mut d = make_test_card(id, name);
    d.card_kind = CardKind::Digimon;
    d.level = Some(7);
    d.dp = Some(14000);
    d.dna_costs = vec![DnaCost {
        memory_cost,
        requirement1: req1,
        requirement2: req2,
    }];
    d
}

/// Push a card into `player`'s trash by id and return its handle.
fn push_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_trash: unknown card id {card_id}"));
    let idx = runner.game.next_card_index();
    let src = CardSource::new(data_idx, player, idx);
    let handle = src.handle();
    runner.game.player_mut(player).trash.push(src);
    handle
}

/// An OnPlay effect that bumps a per-game counter — used to prove the trash
/// material's OnPlay does NOT fire during materialization.
struct OnPlayMemBump;
impl CardEffect for OnPlayMemBump {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("+5 on play (should NOT fire on DNA materialize)")
            .process(|ctx| {
                ctx.gain_memory(5);
            })
            .build()]
    }
}

/// Core success path: field partner + trash partner (recipe-satisfying) DNA
/// digivolve into the hand result, merging into ONE permanent topped by the
/// result. Materials preserved as sources; result and trash card leave their
/// zones.
#[test]
fn dna_trash_partner_merges_field_and_trash_into_hand_result() {
    let mut runner = DebugRunner::builder()
        .add_card(material("FIELD-RED", 4, CardColor::Red))
        .add_card(material("TRASH-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let field = runner.place_on_field(0, "FIELD-RED", None);
    let trash_card = push_trash(&mut runner, 0, "TRASH-PUR");
    let result = runner.game.players[0].hand[0].handle();

    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].trash.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), 1);

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, result, None, 0);
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, 0, false)
    };
    assert!(
        merged.is_some(),
        "field + trash (recipe-satisfying) DNA digivolve into hand result must succeed"
    );

    // ONE merged permanent, topped by the result.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "still a single permanent (field partner became the merged stack)"
    );
    let merged_perm = &runner.game.players[0].battle_area[0];
    assert_eq!(
        merged_perm.top_card().card_id(&runner.game.card_data),
        "RES",
        "merged top card is the result"
    );
    // Stack: field material + trash material + result = 3 sources.
    assert_eq!(
        merged_perm.card_sources.len(),
        3,
        "merged stack = field material + trash material + result"
    );
    let ids: Vec<_> = merged_perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        ids,
        vec!["FIELD-RED", "TRASH-PUR", "RES"],
        "stacking order: field ++ trash ++ result"
    );

    // Trash material left the trash; result left the hand.
    assert_eq!(runner.game.players[0].trash.len(), 0, "trash material consumed");
    assert_eq!(runner.game.players[0].hand.len(), 0, "result consumed from hand");
}

/// The materialize step must NOT fire the trash material's own `[On Play]`.
/// DCGO's `CreateNewPermanent` is a pure placement. We register an OnPlay on
/// the trash material that would add +5 memory; after the DNA digivolve the
/// only memory movement is the printed DNA cost (here 0), so +5 must NOT show.
#[test]
fn dna_trash_partner_materialize_does_not_fire_trash_material_on_play() {
    let mut runner = DebugRunner::builder()
        .add_card(material("FIELD-RED", 4, CardColor::Red))
        .add_card(material("TRASH-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();
    runner.register_effect("TRASH-PUR", Arc::new(OnPlayMemBump));

    let field = runner.place_on_field(0, "FIELD-RED", None);
    let trash_card = push_trash(&mut runner, 0, "TRASH-PUR");
    let result = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, result, None, 0);
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, 0, false)
    };
    assert!(merged.is_some(), "DNA digivolve succeeds");

    assert_eq!(
        runner.game.memory, before,
        "materializing the trash material must NOT fire its [On Play] (+5); \
         memory changed from {before} to {} — DCGO CreateNewPermanent fires nothing",
        runner.game.memory
    );
}

/// Printed DNA cost (gap 1) composes with the trash-partner flow: a
/// recipe-satisfying pair pays the result's printed DNA cost.
#[test]
fn dna_trash_partner_pays_printed_dna_cost() {
    let mut runner = DebugRunner::builder()
        .add_card(material("FIELD-RED", 4, CardColor::Red))
        .add_card(material("TRASH-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            4, // printed DNA cost = 4
        ))
        .hand(0, &["RES"])
        .memory(6)
        .start();

    let field = runner.place_on_field(0, "FIELD-RED", None);
    let trash_card = push_trash(&mut runner, 0, "TRASH-PUR");
    let result = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, result, None, 0);
        let cost = ctx
            .printed_dna_cost_for_field_trash_pair(field, trash_card, result)
            .expect("recipe-satisfying field+trash pair resolves a printed DNA cost");
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, cost, false)
    };
    assert!(merged.is_some(), "recipe-satisfying pair merges");
    assert_eq!(
        before - runner.game.memory,
        4,
        "memory must drop by the result's printed DNA cost (4)"
    );
}

/// Recipe enforcement (gap 2) composes: an illegal field+trash pair (Red + Red
/// against a Red/Purple recipe) is REJECTED at commit with no state change.
#[test]
fn dna_trash_partner_rejects_illegal_recipe() {
    let mut runner = DebugRunner::builder()
        .add_card(material("FIELD-RED", 4, CardColor::Red))
        .add_card(material("TRASH-RED", 4, CardColor::Red))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let field = runner.place_on_field(0, "FIELD-RED", None);
    let trash_card = push_trash(&mut runner, 0, "TRASH-RED");
    let result = runner.game.players[0].hand[0].handle();

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, result, None, 0);
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, 0, false)
    };
    assert!(
        merged.is_none(),
        "illegal field+trash pair (Red + Red, recipe wants Red + Purple) must be REJECTED"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "field partner untouched on rejection"
    );
    assert_eq!(runner.game.players[0].trash.len(), 1, "trash material untouched");
    assert_eq!(runner.game.players[0].hand.len(), 1, "result untouched in hand");
}

/// The result's own `WhenDigivolving` fires on the merged permanent (the
/// merge IS a digivolve). Contrast with the trash material's suppressed OnPlay.
#[test]
fn dna_trash_partner_fires_result_when_digivolving() {
    struct WhenDigivolvingBump;
    impl CardEffect for WhenDigivolvingBump {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_digivolving(card)
                .name("+1 when digivolving")
                .process(|ctx| {
                    ctx.gain_memory(1);
                })
                .build()]
        }
    }

    let mut runner = DebugRunner::builder()
        .add_card(material("FIELD-RED", 4, CardColor::Red))
        .add_card(material("TRASH-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();
    runner.register_effect("RES", Arc::new(WhenDigivolvingBump));

    let field = runner.place_on_field(0, "FIELD-RED", None);
    let trash_card = push_trash(&mut runner, 0, "TRASH-PUR");
    let result = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, result, None, 0);
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, 0, false)
    };
    assert!(merged.is_some(), "DNA digivolve succeeds");
    assert_eq!(
        runner.game.memory - before,
        1,
        "result's WhenDigivolving must fire once on the merged permanent"
    );
}

/// Clone-safety: the trash-partner primitive is ATOMIC — it takes pre-resolved
/// handles and installs no mid-effect `pending_selection` closure. A `Game`
/// cloned BEFORE the call resolves to an identical merged board as the
/// original, and neither path panics through `PendingSelection::clone`
/// (make-engine-cloneable rule 28). The interactive selection chain that
/// FEEDS this primitive (`select_own_permanent` / `select_hand` /
/// `select_trash`) is the resume-driven, clone-safe part — covered by the
/// bt17_095 `_delay_dna_prompts_clone_faithfully` test for the sibling
/// hand-partner verb.
#[test]
fn dna_trash_partner_primitive_is_atomic_and_clone_safe() {
    let mut runner = DebugRunner::builder()
        .add_card(material("FIELD-RED", 4, CardColor::Red))
        .add_card(material("TRASH-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            2,
        ))
        .hand(0, &["RES"])
        .memory(6)
        .start();

    let field = runner.place_on_field(0, "FIELD-RED", None);
    let trash_card = push_trash(&mut runner, 0, "TRASH-PUR");
    let result = runner.game.players[0].hand[0].handle();

    // No selection is pending before the call — clone the pristine state.
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection pending before an atomic primitive call"
    );
    let mut clone_before = runner.game.clone();

    // Resolve on the clone first.
    let clone_merged = {
        let mut ctx = EffectContext::new(&mut clone_before, result, None, 0);
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, 2, false)
    };
    assert!(clone_merged.is_some(), "clone performs the DNA merge");
    // The primitive never installs a bespoke pending_selection.
    assert!(
        clone_before.pending_selection.is_none(),
        "atomic primitive must not install a pending_selection"
    );

    // Original is untouched by the clone's resolution.
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "original's trash material intact while the clone resolved"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "RES"),
        "original has not merged while the clone resolved"
    );

    // Resolving the original reaches the SAME merged board as the clone.
    let orig_merged = {
        let mut ctx = EffectContext::new(&mut runner.game, result, None, 0);
        ctx.effect_initiated_dna_digivolve_trash_partner(field, trash_card, result, 2, false)
    };
    assert!(orig_merged.is_some(), "original reaches the same DNA merge");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        clone_before.players[0].battle_area.len(),
        "original and clone reach the same battle-area shape"
    );
    assert_eq!(
        runner.game.memory, clone_before.memory,
        "original and clone paid the same printed DNA cost"
    );
}
