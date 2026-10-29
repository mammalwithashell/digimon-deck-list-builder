from __future__ import annotations

from tools.impact_scope import scope_for_changed_paths


def fixture_index() -> dict:
    return {
        "schema_version": 1,
        "cards": {
            "BT1-001": {
                "path": "bt1/BT1-001.yaml",
                "verbs": ["draw"],
                "predicates": ["name_contains"],
                "timings": ["on_play"],
            },
            "BT1-002": {
                "path": "bt1/BT1-002.yaml",
                "verbs": ["draw", "delete_permanent"],
                "predicates": [],
                "timings": ["on_deletion"],
            },
        },
        "verbs": {
            "delete_permanent": ["BT1-002"],
            "draw": ["BT1-001", "BT1-002"],
        },
        "predicates": {"name_contains": ["BT1-001"]},
        "timings": {"on_play": ["BT1-001"], "on_deletion": ["BT1-002"]},
    }


def test_scope_for_mapped_lowering_file_emits_consuming_cards():
    result = scope_for_changed_paths(
        ["code/digimon-engine/src/dsl_cards/step/draw.rs"],
        dsl_index=fixture_index(),
        lowering_map={
            "code/digimon-engine/src/dsl_cards/step/draw.rs": ["draw"],
        },
    )

    assert not result.full_suite_required
    assert result.verbs == ["draw"]
    assert result.cards == ["BT1-001", "BT1-002"]
    assert result.cards_behavioral_filter == "BT1-001|BT1-002"
    assert result.side_binaries == ["dsl"]


def test_scope_for_card_yaml_emits_the_changed_card():
    result = scope_for_changed_paths(
        ["code/digimon-engine/cards/bt1/BT1-001.yaml"],
        dsl_index=fixture_index(),
        lowering_map={},
    )

    assert not result.full_suite_required
    assert result.cards == ["BT1-001"]
    assert result.cards_behavioral_filter == "BT1-001"


def test_scope_for_unmapped_engine_file_escalates_to_full_suite():
    result = scope_for_changed_paths(
        ["code/digimon-engine/src/combat.rs"],
        dsl_index=fixture_index(),
        lowering_map={},
    )

    assert result.full_suite_required
    assert result.cards_behavioral_filter == ""
    assert result.reasons == [
        "code/digimon-engine/src/combat.rs is an unmapped engine/core file"
    ]


def helper_scope_repo(tmp_path, monkeypatch):
    from tools import impact_scope
    code = tmp_path / "code"
    engine = code / "digimon-engine"
    (engine / "tests/support").mkdir(parents=True)
    (engine / "tests/cards_behavioral").mkdir()
    (engine / "tests/effects").mkdir()
    (engine / "tests/infra").mkdir()
    (engine / "Cargo.toml").write_text(
        '[package]\nname="probe"\nversion="0.1.0"\n'
        '[[test]]\nname="behavioral_alias"\npath="tests/cards_behavioral/main.rs"\n'
        '[[test]]\nname="effects"\npath="tests/effects/main.rs"\n'
        '[[test]]\nname="infra"\npath="tests/infra/main.rs"\n', encoding="utf-8")
    for domain in ("cards_behavioral", "effects", "infra"):
        (engine / f"tests/{domain}/main.rs").write_text("", encoding="utf-8")
    (engine / "tests/qualification.rs").write_text("", encoding="utf-8")
    monkeypatch.setattr(impact_scope, "CODE_DIR", code)
    return engine


def test_shared_helper_fans_out_to_actual_folder_and_standalone_consumers(tmp_path, monkeypatch):
    engine = helper_scope_repo(tmp_path, monkeypatch)
    (engine / "tests/support/shared.rs").write_text("pub fn helper() {}", encoding="utf-8")
    (engine / "tests/cards_behavioral/main.rs").write_text(
        '#[path="../support/shared.rs"] mod helper;', encoding="utf-8")
    (engine / "tests/effects/nested.rs").write_text(
        '#[path="../support/shared.rs"] mod helper;', encoding="utf-8")
    (engine / "tests/qualification.rs").write_text(
        '#[path="support/shared.rs"] mod helper;', encoding="utf-8")
    result = scope_for_changed_paths(["code/digimon-engine/tests/support/shared.rs"],
        dsl_index=fixture_index(), lowering_map={})
    assert result.side_binaries == ["behavioral_alias", "effects", "qualification"]
    assert "support" not in result.side_binaries


def test_shared_helper_fanout_follows_transitive_helper_modules(tmp_path, monkeypatch):
    engine = helper_scope_repo(tmp_path, monkeypatch)
    (engine / "tests/support/leaf.rs").write_text("pub fn helper() {}", encoding="utf-8")
    (engine / "tests/support/outer.rs").write_text('#[path="leaf.rs"] mod leaf;', encoding="utf-8")
    (engine / "tests/infra/main.rs").write_text(
        '#[path="../support/outer.rs"] mod helper;', encoding="utf-8")
    result = scope_for_changed_paths(["code/digimon-engine/tests/support/leaf.rs"],
        dsl_index=fixture_index(), lowering_map={})
    assert result.side_binaries == ["infra"]


def test_unresolved_shared_helper_requires_full_real_target_scope(tmp_path, monkeypatch):
    helper_scope_repo(tmp_path, monkeypatch)
    result = scope_for_changed_paths(["code/digimon-engine/tests/support/deleted.rs"],
        dsl_index=fixture_index(), lowering_map={})
    assert result.full_suite_required
    assert result.side_binaries == ["behavioral_alias", "effects", "infra", "qualification"]
    assert "support" not in result.side_binaries
    assert any("shared test helper" in reason for reason in result.reasons)


def test_shared_helper_follows_standalone_out_of_line_module(tmp_path, monkeypatch):
    engine = helper_scope_repo(tmp_path, monkeypatch)
    (engine / "tests/support/shared.rs").write_text("pub fn helper() {}", encoding="utf-8")
    (engine / "tests/qualification_parts").mkdir()
    (engine / "tests/qualification.rs").write_text(
        '#[path="qualification_parts/search.rs"] mod search;', encoding="utf-8")
    (engine / "tests/qualification_parts/search.rs").write_text(
        '#[path="../support/shared.rs"] mod helper;', encoding="utf-8")
    result = scope_for_changed_paths(["code/digimon-engine/tests/support/shared.rs"],
        dsl_index=fixture_index(), lowering_map={})
    assert result.side_binaries == ["qualification"]


def test_dynamic_helper_include_keeps_uncertain_consumer_and_full_scope(tmp_path, monkeypatch):
    engine = helper_scope_repo(tmp_path, monkeypatch)
    (engine / "tests/infra/main.rs").write_text(
        'include!(concat!(env!("OUT_DIR"), "/generated.rs"));', encoding="utf-8")
    result = scope_for_changed_paths(["code/digimon-engine/tests/support/shared.rs"],
        dsl_index=fixture_index(), lowering_map={})
    assert result.full_suite_required
    assert "infra" in result.side_binaries
    assert "support" not in result.side_binaries


def test_default_worktree_paths_include_staged_and_untracked(monkeypatch):
    from types import SimpleNamespace
    from tools import impact_scope
    calls = []
    def run(command, **kwargs):
        calls.append(command)
        if command[1] == "ls-files":
            return SimpleNamespace(stdout="new.rs\nworking.rs\n")
        return SimpleNamespace(stdout="staged.rs\nworking.rs\n" if "HEAD" in command else "working.rs\n")
    monkeypatch.setattr(impact_scope.subprocess, "run", run)
    assert impact_scope.changed_paths_from_git(None, None) == ["staged.rs", "working.rs", "new.rs"]
    assert calls == [["git", "diff", "--name-only", "HEAD"],
                     ["git", "ls-files", "--others", "--exclude-standard"]]


def test_base_without_head_includes_untracked_worktree_paths(monkeypatch):
    from types import SimpleNamespace
    from tools import impact_scope
    calls = []
    def run(command, **kwargs):
        calls.append(command)
        return SimpleNamespace(stdout="new.rs\n" if command[1] == "ls-files" else "base-change.rs\n")
    monkeypatch.setattr(impact_scope.subprocess, "run", run)
    assert impact_scope.changed_paths_from_git("origin/main", None) == ["base-change.rs", "new.rs"]
    assert calls[0] == ["git", "diff", "--name-only", "origin/main"]


def test_explicit_revision_pair_does_not_consume_worktree_paths(monkeypatch):
    from types import SimpleNamespace
    from tools import impact_scope
    calls = []
    def run(command, **kwargs):
        calls.append(command)
        return SimpleNamespace(stdout="committed.rs\n")
    monkeypatch.setattr(impact_scope.subprocess, "run", run)
    assert impact_scope.changed_paths_from_git("base", "head") == ["committed.rs"]
    assert calls == [["git", "diff", "--name-only", "base", "head"]]


def test_real_git_worktree_inventory_keeps_all_three_change_states(tmp_path, monkeypatch):
    import subprocess
    from tools.impact_scope import changed_paths_from_git
    def git(*args):
        return subprocess.run(["git", *args], cwd=tmp_path, check=True, capture_output=True, text=True)
    git("init")
    for name in ("staged.rs", "working.rs"):
        (tmp_path / name).write_text("old", encoding="utf-8")
    git("add", ".")
    git("-c", "user.name=Scope Probe", "-c", "user.email=scope@example.invalid", "commit", "-m", "fixture")
    (tmp_path / "staged.rs").write_text("staged", encoding="utf-8")
    git("add", "staged.rs")
    (tmp_path / "working.rs").write_text("working", encoding="utf-8")
    (tmp_path / "new.rs").write_text("untracked", encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    assert set(changed_paths_from_git(None, None)) == {"staged.rs", "working.rs", "new.rs"}
