//! One function per tool. Tasks 2-7 fill these in; the dispatcher exists from
//! Task 1 so `tools/list` and the stdio loop are testable immediately.

use std::path::{Path, PathBuf};

use crate::exam::verdict::{sha256_hex, ClauseTextBook, Verdict, VerdictStore};
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

/// The repo's tracked `clause_coverage extract` output -- the printed
/// denominator for the cards currently under active exam. Regenerate with:
///
/// ```text
/// PYTHONPATH=code python -m tools.clause_coverage.extract \
///     --card-ids <IDS...> --out qa/exam-clause-text.json
/// ```
const DEFAULT_CLAUSE_TEXT_JSON: &str = "qa/exam-clause-text.json";

/// Path to the clause-text book for this call: the explicit
/// `clause_text_json` argument, or the repo default.
fn clause_text_json_path(params: &serde_json::Value) -> PathBuf {
    tools::opt_str_arg(params, "clause_text_json")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CLAUSE_TEXT_JSON))
}

/// Load the clause-text book at `path` and seed `store`'s drift-detection
/// state from it (`VerdictStore::set_current_text_sha` for every clause the
/// book carries), so `store.summary()` / `store.is_invalidated()` see the
/// SAME text an agent would see today, not whatever text was current when the
/// verdict was recorded.
///
/// Returns `Err` (never a silent empty book) when the book cannot be read --
/// callers must say so in the payload rather than quietly falling back to the
/// stored-rows count, which is exactly the bug this whole fix exists to close
/// (see `exam::verdict`'s module docs).
fn load_clause_book(path: &Path, store: &mut VerdictStore) -> Result<ClauseTextBook, String> {
    let book = ClauseTextBook::load(path).map_err(|e| {
        format!(
            "no clause-text book at {} ({e}); pass `clause_text_json` pointing at a \
             `clause_coverage extract` output, or generate the tracked default with \
             `PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids <IDS> \
             --out {DEFAULT_CLAUSE_TEXT_JSON}`",
            path.display()
        )
    })?;
    for id in book.clause_ids() {
        if let Some(ct) = book.get(&id) {
            store.set_current_text_sha(&id, &sha256_hex(&ct.text));
        }
    }
    Ok(book)
}

/// The card id a clause id belongs to: the part before the first `#`.
fn clause_card_id(clause_id: &str) -> &str {
    clause_id.split('#').next().unwrap_or(clause_id)
}

/// Every clause id in `book` belonging to one of `cards`, unsorted.
fn card_scoped_clause_ids(book: &ClauseTextBook, cards: &[String]) -> Vec<String> {
    book.clause_ids()
        .into_iter()
        .filter(|id| cards.iter().any(|c| clause_card_id(id) == c.as_str()))
        .collect()
}

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
        "run_scenario" => run_scenario(params, root),
        "exam_probe" => exam_probe(params, root),
        "claim" => claim(params, root),
        "release" => release(params, root),
        "node_health" => node_health(params, root),
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
    let mut store = VerdictStore::load_dir(&verdicts_dir(root))?;
    let book_path = clause_text_json_path(params);

    match load_clause_book(&book_path, &mut store) {
        Ok(book) => {
            // The full printed denominator -- including clause ids with NO
            // stored row at all, which `VerdictStore::summary` reports as
            // `unmeasured`. This is the fix: the old code counted only
            // `store.iter()` (the rows someone happened to record), which
            // made a card with 2-of-6 clauses examined read as "100%
            // confirmed" the instant those 2 were.
            let denom_ids = card_scoped_clause_ids(&book, &cards);
            let sum = store.summary(&denom_ids);
            Ok(serde_json::json!({
                "cards": cards,
                "total_clauses": sum.total,
                "by_verdict": {
                    "confirmed": sum.confirmed,
                    "diverged": sum.diverged,
                    "unreachable": sum.unreachable,
                    "unavailable": sum.unavailable,
                    "unmeasured": sum.unmeasured,
                },
                // Not a sixth class -- how many of `unmeasured` are there
                // because stored text_sha256 no longer matches the clause's
                // CURRENT text (see `exam::verdict`'s module docs on drift).
                "invalidated_by_text_drift": sum.invalidated,
                "denominator_source": format!("clause-text book: {}", book_path.display()),
            }))
        }
        Err(reason) => {
            // No book available. Falling back to the stored-rows count is
            // allowed -- refusing outright would make the tool useless on a
            // fresh checkout -- but it must be SAID, not implied: this count
            // is not the printed denominator, only what someone happened to
            // record. A silent fallback here is the exact bug this fixes.
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
                "denominator_source": "stored rows only -- NOT the printed denominator",
                "note": format!(
                    "no clause-text book was available, so this counts only clauses \
                     someone has recorded a verdict for; it undercounts anything never \
                     examined. {reason}"
                ),
            }))
        }
    }
}

pub fn exam_plan(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = requested_cards(params)?;
    let limit = tools::usize_arg(params, "limit", DEFAULT_LIMIT);
    let mut store = VerdictStore::load_dir(&verdicts_dir(root))?;
    let book_path = clause_text_json_path(params);

    let (outstanding, total_outstanding, denominator_source) =
        match load_clause_book(&book_path, &mut store) {
            Ok(book) => {
                // Every clause id the book carries for these cards -- INCLUDING
                // ids with no stored row at all. Those are the unmeasured ones,
                // and they are the entire point: the old code iterated
                // `store.iter()` only, so a card with zero rows returned
                // `outstanding_total: 0` -- "no work" for the card needing the
                // most.
                let mut denom_ids = card_scoped_clause_ids(&book, &cards);
                denom_ids.sort();
                let mut outstanding: Vec<serde_json::Value> = Vec::new();
                let mut total = 0usize;
                for id in &denom_ids {
                    let verdict = match store.get(id) {
                        None => Verdict::Unmeasured,
                        Some(_) if store.is_invalidated(id) => Verdict::Unmeasured,
                        Some(cv) => cv.verdict,
                    };
                    // Confirmed is not outstanding work. Unavailable has no
                    // oracle, so it is not work either -- but it is reported
                    // by exam_status, never hidden.
                    if matches!(verdict, Verdict::Confirmed | Verdict::Unavailable) {
                        continue;
                    }
                    total += 1;
                    if outstanding.len() < limit {
                        let label = book.get(id).map(|c| c.label.clone()).unwrap_or_default();
                        let reason = store.get(id).and_then(|cv| cv.reason.clone());
                        outstanding.push(serde_json::json!({
                            "clause_id": id,
                            "card_id": clause_card_id(id),
                            "label": label,
                            "verdict": verdict.as_str(),
                            "reason": reason,
                        }));
                    }
                }
                (
                    outstanding,
                    total,
                    format!("clause-text book: {}", book_path.display()),
                )
            }
            Err(reason) => {
                // Degraded fallback, said plainly: only clauses someone
                // recorded a row for can appear at all here. A clause with NO
                // stored row -- the unmeasured case that is the whole point --
                // is invisible under this fallback.
                let mut outstanding: Vec<serde_json::Value> = Vec::new();
                let mut total = 0usize;
                for (_, cv) in store.iter() {
                    if !cards.contains(&cv.card_id) {
                        continue;
                    }
                    if matches!(cv.verdict, Verdict::Confirmed | Verdict::Unavailable) {
                        continue;
                    }
                    total += 1;
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
                (
                    outstanding,
                    total,
                    format!(
                        "stored rows only -- no clause-text book ({reason}); a clause with \
                         NO stored row (the point of `unmeasured`) cannot appear here"
                    ),
                )
            }
        };

    let returned = outstanding.len();
    Ok(serde_json::json!({
        "cards": cards,
        "clauses": outstanding,
        "returned": returned,
        "outstanding_total": total_outstanding,
        "elided": total_outstanding.saturating_sub(returned),
        "denominator_source": denominator_source,
    }))
}

/// Lint a draft scenario before spending Unity time on it. See
/// `exam::validate` for the rule catalog and why each one exists.
///
/// The "catches unknown clause ids" claim in this tool's description is only
/// true when a clause-text book is available: both call sites used to pass
/// `known_clause_ids: None` unconditionally, which silently skipped that rule
/// every time. Loading the same book `exam_status`/`exam_plan` use (default
/// or explicit `clause_text_json`) makes the marquee rule real; when no book
/// is available the check degrades to the card-prefix check only, and the
/// `note` says so rather than pretending the id was verified.
pub fn exam_validate(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let yaml = tools::str_arg(params, "yaml")?;
    let book_path = clause_text_json_path(params);
    let (known_ids, note) = match ClauseTextBook::load(&book_path) {
        Ok(book) => (
            Some(book.clause_ids()),
            format!(
                "clause ids are checked against the extractor's denominator ({})",
                book_path.display()
            ),
        ),
        Err(e) => (
            None,
            format!(
                "no clause-text book at {} ({e}); clause ids are checked for shape only -- \
                 pass `clause_text_json`, or generate the tracked default with \
                 `PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids <IDS> \
                 --out {DEFAULT_CLAUSE_TEXT_JSON}`",
                book_path.display()
            ),
        ),
    };
    let findings = crate::exam::validate::validate_yaml(&yaml, known_ids.as_deref());
    Ok(serde_json::json!({
        "clean": findings.is_empty(),
        "findings": findings.iter().map(|f| serde_json::json!({
            "rule": f.rule, "message": f.message, "guide_topic": f.guide_topic
        })).collect::<Vec<_>>(),
        "note": note,
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

/// Note attached to every sim-only payload. Stated on the wire, not left to the
/// agent's memory: six sim-green scenarios were put to the oracle in the first
/// campaign and ALL SIX failed, every one on prompt sequence.
const SIM_ONLY_NOTE: &str = "sim-only ran our engine alone: it proves the line is legal HERE \
    and cannot see DCGO's prompt sequence, which is where lines actually break. It cannot \
    find a new divergence -- only an oracle pass moves a clause to confirmed.";

pub fn run_scenario(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let path = tools::str_arg(params, "path")?;
    let sim_only = tools::bool_arg(params, "sim_only", true);
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("no scenario at {path}"));
    }
    let report = crate::exam::run_one(p, sim_only, root)?;
    let mut value = serde_json::to_value(&report)
        .map_err(|e| format!("serializing the diff report: {e}"))?;
    if sim_only {
        value["note"] = serde_json::json!(SIM_ONLY_NOTE);
    }
    Ok(value)
}

pub fn exam_probe(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let yaml = tools::str_arg(params, "yaml")?;
    let sim_only = tools::bool_arg(params, "sim_only", true);

    // Lint first: milliseconds, and it catches the families that would
    // otherwise burn a lowering pass or a Unity run.
    //
    // `lint_clean`, not `clean`: this payload can also carry a `DiffReport`
    // below, whose own notion of clean (`DiffReport::is_clean` -- every row
    // compared AND no divergence) is a completely different question. Naming
    // both `clean` would let one answer be read as the other.
    let findings = crate::exam::validate::validate_yaml(&yaml, None);
    if !findings.is_empty() {
        return Ok(serde_json::json!({
            "lint_clean": false,
            "stage": "validate",
            "findings": findings.iter().map(|f| serde_json::json!({
                "rule": f.rule, "message": f.message, "guide_topic": f.guide_topic
            })).collect::<Vec<_>>(),
        }));
    }

    // Scratch file: a probe never commits a scenario.
    let scratch = std::env::temp_dir().join(format!(
        "exam-probe-{}.yaml",
        crate::exam::verdict::sha256_hex(&yaml)[..16].to_string()
    ));
    std::fs::write(&scratch, &yaml)
        .map_err(|e| format!("writing the probe scratch file: {e}"))?;

    let report = crate::exam::run_one(&scratch, sim_only, root);
    let _ = std::fs::remove_file(&scratch);
    let report = report?;

    // No `clean` stamp here: `DiffReport::is_clean()` is false whenever
    // `dcgo_steps != compared_steps`, which is ALWAYS in sim-only (nothing
    // ran against an oracle, so nothing was compared) -- a report that is
    // definitionally not clean must not carry a field asserting the opposite.
    // The report's own fields (`compared_steps`, `dcgo_steps`, `divergences`,
    // ...) speak for themselves; a caller who wants "did the trace align"
    // reads `DiffReport::is_clean()`'s inputs directly rather than trusting a
    // label this handler fabricated.
    let mut value = serde_json::to_value(&report)
        .map_err(|e| format!("serializing the diff report: {e}"))?;
    value["stage"] = serde_json::json!(if sim_only { "sim" } else { "oracle" });
    if sim_only {
        value["note"] = serde_json::json!(SIM_ONLY_NOTE);
    }
    Ok(value)
}

/// Where the claim ledger lives under `root` (or the repo default when root
/// is None).
fn claims_dir(root: Option<&Path>) -> std::path::PathBuf {
    match root {
        Some(r) => r.join("exam-claims"),
        None => std::path::PathBuf::from(crate::exam::ledger::DEFAULT_CLAIMS),
    }
}

/// Take advisory leases on cards so another node does not duplicate the work.
/// See `exam::ledger` for why this is advisory on purpose.
pub fn claim(params: &serde_json::Value, root: Option<&Path>) -> Result<serde_json::Value, String> {
    let cards = tools::vec_arg(params, "cards");
    if cards.is_empty() {
        return Err("`cards` must name at least one card".to_string());
    }
    let job_id = tools::str_arg(params, "job_id")?;
    let ttl_hours = tools::usize_arg(params, "ttl_hours", 24) as i64;

    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(ttl_hours);
    let c = crate::exam::ledger::Claim {
        job_id,
        node: tools::opt_str_arg(params, "node").unwrap_or_else(|| "unnamed".to_string()),
        archetype: tools::opt_str_arg(params, "archetype").unwrap_or_default(),
        claimed_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    };
    let outcome = crate::exam::ledger::claim_cards(&claims_dir(root), &cards, &c, &now.to_rfc3339())?;

    Ok(serde_json::json!({
        "granted": outcome.granted,
        "held_by_others": outcome.held_by_others.iter().map(|(card, held)| serde_json::json!({
            "card": card, "job_id": held.job_id, "node": held.node,
            "archetype": held.archetype, "expires_at": held.expires_at,
        })).collect::<Vec<_>>(),
        "note": "claims are ADVISORY: git is the only coordinator, so simultaneous \
                 pushes can both claim. Duplicates are detectable at merge.",
    }))
}

/// Release this job's claims. Another job's claim is never removed --
/// releasing is not a stealing primitive.
pub fn release(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = tools::vec_arg(params, "cards");
    let job_id = tools::str_arg(params, "job_id")?;
    let released = crate::exam::ledger::release_cards(&claims_dir(root), &cards, &job_id)?;
    Ok(serde_json::json!({ "released": released }))
}

pub fn node_health(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let root = root
        .map(|r| r.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let build = tools::opt_str_arg(params, "build").map(std::path::PathBuf::from);

    // node::health never fails -- a node that cannot answer must produce a
    // readable report, not an error string.
    let h = crate::node::health(&root, build.as_deref());
    Ok(serde_json::json!({
        "go": h.go,
        "checks": h.checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "status": c.status.as_str(),
            "detail": c.detail,
            "remedy": c.remedy,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// One clause as `book_clauses` describes it: `(id, label, text)`.
    type BookClause<'a> = (&'a str, &'a str, &'a str);

    /// Build a verdict directory AND a matching clause-text book on disk.
    ///
    /// `book_clauses` is the FULL printed denominator (what a real
    /// `clause_coverage extract` would produce); `stored` is the subset that
    /// actually has a recorded verdict. An id present in `book_clauses` but
    /// absent from `stored` is exactly the case the fix exists for: a clause
    /// nobody has examined yet, which must show up as `unmeasured` rather
    /// than being invisible.
    ///
    /// Returns the clause-text book's path, for passing as `clause_text_json`.
    fn fixture(
        dir: &Path,
        card: &str,
        book_clauses: &[BookClause],
        stored: &[(&str, Verdict)],
    ) -> PathBuf {
        use crate::exam::verdict::ClauseVerdict;
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();

        let mut store = VerdictStore::default();
        for (id, v) in stored {
            let text = book_clauses
                .iter()
                .find(|(cid, _, _)| cid == id)
                .map(|(_, _, t)| *t)
                .unwrap_or_default();
            store.record(ClauseVerdict {
                clause_id: id.to_string(),
                card_id: card.to_string(),
                verdict: *v,
                label: "[On Play]".to_string(),
                text_sha256: sha256_hex(text),
                scenario_path: None,
                reason: None,
                dcgo_build: None,
                job_id: None,
                recorded_at: "2026-08-27T00:00:00+00:00".to_string(),
            });
        }
        store.save_dir(&dir.join("exam-verdicts")).unwrap();

        let clauses: Vec<serde_json::Value> = book_clauses
            .iter()
            .map(|(id, label, text)| {
                json!({
                    "id": id, "card_id": card, "zone": "effect", "label": label,
                    "kind": "timing", "timings": [], "keyword": null, "text": text,
                    "source": "cards_json",
                })
            })
            .collect();
        let book_path = dir.join("clause-text.json");
        std::fs::write(
            &book_path,
            serde_json::to_string(&json!({"clauses": clauses})).unwrap(),
        )
        .unwrap();
        book_path
    }

    #[test]
    fn status_reports_the_printed_denominator_not_just_stored_rows() {
        // The bug this fixes: 4 printed clauses, only 2 examined (1 confirmed,
        // 1 unreachable). The OLD code counted `store.iter()` only, so this
        // read as "2/2 examined, 50% confirmed" -- a card that never read as
        // "passed" because the denominator itself was wrong, not because any
        // class was hidden.
        let dir = std::env::temp_dir().join("mcp_status_denominator");
        let book = fixture(
            &dir,
            "EX12-004",
            &[
                ("EX12-004#effect#0", "[On Play]", "confirmed clause text"),
                ("EX12-004#effect#1", "[On Play]", "unreachable clause text"),
                ("EX12-004#effect#2", "[On Play]", "never examined text"),
                ("EX12-004#effect#3", "[On Play]", "also never examined text"),
            ],
            &[
                ("EX12-004#effect#0", Verdict::Confirmed),
                ("EX12-004#effect#1", Verdict::Unreachable),
            ],
        );
        let params = json!({"arguments": {
            "card": "EX12-004", "clause_text_json": book.display().to_string()
        }});
        let out = exam_status(&params, Some(&dir)).expect("status");

        assert_eq!(
            out["total_clauses"],
            json!(4),
            "the denominator is the BOOK's clause count, not the store's row count"
        );
        assert_eq!(out["by_verdict"]["confirmed"], json!(1));
        assert_eq!(out["by_verdict"]["unreachable"], json!(1));
        assert_eq!(
            out["by_verdict"]["unmeasured"], json!(2),
            "clauses with NO stored row must still count -- that is the whole point"
        );

        let sum: u64 = ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"]
            .iter()
            .map(|c| out["by_verdict"][c].as_u64().unwrap())
            .sum();
        assert_eq!(
            sum,
            out["total_clauses"].as_u64().unwrap(),
            "the five classes must sum to the denominator"
        );
        assert!(
            out["denominator_source"].as_str().unwrap().contains("clause-text book"),
            "the payload must say WHERE the denominator came from: {out}"
        );
    }

    #[test]
    fn status_falls_back_to_stored_rows_when_no_book_is_available_and_says_so() {
        // No clause_text_json argument, and no file at the default path from
        // this test's CWD (the package root under `cargo test`, not the repo
        // root) -- the degraded path. It must still answer, but it must NEVER
        // claim the stored-rows count is the printed denominator.
        let dir = std::env::temp_dir().join("mcp_status_no_book_fallback");
        fixture(
            &dir,
            "EX12-004",
            &[("EX12-004#effect#0", "[On Play]", "text")],
            &[("EX12-004#effect#0", Verdict::Confirmed)],
        );
        let params = json!({"arguments": {"card": "EX12-004"}});
        let out = exam_status(&params, Some(&dir)).expect("status must still answer");

        assert_eq!(out["total_clauses"], json!(1), "falls back to the one stored row");
        assert_eq!(out["by_verdict"]["confirmed"], json!(1));
        let source = out["denominator_source"].as_str().unwrap_or_default();
        assert!(
            !source.contains("clause-text book"),
            "must not claim a book backs this count: {source}"
        );
        assert!(
            out["note"].as_str().unwrap_or_default().contains("no clause-text book"),
            "the fallback must be stated, not silent: {out}"
        );
    }

    #[test]
    fn status_needs_a_card_or_an_archetype() {
        let params = json!({"arguments": {}});
        let err = exam_status(&params, None).expect_err("must refuse");
        assert!(err.contains("card") && err.contains("archetype"),
                "error names both accepted arguments: {err}");
    }

    #[test]
    fn status_rejects_archetype_resolution_with_a_named_error() {
        // The MCP schema no longer advertises `archetype` (FIX 3 -- archetype
        // resolution was never wired up), but the handler still refuses it
        // explicitly for anyone who passes it anyway, rather than silently
        // answering about zero cards.
        let params = json!({"arguments": {"archetype": "Toho Braves"}});
        let err = exam_status(&params, None).expect_err("must refuse");
        assert!(
            err.contains("archetype resolution is not wired yet"),
            "got: {err}"
        );
    }

    #[test]
    fn plan_omits_confirmed_clauses_and_surfaces_never_examined_ones() {
        let dir = std::env::temp_dir().join("mcp_plan_omits");
        let book = fixture(
            &dir,
            "EX12-004",
            &[
                ("EX12-004#effect#0", "[On Play]", "confirmed text"),
                ("EX12-004#effect#1", "[On Play]", "never examined text"),
            ],
            &[("EX12-004#effect#0", Verdict::Confirmed)],
        );
        let params = json!({"arguments": {
            "cards": ["EX12-004"], "clause_text_json": book.display().to_string()
        }});
        let out = exam_plan(&params, Some(&dir)).expect("plan");

        let ids: Vec<&str> = out["clauses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["clause_id"].as_str().unwrap())
            .collect();
        assert!(!ids.contains(&"EX12-004#effect#0"),
                "a confirmed clause is not outstanding work");
        assert!(
            ids.contains(&"EX12-004#effect#1"),
            "a clause with NO stored row at all must appear -- it is unmeasured, and \
             unmeasured is the point"
        );
    }

    #[test]
    fn plan_reports_what_it_elided_rather_than_truncating_silently() {
        let dir = std::env::temp_dir().join("mcp_plan_elided");
        let book = fixture(
            &dir,
            "EX12-004",
            &[
                ("EX12-004#effect#0", "[On Play]", "a"),
                ("EX12-004#effect#1", "[On Play]", "b"),
            ],
            &[],
        );
        let params = json!({"arguments": {
            "cards": ["EX12-004"], "limit": 1, "clause_text_json": book.display().to_string()
        }});
        let out = exam_plan(&params, Some(&dir)).expect("plan");
        assert_eq!(out["clauses"].as_array().unwrap().len(), 1);
        assert!(out["elided"].as_u64().unwrap() >= 1,
                "a silent truncation reads as 'that is all of it'");
    }

    #[test]
    fn a_fresh_checkout_makes_every_book_clause_outstanding() {
        // VerdictStore::load_dir returns an empty store for a missing verdict
        // directory (verdict.rs::load_dir_missing_directory_is_empty_not_an_error)
        // -- a fresh checkout, not an error. With a clause-text BOOK present but
        // the store empty, every clause the book carries is honestly unmeasured
        // -- and unmeasured clauses ARE outstanding work. (This replaces a prior
        // version of this test that asserted `outstanding_total == 0` here,
        // which directly contradicted its own "every clause is honestly
        // unmeasured" comment -- see FIX 6.)
        let dir = std::env::temp_dir().join("mcp_handlers_no_ledger_at_all");
        let _ = std::fs::remove_dir_all(&dir);
        // fixture() creates the verdicts dir as a side effect, so build the
        // book directly instead: NO exam-verdicts/ directory at all.
        std::fs::create_dir_all(&dir).unwrap();
        let book_path = dir.join("clause-text.json");
        std::fs::write(
            &book_path,
            serde_json::to_string(&json!({"clauses": [
                {"id": "EX12-004#effect#0", "card_id": "EX12-004", "zone": "effect",
                 "label": "[On Play]", "kind": "timing", "timings": [], "keyword": null,
                 "text": "a", "source": "cards_json"},
                {"id": "EX12-004#effect#1", "card_id": "EX12-004", "zone": "effect",
                 "label": "[On Play]", "kind": "timing", "timings": [], "keyword": null,
                 "text": "b", "source": "cards_json"},
            ]})).unwrap(),
        )
        .unwrap();
        let ledger_root = dir.join("no-such-ledger");
        let params = json!({"arguments": {
            "cards": ["EX12-004"], "clause_text_json": book_path.display().to_string()
        }});

        let status = exam_status(&params, Some(&ledger_root)).expect("status on a fresh checkout");
        assert_eq!(status["total_clauses"], json!(2));
        assert_eq!(status["by_verdict"]["unmeasured"], json!(2));
        for class in ["confirmed", "diverged", "unreachable", "unavailable"] {
            assert_eq!(status["by_verdict"][class], json!(0), "{class} must still be reported");
        }

        let plan = exam_plan(&params, Some(&ledger_root)).expect("plan on a fresh checkout");
        assert_eq!(
            plan["outstanding_total"],
            json!(2),
            "every clause in the book is outstanding when nothing has been recorded"
        );
    }

    #[test]
    fn plan_keeps_diverged_and_drops_unavailable() {
        // Diverged is outstanding work -- it is a finding to triage. Unavailable
        // is NOT work: DCGO has no script, so no oracle exists. Neither is
        // hidden; exam_status still counts both.
        let dir = std::env::temp_dir().join("mcp_plan_diverged_unavailable");
        let book = fixture(
            &dir,
            "EX12-009",
            &[
                ("EX12-009#effect#0", "[On Play]", "diverged text"),
                ("EX12-009#effect#1", "[On Play]", "unavailable text"),
            ],
            &[
                ("EX12-009#effect#0", Verdict::Diverged),
                ("EX12-009#effect#1", Verdict::Unavailable),
            ],
        );

        let params = json!({"arguments": {
            "cards": ["EX12-009"], "clause_text_json": book.display().to_string()
        }});
        let plan = exam_plan(&params, Some(&dir)).expect("plan");
        let ids: Vec<&str> = plan["clauses"].as_array().unwrap().iter()
            .map(|c| c["clause_id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"EX12-009#effect#0"), "diverged is work to triage");
        assert!(!ids.contains(&"EX12-009#effect#1"), "unavailable has no oracle, so no work");

        let status = exam_status(&params, Some(&dir)).expect("status");
        assert_eq!(status["by_verdict"]["unavailable"], json!(1),
                   "unavailable is excluded from the PLAN but never hidden from STATUS");
    }

    #[test]
    fn drifted_clause_text_invalidates_a_stored_confirmed_verdict() {
        // FIX 2: a stored verdict's text_sha256 is checked against the BOOK's
        // current text (via VerdictStore::set_current_text_sha, wired in
        // load_clause_book). A clause whose printed text changed since the
        // verdict was recorded must re-enter the plan as unmeasured, not keep
        // vouching for text nobody re-examined.
        let dir = std::env::temp_dir().join("mcp_status_drift");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        use crate::exam::verdict::ClauseVerdict;
        let mut store = VerdictStore::default();
        store.record(ClauseVerdict {
            clause_id: "EX12-004#effect#0".to_string(),
            card_id: "EX12-004".to_string(),
            verdict: Verdict::Confirmed,
            label: "[On Play]".to_string(),
            text_sha256: sha256_hex("the OLD clause text"),
            scenario_path: None,
            reason: None,
            dcgo_build: None,
            job_id: None,
            recorded_at: "2026-08-20T00:00:00+00:00".to_string(),
        });
        store.save_dir(&dir.join("exam-verdicts")).unwrap();

        let book_path = dir.join("clause-text.json");
        std::fs::write(
            &book_path,
            serde_json::to_string(&json!({"clauses": [
                {"id": "EX12-004#effect#0", "card_id": "EX12-004", "zone": "effect",
                 "label": "[On Play]", "kind": "timing", "timings": [], "keyword": null,
                 "text": "the NEW clause text -- a re-scrape changed it", "source": "cards_json"},
            ]})).unwrap(),
        )
        .unwrap();

        let params = json!({"arguments": {
            "card": "EX12-004", "clause_text_json": book_path.display().to_string()
        }});
        let status = exam_status(&params, Some(&dir)).expect("status");
        assert_eq!(
            status["by_verdict"]["confirmed"], json!(0),
            "a drifted verdict must not still read as confirmed"
        );
        assert_eq!(status["by_verdict"]["unmeasured"], json!(1));
        assert_eq!(
            status["invalidated_by_text_drift"], json!(1),
            "drift must be visible as its own count, not silently folded into unmeasured"
        );
    }

    #[test]
    fn probe_rejects_a_draft_that_fails_the_linter_before_running_it() {
        // Linting first is the cheap gate: milliseconds instead of a lowering
        // pass, and far cheaper than a Unity run.
        let params = json!({"arguments": {"yaml": "card: EX12-004\nsteps: []\n"}});
        let out = exam_probe(&params, None);
        match out {
            Err(e) => assert!(e.contains("clause"), "must name the missing clause: {e}"),
            Ok(v) => assert_eq!(
                v["lint_clean"],
                json!(false),
                "a bad draft must not report lint_clean"
            ),
        }
    }

    #[test]
    fn probe_says_which_question_it_answered() {
        // sim-only cannot see DCGO's prompt sequence, and a payload that does
        // not say so invites an agent to treat sim-green as confirmation.
        //
        // Runs a REAL sim probe (not just "if it happens to succeed"): under
        // `cargo test` the process CWD is the package root, so `run_one`'s
        // repo-relative defaults (`data/cards.json`, the EX12 deck pool) would
        // not resolve without DIGIMON_REPO_ROOT -- the same override
        // `exam::test_support::load_card_data` uses. Without this, the old
        // `if let Ok(v) = exam_probe(...)` silently skipped the whole
        // assertion on every `cargo test` run: `exam_probe` always errored,
        // the body never executed, and the test passed having checked nothing.
        let root = std::env::var("DIGIMON_REPO_ROOT")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_string());
        std::env::set_var("DIGIMON_REPO_ROOT", &root);

        let params = json!({"arguments": {"yaml": GOOD_YAML, "sim_only": true}});
        let v = exam_probe(&params, None)
            .expect("a real card and a real deck pool must probe cleanly");
        let note = v["note"].as_str().unwrap_or_default();
        assert!(
            note.contains("sim-only") && note.contains("cannot"),
            "the payload must state sim-only's limit: {note}"
        );
        assert!(
            v.get("clean").is_none(),
            "a sim-only run must not stamp a `clean` field that contradicts its own \
             compared_steps/dcgo_steps counts: {v}"
        );
    }

    #[test]
    fn run_scenario_defaults_to_sim_only() {
        let params = json!({"arguments": {"path": "does/not/exist.yaml"}});
        let err = run_scenario(&params, None).expect_err("missing file must fail");
        assert!(err.contains("does/not/exist.yaml"), "error names the path: {err}");
    }

    // Mirrors the committed smoke scenario `qa/dcgo-exams/ST1/ST1-08.yaml`
    // (same card, same deck, same single `pass` step) rather than inventing a
    // new line: that file is part of the 145-scenario corpus the CLI already
    // lowers cleanly, so this is known-legal, not a guess. The original
    // version of this fixture named EX12-004 with a `play: {from: hand}`
    // step -- but EX12-004 is a Digi-Egg (it starts in the egg deck, not
    // hand), so that line was never legal. Nothing caught it because the old
    // `if let Ok` swallowed the resulting error unconditionally (FIX 4).
    const GOOD_YAML: &str = r#"
card: ST1-08
clause: ST1-08#effect#0
seed: 424242
decks:
  p0: { stack: [], rest: starter_st1_gaia_red }
  p1: { stack: [], rest: starter_st1_gaia_red }
steps:
  - actor: 0
    do: { pass: {} }
"#;

    #[test]
    fn claim_reports_who_holds_a_contended_card() {
        let dir = std::env::temp_dir().join("mcp_claim_contended");
        let _ = std::fs::remove_dir_all(&dir);
        let first = json!({"arguments": {
            "cards": ["EX7-005"], "job_id": "musketeers-01", "archetype": "Three Musketeers"}});
        claim(&first, Some(&dir)).expect("first claim");

        let second = json!({"arguments": {
            "cards": ["EX7-005", "EX7-008"], "job_id": "beelstar-01", "archetype": "Beelstar"}});
        let out = claim(&second, Some(&dir)).expect("second claim");

        assert_eq!(out["granted"], json!(["EX7-008"]));
        assert_eq!(out["held_by_others"][0]["card"], json!("EX7-005"));
        assert_eq!(out["held_by_others"][0]["job_id"], json!("musketeers-01"));
    }

    #[test]
    fn release_does_not_take_another_jobs_claim() {
        let dir = std::env::temp_dir().join("mcp_claim_release");
        let _ = std::fs::remove_dir_all(&dir);
        claim(&json!({"arguments": {"cards": ["EX7-005"], "job_id": "musketeers-01"}}),
              Some(&dir)).unwrap();
        let out = release(&json!({"arguments": {"cards": ["EX7-005"], "job_id": "beelstar-01"}}),
                          Some(&dir)).expect("release");
        assert_eq!(out["released"], json!(0), "releasing is not a stealing primitive");
    }

    #[test]
    fn node_health_reports_go_and_every_check() {
        let dir = std::env::temp_dir().join("mcp_node_health");
        let _ = std::fs::remove_dir_all(&dir);
        let params = json!({"arguments": {"build": "does/not/exist"}});
        let out = node_health(&params, Some(&dir)).expect("health never errors");

        assert_eq!(out["go"], json!(false));
        let checks = out["checks"].as_array().expect("checks array");
        assert!(checks.len() >= 3, "every check reports, not just the first failure");
        assert!(
            checks.iter().any(|c| c["status"] == json!("fail") && c["remedy"].is_string()),
            "a failing check must tell the agent what to do: {checks:?}"
        );
    }
}
