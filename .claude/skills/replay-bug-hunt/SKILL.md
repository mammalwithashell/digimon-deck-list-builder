---
name: replay-bug-hunt
description: Step through a single recorded Digimon game (DCGO PvP/bot recording, or a native self-play/eval recording) through the Rust engine to hunt, localize, and confirm engine bugs. Picks a differential playbook (DCGO oracle) or a judge playbook (faithfulness vs card text + rules) from the recording's source format. Drives the digimon-engine MCP's stepping + scanner tools, confirms findings against DCGO C# and general_rule.pdf, and writes confirmed findings to the gap trackers. Does NOT fix the engine.
argument-hint: [RECORDING_PATH] (or a training run + game selector)
---

# Replay Bug-Hunt

You take **one recorded game** and replay it step-by-step through the Rust engine
to find, localize, and **confirm** engine bugs — then write each confirmed finding
to the right tracker. You are the **microscope**: the parity harness
(`code/tools/dcgo-replay/`) is the funnel that flags which games are worth this
attention; this skill investigates one flagged game in depth.

You do **not** fix the engine here. A hunt ends at a confirmed, localized,
sourced finding in a tracker.

## Two modes (the recording's source picks the playbook)

- **Mode 1 — differential (DCGO source).** DCGO is battle-tested, so an action it
  took is ground truth. When the Rust engine **masks out** a recorded action (or
  expects a different actor / winner / reveal), that is a mechanical Rust-bug
  signal. One-directional: the recording stores only the action taken, not DCGO's
  full mask, so you cannot detect "Rust over-permits an action DCGO would reject."
- **Mode 2 — judge (native self-play / eval source).** The recording came from the
  Rust engine itself, so replay reproduces it exactly — **there is no external
  oracle**. Bugs are *faithfulness violations* you judge by reading the card text,
  `general_rule.pdf`, and DCGO C#, using engine signals (`EffectFizzled`, silent
  no-ops, impossible states) as leads.

## Prerequisites

- The engine MCP must be built and registered (it is in `.mcp.json` as
  `digimon-engine-mcp`). If tool calls fail, build it:
  `cargo build -p digimon-engine-mcp` and restart the MCP client.
- A recording to investigate: a path to a native `GameRecorder` JSON recording
  (eval / self-play) or a DCGO JSONL recording. If you were given a training run +
  game selector instead, resolve it to a recording path first (the training MCP's
  `run_recordings` lists a run's recordings; the parity harness output names
  flagged DCGO games).
- Source-priority for "what should this card do" (per CLAUDE.md): `general_rule.pdf`
  (canonical) and DCGO C# (battle-tested) **outrank** the card-text JSON. Resolve
  the base-repo DCGO checkout — never the empty per-worktree placeholder:
  ```bash
  BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
  ```
  C# card files live at `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`
  (filenames use underscores: `BT17-001` → `BT17_001.cs`).

## MCP tool surface (this skill's instrument)

Loading & stepping: `load_recording`, `step_forward`, `step_back`, `seek`,
`restore_checkpoint`, `replay_step_view`.
Scanners (deterministic, cheap, cursor-preserving): `scan_divergences`,
`scan_fizzles`, `scan_panics`.
Context per step: `state`, `hand`, `field`, `security`, `pending_selection`,
`effect_queue`, `events`, `inspect_card`, `legal_actions`, `deck_cards`,
`recorded_actions`.

A **step view** (returned by the stepping tools, or read-only via
`replay_step_view`) carries: `cursor` / `total_steps` / `paused` / `policy`, the
`recorded` next action (decoded), `legal_now` (what the engine *would* allow here),
`divergences` so far, `events` emitted by the last forward step, a memory/zone
`delta`, and `card_ids_in_play` (opaque tops render as `<hidden>`).

---

## Phase 0 — Load the recording and select the playbook

1. `load_recording` with `recording_path` (or inline `recording_json`). It
   auto-detects the format and returns `{ game_id, total_steps, source_format }`.
2. Read `source_format`:
   - `"dcgo"` → **Mode 1** (Phase 1 below). The driver runs `CheckThenApply` — the
     engine is checked against the DCGO oracle at every step.
   - `"native"` → **Mode 2** (Phase 2 below). The driver runs `Trust` — replay
     reproduces the engine's own trajectory; you judge faithfulness.
3. Orient: call `deck_cards` once to read both decks in context, and
   `recorded_actions` (with `decode_labels: true`) for a human-readable action log.

---

## Phase 1 — Mode 1: differential playbook (DCGO oracle)

Goal: find the first place the engine disagrees with DCGO, localize it to a card,
and confirm it's a Rust bug.

1. **Find the divergence.** `scan_divergences` with `stop_at_first: true`. Each
   finding carries `step`, `actor`, `action_id`, `phase`, a `detail`, and a
   structured `divergence` (kind = `mask_miss` / `actor` / `winner` /
   `reveal_kind` / `reveal_exhausted`). If there are none, the engine matched DCGO
   for the whole game — record a clean pass and stop.
2. **Step to the lead-up.** `seek` to the divergence step `N`, then `step_back` a
   few times to inspect the state that *produced* the bad legality. At each cursor
   read `replay_step_view`, plus `field` / `hand` / `pending_selection` /
   `effect_queue` / `legal_now` as needed.
3. **Localize to a card.** Identify which card's legality the engine got wrong:
   - `mask_miss` on a play/digivolve/attack/effect → the engine refused an action
     DCGO took. Find the card the recorded `action_id` refers to (decode via the
     step view's `recorded` label / `inspect_card` / `legal_actions`).
   - `actor` → the engine expected a different decision player (priority / timing
     window bug). Localize to the effect that opened the window.
   - `winner` / `reveal_*` → loss-condition or opaque-pile bug; localize to the
     triggering effect.
4. **Confirm vs ground truth.** A recorded action the Rust engine masks is a
   **Rust-bug signal, not a recording error**, absent strong evidence otherwise.
   Confirm by reading:
   - the card's DCGO C# at `$BASE_DCGO/.../<CARD_ID>.cs` (how it actually
     resolves), and
   - the relevant `general_rule.pdf` rule (keyword semantics in §16; cite the rule
     number, e.g. `16-36`) via the Read tool's `pages` arg.
5. **Record** a finding (Phase 3) citing the card text, the DCGO C# behavior, and
   the rule number where applicable.

---

## Phase 2 — Mode 2: judge playbook (faithfulness)

Goal: for each effect-bearing action, decide whether the engine did what the card
text + rules mandate. No oracle — you are the judge.

1. **Gather leads first.** Run `scan_fizzles` (steps that emitted `EffectFizzled`
   — an effect that did nothing) and `scan_panics` (steps the engine applied as a
   silent no-op — a recorded decision it couldn't reproduce). Both preserve your
   cursor. Treat each finding as a step to investigate, not a confirmed bug.
   - **Read the no-op ratio as a replay-fidelity gauge.** Native replay does not
     re-walk the RNG path, so once a single random effect resolves differently the
     replay diverges and most *later* recorded actions become illegal for the live
     state and are silently dropped. A high `scan_panics` ratio (e.g. most of the
     game flagged) therefore means **the replay diverged early** — localize the
     **first** no-op (or the first effect with a random outcome before it) and do
     **not** trust the judged state past that point. A low ratio means the replay
     tracked the recording and per-step judgment downstream is reliable. (A
     panic-regression recording that was carried through by a soft-fail will also
     show a high ratio for the same reason.)
2. **Walk the game.** `step_forward` through the recording. For each step whose
   `recorded` action is effect-bearing (play / digivolve / attack / option / a
   selection resolution), read from the step view:
   - `recorded` — what action was taken,
   - `events` — what the engine emitted (triggers, memory changes, deletions,
     placements, fizzles),
   - `delta` — memory + zone-size change,
   - and pull `inspect_card` for the card's printed `effect_text` /
     `inherited_text` / `security_text`.
3. **Judge each effect.** Compare what happened against:
   - the card's printed text (`inspect_card`),
   - `general_rule.pdf` (timing / keyword semantics — the canonical source), and
   - DCGO C# at `$BASE_DCGO` when the text is terse or the interaction is subtle.
   Ask: did the effect fire when it should? select a *legal* target? produce the
   board change the text mandates (right zone, right count, right DP)? respect
   "may" optionality, "by [cost]" payment, and "or" exclusivity?
4. **Per-effect verdict:** `faithful` / `not-faithful` / `blocked` (engine lacks a
   primitive to express it). Record `not-faithful` and `blocked` verdicts in
   Phase 3.
5. **Do NOT flag these as faithfulness bugs:**
   - An **unrevealed / `<hidden>` card** in an opaque zone (it's partial
     observability, not a bug).
   - **RNG-replay non-determinism** — native replay does not re-walk the RNG path,
     so effects with random reveal/selection may legitimately differ from the
     recorded outcome. That divergence is a known replay limitation, not a card bug.

---

## Phase 3 — Route confirmed findings (no engine fixes)

Write each confirmed finding to the existing tracker. **Do not modify engine
code** as part of a hunt — this ships the microscope, not the repairs.

- **Card-effect faithfulness gap** (one card's logic is wrong/missing) → append to
  `qa/archetype-qa/engine-gaps.md`.
- **Engine-primitive gap** (a missing mechanic, not a single card's logic — rule
  28's "widen the substrate") → append to `docs/RUST_ENGINE_GAPS.md`.

Each entry MUST record:
- the **recording path + step** (and `game_id`),
- the **divergence kind** (Mode 1) or **verdict** (Mode 2),
- the **card** involved,
- the **source consulted**: DCGO C# location (`$BASE_DCGO/...`) and/or
  `general_rule.pdf` rule number.

### Finding entry template

```markdown
- **[recording basename] step N** — {card_id} {card_name}
  - Mode: 1 differential | 2 judge
  - Signal: mask_miss/actor/winner/reveal_* (Mode 1) | not-faithful/blocked (Mode 2)
  - Observed: {what the engine did — events / delta / mask}
  - Expected: {what the card text + rules require}
  - Source: $BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs ; general_rule.pdf §16-XX
  - Repro: load_recording {path} → seek {N} (→ step_back to inspect lead-up)
```

## Relationship to the parity harness

`dcgo-replay` (batch) and this skill share **one** replay core (`ReplaySession` /
`LiveGame` over the same `DcgoAdapter` / `NativeAdapter`). The harness is the
**funnel** — it scores a whole recording corpus and flags divergent DCGO games.
This skill is the **microscope** — it takes one flagged game (Mode 1) or one
eval/self-play game (Mode 2) and produces a confirmed, localized, sourced finding.
Use the harness to choose what to hunt; use this skill to hunt it.
