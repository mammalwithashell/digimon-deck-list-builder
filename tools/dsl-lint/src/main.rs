//! DSL linter CLI.
//!
//! Usage:
//!   dsl-lint <path> [--format human|json] [--strict]
//!
//! Exit codes:
//!   0 — no diagnostics
//!   1 — errors found (or warnings, if --strict)
//!   2 — warnings only (non-strict)
//!   3 — usage error

use digimon_engine::dsl::loader;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
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
}

fn parse_args() -> Result<Args, String> {
    let mut path: Option<PathBuf> = None;
    let mut format = Format::Human;
    let mut strict = false;
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
            "-h" | "--help" => {
                println!("Usage: dsl-lint <path> [--format human|json] [--strict]");
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
    Ok(Args { path, format, strict })
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

fn lint_file(path: &Path, diags: &mut Vec<Diagnostic>) {
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
    let ctx = ValidationContext { raw_rust: &registry };
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
        let Ok(iter) = std::fs::read_dir(&d) else { continue };
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
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("dsl-lint: {e}");
            eprintln!("try --help");
            return ExitCode::from(3);
        }
    };

    let mut diags = Vec::new();
    for file in walk_yaml(&args.path) {
        lint_file(&file, &mut diags);
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
