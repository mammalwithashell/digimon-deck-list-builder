//! Thin binary wrapper around [`action_space_export::build`].
//!
//! Pretty-prints the action-space JSON descriptor to stdout. Downstream
//! emitters (e.g. `emit_csharp.py`) pipe stdout to a file or to a comparison
//! step. See the library crate docs for the full rationale.

fn main() {
    let v = action_space_export::build();
    println!(
        "{}",
        serde_json::to_string_pretty(&v).expect("serialize action space JSON")
    );
}
