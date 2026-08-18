//! Host side of the DCGO automation harness: job submission, queue status,
//! and corpus triage. The DCGO client itself only reads and writes files in
//! the job directories — see
//! `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`.

pub mod job;
pub mod pool;
pub mod queue;
