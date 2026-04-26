//! JSON Schema export — consumed by the VS Code YAML extension for
//! auto-complete + inline validation.

use schemars::schema_for;

use crate::spec::CardSpec;

pub fn export_json_schema() -> String {
    let schema = schema_for!(CardSpec);
    serde_json::to_string_pretty(&schema).expect("schema serialization must not fail")
}
