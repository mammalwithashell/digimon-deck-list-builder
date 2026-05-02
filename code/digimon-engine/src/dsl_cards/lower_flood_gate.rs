//! Lower `CompiledDeclarativeClause::FloodGate`. On each declarative tick,
//! the process closure scans battle areas for permanents matching `target`
//! and installs `ModifierType::<name>` on each with `Expiry::Permanent`.
//! These entries are tagged as materialized declaratives so each tick can
//! clear and refresh them without stacking or leaving stale flood-gate state.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::expiry_map::lookup_expiry;
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
    target: Option<CompiledPredicate>,
    target_player: Option<CompiledPlayerRef>,
    expiry: Option<&str>,
) -> Option<Effect> {
    let modifier = lookup_modifier_type(modifier_name)?;
    let active_when = active_when.map(Arc::new);
    let target_arc = target.map(Arc::new);
    let player_expiry = expiry.and_then(lookup_expiry).unwrap_or(Expiry::Permanent);
    let label = format!("Flood gate: {modifier_name}");

    let mut builder: EffectBuilder = Effect::declarative(card).name(&label);
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    builder = builder
        .materializes_declarative_state()
        .process(move |ctx| {
            // active_when gate — evaluated under a read borrow.
            {
                let rctx = ctx.as_read();
                if let Some(aw) = &active_when {
                    if !eval_predicate(aw, &rctx, PredicateSubject::None) {
                        return;
                    }
                }
            }
            if let Some(target_player) = target_player {
                for player in players_for_ref(target_player, ctx) {
                    ctx.add_declarative_player_modifier(player, modifier, 0, player_expiry);
                }
            }

            let Some(target_arc) = &target_arc else {
                return;
            };

            // Collect target handles under a read borrow.
            let mut targets: Vec<PermanentHandle> = Vec::new();
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
                            targets.push(handle);
                        }
                    }
                }
            }
            // Install the modifier on each match via the curated EffectContext API.
            for h in targets {
                ctx.add_declarative_modifier(h, modifier, 0, Expiry::Permanent);
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
