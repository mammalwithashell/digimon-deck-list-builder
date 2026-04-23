//! DSL test binary. Submodules (one per task group) contribute the `#[test]`
//! functions. See `digimon-engine/Cargo.toml` for the `[[test]]` entry.

mod parse_minimal;
mod parse_alt_paths;
mod parse_identity;
mod parse_clauses;
mod parse_steps;
mod parse_predicates;
mod parse_formulas;
mod parse_declarative;
mod i18n_scaffolding;
mod loader;
mod cross_check;
mod validator;
mod raw_rust_registry;
mod pretty;
mod roundtrip;
mod schema_export;
mod phase0_exit;
mod real_cards_json;
mod embedded_registry;
