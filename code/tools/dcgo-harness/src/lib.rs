//! Host side of the DCGO automation harness: job submission, queue status,
//! and corpus triage. The DCGO client itself only reads and writes files in
//! the job directories — see
//! `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`.

// An `unreachable_pattern` in this crate is never a style nit -- it means a
// step form the scenario vocabulary accepts is silently NOT lowered, because a
// broader arm above it already swallowed the value. That is invisible at
// runtime (the broader arm just does the wrong thing) and invisible to the
// parser tests (the payload still PARSES). It shipped exactly that way once:
// `SelectPayload::Materials` sat below the catch-all `StepAction::Select(payload)`
// in `exam/adapter.rs`, so multi-pick [Assembly] material declarations were dead
// code for their whole lifetime while 270 harness tests stayed green -- and two
// exam clauses were reported as blocked on "missing vocabulary" that had in fact
// been written, but was unreachable. The compiler had been printing the warning
// the entire time. Make it impossible to walk past again.
#![deny(unreachable_patterns)]

pub mod build;
pub mod daemon;
pub mod exam;
pub mod job;
pub mod manifest;
pub mod pool;
pub mod queue;
pub mod triage;
pub mod watch;
