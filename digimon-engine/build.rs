//! Compile digimon-engine/cards/_examples/*.yaml into $OUT_DIR/cards.pack
//! at build time. The resulting blob is `include_bytes!`-ed by
//! `src/dsl_registry.rs` to give the desktop binary instant access
//! to compiled cards.
//!
//! Phase 1b: operates only on the _examples directory. Phase 1c will
//! point this at digimon-engine/cards/ (the real pack root).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cards/_examples");
    println!("cargo:rerun-if-changed=build.rs");

    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR set by cargo"));
    let pack_path = out_dir.join("cards.pack");

    // Handle the case where _examples doesn't exist yet (e.g. fresh clone
    // before any fixtures land): write an empty pack.
    if !examples_dir.exists() {
        let empty = digimon_dsl::CardPack::new("core", vec![]);
        let bytes = empty.to_bytes().expect("serialize empty pack");
        std::fs::write(&pack_path, &bytes).expect("write empty cards.pack");
        return;
    }

    let (specs, parse_errors) = digimon_dsl::loader::load_dir_ok(&examples_dir);
    if !parse_errors.is_empty() {
        for e in &parse_errors {
            println!("cargo:warning=dsl parse error: {e}");
        }
        panic!("dsl parse errors in cards/_examples/ — see warnings above");
    }

    let registry = match digimon_dsl::CardRegistry::from_specs("core", &specs) {
        Ok(r) => r,
        Err(errs) => {
            for e in &errs {
                println!("cargo:warning=dsl compile error: {e}");
            }
            panic!("dsl compile errors in cards/_examples/ — see warnings above");
        }
    };

    let pack = digimon_dsl::CardPack {
        manifest: registry.manifest.clone(),
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    let bytes = pack.to_bytes().expect("bincode serialize cards.pack");
    std::fs::write(&pack_path, &bytes).expect("write cards.pack");
}
