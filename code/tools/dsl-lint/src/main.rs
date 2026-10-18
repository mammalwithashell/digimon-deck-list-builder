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

    lint_observer_event_conditions(&spec, &file, diags);

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

/// True for a timing whose effect triggers on an EVENT the clause qualifies
/// ("When one of your Digimon with 4000 DP or less is played", "When any of
/// your [TS] trait Digimon win a battle"), as opposed to the card's own
/// timings ([On Play], [When Digivolving], [On Deletion], ...), the phase
/// timings and activation-type timings, whose trigger is fixed.
fn is_observer_timing(t: digimon_engine::dsl::clause::Timing) -> bool {
    use digimon_engine::dsl::clause::Timing as T;
    !matches!(
        t,
        T::OnPlay
            | T::WhenDigivolving
            | T::WhenAttacking
            | T::OnAttack
            | T::EndOfAttack
            | T::EndOfBattle
            | T::OnBlock
            | T::OnDeletion
            | T::OnSecurity
            | T::Main
            | T::MainFromHand
            | T::MainOnField
            | T::MainFromTrash
            | T::Counter
            | T::StartOfYourTurn
            | T::StartOfOpponentsTurn
            | T::StartOfYourMainPhase
            | T::StartOfOpponentsMainPhase
            | T::EndOfYourTurn
            | T::EndOfOpponentsTurn
            | T::EndOfYourNextTurn
            | T::EndOfOpponentsNextTurn
            | T::UntilNextUnsuspend
            | T::BeforePayCost
            | T::BeforePayCostObserve
            | T::Delayed
            | T::WhenLinked
            | T::WhenCardLinkedToThis
            | T::WhenWouldLinkToThis
    )
}

/// Warn when an observer clause's `condition` / `active_when` reads the
/// event (`event_*`): `condition` / `active_when` are PROCESSING conditions,
/// read when the effect activates (§15-8-3-9-1 / §15-8-3-9-3), while a
/// qualifier of the event ("When one of your Digimon **with 4000 DP or
/// less** is played") is the trigger qualifier, decided when the event
/// triggers the effect (§15-8-3-1, §15-8-3-4) and authored in `trigger_if:`.
/// A genuine processing condition on the event ("..., if that Digimon is
/// level 4 or higher, ...") stays where it is; the warning asks the author to
/// decide from the printed text.
/// G-ENGINE-OBSERVER-TRIGGER-QUALIFIER-NOT-CHECKED-AT-TRIGGER.
fn lint_observer_event_conditions(
    spec: &digimon_engine::dsl::spec::CardSpec,
    file: &str,
    diags: &mut Vec<Diagnostic>,
) {
    use digimon_engine::dsl::clause::TimingSet;
    fn event_keys(value: &serde_json::Value, out: &mut Vec<String>) {
        let Some(map) = value.as_object() else {
            return;
        };
        for (key, v) in map {
            let absent = v.is_null() || v.as_array().is_some_and(|a| a.is_empty());
            if absent {
                continue;
            }
            match key.as_str() {
                "all_of" | "any_of" | "none_of" => {
                    for p in v.as_array().into_iter().flatten() {
                        event_keys(p, out);
                    }
                }
                "not" => event_keys(v, out),
                k if k.starts_with("event_") && !out.iter().any(|o| o == k) => {
                    out.push(key.clone())
                }
                _ => {}
            }
        }
    }
    for (i, clause) in spec.effects.iter().enumerate() {
        let Some(t) = clause.as_triggered() else {
            continue;
        };
        let observer = match &t.when {
            TimingSet::Single(w) => is_observer_timing(*w),
            TimingSet::Multi(ws) => ws.iter().any(|w| is_observer_timing(*w)),
        };
        if !observer {
            continue;
        }
        for (field, pred) in [("active_when", &t.active_when), ("condition", &t.condition)] {
            let Some(pred) = pred else {
                continue;
            };
            let mut keys = Vec::new();
            if let Ok(value) = serde_json::to_value(pred) {
                event_keys(&value, &mut keys);
            }
            for key in keys {
                diags.push(Diagnostic {
                    file: file.to_string(),
                    severity: Severity::Warning,
                    path: format!("effects[{i}].{field}"),
                    message: format!(
                        "observer clause's {field} reads `{key}`: is this a trigger qualifier? use \
                         `trigger_if:` -- a qualifier of the event (\"When one of your Digimon with \
                         4000 DP or less is played\") is decided when the event triggers the effect \
                         (§15-8-3-1, §15-8-3-4); `{field}` is a processing condition read at \
                         activation (\"..., if that Digimon is level 4 or higher, ...\", \
                         §15-8-3-9-3) (G-ENGINE-OBSERVER-TRIGGER-QUALIFIER-NOT-CHECKED-AT-TRIGGER)"
                    ),
                });
            }
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
