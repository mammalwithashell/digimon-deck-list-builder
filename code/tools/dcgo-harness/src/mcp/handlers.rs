//! One function per tool. Tasks 2-7 fill these in; the dispatcher exists from
//! Task 1 so `tools/list` and the stdio loop are testable immediately.

use std::path::Path;

use crate::exam::verdict::{Verdict, VerdictStore};
use crate::mcp::tools;

/// Where the ledger lives under `root` (or the repo default when root is None).
fn verdicts_dir(root: Option<&Path>) -> std::path::PathBuf {
    match root {
        Some(r) => r.join("exam-verdicts"),
        None => std::path::PathBuf::from("qa/qa-reports/exam-verdicts"),
    }
}

/// Default cap on returned clause lists. Small payloads are the point.
const DEFAULT_LIMIT: usize = 40;

pub fn dispatch(
    name: &str,
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    match name {
        "exam_status" => exam_status(params, root),
        "exam_plan" => exam_plan(params, root),
        "exam_validate" => exam_validate(params),
        "exam_authoring_guide" => exam_authoring_guide(params),
        "exam_keyword_brief" => exam_keyword_brief(params),
        _ => Err(format!("tool {name:?} is not implemented yet")),
    }
}

/// Which cards a request is about.
fn requested_cards(params: &serde_json::Value) -> Result<Vec<String>, String> {
    let explicit = tools::vec_arg(params, "cards");
    if !explicit.is_empty() {
        return Ok(explicit);
    }
    if let Some(card) = tools::opt_str_arg(params, "card") {
        return Ok(vec![card]);
    }
    if tools::opt_str_arg(params, "archetype").is_some() {
        // Archetype -> card-pool resolution lands with the campaign plan. Until
        // then, say so plainly rather than silently answering about no cards.
        return Err("archetype resolution is not wired yet -- pass `cards` explicitly \
                    (see the campaign plan)"
            .to_string());
    }
    Err("give either `card`, `cards`, or `archetype`".to_string())
}

pub fn exam_status(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = requested_cards(params)?;
    let store = VerdictStore::load_dir(&verdicts_dir(root))?;

    let mut counts = std::collections::BTreeMap::from([
        ("confirmed", 0u64),
        ("diverged", 0u64),
        ("unreachable", 0u64),
        ("unavailable", 0u64),
        ("unmeasured", 0u64),
    ]);
    let mut total = 0u64;
    for (_, cv) in store.iter() {
        if !cards.contains(&cv.card_id) {
            continue;
        }
        total += 1;
        *counts.get_mut(cv.verdict.as_str()).expect("five classes") += 1;
    }

    Ok(serde_json::json!({
        "cards": cards,
        "total_clauses": total,
        "by_verdict": counts,
        // Stated, not implied: the store only knows about clauses someone has
        // recorded. The authoritative denominator comes from clause_coverage.
        "note": "counts cover clauses present in the verdict store; \
                 run the clause extractor for the printed denominator"
    }))
}

pub fn exam_plan(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = requested_cards(params)?;
    let limit = tools::usize_arg(params, "limit", DEFAULT_LIMIT);
    let store = VerdictStore::load_dir(&verdicts_dir(root))?;

    let mut outstanding: Vec<serde_json::Value> = Vec::new();
    let mut total_outstanding = 0usize;
    for (_, cv) in store.iter() {
        if !cards.contains(&cv.card_id) {
            continue;
        }
        // Confirmed is not outstanding work. Unavailable has no oracle, so it
        // is not work either -- but it is reported by exam_status, never hidden.
        if matches!(cv.verdict, Verdict::Confirmed | Verdict::Unavailable) {
            continue;
        }
        total_outstanding += 1;
        if outstanding.len() < limit {
            outstanding.push(serde_json::json!({
                "clause_id": cv.clause_id,
                "card_id": cv.card_id,
                "label": cv.label,
                "verdict": cv.verdict.as_str(),
                "reason": cv.reason,
            }));
        }
    }

    Ok(serde_json::json!({
        "cards": cards,
        "clauses": outstanding,
        "returned": outstanding.len(),
        "outstanding_total": total_outstanding,
        "elided": total_outstanding.saturating_sub(outstanding.len()),
    }))
}

/// Lint a draft scenario before spending Unity time on it. See
/// `exam::validate` for the rule catalog and why each one exists.
pub fn exam_validate(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let yaml = tools::str_arg(params, "yaml")?;
    let findings = crate::exam::validate::validate_yaml(&yaml, None);
    Ok(serde_json::json!({
        "clean": findings.is_empty(),
        "findings": findings.iter().map(|f| serde_json::json!({
            "rule": f.rule, "message": f.message, "guide_topic": f.guide_topic
        })).collect::<Vec<_>>(),
        "note": "clause ids are checked for shape only here; run the extractor \
                 to check them against the real denominator"
    }))
}

/// Where the generated guide lives.
const GUIDE_PATH: &str = "qa/exam-authoring-guide.json";

pub fn exam_authoring_guide(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(GUIDE_PATH).map_err(|e| {
        format!(
            "cannot read {GUIDE_PATH}: {e}. Generate it with \
             `python -m tools.clause_coverage.authoring_guide`."
        )
    })?;
    let guide: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{GUIDE_PATH} is not valid JSON: {e}"))?;

    match tools::opt_str_arg(params, "topic") {
        None => {
            let names: Vec<&str> = guide["topics"]
                .as_object()
                .map(|o| o.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "topics": names,
                "overview": "Author one scenario per clause. Both seats are fully scripted. \
                    `expect:` is asserted BEFORE the step is answered, so a prompt mismatch \
                    reports itself as a finding. `do:` is symbolic and lowered against our \
                    live action mask -- never hand-write action ids. Ask for a topic for the \
                    part you need.",
                "source": guide["source"],
            }))
        }
        Some(topic) => guide["topics"]
            .get(&topic)
            .cloned()
            .ok_or_else(|| {
                let names: Vec<&str> = guide["topics"]
                    .as_object()
                    .map(|o| o.keys().map(|k| k.as_str()).collect())
                    .unwrap_or_default();
                format!("unknown topic {topic:?}; available: {names:?}")
            }),
    }
}

/// A keyword's optional-vs-mandatory kind, its rule section, and the exact
/// `general_rule.pdf` pages -- the kind predicts the PROMPT SHAPE (see
/// `tools::list`'s description for this tool).
///
/// Shells out to the Python parser deliberately: the Markdown keyword table
/// has exactly one owner. A second Rust parser of the same table would drift
/// from it exactly as a second prose copy would.
pub fn exam_keyword_brief(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = tools::str_arg(params, "keyword")?;
    let out = std::process::Command::new("python")
        .args([
            "-c",
            "import json,sys;from pathlib import Path;\
             from tools.clause_coverage.keyword_brief import load_briefs,lookup;\
             b=load_briefs(Path('docs/digimon-rules/keyword-semantics.md'),\
             Path('docs/digimon-rules/rules-index.json'));\
             print(json.dumps(lookup(b,sys.argv[1])))",
            &keyword,
        ])
        .env("PYTHONPATH", "code")
        .output()
        .map_err(|e| format!("running the keyword-brief lookup: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "keyword-brief lookup failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("keyword-brief returned invalid JSON: {e}"))?;
    if value.is_null() {
        return Err(format!(
            "no brief for {keyword:?}. It may not be a §16 keyword -- \
             [DigiXros]/[Assembly], DNA Digivolution and [Counter] are defined elsewhere."
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a verdict directory + clause-text book on disk for the handlers to read.
    fn fixture(dir: &Path) {
        use crate::exam::verdict::{ClauseVerdict, Verdict, VerdictStore};
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();

        let mut store = VerdictStore::default();
        for (clause, v) in [
            ("EX12-004#effect#0", Verdict::Confirmed),
            ("EX12-004#effect#1", Verdict::Unreachable),
            ("EX12-004#effect#2", Verdict::Unmeasured),
        ] {
            store.record(ClauseVerdict {
                clause_id: clause.to_string(),
                card_id: "EX12-004".to_string(),
                verdict: v,
                label: "[On Play]".to_string(),
                text_sha256: crate::exam::verdict::sha256_hex(clause),
                scenario_path: None,
                reason: None,
                dcgo_build: None,
                job_id: None,
                recorded_at: "2026-08-27T00:00:00+00:00".to_string(),
            });
        }
        store.save_dir(&dir.join("exam-verdicts")).unwrap();
    }

    #[test]
    fn status_always_reports_all_five_classes() {
        let dir = std::env::temp_dir().join("mcp_status_five");
        fixture(&dir);
        let params = json!({"arguments": {"card": "EX12-004"}});
        let out = exam_status(&params, Some(&dir)).expect("status");

        for class in ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"] {
            assert!(
                out["by_verdict"].get(class).is_some(),
                "missing class {class} -- a card must never read as 'passed'"
            );
        }
        let sum: u64 = ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"]
            .iter()
            .map(|c| out["by_verdict"][c].as_u64().unwrap())
            .sum();
        assert_eq!(sum, out["total_clauses"].as_u64().unwrap(),
                   "the five classes must sum to the denominator");
    }

    #[test]
    fn status_needs_a_card_or_an_archetype() {
        let params = json!({"arguments": {}});
        let err = exam_status(&params, None).expect_err("must refuse");
        assert!(err.contains("card") && err.contains("archetype"),
                "error names both accepted arguments: {err}");
    }

    #[test]
    fn plan_omits_confirmed_clauses() {
        let dir = std::env::temp_dir().join("mcp_plan_omits");
        fixture(&dir);
        let params = json!({"arguments": {"cards": ["EX12-004"]}});
        let out = exam_plan(&params, Some(&dir)).expect("plan");

        let ids: Vec<&str> = out["clauses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["clause_id"].as_str().unwrap())
            .collect();
        assert!(!ids.contains(&"EX12-004#effect#0"),
                "a confirmed clause is not outstanding work");
        assert!(ids.contains(&"EX12-004#effect#2"), "unmeasured clauses are the point");
    }

    #[test]
    fn plan_reports_what_it_elided_rather_than_truncating_silently() {
        let dir = std::env::temp_dir().join("mcp_plan_elided");
        fixture(&dir);
        let params = json!({"arguments": {"cards": ["EX12-004"], "limit": 1}});
        let out = exam_plan(&params, Some(&dir)).expect("plan");
        assert_eq!(out["clauses"].as_array().unwrap().len(), 1);
        assert!(out["elided"].as_u64().unwrap() >= 1,
                "a silent truncation reads as 'that is all of it'");
    }
    #[test]
    fn a_missing_verdict_directory_is_a_fresh_checkout_not_an_error() {
        // VerdictStore::load_dir returns an empty store for a missing directory
        // (verdict.rs::load_dir_missing_directory_is_empty_not_an_error). This
        // pins that the HANDLERS inherit it: on a fresh checkout every clause is
        // honestly unmeasured, and neither tool errors.
        let dir = std::env::temp_dir().join("mcp_handlers_no_ledger_at_all");
        let _ = std::fs::remove_dir_all(&dir);
        let params = json!({"arguments": {"cards": ["EX12-004"]}});

        let status = exam_status(&params, Some(&dir)).expect("status on a fresh checkout");
        assert_eq!(status["total_clauses"], json!(0));
        for class in ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"] {
            assert_eq!(status["by_verdict"][class], json!(0), "{class} must still be reported");
        }

        let plan = exam_plan(&params, Some(&dir)).expect("plan on a fresh checkout");
        assert_eq!(plan["outstanding_total"], json!(0));
    }

    #[test]
    fn plan_keeps_diverged_and_drops_unavailable() {
        // Diverged is outstanding work -- it is a finding to triage. Unavailable
        // is NOT work: DCGO has no script, so no oracle exists. Neither is
        // hidden; exam_status still counts both.
        use crate::exam::verdict::{ClauseVerdict, Verdict, VerdictStore};
        let dir = std::env::temp_dir().join("mcp_plan_diverged_unavailable");
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = VerdictStore::default();
        for (clause, v) in [
            ("EX12-009#effect#0", Verdict::Diverged),
            ("EX12-009#effect#1", Verdict::Unavailable),
        ] {
            store.record(ClauseVerdict {
                clause_id: clause.to_string(),
                card_id: "EX12-009".to_string(),
                verdict: v,
                label: "[On Play]".to_string(),
                text_sha256: crate::exam::verdict::sha256_hex(clause),
                scenario_path: None,
                reason: None,
                dcgo_build: None,
                job_id: None,
                recorded_at: "2026-08-28T00:00:00+00:00".to_string(),
            });
        }
        store.save_dir(&dir.join("exam-verdicts")).unwrap();

        let params = json!({"arguments": {"cards": ["EX12-009"]}});
        let plan = exam_plan(&params, Some(&dir)).expect("plan");
        let ids: Vec<&str> = plan["clauses"].as_array().unwrap().iter()
            .map(|c| c["clause_id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"EX12-009#effect#0"), "diverged is work to triage");
        assert!(!ids.contains(&"EX12-009#effect#1"), "unavailable has no oracle, so no work");

        let status = exam_status(&params, Some(&dir)).expect("status");
        assert_eq!(status["by_verdict"]["unavailable"], json!(1),
                   "unavailable is excluded from the PLAN but never hidden from STATUS");
    }

}
