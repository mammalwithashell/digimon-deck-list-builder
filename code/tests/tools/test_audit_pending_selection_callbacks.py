from pathlib import Path

from tools.audit_pending_selection_callbacks import (
    CALLBACK_ONLY,
    DEFAULT_CALLBACK_ONLY_BASELINE,
    RESUME_BACKED,
    RESUME_STUB,
    audit_paths,
    compare_to_baseline,
    summarize,
)


def _write_rs(tmp_path: Path, source: str) -> Path:
    path = tmp_path / "sample.rs"
    path.write_text(source, encoding="utf-8")
    return path


def test_classifies_callback_only_pending_selection(tmp_path):
    path = _write_rs(
        tmp_path,
        """
fn callback_only(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        prompt: "choose".into(),
        callback: Box::new(move |game: &mut Game, action_id| {
            game.resolve(action_id);
        }),
    });
}
""",
    )

    findings = audit_paths([path], root=tmp_path)

    assert [finding.category for finding in findings] == [CALLBACK_ONLY]
    assert summarize(findings)[CALLBACK_ONLY] == 1


def test_classifies_resume_backed_pending_selection(tmp_path):
    path = _write_rs(
        tmp_path,
        """
fn resume_backed(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        prompt: "choose".into(),
        callback: Box::new(move |game: &mut Game, action_id| {
            game.resolve(action_id);
        }),
    });
    game.pending_selection_resume = Some(ResumeFrame::LinkLeave);
}
""",
    )

    findings = audit_paths([path], root=tmp_path)

    assert [finding.category for finding in findings] == [RESUME_BACKED]
    assert summarize(findings)[RESUME_BACKED] == 1


def test_classifies_helper_parked_resume_pending_selection(tmp_path):
    path = _write_rs(
        tmp_path,
        """
fn helper_parked(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        prompt: "choose".into(),
        callback: Box::new(move |game: &mut Game, action_id| {
            game.resolve(action_id);
        }),
    });
    self.park_begin_attack_resume_frame();
}
""",
    )

    findings = audit_paths([path], root=tmp_path)

    assert [finding.category for finding in findings] == [RESUME_BACKED]


def test_classifies_long_callback_with_late_resume_assignment(tmp_path):
    filler = "\n".join(f"        game.touch({idx});" for idx in range(120))
    path = _write_rs(
        tmp_path,
        f"""
fn long_callback(game: &mut Game) {{
    game.pending_selection = Some(PendingSelection {{
        prompt: "choose".into(),
        callback: Box::new(move |game: &mut Game, action_id| {{
{filler}
            game.resolve(action_id);
        }}),
    }});
    game.pending_selection_resume = Some(ResumeFrame::LinkLeave);
}}
""",
    )

    findings = audit_paths([path], root=tmp_path)

    assert [finding.category for finding in findings] == [RESUME_BACKED]


def test_classifies_resume_stub_pending_selection(tmp_path):
    path = _write_rs(
        tmp_path,
        """
fn resume_stub(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        prompt: "choose".into(),
        callback: Box::new(move |_game: &mut Game, _action_id| {
            panic!("selection must resolve through ResumeFrame");
        }),
    });
    game.pending_selection_resume = Some(ResumeFrame::LinkLeave);
}
""",
    )

    findings = audit_paths([path], root=tmp_path)

    assert [finding.category for finding in findings] == [RESUME_STUB]
    assert summarize(findings)[CALLBACK_ONLY] == 0


def test_baseline_comparison_flags_new_callback_only_sites(tmp_path):
    path = _write_rs(
        tmp_path,
        """
fn first(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        callback: Box::new(move |_game: &mut Game, _action_id| {}),
    });
}

fn second(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        callback: Box::new(move |_game: &mut Game, _action_id| {}),
    });
}
""",
    )
    findings = audit_paths([path], root=tmp_path)

    violations = compare_to_baseline(findings, {"sample.rs": 1})

    assert len(violations) == 1
    assert violations[0].relpath == "sample.rs"
    assert violations[0].actual == 2


def test_live_engine_callback_only_sites_do_not_exceed_baseline():
    root = Path(__file__).resolve().parents[2] / "digimon-engine" / "src"

    findings = audit_paths(root.rglob("*.rs"), root=root)
    violations = compare_to_baseline(findings, DEFAULT_CALLBACK_ONLY_BASELINE)

    assert violations == []
    assert summarize(findings)[CALLBACK_ONLY] > 0


_PARKED_FIXTURE = """
fn fixture(game: &mut Game) {
    game.pending_selection = Some(PendingSelection {
        callback: Box::new(|_, _| panic!("fixture must not resolve")),
    });
}
"""


def test_out_of_line_cfg_test_modules_are_not_live_sites(tmp_path):
    parent = tmp_path / "parent.rs"
    parent.write_text(
        '#[cfg(test)]\nmod fixture_tests;\n'
        '#[cfg(test)]\n#[path = "elsewhere/moved_tests.rs"]\nmod moved_tests;\n',
        encoding="utf-8",
    )
    (tmp_path / "parent").mkdir()
    (tmp_path / "parent" / "fixture_tests.rs").write_text(
        _PARKED_FIXTURE, encoding="utf-8"
    )
    (tmp_path / "elsewhere").mkdir()
    (tmp_path / "elsewhere" / "moved_tests.rs").write_text(
        _PARKED_FIXTURE, encoding="utf-8"
    )
    live = tmp_path / "live.rs"
    live.write_text(_PARKED_FIXTURE, encoding="utf-8")

    findings = audit_paths(tmp_path.rglob("*.rs"), root=tmp_path)

    assert [finding.relpath for finding in findings] == ["live.rs"]
