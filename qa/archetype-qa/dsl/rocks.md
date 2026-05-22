# Rocks Rust DSL Batch Log

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.


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

Blocked after pass (Phase 2 Track E 2026-05-17 update — BT9-103 advanced to IMPLEMENTED):

- `BT20-055`: face-up security lifecycle and security end-of-opponent-turn play timing.
- `BT21-021`: conditional inherited keyword, Save, and Xros Heart play routing.
- ~~`BT9-103`~~: **IMPLEMENTED** 2026-05-17 via Phase 2 Track E. Authored with
  `add_player_modifier` (CannotAddSecurityByEffect) + `for_each` +
  `add_modifier` (CannotAttackPlayer, `play_cost_lte: 7`, expiry
  `end_of_opponents_turn`). See `code/digimon-engine/cards/bt9/BT9-103.yaml`
  + `code/digimon-engine/tests/cards_behavioral/bt9/bt9_103.rs`.
- `EX10-003`: attack cancellation by trashing three Mineral/Rock sources.
- `EX11-065`: hand-or-source costs plus source placement from hand/trash.
- `EX8-070`: source-cost selection, temporary protection, and lowest-play-cost security delete.
- `P-130`: effect move-from-breeding and on-move suspend-memory trigger.

Phase 2 Track E (2026-05-17) PARTIAL → IMPLEMENTED advancements:

- `P-167`: `[Start of Your Main Phase][When Digivolving]` reveal/source-trash/search/source-placement clause now authored using the new `choose_from_reveal` + `order_remainder` DSL verbs. Inherited source-trash De-Digivolve clause unchanged.
- `EX8-047`: `[On Play]` reveal/two-pick clause (Mineral/Rock + LIBERATOR → hand, remainder to deck bottom) authored using the same new verbs. Inherited source-trash delete unchanged.

Phase 2 Track E (2026-05-17) DSL modernization (no status change):

- `P-206`: inherited security clause's `raw_rust { fn: p_206_add_self_to_hand }` replaced with native `add_this_option_to_hand: {}` (identical engine call). `G-ADD-OPTION-SELF-TO-HAND` closed.

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

---

## Batch 10 (2026-05-11) — formerly-blocked IMPLEMENT pass

Date: 2026-05-11
Pipeline: `/batch-implement-cards-rust-dsl Rocks`
Scope: 4 cards previously listed as "Blocked after pass" — re-attempted given engine progress since 2026-05-04.

| Card ID  | Name | Verdict | Tests | Notes |
|---|---|---|---|---|
| BT21-021 | OmniShoutmon | IMPLEMENTED | 24 (1 ign) | digixros_aliases ["Shoutmon"] + 3 alt-paths + [End of Attack] cost-5 play & self-delete + [On Deletion] hand/trash place under Tamer + auto-Save keyword + inherited [Your Turn] Rush aura. Ignored test: `G-DSL-AURA-TARGET-SOURCE-PERMANENT` for carrier-trait condition on inherited aura. |
| EX11-065 | Close | PARTIAL (dsl) | 20 (4 ign) | Clause 0 [Start of Your Main Phase] omitted (BLOCKED on `G-DSL-SELECT-OWN-SOURCES-FILTER` — hand-OR-digivolution-source trash cost). All Turns observer (event-target-trait filter + suspend-as-cost + place_as_bottom_source from hand/trash) and Security IMPLEMENTED. |
| EX8-070  | Zofr Kabus | IMPLEMENTED | 22 | Main: select Mineral/Rock Digimon + select source to trash + grant Collision/Piercing/Reboot/CannotBeReturnedToHand/CannotBeReturnedToDeck/+3000DP all with `expiry: end_of_opponents_turn`. Security: lowest-cost opponent delete via `raw_rust` workaround for `G-PLAY-COST-AGGREGATE` (same pattern as BT9-112). |
| P-130    | Lui Ohwada | PARTIAL (dsl) | 14 (3 ign) | [Your Turn] on_move trigger + Security IMPLEMENTED. [On Play] move-from-breeding BLOCKED on `G-MOVE-BREEDING-DSL` (no DSL step lowers to `ctx.move_from_breeding_by_effect`) and `G-SELECT-BREEDING-FILTER` (no `filter:` on `select_own_breeding_permanent` → level-3+ filter inexpressible). |

Verification:
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_021 ex11_065 ex8_070 p_130
# 70 passed; 0 failed; 8 ignored
```

Full-suite regression (no per-binary failures):
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
# All binaries pass; cards_behavioral: 2251 passed, 499 ignored.
```

### Blocked / partial residual after Batch 10
- `BT9-103` — play-cost-filtered player attack restriction + opponent security-add lock (carried over).
- `EX10-003` — attack cancellation by trashing three Mineral/Rock sources (carried over).
- `EX11-065` Clause 0 — `G-DSL-SELECT-OWN-SOURCES-FILTER` (new pattern observed).
- `P-130` [On Play] — `G-MOVE-BREEDING-DSL` + `G-SELECT-BREEDING-FILTER` (new gap entry filed).
- `BT20-055` — face-up security lifecycle and security end-of-opponent-turn play timing (carried over).
- All other Rocks AUDIT-mode cards re-verified green under cargo (36 of 36 build + tests pass, prior PARTIAL/BLOCKED verdicts unchanged — no engine-gap closures since 2026-05-04 that lift these).

### New DSL-vocab gaps filed
- `G-DSL-AURA-TARGET-SOURCE-PERMANENT` — carrier-trait condition on inherited aura target filter (BT21-021 [Your Turn] Rush).
- `G-MOVE-BREEDING-DSL` — no DSL step for `move_from_breeding_by_effect` (P-130 [On Play]).
- `G-DSL-SELECT-OWN-SOURCES-FILTER` — `select_own_sources` lacks `filter:` field (EX11-065 Clause 0; also affects any "hand OR digivolution-source" union cost shape).

### New pattern worth documenting in RUST_DSL_TEST_API.md
- **Single-trigger optional auto-fire vs declinable cost-gating.** When a clause is `optional: true` and the body has a synchronous step BEFORE any `PendingSelection`, that step (e.g. `suspend`) runs unconditionally — the optional flag does not gate it. To make a "by suspending this Tamer, you may …" body actually declinable, the FIRST step must install a selection with `optional: true`, and cost-paying steps must live in that selection's callback. EX11-065's Batch 10 fix adopted this pattern (`select_own_permanent: { optional: true }` first → suspend + place in the accept callback).
