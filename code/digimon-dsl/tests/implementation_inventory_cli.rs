//! Public CLI regressions: the native test runner's own stack cannot hide a
//! small-stack `main` in the child process. Keep fixtures/logs in the test TEMP
//! directory, including after failure, for the caller's ordinary test receipts.
use serde_json::{json, Value};
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SIMPLE: &[u8] = b"card: ST1-02\nname: Biyomon\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 2\ndp: 3000\neffects: []\n";
const DUAL: &[u8] = include_bytes!("../../digimon-engine/cards/bt25/BT25-085.yaml");
const SELF_FILTER: &[u8] = include_bytes!("../../digimon-engine/cards/p/P-180.yaml");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    cards: PathBuf,
    ids: PathBuf,
    calls: Cell<usize>,
}
impl Fixture {
    fn new(ids: &[&str]) -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!(
            "implementation-inventory-cli-{}-{stamp}-{}",
            std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let cards = root.join("cards");
        fs::create_dir(&cards).unwrap();
        let input = root.join("E.json");
        fs::write(&input, serde_json::to_vec(ids).unwrap()).unwrap();
        Self { root, cards, ids: input, calls: Cell::new(0) }
    }
    fn card(&self, relative: &str, bytes: &[u8]) {
        let path = self.cards.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_implementation_inventory"));
        command.env_remove("RUST_MIN_STACK");
        #[cfg(windows)] {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW; no shared settings.
        }
        command
    }
    fn execute(&self, command: &mut Command) -> Output {
        let n = self.calls.get();
        self.calls.set(n + 1);
        let result = command.output().unwrap();
        fs::write(self.root.join(format!("call-{n}.stdout")), &result.stdout).unwrap();
        fs::write(self.root.join(format!("call-{n}.stderr")), &result.stderr).unwrap();
        fs::write(self.root.join(format!("call-{n}.status")), format!("{:?}\n", result.status)).unwrap();
        result
    }
    fn run(&self, output: &Path, references: bool, tiny_min_stack: bool) -> Output {
        let mut command = self.command();
        command.arg(&self.ids).arg(&self.cards).arg(output);
        if references { command.arg("--references"); }
        if tiny_min_stack { command.env("RUST_MIN_STACK", "1"); }
        self.execute(&mut command)
    }
}
fn success(result: &Output) {
    assert!(result.status.success(), "child {:?}: {}", result.status, String::from_utf8_lossy(&result.stderr));
    assert!(result.stdout.is_empty(), "CLI output belongs only in the requested file");
    assert!(result.stderr.is_empty(), "{}", String::from_utf8_lossy(&result.stderr));
}
fn refusal(result: &Output, output: &Path) {
    assert_eq!(result.status.code(), Some(1), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(!result.stderr.is_empty());
    assert!(!output.exists(), "refusal unexpectedly published output");
}
fn read(path: &Path) -> Value { serde_json::from_slice(&fs::read(path).unwrap()).unwrap() }

#[test]
fn real_parsed_compiled_cards_survive_default_main_stack_and_preserve_wire() {
    let f = Fixture::new(&["BT25-085", "P-180"]);
    f.card("bt25/BT25-085.yaml", DUAL);
    f.card("p/P-180.yaml", SELF_FILTER);
    let plain = f.root.join("v1.json");
    success(&f.run(&plain, false, false));
    let references = f.root.join("references.json");
    success(&f.run(&references, true, false));
    let tiny = f.root.join("tiny-env.json");
    success(&f.run(&tiny, true, true));
    assert_eq!(fs::read(&references).unwrap(), fs::read(&tiny).unwrap(), "ambient worker-stack setting must not weaken the explicit stack or wire");
    let old = read(&plain);
    let new = read(&references);
    assert_eq!(new["inventory"], old, "opt-in wrapper must retain exact v1 inventory");
    assert_eq!(old["requested_ids"], json!(["BT25-085", "P-180"]));
    assert_eq!(old["cards"].as_array().unwrap().len(), 2);
    for row in old["cards"].as_array().unwrap() {
        assert_eq!(row["status"], "COMPILED_REFERENCE_UNRESOLVED");
    }
    for (id, bytes) in [("BT25-085", DUAL), ("P-180", SELF_FILTER)] {
        let row = old["files"].as_array().unwrap().iter().find(|x| x["card_id"] == id).unwrap();
        assert_eq!(row["input"]["bytes"], json!(bytes));
        assert!(row["parsed"].is_object() && row["compiled"].is_object());
    }
    let graph = &new["reference_graph"];
    assert_eq!(graph["source_authority"], false);
    assert_eq!(graph["runtime_admission"], false);
    assert_eq!(graph["reference_closure_complete"], false);
    let dual = graph["files"].as_array().unwrap().iter().find(|x| x["owner_id"] == "BT25-085").unwrap();
    for face in ["dual.digimon.effects", "dual.option.effects"] {
        assert!(dual["compiled"]["nodes"].as_array().unwrap().iter().any(|x| x["path"].as_str().unwrap().contains(face)));
    }
    let p = graph["files"].as_array().unwrap().iter().find(|x| x["owner_id"] == "P-180").unwrap();
    let nodes = p["compiled"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.iter().filter(|x| x["role"] == "MATCH_CARD_ID" && x["value"] == "P-180").count(), 2);
    assert!(!nodes.iter().any(|x| x["role"] == "TOKEN_CONSTRUCTION"));
}

#[test]
fn global_unselected_duplicates_parse_errors_and_missing_ids_remain_evidence() {
    let f = Fixture::new(&["ST1-02", "MISSING"]);
    f.card("selected.yaml", SIMPLE);
    let unselected = String::from_utf8(SIMPLE.to_vec()).unwrap().replace("ST1-02", "OTHER");
    f.card("nested/a.yaml", unselected.as_bytes());
    f.card("nested/b.yml", unselected.as_bytes());
    f.card("broken.yaml", b"invalid: [");
    f.card("ignored.txt", b"not yaml");
    let path = f.root.join("global.json");
    success(&f.run(&path, true, false));
    let result = read(&path);
    let inventory = &result["inventory"];
    assert_eq!(inventory["files"].as_array().unwrap().len(), 4);
    assert_eq!(inventory["cards"][0]["status"], "COMPILED_REFERENCE_UNRESOLVED");
    assert_eq!(inventory["cards"][1]["status"], "MISSING_DEFINITION");
    for code in ["DUPLICATE_CARD_DEFINITION", "NATIVE_PARSE_FAILURE", "MISSING_DEFINITION"] {
        assert!(inventory["blockers"].as_array().unwrap().iter().any(|x| x == code));
    }
    let outside: Vec<_> = inventory["files"].as_array().unwrap().iter().filter(|x| x["card_id"] == "OTHER").collect();
    assert_eq!(outside.len(), 2);
    assert!(outside.iter().all(|x| x["parsed"].is_object() && x["compiled"].is_null()));
    assert_eq!(result["reference_graph"]["traversal_complete"], false);
    assert_eq!(result["reference_graph"]["reference_closure_complete"], false);
}

#[test]
fn usage_and_invalid_request_keep_nonzero_error_and_no_publication() {
    let f = Fixture::new(&["ST1-02"]);
    f.card("card.yaml", SIMPLE);
    let mut command = f.command();
    let result = f.execute(&mut command);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(String::from_utf8(result.stderr).unwrap().trim(), "usage: implementation_inventory <E-array.json> <YAML-root> <new-output.json> [--references]");
    let output = f.root.join("must-not-exist.json");
    fs::write(&f.ids, b"[").unwrap();
    refusal(&f.run(&output, true, false), &output);
    fs::write(&f.ids, b"[\"ST1-02\",\"ST1-02\"]").unwrap();
    let result = f.run(&output, false, false);
    refusal(&result, &output);
    assert!(String::from_utf8(result.stderr).unwrap().contains("nonempty unique explicit card IDs required"));
}

#[test]
fn create_new_refuses_existing_output_without_truncation_in_both_modes() {
    let f = Fixture::new(&["ST1-02"]);
    f.card("card.yaml", SIMPLE);
    let path = f.root.join("retained.json");
    let original = b"already-owned evidence\n";
    fs::write(&path, original).unwrap();
    for references in [false, true] {
        let result = f.run(&path, references, false);
        assert_eq!(result.status.code(), Some(1), "{}", String::from_utf8_lossy(&result.stderr));
        assert!(!result.stderr.is_empty());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}
