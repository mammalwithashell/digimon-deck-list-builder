//! Card-scripting DSL (Phase 0 — parse + validate only).
//!
//! See `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` for the
//! spec. This module provides:
//!
//! - [`spec::CardSpec`] — the authored YAML card definition.
//! - [`loader`] — YAML file/dir parsing + `cards.json` cross-check.
//! - [`validator`] — semantic validation (timings, enum names, predicate
//!   type-checks, raw_rust references).
//! - [`pretty`] — canonical YAML pretty-printer (round-trip stable).
//! - [`schema`] — JSON Schema export for IDE tooling.
//! - [`raw_rust_registry::RawRustRegistry`] — trait the validator uses to
//!   resolve `raw_rust:` fn-name references. A stub impl is provided for
//!   tests; the real registry lands in Phase 4.

pub mod alt_path;
pub mod compiled;
pub mod clause;
pub mod common;
pub mod errors;
pub mod formula;
pub mod identity;
pub mod loader;
pub mod predicate;
pub mod pretty;
pub mod raw_rust_registry;
pub mod schema;
pub mod spec;
pub mod step;
pub mod validator;

pub use common::PlayerRef;
pub use errors::{DslError, ValidationError};
pub use spec::CardSpec;
