use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// Keep offsets and braces, but discard comments and string contents.
fn code_only(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'r' && {
            let mut j = i + 1;
            while bytes.get(j) == Some(&b'#') {
                j += 1;
            }
            bytes.get(j) == Some(&b'"')
        } {
            let mut j = i + 1;
            while bytes[j] == b'#' {
                j += 1;
            }
            let hashes = j - i - 1;
            i = j + 1;
            while i < bytes.len() {
                if bytes[i] == b'"'
                    && bytes
                        .get(i + 1..i + 1 + hashes)
                        .is_some_and(|s| s.iter().all(|&c| c == b'#'))
                {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
        } else if bytes[i] == b'\''
            && (bytes.get(i + 2) == Some(&b'\'')
                || (bytes.get(i + 1) == Some(&b'\\') && bytes.get(i + 3) == Some(&b'\'')))
        {
            i += if bytes[i + 1] == b'\\' { 4 } else { 3 };
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else {
            i += 1;
            continue;
        }
        out[start..i].fill(b' ');
    }
    String::from_utf8(out).unwrap()
}

fn calls(code: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\b([A-Za-z_][A-Za-z_0-9]*)\s*(?:::\s*<[^;{}]*>)?\s*\(").unwrap();
    re.captures_iter(code)
        .filter(|m| !code[..m.get(0).unwrap().start()].trim_end().ends_with("fn"))
        .map(|m| {
            let name = &m[1];
            let method = code[..m.get(0).unwrap().start()].trim_end().ends_with('.');
            if method && !name.starts_with("auto_resolve") && name != "enqueue_triggered" {
                format!("method:{name}")
            } else {
                name.to_string()
            }
        })
        .collect()
}

fn wrappers(sources: &BTreeMap<String, String>) -> BTreeSet<String> {
    let functions = regex::Regex::new(r"\bfn\s+(\w+)[^{;]*\{").unwrap();
    let mut bodies = Vec::new();
    for code in sources.values() {
        for m in functions.captures_iter(code) {
            let start = m.get(0).unwrap().end();
            let mut depth = 1;
            let end = code[start..]
                .char_indices()
                .find_map(|(i, c)| {
                    if c == '{' {
                        depth += 1;
                    }
                    if c == '}' {
                        depth -= 1;
                    }
                    (depth == 0).then_some(start + i)
                })
                .expect("balanced function");
            bodies.push((m[1].to_string(), calls(&code[start..end])));
        }
    }
    let mut result = BTreeSet::new();
    loop {
        let before = result.len();
        for (name, calls) in &bodies {
            if calls
                .iter()
                .any(|c| c.starts_with("auto_resolve") || result.contains(c))
            {
                result.insert(name.clone());
            }
        }
        if before == result.len() {
            return result;
        }
    }
}

fn scan(dir: &Path, root: &Path, sources: &mut BTreeMap<String, String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scan(&path, root, sources);
        } else if path.extension().is_some_and(|e| e == "rs") {
            sources.insert(
                path.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/"),
                code_only(&std::fs::read_to_string(&path).unwrap()),
            );
        }
    }
}

fn baseline_differences(
    baseline: &BTreeMap<String, usize>,
    actual: &BTreeMap<String, usize>,
) -> Vec<String> {
    let mut differences = Vec::new();
    for path in baseline
        .keys()
        .chain(actual.keys())
        .collect::<BTreeSet<_>>()
    {
        let old = baseline.get(path).copied().unwrap_or(0);
        let new = actual.get(path).copied().unwrap_or(0);
        if path.starts_with("tests/cards_behavioral/card_text/") {
            differences.push(format!("{path}: card_text must never be baselined"));
        } else if old != new {
            differences.push(format!("{path}: baseline {old}, actual {new}"));
        }
    }
    differences
}

#[test]
fn per_card_tests_require_explicit_prompts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let baseline_path = root.join("tests/dsl/auto_resolve_baseline.json");
    let mut sources = BTreeMap::new();
    scan(&root.join("tests/cards_behavioral"), root, &mut sources);
    let wrappers = wrappers(&sources);
    let mut actual = BTreeMap::new();
    for (path, code) in &sources {
        let calls = calls(code);
        let count = calls
            .iter()
            .filter(|c| c.starts_with("auto_resolve") || wrappers.contains(*c))
            .count();
        if path.starts_with("tests/cards_behavioral/card_text/") {
            assert!(
                count == 0 && !calls.iter().any(|c| c == "enqueue_triggered"),
                "{path}: card_text must use explicit prompts and real triggers, never baselined"
            );
        } else if count > 0 {
            actual.insert(path.clone(), count);
        }
    }
    if std::env::var_os("UPDATE_AUTO_RESOLVE_BASELINE").is_some() {
        if baseline_path.exists() {
            let previous: BTreeMap<String, usize> =
                serde_json::from_str(&std::fs::read_to_string(&baseline_path).unwrap()).unwrap();
            assert!(actual.iter().all(|(path, count)| *count <= previous.get(path).copied().unwrap_or(0)),
                "baseline regeneration may only shrink counts; replace new auto_resolve/wrapper calls with explicit prompts");
        }
        std::fs::write(
            &baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
    }
    let baseline: BTreeMap<String, usize> =
        serde_json::from_str(&std::fs::read_to_string(&baseline_path).expect("baseline exists"))
            .unwrap();
    let differences = baseline_differences(&baseline, &actual);
    assert!(differences.is_empty(), "{}\nReplace auto_resolve/wrapper calls with explicit prompts. After removing calls, regenerate with UPDATE_AUTO_RESOLVE_BASELINE=1 cargo test -p digimon-engine --test dsl per_card_tests_require_explicit_prompts; do not increase the baseline.", differences.join("\n"));
}

#[test]
fn lint_rejects_new_increased_stale_and_card_text_baselines() {
    let empty = BTreeMap::new();
    let one = BTreeMap::from([("example.rs".into(), 1)]);
    let two = BTreeMap::from([("example.rs".into(), 2)]);
    assert!(baseline_differences(&empty, &empty).is_empty());
    assert!(baseline_differences(&one, &one).is_empty());
    assert!(!baseline_differences(&empty, &one).is_empty());
    assert!(!baseline_differences(&one, &two).is_empty());
    assert!(!baseline_differences(&two, &one).is_empty());
    assert!(!baseline_differences(&one, &empty).is_empty());
    let forbidden = BTreeMap::from([("tests/cards_behavioral/card_text/example.rs".into(), 0)]);
    assert!(!baseline_differences(&forbidden, &empty).is_empty());
}

#[test]
fn lint_ignores_comments_strings_and_tracks_wrappers() {
    let source = code_only("// r.auto_resolve();\nfn helper(r: R) { r.auto_resolve(); } fn outer(r: R) { helper(r); } fn sample() { let s = r###\"r.auto_resolve()\"###; outer(r); }");
    assert_eq!(
        calls(&source)
            .iter()
            .filter(|s| s.starts_with("auto_resolve"))
            .count(),
        1
    );
    let found = wrappers(&BTreeMap::from([("sample.rs".into(), source)]));
    assert!(found.contains("helper") && found.contains("outer"));
    assert!(!calls("r.helper();").iter().any(|c| found.contains(c)));
    assert!(calls(&code_only(
        "/* nested /* r.auto_resolve() */ */ \"r.auto_resolve()\""
    ))
    .is_empty());
    assert!(wrappers(&BTreeMap::new()).is_empty());
    assert!(
        calls("DebugRunner::auto_resolve(&mut r); r.auto_resolve_declaring_max();")
            .iter()
            .all(|c| c.starts_with("auto_resolve"))
    );
}
