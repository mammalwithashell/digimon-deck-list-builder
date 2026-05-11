//! Track H Phase 4k — typed `AuraScope` / `AuraGrant` builder surface.
//!
//! Wraps the existing `Effect::declarative` + `EffectContext::add_declarative_*`
//! / `grant_declarative_*` install APIs in a fluent builder for raw_rust card
//! scripts. End behavior is identical to direct API calls; this surface is
//! ergonomic-only — typed enums prevent miscalls (wrong ModifierType for
//! payload, wrong target shape for filter, etc.).
//!
//! YAML-authored cards continue to use the field-slot DSL (`kind: aura` body
//! with `dp_modifier`/`security_attack`/`grant_keyword`/`modifier` slots);
//! that path is unchanged.
//!
//! Usage:
//!
//! ```ignore
//! Effect::declarative(card)
//!     .name("All your Holy Digimon gain +1000 DP")
//!     .aura()
//!         .scope(AuraScope::Player(controller))
//!         .target_filter(|rctx, h| {
//!             // ... per-permanent filter predicate
//!             true
//!         })
//!         .grants(AuraGrant::Dp { value: 1000, base: false, origin: false })
//!         .duration(Expiry::EndOfYourTurn)
//!     .build()
//! ```
//!
//! Spec reference: §1 / §2 from the Track H spec
//! (`.claude/plans/track-h-aura-system.md`).

use std::sync::Arc;

use crate::effect::{Effect, EffectBuilder};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{Expiry, Keyword, ModifierType, PlayerId};
use crate::modifiers::UntilConditionFn;
use crate::permanent::PermanentHandle;

/// Predicate evaluated at consult time against each candidate
/// permanent. Returns `true` if the candidate matches the filter.
pub type AuraTargetFilter =
    Arc<dyn Fn(&EffectReadContext<'_>, PermanentHandle) -> bool + Send + Sync + 'static>;

/// Who the aura emits over (mirrors DCGO `GiveEffectToPermanent` /
/// `GiveEffectToPlayer` distinction).
#[derive(Debug, Clone, Copy)]
pub enum AuraScope {
    /// Single-target aura — the existing `grant_keyword(target, ...)`
    /// shape. Filter is ignored; only `target` receives the grant.
    Permanent(PermanentHandle),
    /// Filter-aura over `player_id`'s permanents. Filter sees every
    /// permanent owned by that player.
    Player(PlayerId),
    /// Cross-side aura. `player_id` is the SOURCE player; the filter
    /// scans the opponent's permanents. Duration-flip rule (DCGO
    /// `GiveEffectToPermanentOrPlayer.cs:17-37`) is honored implicitly
    /// because expiry is keyed by `source_player` on the modifier
    /// entry — the same source-relative semantics DCGO produces after
    /// flipping.
    OpponentPlayer(PlayerId),
    /// Filter sees permanents on both sides. Filter predicate
    /// narrows if needed.
    Bilateral,
    /// Security-zone-sourced aura. Source is in `player_id`'s
    /// security stack; filter sees battle-area permanents per
    /// printed text. Wired through `tick_declarative_effects`'s
    /// face-up security walk (Phase 4 §5).
    SecurityZone(PlayerId),
}

/// What the aura grants to each match. v1 covers all printed-text
/// shapes through the existing modifier registry / keyword registry
/// installs.
#[derive(Debug, Clone)]
pub enum AuraGrant {
    /// Grant a printed-equivalent keyword (Rush, Reboot, Piercing,
    /// Blocker, Jamming, Save, Vortex, Decoy(color), etc.).
    Keyword(Keyword),
    /// DP modification. `base: true` overrides printed DP (rare); v1
    /// treats both `base` and `origin` as advisory metadata — the
    /// install lands `ModifierType::ChangeDp` carrying `value`.
    Dp {
        value: i32,
        base: bool,
        origin: bool,
    },
    /// Security A. ±N grant. Maps to `ModifierType::SecurityAttackChange`
    /// with the literal delta.
    SecurityAttack(i32),
    /// Play / digivolve / link cost ±N. Maps through the existing
    /// modifier types.
    PlayCost(i32),
    DigivolutionCost(i32),
    LinkCost(i32),
    /// Narrow opponent-effect / battle-deletion immunity. Maps to the
    /// matching `ModifierType` variant.
    Immunity(ImmunityKind),
    /// Restriction grant — "cannot suspend", "cannot block", etc.
    Cannot(CannotKind),
    /// Escape hatch for variants that don't yet have a typed slot.
    /// Use `AuraGrant::Modifier { ty, value }` when the existing
    /// modifier registry already covers the case via a string-mapped
    /// `ModifierType`.
    Modifier {
        ty: ModifierType,
        value: i32,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ImmunityKind {
    /// Immune to opponent's DP-reduction effects.
    OpponentDpReduction,
    /// Immune to opponent's de-digivolve effects.
    OpponentDeDigivolve,
    /// Cannot be deleted by battle.
    BattleDeletion,
    /// Cannot be deleted by effect.
    EffectDeletion,
}

#[derive(Debug, Clone, Copy)]
pub enum CannotKind {
    Suspend,
    Unsuspend,
    Block,
    Attack,
    AttackPlayer,
    ReturnToHand,
    ReturnToDeck,
    DeDigivolve,
}

impl AuraGrant {
    /// Convert a typed grant into the `(ModifierType, value)` pair
    /// that lands through `add_declarative_modifier`. Keyword grants
    /// route through a separate path — return `None` for those.
    fn as_modifier(&self) -> Option<(ModifierType, i32)> {
        Some(match *self {
            AuraGrant::Keyword(_) => return None,
            AuraGrant::Dp { value, .. } => (ModifierType::ChangeDp, value),
            AuraGrant::SecurityAttack(value) => (ModifierType::SecurityAttackChange, value),
            AuraGrant::PlayCost(value) => (ModifierType::ChangePlayCost, value),
            AuraGrant::DigivolutionCost(value) => (ModifierType::ChangeDigivolveCost, value),
            AuraGrant::LinkCost(value) => (ModifierType::ChangeLinkCost, value),
            AuraGrant::Immunity(kind) => (
                match kind {
                    ImmunityKind::OpponentDpReduction => ModifierType::ImmuneFromDPMinus,
                    ImmunityKind::OpponentDeDigivolve => ModifierType::CannotBeDeDigivolved,
                    ImmunityKind::BattleDeletion => ModifierType::CannotBeDestroyedByBattle,
                    ImmunityKind::EffectDeletion => ModifierType::CannotBeDestroyedByEffect,
                },
                0,
            ),
            AuraGrant::Cannot(kind) => (
                match kind {
                    CannotKind::Suspend => ModifierType::CannotSuspend,
                    CannotKind::Unsuspend => ModifierType::CannotUnsuspend,
                    CannotKind::Block => ModifierType::CannotBlock,
                    CannotKind::Attack => ModifierType::CannotAttack,
                    CannotKind::AttackPlayer => ModifierType::CannotAttackPlayer,
                    CannotKind::ReturnToHand => ModifierType::CannotBeReturnedToHand,
                    CannotKind::ReturnToDeck => ModifierType::CannotBeReturnedToDeck,
                    CannotKind::DeDigivolve => ModifierType::CannotBeDeDigivolved,
                },
                0,
            ),
            AuraGrant::Modifier { ty, value } => (ty, value),
        })
    }

    fn as_keyword(&self) -> Option<Keyword> {
        match *self {
            AuraGrant::Keyword(kw) => Some(kw),
            _ => None,
        }
    }
}

/// Fluent builder returned by `EffectBuilder::aura()`. Accumulates
/// scope / filter / grants / duration state, then materializes into
/// an `Effect` via `build()`.
pub struct AuraBuilder {
    inner: EffectBuilder,
    scope: Option<AuraScope>,
    target_filter: Option<AuraTargetFilter>,
    grants: Vec<AuraGrant>,
    duration: Expiry,
    while_pred: Option<UntilConditionFn>,
}

impl AuraBuilder {
    pub(crate) fn new(inner: EffectBuilder) -> Self {
        Self {
            inner,
            scope: None,
            target_filter: None,
            grants: Vec::new(),
            duration: Expiry::Permanent,
            while_pred: None,
        }
    }

    /// Required. Sets who the aura emits over.
    pub fn scope(mut self, scope: AuraScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Optional. Filter predicate evaluated lazily at consult time
    /// against each candidate permanent. When unset, every permanent
    /// in the scope receives the grant.
    pub fn target_filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&EffectReadContext<'_>, PermanentHandle) -> bool + Send + Sync + 'static,
    {
        self.target_filter = Some(Arc::new(filter));
        self
    }

    /// Append one grant. Call multiple times to bundle several grants
    /// in the same aura (e.g., DP +1000 AND keyword Rush).
    pub fn grants(mut self, grant: AuraGrant) -> Self {
        self.grants.push(grant);
        self
    }

    /// Default `Expiry::Permanent`. Common values:
    /// `Permanent` (until source leaves field — UntilLeaveField is
    /// installed on the modifier when source-permanent is known),
    /// `EndOfYourTurn` / `EndOfOpponentsTurn`, `EndOfTurn`,
    /// `UntilCondition` (paired with `while_condition`).
    pub fn duration(mut self, expiry: Expiry) -> Self {
        self.duration = expiry;
        self
    }

    /// Sugar over `.duration(Expiry::UntilCondition)` plus storing the
    /// predicate. The UntilCondition controller (PR #458) handles
    /// eviction; printed-semantics rule (`false → true` does NOT
    /// re-install) holds.
    pub fn while_condition<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&crate::game::Game, crate::modifiers::ModifierSubject) -> bool
            + Send
            + Sync
            + 'static,
    {
        self.duration = Expiry::UntilCondition;
        self.while_pred = Some(Arc::new(predicate));
        self
    }

    /// Materialize the accumulated state into an `Effect`. The
    /// resulting Effect is a process-backed declarative whose
    /// `process` closure walks the field per scope and applies each
    /// grant via the existing typed install APIs.
    pub fn build(self) -> Effect {
        let AuraBuilder {
            inner,
            scope,
            target_filter,
            grants,
            duration,
            while_pred,
        } = self;
        let scope = scope.expect("AuraBuilder::scope() is required before build()");
        let grants = Arc::new(grants);
        let mut builder = inner.materializes_declarative_state();
        let process_filter = target_filter;
        let process_grants = grants;
        let process_while = while_pred;
        builder = builder.process(move |ctx| {
            apply_aura_grants(
                ctx,
                scope,
                process_filter.as_ref(),
                &process_grants,
                duration,
                process_while.as_ref(),
            );
        });
        builder.build()
    }
}

/// Walk the scope, evaluate the filter against each candidate, apply
/// each grant. Called from the lowered Effect's process closure each
/// declarative tick.
fn apply_aura_grants(
    ctx: &mut EffectContext<'_>,
    scope: AuraScope,
    filter: Option<&AuraTargetFilter>,
    grants: &[AuraGrant],
    duration: Expiry,
    while_pred: Option<&UntilConditionFn>,
) {
    let candidates: Vec<PermanentHandle> = match scope {
        AuraScope::Permanent(h) => vec![h],
        AuraScope::Player(player) => snapshot_player_battle_area(ctx, player, filter),
        AuraScope::OpponentPlayer(source_player) => {
            let opp = if source_player == 0 { 1 } else { 0 };
            snapshot_player_battle_area(ctx, opp, filter)
        }
        AuraScope::Bilateral | AuraScope::SecurityZone(_) => snapshot_all_battle_areas(ctx, filter),
    };

    for handle in candidates {
        for grant in grants {
            apply_one_grant(ctx, handle, grant, duration, while_pred);
        }
    }
}

fn snapshot_player_battle_area(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    filter: Option<&AuraTargetFilter>,
) -> Vec<PermanentHandle> {
    let rctx = ctx.as_read();
    let mut out = Vec::new();
    let n = rctx.game.player(player).battle_area.len();
    for i in 0..n {
        let h = PermanentHandle {
            player,
            index: i as u8,
        };
        if filter.map_or(true, |f| f(&rctx, h)) {
            out.push(h);
        }
    }
    out
}

fn snapshot_all_battle_areas(
    ctx: &EffectContext<'_>,
    filter: Option<&AuraTargetFilter>,
) -> Vec<PermanentHandle> {
    let rctx = ctx.as_read();
    let mut out = Vec::new();
    let n_players = rctx.game.players.len() as PlayerId;
    for p in 0..n_players {
        let m = rctx.game.player(p).battle_area.len();
        for i in 0..m {
            let h = PermanentHandle {
                player: p,
                index: i as u8,
            };
            if filter.map_or(true, |f| f(&rctx, h)) {
                out.push(h);
            }
        }
    }
    out
}

fn apply_one_grant(
    ctx: &mut EffectContext<'_>,
    target: PermanentHandle,
    grant: &AuraGrant,
    duration: Expiry,
    while_pred: Option<&UntilConditionFn>,
) {
    // UntilCondition path uses the typed-with-predicate install APIs.
    if matches!(duration, Expiry::UntilCondition) {
        let pred = match while_pred {
            Some(p) => p.clone(),
            None => return, // misconfigured — Until requires a predicate
        };
        if let Some(kw) = grant.as_keyword() {
            ctx.grant_keyword_with_until_condition(target, kw, pred);
            return;
        }
        if let Some((ty, value)) = grant.as_modifier() {
            ctx.add_modifier_with_until_condition(target, ty, value, pred);
        }
        return;
    }
    if let Some(kw) = grant.as_keyword() {
        ctx.grant_declarative_keyword(target, kw, duration);
        return;
    }
    if let Some((ty, value)) = grant.as_modifier() {
        ctx.add_declarative_modifier(target, ty, value, duration);
    }
}
