use digimon_engine::dsl::loader;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dsl/fixtures")
        .join(name)
}

#[test]
fn load_file_ok() {
    let spec = loader::load_file(&fixture("ST2-13.yaml")).unwrap();
    assert_eq!(spec.card, "ST2-13");
    assert_eq!(spec.effects.len(), 2);
}

#[test]
fn load_file_missing_file() {
    let err = loader::load_file(&fixture("no-such-file.yaml")).unwrap_err();
    assert!(matches!(err, digimon_engine::dsl::DslError::Io { .. }));
}

#[test]
fn load_file_malformed_yaml() {
    let err = loader::load_file(&fixture("bad.yaml")).unwrap_err();
    assert!(matches!(err, digimon_engine::dsl::DslError::Yaml { .. }));
}

#[test]
fn load_dir_ok_collects_errors_separately() {
    let dir = fixture("ST2-13.yaml").parent().unwrap().to_path_buf();
    let (loaded, errors) = loader::load_dir_ok(&dir);
    assert!(loaded.iter().any(|s| s.card == "ST2-13"));
    assert_eq!(errors.len(), 1); // bad.yaml produces one error
}

#[test]
fn load_dir_fails_fast() {
    let dir = fixture("ST2-13.yaml").parent().unwrap().to_path_buf();
    let result = loader::load_dir(&dir);
    assert!(result.is_err(), "load_dir should fail-fast on bad.yaml");
}
