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
