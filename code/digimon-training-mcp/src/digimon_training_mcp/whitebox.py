"""Lazy-loaded, strictly read-only PyTorch white-box diagnostics for SB3 `.zip` checkpoints.

Implements Phase 1C of `2026-11-27-model-pilot-llm-puzzle-lab-design.md` (§2.3):
- ``card_embedding_neighbors``
- ``position_saliency``
- ``plasticity_probe``
- ``export_checkpoint_onnx``

Invariants:
1. All heavy dependencies (``torch``, ``sb3_contrib``, ``digimon_engine``,
   ``digimon_gym``, ``onnx``, ``onnxruntime``, ``tools.export_onnx``) are
   imported lazily inside call paths so ``digimon-training-mcp`` boots and
   serves run-inspection tools without PyTorch installed.
2. ``checkpoint_path`` (and ``runs/`` / ``models/``) are 100% READ-ONLY.
   Checkpoints are read into ``io.BytesIO`` buffers on CPU; ONNX exports go
   exclusively into a ``sha256``-keyed cache directory.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import sys
import tempfile
import zipfile
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Sequence, Tuple


DEFAULT_FILLER_DECK: Tuple[str, ...] = ("ST1-03",) * 50


def _ensure_code_root_on_sys_path() -> None:
    """Ensure `<repo>/code` is importable for `model_contract` and `tools.export_onnx`."""
    # whitebox.py lives at <repo>/code/digimon-training-mcp/src/digimon_training_mcp/whitebox.py
    resolved = Path(__file__).resolve()
    if len(resolved.parents) >= 4:
        code_root = resolved.parents[3]
        if (code_root / "model_contract.py").is_file():
            code_str = str(code_root)
            if code_str not in sys.path:
                sys.path.insert(0, code_str)


def _layout_get(layout: Any, key: str, default: Any = ...) -> Any:
    if isinstance(layout, Mapping):
        if default is ...:
            return layout[key]
        return layout.get(key, default)
    if default is ...:
        return getattr(layout, key)
    return getattr(layout, key, default)


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _detect_recurrent_zip(raw_bytes: bytes, ckpt_label: str) -> bool:
    """Inspect SB3 `.zip` metadata in memory to distinguish LSTM vs MLP."""
    try:
        with zipfile.ZipFile(io.BytesIO(raw_bytes), "r") as zf:
            names = set(zf.namelist())
            if "data" in names:
                raw_data = zf.read("data").decode("utf-8", errors="replace")
                lowered = raw_data.lower()
                if "maskablerecurrent" in lowered or "recurrentactorcritic" in lowered or "mlplstmpolicy" in lowered:
                    return True
    except zipfile.BadZipFile as exc:
        raise ValueError(f"checkpoint is not a valid .zip archive: {ckpt_label}") from exc
    return False


def _load_sb3_checkpoint(checkpoint_path: str | Path) -> Tuple[Any, str, Dict[str, Any], bytes]:
    """Load an MLP or LSTM SB3 `.zip` checkpoint purely from memory on CPU.

    Returns ``(model, model_type, contract, raw_bytes)`` where ``model_type``
    is ``"mlp"`` or ``"lstm"``.
    """
    _ensure_code_root_on_sys_path()
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO
    from model_contract import validate_checkpoint

    ckpt_path = Path(checkpoint_path)
    if not ckpt_path.is_file():
        raise FileNotFoundError(f"checkpoint file not found: {ckpt_path}")

    raw_bytes = ckpt_path.read_bytes()
    is_recurrent = _detect_recurrent_zip(raw_bytes, str(ckpt_path))
    custom_objects = {
        "learning_rate": 0.0,
        "lr_schedule": lambda _: 0.0,
        "clip_range": lambda _: 0.2,
    }

    if is_recurrent:
        model = MaskableRecurrentPPO.load(
            io.BytesIO(raw_bytes),
            device="cpu",
            custom_objects=custom_objects,
        )
        model_type = "lstm"
    else:
        try:
            model = MaskablePPO.load(
                io.BytesIO(raw_bytes),
                device="cpu",
                custom_objects=custom_objects,
            )
            if hasattr(model.policy, "lstm_actor"):
                model = MaskableRecurrentPPO.load(
                    io.BytesIO(raw_bytes),
                    device="cpu",
                    custom_objects=custom_objects,
                )
                model_type = "lstm"
            else:
                model_type = "mlp"
        except Exception:
            model = MaskableRecurrentPPO.load(
                io.BytesIO(raw_bytes),
                device="cpu",
                custom_objects=custom_objects,
            )
            model_type = "lstm"

    contract = validate_checkpoint(model)
    model.policy.eval()
    return model, model_type, contract, raw_bytes


def _extract_decks(decks_raw: Any) -> Tuple[List[str], List[str]]:
    if isinstance(decks_raw, Mapping):
        d1 = list(decks_raw.get("1") or decks_raw.get(1) or DEFAULT_FILLER_DECK)
        d2 = list(decks_raw.get("2") or decks_raw.get(2) or DEFAULT_FILLER_DECK)
        return d1, d2
    if isinstance(decks_raw, Sequence) and not isinstance(decks_raw, (str, bytes)) and len(decks_raw) >= 2:
        d1 = list(decks_raw[0] or DEFAULT_FILLER_DECK)
        d2 = list(decks_raw[1] or DEFAULT_FILLER_DECK)
        return d1, d2
    return list(DEFAULT_FILLER_DECK), list(DEFAULT_FILLER_DECK)


def _build_position(
    puzzle_path: str | Path | None,
    scenario_json: str | Mapping[str, Any] | None,
    observation_profile: str,
) -> Tuple[Any, List[int]]:
    """Construct a `RustHeadlessGame` + prefix action list from a puzzle, search exam, or scenario."""
    import digimon_engine as de

    has_puzzle = puzzle_path is not None and str(puzzle_path).strip() != ""
    has_scenario = scenario_json is not None and (
        not isinstance(scenario_json, str) or scenario_json.strip() != ""
    )
    if not has_puzzle and not has_scenario:
        raise ValueError("either 'puzzle_path' or 'scenario_json' must be provided")
    if has_puzzle and has_scenario:
        raise ValueError("provide either 'puzzle_path' or 'scenario_json', not both")

    if has_puzzle:
        p = Path(str(puzzle_path))
        if not p.is_file():
            raise FileNotFoundError(f"puzzle file not found: {p}")
        doc = json.loads(p.read_text(encoding="utf-8"))
    elif isinstance(scenario_json, str):
        doc = json.loads(scenario_json)
    elif isinstance(scenario_json, Mapping):
        doc = dict(scenario_json)
    else:
        raise ValueError("'scenario_json' must be a JSON string or object")

    if not isinstance(doc, Mapping):
        raise ValueError("puzzle/scenario payload must be a JSON object")

    schema = doc.get("schema")

    if schema == "studio-puzzle-v1":
        deck1, deck2 = _extract_decks(doc.get("decks"))
        seed = int(doc.get("seed", 0))
        seat = int(doc.get("seat", 1))
        setup = doc.get("setup") or {}
        kind = setup.get("kind", "staged")
        prefix_actions = [int(a) for a in (setup.get("prefix_actions") or setup.get("actions") or [])]

        game = de.RustHeadlessGame(
            deck1,
            deck2,
            seed=seed,
            observation_profile=observation_profile,
        )
        if kind == "staged" or "zones" in setup or "state" in setup:
            game.skip_mulligan()
            staged_payload = {
                "state": setup.get(
                    "state",
                    {"memory": 0, "phase": "Main", "turn": 1, "first_player": seat},
                ),
                "zones": setup.get("zones", {}),
            }
            game.apply_scenario(json.dumps(staged_payload))
        elif not prefix_actions:
            game.skip_mulligan()
        return game, prefix_actions

    if schema == "search-exam-v1":
        setup = doc.get("setup") or {}
        deck1, deck2 = _extract_decks(setup.get("decks"))
        seed = int(setup.get("game_seed", 0))
        prefix_actions = [int(a) for a in (setup.get("actions") or [])]

        game = de.RustHeadlessGame(
            deck1,
            deck2,
            seed=seed,
            observation_profile=observation_profile,
        )
        staged = setup.get("staged")
        if isinstance(staged, Mapping):
            game.skip_mulligan()
            game.apply_scenario(json.dumps(dict(staged)))
        elif not prefix_actions:
            game.skip_mulligan()
        return game, prefix_actions

    # Generic scenario JSON (`{"state": ..., "zones": ..., ...}` or untagged setup wrapper)
    if isinstance(doc.get("setup"), Mapping) and ("zones" not in doc and "state" not in doc):
        setup = doc["setup"]
        deck1, deck2 = _extract_decks(doc.get("decks") or setup.get("decks"))
        seed = int(doc.get("seed", setup.get("game_seed", 0)))
        prefix_actions = [int(a) for a in (setup.get("prefix_actions") or setup.get("actions") or [])]
        game = de.RustHeadlessGame(
            deck1,
            deck2,
            seed=seed,
            observation_profile=observation_profile,
        )
        if isinstance(setup.get("staged"), Mapping):
            game.skip_mulligan()
            game.apply_scenario(json.dumps(dict(setup["staged"])))
        elif "zones" in setup or "state" in setup:
            game.skip_mulligan()
            game.apply_scenario(
                json.dumps(
                    {
                        "state": setup.get("state", {"memory": 0, "phase": "Main", "turn": 1, "first_player": 1}),
                        "zones": setup.get("zones", {}),
                    }
                )
            )
        elif not prefix_actions:
            game.skip_mulligan()
        return game, prefix_actions

    deck1, deck2 = _extract_decks(doc.get("decks"))
    seed = int(doc.get("seed", 0))
    prefix_actions = [int(a) for a in (doc.get("prefix_actions") or [])]
    game = de.RustHeadlessGame(
        deck1,
        deck2,
        seed=seed,
        observation_profile=observation_profile,
    )
    game.skip_mulligan()
    scenario_payload = {
        "state": doc.get("state", {"memory": 0, "phase": "Main", "turn": 1, "first_player": 1}),
        "zones": doc.get("zones", {}),
    }
    game.apply_scenario(json.dumps(scenario_payload))
    return game, prefix_actions


def _zero_lstm_state(policy: Any) -> Dict[str, Tuple[Any, Any]]:
    import torch

    actor_lstm = policy.lstm_actor
    h_pi = torch.zeros(actor_lstm.num_layers, 1, actor_lstm.hidden_size, dtype=torch.float32)
    c_pi = torch.zeros(actor_lstm.num_layers, 1, actor_lstm.hidden_size, dtype=torch.float32)

    critic_lstm = getattr(policy, "lstm_critic", None)
    if critic_lstm is not None:
        h_vf = torch.zeros(critic_lstm.num_layers, 1, critic_lstm.hidden_size, dtype=torch.float32)
        c_vf = torch.zeros(critic_lstm.num_layers, 1, critic_lstm.hidden_size, dtype=torch.float32)
    else:
        h_vf = h_pi.clone()
        c_vf = c_pi.clone()
    return {"pi": (h_pi, c_pi), "vf": (h_vf, c_vf)}


def _detach_lstm_state(state: Optional[Dict[str, Tuple[Any, Any]]]) -> Optional[Dict[str, Tuple[Any, Any]]]:
    if state is None:
        return None
    return {
        "pi": (state["pi"][0].detach(), state["pi"][1].detach()),
        "vf": (state["vf"][0].detach(), state["vf"][1].detach()),
    }


def _forward_policy_value(
    model: Any,
    model_type: str,
    obs_t: Any,
    lstm_state: Optional[Dict[str, Tuple[Any, Any]]] = None,
) -> Tuple[Any, Any, Optional[Dict[str, Tuple[Any, Any]]]]:
    """Differentiable single-step actor+critic forward pass matching `export_onnx.py`."""
    from digimon_gym.agents import value_activation

    policy = model.policy
    activation = value_activation.get(model)
    pi_extractor = getattr(policy, "pi_features_extractor", None) or policy.features_extractor
    vf_extractor = getattr(policy, "vf_features_extractor", None) or policy.features_extractor

    if model_type == "mlp":
        pi_feat = pi_extractor(obs_t)
        vf_feat = vf_extractor(obs_t) if vf_extractor is not pi_extractor else pi_feat
        latent_pi = policy.mlp_extractor.policy_net(pi_feat)
        latent_vf = policy.mlp_extractor.value_net(vf_feat)
        logits = policy.action_net(latent_pi)
        raw_value = policy.value_net(latent_vf)
        value = value_activation.apply(activation, raw_value)
        return logits, value, None

    if lstm_state is None:
        lstm_state = _zero_lstm_state(policy)

    h_pi, c_pi = lstm_state["pi"]
    h_vf, c_vf = lstm_state["vf"]

    pi_feat = pi_extractor(obs_t)
    lstm_pi_out, (h_pi_out, c_pi_out) = policy.lstm_actor(pi_feat.unsqueeze(0), (h_pi, c_pi))
    latent_pi = policy.mlp_extractor.policy_net(lstm_pi_out.squeeze(0))
    logits = policy.action_net(latent_pi)

    vf_feat = vf_extractor(obs_t) if vf_extractor is not pi_extractor else pi_feat
    lstm_critic = getattr(policy, "lstm_critic", None)
    if lstm_critic is not None:
        lstm_vf_out, (h_vf_out, c_vf_out) = lstm_critic(vf_feat.unsqueeze(0), (h_vf, c_vf))
        vf_seq = lstm_vf_out.squeeze(0)
    elif getattr(policy, "shared_lstm", False):
        vf_seq = lstm_pi_out.squeeze(0)
        h_vf_out, c_vf_out = h_pi_out, c_pi_out
    else:
        critic_net = getattr(policy, "critic", None)
        vf_seq = critic_net(vf_feat) if critic_net is not None else vf_feat
        h_vf_out, c_vf_out = h_vf, c_vf

    latent_vf = policy.mlp_extractor.value_net(vf_seq)
    raw_value = policy.value_net(latent_vf)
    value = value_activation.apply(activation, raw_value)
    next_state = {"pi": (h_pi_out, c_pi_out), "vf": (h_vf_out, c_vf_out)}
    return logits, value, next_state


def _advance_prefix_and_warm_state(
    game: Any,
    prefix_actions: Sequence[int],
    model: Any,
    model_type: str,
) -> Tuple[int, Optional[Dict[str, Tuple[Any, Any]]]]:
    """Replay `prefix_actions`, warming per-seat LSTM `(h, c)` identically to `search_exam.rs`."""
    import numpy as np
    import torch

    seat_states: Dict[int, Optional[Dict[str, Tuple[Any, Any]]]] = (
        {1: _zero_lstm_state(model.policy), 2: _zero_lstm_state(model.policy)}
        if model_type == "lstm"
        else {1: None, 2: None}
    )

    for idx, raw_action in enumerate(prefix_actions):
        if game.is_game_over:
            raise ValueError(f"game already ended before prefix action index {idx} ({raw_action})")
        aid = int(raw_action)
        pid = int(game.current_player_id)
        mask_np = np.asarray(game.get_action_mask(), dtype=np.float32)
        if aid < 0 or aid >= len(mask_np) or mask_np[aid] <= 0.5:
            raise ValueError(f"prefix action {aid} at index {idx} is illegal for player {pid}")
        if model_type == "lstm" and int((mask_np > 0.5).sum()) > 1:
            obs_np = np.asarray(game.get_board_tensor(pid), dtype=np.float32)
            with torch.no_grad():
                obs_t = torch.from_numpy(obs_np).unsqueeze(0)
                _, _, next_state = _forward_policy_value(model, model_type, obs_t, seat_states.get(pid))
                seat_states[pid] = _detach_lstm_state(next_state)
        game.step(aid)

    if game.is_game_over:
        raise ValueError("position is already terminal after applying prefix actions")

    root_pid = int(game.current_player_id)
    return root_pid, _detach_lstm_state(seat_states.get(root_pid))


def _find_card_embedding_module(policy: Any) -> Optional[Any]:
    for attr in ("features_extractor", "pi_features_extractor", "vf_features_extractor"):
        extractor = getattr(policy, attr, None)
        if extractor is not None and hasattr(extractor, "embedding"):
            emb = getattr(extractor, "embedding", None)
            if emb is not None and hasattr(emb, "weight"):
                return emb
    return None


# ─── Public White-Box Tool Implementations ───────────────────────────


def card_embedding_neighbors(
    checkpoint_path: str | Path,
    card_ids: Sequence[str],
    top_k: int = 10,
) -> Dict[str, Any]:
    """Inspect learned card embeddings (`l2_norm`, nearest neighbors, pairwise cosines)."""
    try:
        _ensure_code_root_on_sys_path()
        import torch
        import digimon_engine as de
    except ImportError as exc:
        return {"ok": False, "error": f"missing whitebox dependency: {exc}"}

    if not isinstance(card_ids, Sequence) or isinstance(card_ids, (str, bytes)) or len(card_ids) == 0:
        return {"ok": False, "error": "'card_ids' must be a non-empty list of card ID strings"}
    if int(top_k) < 1:
        return {"ok": False, "error": "'top_k' must be >= 1"}

    try:
        model, _model_type, _contract, _raw_bytes = _load_sb3_checkpoint(checkpoint_path)
    except Exception as exc:
        return {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    emb_module = _find_card_embedding_module(model.policy)
    if emb_module is None:
        return {
            "ok": False,
            "error": "checkpoint policy does not contain a CardEmbeddingExtractor embedding table",
        }

    weight = emb_module.weight.detach().cpu().to(torch.float32)
    vocab_size = int(weight.shape[0])
    reg = de.CardRegistry()
    db = de.CardDatabase()

    # Enumerate valid mapped registry indices (skipping padding index 0 and unmapped slots).
    valid_indices: List[int] = []
    valid_card_ids: List[str] = []
    upper = min(vocab_size, 65536)
    for idx in range(1, upper):
        cid = reg.id_of(idx)
        if cid is not None:
            valid_indices.append(idx)
            valid_card_ids.append(cid)

    if not valid_indices:
        return {"ok": False, "error": "CardRegistry has no mapped card indices within embedding table"}

    valid_idx_t = torch.tensor(valid_indices, dtype=torch.long)
    valid_vecs = weight[valid_idx_t]
    valid_norms = torch.linalg.vector_norm(valid_vecs, dim=1, keepdim=True).clamp(min=1e-12)
    valid_unit = valid_vecs / valid_norms

    resolved_queries: List[Tuple[str, int]] = []
    for raw_cid in card_ids:
        cid_str = str(raw_cid).strip()
        r_idx = reg.index_of(cid_str)
        if r_idx is None or r_idx <= 0 or r_idx >= vocab_size:
            return {
                "ok": False,
                "error": f"card_id {cid_str!r} is not mapped within checkpoint embedding table (vocab_size={vocab_size})",
            }
        canonical_id = reg.id_of(int(r_idx)) or cid_str
        resolved_queries.append((canonical_id, int(r_idx)))

    k = int(top_k)
    cards_out: List[Dict[str, Any]] = []
    for cid, r_idx in resolved_queries:
        vec = weight[r_idx]
        l2_norm = float(torch.linalg.vector_norm(vec).item())
        q_unit = vec / max(l2_norm, 1e-12)
        sims = torch.mv(valid_unit, q_unit)  # (num_valid,)

        # Exclude self from neighbors
        order = torch.argsort(sims, descending=True).tolist()
        neighbors: List[Dict[str, Any]] = []
        for pos in order:
            cand_idx = valid_indices[pos]
            if cand_idx == r_idx:
                continue
            cand_cid = valid_card_ids[pos]
            cand_meta = db.get_card(cand_cid)
            neighbors.append(
                {
                    "card_id": cand_cid,
                    "card_name": cand_meta.card_name if cand_meta is not None else cand_cid,
                    "cosine_similarity": float(sims[pos].item()),
                    "level": cand_meta.level if cand_meta is not None else None,
                    "colors": list(cand_meta.colors) if cand_meta is not None else [],
                    "traits": list(cand_meta.traits) if cand_meta is not None else [],
                }
            )
            if len(neighbors) >= k:
                break

        q_meta = db.get_card(cid)
        cards_out.append(
            {
                "card_id": cid,
                "card_name": q_meta.card_name if q_meta is not None else cid,
                "registry_index": r_idx,
                "l2_norm": l2_norm,
                "neighbors": neighbors,
            }
        )

    pairwise_cosine: Dict[str, float] = {}
    for i in range(len(resolved_queries)):
        cid_a, idx_a = resolved_queries[i]
        vec_a = weight[idx_a]
        norm_a = max(float(torch.linalg.vector_norm(vec_a).item()), 1e-12)
        for j in range(i + 1, len(resolved_queries)):
            cid_b, idx_b = resolved_queries[j]
            vec_b = weight[idx_b]
            norm_b = max(float(torch.linalg.vector_norm(vec_b).item()), 1e-12)
            cos_ab = float(torch.dot(vec_a, vec_b).item() / (norm_a * norm_b))
            pairwise_cosine[f"{cid_a}|{cid_b}"] = cos_ab

    return {
        "ok": True,
        "cards": cards_out,
        "pairwise_cosine": pairwise_cosine,
    }


def position_saliency(
    checkpoint_path: str | Path,
    puzzle_path: str | Path | None = None,
    scenario_json: str | Mapping[str, Any] | None = None,
    target_action_id: Optional[int] = None,
    avoid_action_id: Optional[int] = None,
    top_k: int = 15,
) -> Dict[str, Any]:
    """Compute exact PyTorch gradient attribution on a position (`value`, `target_logit`, or `logit_margin`)."""
    try:
        _ensure_code_root_on_sys_path()
        import numpy as np
        import torch
        import digimon_engine as de
    except ImportError as exc:
        return {"ok": False, "error": f"missing whitebox dependency: {exc}"}

    if avoid_action_id is not None and target_action_id is None:
        return {"ok": False, "error": "'avoid_action_id' requires 'target_action_id'"}
    if int(top_k) < 1:
        return {"ok": False, "error": "'top_k' must be >= 1"}

    try:
        model, model_type, contract, _raw_bytes = _load_sb3_checkpoint(checkpoint_path)
        profile_id = contract["observation_profile"]
        action_space_size = int(contract["action_space_size"])

        if target_action_id is not None and not (0 <= int(target_action_id) < action_space_size):
            return {
                "ok": False,
                "error": f"target_action_id {target_action_id} out of range [0, {action_space_size})",
            }
        if avoid_action_id is not None and not (0 <= int(avoid_action_id) < action_space_size):
            return {
                "ok": False,
                "error": f"avoid_action_id {avoid_action_id} out of range [0, {action_space_size})",
            }

        game, prefix_actions = _build_position(puzzle_path, scenario_json, profile_id)
        root_pid, root_lstm_state = _advance_prefix_and_warm_state(
            game, prefix_actions, model, model_type
        )
        obs_np = np.asarray(game.get_board_tensor(root_pid), dtype=np.float32)
        layout = de.get_observation_layout(profile_id)
    except Exception as exc:
        return {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    tensor_size = int(_layout_get(layout, "tensor_size"))
    if obs_np.shape != (tensor_size,):
        return {
            "ok": False,
            "error": f"board tensor shape {obs_np.shape} != expected ({tensor_size},)",
        }

    # Capture embedding gradients for CardEmbeddingExtractor slots (where `.long()`
    # detaches raw card-ID floats from autograd).
    captured_embeds: List[Tuple[Any, Any]] = []
    hook_handles: List[Any] = []

    def _make_embed_hook(card_id_indices_buf: Any):
        def _hook(_mod: Any, _inp: Any, out: Any) -> None:
            if out.requires_grad:
                out.retain_grad()
                captured_embeds.append((card_id_indices_buf, out))

        return _hook

    try:
        for mod in model.policy.modules():
            if hasattr(mod, "embedding") and hasattr(mod, "card_id_indices"):
                emb_layer = getattr(mod, "embedding")
                idx_buf = getattr(mod, "card_id_indices")
                hook_handles.append(emb_layer.register_forward_hook(_make_embed_hook(idx_buf)))

        model.policy.zero_grad(set_to_none=True)
        obs_t = torch.tensor(obs_np, dtype=torch.float32).unsqueeze(0).requires_grad_(True)
        logits, value, _ = _forward_policy_value(model, model_type, obs_t, root_lstm_state)

        base_value = float(value[0, 0].detach().item())
        margin_val: Optional[float] = None

        if target_action_id is not None and avoid_action_id is not None:
            objective = "logit_margin"
            t_id = int(target_action_id)
            a_id = int(avoid_action_id)
            scalar_obj = logits[0, t_id] - logits[0, a_id]
            margin_val = float(scalar_obj.detach().item())
        elif target_action_id is not None:
            objective = "target_logit"
            t_id = int(target_action_id)
            scalar_obj = logits[0, t_id]
        else:
            objective = "value"
            scalar_obj = value[0, 0]

        scalar_obj.backward()
    finally:
        for h in hook_handles:
            h.remove()

    if obs_t.grad is not None:
        obs_grad = obs_t.grad[0].detach().cpu().numpy().astype(np.float64)
    else:
        obs_grad = np.zeros((tensor_size,), dtype=np.float64)

    grad_vec = obs_grad.copy()
    grad_x_input = obs_grad * obs_np.astype(np.float64)

    for idx_buf, embed_out in captured_embeds:
        if embed_out.grad is None:
            continue
        pos = idx_buf.detach().cpu().numpy().astype(np.int64)
        e_np = embed_out[0].detach().cpu().numpy().astype(np.float64)
        g_np = embed_out.grad[0].detach().cpu().numpy().astype(np.float64)
        occupied = np.rint(obs_np[pos]).astype(np.int64) > 0
        dot_ge = (g_np * e_np).sum(axis=1)
        grad_x_input[pos] += np.where(occupied, dot_ge, 0.0)
        grad_vec[pos] += np.linalg.norm(g_np, axis=1)

    sections = _layout_get(layout, "sections")
    section_names_by_index = ["unknown"] * tensor_size
    section_attribution: List[Dict[str, Any]] = []

    for sec in sections:
        sec_name = str(_layout_get(sec, "name"))
        offset = int(_layout_get(sec, "offset"))
        size = int(_layout_get(sec, "size"))
        end = min(offset + size, tensor_size)
        for i in range(max(0, offset), end):
            section_names_by_index[i] = sec_name
        l1_gxi = float(np.abs(grad_x_input[offset:end]).sum())
        l1_g = float(np.abs(grad_vec[offset:end]).sum())
        section_attribution.append(
            {
                "section": sec_name,
                "l1_grad_x_input": l1_gxi,
                "l1_grad": l1_g,
            }
        )

    section_attribution.sort(
        key=lambda row: (row["l1_grad_x_input"], row["l1_grad"]),
        reverse=True,
    )

    card_id_pos_set = {int(v) for v in _layout_get(layout, "card_id_positions")}
    reg = de.CardRegistry()

    ranked_indices = sorted(
        range(tensor_size),
        key=lambda idx: (abs(grad_x_input[idx]), abs(grad_vec[idx])),
        reverse=True,
    )[: int(top_k)]

    top_features: List[Dict[str, Any]] = []
    for idx in ranked_indices:
        feat_entry: Dict[str, Any] = {
            "index": int(idx),
            "section": section_names_by_index[idx],
            "grad_x_input": float(grad_x_input[idx]),
            "grad": float(grad_vec[idx]),
            "obs_value": float(obs_np[idx]),
        }
        if idx in card_id_pos_set:
            raw_card_idx = int(round(float(obs_np[idx])))
            if 0 < raw_card_idx <= 65535:
                cid = reg.id_of(raw_card_idx)
                if cid is not None:
                    feat_entry["card_id"] = cid
        top_features.append(feat_entry)

    result: Dict[str, Any] = {
        "ok": True,
        "objective": objective,
        "base_value": base_value,
        "section_attribution": section_attribution,
        "top_features": top_features,
    }
    if margin_val is not None:
        result["margin"] = margin_val
    return result


def plasticity_probe(
    checkpoint_path: str | Path,
    puzzle_path: str | Path | None = None,
    scenario_json: str | Mapping[str, Any] | None = None,
    target_action_id: Optional[int] = None,
    avoid_action_ids: Optional[Sequence[int]] = None,
    steps: int = 10,
    lr: float = 1e-4,
) -> Dict[str, Any]:
    """Run an in-memory N-step Adam plasticity / tensor-blindness probe on CPU."""
    try:
        _ensure_code_root_on_sys_path()
        import numpy as np
        import torch
    except ImportError as exc:
        return {"ok": False, "error": f"missing whitebox dependency: {exc}"}

    if target_action_id is None:
        return {"ok": False, "error": "missing required 'target_action_id'"}
    if int(steps) < 1:
        return {"ok": False, "error": "'steps' must be >= 1"}
    if float(lr) <= 0.0:
        return {"ok": False, "error": "'lr' must be > 0"}

    try:
        # Fresh in-memory copy from raw bytes; never touches disk.
        model, model_type, contract, _raw_bytes = _load_sb3_checkpoint(checkpoint_path)
        profile_id = contract["observation_profile"]
        action_space_size = int(contract["action_space_size"])
        target_id = int(target_action_id)
        if not (0 <= target_id < action_space_size):
            return {
                "ok": False,
                "error": f"target_action_id {target_id} out of range [0, {action_space_size})",
            }

        avoid_ids: List[int] = []
        if avoid_action_ids is not None:
            for raw_aid in avoid_action_ids:
                aid = int(raw_aid)
                if not (0 <= aid < action_space_size):
                    return {
                        "ok": False,
                        "error": f"avoid_action_id {aid} out of range [0, {action_space_size})",
                    }
                if aid != target_id and aid not in avoid_ids:
                    avoid_ids.append(aid)

        game, prefix_actions = _build_position(puzzle_path, scenario_json, profile_id)
        root_pid, root_lstm_state = _advance_prefix_and_warm_state(
            game, prefix_actions, model, model_type
        )
        obs_np = np.asarray(game.get_board_tensor(root_pid), dtype=np.float32)
        mask_np = np.asarray(game.get_action_mask(), dtype=np.float32)
    except Exception as exc:
        return {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    legal_bool_np = mask_np > 0.5
    legal_ids = [int(i) for i in np.flatnonzero(legal_bool_np)]
    if not legal_bool_np[target_id]:
        return {
            "ok": False,
            "error": f"target_action_id {target_id} is illegal at root position (legal actions: {legal_ids})",
        }

    obs_t = torch.from_numpy(obs_np).unsqueeze(0)
    legal_mask_t = torch.from_numpy(legal_bool_np)
    ref_ids = avoid_ids if avoid_ids else [aid for aid in legal_ids if aid != target_id]

    def _compute_metrics_and_loss() -> Tuple[Dict[str, float], Any]:
        logits, value, _ = _forward_policy_value(model, model_type, obs_t, root_lstm_state)
        row_logits = logits[0]
        masked_logits = row_logits.masked_fill(~legal_mask_t, -1e9)
        log_probs = torch.log_softmax(masked_logits, dim=-1)
        probs = torch.softmax(masked_logits, dim=-1)

        ce_loss = -log_probs[target_id]
        val_scalar = value[0, 0]
        val_loss = 0.5 * (val_scalar - 1.0).pow(2)

        total_loss = ce_loss + val_loss
        if avoid_ids:
            avoid_t = torch.tensor(avoid_ids, dtype=torch.long)
            hinge = torch.relu(0.5 - (row_logits[target_id] - row_logits[avoid_t])).mean()
            total_loss = total_loss + hinge

        if ref_ids:
            best_other = max(float(row_logits[aid].detach().item()) for aid in ref_ids)
            margin = float(row_logits[target_id].detach().item()) - best_other
        else:
            margin = 0.0

        metrics = {
            "target_prob": float(probs[target_id].detach().item()),
            "value": float(val_scalar.detach().item()),
            "margin": margin,
        }
        return metrics, total_loss

    model.policy.train()
    optimizer = torch.optim.Adam(model.policy.parameters(), lr=float(lr))

    with torch.no_grad():
        pre_metrics, init_loss = _compute_metrics_and_loss()
        loss_start = float(init_loss.item())

    first_grad_norm = 0.0
    n_steps = int(steps)
    for step_idx in range(n_steps):
        optimizer.zero_grad(set_to_none=True)
        _m, step_loss = _compute_metrics_and_loss()
        step_loss.backward()
        if step_idx == 0:
            sq_sum = 0.0
            for p in model.policy.parameters():
                if p.grad is not None:
                    sq_sum += float(p.grad.detach().pow(2).sum().item())
            first_grad_norm = sq_sum ** 0.5
        optimizer.step()

    model.policy.eval()
    with torch.no_grad():
        post_metrics, final_loss = _compute_metrics_and_loss()
        loss_end = float(final_loss.item())

    zero_grad = first_grad_norm < 1e-8 or not (first_grad_norm == first_grad_norm)
    loss_improved = loss_end < loss_start - 1e-6
    prob_gain = post_metrics["target_prob"] - pre_metrics["target_prob"]
    margin_gain = post_metrics["margin"] - pre_metrics["margin"]

    learnable = bool(
        not zero_grad
        and (
            prob_gain > 0.05
            or post_metrics["target_prob"] >= 0.90
            or loss_end < 0.7 * loss_start
            or (margin_gain > 0.25 and loss_improved)
        )
    )

    if zero_grad or not loss_improved:
        diagnosis = (
            f"observation_blindness suspected: grad_norm={first_grad_norm:.3e}, "
            f"loss {loss_start:.4f} -> {loss_end:.4f} failed to improve over {n_steps} Adam steps."
        )
    elif not learnable:
        diagnosis = (
            f"weak plasticity (possible partial observation_blindness or small lr): "
            f"target_prob {pre_metrics['target_prob']:.3f} -> {post_metrics['target_prob']:.3f}, "
            f"margin {pre_metrics['margin']:.3f} -> {post_metrics['margin']:.3f}, "
            f"loss {loss_start:.4f} -> {loss_end:.4f} over {n_steps} steps."
        )
    else:
        diagnosis = (
            f"learnable: target_prob {pre_metrics['target_prob']:.3f} -> {post_metrics['target_prob']:.3f}, "
            f"value {pre_metrics['value']:.3f} -> {post_metrics['value']:.3f}, "
            f"margin {pre_metrics['margin']:.3f} -> {post_metrics['margin']:.3f}, "
            f"loss {loss_start:.4f} -> {loss_end:.4f} in {n_steps} steps "
            f"(features separate target action; bottleneck is training signal/curriculum, not observation_blindness)."
        )

    del optimizer, model
    return {
        "ok": True,
        "pre": pre_metrics,
        "post": post_metrics,
        "loss_start": loss_start,
        "loss_end": loss_end,
        "learnable": learnable,
        "diagnosis": diagnosis,
    }


def export_checkpoint_onnx(
    checkpoint_path: str | Path,
    cache_dir: str | Path | None = None,
) -> Dict[str, Any]:
    """Export an SB3 `.zip` checkpoint into a sha256-keyed ONNX cache directory."""
    try:
        _ensure_code_root_on_sys_path()
        import onnx  # noqa: F401
        import onnxruntime  # noqa: F401
        from digimon_gym.tensor_profiles import get_tensor_profile
        from tools import export_onnx as onnx_exporter
    except ImportError as exc:
        return {"ok": False, "error": f"missing whitebox/onnx dependency: {exc}"}

    try:
        ckpt_path = Path(checkpoint_path).expanduser().resolve()
        model, model_type, contract, raw_bytes = _load_sb3_checkpoint(ckpt_path)
    except Exception as exc:
        return {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    sha256 = _sha256_bytes(raw_bytes)
    if cache_dir is None or str(cache_dir).strip() == "":
        out_dir = Path(tempfile.gettempdir()).resolve() / "digimon_onnx_cache" / sha256
    else:
        base_cache = Path(cache_dir).expanduser().resolve()
        out_dir = base_cache if base_cache.name == sha256 else (base_cache / sha256)

    if out_dir == ckpt_path.parent:
        return {
            "ok": False,
            "error": f"refusing to write ONNX artifacts next to checkpoint_path ({ckpt_path.parent})",
        }

    onnx_file = out_dir / "policy.onnx"
    meta_file = out_dir / "policy.onnx.meta.json"
    value_onnx_file = out_dir / "policy.value.onnx"

    already_cached = onnx_file.is_file() and meta_file.is_file() and (
        model_type != "lstm" or value_onnx_file.is_file()
    )

    if not already_cached:
        try:
            out_dir.mkdir(parents=True, exist_ok=True)
            observation_layout = get_tensor_profile(contract["observation_profile"])
            # Redirect stdout so export_onnx's print() statements never corrupt MCP stdio!
            with contextlib.redirect_stdout(io.StringIO()):
                if model_type == "mlp":
                    try:
                        onnx_exporter.export_mlp(str(ckpt_path), str(onnx_file), observation_layout)
                    except (ValueError, TypeError):
                        # Cross-Python schedule fallback: re-save loaded model with constant float schedules
                        # inside `out_dir` (never next to `ckpt_path`) and clean up immediately.
                        compat_zip = out_dir / "_compat_ckpt.zip"
                        try:
                            model.lr_schedule = lambda _: 0.0
                            model.clip_range = lambda _: 0.2
                            model.save(str(compat_zip))
                            onnx_exporter.export_mlp(str(compat_zip), str(onnx_file), observation_layout)
                        finally:
                            compat_zip.unlink(missing_ok=True)
                else:
                    onnx_exporter.export_lstm(str(ckpt_path), str(onnx_file), observation_layout)
        except Exception as exc:
            return {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    return {
        "ok": True,
        "onnx_path": str(onnx_file),
        "value_onnx_path": str(value_onnx_file) if model_type == "lstm" else None,
        "meta_path": str(meta_file),
        "model_type": model_type,
        "sha256": sha256,
    }
