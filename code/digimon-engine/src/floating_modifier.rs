//! Floating continuous mass modifiers — source-independent, turn-scoped field
//! effects re-applied to a *live* candidate set every declarative tick.
//!
//! Motivating card: EX4-074 ShineGreymon: Ruin Mode —
//!   "[When Digivolving] [On Deletion] Until the end of your opponent's next
//!    turn, all of your opponent's Digimon get -5000DP."
//!
//! A one-time `add_modifier` scan (the old behavior) only debuffs the Digimon
//! present at install time. Per the rules a continuous "all X get ±Y until Z"
//! effect ALSO applies to Digimon that ENTER during the window
//! (G-CONTINUOUS-MASS-DP-DEBUFF, judge-quiz Q14). And because the trigger can be
//! `[On Deletion]`, the effect must survive the source leaving the field — so it
//! cannot be a while-in-play aura bound to a source permanent.
//!
//! Model: a data-only descriptor stored on `Game`. It carries the SAME
//! `CompiledPredicate` the DSL `add_modifier` target used (e.g. `{ of: opponent,
//! kind: digimon }`, evaluated relative to `source_player`, so "opponent" stays
//! correct after the source is gone). Each `tick_declarative_effects` re-scans
//! with that predicate and installs a *materialized-declarative* modifier on
//! every current match (cleared + re-installed each tick, exactly like a
//! source-bound aura). The DESCRIPTOR'S own `expiry` (+ `pending_skips` for the
//! `*NextTurn` semantics) governs the window and is pruned at turn-end via
//! `Game::expire_floating_mass_modifiers`.
//!
//! Closure-free by design (only `CompiledPredicate`, which is `Clone + Serialize`)
//! so it does not obstruct the in-progress `make-engine-cloneable` work.

use digimon_dsl::compiled::CompiledPredicate;

use crate::card_source::CardHandle;
use crate::enums::{Expiry, ModifierType, PlayerId};

/// A source-independent continuous modifier applied to a live, predicate-matched
/// candidate set every declarative tick until its `expiry` fires.
#[derive(Debug, Clone)]
pub struct FloatingMassModifier {
    /// Target predicate, evaluated relative to `source_player` (so `of:
    /// opponent` resolves against the installing controller even after the
    /// source card has left the field).
    pub filter: CompiledPredicate,
    /// The modifier kind to install on each match (e.g. `ChangeDp`).
    pub modifier: ModifierType,
    /// Signed magnitude (e.g. `-5000`).
    pub value: i32,
    /// The installing card — only used to build the per-tick predicate
    /// evaluation context; the predicates these cards use (`of: opponent`,
    /// `kind: digimon`) do not reference "this card", so a trash-resident
    /// handle is fine.
    pub source_card: CardHandle,
    /// The controller of the installing effect — fixes "opponent"/"your"
    /// relativity for the whole lifetime.
    pub source_player: PlayerId,
    /// Window end. Turn-relative expiries (`EndOfOpponentsNextTurn`,
    /// `EndOfYourTurn`, …) are honored at turn-end; `pending_skips` carries the
    /// `*NextTurn` skip count.
    pub expiry: Expiry,
    /// Turn-end firings still to skip before this descriptor expires — the same
    /// mechanism `ModifierEntry` uses (`modifiers::pending_skips_for_install`).
    pub pending_skips: u8,
    /// When `Some`, each materialized per-permanent entry carries this
    /// effect-immunity filter (the modifier is a `CannotBeAffected`-class
    /// protection rather than a numeric delta). Continuous controlled
    /// immunity — G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA (judge-quiz
    /// Q28 / BT20-059): the live re-scan covers permanents played later in
    /// the window.
    pub effect_immunity: Option<crate::modifiers::EffectImmunityFilter>,
}
