# Top-10 construct campaign completion

> Historical campaign result below: 9/10 admitted. The subsequent
> [App Fuse OPT followup](app-fuse-opt-followup.md) closes BT25-089's hold;
> the refreshed artifact admits 10/10 lists and 167/167 unique pool IDs.

The frozen DigiLab-11087 pool contains 10 lists, 540 card entries and 167
unique IDs. All 51 original untested construct keys now have behavioral
coverage. Final admission measurement exposed four additional parsed-only
keys, covered by six more tests: `predicate:memory_lte`,
`predicate:name_not_shared_by_field_tamer`, `predicate:event_target_is_source`
and `timing:on_ally_played`. The original 51-key union remains unchanged as
the campaign baseline.

| Measurement | Final result |
|---|---|
| Campaign construct keys behaviorally covered | 55/55 (51 original + 4 tail) |
| Scanner lowering-covered lists | 10/10 |
| Per-card admissible lists | 9/10 |
| Per-card admissible unique pool IDs | 166/167 |
| Remaining blocked pool card | BT25-089: `failed_verdict`, no untested constructs |

Beelstarmon, both Jupitermon lists, TS Toolbox, Chronomon, Examon, Toho Braves,
Medusamon and DATA SQUAD Ravemon pass the per-card artifact. Rebootmon does not.
Scanner coverage includes parsed-only credit; admission requires behavioral
coverage. Neither result establishes DCGO parity or full training fidelity.
Gate semantics, global mechanics triage and production YAML are unchanged.

## Implementation and scope

Six families cover security (7 keys), play/digivolve/link (13), cost/event
(13), identity/state (10), zone moves (5), and protection (3). Fixes address
successful security movement receipts/observers, destination restrictions,
atomic source costs, replacement-aware deletion receipts, stable Option
relocation, grouped returns, subject-aware predicates, hatch/DNA choices,
negative DP formulas and source-return protection. Player choices remain
visible through existing action masks and pending selections.

The owner-authorized followups implement Q3185 attacks without suspending,
including attack-count bookkeeping, and all 12 English name/query exclusions
with alias and DUAL-face handling. P-217's proposed Creation link-trait change
was withdrawn after inspecting the printed image. The old App Fuse and
grouped-return test targets were folded into the DSL suite without dropping
their nine and seven tests respectively.

## Known limits

Open gaps remain documented in `docs/RUST_ENGINE_GAPS.md` and
`qa/dsl-vocab-gaps.md`: App Fuse result/OPT refund and procedure details,
effect-link would-link timing, the 18-card own-number self-proxy sweep,
queued sequential-deletion observer compaction, nonbatch plural prevention,
selection encoding limits and the other family-specific findings. Protection
rows requiring own DP-payment or keyword-removal DSL primitives were not
expressible and remain unmeasured. Face-down source-count behavior was retained
after the specified DCGO comparison; that reference is not an official ruling.

## Validation

| Check | Observed result |
|---|---|
| Tier 0 | 13/13 checks; 174 tool tests passed |
| Full DSL before six tail tests | 1,350 passed, 2 baseline failures, 1 existing ignored |
| Six tail tests in rebuilt main DSL target | 6 passed, 1,353 filtered |
| Rules conformance | 361 passed, 1 existing ignored |
| Timing dispatch / cost hooks / selection | 52 / 100 / 79 passed |
| Clone fuzz | 1 passed |
| Replacements, final production source | 126 passed |
| Keyword D / E / F, final production source | 48 / 10 / 50 passed |
| Replay goldens | 10/10 verified, unchanged |
| Full card suite | 12,261 passed, 4 failed, 36 existing ignored |
| Corrected card modules in rebuilt main cards target | 37 passed, 12,264 filtered |

The full card run exposed three outdated fixtures: a DNA target without DNA
requirements, an expectation of partial payment for an atomic source cost,
and reversed first-pick deck-bottom order. The three affected modules then passed all 37 tests in the rebuilt main card
target. The failures remain recorded in the aggregate run; this focused result
does not replace that run. Both fresh spec and quality reviews approved the
fixture corrections; no further production changes were required.

The remaining known failures were reproduced on baseline `f24f26dfa` in an
isolated target before attribution: DSL `official_rule_grants` and
`trait_parity` (missing Aquatic on EX12-026/031 and Holy Dragon on BT22-023),
and the BT24-085 option-use clause color assertion (Green/Red versus expected
Yellow/Red). The three fixture failures passed on that baseline and were
investigated separately. No whole-repository green claim is made. All six
families and the authorized followups had fresh spec and quality review;
final integration review found no actionable issue.

## Reproduction

Set `PYTHONIOENCODING=utf-8`, `PYTHONUTF8=1`, and `PYTHONPATH=code`.
Run `python -m tools.lowering_coverage`, then
`python -m digimon_gym.agents.card_admissibility`. For each frozen deck, require
all its IDs to have `cards[id].admissible` in `data/card_admissibility.json`.
Run `python scripts/verify --tier 0` to check generated artifact freshness.
Use `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs` for Rust validation and
`RUST_MIN_STACK=268435456` for the card suite. Local run logs and parked
reproducers are under `.superpowers/sdd/2026-10-22-construct-tests/`; tail logs
are under `runs/task8-tail/`. These ignored logs are local evidence, not
portable CI artifacts.
