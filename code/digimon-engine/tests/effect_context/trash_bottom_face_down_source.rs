//! Task A4.1 — `EffectContext::trash_bottom_face_down_source`.
//!
//! The cost-form trash primitive for ST-23 BEATBREAK / ST-24 DATA SQUAD
//! cards whose printed text reads "by trashing the bottom face-down card
//! from under any of your Tamers, ...".
//!
//! Behavioral contract:
//!   - Inspects `card_sources[0]` of the target permanent. If it exists AND
//!     is `face_down`, removes it, routes it to its OWNER's trash, fires
//!     `OnDigivolutionCardTrashed`, and returns `true`.
//!   - Otherwise (no face-down bottom source, or `target` missing) returns
//!     `false` with NO mutation.

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::{Permanent, PermanentHandle};

fn make_tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
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
    }
}

fn make_digimon(id: &str) -> CardData {
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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

/// Seed a single-card permanent on player 0's battle area (top card only).
fn seed_permanent(r: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    let g = r.game_mut();
    let turn = g.turn_count;
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("seed_permanent: unknown card_id {}", card_id));
    let card_idx = g.next_card_index();
    let card = CardSource::new(data_idx, 0, card_idx);
    g.players[0].battle_area.push(Permanent::new(card, turn));
    PermanentHandle {
        player: 0,
        index: (g.players[0].battle_area.len() - 1) as u8,
    }
}

/// Stash a card as the BOTTOM digivolution source of `target`, with the
/// given `face_down` flag. Returns the stashed source handle.
fn stash_bottom_source(
    r: &mut DebugRunner,
    target: PermanentHandle,
    card_id: &str,
    face_down: bool,
) -> digimon_engine::card_source::CardHandle {
    let g = r.game_mut();
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("stash_bottom_source: unknown card_id {}", card_id));
    let card_idx = g.next_card_index();
    let mut src = CardSource::new(data_idx, 0, card_idx);
    src.face_down = face_down;
    let handle = src.handle();
    g.players[target.player as usize].battle_area[target.index as usize]
        .card_sources
        .insert(0, src);
    handle
}

/// 1. Trashing the bottom face-down source pops it, routes it to the
///    owner's trash, and removes it from the Tamer's stack.
#[test]
fn trash_bottom_face_down_source_pops_and_routes_to_owner_trash() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("STASH"))
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    let stash_handle = stash_bottom_source(&mut r, tamer, "STASH", true);

    let trash_before = r.game.player(0).trash.len();
    assert_eq!(
        r.game.player(0).battle_area[tamer.index as usize]
            .card_sources
            .len(),
        2,
        "precondition: tamer has its top card + the stashed source"
    );

    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(tamer)
    };

    assert!(
        result,
        "trash_bottom_face_down_source returns true when a face-down bottom source exists"
    );

    let trash = &r.game.player(0).trash;
    assert_eq!(
        trash.len(),
        trash_before + 1,
        "owner's trash grew by exactly 1"
    );
    assert_eq!(
        trash.last().unwrap().handle(),
        stash_handle,
        "the trashed card is the stashed face-down source"
    );

    let perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert!(
        perm.card_sources
            .iter()
            .all(|c| c.handle() != stash_handle),
        "the stashed source is no longer in the Tamer's stack"
    );
    assert_eq!(
        perm.card_sources.len(),
        1,
        "tamer stack shrank back to just its own top card"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "tamer's own card remains as the top"
    );
}

/// 2. A face-UP bottom source is NOT eligible — the helper returns false and
///    leaves the stack untouched.
#[test]
fn trash_bottom_face_down_source_no_face_down_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("STASH"))
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    let stash_handle = stash_bottom_source(&mut r, tamer, "STASH", false);

    let trash_before = r.game.player(0).trash.len();

    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(tamer)
    };

    assert!(
        !result,
        "trash_bottom_face_down_source returns false when the bottom source is face-up"
    );

    let perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        2,
        "face-up bottom source: no mutation, stack unchanged"
    );
    assert_eq!(
        perm.card_sources[0].handle(),
        stash_handle,
        "the face-up source is still at the bottom of the stack"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before,
        "face-up bottom source: nothing pushed to trash"
    );
}

/// 3. An un-stashed Tamer's `card_sources[0]` is its own (face-up) top card —
///    so "bottom face-down source" does not exist → false, no mutation.
#[test]
fn trash_bottom_face_down_source_empty_or_missing_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    // Un-stashed tamer: card_sources[0] is the Tamer's own card (face-up).
    assert_eq!(
        r.game.player(0).battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "precondition: un-stashed tamer has only its own card"
    );
    let trash_before = r.game.player(0).trash.len();

    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(tamer)
    };

    assert!(
        !result,
        "trash_bottom_face_down_source returns false on a tamer with no stashable face-down source"
    );

    let perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "no stashable source: stack unchanged — only the tamer's own card"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "tamer's own card remains"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before,
        "no stashable source: nothing pushed to trash"
    );

    // Also exercise the missing-target path: an out-of-bounds handle.
    let bogus = PermanentHandle {
        player: 0,
        index: 99,
    };
    let result_bogus = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(bogus)
    };
    assert!(
        !result_bogus,
        "trash_bottom_face_down_source returns false when the target permanent is missing"
    );
}

/// A `CardEffect` that gains +1 memory whenever a digivolution source is
/// trashed — used to detect `OnDigivolutionCardTrashed` dispatch.
struct DigCardTrashedObs;
impl CardEffect for DigCardTrashedObs {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolution_card_trashed(card)
            .name("+1 when source trashed")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// 4. Trashing the bottom face-down source fires `OnDigivolutionCardTrashed`,
///    observable by an on-field observer that gains memory on the event.
#[test]
fn trash_bottom_face_down_source_fires_on_digivolution_card_trashed() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("STASH"))
        .add_card(make_digimon("OBS"))
        .memory(5)
        .start();
    r.register_effect("OBS", Arc::new(DigCardTrashedObs));

    // Place the observer on player 0's field.
    let _obs = r.place_on_field(0, "OBS", Some(0));

    let tamer = seed_permanent(&mut r, "TAMER");
    let stash_handle = stash_bottom_source(&mut r, tamer, "STASH", true);

    let before = r.memory();
    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(tamer)
    };

    assert!(result, "trash_bottom_face_down_source returns true");
    assert!(
        r.memory() > before,
        "OnDigivolutionCardTrashed should fire when the face-down stash is trashed \
         (before={}, after={})",
        before,
        r.memory()
    );

    // The stashed source did land in the trash.
    let trash = &r.game.player(0).trash;
    assert_eq!(
        trash.last().unwrap().handle(),
        stash_handle,
        "the stashed source was trashed"
    );
}
