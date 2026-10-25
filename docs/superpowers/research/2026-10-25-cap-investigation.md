# Decision-cap investigation: games 340 and 850

Both capped games from the second 3,300-game collection were reconstructed
against its exact deployed image and parent ONNX model. These are two different
failure modes, not naturally long games. Neither outcome is a valid draw.

The reproduction input and compact action prefixes are in
`2026-10-25-cap-investigation.json`. Player IDs in its action list are Rust
zero-based; UI evidence retains Python's one-based player IDs.

## Method and evidence boundary

`code/tools/cloud_search/replay_capped_games.py` verifies the committed chunk
hashes, repeats the first 40 decisions with the original 320/4/batch-4 search and
model to recover the temperature-sampled opening, then uses the committed
highest-visit actions after temperature ends. Every saved observation must match
bit-for-bit before executing its action. Forced actions are reconstructed from
the mask. This verifies the recorded trajectory without repeating thousands of
expensive searches.

All 1,199 saved observations for game 340 and all 177 for game 850 matched.
Both reconstructed trajectories ran to the original 1,200-decision cap.
UI hashes diagnose visible recurrence; they are not claimed to hash every hidden
engine continuation. Game 850's mask/decoder mismatch is independently evident
from the missing pending selection and native dispatch implementation.

Game 850 initially hit the Python environment's no-op tripwire after repeated
Pass. The diagnostic was then rerun through `env.runner.step`, matching native
self-play, which has no such Python tripwire. This bypass exists only in the
read-only diagnostic; production guards were not changed.

Raw full traces are archived remotely under
`/opt/digimon-search/generation1-cap-investigation/`, and locally under
`runs/cloud-search-20261025-caps/`. Production experiment/data/model mounts were
read-only. The diagnostic used one CPU, low CPU shares, and at most 3 GiB.

## Game 340: Manekimon declaration/cancellation cycle

- Matchup: Chronomon slot 0 versus Toho Braves slot 1.
- Game seed: 5522269491488737205; turn 13.
- Only 95 unique visible states over 1,200 decisions.
- From decision 93 onward, action 1002 opens field-slot-0 Manekimon BT26-012's
  optional hand-play/use selector; action 62 declines it. Nothing in the visible
  position changes when that pair completes. The two states occur 554 and 553
  times respectively.
- Main-phase legal actions include ordinary plays, digivolutions and passing
  the turn. The hand selector has four TB candidates plus decline; it is not an
  empty or forced selection.
- At decision 93, root visits favored reopening the effect: 106/320 versus
  47/320 for the next action. At decision 94, decline received 203/320. At the
  end of the recorded game, the same two choices still ranked highest despite
  different search seeds. This is not just a fixed-action tie-break.

Printed card image inspected:
`C:/Users/james/Documents/DCGO_Application/Assets/Textures/Card/BT26-012.webp`.
Its main effect optionally plays or uses a TB card with cost reduced by two.
The authored YAML explicitly refunds OPT on decline, and the existing behavioral
test `bt26_012_declining_refunds_once_per_turn` requires reopening to remain legal.
DCGO's `BT26_012.cs` likewise calls `RemoveUse()` when no card was selected.

This establishes the implementation's intended cancellation behavior, not a
publisher ruling that repeatedly reopening a selector is a distinct game action.
CR 4.3 sections 15-8-4, 15-9-2 and 15-14-1 distinguish declaring activation,
optional processing and counting uses. A fix must explicitly distinguish a
cancelled pre-commit selection from an already declared activation. Do not
silently spend OPT or remove legal target choices merely to stop the loop.

Search currently bounds traversal depth but does not identify this repeated
decision cycle. Review cancellation semantics first, then implement a tested
cycle-aware search/collection policy that preserves engine legality. Keep the
simulation budget and determinization quality separate from this issue.

## Game 850: orphaned effect-choice phase after nested combat

- Matchup: Chronomon slot 1 versus Toho Braves slot 0.
- Game seed: 9305982431877686771; turn 22.
- From decision 188 onward, the same visible state appears 1,012 times.
- Phase is `EffectChoice` (UI 13), but `pendingSelection` and `pendingAttack`
  are both absent. The action mask offers only Pass (62).
- `decode_action` routes EffectChoice to `resolve_selection`; without a pending
  selection that returns `NoPendingSelection`, which the decoder discards.
  Consequently Pass cannot advance the game. Native self-play treats it as
  forced, so those 1,012 iterations do not create search targets.

The transition trace narrows phase leakage to this sequence:

1. Chronomon's attack installs a deletion choice for the opposing MarineBullmon.
2. The opponent accepts Sanmyojin Arrival EX12-070's leave-area replacement.
3. The resulting free-play selector already records `previous_phase=EffectChoice`
   instead of the enclosing Main phase; Ryugumon EX12-036 is played.
4. Nested trash-return and trigger-order prompts preserve that EffectChoice
   return phase; Ryugumon attaches a source, unsuspends Susanoomon, and locks the
   opposing attacker.
5. Susanoomon blocks. After combat deletes the attacker and clears the pending
   attack, the engine remains in EffectChoice with no selection.

This is a confirmed phase/pending-selection invariant violation. The exact
writer that first loses the enclosing phase still needs an instrumented Rust
regression; the trace localizes it between decisions 178 and 179. Investigate
replacement resumption, nested play/trigger prompts, and combat completion.
Do not globally normalize every empty selection phase to Main: real parent
contexts can be end-of-turn, breeding, or another suspended operation.

Repair priority: preserve/restore the parent continuation's phase, test this
exact replay plus nested/declined variants, and replace the misleading orphan
PASS fallback with an explicit diagnostic failure if the invariant is violated.
Increasing the search budget cannot repair a forced Pass that has no handler.

## Run disposition

The 1,376 saved rows from these two games have no valid terminal targets. The
entire old collection remains preserved and outside the active campaign, as
configured before this investigation. No model was fitted from it.

No card, engine, search-budget, training configuration or running service was
changed during this investigation. The new campaign remained active in baseline
evaluation at the final live check. Replays and findings are investigation
evidence; neither defect is claimed fixed or deployed.
