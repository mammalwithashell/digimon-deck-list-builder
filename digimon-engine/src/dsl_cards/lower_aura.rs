//! Lower `CompiledDeclarativeClause::Aura`.
//!
//! Two shapes handled:
//!
//! 1. Self aura (empty target) — use the builder's static `.dp_modifier(n)`
//!    so the tensor layer reads it without invoking the process closure.
//! 2. Filtered aura (non-empty target) — emit a declarative Effect whose
//!    process scans battle areas and applies `add_dp_modifier` +
//!    `grant_keyword` to each matching permanent. The engine's modifier
//!    registry dedups identical entries (same target/modifier/value/expiry)
//!    so re-installing on each declarative tick is safe.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledGrantKeywordValue, CompiledPredicate, CompiledScope,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_keyword;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, PlayerId};
use crate::permanent::PermanentHandle;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    target: CompiledPredicate,
    dp_modifier: Option<i32>,
    grant_keyword: Option<CompiledGrantKeywordValue>,
) -> Option<Effect> {
    let is_self_aura = target == CompiledPredicate::default();
    let active_when = active_when.map(Arc::new);

    let mut builder: EffectBuilder = Effect::declarative(card).name("Aura");
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if let Some(aw) = active_when.clone() {
        builder = builder.condition(move |rctx| {
            eval_predicate(&aw, rctx, PredicateSubject::None)
        });
    }

    if is_self_aura {
        if let Some(dp) = dp_modifier {
            builder = builder.dp_modifier(dp);
        }
        if grant_keyword.is_none() {
            return Some(builder.build());
        }
        // If the YAML put grant_keyword on a self aura, fall through to
        // the process path (shouldn't happen with current fixtures, but
        // keep the behavior well-defined).
    }

    let target_arc = Arc::new(target);
    let dp = dp_modifier;
    let gk = grant_keyword.and_then(|g| lookup_keyword(&g.keyword, g.value));

    builder = builder.process(move |ctx| {
        // Collect matching targets under a read borrow.
        let mut matched: Vec<PermanentHandle> = Vec::new();
        {
            let rctx = ctx.as_read();
            let n_players = rctx.game.players.len() as PlayerId;
            for p in 0..n_players {
                let m = rctx.game.player(p).battle_area.len();
                for i in 0..m {
                    let handle = PermanentHandle { player: p, index: i as u8 };
                    if eval_predicate(&target_arc, &rctx, PredicateSubject::Permanent(handle)) {
                        matched.push(handle);
                    }
                }
            }
        }
        // Apply outside the read borrow.
        for h in matched {
            if let Some(dp) = dp {
                ctx.add_dp_modifier(h, dp, Expiry::Permanent);
            }
            if let Some(kw) = gk {
                ctx.grant_keyword(h, kw, Expiry::Permanent);
            }
        }
    });

    Some(builder.build())
}
