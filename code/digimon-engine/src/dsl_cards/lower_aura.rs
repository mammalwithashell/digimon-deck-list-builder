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
//!
//! Track H §4 — `while_condition` extension. When the aura body carries a
//! `while_condition` predicate, lowering switches off the declarative-tick
//! path entirely. Instead a pair of triggered effects (`OnPlay` +
//! `OnDigivolve`) install the modifier exactly once, tagged with
//! `Expiry::UntilCondition` carrying the predicate. The UntilCondition
//! controller (PR #458) handles eviction; the printed-semantics rule
//! (`false → true` does NOT re-install) holds because the install happens
//! once at field-entry, not per-tick. DCGO reference: the
//! `IVortexCanAttackPlayersEffect.CanUse(null)` guard inside
//! `Vortex.cs:PermanentHasVortexCanAttackPlayers` is the lazy-filter
//! equivalent — DCGO consults at attack-target time, while we evict at
//! mutation events. End behavior matches.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledFormula, CompiledGrantKeywordValue, CompiledPlayerRef, CompiledPredicate, CompiledScope,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::modifier_map::{lookup_keyword, lookup_modifier_type};
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, ModifierType, PlayerId};
use crate::modifiers::ModifierSubject;
use crate::permanent::PermanentHandle;

/// Public entry point. Returns one or more `Effect`s.
///
/// - Without `while_condition`: returns a single declarative-tick Effect
///   via the legacy `lower(...)` path.
/// - With `while_condition` on a self-aura body: returns one OnPlay and
///   one OnDigivolve effect that install the modifier with
///   `Expiry::UntilCondition` carrying the compiled predicate. Both
///   trigger paths fire on field-entry; the registry de-dupes via its
///   normal source-permanent semantics.
#[allow(clippy::too_many_arguments)]
pub fn lower_all(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    target: CompiledPredicate,
    target_player: Option<CompiledPlayerRef>,
    dp_modifier: Option<i32>,
    dp_modifier_fn: Option<CompiledFormula>,
    security_attack: Option<i32>,
    security_attack_fn: Option<CompiledFormula>,
    grant_keyword: Option<CompiledGrantKeywordValue>,
    modifier: Option<String>,
    while_condition: Option<CompiledPredicate>,
    applies_to_opponent_security_dp: bool,
    raw: Arc<EngineRawRustRegistry>,
) -> Vec<Effect> {
    if applies_to_opponent_security_dp {
        // PUPPETS-G008: this is NOT a target-permanent aura — it rides on the
        // attacker's effect chain during the security battle. Lower directly
        // to a declarative inherited effect with the `dp_modifier` flag
        // wired through `applies_to_opponent_security_dp()`. The other aura
        // fields (target, target_player, grant_keyword, modifier, ...) are
        // ignored on this code path; the YAML author should leave them unset.
        let mut builder = Effect::declarative(card).name("Aura (opponent security DP)");
        if matches!(scope, CompiledScope::Inherited) {
            builder = builder.inherited();
        }
        if let Some(aw) = active_when.map(Arc::new) {
            builder = builder
                .condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None));
        }
        if let Some(dp) = dp_modifier {
            builder = builder.dp_modifier(dp);
        }
        builder = builder.applies_to_opponent_security_dp();
        return vec![builder.build()];
    }
    if let Some(predicate) = while_condition.clone() {
        let is_self_aura = target == CompiledPredicate::default();
        let supports = is_self_aura
            && target_player.is_none()
            && (dp_modifier.is_some()
                || security_attack.is_some()
                || modifier.is_some()
                || grant_keyword.is_some());
        if supports {
            return lower_self_while_condition(
                card,
                scope,
                active_when,
                predicate,
                dp_modifier,
                security_attack,
                modifier,
                grant_keyword,
            );
        }
        // For unsupported shapes (filter-aura, player-scope) the legacy
        // declarative path still fires. The gap tracker documents which
        // combinations are not yet wired.
    }
    if let Some(effect) = lower(
        card,
        scope,
        active_when,
        target,
        target_player,
        dp_modifier,
        dp_modifier_fn,
        security_attack,
        security_attack_fn,
        grant_keyword,
        modifier,
        raw,
    ) {
        vec![effect]
    } else {
        Vec::new()
    }
}

/// Build the `(OnPlay, OnDigivolve)` pair for a self-aura with
/// `while_condition`. Both effects share an identical install closure;
/// either timing covers a field-entry path.
fn lower_self_while_condition(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    predicate: CompiledPredicate,
    dp_modifier: Option<i32>,
    security_attack: Option<i32>,
    modifier: Option<String>,
    grant_keyword: Option<CompiledGrantKeywordValue>,
) -> Vec<Effect> {
    let active_when = active_when.map(Arc::new);
    let predicate = Arc::new(predicate);
    let modifier_type = modifier.as_deref().and_then(lookup_modifier_type);
    let granted_kw = grant_keyword.and_then(|g| lookup_keyword(&g.keyword, g.value));
    let dp = dp_modifier;
    let sa = security_attack;

    let install = move |ctx: &mut crate::effect_context::EffectContext<'_>| {
        let Some(handle) = ctx.source_permanent else {
            return;
        };
        let player = ctx.player;
        let pred = predicate.clone();
        let until = Arc::new(
            move |game: &crate::game::Game, subject: ModifierSubject| -> bool {
                let target_handle = match subject {
                    ModifierSubject::Permanent(h) => h,
                    ModifierSubject::Player(_) => return false,
                };
                let rctx = crate::effect_context::EffectReadContext::new(
                    game,
                    game.player(target_handle.player)
                        .battle_area
                        .get(target_handle.index as usize)
                        .map(|p| p.top_card().handle())
                        .unwrap_or(crate::card_source::CardHandle(0)),
                    Some(target_handle),
                    player,
                );
                eval_predicate(&pred, &rctx, PredicateSubject::Permanent(target_handle))
            },
        );

        if let Some(value) = dp {
            ctx.add_modifier_with_until_condition(
                handle,
                ModifierType::ChangeDp,
                value,
                until.clone(),
            );
        }
        if let Some(value) = sa {
            ctx.add_modifier_with_until_condition(
                handle,
                ModifierType::SecurityAttackChange,
                value,
                until.clone(),
            );
        }
        if let Some(modifier_type) = modifier_type {
            ctx.add_modifier_with_until_condition(handle, modifier_type, 0, until.clone());
        }
        if let Some(kw) = granted_kw {
            // Track H §4 — keyword grant + while_condition. Uses the
            // KeywordEntry.until_condition extension; same controller
            // drives eviction.
            ctx.grant_keyword_with_until_condition(handle, kw, until.clone());
        }
    };
    let install_arc: Arc<
        dyn Fn(&mut crate::effect_context::EffectContext<'_>) + Send + Sync + 'static,
    > = Arc::new(install);

    let inherited = matches!(scope, CompiledScope::Inherited);
    let make = |timing_builder: EffectBuilder, name: &str| -> Effect {
        let install = install_arc.clone();
        let mut b = timing_builder.name(name).process(move |ctx| install(ctx));
        if inherited {
            b = b.inherited();
        }
        if let Some(aw) = active_when.clone() {
            b = b.condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None));
        }
        b.build()
    };

    vec![
        make(Effect::on_play(card), "Aura (while_condition / OnPlay)"),
        make(
            Effect::on_digivolve(card),
            "Aura (while_condition / OnDigivolve)",
        ),
    ]
}

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    target: CompiledPredicate,
    target_player: Option<CompiledPlayerRef>,
    dp_modifier: Option<i32>,
    dp_modifier_fn: Option<CompiledFormula>,
    security_attack: Option<i32>,
    security_attack_fn: Option<CompiledFormula>,
    grant_keyword: Option<CompiledGrantKeywordValue>,
    modifier: Option<String>,
    raw: Arc<EngineRawRustRegistry>,
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

    if is_self_aura && target_player.is_none() && modifier.is_none() && security_attack.is_none() {
        if let Some(dp) = dp_modifier {
            builder = builder.dp_modifier(dp);
        }
        if let Some(formula) = dp_modifier_fn.clone() {
            builder = builder.dp_modifier_fn(dynamic_formula_fn(
                formula,
                target.clone(),
                true,
                raw.clone(),
            ));
        }
        if let Some(formula) = security_attack_fn.clone() {
            builder = builder.security_attack_fn(dynamic_formula_fn(
                formula,
                target.clone(),
                true,
                raw.clone(),
            ));
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
    let dp_formula = if is_self_aura { None } else { dp_modifier_fn };
    let security_attack_formula = if is_self_aura {
        None
    } else {
        security_attack_fn
    };
    let sa_flat = security_attack;
    let gk = grant_keyword.and_then(|g| lookup_keyword(&g.keyword, g.value));
    let modifier = modifier.as_deref().and_then(lookup_modifier_type);

    if is_self_aura && target_player.is_none() {
        builder = builder
            .materializes_declarative_state()
            .process(move |ctx| {
                let Some(handle) = ctx.source_permanent else {
                    return;
                };
                if let Some(dp) = dp {
                    ctx.add_declarative_dp_modifier(handle, dp, Expiry::Permanent);
                }
                if let Some(sa) = sa_flat {
                    ctx.add_declarative_modifier(
                        handle,
                        ModifierType::SecurityAttackChange,
                        sa,
                        Expiry::Permanent,
                    );
                }
                if let Some(kw) = gk {
                    ctx.grant_declarative_keyword(handle, kw, Expiry::Permanent);
                }
                if let Some(modifier) = modifier {
                    ctx.add_declarative_modifier(handle, modifier, 0, Expiry::Permanent);
                }
            });
        return Some(builder.build());
    }

    if let Some(formula) = dp_formula {
        builder = builder.dp_modifier_fn(dynamic_formula_fn(
            formula,
            (*target_arc).clone(),
            false,
            raw.clone(),
        ));
    }
    if let Some(formula) = security_attack_formula {
        builder = builder.security_attack_fn(dynamic_formula_fn(
            formula,
            (*target_arc).clone(),
            false,
            raw.clone(),
        ));
    }

    builder = builder
        .materializes_declarative_state()
        .process(move |ctx| {
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
                if let Some(sa) = sa_flat {
                    ctx.add_declarative_modifier(
                        h,
                        ModifierType::SecurityAttackChange,
                        sa,
                        Expiry::Permanent,
                    );
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

fn dynamic_formula_fn(
    formula: CompiledFormula,
    target: CompiledPredicate,
    self_only: bool,
    raw: Arc<EngineRawRustRegistry>,
) -> impl Fn(&crate::effect_context::EffectReadContext<'_>, PermanentHandle) -> Option<i32>
       + Send
       + Sync
       + 'static {
    let target = Arc::new(target);
    move |ctx, handle| {
        if self_only {
            if ctx.source_permanent != Some(handle) {
                return None;
            }
        } else if !eval_predicate(&target, ctx, PredicateSubject::Permanent(handle)) {
            return None;
        }
        Some(formula_eval::evaluate_read_with_raw(
            &formula, ctx, handle, &raw,
        ))
    }
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
