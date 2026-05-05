# Rocks Rust DSL Batch Log

Date: 2026-05-04

Deck resolver input: `Rocks`

Resolved pool artifact: `qa/archetype-qa/rocks/deck_pool.json`

## Batch 1

Cards: `BT14-009`, `BT18-064`, `EX8-051`, `ST13-08`

Status: `IMPLEMENTED`

Verification:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt14_009 bt18_064 ex8_051 st13_08 --nocapture
```

Result: 9 passed.

Notes:

- `BT14-009` moved from example-only YAML to production set YAML and covers bilateral `CannotPlayDigimonByEffect`.
- `BT18-064` covers hand/deck return immunity plus inherited opponent-turn DP contribution.
- `EX8-051` covers printed keywords and exact-source-trash inherited De-Digivolve 1; this required reusable queue support for the trashed source card's own inherited `OnDigivolutionCardTrashed` effect.
- `ST13-08` covers bilateral play-cost-reduction lock.

## Batches 2-9

Status: pool pass complete.

Implemented or partial YAML/test passes:

- Batch 2: `EX8-005`, `BT21-055`, `EX10-025`, `EX8-047` - source-trash inherited memory/delete clauses.
- Batch 3: `EX8-046`, `EX11-038`, `EX10-028`, `EX10-032` - source-trash draw/delete/De-Digivolve plus `EX8-046` Blocker slice.
- Batch 4: `BT4-072`, `EX8-050`, `EX10-034`, `P-215` - static inherited/face-up keyword and DP slices.
- Batch 5: `EX8-048`, `P-167`, `P-186`, `EX8-055` - source-trash delete/De-Digivolve, Rush/Blocker, Fragment(3).
- Batch 6: `BT23-059`, `EX10-033`, `EX10-036`, `EX11-044` - Blocker/Reboot/Fragment(3) slices.
- Batch 7: `EX8-067`, `P-039`, `P-107`, `P-169` - memory setters, Memory Boost/Training reveal and Delay slices.
- Batch 8: `EX10-063`, `EX7-049`, `LM-031`, `LM-032` - source-trash Tamer memory, De-Digivolve, and Scramble main digivolve slices.
- Batch 9: `BT23-096`, `BT8-094`, `EX10-069`, `ST22-11` - security/main De-Digivolve, security play, and Unique Emblem hand/trash play slices.

Blocked after pass:

- `BT20-055`: face-up security lifecycle and security end-of-opponent-turn play timing.
- `BT21-021`: conditional inherited keyword, Save, and Xros Heart play routing.
- `BT9-103`: play-cost-filtered player attack restriction and opponent security-add lock.
- `EX10-003`: attack cancellation by trashing three Mineral/Rock sources.
- `EX11-065`: hand-or-source costs plus source placement from hand/trash.
- `EX8-070`: source-cost selection, temporary protection, and lowest-play-cost security delete.
- `P-130`: effect move-from-breeding and on-move suspend-memory trigger.

Pulled-main update:

- `P-123` now has production YAML/tests on main and is no longer counted in the Rocks blocked remainder.

Verification slices:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_005 bt21_055 ex10_025 ex8_047 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_046 ex11_038 ex10_028 ex10_032 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt4_072 ex8_050 ex10_034 p_215 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_048 ex8_055 p_167 p_186 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_059 ex10_033 ex10_036 ex11_044 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_067 p_039 p_107 p_169 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_063 ex7_049 lm_031 lm_032 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_096 bt8_094 ex10_069 st22_11 --nocapture
```
