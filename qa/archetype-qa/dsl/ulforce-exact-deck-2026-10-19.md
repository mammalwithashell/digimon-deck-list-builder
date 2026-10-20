# Exact UlforceVeedramon deck - 2026-10-19

All 19 distinct cards in the user-provided digimonmeta.com export have production YAML that loads, lowers, and registers. The exact-deck regression constructs 50 main cards and four EX13-002 Digi-Eggs for each player.

Worktree: `C:/Users/james/.codex/worktrees/ulforce-deck-yaml/digimon-deck-list-builder-1`, based on local `origin/main` at `a081bfd44`. The original dirty checkout is separate. This is the requested exact-deck milestone, not completion of the broader historical archetype pool.

## Card scope

Seven new YAML cards, one carried-forward implementation (BT11-023), and 11 existing YAML cards.

| Card | Copies | Work | Embedded/runtime load |
|---|---:|---|---|
| EX13-002 | 4 | Existing YAML | PASS |
| EX13-017 | 4 | Existing YAML | PASS |
| BT11-023 | 4 | Carried forward | PASS |
| BT22-019 | 4 | New YAML | PASS |
| P-138 | 4 | New YAML | PASS |
| EX13-019 | 4 | Existing YAML | PASS |
| BT22-022 | 1 | New YAML | PASS |
| EX13-022 | 4 | Existing YAML | PASS |
| BT22-023 | 3 | New YAML | PASS |
| EX13-023 | 4 | Existing YAML | PASS |
| BT12-029 | 2 | New YAML | PASS |
| EX13-069 | 4 | Existing YAML | PASS |
| BT11-112 | 4 | New YAML | PASS |
| BT22-085 | 1 | New YAML | PASS |
| P-104 | 2 | Existing YAML | PASS |
| LM-034 | 2 | Existing YAML | PASS |
| LM-028 | 1 | Existing YAML | PASS |
| LM-057 | 1 | Existing YAML | PASS |
| LM-058 | 1 | Existing YAML | PASS |

## Verification

- Exact deck load/registry/lowering/construction test: PASS.
- Batch 1 (BT22-019, BT22-022, BT22-023, P-138): 29 behavioral tests passed. Independent spec and code review passed.
- Batch 2 (BT12-029, BT11-112, BT22-085, carried BT11-023): independent spec and code review passed.
- Exact 19-card focused run: 229 passed, one failed out of 230. The failure was an invalid synthetic Tamer fixture with level 0 and DP; the fixture was corrected to no level or DP, retaining tie selection, Tamer exclusion, mandatory choice, and OPT assertions. Followup: all six BT12-029 tests plus the exact-deck test passed (7/7). Across these runs, all 230 unique focused checks have passing evidence.
- Scoped Rust formatting and whitespace checks passed.
- No shared engine, DSL vocabulary, action, tensor, or generated printed-data changes.
- Full engine suite and DCGO comparisons were not run: verification was scoped to the user's exact-deck loading milestone and focused card regressions.

Local evidence logs (retained in the authoring worktree): `.codex/ulforce-exact-red.log`, `.codex/ulforce-batches-mixed.log`, `.codex/ulforce-exact-final.log`, `.codex/ulforce-exact-followup.log`.

Reproduce the exact-deck and all focused regressions from this worktree:

```powershell
$env:CARGO_TARGET_DIR='D:/cargo-target/ulforce-deck-yaml'
$env:RUST_MIN_STACK='33554432'
cargo test -p digimon-engine --test cards_behavioral -- ulforce_exact_export ex13_002:: ex13_017:: bt11_023:: bt22_019:: p_138:: ex13_019:: bt22_022:: ex13_022:: bt22_023:: ex13_023:: bt12_029:: ex13_069:: bt11_112:: bt22_085:: p_104:: lm_034:: lm_028:: lm_057:: lm_058:: --test-threads 4
```

## Limits

Loadability and focused simulator tests are not whole-archetype, training, or DCGO certification. DCGO status is unmeasured. BT11-112 Q2143 (the same Digimon evolves before Rina's pending refire resolves) remains unverified by this test set; the newer baseline has identity-aware event resolution, so this is not labeled a demonstrated engine defect. BT11-112's keyword expiry uses the established opponent-turn expiry primitive but lacks its own boundary assertion. BT22-019 lacks an opponent-turn cost-reduction negative fixture.
