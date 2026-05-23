//! Active-deletion-batch state machine.
//!
//! Carries the per-call working set for `Game::delete_permanents_batch`
//! through its DCGO-modeled 10-step flow. The single `Game::active_deletion_batch`
//! slot holds at most one batch at a time; nested calls inside an OnDeletion
//! handler are bounded by the existing depth guard (matched on
//! `DeletionBatch::depth`).
//!
//! See `openspec/changes/align-deletion-with-dcgo-model/design.md` D2/D3
//! for the design rationale, and `specs/permanent-deletion-semantics/spec.md`
//! for the requirement contracts this state implements.
//!
//! ## Stage flow
//!
//! ```text
//! Filtering
//!     └── (filter null handles; mark kill_list)
//! Stage1ReplacementDrain
//!     └── enqueue WhenWouldLeaveBattleArea for each kill_list member
//!     └── drain (deferred-drain scope absorbs nested parks)
//!     └── re-filter cancelled
//! Stage2ReplacementDrain
//!     └── enqueue WhenWouldBeDeleted for each survivor
//!     └── drain
//!     └── re-filter cancelled
//! Snapshotting
//!     └── build DeletedObjectSnapshot for each survivor
//! Trashing
//!     └── linked-card cascade per survivor
//!     └── trash top + sources
//! OnDeletionDrain
//!     └── enqueue OnDeletion per survivor (snapshot in trigger context)
//!     └── drain (handler parks unwind through pending_selection;
//!         active_deletion_batch state persists across the park)
//! OnAnyDeletionDrain
//!     └── enqueue OnAnyDeletion + OnLeaveField per survivor
//!     └── drain
//!     └── batch slot cleared
//! ```

use crate::card_source::CardHandle;
use crate::permanent::PermanentHandle;
use crate::replacement::ReplacementCause;
use crate::trigger_context::DeletedObjectSnapshot;

/// In-flight deletion batch state. Owned by `Game::active_deletion_batch`
/// for the duration of a top-level `delete_permanents_batch` call.
#[derive(Debug, Clone)]
pub struct DeletionBatch {
    /// The currently-targeted set of permanent handles. Mutated during the
    /// replacement stages: cancelled handles are removed; substitutes
    /// (e.g. `<Decoy>`) are appended.
    pub kill_list: Vec<PermanentHandle>,

    /// Current stage of the batch flow.
    pub stage: BatchStage,

    /// Pre-removal snapshots captured during the `Snapshotting` stage, one
    /// per surviving permanent (parallel to `kill_list` at that moment).
    pub snapshots: Vec<DeletedObjectSnapshot>,

    /// Permanents that were trashed in this batch.
    pub completed: Vec<PermanentHandle>,

    /// Permanents whose deletion was cancelled (replacement returned
    /// `Cancelled` / `CustomHandled`, or was redirected to a non-trash zone).
    pub cancelled: Vec<PermanentHandle>,

    /// Permanents added to the active batch via mid-batch substitution
    /// (`<Decoy>`-style "delete this Digimon instead"). Tracked separately
    /// so observers and tests can see which deletions are original vs added.
    pub substituted_in: Vec<PermanentHandle>,

    /// Top cards captured at the start of the trash step for each surviving
    /// permanent. Used to thread `top_card` into OnDeletion / OnAnyDeletion
    /// trigger contexts after the carrier is gone from `battle_area`.
    pub top_cards: Vec<Option<CardHandle>>,

    /// Original deletion cause for the batch (e.g. Battle, OwnEffect,
    /// OpponentEffect). Carried into each permanent's OnDeletion drain via
    /// `Game::current_deletion_cause` save/restore around the drain.
    pub cause: ReplacementCause,

    /// Recursion depth guard. Incremented per substitute-append to bound
    /// pathological Decoy chains; matches the previous depth-guarded
    /// recursion in `combat::commit_permanent_deletion`'s
    /// `Substituted(ReplacementSubject::Permanent(_))` arm.
    pub depth: u8,
}

impl DeletionBatch {
    /// Construct a batch from a kill list, in the `Filtering` stage.
    pub fn new(kill_list: Vec<PermanentHandle>, cause: ReplacementCause) -> Self {
        Self {
            kill_list,
            stage: BatchStage::Filtering,
            snapshots: Vec::new(),
            completed: Vec::new(),
            cancelled: Vec::new(),
            substituted_in: Vec::new(),
            top_cards: Vec::new(),
            cause,
            depth: 0,
        }
    }
}

/// Stage marker for `DeletionBatch`. The flow is strictly forward — no
/// stage transitions backwards. Replacement-stage parks (selection
/// install) unwind back through `delete_permanents_batch` and re-enter at
/// the same stage when resolution resumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStage {
    /// Initial — drop null handles, mark `kill_list` survivors.
    Filtering,
    /// Stage 1 cut-in: WhenWouldLeaveBattleArea enqueue + drain.
    Stage1ReplacementDrain,
    /// Stage 2 cut-in: WhenWouldBeDeleted enqueue + drain.
    Stage2ReplacementDrain,
    /// Capture `DeletedObjectSnapshot` for each survivor (pre-trash).
    Snapshotting,
    /// Move top + sources to trash; cascade linked cards.
    Trashing,
    /// Enqueue OnDeletion per survivor; drain.
    OnDeletionDrain,
    /// Enqueue OnAnyDeletion + OnLeaveField; drain. Batch is closed
    /// when this stage completes.
    OnAnyDeletionDrain,
}

/// Returned by `Game::delete_permanents_batch` to let callers introspect
/// which handles actually trashed vs. were redirected. Existing callers
/// (battle resolution, DSL `DeleteBoundPermanents`, cost-trash) generally
/// don't inspect this — it's provided for tests and future scripts.
#[derive(Debug, Clone, Default)]
pub struct DeletionBatchOutcome {
    pub completed: Vec<PermanentHandle>,
    pub cancelled: Vec<PermanentHandle>,
    pub substituted_in: Vec<PermanentHandle>,
}
