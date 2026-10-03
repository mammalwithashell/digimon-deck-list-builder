//! `<Succession ([X])>` — effect adoption from a digivolution card
//! (G-ENGINE-SUCCESSION-KEYWORD).
//!
//! "This Digimon gains all effects other than <Succession> on its topmost
//! specified digivolution card." DCGO: `SuccessionSelfEffect` →
//! `CardEffectFactory.CopyDigivolutionCardEffects(isSuccession: true)`: while
//! the card printing the keyword is the permanent's TOP card, the permanent
//! gains every non-inherited, non-linked effect printed on the topmost
//! digivolution card matching `[X]`, re-evaluated continuously (a copied
//! trigger checks the card is still the adopted one at trigger AND at
//! activation time).
//!
//! The engine models an adopted card as a below-top source whose TOP-scope
//! effects are also active. Every scan that walks a permanent's stack with
//! the "top card → non-inherited, below-top → inherited" rule asks
//! [`Game::source_effect_is_active`] instead, so adoption reaches triggers,
//! keyword queries, declarative auras, static DP, replacements and `[Main]`
//! activations through one predicate.
//!
//! A copied once-per-turn effect is tracked independently of the source
//! card's own uses (DCGO builds a fresh `ActivateClass` per copy, and
//! redirects the body's `RemoveUse` to it): see [`Game::opt_key_for_source`].

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::permanent::PermanentHandle;

/// XOR-ed into the OPT counter key of an adopted (copied) effect so it never
/// shares a counter with the same card's own uses on the same permanent.
/// Raw effect slots stay below 0x40 and `shared_opt_group` keys are
/// `0x80 | clause_index` (clause_index < 0x40), so the flipped key is disjoint
/// from both.
pub(crate) const ADOPTED_OPT_KEY_BIT: u8 = 0x40;

impl crate::game::Game {
    /// The Option use cost of `card` for `player`: its printed use cost (a
    /// DUAL's Option face, else the play cost) plus every self use-cost
    /// increase printed on the card (G-ENGINE-OPTION-SELF-USE-COST-INCREASE,
    /// DCGO `ChangeCostClass` on the card itself). Cost REDUCTIONS are applied
    /// later, at payment.
    pub fn option_use_cost(
        &self,
        card: &crate::card_source::CardSource,
        player: crate::enums::PlayerId,
    ) -> u16 {
        let base = card
            .option_use_cost(&self.card_data)
            .unwrap_or_else(|| card.play_cost(&self.card_data)) as i32;
        let card_id = card.card_id(&self.card_data).to_string();
        let increase: i32 = self
            .effects_for_card(&card_id, card.handle())
            .map(|effects| {
                let ctx = crate::effect_context::EffectReadContext::new(
                    self,
                    card.handle(),
                    None,
                    player,
                );
                effects
                    .iter()
                    .filter_map(|e| e.use_cost_increase_fn.as_ref())
                    .map(|f| f(&ctx))
                    .sum()
            })
            .unwrap_or(0);
        (base + increase).clamp(0, u16::MAX as i32) as u16
    }

    /// Indices into the battle-area permanent's `card_sources` whose
    /// non-inherited effects the permanent gains via `<Succession>` printed on
    /// its top card: for each Succession clause, the TOPMOST below-top card
    /// its filter matches. Empty for every permanent without the keyword.
    pub fn succession_source_indices(&self, handle: PermanentHandle) -> Vec<usize> {
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return Vec::new();
        };
        let len = perm.card_sources.len();
        if len < 2 {
            return Vec::new();
        }
        let top = perm.top_card();
        let top_id = top.card_id(&self.card_data).to_string();
        let top_handle = top.handle();
        let Some(effects) = self.effects_for_card(&top_id, top_handle) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for effect in effects.iter() {
            let Some(filter) = effect.succession_filter.as_ref() else {
                continue;
            };
            if effect.inherited || effect.linked {
                continue;
            }
            let ctx = crate::effect_context::EffectReadContext::new(
                self,
                top_handle,
                Some(handle),
                handle.player,
            );
            if let Some(cond) = &effect.condition {
                if !cond(&ctx) {
                    continue;
                }
            }
            for source_index in (0..len - 1).rev() {
                let source = &perm.card_sources[source_index];
                let sref = crate::selection::SourceSelectionRef {
                    permanent: handle,
                    field_index: handle.index,
                    source_index: source_index as u8,
                    card: source.handle(),
                };
                if filter(&ctx, sref) {
                    if !out.contains(&source_index) {
                        out.push(source_index);
                    }
                    break;
                }
            }
        }
        out
    }

    /// True when `card` is a below-top card of `handle` whose top-scope
    /// effects are adopted via `<Succession>`.
    pub fn is_adopted_source_card(&self, handle: PermanentHandle, card: CardHandle) -> bool {
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return false;
        };
        self.succession_source_indices(handle)
            .into_iter()
            .any(|i| perm.card_sources.get(i).is_some_and(|c| c.handle() == card))
    }

    /// Whether `effect`, printed on the card at `source_index` of `handle`'s
    /// stack (`stack_size` cards), is active for the permanent: the top card's
    /// non-inherited effects, a below-top card's inherited effects, and — for
    /// a card adopted via `<Succession>` (`adopted`) — also its adoptable
    /// top-scope effects. Callers pass `adopted` from
    /// [`Game::succession_source_indices`] (computed once per stack walk).
    pub fn source_effect_is_active(
        source_index: usize,
        stack_size: usize,
        adopted: bool,
        effect: &Effect,
    ) -> bool {
        let is_top = source_index + 1 == stack_size;
        if is_top {
            return !effect.inherited;
        }
        effect.inherited || (adopted && Self::is_adoptable_effect(effect))
    }

    /// The effects `<Succession>` copies: everything on the card's face
    /// (triggered, declarative, keyword auto-effects, replacements, `[Main]`)
    /// EXCEPT inherited / link-only / trash-zone / security-zone effects and
    /// the card's own `<Succession>` ("all effects other than <Succession>").
    pub fn is_adoptable_effect(effect: &Effect) -> bool {
        !effect.inherited
            && !effect.linked
            && !effect.trash_zone
            && !effect.security
            && effect.succession_filter.is_none()
    }

    /// OPT counter key for `effect` (base key `key`) fired from `source_card`
    /// on `handle`: a non-inherited effect whose card is NOT the permanent's
    /// top card can only be an adopted copy, which keeps its own counter.
    pub(crate) fn opt_key_for_source(
        &self,
        handle: PermanentHandle,
        source_card: CardHandle,
        effect_inherited: bool,
        effect_linked: bool,
        key: u8,
    ) -> u8 {
        if effect_inherited || effect_linked {
            return key;
        }
        let is_under_top = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
            .is_some_and(|perm| {
                perm.card_sources
                    .iter()
                    .take(perm.card_sources.len().saturating_sub(1))
                    .any(|c| c.handle() == source_card)
            });
        if is_under_top {
            key ^ ADOPTED_OPT_KEY_BIT
        } else {
            key
        }
    }
}
