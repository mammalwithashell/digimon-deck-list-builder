"""Native PyO3 page/token checks; requires the matching rebuilt wheel."""
import copy

import numpy as np
import pytest


def staged_inspection():
    import digimon_engine as engine
    game = engine.RustDebugGame(["ST1-03"] * 50, ["ST1-03"] * 50, seed=7,
                               observation_profile="standard_lite_v2")
    game.set_first_player(1)
    game.skip_mulligan()
    for zone in ("hand", "security", "battle_area"):
        game.clear_zone(1, zone)
    game.place_on_field(1, ["BT6-022"])
    game.inject_card(1, "BT18-037", "hand")
    for _ in range(40):
        game.inject_card(1, "ST1-03", "security_top")
    game.set_memory(10)
    game.set_phase("Main")
    assert game.get_action_mask()[400] == 1
    game.step(400)
    for _ in range(8):
        pending = game.get_pending_selection()
        if pending and pending.get("cardChoice"):
            return game
        assert pending, "digivolution must reach the security inspection"
        game.step(game.greedy_action())
    pytest.fail("security inspection was not reached")


def test_native_security_inspection_checked_tokens_and_navigation():
    game = staged_inspection()
    page = game.get_pending_selection()["cardChoice"]
    token = {key: page[key] for key in ("prompt_id", "page_generation")}
    assert page["inspection_only"] is True
    rows = [row for row in page["rows"] if row["operation"] == "inspect_card"]
    assert len(rows) == 28
    assert all(row["card_id"] == "ST1-03" for row in rows)
    mask = game.get_action_mask().copy()
    assert all(mask[row["action_id"]] == 0 for row in rows)
    before = copy.deepcopy(game.to_scenario())
    with pytest.raises(ValueError):
        game.step_checked(59)
    assert game.to_scenario() == before
    assert np.array_equal(game.get_action_mask(), mask)
    game.step_checked(59, token)
    second = game.get_pending_selection()["cardChoice"]
    assert second["page_index"] == "1"
    assert second["page_generation"] != token["page_generation"]
    before = copy.deepcopy(game.to_scenario())
    with pytest.raises(ValueError):
        game.step_checked(62, token)
    assert game.to_scenario() == before
    current = {key: second[key] for key in token}
    assert game.get_action_mask()[62] == 1
    game.step_checked(62, current)
    assert game.get_pending_selection() is None


def test_native_layout_publishes_complete_card_page_descriptor():
    import digimon_engine as engine
    from digimon_gym.tensor_profiles import get_tensor_profile
    assert engine.ACTION_SCHEMA_VERSION == 2
    profile = get_tensor_profile("standard_lite_v2")
    assert profile.tensor_size == 9434
    assert profile.card_choice["row_size"] == 128
    assert profile.card_choice["includes_nonselectable_inspection_rows"] is True
    assert profile.card_choice["zone_codes"] == {
        "battle_source": 1, "breeding_source": 2, "security": 3,
        "hand": 4, "trash": 5, "battle_top": 6,
    }
    assert profile.card_choice["actions"]["refresh_page"] == 59
    assert profile.card_choice["actions"]["finish_impossible"] == 62


def test_native_recurrent_export_preserves_masks_and_resets_state(tmp_path):
    """Fresh untrained weights verify contracts/IO, without any training fit."""
    from sb3_contrib.common.wrappers import ActionMasker
    from digimon_gym.digimon_gym import DigimonEnv
    from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO, MaskableMlpLstmPolicy
    from digimon_gym.inference.onnx_policy import OnnxLstmPolicy
    from digimon_gym.tensor_profiles import get_tensor_profile
    from model_contract import current_contract
    from tools.export_onnx import export_lstm

    profile = get_tensor_profile("standard_lite_v2")
    env = ActionMasker(DigimonEnv(tensor_profile=profile.id), lambda wrapped: wrapped.action_mask())
    model = MaskableRecurrentPPO(MaskableMlpLstmPolicy, env,
        n_steps=16, batch_size=16, seed=17, device="cpu",
        policy_kwargs=dict(lstm_hidden_size=32, n_lstm_layers=1,
            enable_critic_lstm=True, net_arch=dict(pi=[16], vf=[16])))
    # These are newly initialized weights with this exact active contract.
    model.digimon_contract = current_contract(profile.id)
    obs, _ = env.reset(seed=17)
    mask = env.action_masks()
    first_action, first_state = model.predict(obs, state=None,
        episode_start=np.array([True]), action_masks=mask, deterministic=True)
    assert mask[int(first_action)]
    _, later_state = model.predict(obs, state=first_state,
        episode_start=np.array([False]), action_masks=mask, deterministic=True)
    reset_action, reset_state = model.predict(obs, state=later_state,
        episode_start=np.array([True]), action_masks=mask, deterministic=True)
    assert int(reset_action) == int(first_action)
    for expected, actual in zip(first_state, reset_state):
        np.testing.assert_allclose(actual, expected, rtol=1e-5, atol=1e-6)
    checkpoint = tmp_path / "fresh-untrained.zip"
    actor = tmp_path / "fresh-untrained.onnx"
    model.save(checkpoint)
    export_lstm(str(checkpoint), str(actor), profile)
    policy = OnnxLstmPolicy(str(actor), hidden_size=32)
    action, onnx_first = policy.predict_with_state(obs, mask, episode_start=True)
    assert mask[action]
    _, onnx_later = policy.predict_with_state(obs, mask, state=onnx_first, episode_start=False)
    reset_action, onnx_reset = policy.predict_with_state(obs, mask, state=onnx_later, episode_start=True)
    assert mask[reset_action]
    for expected, actual in zip(onnx_first, onnx_reset):
        np.testing.assert_allclose(actual, expected, rtol=1e-5, atol=1e-6,
            err_msg="episode_start must discard caller-supplied previous-episode state")
    env.close()
