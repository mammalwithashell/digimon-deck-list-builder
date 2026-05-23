//! Behavioral tests for the batched deletion flow introduced in the
//! `align-deletion-with-dcgo-model` change.
//!
//! Covers the DCGO-modeled
//! `delete_permanents_batch` semantics: trash-before-OnDeletion-drain,
//! snapshot-threaded trigger contexts, and the multi-permanent regression
//! cases — most importantly the AoE-deletes-two-`<Save>` panic that this
//! change closes (`G-DELETION-RESUME-NESTED`).

mod aoe_save_park;
mod mutual_destruction;
