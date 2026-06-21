//! Resumable-effect-VM frame stack — `make-engine-cloneable` Phase 2 (task 2.1).
//!
//! Plain-data replacement for the `Box<dyn FnOnce>` callbacks on
//! [`crate::selection::PendingSelection`]. The legacy executor pauses a
//! mid-effect selection by parking a `SelectionCallback = Box<dyn FnOnce(&mut
//! Game, u16)>` (and an `on_decline`), and *composes* nested continuations by
//! wrapping one closure inside another. Neither is `Clone`/serializable, which
//! is the sole remaining blocker for `Game: Clone` (see
//! `make-engine-cloneable/design.md` — "Verified defunctionalization
//! inventory": after Phase 1, `pending_selection` is the only hard blocker).
//!
//! This module models that suspended continuation as **data**: a stack of
//! [`ResumeFrame`]s run inner→outer when a selection resolves. "Wrapping" a
//! callback becomes `Vec::push` (a frame), not a nested closure. Every capture
//! across the ~50 callback sites + the 7 recursive trampolines was verified to
//! be `Copy`/`Clone` (filters are `CompiledPredicate` data, the
//! `Arc<Mutex<Box<dyn FnOnce>>>` trampoline plumbing collapses into the
//! `MultiPickStep::then` frame), so the whole stack is `Clone`.
//!
//! Wiring this into `PendingSelection` (a coexistence `resume: Option<...>`
//! field run by `resolve_generic_selection` ahead of the legacy closure) and
//! porting `count_capped` onto it is the next step — the task-0.2 spike.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;
use crate::trigger_context::TriggerContext;

/// A paused continuation, as data. Frames run inner→outer on selection
/// resolution; this replaces the closure-nesting performed today by
/// `wrap_pending_selection_with_tail` and the `effect_queue` / `game_actions`
/// composition sites.
#[derive(Debug, Clone, Default)]
pub struct ResumeStack {
    /// Inner-to-outer: the last pushed frame runs first.
    pub frames: Vec<ResumeFrame>,
}

impl ResumeStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a continuation frame (the data analog of wrapping a callback).
    pub fn push(&mut self, frame: ResumeFrame) {
        self.frames.push(frame);
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Provenance carried by every frame (former closure captures of the `Copy`
/// handles the no-lifetimes selection callbacks already close over).
#[derive(Debug, Clone, Copy)]
pub struct ResumeProvenance {
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
}

/// One resumable continuation frame. Each variant corresponds to a former
/// `Box::new(move |game, action_id| …)` closure body; all captured fields are
/// `Copy`/`Clone` data.
#[derive(Debug, Clone)]
pub enum ResumeFrame {
    /// ~95% of DSL selection sites (Shapes A+B): bind the chosen value into a
    /// slot, run the inner step tail, then drain the outer tail. The
    /// `decline_aborts_clause` flag carries the optional-cost "By trashing X,
    /// do Y" semantics.
    RunTail {
        prov: ResumeProvenance,
        /// Binding slot the chosen value is written into (`bind_as`), if any.
        bind_as: Option<String>,
        /// Steps after the select within the same effect body.
        inner_tail: Arc<Vec<CompiledStep>>,
        /// The `dsl_outer_tail` continuation (already data on `Game` today).
        outer_tail: Option<Arc<Vec<CompiledStep>>>,
        bindings: Bindings,
        runtime: StepRuntime,
        trigger_context: Option<TriggerContext>,
        decline_aborts_clause: bool,
    },
    /// Recursive multi-pick accumulator — the `count_capped` / source-multi /
    /// dp-budget / play-cost-budget / reveal-bucket / permutation /
    /// partition trampolines. The former `Arc<Mutex<Option<Box<dyn
    /// FnOnce>>>>` threading collapses into plain `accum`/`candidates` data
    /// plus the `then` continuation frame.
    MultiPickStep {
        prov: ResumeProvenance,
        accum: Vec<CardHandle>,
        candidates: Vec<(u16, CardHandle)>,
        min: u8,
        max: u8,
        filter: Option<CompiledPredicate>,
        /// Runs once the pick count is satisfied (or candidates exhausted).
        then: Box<ResumeFrame>,
    },
}
