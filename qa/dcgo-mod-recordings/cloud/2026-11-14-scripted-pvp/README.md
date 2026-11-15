# Scripted DCGO PvP on Linux cloud

Actual live run on 2026-11-14, using the official DCGO 1.19.3 IL2CPP standalone,
Wine 10, MelonLoader 0.7.3, DXVK/Lavapipe and Xorg dummy `:102`. Both seats ran
`dcgo-agent --script qa/dcgo-mod-recordings/playback/sim_gd_dna.json`. The
`prepare-playback.py` helper created these isolated clients and configurations.
See [the cloud playbook](../../../../docs/runbooks/dcgo-linux-cloud.md).

## Result

**The cloud transport and two-seat automation work. This fixture diverged on
turn 6 and did not complete. No winner or Oracle clause confirmation is claimed.**

Both clients joined the same Photon room, loaded the 50+4-card plans, selected
their decks, and entered a real PvP battle (`is_ai: false`). Both mulligans,
digivolutions, plays, attacks, security effects and selection prompts were driven
automatically. BattleScene loaded at 21:08:53 UTC; the first script divergence
was reported at 21:25:52 UTC. No manual game decisions were used to continue it.

Each converted recording contains 53 rows: two headers, **31 actions and
20 selections**, with **0 inferred decisions** and no `game_end`. Conversion
used no `--allow-truncated` override. Each replay reports **0 pass, 1 partial
pass, 0 fail, 0 skipped actions**. These are valid captured prefixes, not full
game passes. The native input's separate preflight finishes with seat 1 winning
on turn 7; that outcome was **not** reached by this live run.

## Reproducible findings

1. **Duplicate permanent identity / state-audit gap.** After the two `ST23-03`
   attacks on turn 4, DCGO's P0 trash lacks `ST23-01` and `ST23-12` present in the
   engine replay. The difference persists in later snapshots. The native field
   dump before the first attack has a single-card Cougarmon in slot 0 and a
   three-card Cougarmon stack (`ST23-01`, `ST23-12`, `ST23-03`) in slot 2.
   Recorded attack boards identify both only as `ST23-03`, in DCGO frame order.
   This supports investigating duplicate-permanent identity in execution and
   replay; it does not establish a DCGO card-rules bug. Both raw logs and all
   audit warnings are retained. The agents reported four and five warnings,
   respectively, as the surrounding trash contents changed.

2. **Optional trigger / cost-reduction divergence.** Both initial states and
   the first 50 recorded decisions match the existing Windows recordings after
   excluding game ID and timestamp. At converted decision 50, the baseline has
   `MultipleSkills` with `int_value: -1`; this run has `int_value: 0` and
   `chosen_effect_card: BT25-049`. The existing driver's fallback says
   `DCGO asks a trigger order the engine did not - resolving BT25-049 first`.
   After playing e-Pulse (`ST23-15`), the script expects
   `Use Cost reduction to reduce Option-use cost?` but the reconstructed state
   is `Choose a Tamer to trash a face-down card from under`. The visible client
   also shows that Tamer-selection prompt. The host returns `status: diverged`
   at script cursor 48 / replay step 49. The agents were stopped without forcing
   a choice or a concession. No game-rule or replay-fallback fix was applied.

The script fingerprint is based on the engine replay of captured actions.
Matching it does not establish equality with DCGO's actual zones. Preserve the
independent zone audit and resolve these issues before using this fixture as
full-state Oracle evidence or training ground truth.

## Evidence and verification

- `host-raw.jsonl` / `join-raw.jsonl`: 199 / 193 original mod rows, including snapshots.
- `host.jsonl` / `join.jsonl`: converted prefixes, kept separate from raw data.
- `*-conversion.log`, `*-parity.json`, `*-replay.log`: unmodified tool results.
- `agent0.log` / `agent1.log`: scripted answers, warnings and divergence details.
- `*-melon.log`, `*-player.log`, `*-wine.log`: both live clients and transport.
- `summary.json`: hashes, per-seat counts, exact first divergence and baseline comparison.
- `plan.txt`: shared deterministic deck order; native fixture remains at its existing repo path.
- `native-before-first-attack*.json`: native state and physical stack differences.
- `native-replay.log`: input preflight; `path-tests.log`: two passing Wine-path tests.
- `prepare-playback.log`: helper exercised on both actual client copies.
- `mod-build.log`: Linux mod rebuild, 38 dependency warnings and zero errors.

The live DLL hash and final rebuild hash are recorded separately. The final
source cleanup only removed an unused experimental DNS switch; the live run
used the managed WebSocket path and did not set that switch.

An earlier attempt lost its X server during turn 3. Its bounded status is saved
as `interrupted-summary.json`; these raw captures come from the fresh run above.
Each client also issued one request before its raw file existed during startup;
both recovered on the next request and completed the scripted mulligans.

No engine card effects, Oracle clause ledger entries, or admissibility artifacts
were changed as a result of this run. The separately authored ST1-15 scenario
remains simulator-only.
