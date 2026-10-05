//! DSL linter CLI.
//!
//! Usage:
//!   dsl-lint <path> [--format human|json] [--strict] [--cross-check <cards.json>]
//!            [--printed <card_official.json>]
//!
//! `--printed` enables the security-icon check: `scope: security` clauses must
//! match a pink `{Security}` (face-up) effect on the printed card and
//! `when: on_security` clauses a blue `[Security]` effect
//! (`digimon_dsl::security_icon_lint`). It also enables the error
//! `spec-lint/SecurityAttackAuraDoubleCountsPrintedKeyword`: an unconditional
//! self `security_attack` aura on a card whose printed face carries an innate
//! `<Security A. ±N>` (the engine counts both; `digimon_dsl::spec_lint`).
//!
//! Always on: the warning `spec-lint/CappedMultiMinZeroWithoutOptionalZero`
//! (`select_count_capped_multi { min: 0 }` without `optional_zero`).
//!
//! Exit codes:
//!   0 — no diagnostics
//!   1 — errors found (or warnings, if --strict)
//!   2 — warnings only (non-strict)
//!   3 — usage error

use digimon_engine::dsl::loader;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::security_icon_lint::{check_security_icons, PrintedSection};
use digimon_engine::dsl::spec_lint::{
    check_capped_multi_min_zero, check_security_attack_self_aura,
};
use digimon_engine::dsl::validator::{validate, ValidationContext};
use digimon_engine::enums::Keyword;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Json,
}

#[derive(Debug)]
struct Args {
    path: PathBuf,
    format: Format,
    strict: bool,
    cross_check_path: Option<PathBuf>,
    printed_path: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut path: Option<PathBuf> = None;
    let mut format = Format::Human;
    let mut strict = false;
    let mut cross_check_path: Option<PathBuf> = None;
    let mut printed_path: Option<PathBuf> = None;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" => {
                let v = iter.next().ok_or("--format requires a value")?;
                format = match v.as_str() {
                    "human" => Format::Human,
                    "json" => Format::Json,
                    other => return Err(format!("unknown format: {other}")),
                };
            }
            "--strict" => strict = true,
            "--cross-check" => {
                let v = iter.next().ok_or("--cross-check requires a path")?;
                cross_check_path = Some(PathBuf::from(v));
            }
            "--printed" => {
                let v = iter.next().ok_or("--printed requires a path")?;
                printed_path = Some(PathBuf::from(v));
            }
            "-h" | "--help" => {
                println!("Usage: dsl-lint <path> [--format human|json] [--strict] [--cross-check <cards.json>] [--printed <card_official.json>]");
                std::process::exit(0);
            }
            s if s.starts_with("--") => return Err(format!("unknown flag: {s}")),
            _ => {
                if path.is_some() {
                    return Err("multiple path arguments".into());
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }
    let path = path.ok_or("missing <path> argument")?;
    Ok(Args {
        path,
        format,
        strict,
        cross_check_path,
        printed_path,
    })
}

#[derive(serde::Serialize, Debug)]
struct Diagnostic {
    file: String,
    severity: Severity,
    path: String,
    message: String,
}

#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Error,
    Warning,
}

type PrintedDb = std::collections::HashMap<String, Vec<PrintedSection>>;

/// Load `card_official.json` (`{cards: {<id>: {text_sections: [{label, text}]}}}`).
fn load_printed(path: &Path) -> Result<PrintedDb, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let cards = v["cards"].as_object().ok_or("no `cards` object")?;
    Ok(cards
        .iter()
        .map(|(id, c)| {
            let secs = c["text_sections"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|s| PrintedSection {
                            label: s["label"].as_str().unwrap_or_default().to_string(),
                            text: s["text"].as_str().unwrap_or_default().to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            (id.clone(), secs)
        })
        .collect())
}

/// Whether the printed FACE text (the official "Effect" section — what the
/// engine's `face_keywords` parses, not the inherited / security sections)
/// carries an innate `<Security A. ±N>`, judged by the engine's own parser.
fn face_prints_security_attack(sections: &[PrintedSection]) -> bool {
    sections
        .iter()
        .filter(|s| s.label.eq_ignore_ascii_case("Effect"))
        .any(|s| {
            digimon_engine::card_data::parse_printed_keywords(&s.text, "", "")
                .iter()
                .any(|k| {
                    matches!(
                        k,
                        Keyword::SecurityAttackPlus(_) | Keyword::SecurityAttackMinus(_)
                    )
                })
        })
}

fn lint_file(
    path: &Path,
    adapter: Option<&dyn digimon_engine::dsl::loader::CardDataDb>,
    printed: Option<&PrintedDb>,
    diags: &mut Vec<Diagnostic>,
) {
    let file = path.display().to_string();
    let spec = match loader::load_file(path) {
        Ok(s) => s,
        Err(e) => {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: String::new(),
                message: format!("{e}"),
            });
            return;
        }
    };

    let registry = StubRegistry::with([
        "bt13_007_royal_knight_cost_reduction",
        "bt10_111_arm_digixros_wildcard_for_turn",
        "ad1_025_on_play_process",
    ]);
    let ctx = ValidationContext {
        raw_rust: &registry,
    };
    if let Err(errs) = validate(&spec, &ctx) {
        for e in errs {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: e.path,
                message: e.message,
            });
        }
    }

    if let Some(sections) = printed.and_then(|p| p.get(&spec.card)) {
        for f in check_security_icons(&spec, sections) {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: f.path,
                message: format!("security-icon/{:?}: {}", f.rule, f.message),
            });
        }
    }

    if let Some(sections) = printed.and_then(|p| p.get(&spec.card)) {
        if face_prints_security_attack(sections) {
            for f in check_security_attack_self_aura(&spec, true) {
                diags.push(Diagnostic {
                    file: file.clone(),
                    severity: Severity::Error,
                    path: f.path,
                    message: format!("spec-lint/{:?}: {}", f.rule, f.message),
                });
            }
        }
    }

    if let Ok(raw) = std::fs::read_to_string(path) {
        for f in check_capped_multi_min_zero(&raw) {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Warning,
                path: f.path,
                message: format!("spec-lint/{:?}: {}", f.rule, f.message),
            });
        }
    }

    if let Some(db) = adapter {
        if let Err(e) = digimon_engine::dsl::loader::cross_check(&spec, db) {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: e.path,
                message: e.message,
            });
        }
    }
}

fn walk_yaml(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    if !path.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(iter) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in iter.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn main() -> ExitCode {
    // Windows defaults the main thread stack to 1 MiB, which is too small
    // for the recursive YAML deserializer that parses card specs (the same
    // workaround digimon-engine's build.rs uses). Run the real work on a
    // worker thread with a larger stack.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(real_main)
        .expect("spawn lint worker")
        .join()
        .expect("lint worker panicked")
}

fn real_main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("dsl-lint: {e}");
            eprintln!("try --help");
            return ExitCode::from(3);
        }
    };

    let adapter_opt = match &args.cross_check_path {
        Some(p) => Some(
            digimon_engine::dsl_bridge::RealCardDataAdapter::from_path(p).unwrap_or_else(|e| {
                eprintln!(
                    "dsl-lint: failed to load cards.json from {}: {e}",
                    p.display()
                );
                std::process::exit(3);
            }),
        ),
        None => None,
    };
    let adapter_dyn: Option<&dyn digimon_engine::dsl::loader::CardDataDb> = adapter_opt
        .as_ref()
        .map(|a| a as &dyn digimon_engine::dsl::loader::CardDataDb);

    let printed = args.printed_path.as_ref().map(|p| {
        load_printed(p).unwrap_or_else(|e| {
            eprintln!("dsl-lint: failed to load {}: {e}", p.display());
            std::process::exit(3);
        })
    });

    let mut diags = Vec::new();
    for file in walk_yaml(&args.path) {
        lint_file(&file, adapter_dyn, printed.as_ref(), &mut diags);
    }

    match args.format {
        Format::Human => {
            for d in &diags {
                println!(
                    "{}:1:1: {}: [{}] {}",
                    d.file,
                    match d.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                    },
                    d.path,
                    d.message
                );
            }
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&diags).unwrap());
        }
    }

    let has_errors = diags.iter().any(|d| d.severity == Severity::Error);
    let has_warnings = diags.iter().any(|d| d.severity == Severity::Warning);
    if has_errors || (args.strict && has_warnings) {
        ExitCode::from(1)
    } else if has_warnings {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(label: &str, text: &str) -> PrintedSection {
        PrintedSection {
            label: label.into(),
            text: text.into(),
        }
    }

    #[test]
    fn innate_face_security_attack_is_detected() {
        assert!(face_prints_security_attack(&[sec(
            "Effect",
            "＜Security A. +1＞ ＜Reboot＞ ＜Blocker＞ [On Play] Delete 1 Digimon."
        )]));
        assert!(face_prints_security_attack(&[sec(
            "Effect",
            "＜Security A. -1＞ (This Digimon checks 1 fewer security card.)"
        )]));
    }

    #[test]
    fn granted_inherited_or_absent_security_attack_is_not_face() {
        // Granted by prose ("gains") — a conditional DSL effect, not innate.
        assert!(!face_prints_security_attack(&[sec(
            "Effect",
            "[Your Turn] This Digimon gains ＜Security A. +1＞."
        )]));
        // Printed only in the inherited section.
        assert!(!face_prints_security_attack(&[
            sec("Effect", "[On Play] Draw 1."),
            sec("Inherited Effect", "＜Security A. +1＞"),
        ]));
        assert!(!face_prints_security_attack(&[]));
    }

    /// End to end over a temp YAML: the real lint pass reports the aura as an
    /// error when the printed face carries the keyword, and nothing otherwise.
    #[test]
    fn lint_file_flags_self_sa_aura_only_with_printed_keyword() {
        let dir = std::env::temp_dir().join(format!("dsl-lint-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("TEST-SA.yaml");
        std::fs::write(
            &file,
            "card: TEST-SA\nname: T\nkind: digimon\nlevel: 6\ncolor: [red]\ncost: 10\n\
             dp: 12000\neffects:\n  - kind: aura\n    target: {}\n    security_attack: 1\n",
        )
        .unwrap();

        let mut printed = PrintedDb::new();
        printed.insert(
            "TEST-SA".into(),
            vec![sec(
                "Effect",
                "＜Security A. +1＞ (This Digimon checks 1 additional security card.)",
            )],
        );
        let mut diags = Vec::new();
        lint_file(&file, None, Some(&printed), &mut diags);
        assert!(
            diags.iter().any(|d| d.severity == Severity::Error
                && d.message
                    .contains("SecurityAttackAuraDoubleCountsPrintedKeyword")),
            "{diags:?}"
        );

        printed.insert("TEST-SA".into(), vec![sec("Effect", "[On Play] Draw 1.")]);
        let mut diags = Vec::new();
        lint_file(&file, None, Some(&printed), &mut diags);
        assert!(diags.is_empty(), "{diags:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lint_file_warns_on_capped_multi_min_zero() {
        let dir = std::env::temp_dir().join(format!("dsl-lint-test-cm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("TEST-CM.yaml");
        std::fs::write(
            &file,
            "card: TEST-CM\nname: T\nkind: digimon\nlevel: 6\ncolor: [red]\ncost: 10\n\
             dp: 12000\neffects:\n  - when: on_play\n    process:\n      \
             - select_count_capped_multi: { of: you, zone: trash, max: 3, min: 0, filter: {}, prompt: p }\n",
        )
        .unwrap();
        let mut diags = Vec::new();
        lint_file(&file, None, None, &mut diags);
        assert!(
            diags.iter().any(|d| d.severity == Severity::Warning
                && d.message.contains("CappedMultiMinZeroWithoutOptionalZero")),
            "{diags:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
