# Glowing Dawn exam repros

Scenario-format repros for engine findings from the Glowing Dawn DCGO-exam
authoring pass (2026-10-04). See `qa/dcgo-exams/ST23/NOTES-GLOWING-DAWN.md`
and `docs/RUST_ENGINE_GAPS.md`. They are NOT exam scenarios: each one's
`assert:` block pins the CURRENT (buggy) behaviour, so it passes `--sim-only`
today and must be flipped when the bug is fixed. They live outside
`qa/dcgo-exams/` so the exam corpus does not count or run them.

```bash
target/debug/dcgo-harness --root "$ROOT" exam --scenario qa/archetype-qa/glowing-dawn-repros \
    --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/ST23/glowing_dawn_pool.json
```

| File | Finding |
|---|---|
| `repro-barrier-leak.yaml` | G-ENGINE-BARRIER-CONDITION-NOT-CARRIER-SCOPED: inherited `<Barrier>` prompts the opponent and drops `<Piercing>` |
| `repro-dual-option-reducer.yaml` | BT25-049's Option-use reducer fires on a digivolve into a DUAL card |
| `repro-retaliation-effect-battle.yaml` | G-ENGINE-RETALIATION-EFFECT-BATTLE: `<Retaliation>` offered but no-ops after an effect battle |
| `repro-tamer-trash-trigger-mid-cost.yaml` | ST23-13's trash trigger resolves between the two cards of a "trash 2" cost |
