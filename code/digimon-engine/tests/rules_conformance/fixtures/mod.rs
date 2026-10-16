//! Synthetic YAML DSL fixture cards for the rules_conformance chapters.
//!
//! Each `*.yaml` file here holds several cards, one YAML document per card,
//! separated by a line that is exactly `---`. `DebugRunnerBuilder::from_dsl_yaml`
//! compiles ONE card, so [`load`] splits a file and registers every document.
//!
//! `common.yaml` holds cards reused across chapters; each chapter adds its own
//! file. A chapter loads `COMMON` plus its own file via [`builder`].

#![allow(dead_code)] // each chapter uses a subset

use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};

/// Cards shared by several chapters (`WA_MEM`, `WA_DRAW`, `OD_MEM`,
/// `OPP_WA_OBS`, `OP_PLAY_LV3`, `VANILLA_LV3`).
pub const COMMON: &str = include_str!("common.yaml");

/// §15-4 chapter cards (`WA_ALLIANCE`, `WA_HYBRID_DIGI`, `WD_DEL_RET`, ...).
pub const CH15_04: &str = include_str!("ch15_04.yaml");

/// Split a multi-document YAML file into its documents. A document that holds
/// only comments / blank lines is dropped.
pub fn documents(yaml: &str) -> Vec<String> {
    let mut docs = Vec::new();
    let mut current = String::new();
    for line in yaml.lines() {
        if line.trim_end() == "---" {
            docs.push(std::mem::take(&mut current));
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    docs.push(current);
    docs.into_iter()
        .filter(|d| {
            d.lines()
                .any(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        })
        .collect()
}

/// Register every card of `yaml` on `builder`. Panics with the card's compile
/// error, so a broken fixture fails loudly rather than silently missing.
pub fn load(mut builder: DebugRunnerBuilder, yaml: &str) -> DebugRunnerBuilder {
    for doc in documents(yaml) {
        builder = builder
            .from_dsl_yaml(&doc)
            .unwrap_or_else(|e| panic!("fixture card failed to compile: {e}\n---\n{doc}"));
    }
    builder
}

/// A builder with `COMMON` plus each of `chapter_files` loaded.
pub fn builder(chapter_files: &[&str]) -> DebugRunnerBuilder {
    let mut b = load(DebugRunner::builder(), COMMON);
    for file in chapter_files {
        b = load(b, file);
    }
    b
}
