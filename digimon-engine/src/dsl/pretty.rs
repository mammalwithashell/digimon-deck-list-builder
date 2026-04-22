//! Canonical YAML pretty-printer for `CardSpec`.
//!
//! Idempotent: `format_spec(parse(format_spec(spec))) == format_spec(spec)`.
//! Relies on serde_yml's default emitter + struct field declaration order
//! for canonical output. `#[serde(skip_serializing_if = "...")]` on
//! optional fields keeps output minimal.

use crate::dsl::spec::CardSpec;

pub fn format_spec(spec: &CardSpec) -> String {
    serde_yml::to_string(spec).expect("CardSpec serialization must not fail")
}
