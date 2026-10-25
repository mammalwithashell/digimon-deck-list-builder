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
The original YAML explicitly refunded OPT on decline, and the then-existing
test `bt26_012_declining_refunds_once_per_turn` incorrectly required reopening.
DCGO's `BT26_012.cs` likewise calls `RemoveUse()` when no card was selected.

Follow-up correction: the explicit Main-effect action is the declaration of
activation, not a provisional selection. CR 4.3 15-14-1-1/4 counts that activation
toward the use even when processing does not execute. Declining the optional
hand play therefore must not refund OPT. The user's installed-DCGO observation
also reports the use remains spent; the local C# refund is conflicting reference
evidence, not certification of that installed build or authority over the rule.

The refund has been removed. No repetition penalty, action override, simulation
budget change or cycle-aware search is needed to eliminate this specific cycle.
The other 52 YAML files mentioning `refund_opt` have not been adjudicated by this
single-card repair; cost-payment/declaration boundaries require individual review.

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

## Follow-up repair: nested Delay hand-play continuation

The Rust replay confirmed the first lost choice: accepting EX12-070 at decision
178 enters `install_delay_play_from_hand_after_paid`. Its `finish_owned_leave`
opens another leave-replacement selection. The hand reward then overwrote that
selection and its resume stack, inheriting the transient EffectChoice phase.

The hand-only flow now preserves that inner prompt and arms the existing
`DelayPlayFromHandAfterSelection` continuation. This matches the sibling
union-zone flow. The reward resumes only after the inner operation finishes;
there is no blanket phase reset, action suppression, or search-budget change.

`tests/selection/selfplay_phase_resume.rs` reconstructs the recorded seed/decks
and first 179 actions. The corrected continuation has an additional legal
replacement choice, so the old suffix cannot be replayed verbatim. The test
branches over accepting/declining that replacement and playing/declining the
reward, compares legal masks and UI state after cloned resumption, and requires
completion into playable Main or GameOver without an orphaned prompt phase.

At completion of the phase repair, game 340 was still unresolved; the later
activation-use correction above supersedes that disposition. The old collection
remains excluded from training. The cloud service is unchanged;
the repaired engine must be built into a new image before a future campaign
can use it.

Validation: the initial Rust replay failed with `EffectChoice` instead of Main,
matching the recorded defect. After repair, all four continuation branches
complete with a winner (not a loop draw), with original/clone masks and UI states
matching after each choice. Selection: 80 passed; replacement: 126 passed;
combat: 299 passed, 1 pre-existing ignored; Option flow: 143 passed. All 13 tier-0
checks passed. These are focused engine checks, not a full-engine or DCGO run.

## Follow-up validation: Manekimon activation consumes its use

Before the card fix, `capped_game_340_cannot_reopen_manekimon_after_declining`
failed because action 1002 remained legal after the recorded activation and
decline. After removing the authored refund, the same prefix passes in original
and cloned states: hand and memory are unchanged, Main resumes, the spent
activation is unavailable, and ordinary legal actions remain. This verifies the
specific recorded cancellation cycle is eliminated; it is not a fresh complete
search-game outcome or a trained-model quality evaluation.

Validation: all 6 Manekimon card tests passed, all 81 selection tests passed
(including both capped-game regressions), and all 13 tier-0 checks passed.
The corrected per-card test also verifies another copy retains its own use.
Generated impact/coverage/admission metadata was refreshed. The running cloud
campaign was not changed; this correction still requires a new deployed image.
