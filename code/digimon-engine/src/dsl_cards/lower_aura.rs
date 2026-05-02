//! Lower `CompiledDeclarativeClause::Aura`.
//!
//! Two shapes handled:
//!
//! 1. Self aura (empty target) — use the builder's static `.dp_modifier(n)`
//!    so the tensor layer reads it without invoking the process closure.
//! 2. Filtered aura (non-empty target) — emit a declarative Effect whose
//!    process scans battle areas and applies `add_dp_modifier` +
//!    `grant_keyword` to each matching permanent. These entries are tagged
//!    as materialized declaratives so each declarative tick can clear and
//!    refresh them without stacking or leaving stale aura state behind.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledGrantKeywordValue, CompiledPlayerRef, CompiledPredicate, CompiledScope,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::{lookup_keyword, lookup_modifier_type};
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, PlayerId};
use crate::permanent::PermanentHandle;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    target: CompiledPredicate,
    target_player: Option<CompiledPlayerRef>,
    dp_modifier: Option<i32>,
    grant_keyword: Option<CompiledGrantKeywordValue>,
    modifier: Option<String>,
) -> Option<Effect> {
    let is_self_aura = target == CompiledPredicate::default();
    let active_when = active_when.map(Arc::new);

    let mut builder: EffectBuilder = Effect::declarative(card).name("Aura");
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if let Some(aw) = active_when.clone() {
        builder = builder.condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None));
    }

    if is_self_aura && target_player.is_none() && modifier.is_none() {
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
    let modifier = modifier.as_deref().and_then(lookup_modifier_type);

    builder = builder.process(move |ctx| {
        if let (Some(player_ref), Some(modifier)) = (target_player, modifier) {
            for player in players_for_ref(player_ref, ctx) {
                ctx.add_declarative_player_modifier(player, modifier, 0, Expiry::Permanent);
            }
            return;
        }

        // Collect matching targets under a read borrow.
        let mut matched: Vec<PermanentHandle> = Vec::new();
        {
            let rctx = ctx.as_read();
            let n_players = rctx.game.players.len() as PlayerId;
            for p in 0..n_players {
                let m = rctx.game.player(p).battle_area.len();
                for i in 0..m {
                    let handle = PermanentHandle {
                        player: p,
                        index: i as u8,
                    };
                    if eval_predicate(&target_arc, &rctx, PredicateSubject::Permanent(handle)) {
                        matched.push(handle);
                    }
                }
            }
        }
        // Apply outside the read borrow.
        for h in matched {
            if let Some(dp) = dp {
                ctx.add_declarative_dp_modifier(h, dp, Expiry::Permanent);
            }
            if let Some(kw) = gk {
                ctx.grant_declarative_keyword(h, kw, Expiry::Permanent);
            }
            if let Some(modifier) = modifier {
                ctx.add_declarative_modifier(h, modifier, 0, Expiry::Permanent);
            }
        }
    });

    Some(builder.build())
}

fn players_for_ref(
    of: CompiledPlayerRef,
    ctx: &crate::effect_context::EffectContext<'_>,
) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![ctx.player],
        CompiledPlayerRef::Opponent => vec![ctx.opponent_id()],
        CompiledPlayerRef::Active => vec![ctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..ctx.game.players.len() as PlayerId).collect(),
    }
}
