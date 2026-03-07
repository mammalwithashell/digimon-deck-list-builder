# Board State Tensor & Action Decoder Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix 8 issues (4 High, 4 Medium) in `get_board_state_tensor`, `get_action_mask`, and `decode_action` affecting RL training correctness.

**Architecture:** Normalizes unbounded scalar fields, adds face-up security tracking, fixes selection mask bugs, adds post-callback recovery guards, and improves action descriptions for reused action ranges.

**Tech Stack:** Python, NumPy, Gymnasium RL environment

---

## Context

Key correction on H2: In Digimon TCG, both players' security is face-down by default. Cards can be placed "face up" in security. The engine currently has no face-up tracking.

### Current Constants

```
TENSOR_SIZE = 1375    FIELD_SLOTS = 14      MAX_SOURCES = 11
SLOT_SIZE = 40        ACTION_SPACE_SIZE = 2168  SOURCES_PER_FIELD = 12
```

### Critical Files

- `digimon_gym/engine/game.py` — tensor, mask, decoders, descriptions, serialization
- `digimon_gym/engine/core/permanent.py` — `source_opt_state`
- `digimon_gym/engine/core/player.py` — security methods
- `docs/TENSOR_SPEC.md`, `docs/ACTION_SPEC.md`
- `tests/test_tensor_and_actions.py`

---

## Task 1: H1 — Normalize DP values in tensor

**Files:** `game.py` (constants + `_write_field`)

- Add `DP_NORM = 30000.0` near tensor constants
- Slot offset +1: `float(perm.dp or 0) / DP_NORM`
- Per-source offset +2: `perm.source_dp_contribution(src) / DP_NORM`

## Task 2: H2 — Face-up security tracking + tensor fix

**Files:** `player.py`, `game.py`

### 2a. Player model
- Add `self.face_up_security: Set['CardSource'] = set()` to `Player.__init__`
- Add helpers: `add_to_security_face_up()`, `flip_security_face_down()`, `is_security_face_up()`
- Update security removal methods to clean up: `trash_security_card`, `remove_from_security`, `security_attack` (the `security_cards.pop(0)`)

### 2b. Tensor
- Replace both security writes with `_write_security_ids()` that only writes face-up card IDs

### 2c. Game state serialization
- Update `player_ui_data()` to include `securityFaceUp` list

## Task 3: H3 — SelectTrash mask: respect valid_indices

**File:** `game.py`, SelectTrash branch

- Check `ps.valid_indices` first; fall back to unconditional mask only when no valid_indices
- Add optional pass support

## Task 4: H4 — SelectSource mask: respect valid_indices

**File:** `game.py`, SelectSource branch

- Same pattern as H3

## Task 5: M1 — Normalize turn count and memory

**File:** `game.py`, global tensor section

- `t[0] = min(float(self.turn_count) / 30.0, 1.0)`
- `t[2] = float(self._get_memory_for(me)) / 10.0`

## Task 6: M2 — Replace OPT -1 sentinel

**File:** `permanent.py`, `source_opt_state`

- Change `return -1.0` to `return 0.0` when `total == 0`

## Task 7: M3 — Add post-callback recovery guards

**File:** `game.py`, `_decode_trash_selection`, `_decode_source_selection`

- Extract `_recover_from_stale_selection()` from `_decode_selection`
- Call after `callback()` in both decoders
- Refactor `_decode_selection` to use it too

## Task 8: M4 — Phase-aware _describe_single_action for 100–113

**File:** `game.py`, `_describe_single_action`

- Add BlockTiming/AllianceTiming early returns before the attack formula for actions 100-113

## Spec Updates

- `TENSOR_SPEC.md`: document normalization, face-up security, OPT sentinel change
- `ACTION_SPEC.md`: document 100-113 reuse, SelectTrash/SelectSource valid_indices

## Verification

```bash
python -m pytest tests/test_tensor_and_actions.py tests/test_phase_decoders.py -v
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, obs.max(), info['action_mask'].shape)"
python -m pytest tests -v
```
