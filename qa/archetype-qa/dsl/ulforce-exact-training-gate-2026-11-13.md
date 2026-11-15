# Exact EX13 UlforceVeedramon training qualification

Scope: the original 50-card main deck plus four EX13-002 eggs, 19 distinct IDs,
recorded in `ulforce-exact-deck-2026-10-19.json`. The card counts match library
entry `Ulforce / dcg_nexus_b760a865ce88` exactly:
https://dcg-nexus.com/decklist/6cc52111-a017-48db-a22d-c277535a326d.
Original implementation PR #740 is merged. This is a follow-up qualification,
not a claim about every historical Ulforce list.

Baseline main: `6b1a4fce1` (2026-11-13). A fresh canonical admission build admitted
16/19 IDs. BT12-029 and BT22-023 lacked behavioral proof for
`predicate:can_unsuspend`; LM-028 retained `failed_verdict` from an obsolete
G-ENGINE-EFFECT-DIGIVOLVE-COST-CHOICE blocker.

## Changes and evidence

- Add a shared permanent-target predicate matrix: both controllers, Digimon and
  Tamer subjects, both orientations, both predicate polarities, and individual
  versus player-wide locks. Verify actual unsuspension agrees with the predicate.
- Add real-card lock-filter and clone/resume checks for BT12-029 and BT22-023.
  Tamers remain selectable under a player-wide Digimon unsuspend lock.
- Requalify LM-028 only after all 22 existing card tests pass, including actual
  paid Option use, declining either prompt, and both reduced 0/1 cost routes
  after cloning. Main already contains the shared cost-choice repair. Its YAML
  and runtime are unchanged.
- Regenerate the canonical lowering/admission artifacts. Admission policy,
  mechanics flags, card counts, and action/tensor contracts are unchanged.
- Add a focused harness importing the existing tests for every exact-list card,
  plus an exact-list regression that checks the canonical gate, library identity,
  production card data and registry, and eight seeded masked games without
  conceding (4,000-step bound per game).

The card images and official bundles for all three blockers were checked.
DCGO source at the repository's pinned submodule revision
`02922babedf58c83b0687eeb91e2377cc977c58e` uses `Mode.UnTap` for both unsuspend
selections. Blue Scramble uses paid hand evolution reduced by three and then
places the Delay Option. This was source inspection, not a live DCGO comparison.

## Validation

The unchanged two-layer gate passes: 19/19 distinct cards admissible and
mechanics readiness true. All eight seeded smoke games passed, with legal
masks, production registry/data, and exact library identity checked in every
game. Action counts by seed 0–7: 257, 622, 314, 311, 228, 304, 295, 208.
No game used concede or the 4,000-action cutoff.

- 235/235 distinct focused checks have passing evidence, including all 22
  LM-028 tests and the exact embedded-registry/deck construction test.
- The combined process recorded 227 passes and three new test-fixture failures,
  then was killed by SIGKILL near the end. All three fixtures were corrected
  and reran green; all six P-138 tests passed in a fresh process. Test names
  were reconciled against the compiled harness's complete 235-test inventory:
  no missing checks. This is split-run evidence, not a claim of one clean
  monolithic process.
- Gate-policy regression tests: 103 passed, one optional dependency skip.
- `python scripts/verify --tier 0`: all checks passed, including 183
  verification-tool tests and freshness of the canonical gate artifacts.
- The exact-list gate regression first failed on LM-028's blocked verdict,
  before its 22 passing card tests justified requalification.

The new shared proof also naturally admits BT20-027 through the existing policy;
that card is outside this exact-deck behavioral qualification.

## Reproduction

```sh
PYTHONPATH=code python -m tools.rules.mechanics_readiness
PYTHONPATH=code python -m tools.lowering_coverage
PYTHONPATH=code python -m digimon_gym.agents.card_admissibility
RUST_MIN_STACK=268435456 cargo test -p digimon-engine --test ulforce_training_gate --test ulforce_training_smoke -- --test-threads=2
python -m pytest code/tests/rl/test_card_admissibility.py code/tests/tools/test_lowering_coverage.py code/tests/tools/test_rules_mechanics_readiness.py -q
```

Focused-card cloud builds used `CARGO_BUILD_JOBS=2`,
`CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_PROFILE_DEV_OPT_LEVEL=0`.
Smoke verification uses one build job plus
`--config profile.dev.package.digimon-engine.opt-level=2`; unoptimized
simulation was progressing but too slow (roughly 100 actions per minute).
The same eight seeds and 4,000-step bounds are retained.
The default ONNX Runtime CDN was unavailable; the matching 1.24.2 shared library
from the Python wheel was supplied through `ORT_LIB_LOCATION`,
`ORT_PREFER_DYNAMIC_LINK=1`, and `LD_LIBRARY_PATH`. These are local build settings,
not repository changes. To bound memory, run the focused harness one module
filter at a time (for example `cargo test -p digimon-engine --test
ulforce_training_gate lm_028:: -- --test-threads=1`).

## Limits

The full monolithic card suite, full DSL suite, and live DCGO comparisons were
not run for this follow-up. No trained model or training job is produced.
The mechanics gate means no flagged outcome-changing blocker: its ledger still
reports 314 unflagged open mechanics gaps. Existing unmeasured card interactions
in the original implementation report remain unmeasured; admission is not a
blanket certification of every card interaction.
