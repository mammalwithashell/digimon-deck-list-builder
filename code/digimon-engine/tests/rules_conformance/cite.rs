//! The `cite!` macro and the embedded Comprehensive Rules denominator.
//!
//! `qa/rules/cr_obligations.json` (produced by `code/tools/rules/`) maps each
//! rule id to its text and a SHA-256 `fingerprint` of the *normalised* text.
//! `cite!("id", "fingerprint")` makes a test fail loudly if the rule it cites
//! is renumbered or reworded in a future manual revision. The Rust side never
//! re-hashes the source PDF; it trusts the JSON, and `normalise` below is a
//! byte-for-byte port of `normalise` in `code/tools/rules/parse_rules.py`
//! (guarded by `meta::rust_normalise_matches_python_fingerprints`).

#![allow(dead_code)] // chapter modules consume the API; unused until they land

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const OBLIGATIONS_JSON: &str = include_str!("../../../../qa/rules/cr_obligations.json");

#[derive(Debug, Deserialize)]
pub struct Obligation {
    pub chapter: u32,
    pub text: String,
    pub fingerprint: String,
    pub mechanisms: Vec<String>,
    pub status: String,
    pub blocks_training: bool,
    pub tests: Vec<String>,
    pub gap_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Deserialize)]
struct Denominator {
    obligations: BTreeMap<String, Obligation>,
}

fn table() -> &'static BTreeMap<String, Obligation> {
    static TABLE: OnceLock<BTreeMap<String, Obligation>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str::<Denominator>(OBLIGATIONS_JSON)
            .expect("qa/rules/cr_obligations.json must parse")
            .obligations
    })
}

/// All obligations, ordered by id string.
pub fn obligations() -> impl Iterator<Item = (&'static str, &'static Obligation)> {
    table().iter().map(|(k, v)| (k.as_str(), v))
}

/// The raw (un-normalised) rule text. Panics with `unknown rule id` if absent.
pub fn rule_text(id: &str) -> &'static str {
    match table().get(id) {
        Some(o) => o.text.as_str(),
        None => panic!("unknown rule id {id}"),
    }
}

struct Patterns {
    ws: Regex,
    page_num: Regex,
    cross_ref: Regex,
    bare_rule_num: Regex,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        // Mirrors _WS / _PAGE_NUM / _CROSS_REF / _BARE_RULE_NUM in parse_rules.py.
        ws: Regex::new(r"\s+").unwrap(),
        page_num: Regex::new(r"([a-z.)])\d{1,2}(\s|$)").unwrap(),
        cross_ref: Regex::new(r"(?i)\(?\s*for details,? refer to [^)\n]*\)?\s*(\d{1,2}(?:-\d+)*)?")
            .unwrap(),
        bare_rule_num: Regex::new(r"\b\d{1,2}(-\d+)+\b").unwrap(),
    })
}

/// Port of Python `normalise`. Steps in order: collapse whitespace; drop
/// glued trailing page numbers; fold curly quotes; U+3000 -> space;
/// lower-case; drop "(For details, refer to ...)" cross-references; replace
/// bare rule numbers with `#`; collapse whitespace and trim.
pub fn normalise(s: &str) -> String {
    let p = patterns();
    let t = p.ws.replace_all(s, " ");
    let t = p.page_num.replace_all(&t, "$1$2");
    let t = t
        .replace('\u{2019}', "'")
        .replace('\u{201c}', "\"")
        .replace('\u{201d}', "\"")
        .replace('\u{3000}', " ");
    let t = t.to_lowercase();
    let t = p.cross_ref.replace_all(&t, "");
    let t = p.bare_rule_num.replace_all(&t, "#");
    p.ws.replace_all(&t, " ").trim().to_string()
}

/// Lower-case hex SHA-256 of an already-normalised string.
pub fn fingerprint(norm: &str) -> String {
    let digest = Sha256::digest(norm.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Backing function of `cite!`. `expected = None` is the unpinned form
/// (existence check only).
pub fn check(id: &str, expected: Option<&str>) {
    let Some(o) = table().get(id) else {
        panic!("unknown rule id {id}");
    };
    if let Some(want) = expected {
        if o.fingerprint != want {
            panic!(
                "fingerprint mismatch for {id}: pinned {want}, current {} -- \
                 text is now: {}",
                o.fingerprint,
                normalise(&o.text)
            );
        }
    }
}

/// `cite!("15-4-3-5-1", "<64-hex fingerprint>")` pins a test to a rule's
/// current wording; `cite!("15-4-3-5-1")` only checks the id exists (first
/// drafts only -- the chapter's last step pins every cite).
macro_rules! cite {
    ($id:expr) => {
        $crate::cite::check($id, None)
    };
    ($id:expr, $fp:expr) => {
        $crate::cite::check($id, Some($fp))
    };
}
