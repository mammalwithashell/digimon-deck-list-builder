//! Lower `CompiledDeclarativeClause::FloodGate`. On each declarative tick,
//! the process closure scans battle areas for permanents matching `target`
//! and installs `ModifierType::<name>` on each with `Expiry::Permanent`.
//! The modifier registry dedups identical entries so re-installation is
//! safe.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_modifier_type;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, PlayerId};
use crate::permanent::PermanentHandle;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    modifier_name: &str,
    target: CompiledPredicate,
) -> Option<Effect> {
    let modifier = lookup_modifier_type(modifier_name)?;
    let active_when = active_when.map(Arc::new);
    let target_arc = Arc::new(target);
    let label = format!("Flood gate: {modifier_name}");

    let mut builder: EffectBuilder = Effect::declarative(card).name(&label);
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    builder = builder.process(move |ctx| {
        // active_when gate — evaluated under a read borrow.
        {
            let rctx = ctx.as_read();
            if let Some(aw) = &active_when {
                if !eval_predicate(aw, &rctx, PredicateSubject::None) {
                    return;
                }
            }
        }
        // Collect target handles under a read borrow.
        let mut targets: Vec<PermanentHandle> = Vec::new();
        {
            let rctx = ctx.as_read();
            let n_players = rctx.game.players.len() as PlayerId;
            for p in 0..n_players {
                let m = rctx.game.player(p).battle_area.len();
                for i in 0..m {
                    let handle = PermanentHandle { player: p, index: i as u8 };
                    if eval_predicate(&target_arc, &rctx, PredicateSubject::Permanent(handle)) {
                        targets.push(handle);
                    }
                }
            }
        }
        // Install the modifier on each match via the curated EffectContext API.
        for h in targets {
            ctx.add_modifier(h, modifier, 0, Expiry::Permanent);
        }
    });

    Some(builder.build())
}
