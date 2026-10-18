//! DSL linter CLI.
//!
//! Usage:
//!   dsl-lint <path> [--format human|json] [--strict] [--cross-check <cards.json>]
//!            [--printed <card_official.json>]
//!
//! `--printed` enables the security-icon check: `scope: security` clauses must
//! match a pink `{Security}` (face-up) effect on the printed card and
//! `when: on_security` clauses a blue `[Security]` effect
//! (`digimon_dsl::security_icon_lint`).
//!
//! Exit codes:
//!   0 — no diagnostics
//!   1 — errors found (or warnings, if --strict)
//!   2 — warnings only (non-strict)
//!   3 — usage error

use digimon_engine::dsl::loader;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::security_icon_lint::{check_security_icons, PrintedSection};
use digimon_engine::dsl::validator::{validate, ValidationContext};
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

    for f in check_carrier_name_stack_scan(&spec) {
        diags.push(Diagnostic {
            file: file.clone(),
            severity: Severity::Warning,
            path: f.path,
            message: f.message,
        });
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

/// Cards confirmed to use `self_digivolution_contains_name` for a printed
/// "[X] in its name or digivolution cards" clause (the one shape that reads
/// the top card AND the digivolution cards), as `(card id, printed clause)`.
/// Empty: no production card prints that shape today.
const SELF_DIGIVOLUTION_CONTAINS_NAME_ALLOWLIST: &[(&str, &str)] = &[];

#[derive(Debug, PartialEq, Eq)]
struct LintFinding {
    path: String,
    message: String,
}

/// G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS: `self_digivolution_contains_name`
/// scans the top card AND every digivolution card. A Digimon's name is its top
/// card's (§4-7-7 ruling; Q3225; DCGO `TopCard`), and digivolution cards are
/// read only when the text references them (§4-8-2). So:
/// - "this Digimon's name / has [X] in its name / is [X]" -> `source_name_contains`;
/// - "[X] is in this Digimon's digivolution cards" -> `self_digivolution_sources_contain_name`;
/// - only "[X] in its name or digivolution cards" may keep the stack scan.
///
/// The linter cannot tell these apart reliably from the summary, so every use
/// outside `SELF_DIGIVOLUTION_CONTAINS_NAME_ALLOWLIST` is a warning that names
/// the clause summary for the reviewer.
fn check_carrier_name_stack_scan(spec: &digimon_engine::dsl::spec::CardSpec) -> Vec<LintFinding> {
    if SELF_DIGIVOLUTION_CONTAINS_NAME_ALLOWLIST
        .iter()
        .any(|(id, _)| *id == spec.card)
    {
        return Vec::new();
    }
    let Ok(value) = serde_json::to_value(spec) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk_for_stack_name_scan(&value, String::new(), None, &mut out);
    out
}

fn walk_for_stack_name_scan(
    v: &serde_json::Value,
    path: String,
    summary: Option<&str>,
    out: &mut Vec<LintFinding>,
) {
    match v {
        serde_json::Value::Object(map) => {
            let summary = map
                .get("summary")
                .and_then(|s| s.as_str())
                .or(summary);
            for (k, child) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                if k == "self_digivolution_contains_name" {
                    out.push(LintFinding {
                        path: child_path.clone(),
                        message: format!(
                            concat!(
                                "carrier-name/stack-scan: `self_digivolution_contains_name: {}` ",
                                "reads the top card AND every digivolution card. Use ",
                                "`source_name_contains` for \"this Digimon's name / is [X]\" and ",
                                "`self_digivolution_sources_contain_name` for \"[X] in this ",
                                "Digimon's digivolution cards\" (§4-7-7, §4-8-2; ",
                                "G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS). Only an \"[X] in ",
                                "its name or digivolution cards\" clause may keep it: add the card ",
                                "to SELF_DIGIVOLUTION_CONTAINS_NAME_ALLOWLIST with the printed ",
                                "clause. Clause summary: {}"
                            ),
                            child,
                            summary.unwrap_or("<none>")
                        ),
                    });
                }
                walk_for_stack_name_scan(child, child_path, summary, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                walk_for_stack_name_scan(child, format!("{path}[{i}]"), summary, out);
            }
        }
        _ => {}
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

    /// Parse a test card through the same `loader::load_file` the CLI uses.
    fn spec(yaml: &str) -> digimon_engine::dsl::spec::CardSpec {
        let dir = std::env::temp_dir().join(format!(
            "dsl_lint_carrier_name_{}_{}",
            std::process::id(),
            yaml.len()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("card.yaml");
        std::fs::write(&path, yaml).unwrap();
        let s = loader::load_file(&path).expect("test card loads");
        let _ = std::fs::remove_dir_all(&dir);
        s
    }

    #[test]
    fn stack_name_scan_is_flagged_with_its_clause_summary() {
        let s = spec(
            r#"
card: LINT_PROBE
name: Probemon
kind: digimon
level: 3
color: [red]
cost: 2
dp: 1000
effects:
  - scope: inherited
    kind: aura
    active_when:
      all_of:
        - all_turns: true
        - self_digivolution_contains_name: "Greymon"
    target: {}
    dp_modifier: 1000
    summary: "[All Turns] While this Digimon has [Greymon] in its name, it gets +1000 DP."
"#,
        );
        let f = check_carrier_name_stack_scan(&s);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].path.ends_with("self_digivolution_contains_name"), "{}", f[0].path);
        assert!(f[0].message.contains("has [Greymon] in its name"), "{}", f[0].message);
    }

    #[test]
    fn top_card_and_sources_only_predicates_are_not_flagged() {
        let s = spec(
            r#"
card: LINT_PROBE_OK
name: Probemon
kind: digimon
level: 3
color: [red]
cost: 2
dp: 1000
effects:
  - kind: aura
    active_when:
      any_of:
        - source_name_contains: "Greymon"
        - self_digivolution_sources_contain_name: "Greymon"
    target: {}
    dp_modifier: 1000
    summary: "probe"
"#,
        );
        assert!(check_carrier_name_stack_scan(&s).is_empty());
    }
}
