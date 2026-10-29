import json
import pytest
from digimon_gym.agents.belief_contract import (
    information_contract,
    require_same_information,
    freeze_run_information,
)


def closed():
    return {
        "information": {
            "regime": "closed_decklist",
            "knowledge_schema": 1,
            "prior_hash": "a" * 64,
            "legality_hash": "b" * 64,
            "prior_provenance": "frozen train split",
        }
    }


def test_legacy_is_identifiable_and_cannot_be_relabelled():
    assert information_contract({}) == {"regime": "legacy_exact"}
    with pytest.raises(ValueError, match="mixed"):
        require_same_information([{}, closed()])


def test_prior_mismatch_and_incomplete_contract_rejected():
    a = closed()
    b = closed()
    b["information"]["prior_hash"] = "c" * 64
    with pytest.raises(ValueError, match="mixed"):
        require_same_information([a, b])
    del a["information"]["legality_hash"]
    with pytest.raises(ValueError, match="legality_hash"):
        information_contract(a)


def test_full_run_resume_cannot_switch_prior(tmp_path):
    prior = tmp_path / "prior.json"
    prior.write_text("{}")
    run = tmp_path / "run"
    run.mkdir()
    a = freeze_run_information(run, "closed_decklist", prior)
    assert freeze_run_information(run, "closed_decklist", prior) == a
    prior.write_text('{"changed":true}')
    with pytest.raises(ValueError, match="resume"):
        freeze_run_information(run, "closed_decklist", prior)
    with pytest.raises(ValueError):
        freeze_run_information(run, "open_decklist", prior)


def test_same_prior_across_generations_allowed():
    assert require_same_information([closed(), closed()])["regime"] == "closed_decklist"


def test_field_knowledge_version_is_preserved_and_cannot_mix_with_old_shards():
    old, new = closed(), closed()
    new["information"]["knowledge_schema"] = 2
    assert information_contract(new)["knowledge_schema"] == 2
    with pytest.raises(ValueError, match="mixed"):
        require_same_information([old, new])
    new["information"]["knowledge_schema"] = 3
    with pytest.raises(ValueError, match="schema"):
        information_contract(new)


def test_legacy_resume_requires_new_run(tmp_path):
    (tmp_path / "state.json").write_text(json.dumps({"next_gen": 1}))
    with pytest.raises(ValueError, match="legacy"):
        freeze_run_information(tmp_path, "open_decklist", None)
