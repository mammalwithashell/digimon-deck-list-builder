//! Compile every YAML under digimon-engine/cards/ (curated `_examples/`
//! plus per-set production subdirectories such as `bt21/`, `ex11/`, ...)
//! into $OUT_DIR/cards.pack at build time. The resulting blob is
//! `include_bytes!`-ed by `src/dsl_registry.rs` to give the desktop
//! binary instant access to compiled cards.
//!
//! Phase 1c: scans the entire `cards/` tree. Each subdirectory is
//! treated as a card source; specs across directories are merged into
//! a single `CardRegistry`. Duplicate `card:` IDs across directories
//! cause a build failure.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cards");
    println!("cargo:rerun-if-changed=build.rs");

    let cards_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR set by cargo"));
    let pack_path = out_dir.join("cards.pack");

    // Handle the case where cards/ doesn't exist yet (fresh clone): write an
    // empty pack so downstream `include_bytes!` doesn't fail.
    if !cards_root.exists() {
        let empty = digimon_dsl::CardPack::new("core", vec![]);
        let bytes = empty.to_bytes().expect("serialize empty pack");
        std::fs::write(&pack_path, &bytes).expect("write empty cards.pack");
        return;
    }

    let mut all_specs: Vec<digimon_dsl::CardSpec> = Vec::new();
    let mut all_parse_errors: Vec<digimon_dsl::DslError> = Vec::new();

    // Iterate every direct subdirectory of cards/ (e.g. _examples, bt21, ex11).
    let entries = std::fs::read_dir(&cards_root)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", cards_root.display()));
    let mut subdirs: Vec<PathBuf> = entries
        .filter_map(|res| res.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    // Stable iteration for reproducible builds.
    subdirs.sort();

    for dir in subdirs {
        let (specs, errors) = digimon_dsl::loader::load_dir_ok(&dir);
        all_specs.extend(specs);
        all_parse_errors.extend(errors);
    }

    if !all_parse_errors.is_empty() {
        for e in &all_parse_errors {
            println!("cargo:warning=dsl parse error: {e}");
        }
        panic!("dsl parse errors in cards/ — see warnings above");
    }

    let registry = match digimon_dsl::CardRegistry::from_specs("core", &all_specs) {
        Ok(r) => r,
        Err(errs) => {
            for e in &errs {
                println!("cargo:warning=dsl compile error: {e}");
            }
            panic!("dsl compile errors in cards/ — see warnings above");
        }
    };

    let pack = digimon_dsl::CardPack {
        manifest: registry.manifest.clone(),
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    let bytes = pack.to_bytes().expect("bincode serialize cards.pack");
    std::fs::write(&pack_path, &bytes).expect("write cards.pack");
}
