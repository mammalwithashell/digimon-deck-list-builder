//! Shared first-match-wins selection of a `[Main]`-activated effect.
//!
//! Both `build_action_mask` (which decides whether to *emit* a `[Main]`
//! activation bit) and `explain_action` (which *labels* that action with the
//! matched effect's name) must agree on exactly **which** effect a `[Main]`
//! activation refers to. The mask is first-match-wins per zone slot, so there
//! is precisely one effect per surfaced bit. Centralizing that selection here
//! keeps the emitter and the decoder from ever drifting (the bug class this
//! module exists to prevent: a label naming a different effect than the one
//! the mask offered).
//!
//! Each `*_main_match` returns `Some(MainEffectMatch)` exactly when the mask
//! emits the corresponding bit, so `is_some()` is the emission predicate and
//! `name` is the label. An eligible effect with no printed name still returns
//! `Some` (with an empty `name`) — the bit is legal, the label just falls back
//! to the source card.

use crate::effect_context::EffectReadContext;
use crate::enums::{EffectTiming, Keyword, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;

/// The effect a `[Main]` activation resolves to.
pub(crate) struct MainEffectMatch {
    /// The matched effect's printed name; empty when the effect carries none.
    pub name: String,
}

impl MainEffectMatch {
    /// `effect_name` for the decoder: `None` when the matched effect has no
    /// name (so the label degrades to the source card alone).
    pub fn name_opt(&self) -> Option<String> {
        if self.name.is_empty() {
            None
        } else {
            Some(self.name.clone())
        }
    }
}

/// Hand `[Main]` (bits 30-59): the first effect on the hand card at `hand_idx`
/// whose timing is `MainFromHand` and whose condition (if any) passes.
pub(crate) fn hand_main_match(
    game: &Game,
    player_id: PlayerId,
    hand_idx: usize,
) -> Option<MainEffectMatch> {
    let card = game.player(player_id).hand.get(hand_idx)?;
    let card_id = card.card_id(&game.card_data);
    let effects = game.effects_for_card(card_id, card.handle())?;
    for effect in &effects {
        if effect.timing != EffectTiming::MainFromHand {
            continue;
        }
        if let Some(cond) = &effect.condition {
            let ctx = EffectReadContext::new(game, card.handle(), None, player_id);
            if !cond(&ctx) {
                continue;
            }
        }
        return Some(MainEffectMatch {
            name: effect.name.clone(),
        });
    }
    None
}

/// Field `[Main]` (bits 1000-1149, sub-slot +2): first-match-wins across the
/// permanent's digivolution stack (bottom to top), honoring the inherited-vs-top
/// filter, once-per-turn lockout, and condition gate — exactly as the mask's
/// standard field emitter does. This intentionally does NOT cover the DigiLink
/// Shape-B (sub-slot +3) or delayed-Option activations, which are separate
/// non-card-effect bits handled by their own emitters.
pub(crate) fn field_main_match(
    game: &Game,
    player_id: PlayerId,
    perm_index: usize,
) -> Option<MainEffectMatch> {
    let perm = game.player(player_id).battle_area.get(perm_index)?;
    let perm_handle = PermanentHandle {
        player: player_id,
        index: perm_index as u8,
    };
    let stack_size = perm.card_sources.len();
    for (source_index, source) in perm.card_sources.iter().enumerate() {
        let is_under = source_index + 1 < stack_size;
        let card_id = source.card_id(&game.card_data);
        let Some(effects) = game.effects_for_card(card_id, source.handle()) else {
            continue;
        };
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::MainOnField {
                continue;
            }
            if is_under != effect.inherited {
                continue;
            }
            // G-OPT-MULTI-TIMING-SHARED-LOCKOUT: a multi-timing OPT cluster
            // shares one counter via `shared_opt_group`.
            let opt_key = effect.shared_opt_group.unwrap_or(slot as u8);
            if effect.max_per_turn > 0
                && perm.activation_count(source.handle(), opt_key) >= effect.max_per_turn
            {
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx =
                    EffectReadContext::new(game, source.handle(), Some(perm_handle), player_id);
                if !cond(&ctx) {
                    continue;
                }
            }
            return Some(MainEffectMatch {
                name: effect.name.clone(),
            });
        }
    }
    None
}

/// Breeding-area `<Training>` (action 1142): gated purely on the breeding-area
/// top card carrying `Keyword::Training` and not being suspended (matching the
/// mask emitter, which deliberately does not run the effect's own condition
/// closure here). The name is a best-effort read of the top card's
/// non-inherited `MainOnField` effect (`<Training>` auto-effect).
pub(crate) fn breeding_training_match(
    game: &Game,
    player_id: PlayerId,
) -> Option<MainEffectMatch> {
    let breeding = game.player(player_id).breeding_area.as_ref()?;
    let top = breeding.top_card();
    let top_data = &game.card_data[top.data_index];
    if !top_data.keywords.contains(&Keyword::Training) || breeding.is_suspended {
        return None;
    }
    let card_id = top.card_id(&game.card_data);
    let name = game
        .effects_for_card(card_id, top.handle())
        .and_then(|effects| {
            effects
                .iter()
                .find(|e| e.timing == EffectTiming::MainOnField && !e.inherited)
                .map(|e| e.name.clone())
        })
        .unwrap_or_default();
    Some(MainEffectMatch { name })
}

/// Trash `[Main]` (bits 1150-1194): the first effect on the trash card at
/// `trash_idx` whose timing is `MainFromTrash` and whose condition (if any)
/// passes.
pub(crate) fn trash_main_match(
    game: &Game,
    player_id: PlayerId,
    trash_idx: usize,
) -> Option<MainEffectMatch> {
    let card = game.player(player_id).trash.get(trash_idx)?;
    let card_id = card.card_id(&game.card_data);
    let effects = game.effects_for_card(card_id, card.handle())?;
    for effect in &effects {
        if effect.timing != EffectTiming::MainFromTrash {
            continue;
        }
        if let Some(cond) = &effect.condition {
            let ctx = EffectReadContext::new(game, card.handle(), None, player_id);
            if !cond(&ctx) {
                continue;
            }
        }
        return Some(MainEffectMatch {
            name: effect.name.clone(),
        });
    }
    None
}
