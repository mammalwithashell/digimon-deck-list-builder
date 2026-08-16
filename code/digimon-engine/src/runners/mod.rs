pub mod headless;
pub mod replay;
pub mod selection_resolve;

pub use headless::HeadlessRunner;
pub use replay::{
    DcgoAdapter, Divergence, DivergenceKind, DivergenceReport, NativeAdapter, RecordingSource,
    ReplayError, ReplayRunner, ReplaySession, ReplayStepResult, StepPolicy, StepSpec,
    VerificationReplayAdapter, VerificationReplayCheck, VerificationReplayDivergence,
    VerificationReplayReport,
};
