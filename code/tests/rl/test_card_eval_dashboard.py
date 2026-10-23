from pathlib import Path

import pytest

pytest.importorskip("streamlit")
from streamlit.testing.v1 import AppTest

from digimon_gym.agents.card_eval import CardEvalCollector, save_report


APP = Path(__file__).resolve().parents[2] / "tools" / "card_eval_dashboard.py"


def sample(directory, step):
    collector = CardEvalCollector(step, 0)
    ui = {"player1": {"handIds": ["A"]}, "player2": {"handIds": ["X"]}}
    collector.start_game(ui, (["A"], ["X"]), 1)
    collector.before_step({"current_player_id": 1, "phase": "Main"}, ui, [True], 0)
    collector.after_step({"turn_count": 1, "winner_id": 1, "terminal_reason": "security_attack"}, ui, [
        {"type": "Play", "player": 1, "source_card_id": "A", "meta": {"cost_paid": 3}}
    ], True, False)
    save_report(collector.report(), directory)


def test_dashboard_local_checkpoints_filter_and_empty(tmp_path):
    sample(tmp_path, 100)
    sample(tmp_path, 200)
    app = AppTest.from_file(str(APP), default_timeout=30).run()
    assert not app.exception
    app.sidebar.text_input[0].set_value(str(tmp_path))
    app.sidebar.button[0].click().run()
    assert not app.exception
    assert len(app.dataframe) == 2
    assert set(app.dataframe[0].value.checkpoint_step) == {100, 200}
    # Select only one checkpoint; cards and routes must both filter.
    next(m for m in app.multiselect if m.label == "Checkpoints").set_value([200]).run()
    assert not app.exception
    assert set(app.dataframe[0].value.checkpoint_step) == {200}
    next(m for m in app.multiselect if m.label == "Cards").set_value([]).run()
    assert not app.exception
    assert any("No samples" in i.value for i in app.info)


def test_dashboard_wandb_error_is_visible():
    app = AppTest.from_file(str(APP), default_timeout=30).run()
    app.sidebar.radio[0].set_value("Weights & Biases").run()
    app.sidebar.text_input[0].set_value("invalid")
    app.sidebar.button[0].click().run()
    assert not app.exception
    assert "entity/project/run_id" in app.error[0].value
