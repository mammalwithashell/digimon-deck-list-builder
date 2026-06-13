// Standalone YAML compile validator — bypasses digimon-engine's build.rs card
// scan (which panics on ANY broken WIP card in the shared cards/ tree).
// Usage: cargo run -p digimon-dsl --example validate_yaml -- <file.yaml> [...]
use std::fs;
fn main() {
    let mut bad = 0;
    for path in std::env::args().skip(1) {
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => { eprintln!("READ-ERR {path}: {e}"); bad += 1; continue; }
        };
        let spec: Result<digimon_dsl::CardSpec, _> = serde_yml::from_str(&text);
        match spec {
            Err(e) => { eprintln!("PARSE-FAIL {path}: {e}"); bad += 1; }
            Ok(s) => match digimon_dsl::compile::compile(&s) {
                Err(e) => { eprintln!("COMPILE-FAIL {path}: {e:?}"); bad += 1; }
                Ok(c) => println!("OK {path}  ({} clauses, {} alt_paths)", c.effects.len(), c.alt_paths.len()),
            },
        }
    }
    if bad > 0 { std::process::exit(1); }
}
